//! Run vault and access changes that need an authenticator code.
//!
//! Callers say what the code confirms ([`StepUpAsk`](crate::pages::step_up::StepUpAsk)), so Lepton's step-up
//! dialog reads "Type the code from your authenticator app to read
//! orders-db-password." Refused codes come back in words through
//! [`crate::components::error_copy`]; a code that was already accepted once in
//! this tab is reported as reused rather than wrong.

use std::cell::RefCell;

use lepton_auth::actions::step_up::verify_step_up_totp;
use lepton_auth_ui::{
    use_step_up_controller, StepUpController, StepUpFactors, StepUpPolicy, StepUpRequest,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;

use crate::components::error_copy;

/// True when the server asked for (or expired) a TOTP sudo window.
#[must_use]
pub fn is_step_up_challenge(err: &impl std::fmt::Display) -> bool {
    let s = err.to_string();
    s.contains("STEP_UP:step_up_required") || s.contains("STEP_UP:step_up_expired")
}

/// Error text that stands for a reused code until Lepton reports reuse itself.
const REUSED_CODE: &str = "STEP_UP:step_up_reused: that code was already used";

thread_local! {
    /// The last code the server accepted in this tab. Codes work once, so the
    /// same code coming back refused means it was reused. Never stored anywhere.
    static LAST_ACCEPTED_CODE: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn remember_accepted(code: &str) {
    LAST_ACCEPTED_CODE.with(|c| *c.borrow_mut() = Some(code.trim().to_string()));
}

/// Lepton answers `step_up_invalid` for a wrong code and for a reused one.
/// It's a reuse when the refused code is the one accepted last.
fn is_reused_code(last_accepted: Option<&str>, code: &str, err_text: &str) -> bool {
    err_text.contains("STEP_UP:step_up_invalid")
        && !code.trim().is_empty()
        && last_accepted == Some(code.trim())
}

fn reuse_aware(err: ServerFnError, code: &str) -> ServerFnError {
    let reused =
        LAST_ACCEPTED_CODE.with(|c| is_reused_code(c.borrow().as_deref(), code, &err.to_string()));
    if reused {
        ServerFnError::new(REUSED_CODE)
    } else {
        err
    }
}

/// No authenticator app: the dialog can't help, so the page explains it instead.
fn needs_enrollment(err: &ServerFnError) -> bool {
    err.to_string().contains("STEP_UP:totp_enrollment_required")
}

/// What the code confirms: the dialog's title and the action line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepUpAsk {
    title: &'static str,
    action: String,
}

impl StepUpAsk {
    /// A vault action, such as "read orders-db-password".
    #[must_use]
    pub fn vault(action: impl Into<String>) -> Self {
        Self {
            title: "Confirm it's you",
            action: action.into(),
        }
    }

    /// A grant or revoke, such as "give Mara Lindqvist Read the value on orders-db-password".
    #[must_use]
    pub fn access(action: impl Into<String>) -> Self {
        Self {
            title: "Confirm access change",
            action: action.into(),
        }
    }

    fn description(&self) -> String {
        format!(
            "Type the code from your authenticator app to {}.",
            self.action
        )
    }

    fn request(&self) -> StepUpRequest {
        StepUpRequest {
            title: self.title.to_string(),
            description: Some(self.description()),
            policy: StepUpPolicy::Totp,
        }
    }
}

/// Where a step-up call reports back.
pub struct StepUpCall<T: 'static> {
    /// Set when the call fails for good; cleared when it starts.
    pub error: RwSignal<Option<ServerFnError>>,
    /// True from the first try until success, failure or a closed dialog.
    pub busy: RwSignal<bool>,
    /// Runs with the call's value on success.
    pub on_ok: Callback<T>,
    /// Runs when the code dialog is closed without confirming.
    pub on_cancel: Option<Callback<()>>,
}

impl<T: 'static> Clone for StepUpCall<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for StepUpCall<T> {}

impl<T: 'static> StepUpCall<T> {
    /// Report to `error` and `busy`, then run `on_ok`.
    #[must_use]
    pub const fn new(
        error: RwSignal<Option<ServerFnError>>,
        busy: RwSignal<bool>,
        on_ok: Callback<T>,
    ) -> Self {
        Self {
            error,
            busy,
            on_ok,
            on_cancel: None,
        }
    }

    /// Also run `on_cancel` when the code dialog is closed without confirming.
    #[must_use]
    pub const fn with_cancel(mut self, on_cancel: Callback<()>) -> Self {
        self.on_cancel = Some(on_cancel);
        self
    }

    fn start(self) {
        self.error.set(None);
        self.busy.set(true);
    }

    fn succeed(self, value: T) {
        self.busy.set(false);
        self.on_ok.run(value);
    }

    fn fail(self, err: ServerFnError) {
        self.busy.set(false);
        self.error.set(Some(err));
    }
}

/// Watch the dialog close; if the flow hasn't finished, it was cancelled.
fn watch_cancel<T: 'static>(ctrl: StepUpController, done: StoredValue<bool>, call: StepUpCall<T>) {
    Effect::watch(
        move || ctrl.open().get(),
        move |open, _, _| {
            if !*open && !done.get_value() {
                done.set_value(true);
                call.busy.set(false);
                if let Some(cancel) = call.on_cancel {
                    cancel.run(());
                }
            }
        },
        false,
    );
}

/// Close the dialog because the flow is over (success or a failure the page shows).
fn finish_dialog(ctrl: StepUpController, done: StoredValue<bool>) {
    done.set_value(true);
    ctrl.complete_success();
}

/// Run `action`; on a window challenge, open step-up, verify, then retry once.
pub fn spawn_with_step_up<T, F, Fut>(ask: StepUpAsk, call: StepUpCall<T>, action: F)
where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let step_up = use_step_up_controller();
    call.start();
    spawn_local_scoped(async move {
        match action().await {
            Ok(value) => call.succeed(value),
            Err(err) if is_step_up_challenge(&err) => {
                let Some(ctrl) = step_up else {
                    call.fail(err);
                    return;
                };
                request_window_then_retry(ctrl, &ask, call, action);
            }
            Err(err) => call.fail(err),
        }
    });
}

fn request_window_then_retry<T, F, Fut>(
    ctrl: StepUpController,
    ask: &StepUpAsk,
    call: StepUpCall<T>,
    action: F,
) where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let done = StoredValue::new(false);
    watch_cancel(ctrl, done, call);
    ctrl.request(
        ask.request(),
        Callback::new(move |factors: StepUpFactors| {
            let action = action.clone();
            let code = factors.totp_code;
            spawn_local_scoped(async move {
                if let Err(e) = verify_step_up_totp(code.clone()).await {
                    let e = reuse_aware(e, &code);
                    if needs_enrollment(&e) {
                        finish_dialog(ctrl, done);
                        call.fail(e);
                    } else {
                        ctrl.report_error(error_copy(&e).line());
                    }
                    return;
                }
                remember_accepted(&code);
                match action().await {
                    Ok(value) => {
                        finish_dialog(ctrl, done);
                        call.succeed(value);
                    }
                    Err(e) if is_step_up_challenge(&e) => {
                        ctrl.report_error(error_copy(&e).line());
                    }
                    Err(e) => {
                        finish_dialog(ctrl, done);
                        call.fail(e);
                    }
                }
            });
        }),
    );
}

/// Collect a fresh TOTP code, then run `action(code)` (reveal / break-glass / Reveal and
/// Maintain grants).
pub fn spawn_with_fresh_totp<T, F, Fut>(ask: StepUpAsk, call: StepUpCall<T>, action: F)
where
    T: 'static,
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    call.start();
    #[cfg(feature = "e2e-lab")]
    {
        let _ = ask;
        spawn_local_scoped(async move {
            let code = match crate::e2e_lab::e2e_lab_totp_code().await {
                Ok(c) => c,
                Err(e) => {
                    call.fail(e);
                    return;
                }
            };
            match action(code.clone()).await {
                Ok(value) => {
                    remember_accepted(&code);
                    call.succeed(value);
                }
                Err(e) => call.fail(reuse_aware(e, &code)),
            }
        });
    }
    #[cfg(not(feature = "e2e-lab"))]
    {
        let Some(ctrl) = use_step_up_controller() else {
            call.fail(ServerFnError::new(
                "STEP_UP:step_up_required: step-up UI is not mounted",
            ));
            return;
        };
        let done = StoredValue::new(false);
        watch_cancel(ctrl, done, call);
        ctrl.request(
            ask.request(),
            Callback::new(move |factors: StepUpFactors| {
                let action = action.clone();
                let code = factors.totp_code;
                spawn_local_scoped(async move {
                    match action(code.clone()).await {
                        Ok(value) => {
                            remember_accepted(&code);
                            finish_dialog(ctrl, done);
                            call.succeed(value);
                        }
                        Err(e) => {
                            let e = reuse_aware(e, &code);
                            // A refused code keeps the dialog open to type the next one.
                            if e.to_string().contains("STEP_UP:") && !needs_enrollment(&e) {
                                ctrl.report_error(error_copy(&e).line());
                            } else {
                                finish_dialog(ctrl, done);
                                call.fail(e);
                            }
                        }
                    }
                });
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVALID: &str = "error running server function: STEP_UP:step_up_invalid: reason_class=step_up_invalid: invalid or reused totp code";

    #[test]
    fn same_code_refused_again_is_reuse_happy() {
        assert!(is_reused_code(Some("123456"), " 123456 ", INVALID));
    }

    #[test]
    fn different_code_or_other_error_is_not_reuse_sad() {
        assert!(!is_reused_code(Some("123456"), "654321", INVALID));
        assert!(!is_reused_code(None, "123456", INVALID));
        assert!(!is_reused_code(
            Some("123456"),
            "123456",
            "STEP_UP:step_up_rate_limited: too many"
        ));
        assert!(!is_reused_code(Some(""), "", INVALID));
    }

    #[test]
    fn reused_text_maps_to_the_reused_sentence() {
        let copy = crate::components::error_copy_text(REUSED_CODE);
        assert_eq!(copy.title, "That code was already used");
    }

    #[test]
    fn asks_name_the_action_and_pick_the_title() {
        let read = StepUpAsk::vault("read orders-db-password");
        assert_eq!(read.request().title, "Confirm it's you");
        assert_eq!(
            read.description(),
            "Type the code from your authenticator app to read orders-db-password."
        );
        assert_eq!(
            StepUpAsk::access("give Ada View on x").request().title,
            "Confirm access change"
        );
    }
}

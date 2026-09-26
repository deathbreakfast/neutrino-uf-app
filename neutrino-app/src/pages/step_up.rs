//! Retry Tier A vault mutations after a TOTP sudo window (or collect a fresh code).

use lepton_auth::actions::step_up::verify_step_up_totp;
use lepton_auth_ui::{
    use_step_up_controller, StepUpController, StepUpFactors, StepUpPolicy, StepUpRequest,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;

/// True when the server asked for (or expired) a TOTP sudo window.
#[must_use]
pub fn is_step_up_challenge(err: &impl std::fmt::Display) -> bool {
    let s = err.to_string();
    s.contains("STEP_UP:step_up_required") || s.contains("STEP_UP:step_up_expired")
}

fn default_window_request() -> StepUpRequest {
    StepUpRequest {
        title: "Confirm it's you".to_string(),
        description: Some(
            "Enter the code from your authenticator app to continue this action.".to_string(),
        ),
        policy: StepUpPolicy::Totp,
    }
}

fn default_fresh_request() -> StepUpRequest {
    StepUpRequest {
        title: "Confirm secret reveal".to_string(),
        description: Some(
            "Revealing a secret needs a fresh authenticator code for this call.".to_string(),
        ),
        policy: StepUpPolicy::Totp,
    }
}

/// Run `action`; on a window challenge, open step-up, verify, then retry once.
pub fn spawn_with_step_up<T, F, Fut>(error: RwSignal<Option<String>>, on_ok: Callback<T>, action: F)
where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let step_up = use_step_up_controller();
    spawn_local_scoped(async move {
        match action().await {
            Ok(value) => on_ok.run(value),
            Err(err) if is_step_up_challenge(&err) => {
                let Some(ctrl) = step_up else {
                    error.set(Some(err.to_string()));
                    return;
                };
                request_window_then_retry(ctrl, error, on_ok, action);
            }
            Err(err) => error.set(Some(err.to_string())),
        }
    });
}

fn request_window_then_retry<T, F, Fut>(
    ctrl: StepUpController,
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    ctrl.request(
        default_window_request(),
        Callback::new(move |factors: StepUpFactors| {
            let action = action.clone();
            spawn_local_scoped(async move {
                if let Err(e) = verify_step_up_totp(factors.totp_code).await {
                    ctrl.report_error(e.to_string());
                    return;
                }
                match action().await {
                    Ok(value) => {
                        ctrl.complete_success();
                        on_ok.run(value);
                    }
                    Err(e) => {
                        if is_step_up_challenge(&e) {
                            ctrl.report_error(e.to_string());
                        } else {
                            ctrl.complete_success();
                            error.set(Some(e.to_string()));
                        }
                    }
                }
            });
        }),
    );
}

/// Collect a fresh TOTP code, then run `action(code)` (reveal / break-glass).
pub fn spawn_with_fresh_totp<T, F, Fut>(
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    #[cfg(feature = "e2e-lab")]
    {
        let _ = default_fresh_request;
        let _ = use_step_up_controller;
        spawn_local_scoped(async move {
            let code = match crate::e2e_lab::e2e_lab_totp_code().await {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e.to_string()));
                    return;
                }
            };
            match action(code).await {
                Ok(value) => on_ok.run(value),
                Err(e) => error.set(Some(e.to_string())),
            }
        });
        return;
    }
    #[cfg(not(feature = "e2e-lab"))]
    {
        let Some(ctrl) = use_step_up_controller() else {
            error.set(Some(
                "STEP_UP:step_up_required: step-up UI is not mounted".to_string(),
            ));
            return;
        };
        ctrl.request(
            default_fresh_request(),
            Callback::new(move |factors: StepUpFactors| {
                let action = action.clone();
                spawn_local_scoped(async move {
                    match action(factors.totp_code).await {
                        Ok(value) => {
                            ctrl.complete_success();
                            on_ok.run(value);
                        }
                        Err(e) => ctrl.report_error(e.to_string()),
                    }
                });
            }),
        );
    }
}

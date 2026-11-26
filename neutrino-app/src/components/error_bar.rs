//! Turn server-fn errors into a title, a sentence and one thing to do next.
//!
//! Server errors carry a stable leading tag: Gauge's `permission_denied::…`,
//! Lepton's `STEP_UP:…` reason, or one of [`crate::server::error_tags`]. The
//! raw text never reaches the screen; anything unrecognised gets a generic line.

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use uf_product::primitives::*;
use uf_product::services::permission_server_errors::{
    parse_permission_server_error, PermissionServerError,
};

use crate::server::{error_tags, request_neutrino_permission};

/// How loud the bar is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTone {
    /// Something failed.
    Error,
    /// Nothing failed yet, but the action can't go ahead as things stand.
    Warning,
    /// Nothing changed and nothing is wrong (for example, a dialog was closed).
    Info,
}

impl ErrorTone {
    const fn intent(self) -> MessageBarIntent {
        match self {
            Self::Error => MessageBarIntent::Error,
            Self::Warning => MessageBarIntent::Warning,
            Self::Info => MessageBarIntent::Info,
        }
    }
}

/// The one thing the bar offers to do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorAction {
    /// Run the same call again (the page passes the retry).
    TryAgain,
    /// Ask for the named app-wide permission through Gauge.
    RequestAccess(String),
    /// Go to Account settings to set up an authenticator app.
    OpenAccountSettings,
    /// Nothing to press; the sentence says what to do.
    None,
}

/// What a page shows for one failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCopy {
    /// Short headline, no full stop.
    pub title: String,
    /// One or two sentences saying what happened and what to do.
    pub sentence: String,
    /// Bar intent.
    pub tone: ErrorTone,
    /// The button the bar shows, if any.
    pub action: ErrorAction,
}

impl ErrorCopy {
    fn new(title: &str, sentence: impl Into<String>, action: ErrorAction) -> Self {
        Self {
            title: title.to_string(),
            sentence: sentence.into(),
            tone: ErrorTone::Error,
            action,
        }
    }

    /// Title and sentence as one line, for surfaces that take a single string
    /// (Lepton's step-up dialog).
    #[must_use]
    pub fn line(&self) -> String {
        format!("{}. {}", self.title, self.sentence)
    }
}

/// Neutrino's own permission names, which Request access can ask for.
fn requestable(permission: &str) -> bool {
    matches!(
        permission,
        "SecretsReveal" | "SecretsWrite" | "SecretsRotate"
    )
}

/// The Lepton step-up reason after `STEP_UP:`, up to the next `:`.
fn step_up_reason(raw: &str) -> Option<&str> {
    let rest = raw.split("STEP_UP:").nth(1)?;
    Some(rest.split([':', ' ']).next().unwrap_or(""))
}

/// The text after `tag`, wherever the tag sits in `raw`.
fn after_tag<'a>(raw: &'a str, tag: &str) -> Option<&'a str> {
    raw.find(tag).map(|i| raw[i + tag.len()..].trim())
}

/// Explain a server-fn error.
#[must_use]
pub fn error_copy(err: &ServerFnError) -> ErrorCopy {
    error_copy_text(&err.to_string())
}

/// Explain the text of a server-fn error (as `ServerFnError::to_string` gives it).
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per known failure, kept in one table
pub fn error_copy_text(raw: &str) -> ErrorCopy {
    if raw.contains("error reaching server") {
        return ErrorCopy::new(
            "Couldn't reach Neutrino",
            "Check your connection, then try again.",
            ErrorAction::TryAgain,
        );
    }
    if let Some(reason) = step_up_reason(raw) {
        return match reason {
            "step_up_reused" => ErrorCopy::new(
                "That code was already used",
                "Each code works once. Wait for the next one in your authenticator app, then type it here.",
                ErrorAction::None,
            ),
            "step_up_invalid" | "invalid_totp" => ErrorCopy::new(
                "That code didn't work",
                "Codes change every 30 seconds. Type the newest one from your authenticator app.",
                ErrorAction::None,
            ),
            "step_up_rate_limited" => ErrorCopy::new(
                "Too many codes tried",
                "Wait a few minutes, then try again with the newest code from your authenticator app.",
                ErrorAction::None,
            ),
            "totp_enrollment_required" => ErrorCopy {
                tone: ErrorTone::Warning,
                ..ErrorCopy::new(
                    "Set up an authenticator app first",
                    "This needs a code from an authenticator app, and you haven't set one up. Set one up in Account settings, then come back. This takes about a minute.",
                    ErrorAction::OpenAccountSettings,
                )
            },
            "auth_required" => ErrorCopy::new(
                "Sign in again",
                "Your session ended. Sign in, then try again.",
                ErrorAction::None,
            ),
            "step_up_required" | "step_up_expired" => ErrorCopy::new(
                "Confirm it's you first",
                "This needs a code from your authenticator app. Try again and type the code when asked.",
                ErrorAction::TryAgain,
            ),
            _ => ErrorCopy::new(
                "Couldn't check your code",
                "Something went wrong while checking it. Nothing changed. Try again.",
                ErrorAction::TryAgain,
            ),
        };
    }
    match parse_permission_server_error(raw) {
        Some(PermissionServerError::Denied { permission }) => {
            let action = if requestable(&permission) {
                ErrorAction::RequestAccess(permission.clone())
            } else {
                ErrorAction::None
            };
            return ErrorCopy::new(
                &format!("This needs {permission}"),
                format!(
                    "Your role doesn't include {permission}. Request it, or ask someone who has it to do this for you."
                ),
                action,
            );
        }
        Some(PermissionServerError::CheckFailed { .. }) => {
            return ErrorCopy::new(
                "Couldn't check your access",
                "Neutrino couldn't check your permissions just now. Nothing changed. Try again.",
                ErrorAction::TryAgain,
            );
        }
        None => {}
    }
    if raw.contains(error_tags::ACCESS_DENIED) {
        return ErrorCopy::new(
            "You don't have access to this secret",
            "Ask the secret's owner, or someone who manages its access, to give it to you.",
            ErrorAction::None,
        );
    }
    if raw.contains(error_tags::NOT_FOUND) {
        return ErrorCopy::new(
            "That secret isn't there",
            "It may have been deleted, or you can't see it. Refresh the list.",
            ErrorAction::None,
        );
    }
    if let Some(sentence) = after_tag(raw, error_tags::INVALID_INPUT) {
        return ErrorCopy::new("Check what you typed", sentence, ErrorAction::None);
    }
    if let Some(rest) = after_tag(raw, error_tags::INVALID_STATE) {
        // Neutrino's text starts "neutrino {operation}: …"; keep only the sentence.
        let sentence = rest
            .strip_prefix("neutrino ")
            .and_then(|r| r.split_once(": ").map(|(_, s)| s))
            .unwrap_or(rest);
        return ErrorCopy::new("That can't be done right now", sentence, ErrorAction::None);
    }
    if raw.contains(error_tags::GAUGE_FAILED) {
        return ErrorCopy::new(
            "Gauge didn't save that",
            "Nothing changed. Try again.",
            ErrorAction::TryAgain,
        );
    }
    if raw.contains(error_tags::SIGN_IN_REQUIRED) {
        return ErrorCopy::new(
            "Sign in again",
            "Your session ended. Sign in, then try again.",
            ErrorAction::None,
        );
    }
    if raw.contains(error_tags::VAULT_INTERNAL) {
        return ErrorCopy::new(
            "Neutrino couldn't finish that",
            "Something went wrong on the server. Try again in a moment.",
            ErrorAction::TryAgain,
        );
    }
    ErrorCopy::new(
        "Something went wrong",
        "Neutrino couldn't finish that. Try again.",
        ErrorAction::TryAgain,
    )
}

/// Error bar for a page or dialog: nothing while `error` is `None`.
///
/// `on_retry` backs the Try again button; without it the button doesn't show.
#[component]
pub fn ErrorBar(
    /// The failure to explain.
    #[prop(into)]
    error: Signal<Option<ServerFnError>>,
    /// Runs the failed call again.
    #[prop(optional)]
    on_retry: Option<Callback<()>>,
    /// `data-testid` for the wrapper.
    #[prop(optional, into)]
    testid: Option<String>,
) -> impl IntoView {
    move || {
        error.get().map(|err| {
            let testid = testid.clone();
            let copy = error_copy(&err);
            let bar = match on_retry {
                Some(retry) => view! { <ErrorCopyBar copy=copy on_retry=retry /> }.into_any(),
                None => view! { <ErrorCopyBar copy=copy /> }.into_any(),
            };
            view! { <div data-testid=testid>{bar}</div> }
        })
    }
}

/// Draw one [`ErrorCopy`] as a message bar with its action.
#[component]
pub fn ErrorCopyBar(
    /// What to say.
    copy: ErrorCopy,
    /// Runs the failed call again (shown only for [`ErrorAction::TryAgain`]).
    #[prop(optional)]
    on_retry: Option<Callback<()>>,
) -> impl IntoView {
    let action = match copy.action {
        ErrorAction::TryAgain => on_retry.map(|retry| {
            view! {
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=Callback::new(move |_| retry.run(()))
                >
                    "Try again"
                </Button>
            }
            .into_any()
        }),
        ErrorAction::RequestAccess(permission) => {
            Some(view! { <RequestAccessButton permission=permission /> }.into_any())
        }
        ErrorAction::OpenAccountSettings => Some(
            view! {
                <Link href=lepton_auth::paths::USER_ACCOUNT_SETTINGS.to_string()>
                    "Open Account settings"
                </Link>
            }
            .into_any(),
        ),
        ErrorAction::None => None,
    };
    view! {
        <MessageBar intent=copy.tone.intent() layout=MessageBarLayout::Multiline>
            <MessageBarBody>
                <MessageBarTitle>{copy.title}</MessageBarTitle>
                {copy.sentence}
            </MessageBarBody>
            {action.map(|a| view! { <MessageBarActions>{a}</MessageBarActions> })}
        </MessageBar>
    }
}

/// Request access to one of Neutrino's permissions through Gauge, then say it was sent.
#[component]
pub fn RequestAccessButton(
    /// A `Secrets*` permission name.
    permission: String,
) -> impl IntoView {
    let sending = RwSignal::new(false);
    let sent = RwSignal::new(false);
    let failed = RwSignal::new(Option::<ErrorCopy>::None);
    let on_click = Callback::new(move |_| {
        let permission = permission.clone();
        sending.set(true);
        failed.set(None);
        spawn_local_scoped(async move {
            match request_neutrino_permission(permission).await {
                Ok(()) => sent.set(true),
                Err(e) => failed.set(Some(error_copy(&e))),
            }
            sending.set(false);
        });
    });
    view! {
        <div data-testid="neutrino-request-access">
            <Show
                when=move || sent.get()
                fallback=move || view! {
                    <Flex vertical=true gap=FlexGap::Small>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=Signal::derive(move || sending.get())
                            on_click=on_click
                        >
                            {move || if sending.get() { "Sending…" } else { "Request access" }}
                        </Button>
                        {move || failed.get().map(|copy| view! {
                            <Caption2>{copy.line()}</Caption2>
                        })}
                    </Flex>
                }
            >
                <span data-testid="neutrino-request-access-sent">
                    <Caption2>"Request sent. You'll get a notification when someone answers."</Caption2>
                </span>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_PREFIX: &str = "error running server function: ";

    fn copy(raw: &str) -> ErrorCopy {
        error_copy_text(&format!("{RAW_PREFIX}{raw}"))
    }

    #[test]
    fn step_up_reasons_map_to_sentences() {
        assert_eq!(
            copy("STEP_UP:step_up_reused: that code was already used").title,
            "That code was already used"
        );
        let wrong = copy(
            "STEP_UP:step_up_invalid: reason_class=step_up_invalid: invalid or reused totp code",
        );
        assert_eq!(wrong.title, "That code didn't work");
        assert!(wrong.sentence.starts_with("Codes change every 30 seconds"));
        let enroll = copy("STEP_UP:totp_enrollment_required: enroll totp in account settings");
        assert_eq!(enroll.tone, ErrorTone::Warning);
        assert_eq!(enroll.action, ErrorAction::OpenAccountSettings);
        assert_eq!(
            copy("STEP_UP:step_up_required: recent totp verification required").action,
            ErrorAction::TryAgain
        );
        assert_eq!(
            copy("STEP_UP:step_up_rate_limited: x").title,
            "Too many codes tried"
        );
        assert_eq!(copy("STEP_UP:store: x").title, "Couldn't check your code");
    }

    #[test]
    fn permission_denied_offers_request_access_for_neutrino_permissions() {
        let write = copy("permission_denied::SecretsWrite");
        assert_eq!(write.title, "This needs SecretsWrite");
        assert_eq!(
            write.action,
            ErrorAction::RequestAccess("SecretsWrite".into())
        );
        let other = copy("permission_denied::counter.admin.set");
        assert_eq!(other.action, ErrorAction::None);
        assert_eq!(
            copy("permission_check_failed::SecretsRead::backend down").action,
            ErrorAction::TryAgain
        );
    }

    #[test]
    fn vault_tags_map_to_sentences() {
        let invalid = copy("invalid_input::Name is required.");
        assert_eq!(invalid.title, "Check what you typed");
        assert_eq!(invalid.sentence, "Name is required.");
        let state = copy("invalid_state::neutrino lease: archived only");
        assert_eq!(state.sentence, "archived only");
        assert_eq!(
            copy("access_denied::not authorized to reveal").action,
            ErrorAction::None
        );
        assert_eq!(
            copy("not_found::secret not found: s1").title,
            "That secret isn't there"
        );
        assert_eq!(
            copy("gauge_failed::grant secret action").title,
            "Gauge didn't save that"
        );
        assert_eq!(
            copy("vault_internal::An internal vault error occurred. Check server logs.").title,
            "Neutrino couldn't finish that"
        );
        assert_eq!(copy("sign_in_required::x").title, "Sign in again");
    }

    #[test]
    fn unknown_and_network_errors_never_show_raw_text() {
        let unknown = copy("db exploded at /var/lib/secret.sqlite");
        assert_eq!(unknown.title, "Something went wrong");
        assert!(!unknown.line().contains("sqlite"));
        let network = error_copy_text("error reaching server to call server function: TypeError");
        assert_eq!(network.title, "Couldn't reach Neutrino");
    }

    #[test]
    fn no_copy_leaks_raw_tags() {
        let raws = [
            "STEP_UP:step_up_reused: x",
            "STEP_UP:step_up_invalid: x",
            "STEP_UP:step_up_required: x",
            "STEP_UP:totp_enrollment_required: x",
            "permission_denied::SecretsWrite",
            "permission_check_failed::SecretsRead::boom",
            "access_denied::not authorized to reveal",
            "not_found::secret not found: s1",
            "invalid_state::neutrino rotate: archived",
            "gauge_failed::grant",
            "vault_internal::boom",
            "something else",
        ];
        for raw in raws {
            let line = copy(raw).line();
            for leak in [
                "STEP_UP:",
                "permission_denied::",
                "error running server function:",
                "::",
            ] {
                assert!(!line.contains(leak), "{raw} -> {line}");
            }
        }
    }
}

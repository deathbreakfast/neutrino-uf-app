//! Read value, Rotate and Delete dialogs, shared by the secrets list and the
//! secret's own page.
//!
//! Each dialog says what the action does before it runs: Read value shows the
//! value as text for a minute, Rotate says what happens to the old version,
//! and Delete lists what goes and asks for the name.

use std::time::Duration;

use base64::Engine as _;
use leptos::prelude::*;
use uf_product::components::{Caption1, SpacingSize};
use uf_product::primitives::*;

use crate::components::{ErrorBar, KeepsNotice, KeepsWhat};
use crate::pages::step_up::{spawn_with_step_up, StepUpAsk, StepUpCall};
use crate::server::{
    delete_vault_secret, renotify_rotated_secret, rotate_vault_secret, RotateOutcome,
    VaultSecretRow,
};

/// How long a read value stays on screen.
const HIDE_AFTER_SECS: u32 = 60;

/// A read value, as the dialog shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ReadValue {
    /// UTF-8 text, shown as is.
    Text(String),
    /// Bytes that aren't text, shown as base64.
    Binary(String),
}

impl ReadValue {
    /// Decode the server's base64 payload; text when the bytes are UTF-8.
    fn from_b64(b64: &str) -> Self {
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map_or_else(|| Self::Binary(b64.to_string()), Self::Text)
    }

    fn shown(&self) -> &str {
        match self {
            Self::Text(s) | Self::Binary(s) => s,
        }
    }
}

/// "Hides in N seconds. Closing clears it."
fn hides_in(seconds: u32) -> String {
    match seconds {
        1 => "Hides in 1 second. Closing clears it.".to_string(),
        n => format!("Hides in {n} seconds. Closing clears it."),
    }
}

/// True when the typed confirmation matches the secret's name exactly.
fn delete_name_matches(typed: &str, name: &str) -> bool {
    !name.is_empty() && typed == name
}

/// Read value: the value as text with Copy, hidden again after 60 seconds.
///
/// The page runs the read (it needs a fresh code) and fills `value_b64`; the
/// dialog clears it on Hide now, on Done, on any close and when the timer ends.
#[component]
#[allow(clippy::too_many_lines)] // value, timer, hidden and binary states in one dialog
pub fn ReadValueDialog(
    /// Open flag.
    open: RwSignal<bool>,
    /// The secret being read; `None` while a Help tour shows the empty dialog.
    target: RwSignal<Option<VaultSecretRow>>,
    /// Base64 payload from the read; empty until it arrives.
    value_b64: RwSignal<String>,
    /// True while the read runs.
    loading: RwSignal<bool>,
    /// Why the read failed.
    error: RwSignal<Option<ServerFnError>>,
    /// Read the value again (another recorded read).
    on_read_again: Callback<VaultSecretRow>,
) -> impl IntoView {
    let hidden = RwSignal::new(false);
    let seconds_left = RwSignal::new(HIDE_AFTER_SECS);
    let show_b64 = RwSignal::new(true);
    let timer = StoredValue::new(None::<IntervalHandle>);

    let stop_timer = move || {
        if let Some(handle) = timer.get_value() {
            handle.clear();
        }
        timer.set_value(None);
    };
    let hide = move || {
        stop_timer();
        value_b64.update(String::clear);
        hidden.set(true);
    };

    // Start the countdown when a value arrives.
    Effect::new(move |_| {
        if value_b64.with(String::is_empty) {
            return;
        }
        stop_timer();
        hidden.set(false);
        seconds_left.set(HIDE_AFTER_SECS);
        let handle = set_interval_with_handle(
            move || {
                let left = seconds_left.get_untracked().saturating_sub(1);
                seconds_left.set(left);
                if left == 0 {
                    hide();
                }
            },
            Duration::from_secs(1),
        );
        timer.set_value(handle.ok());
    });
    // Any close (Done, Escape, backdrop) clears the value.
    Effect::new(move |_| {
        if !open.get() {
            stop_timer();
            value_b64.update(String::clear);
            hidden.set(false);
        }
    });
    on_cleanup(stop_timer);

    let value = Memo::new(move |_| {
        value_b64.with(|b64| (!b64.is_empty()).then(|| ReadValue::from_b64(b64)))
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-reveal-secret-dialog">
                <DialogBody>
                    <DialogTitle>
                        {move || target.get().map_or_else(|| "Read value".to_string(), |t| t.name)}
                    </DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                            {move || target.get().map(|t| view! {
                                <Flex vertical=true gap=FlexGap::Size(0)>
                                    <Caption1>{t.scope_path}</Caption1>
                                    <Caption1>{format!("Version {}", t.current_version)}</Caption1>
                                </Flex>
                            })}
                            {move || loading.get().then(|| view! { <Spinner /> })}
                            <ErrorBar error=error testid="neutrino-reveal-error" />
                            // Spotlight cutout uses the HTML id (always mounted). The test id
                            // is only there with a value, so denied reads can assert its absence.
                            <div id="secrets-reveal-plaintext">
                                {move || match value.get() {
                                    Some(read) => {
                                        let shown = read.shown().to_string();
                                        let copy_text = shown.clone();
                                        let binary = matches!(read, ReadValue::Binary(_));
                                        view! {
                                            <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                                                <Card>
                                                    <CardContent>
                                                        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                                                            <span data-testid="neutrino-reveal-plaintext">
                                                                {move || {
                                                                    let text = shown.clone();
                                                                    if !binary || show_b64.get() {
                                                                        view! { <Text font=TextFont::Monospace block=true>{text}</Text> }.into_any()
                                                                    } else {
                                                                        view! { <Caption1>"This value can't be shown as text."</Caption1> }.into_any()
                                                                    }
                                                                }}
                                                            </span>
                                                            <span data-testid="neutrino-reveal-copy">
                                                                <CopyButton text=copy_text />
                                                            </span>
                                                        </Flex>
                                                    </CardContent>
                                                </Card>
                                                {binary.then(|| view! {
                                                    <div data-testid="neutrino-reveal-base64">
                                                        <Switch bind=show_b64 label="Show as base64" />
                                                        <Caption1>"This value isn't text, so it's shown as base64."</Caption1>
                                                    </div>
                                                })}
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <ProgressBar value=Signal::derive(move || {
                                                        f64::from(seconds_left.get()) / f64::from(HIDE_AFTER_SECS)
                                                    }) />
                                                    <span data-testid="neutrino-reveal-timer">
                                                        <Caption1>{move || hides_in(seconds_left.get())}</Caption1>
                                                    </span>
                                                </Flex>
                                            </Flex>
                                        }.into_any()
                                    }
                                    None if hidden.get() => view! {
                                        <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                                            <Caption1>"Hidden. Read it again to see the value; that read is recorded too."</Caption1>
                                            <div data-testid="neutrino-reveal-again">
                                                <Button
                                                    appearance=ButtonAppearance::Secondary
                                                    on_click=Callback::new(move |_| {
                                                        if let Some(t) = target.get_untracked() {
                                                            hidden.set(false);
                                                            on_read_again.run(t);
                                                        }
                                                    })
                                                >
                                                    "Read again"
                                                </Button>
                                            </div>
                                        </Flex>
                                    }.into_any(),
                                    None => view! {
                                        <Show when=move || !loading.get() && error.with(Option::is_none)>
                                            <Caption1>"The value appears here after the read."</Caption1>
                                        </Show>
                                    }.into_any(),
                                }}
                            </div>
                            <KeepsNotice what=KeepsWhat::Read />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <div data-testid="neutrino-reveal-hide">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=Callback::new(move |_| {
                                    if value_b64.with_untracked(String::is_empty) {
                                        open.set(false);
                                    } else {
                                        hide();
                                    }
                                })
                            >
                                {move || if value.with(Option::is_some) { "Hide now" } else { "Close" }}
                            </Button>
                        </div>
                        <div id="secrets-reveal-close" data-testid="neutrino-reveal-close">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=Callback::new(move |_| open.set(false))
                            >
                                "Done"
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

/// Rotate: store a new version, saying what happens to the old one and to apps.
///
/// Cancel and Rotate both disable while the call runs. Success closes the
/// dialog and runs `on_rotated` with the secret and the outcome.
#[component]
#[allow(clippy::too_many_lines)] // copy, field, validation and actions in one dialog
pub fn RotateDialog(
    /// Open flag.
    open: RwSignal<bool>,
    /// The secret to rotate; `None` while a Help tour shows the empty dialog.
    target: RwSignal<Option<VaultSecretRow>>,
    /// Runs after a version is saved (check the outcome's notify error).
    on_rotated: Callback<(VaultSecretRow, RotateOutcome)>,
) -> impl IntoView {
    let new_value = RwSignal::new(String::new());
    let tried = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<ServerFnError>::None);

    let reset = move || {
        new_value.set(String::new());
        tried.set(false);
        error.set(None);
    };
    // Clear on close, so the next open starts clean without racing the first keystroke.
    Effect::new(move |_| {
        if !open.get() {
            reset();
        }
    });

    let on_submit = Callback::new(move |_| {
        let Some(row) = target.get_untracked() else {
            return;
        };
        tried.set(true);
        let value = new_value.get_untracked();
        if value.is_empty() {
            return;
        }
        let rotated = row.clone();
        let call = StepUpCall::new(
            error,
            busy,
            Callback::new(move |outcome: RotateOutcome| {
                open.set(false);
                new_value.set(String::new());
                on_rotated.run((rotated.clone(), outcome));
            }),
        );
        spawn_with_step_up(
            StepUpAsk::vault(format!("rotate {}", row.name)),
            call,
            move || {
                let id = row.id.clone();
                let value = value.clone();
                async move { rotate_vault_secret(id, value).await }
            },
        );
    });

    let next_version = move || target.with(|t| t.as_ref().map(|t| t.current_version + 1));

    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-rotate-secret-dialog">
                <DialogBody>
                    <DialogTitle>
                        {move || target.get().map_or_else(|| "Rotate".to_string(), |t| format!("Rotate {}", t.name))}
                    </DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                            {move || target.get().map(|t| view! {
                                <Body1>{format!(
                                    "Version {} replaces version {}. Apps get the new value the next time they read it.",
                                    t.current_version + 1,
                                    t.current_version,
                                )}</Body1>
                            })}
                            <Field label="New value" required=true>
                                <div id="secrets-rotate-plaintext" data-testid="neutrino-rotate-plaintext">
                                    <Input
                                        bind=new_value
                                        appearance=InputAppearance {
                                            input_type: Signal::from(InputType::Password),
                                            disabled: busy.into(),
                                            ..Default::default()
                                        }
                                    />
                                </div>
                                {move || (tried.get() && new_value.with(String::is_empty)).then(|| view! {
                                    <div data-testid="neutrino-rotate-validation">
                                        <FieldValidationMessage state=FieldValidationState::Error(
                                            "Paste the new value.".to_string(),
                                        ) />
                                    </div>
                                })}
                            </Field>
                            {move || target.get().map(|t| view! {
                                <Caption1>{format!("Version {} stays in the history, sealed.", t.current_version)}</Caption1>
                            })}
                            <ErrorBar error=error testid="neutrino-rotate-error" />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <div data-testid="neutrino-rotate-cancel">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                disabled=busy
                                on_click=Callback::new(move |_| {
                                    open.set(false);
                                    reset();
                                })
                            >
                                "Cancel"
                            </Button>
                        </div>
                        <div id="secrets-rotate-submit" data-testid="neutrino-rotate-submit">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=on_submit
                                disabled=busy
                            >
                                {move || match (busy.get(), next_version()) {
                                    (true, _) => "Rotating…".to_string(),
                                    (false, Some(n)) => format!("Rotate to version {n}"),
                                    (false, None) => "Rotate".to_string(),
                                }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

/// A rotate that saved a version but couldn't tell Nucleus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotifyFailed {
    /// Secret id, for the resend.
    pub secret_id: String,
    /// Secret name, for the code dialog.
    pub name: String,
    /// The version that was saved.
    pub version: i64,
}

/// Page bar after a DB-scoped rotate whose notice to Nucleus failed: the version
/// is saved, and Send notice again retries the notice (not the rotate).
#[component]
pub fn NotifyFailedBar(
    /// The failed notice; the bar hides once it is `None`.
    notice: RwSignal<Option<NotifyFailed>>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<ServerFnError>::None);
    let on_resend = Callback::new(move |_| {
        let Some(failed) = notice.get_untracked() else {
            return;
        };
        let call = StepUpCall::new(error, busy, Callback::new(move |()| notice.set(None)));
        let secret_id = failed.secret_id.clone();
        spawn_with_step_up(
            StepUpAsk::vault(format!("send the notice for {} again", failed.name)),
            call,
            move || renotify_rotated_secret(secret_id.clone()),
        );
    });
    move || {
        notice.get().map(|failed| {
            view! {
                <div data-testid="neutrino-notify-failed">
                    <Flex vertical=true gap=FlexGap::Small>
                        <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                            <MessageBarBody>
                                <MessageBarTitle>{format!(
                                    "Version {} is saved, but Nucleus hasn't been told",
                                    failed.version,
                                )}</MessageBarTitle>
                                {format!(
                                    "The database keeps using version {} until Nucleus applies the new password. Send the notice again; rotating again would store another version.",
                                    failed.version - 1,
                                )}
                            </MessageBarBody>
                            <MessageBarActions>
                                <div data-testid="neutrino-send-notice-again">
                                    <Button
                                        appearance=ButtonAppearance::Secondary
                                        disabled=busy
                                        on_click=on_resend
                                    >
                                        {move || if busy.get() { "Sending…" } else { "Send notice again" }}
                                    </Button>
                                </div>
                            </MessageBarActions>
                        </MessageBar>
                        <ErrorBar error=error />
                    </Flex>
                </div>
            }
        })
    }
}

/// Delete: list what goes and what stays, ask for the name, then delete.
///
/// The Danger button stays disabled until the typed name matches.
#[component]
#[allow(clippy::too_many_lines)] // list, notice, field and actions in one dialog
pub fn DeleteDialog(
    /// Open flag.
    open: RwSignal<bool>,
    /// The secret to delete; `None` while a Help tour shows the empty dialog.
    target: RwSignal<Option<VaultSecretRow>>,
    /// Runs after the secret is deleted.
    on_deleted: Callback<()>,
) -> impl IntoView {
    let typed = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<ServerFnError>::None);
    Effect::new(move |_| {
        if !open.get() {
            typed.set(String::new());
            error.set(None);
        }
    });

    let name_matches = Signal::derive(move || {
        target.with(|t| {
            t.as_ref()
                .is_some_and(|t| typed.with(|typed| delete_name_matches(typed, &t.name)))
        })
    });

    let on_confirm = Callback::new(move |_| {
        let Some(row) = target.get_untracked() else {
            return;
        };
        if !name_matches.get_untracked() {
            return;
        }
        let call = StepUpCall::new(
            error,
            busy,
            Callback::new(move |()| {
                open.set(false);
                on_deleted.run(());
            }),
        );
        spawn_with_step_up(
            StepUpAsk::vault(format!("delete {}", row.name)),
            call,
            move || delete_vault_secret(row.id.clone()),
        );
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-delete-secret-dialog">
                <DialogBody>
                    <DialogTitle>
                        {move || target.get().map_or_else(|| "Delete secret?".to_string(), |t| format!("Delete {}?", t.name))}
                    </DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                            {move || target.get().map(|t| view! {
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Text weight=TextWeight::Semibold>"This can't be undone. It removes:"</Text>
                                    <ul data-testid="neutrino-delete-loses">
                                        <li>{format!("Every version ({}) and its value", t.current_version)}</li>
                                        <li>"Its access grants"</li>
                                        <li>"Apps reading "<Code text=t.scope_path />" stop getting a value"</li>
                                    </ul>
                                </Flex>
                            })}
                            <KeepsNotice what=KeepsWhat::Delete />
                            <Field
                                label=Signal::derive(move || {
                                    target.with(|t| t.as_ref().map_or_else(
                                        || "Type the secret's name to confirm".to_string(),
                                        |t| format!("Type {} to confirm", t.name),
                                    ))
                                })
                                required=true
                            >
                                <div data-testid="neutrino-delete-type">
                                    <Input bind=typed appearance=InputAppearance {
                                        disabled: busy.into(),
                                        ..Default::default()
                                    } />
                                </div>
                            </Field>
                            <ErrorBar error=error testid="neutrino-delete-error" />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <div data-testid="neutrino-delete-cancel">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                disabled=busy
                                on_click=Callback::new(move |_| open.set(false))
                            >
                                "Cancel"
                            </Button>
                        </div>
                        <div id="secrets-delete-confirm" data-testid="neutrino-delete-confirm">
                            <Button
                                appearance=ButtonAppearance::Danger
                                on_click=on_confirm
                                disabled=Signal::derive(move || busy.get() || !name_matches.get())
                            >
                                {move || if busy.get() { "Deleting…" } else { "Delete secret" }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_values_read_as_text_happy() {
        let b64 = base64::engine::general_purpose::STANDARD.encode("v1-plaintext");
        assert_eq!(
            ReadValue::from_b64(&b64),
            ReadValue::Text("v1-plaintext".into())
        );
    }

    #[test]
    fn non_utf8_values_stay_base64_sad() {
        let b64 = base64::engine::general_purpose::STANDARD.encode([0xff_u8, 0xfe, 0x00]);
        assert_eq!(ReadValue::from_b64(&b64), ReadValue::Binary(b64.clone()));
        assert_eq!(
            ReadValue::from_b64("not base64!"),
            ReadValue::Binary("not base64!".into())
        );
    }

    #[test]
    fn delete_needs_the_exact_name() {
        assert!(delete_name_matches(
            "orders-db-password",
            "orders-db-password"
        ));
        assert!(!delete_name_matches("orders-db", "orders-db-password"));
        assert!(!delete_name_matches(
            " orders-db-password",
            "orders-db-password"
        ));
        assert!(!delete_name_matches("", ""));
    }

    #[test]
    fn timer_line_counts_down() {
        assert_eq!(hides_in(42), "Hides in 42 seconds. Closing clears it.");
        assert_eq!(hides_in(1), "Hides in 1 second. Closing clears it.");
    }
}

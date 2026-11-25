//! Reveal action and dialog spotlight steps (`/secrets`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::{help_stack, SecretsTourDialogs};

fn open_reveal_teaching() {
    if let Some(dialogs) = use_context::<SecretsTourDialogs>() {
        dialogs.reveal_open.set(true);
    }
}

/// Reveal row action button.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-reveal",
    title = "Reveal a value",
    spotlight = "secrets-action-reveal",
    position = "left",
    order = 160
)]
#[component]
pub fn SecretsRevealHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-reveal",
        "Reveal opens an audited dialog with the current value. Next we look inside that dialog.",
        None,
        &[],
    )
}

/// Reveal dialog value region (may be empty during tour; no auto-reveal API call).
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-reveal-value",
    title = "The value",
    spotlight = "secrets-reveal-plaintext",
    position = "top",
    order = 170
)]
#[component]
pub fn SecretsRevealValueHelp() -> impl IntoView {
    Effect::new(move |_| {
        open_reveal_teaching();
    });
    help_stack(
        "help-step-secrets-reveal-value",
        "The value shows here as text for a minute, then hides. Copy it if you need it.",
        Some(
            "This tour opens the dialog for teaching only; it does not fetch plaintext by itself.",
        ),
        &[],
    )
}

/// Reveal dialog Close.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-reveal-close",
    title = "Done",
    spotlight = "secrets-reveal-close",
    position = "top",
    order = 180
)]
#[component]
pub fn SecretsRevealCloseHelp() -> impl IntoView {
    Effect::new(move |_| {
        open_reveal_teaching();
    });
    help_stack(
        "help-step-secrets-reveal-close",
        "Done closes the dialog and clears the value from the screen.",
        None,
        &[],
    )
}

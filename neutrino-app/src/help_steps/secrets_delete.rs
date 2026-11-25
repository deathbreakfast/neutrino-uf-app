//! Delete action and dialog spotlight steps (`/secrets`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::{help_stack, SecretsTourDialogs};

fn open_delete_teaching() {
    if let Some(dialogs) = use_context::<SecretsTourDialogs>() {
        dialogs.delete_open.set(true);
    }
}

/// Delete row action button.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-delete",
    title = "Delete a secret",
    spotlight = "secrets-action-delete",
    position = "left",
    order = 220
)]
#[component]
pub fn SecretsDeleteHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-delete",
        "Delete asks you to confirm removing this entry. Prefer Rotate when apps still need a value under the same name.",
        None,
        &[],
    )
}

/// Delete dialog confirm.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-delete-confirm",
    title = "Confirm delete",
    spotlight = "secrets-delete-confirm",
    position = "top",
    order = 230
)]
#[component]
pub fn SecretsDeleteConfirmHelp() -> impl IntoView {
    Effect::new(move |_| {
        open_delete_teaching();
    });
    help_stack(
        "help-step-secrets-delete-confirm",
        "Type the secret's name, then Delete secret removes it and every version. Cancel keeps it.",
        None,
        &[],
    )
}

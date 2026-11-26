//! Spotlight steps for the Access page (`/secrets/acl`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Centered Access page intro.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-intro",
    title = "Share one secret at a time",
    order = 10
)]
#[component]
pub fn SecretsAclIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-intro",
        "Give someone access to one secret without opening the whole vault.",
        Some("Pick a secret, see who can see it, read it, change it, delete it or manage its access, then give or remove access."),
        &[],
    )
}

/// Access page title.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-title",
    title = "Access",
    spotlight = "secrets-acl-title",
    position = "bottom",
    order = 20
)]
#[component]
pub fn SecretsAclTitleHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-title",
        "This heading marks the sharing workspace for the secret you selected.",
        None,
        &[],
    )
}

/// Secret picker.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-secret-select",
    title = "Choose a secret",
    spotlight = "secrets-acl-secret-select",
    position = "bottom",
    order = 30
)]
#[component]
pub fn SecretsAclSecretSelectHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-secret-select",
        "Start here. Grants always apply to one secret at a time.",
        Some("You need SecretsGrantManage and Manage access on that secret to see or change who has it."),
        &[],
    )
}

/// Grant form.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-grant-form",
    title = "Give access",
    spotlight = "secrets-acl-secret-select",
    position = "top",
    order = 40
)]
#[component]
pub fn SecretsAclGrantFormHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-grant-form",
        "After you pick a secret, use Give access: find a person or service user and tick what they can do. You confirm with your authenticator code.",
        None,
        &[],
    )
}

/// Nav back to Secrets from the Access page.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-nav-secrets",
    title = "Back to the vault",
    spotlight = "secrets-nav-secrets",
    position = "right",
    order = 50
)]
#[component]
pub fn SecretsAclNavSecretsHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-nav-secrets",
        "Open Secrets to store, read, rotate and delete secrets. Help, then Replay, restarts this page's tour.",
        None,
        &[],
    )
}

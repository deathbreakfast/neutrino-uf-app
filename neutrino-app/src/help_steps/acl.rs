//! Spotlight steps for the ACL manage page (`/secrets/acl`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Centered ACL product intro.
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
        "Grant View, Reveal, Edit, Delete, or Maintain on a single secret without opening the whole vault.",
        Some("Pick a secret, review who already has each action, then add or revoke direct user grants."),
        &[],
    )
}

/// ACL page title.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-title",
    title = "Secret access grants",
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
        Some("You need SecretsGrantManage and maintain rights on that secret's Gauge permissions to see or edit grants."),
        &[],
    )
}

/// Grant form.
#[help_spotlight_step(
    route = "/secrets/acl",
    feature_highlight = "secrets-acl-grant-form",
    title = "Add a grant",
    spotlight = "secrets-acl-secret-select",
    position = "top",
    order = 40
)]
#[component]
pub fn SecretsAclGrantFormHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-acl-grant-form",
        "After you pick a secret, enter a user id and an action. Reveal and Maintain ask for a fresh authenticator code.",
        None,
        &[],
    )
}

/// Nav back to Secrets from ACL page.
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
        "Open Secrets for create, reveal, rotate, and delete. Help, then Replay, restarts this page's tour.",
        None,
        &[],
    )
}

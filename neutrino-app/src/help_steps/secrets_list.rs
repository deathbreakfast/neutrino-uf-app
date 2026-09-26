//! Spotlight steps for the Secrets vault list chrome (`/secrets`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Centered intro: vault purpose and vocabulary.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-intro",
    title = "Welcome to Secrets",
    order = 10
)]
#[component]
pub fn SecretsIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-intro",
        "Secrets is the locked cabinet for passwords, API tokens, and other values apps need but people should not leave in chat or config files. Think of a bank safe-deposit box: the label is visible; opening the box (Reveal) is a separate, audited step.",
        Some("Neutrino is the sealed store behind this app. We will walk the list, the create form, and each row action, including the dialogs they open."),
        &[
            "Secret: one named entry in the vault",
            "Version: each time the value is replaced",
            "Reveal: show the current value (audited)",
            "Rotate: store a new value; version goes up",
        ],
    )
}

/// Page title.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-page-title",
    title = "Secret vault",
    spotlight = "secrets-page-title",
    position = "bottom",
    order = 20
)]
#[component]
pub fn SecretsPageTitleHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-page-title",
        "This heading marks the vault list, the home page for every secret metadata row.",
        Some("Create, reveal, rotate, and delete all start from here."),
        &[],
    )
}

/// Permission caption under the title.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-permissions-note",
    title = "Who can do what",
    spotlight = "secrets-permissions-note",
    position = "bottom",
    order = 30
)]
#[component]
pub fn SecretsPermissionsNoteHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-permissions-note",
        "Buttons stay visible even when your role cannot use them. The server still checks permission and returns a clear error if Reveal or Rotate is denied.",
        Some("Seeing the list is separate from reading the secret value."),
        &[],
    )
}

/// Create secret button.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-create",
    title = "Add a secret",
    spotlight = "secrets-create-button",
    position = "left",
    order = 40
)]
#[component]
pub fn SecretsCreateHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-create",
        "Create secret opens the form for a new vault entry.",
        Some("Next we walk each field in that form."),
        &[],
    )
}

/// Name column header.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-col-name",
    title = "Name",
    spotlight = "secrets-col-name",
    position = "bottom",
    order = 50
)]
#[component]
pub fn SecretsColNameHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-col-name",
        "Name is the friendly label operators use in the list. It is the entry label, not the secret value itself.",
        None,
        &[],
    )
}

/// Scope column header.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-col-scope",
    title = "Scope",
    spotlight = "secrets-col-scope",
    position = "bottom",
    order = 60
)]
#[component]
pub fn SecretsColScopeHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-col-scope",
        "Scope path groups secrets like folders, for example by provider or environment, so large vaults stay scannable.",
        None,
        &[],
    )
}

/// Kind column header.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-col-kind",
    title = "Kind",
    spotlight = "secrets-col-kind",
    position = "bottom",
    order = 70
)]
#[component]
pub fn SecretsColKindHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-col-kind",
        "Kind is a type label (for example an API token). It helps you tell credentials apart without opening them.",
        None,
        &[],
    )
}

/// Version column header.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-col-version",
    title = "Version",
    spotlight = "secrets-col-version",
    position = "bottom",
    order = 80
)]
#[component]
pub fn SecretsColVersionHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-col-version",
        "Version counts how many times the value was replaced. After a Rotate, this number goes up.",
        None,
        &[],
    )
}

/// Created column header.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-col-created",
    title = "Created",
    spotlight = "secrets-col-created",
    position = "bottom",
    order = 90
)]
#[component]
pub fn SecretsColCreatedHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-col-created",
        "Created is when the entry first appeared in the vault. It stays the same when you rotate the value.",
        None,
        &[],
    )
}

/// Left-nav Secrets link.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-nav-secrets",
    title = "Secrets in the menu",
    spotlight = "secrets-nav-secrets",
    position = "right",
    order = 240
)]
#[component]
pub fn SecretsNavSecretsHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-nav-secrets",
        "Secrets returns you to this vault list.",
        None,
        &[],
    )
}

/// Left-nav ACLs link.
#[help_spotlight_step(
    route = "/secrets",
    feature_highlight = "secrets-nav-acl",
    title = "ACLs in the menu",
    spotlight = "secrets-nav-acl",
    position = "right",
    order = 250
)]
#[component]
pub fn SecretsNavAclHelp() -> impl IntoView {
    help_stack(
        "help-step-secrets-nav-acl",
        "ACLs opens the sharing page where you grant or revoke access per secret. A short tour starts there.",
        Some("Help, then Replay, restarts this page's tour."),
        &[],
    )
}

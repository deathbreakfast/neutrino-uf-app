//! Top-level route pages for the Neutrino app: the vault list and Access, plus
//! the dialogs and access card the secret's own page reuses.

/// Who can do what with one secret: Access card, Give access, Remove access.
pub mod access;
mod acl_manage;
/// Read value, Rotate and Delete dialogs, and the notify-failed bar.
pub mod secret_actions;
mod secrets_list;
/// Step-up retry helpers for Tier A vault mutations.
pub mod step_up;
pub use acl_manage::AclManagePage;
pub use secrets_list::SecretsListPage;

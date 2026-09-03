//! Top-level route pages for the Secrets app: vault list and ACL placeholder.

mod acl_manage;
mod secrets_list;
/// Step-up retry helpers for Tier A vault mutations.
pub mod step_up;
pub use acl_manage::AclManagePage;
pub use secrets_list::SecretsListPage;

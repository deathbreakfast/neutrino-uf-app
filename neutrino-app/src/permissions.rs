//! Permission manifest for the Secrets app's permission domain.

use uf_product_macros::UfPermissionManifest;

/// Permission manifest for the `secrets` domain, registered with the platform
/// permission system via `#[derive(UfPermissionManifest)]`.
// The `UfPermissionManifest` derive macro generates an explicit `Clone`
// impl alongside a derived `Copy` impl; that's macro-generated code we don't
// control here, so silence the lint at the use site.
#[allow(clippy::expl_impl_clone_on_copy)]
#[derive(UfPermissionManifest)]
#[permission_manifest(
    domain_key = "secrets",
    domain_name = "Secrets",
    domain_description = "Neutrino encrypted secret vault"
)]
pub enum NeutrinoPermission {
    /// View secret metadata (not plaintext).
    #[permission(description = "View secret metadata")]
    SecretsRead,
    /// Reveal secret plaintext (audited).
    #[permission(description = "Reveal secret plaintext (audited)")]
    SecretsReveal,
    /// Create or update secrets.
    #[permission(description = "Create or update secrets")]
    SecretsWrite,
    /// Trigger rotation to a new secret version.
    #[permission(description = "Trigger rotation")]
    SecretsRotate,
    /// Manage ACLs/grants on secrets.
    #[permission(description = "Manage ACLs on secrets")]
    SecretsGrantManage,
}

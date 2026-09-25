//! Permission-gated server functions for Neutrino vault operations.
//!
//! Thin Higgs wrappers over [`neutrino::vault`]. Domain contracts are tested
//! in the `neutrino` crate (`vault_crud_contract`, `vault_authz_contract`).
//! Gauge RBAC deny/allow for these permission names is covered by
//! `vault_server_rbac`.
//!
//! # Authorization layers
//!
//! 1. **Gauge coarse** — `#[uf_product_macros::server(permission = "...")]` requires the
//!    named permission on the request actor.
//! 2. **Valence privacy** — session [`higgs::Higgs::valence`] drives ORM access; Neutrino
//!    schemas enforce per-secret Gauge grants inside Valence (no mid-request System elevate).
//! 3. **Per-secret Gauge** — vault helpers authorize via the store's request actor
//!    (`actor_can_secret` / Valence privacy policies).
//! 4. **Audit** — success rows append under the session actor; denials use a System sink.

use leptos::prelude::*;
#[cfg(not(feature = "ssr"))]
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use valence::Actor;

/// Row for vault list views (no ciphertext).
#[cfg(feature = "ssr")]
pub use neutrino::VaultSecretRow;

/// One-shot reveal payload (base64 for JSON-safe transport).
#[cfg(feature = "ssr")]
pub use neutrino::RevealedVaultSecret;

/// Row for vault list views (no ciphertext) — hydrate stub shape.
#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultSecretRow {
    /// Secret id.
    pub id: String,
    /// Human-readable secret name.
    pub name: String,
    /// Scope path the secret is stored under (e.g. `/gluon/provider_account/...`).
    pub scope_path: String,
    /// Secret kind/category (free-form, product-defined).
    pub kind: String,
    /// Current version number (increments on rotate).
    pub current_version: i64,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
}

/// One-shot reveal payload (base64 for JSON-safe transport) — hydrate stub shape.
#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevealedVaultSecret {
    /// Base64-encoded plaintext; never persisted client-side.
    pub plaintext_b64: String,
}

pub(crate) mod grants;

pub use grants::{
    grant_secret_action, list_secret_grants, revoke_secret_action, SecretActionGrant,
    SecretGrantPrincipal,
};

/// Permission names enforced by vault `#[server]` wrappers.
#[cfg(feature = "ssr")]
pub mod vault_permissions {
    /// `list_vault_secrets` / `neutrino_vault_ping`.
    pub const SECRETS_READ: &str = "SecretsRead";
    /// `reveal_vault_secret`.
    pub const SECRETS_REVEAL: &str = "SecretsReveal";
    /// `create_vault_secret` / `delete_vault_secret`.
    pub const SECRETS_WRITE: &str = "SecretsWrite";
    /// `rotate_vault_secret`.
    pub const SECRETS_ROTATE: &str = "SecretsRotate";

    /// All vault server-fn permission names.
    pub const ALL: &[&str] = &[SECRETS_READ, SECRETS_REVEAL, SECRETS_WRITE, SECRETS_ROTATE];
}

#[cfg(feature = "ssr")]
pub(crate) fn session_valence_from_ctx(
    ctx: &higgs::Higgs,
) -> Result<valence::Valence, ServerFnError> {
    ctx.valence()
        .map_err(|e| ServerFnError::new(format!("Failed to build request Valence: {e}")))
}

// `Actor` only resolves when `valence` is enabled (via this crate's own `ssr`
// feature); this helper is only ever called from SSR-only server fn bodies.
#[cfg(feature = "ssr")]
fn actor_owner_label(actor: Actor) -> String {
    match actor {
        // SessionSnapshot / Higgs already use `user:…` ids (gauge e2e convention).
        Actor::User { user_id } => {
            if user_id.starts_with("user:") {
                user_id
            } else {
                format!("user:{user_id}")
            }
        }
        Actor::ServiceUser { service_name } => {
            if service_name.starts_with("service:") {
                service_name
            } else {
                format!("service:{service_name}")
            }
        }
        Actor::System { operation } => format!("system:{operation}"),
        Actor::Anonymous => "anonymous".to_string(),
    }
}

/// Client-safe message when domain failures must not leak internal detail.
#[cfg(feature = "ssr")]
const INTERNAL_VAULT_ERROR: &str = "An internal vault error occurred. Check server logs.";

#[cfg(feature = "ssr")]
#[allow(clippy::needless_pass_by_value)] // `map_err(map_neutrino_error)` needs owned Err
fn map_neutrino_error(err: neutrino::NeutrinoError) -> ServerFnError {
    use neutrino::NeutrinoError;
    match &err {
        NeutrinoError::NotFound { .. }
        | NeutrinoError::AccessDenied { .. }
        | NeutrinoError::Validation { .. }
        | NeutrinoError::InvalidState { .. } => ServerFnError::new(err.to_string()),
        NeutrinoError::Config(_)
        | NeutrinoError::Crypto { .. }
        | NeutrinoError::Unsupported { .. }
        | NeutrinoError::Service { .. } => {
            tracing::warn!(
                target: "neutrino_app",
                error = %err,
                "vault server fn internal failure"
            );
            ServerFnError::new(INTERNAL_VAULT_ERROR)
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{actor_owner_label, map_neutrino_error, INTERNAL_VAULT_ERROR};
    use neutrino::NeutrinoError;
    use valence::Actor;

    #[test]
    fn actor_owner_label_formats_session_actors() {
        assert_eq!(
            actor_owner_label(Actor::User {
                user_id: "alice".into()
            }),
            "user:alice"
        );
        assert_eq!(
            actor_owner_label(Actor::User {
                user_id: "user:bob".into()
            }),
            "user:bob"
        );
        assert_eq!(
            actor_owner_label(Actor::ServiceUser {
                service_name: "cron".into()
            }),
            "service:cron"
        );
        assert_eq!(actor_owner_label(Actor::Anonymous), "anonymous");
    }

    #[test]
    fn map_neutrino_error_passes_client_safe_variants() {
        let not_found = map_neutrino_error(NeutrinoError::NotFound { id: "sid-1".into() });
        assert!(not_found.to_string().contains("secret not found"));

        let denied = map_neutrino_error(NeutrinoError::AccessDenied {
            operation: "reveal",
        });
        assert!(denied.to_string().contains("not authorized"));

        let validation = map_neutrino_error(NeutrinoError::Validation {
            field: "Name",
            message: "required".into(),
        });
        assert!(validation.to_string().contains("required"));

        let invalid = map_neutrino_error(NeutrinoError::InvalidState {
            operation: "lease",
            message: "archived only".into(),
        });
        assert!(invalid.to_string().contains("archived only"));
    }

    #[test]
    fn map_neutrino_error_hides_internal_variants() {
        let internal = map_neutrino_error(NeutrinoError::Unsupported {
            operation: "delete",
        });
        assert!(internal.to_string().contains(INTERNAL_VAULT_ERROR));
        assert!(!internal.to_string().contains("delete"));
    }
}

#[cfg(feature = "ssr")]
fn store_for_request(
    ctx: &higgs::Higgs,
    session_v: valence::Valence,
) -> neutrino::ValenceSealedStore {
    let actor = actor_owner_label(ctx.actor());
    neutrino::store_from_valence_for_request(session_v, actor)
}

/// Verifies that the sealed store can be reached (RBAC: [`crate::permissions::NeutrinoPermission::SecretsRead`]).
#[uf_product_macros::server(permission = "SecretsRead")]
pub async fn neutrino_vault_ping() -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    neutrino::assert_neutrino_catalog_seeded(&session_valence_from_ctx(&ctx)?)
        .await
        .map_err(map_neutrino_error)?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let store = store_for_request(&ctx, session_v);
    neutrino::neutrino_vault_ping(&store)
        .await
        .map_err(map_neutrino_error)
}

/// Lists non-sensitive metadata for vault secrets visible to the caller.
#[uf_product_macros::server(permission = "SecretsRead")]
pub async fn list_vault_secrets() -> Result<Vec<VaultSecretRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    neutrino::list_vault_secrets(&session_v, None)
        .await
        .map_err(map_neutrino_error)
}

/// Creates a new secret (version 1).
#[uf_product_macros::server(permission = "SecretsWrite", step_up)]
pub async fn create_vault_secret(
    /// Human-readable secret name.
    name: String,
    /// Scope path the secret is stored under (e.g. `/gluon/provider_account/...`).
    scope_path: String,
    /// Secret kind/category (free-form, product-defined).
    kind: String,
    /// Plaintext secret value to seal and store.
    plaintext: String,
) -> Result<VaultSecretRow, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let owner = actor_owner_label(ctx.actor());
    let store = store_for_request(&ctx, session_v);
    neutrino::create_vault_secret(&store, name, scope_path, kind, plaintext, owner)
        .await
        .map_err(map_neutrino_error)
}

/// Returns the current version plaintext (base64). Never persisted client-side.
#[uf_product_macros::server(permission = "SecretsReveal", step_up = "fresh")]
pub async fn reveal_vault_secret(
    /// Unique identifier of the secret to reveal.
    id: String,
    /// Fresh TOTP code required for reveal (including Super User break-glass).
    totp_code: String,
) -> Result<RevealedVaultSecret, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        #[cfg(feature = "e2e-lab")]
        {
            crate::e2e_lab::verify_fresh_totp(&totp_code).await?;
        }
        #[cfg(not(feature = "e2e-lab"))]
        {
            lepton_auth::verify_fresh_totp(&totp_code)
                .await
                .map_err(|e| e.to_server_fn_error())?;
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = totp_code;
    }
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let store = store_for_request(&ctx, session_v);
    neutrino::reveal_vault_secret(&store, id)
        .await
        .map_err(map_neutrino_error)
}

/// Deletes a secret and all versions (hard delete with prior audit event in Neutrino).
#[uf_product_macros::server(permission = "SecretsWrite", step_up)]
pub async fn delete_vault_secret(
    /// Unique identifier of the secret to delete.
    id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let store = store_for_request(&ctx, session_v);
    neutrino::delete_vault_secret(&store, id)
        .await
        .map_err(map_neutrino_error)
}

/// Rotates ciphertext to a new version (publishes Photon event when DB-scoped).
#[uf_product_macros::server(permission = "SecretsRotate", step_up)]
pub async fn rotate_vault_secret(
    /// Unique identifier of the secret to rotate.
    id: String,
    /// New plaintext value to seal as the next version.
    new_plaintext: String,
) -> Result<VaultSecretRow, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let actor = actor_owner_label(ctx.actor());
    let store = store_for_request(&ctx, session_v);
    let secret_id = id.clone();
    let row = neutrino::rotate_vault_secret(&store, id, new_plaintext, actor.as_str())
        .await
        .map_err(map_neutrino_error)?;

    // Live L4: DB-scoped rotates publish `neutrino.secret.rotated` for Gluon apply.
    // Non-DB scopes (provider tokens, etc.) stay quiet — no mid-request System elevate.
    match neutrino::publish_if_db_scoped_secret_rotated(
        &row.id,
        row.current_version,
        None,
        &row.scope_path,
        format!("vault_rotate:{}:{}", row.id, row.current_version),
    )
    .await
    {
        Ok(true) => {
            tracing::info!(
                target: "neutrino_app",
                secret_id = %secret_id,
                version = row.current_version,
                "vault rotate published neutrino.secret.rotated"
            );
        }
        Ok(false) => {
            tracing::debug!(
                target: "neutrino_app",
                secret_id = %secret_id,
                "vault rotate complete (non-DB scope; Photon publish skipped)"
            );
        }
        Err(e) => {
            tracing::warn!(
                target: "neutrino_app",
                secret_id = %secret_id,
                error = %e,
                "vault rotate succeeded but Photon publish failed"
            );
        }
    }

    Ok(row)
}

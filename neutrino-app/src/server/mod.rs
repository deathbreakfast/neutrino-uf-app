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
    change_secret_actions, grant_secret_actions, list_secret_grants, revoke_secret_actions,
    GrantPrincipal, GrantPrincipalKind, SecretActionGrant, SecretGrantPrincipal,
};

/// Leading tags on server-fn error text, so the pages can explain a failure
/// without showing the raw message. [`crate::components::error_copy`] reads them.
pub mod error_tags {
    /// The secret (or Gauge permission) doesn't exist, or the caller can't see it.
    pub const NOT_FOUND: &str = "not_found::";
    /// Per-secret Gauge check refused the caller.
    pub const ACCESS_DENIED: &str = "access_denied::";
    /// A form value was rejected; the rest of the text is a sentence for people.
    pub const INVALID_INPUT: &str = "invalid_input::";
    /// The secret is in a state that doesn't allow the action.
    pub const INVALID_STATE: &str = "invalid_state::";
    /// Something failed inside the vault; details are in the server log only.
    pub const VAULT_INTERNAL: &str = "vault_internal::";
    /// Gauge didn't save a change; details are in the server log only.
    pub const GAUGE_FAILED: &str = "gauge_failed::";
    /// No signed-in user on the request.
    pub const SIGN_IN_REQUIRED: &str = "sign_in_required::";
}

/// What [`rotate_vault_secret`] saved, and whether Nucleus was told about it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotateOutcome {
    /// The version the rotate stored (now the current one).
    pub version: i64,
    /// Set when the version saved but the `neutrino.secret.rotated` notice to
    /// Nucleus failed. [`renotify_rotated_secret`] sends it again.
    pub notify_error: Option<String>,
}

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
    /// Names a caller may ask for through [`super::request_neutrino_permission`].
    pub const REQUESTABLE: &[&str] = &[SECRETS_REVEAL, SECRETS_WRITE, SECRETS_ROTATE];
}

#[cfg(feature = "ssr")]
pub(crate) fn session_valence_from_ctx(
    ctx: &higgs::Higgs,
) -> Result<valence::Valence, ServerFnError> {
    ctx.valence().map_err(|e| {
        tracing::warn!(target: "neutrino_app", error = %e, "failed to build request Valence");
        ServerFnError::new(INTERNAL_VAULT_ERROR)
    })
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
const INTERNAL_VAULT_ERROR: &str =
    "vault_internal::An internal vault error occurred. Check server logs.";

#[cfg(feature = "ssr")]
#[allow(clippy::needless_pass_by_value)] // `map_err(map_neutrino_error)` needs owned Err
fn map_neutrino_error(err: neutrino::NeutrinoError) -> ServerFnError {
    use neutrino::NeutrinoError;
    let tag = match &err {
        NeutrinoError::NotFound { .. } => Some(error_tags::NOT_FOUND),
        NeutrinoError::AccessDenied { .. } => Some(error_tags::ACCESS_DENIED),
        NeutrinoError::Validation { .. } => Some(error_tags::INVALID_INPUT),
        NeutrinoError::InvalidState { .. } => Some(error_tags::INVALID_STATE),
        _ => None,
    };
    match tag {
        Some(tag) => ServerFnError::new(format!("{tag}{err}")),
        None => {
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
    use super::{actor_owner_label, error_tags, map_neutrino_error, INTERNAL_VAULT_ERROR};
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
        assert!(not_found.to_string().contains(error_tags::NOT_FOUND));
        assert!(not_found.to_string().contains("secret not found"));

        let denied = map_neutrino_error(NeutrinoError::AccessDenied {
            operation: "reveal",
        });
        assert!(denied.to_string().contains("not authorized"));

        let validation = map_neutrino_error(NeutrinoError::Validation {
            field: "Name",
            message: "required".into(),
        });
        assert!(validation.to_string().contains(error_tags::INVALID_INPUT));
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
    session_v: valence::Valence,
) -> Result<neutrino::ValenceSealedStore, ServerFnError> {
    let actor = neutrino::RequestActor::from_valence(&session_v).ok_or_else(|| {
        ServerFnError::new(format!(
            "{}Vault access requires a signed-in user",
            error_tags::SIGN_IN_REQUIRED
        ))
    })?;
    Ok(neutrino::store_from_valence_for_request(session_v, actor))
}

/// Verifies that the sealed store can be reached (RBAC: [`crate::permissions::NeutrinoPermission::SecretsRead`]).
#[uf_product_macros::server(permission = "SecretsRead")]
pub async fn neutrino_vault_ping() -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    neutrino::assert_neutrino_catalog_seeded(&session_valence_from_ctx(&ctx)?)
        .await
        .map_err(map_neutrino_error)?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let store = store_for_request(session_v)?;
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
    let store = store_for_request(session_v)?;
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
    let store = store_for_request(session_v)?;
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
    let store = store_for_request(session_v)?;
    neutrino::delete_vault_secret(&store, id)
        .await
        .map_err(map_neutrino_error)
}

/// Rotates ciphertext to a new version (publishes Photon event when DB-scoped).
///
/// A DB-scoped rotate also tells Nucleus through `neutrino.secret.rotated`. When
/// that notice fails the new version is still saved, so this returns `Ok` with
/// [`RotateOutcome::notify_error`] set; [`renotify_rotated_secret`] retries it.
#[uf_product_macros::server(permission = "SecretsRotate", step_up)]
pub async fn rotate_vault_secret(
    /// Unique identifier of the secret to rotate.
    id: String,
    /// New plaintext value to seal as the next version.
    new_plaintext: String,
) -> Result<RotateOutcome, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let actor = actor_owner_label(ctx.actor());
    let store = store_for_request(session_v)?;
    let row = neutrino::rotate_vault_secret(&store, id, new_plaintext, actor.as_str())
        .await
        .map_err(map_neutrino_error)?;
    let correlation = format!("vault_rotate:{}:{}", row.id, row.current_version);
    let notify_error = publish_rotated(
        &row.id,
        row.current_version,
        &row.scope_path,
        correlation,
        "vault rotate",
    )
    .await;
    Ok(RotateOutcome {
        version: row.current_version,
        notify_error,
    })
}

/// Sends the `neutrino.secret.rotated` notice for the secret's current version
/// again, after [`rotate_vault_secret`] saved a version but couldn't tell Nucleus.
///
/// Same permission and step-up as rotate, plus the per-secret Edit grant.
/// Non-DB scopes have nothing to send, so they succeed without publishing.
#[uf_product_macros::server(permission = "SecretsRotate", step_up)]
pub async fn renotify_rotated_secret(
    /// Unique identifier of the rotated secret.
    secret_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let allowed = neutrino::actor_can_secret(
        &session_v,
        &secret_id,
        gauge::resource_permissions::ResourceAction::Edit,
    )
    .await
    .map_err(map_neutrino_error)?;
    if !allowed {
        return Err(map_neutrino_error(neutrino::NeutrinoError::AccessDenied {
            operation: "renotify",
        }));
    }
    // One row by id, read under the session Valence (the same privacy rules as the list).
    let secret = {
        use valence::Model;
        neutrino::generated::NeutrinoSecret::get(
            &secret_id,
            &session_v,
            valence::use_!(r"When someone **sends a rotated secret's notice to Nucleus again**, we read that secret's **current version and scope path**, never its value, so the notice names the right version."),
        )
        .await
        .map_err(|e| {
            tracing::warn!(target: "neutrino_app", error = %e, "renotify secret lookup failed");
            ServerFnError::new(INTERNAL_VAULT_ERROR)
        })?
        .ok_or_else(|| {
            map_neutrino_error(neutrino::NeutrinoError::NotFound {
                id: secret_id.clone(),
            })
        })?
    };
    let version = *secret.current_version();
    // A fresh correlation: the rotate's own one may already be spent on the failed try.
    let correlation = format!(
        "vault_renotify:{secret_id}:{version}:{}",
        chrono::Utc::now().timestamp_millis()
    );
    match publish_rotated(
        &secret_id,
        version,
        secret.scope_path(),
        correlation,
        "vault renotify",
    )
    .await
    {
        None => Ok(()),
        Some(message) => Err(ServerFnError::new(message)),
    }
}

/// Publish `neutrino.secret.rotated` for a DB-scoped row; the client-safe error on failure.
///
/// Live L4: DB-scoped rotates publish for Gluon apply. Non-DB scopes (provider
/// tokens, etc.) stay quiet. No mid-request System elevate.
#[cfg(feature = "ssr")]
async fn publish_rotated(
    secret_id: &str,
    version: i64,
    scope_path: &str,
    correlation: String,
    operation: &'static str,
) -> Option<String> {
    match neutrino::publish_if_db_scoped_secret_rotated(
        secret_id,
        version,
        None,
        scope_path,
        correlation,
    )
    .await
    {
        Ok(true) => {
            tracing::info!(
                target: "neutrino_app",
                secret_id = %secret_id,
                version,
                operation,
                "published neutrino.secret.rotated"
            );
            None
        }
        Ok(false) => {
            tracing::debug!(
                target: "neutrino_app",
                secret_id = %secret_id,
                operation,
                "non-DB scope; Photon publish skipped"
            );
            None
        }
        Err(e) => {
            tracing::error!(
                target: "neutrino_app",
                secret_id = %secret_id,
                error = %e,
                operation,
                "version saved but Photon publish failed"
            );
            Some(format!(
                "{}Nucleus wasn't told about version {}",
                error_tags::VAULT_INTERNAL,
                version
            ))
        }
    }
}

/// Asks for one of Neutrino's app-wide permissions through Gauge's request flow.
///
/// Whoever reviews Gauge requests for that permission gets it to approve or deny.
/// Only Neutrino's own permission names are accepted.
#[uf_product_macros::server(permission = "SecretsRead")]
pub async fn request_neutrino_permission(
    /// A `Secrets*` permission name, such as `SecretsWrite`.
    permission: String,
) -> Result<(), ServerFnError> {
    use gauge::types::{PermissionRequestCreateInput, PermissionRequestTargetKind};

    if !vault_permissions::REQUESTABLE.contains(&permission.as_str()) {
        return Err(ServerFnError::new(format!(
            "{}Unknown permission {permission}",
            error_tags::INVALID_INPUT
        )));
    }
    let Some(target_id) = gauge::server::resolve_permission_id_by_name(permission.clone()).await?
    else {
        return Err(ServerFnError::new(format!(
            "{}Permission {permission}",
            error_tags::NOT_FOUND
        )));
    };
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    gauge::service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Permission,
            target_id,
            reason: format!("Requested from Neutrino to use {permission}."),
            on_behalf_of: None,
        },
        &session_v,
    )
    .await
    .map_err(|e| {
        let message = e.to_string();
        // Gauge's own refusals ("already pending", "already have") read fine to people.
        if message.contains("already") {
            return ServerFnError::new(format!("{}{message}", error_tags::INVALID_STATE));
        }
        tracing::warn!(target: "neutrino_app", error = %message, "permission request failed");
        ServerFnError::new(format!("{}request access", error_tags::GAUGE_FAILED))
    })?;
    Ok(())
}

/// What the caller may do app-wide in Neutrino; the pages show only the buttons that work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultAbilities {
    /// Holds `SecretsWrite`: may store new secrets.
    pub can_write: bool,
}

/// The caller's app-wide Neutrino abilities, from the same Gauge check the
/// server functions run.
#[uf_product_macros::server(permission = "SecretsRead")]
pub async fn get_vault_abilities() -> Result<VaultAbilities, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let can_write = gauge::service::actor_can(&session_v, vault_permissions::SECRETS_WRITE)
        .await
        .map_err(|e| {
            tracing::warn!(target: "neutrino_app", error = %e, "vault abilities check failed");
            ServerFnError::new(INTERNAL_VAULT_ERROR)
        })?;
    Ok(VaultAbilities { can_write })
}

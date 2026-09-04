//! Per-secret Gauge grant management for the ACL page.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use gauge::resource_permissions::{permission_record_id, ResourceAction};
#[cfg(feature = "ssr")]
use gauge::service;
#[cfg(feature = "ssr")]
use gauge::types::{PrincipalKind, PrincipalRefDto};
#[cfg(feature = "ssr")]
use neutrino::NEUTRINO_SECRET;

#[cfg(feature = "ssr")]
use super::session_valence_from_ctx;

/// One direct user grant on a secret action permission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretActionGrant {
    /// Gauge action suffix (`View`, `Reveal`, …).
    pub action: String,
    /// Stable permission record id for grant/revoke calls.
    pub permission_id: String,
    /// Fully qualified permission name (`neutrino_secret.{id}.Reveal`).
    pub permission_name: String,
    /// User principals currently on the allow list for this action.
    pub user_grants: Vec<SecretGrantPrincipal>,
}

/// A user principal holding a direct grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretGrantPrincipal {
    /// Bare user id (without the `user:` prefix).
    pub user_id: String,
    /// Display label from Gauge (may redact email).
    pub label: String,
}

#[cfg(feature = "ssr")]
fn parse_secret_action(action: &str) -> Result<ResourceAction, ServerFnError> {
    match action.trim() {
        "View" => Ok(ResourceAction::View),
        "Reveal" => Ok(ResourceAction::Reveal),
        "Edit" => Ok(ResourceAction::Edit),
        "Delete" => Ok(ResourceAction::Delete),
        "Maintain" => Ok(ResourceAction::Maintain),
        other => Err(ServerFnError::new(format!(
            "Unknown secret action: {other}"
        ))),
    }
}

#[cfg(feature = "ssr")]
fn action_requires_fresh_step_up(action: ResourceAction) -> bool {
    matches!(action, ResourceAction::Reveal | ResourceAction::Maintain)
}

#[cfg(feature = "ssr")]
async fn verify_fresh_totp_code(totp_code: &str) -> Result<(), ServerFnError> {
    #[cfg(feature = "e2e-lab")]
    {
        crate::e2e_lab::verify_fresh_totp(totp_code).await
    }
    #[cfg(not(feature = "e2e-lab"))]
    {
        lepton_auth::verify_fresh_totp(totp_code)
            .await
            .map_err(|e| e.to_server_fn_error())
    }
}

#[cfg(feature = "ssr")]
fn map_grant_service_err(context: &str, err: impl std::fmt::Display) -> ServerFnError {
    let message = err.to_string();
    if message.contains("not authorized") || message.contains("Permission not found") {
        return ServerFnError::new(message);
    }
    tracing::warn!(
        target: "neutrino_app",
        context,
        error = %message,
        "secret grant server fn failure"
    );
    ServerFnError::new(format!("Failed to {context}"))
}

#[cfg(feature = "ssr")]
fn principal_to_grant(principal: PrincipalRefDto) -> Option<SecretGrantPrincipal> {
    if principal.kind != PrincipalKind::User {
        return None;
    }
    let user_id = principal
        .id
        .strip_prefix("user:")
        .unwrap_or(principal.id.as_str())
        .to_string();
    Some(SecretGrantPrincipal {
        user_id,
        label: principal.label,
    })
}

/// List direct user grants for each action on a secret.
#[uf_product_macros::server(permission = "SecretsGrantManage")]
pub async fn list_secret_grants(
    secret_id: String,
) -> Result<Vec<SecretActionGrant>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;

    let mut rows = Vec::new();
    for action in NEUTRINO_SECRET.actions {
        let permission_id = permission_record_id(NEUTRINO_SECRET, &secret_id, *action);
        let Some(detail) = service::get_permission_detail(&permission_id, &session_v)
            .await
            .map_err(|e| map_grant_service_err("load secret grants", e))?
        else {
            continue;
        };
        let user_grants = detail
            .allow_list
            .into_iter()
            .filter_map(principal_to_grant)
            .collect();
        rows.push(SecretActionGrant {
            action: action.as_str().to_string(),
            permission_id: detail.id,
            permission_name: detail.name,
            user_grants,
        });
    }
    Ok(rows)
}

/// Grant a user a single action on a secret.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn grant_secret_action(
    secret_id: String,
    user_id: String,
    action: String,
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let parsed = parse_secret_action(&action)?;
    if action_requires_fresh_step_up(parsed) {
        let code = totp_code.filter(|c| !c.trim().is_empty()).ok_or_else(|| {
            ServerFnError::new(
                "STEP_UP:step_up_required: fresh authenticator code required for this grant"
                    .to_string(),
            )
        })?;
        verify_fresh_totp_code(&code).await?;
    }

    let permission_id = permission_record_id(NEUTRINO_SECRET, &secret_id, parsed);
    service::grant_permission_to_user(&permission_id, user_id.trim(), &session_v)
        .await
        .map_err(|e| map_grant_service_err("grant secret action", e))?;
    Ok(())
}

/// Revoke a user's direct grant for one secret action.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn revoke_secret_action(
    secret_id: String,
    user_id: String,
    action: String,
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let parsed = parse_secret_action(&action)?;
    if action_requires_fresh_step_up(parsed) {
        let code = totp_code.filter(|c| !c.trim().is_empty()).ok_or_else(|| {
            ServerFnError::new(
                "STEP_UP:step_up_required: fresh authenticator code required for this revoke"
                    .to_string(),
            )
        })?;
        verify_fresh_totp_code(&code).await?;
    }

    let permission_id = permission_record_id(NEUTRINO_SECRET, &secret_id, parsed);
    service::revoke_permission_from_user(&permission_id, user_id.trim(), &session_v)
        .await
        .map_err(|e| map_grant_service_err("revoke secret action", e))?;
    Ok(())
}

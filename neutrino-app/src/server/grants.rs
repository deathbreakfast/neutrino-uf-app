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
use lepton_identity::service_users::ServiceUserId;
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

/// Which kind of principal a grant names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantPrincipalKind {
    /// A signed-in person.
    User,
    /// A Lepton service user.
    ServiceUser,
}

impl GrantPrincipalKind {
    /// Wire form sent to [`grant_secret_action`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::ServiceUser => "service_user",
        }
    }

    /// Parse the wire form; anything else is refused.
    ///
    /// # Errors
    ///
    /// A [`ServerFnError`] naming the unsupported kind.
    pub fn parse(raw: &str) -> Result<Self, ServerFnError> {
        match raw.trim() {
            "user" => Ok(Self::User),
            "service_user" => Ok(Self::ServiceUser),
            other => Err(ServerFnError::new(format!(
                "Secrets can be granted to people or service users, not {other:?}."
            ))),
        }
    }
}

/// A person or service user holding a direct grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretGrantPrincipal {
    /// Bare principal id (without the `user:` prefix).
    pub user_id: String,
    /// Display label from Gauge (may redact email).
    pub label: String,
    /// Whether this is a person or a service user.
    pub kind: GrantPrincipalKind,
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
    let kind = match principal.kind {
        PrincipalKind::User => GrantPrincipalKind::User,
        PrincipalKind::ServiceUser => GrantPrincipalKind::ServiceUser,
        _ => return None,
    };
    let user_id = principal
        .id
        .strip_prefix("user:")
        .unwrap_or(principal.id.as_str())
        .to_string();
    Some(SecretGrantPrincipal {
        user_id,
        label: principal.label,
        kind,
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

/// Grant a person or service user a single action on a secret.
///
/// `principal_kind` is [`GrantPrincipalKind::as_str`] of the picked principal.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn grant_secret_action(
    secret_id: String,
    user_id: String,
    principal_kind: String,
    action: String,
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let kind = GrantPrincipalKind::parse(&principal_kind)?;
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
    match kind {
        GrantPrincipalKind::User => {
            service::grant_permission_to_user(&permission_id, user_id.trim(), &session_v).await
        }
        GrantPrincipalKind::ServiceUser => {
            let service_user = ServiceUserId::parse(user_id.trim())
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            service::grant_permission_to_service_user_checked(
                &permission_id,
                &service_user,
                &session_v,
            )
            .await
        }
    }
    .map_err(|e| map_grant_service_err("grant secret action", e))?;
    Ok(())
}

/// Revoke a person's or service user's direct grant for one secret action.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn revoke_secret_action(
    secret_id: String,
    user_id: String,
    principal_kind: String,
    action: String,
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let kind = GrantPrincipalKind::parse(&principal_kind)?;
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
    match kind {
        GrantPrincipalKind::User => {
            service::revoke_permission_from_user(&permission_id, user_id.trim(), &session_v).await
        }
        GrantPrincipalKind::ServiceUser => {
            let service_user = ServiceUserId::parse(user_id.trim())
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            service::revoke_permission_from_service_user(&permission_id, &service_user, &session_v)
                .await
        }
    }
    .map_err(|e| map_grant_service_err("revoke secret action", e))?;
    Ok(())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn dto(kind: PrincipalKind, id: &str) -> PrincipalRefDto {
        PrincipalRefDto {
            kind,
            id: id.to_string(),
            label: "Label".to_string(),
            owner_label: None,
        }
    }

    #[test]
    fn kind_round_trips_for_person_and_service_user_happy() {
        for kind in [GrantPrincipalKind::User, GrantPrincipalKind::ServiceUser] {
            assert!(matches!(GrantPrincipalKind::parse(kind.as_str()), Ok(k) if k == kind));
        }
    }

    #[test]
    fn kind_refuses_groups_and_junk_sad() {
        for raw in ["group", "team", "", "USER"] {
            assert!(GrantPrincipalKind::parse(raw).is_err(), "{raw:?}");
        }
    }

    #[test]
    fn grants_list_keeps_people_and_service_users_happy() {
        let person = principal_to_grant(dto(PrincipalKind::User, "user:abc"));
        assert_eq!(
            person,
            Some(SecretGrantPrincipal {
                user_id: "abc".into(),
                label: "Label".into(),
                kind: GrantPrincipalKind::User,
            })
        );
        let svc = principal_to_grant(dto(PrincipalKind::ServiceUser, "svc-1"));
        assert_eq!(svc.map(|g| g.kind), Some(GrantPrincipalKind::ServiceUser));
    }

    #[test]
    fn grants_list_drops_groups_sad() {
        assert!(principal_to_grant(dto(PrincipalKind::Group, "g")).is_none());
    }
}

//! Per-secret Gauge grant management for the Access page.

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
use super::error_tags;
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
    /// Short key for `data-kind` attributes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::ServiceUser => "service_user",
        }
    }
}

/// A person or service user holding a direct grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretGrantPrincipal {
    /// Bare principal id (without the `user:` prefix).
    pub user_id: String,
    /// The name to show: a person's display label from Gauge (may redact email),
    /// or a service user's display name (its handle when the record can't be read).
    pub display_name: String,
    /// Display name of a service user's owner. `None` for people, and when Gauge
    /// can't load the owner.
    pub owner_display_name: Option<String>,
    /// Whether this is a person or a service user.
    pub kind: GrantPrincipalKind,
}

/// Who a grant change is for: the id and kind from the person picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantPrincipal {
    /// Bare user id, or a service user id.
    pub id: String,
    /// Whether `id` names a person or a service user.
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
            "{}Unknown secret action: {other}",
            error_tags::INVALID_INPUT
        ))),
    }
}

/// Parse every action before any write, so one bad name changes nothing.
#[cfg(feature = "ssr")]
fn parse_secret_actions(actions: &[String]) -> Result<Vec<ResourceAction>, ServerFnError> {
    if actions.is_empty() {
        return Err(ServerFnError::new(format!(
            "{}Pick at least one thing they can do.",
            error_tags::INVALID_INPUT
        )));
    }
    let mut parsed = Vec::with_capacity(actions.len());
    for action in actions {
        let action = parse_secret_action(action)?;
        if !parsed.contains(&action) {
            parsed.push(action);
        }
    }
    Ok(parsed)
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

/// Reveal and Maintain need a fresh code on top of the window; one code covers the batch.
#[cfg(feature = "ssr")]
async fn require_fresh_code_if_needed(
    actions: &[ResourceAction],
    totp_code: Option<String>,
    what: &str,
) -> Result<(), ServerFnError> {
    if !actions.iter().copied().any(action_requires_fresh_step_up) {
        return Ok(());
    }
    let code = totp_code.filter(|c| !c.trim().is_empty()).ok_or_else(|| {
        ServerFnError::new(format!(
            "STEP_UP:step_up_required: fresh authenticator code required for this {what}"
        ))
    })?;
    verify_fresh_totp_code(&code).await
}

#[cfg(feature = "ssr")]
fn map_grant_service_err(context: &str, err: impl std::fmt::Display) -> ServerFnError {
    let message = err.to_string();
    if message.contains("not authorized") {
        return ServerFnError::new(format!("{}{message}", error_tags::ACCESS_DENIED));
    }
    if message.contains("Permission not found") {
        return ServerFnError::new(format!("{}{message}", error_tags::NOT_FOUND));
    }
    tracing::warn!(
        target: "neutrino_app",
        context,
        error = %message,
        "secret grant server fn failure"
    );
    ServerFnError::new(format!("{}{context}", error_tags::GAUGE_FAILED))
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
        display_name: principal.label,
        owner_display_name: principal.owner_label,
        kind,
    })
}

/// Gauge labels a service user by its handle; swap in its display name where the
/// caller may read the service user (owners, and admins who manage any).
#[cfg(feature = "ssr")]
async fn service_user_display_name(id: &str, v: &valence::Valence) -> Option<String> {
    use lepton_identity::generated::ServiceUserAccount;
    use valence::Model;

    match ServiceUserAccount::get(
        id,
        v,
        valence::use_!(r"On a secret's **Access** list, we **read a service user's display name** so the person managing access sees the name its owner gave it instead of its handle. Service users the person may not read keep showing their handle."),
    )
    .await
    {
        Ok(Some(row)) => Some(row.display_name().clone()),
        Ok(None) => None,
        Err(e) => {
            tracing::debug!(target: "neutrino_app", error = %e, "service user display name unavailable");
            None
        }
    }
}

/// List direct user grants for each action on a secret.
#[uf_product_macros::server(permission = "SecretsGrantManage")]
pub async fn list_secret_grants(
    /// Secret whose per-action permissions to read.
    secret_id: String,
) -> Result<Vec<SecretActionGrant>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;

    let mut names = std::collections::HashMap::<String, Option<String>>::new();
    let mut rows = Vec::new();
    for action in NEUTRINO_SECRET.actions {
        let permission_id = permission_record_id(NEUTRINO_SECRET, &secret_id, *action);
        let Some(detail) = service::get_permission_detail(&permission_id, &session_v)
            .await
            .map_err(|e| map_grant_service_err("load secret grants", e))?
        else {
            continue;
        };
        let mut user_grants = Vec::new();
        for mut grant in detail.allow_list.into_iter().filter_map(principal_to_grant) {
            if grant.kind == GrantPrincipalKind::ServiceUser {
                if !names.contains_key(&grant.user_id) {
                    let name = service_user_display_name(&grant.user_id, &session_v).await;
                    names.insert(grant.user_id.clone(), name);
                }
                if let Some(Some(name)) = names.get(&grant.user_id) {
                    grant.display_name.clone_from(name);
                }
            }
            user_grants.push(grant);
        }
        rows.push(SecretActionGrant {
            action: action.as_str().to_string(),
            permission_id: detail.id,
            permission_name: detail.name,
            user_grants,
        });
    }
    Ok(rows)
}

/// Grant (`give = true`) or revoke each action for one principal, in order.
#[cfg(feature = "ssr")]
async fn write_actions(
    session_v: &valence::Valence,
    secret_id: &str,
    principal: &GrantPrincipal,
    actions: &[ResourceAction],
    give: bool,
) -> Result<(), ServerFnError> {
    let service_user = match principal.kind {
        GrantPrincipalKind::User => None,
        GrantPrincipalKind::ServiceUser => Some(
            ServiceUserId::parse(principal.id.trim())
                .map_err(|e| ServerFnError::new(format!("{}{e}", error_tags::INVALID_INPUT)))?,
        ),
    };
    let user_id = principal.id.trim();
    for action in actions {
        let permission_id = permission_record_id(NEUTRINO_SECRET, secret_id, *action);
        let result = match (&service_user, give) {
            (None, true) => {
                service::grant_permission_to_user(&permission_id, user_id, session_v).await
            }
            (None, false) => {
                service::revoke_permission_from_user(&permission_id, user_id, session_v).await
            }
            (Some(su), true) => {
                service::grant_permission_to_service_user_checked(&permission_id, su, session_v)
                    .await
            }
            (Some(su), false) => {
                service::revoke_permission_from_service_user(&permission_id, su, session_v).await
            }
        };
        let context = if give {
            "grant secret action"
        } else {
            "revoke secret action"
        };
        result.map_err(|e| map_grant_service_err(context, e))?;
    }
    Ok(())
}

/// Give a person or service user one or more actions on a secret.
///
/// Every action is checked before anything is written, and the batch shares one
/// step-up: the window, plus a fresh code when Reveal or Maintain is in it.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn grant_secret_actions(
    /// Secret the grants are on.
    secret_id: String,
    /// The person or service user receiving the actions.
    principal: GrantPrincipal,
    /// Action names: `View`, `Reveal`, `Edit`, `Delete`, `Maintain`.
    actions: Vec<String>,
    /// Fresh authenticator code, required when the batch has Reveal or Maintain.
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let parsed = parse_secret_actions(&actions)?;
    require_fresh_code_if_needed(&parsed, totp_code, "grant").await?;
    write_actions(&session_v, &secret_id, &principal, &parsed, true).await
}

/// Remove one or more of a person's or service user's actions on a secret.
///
/// Only direct grants on this secret go; access through a Gauge group stays.
/// Same checks and step-up as [`grant_secret_actions`].
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn revoke_secret_actions(
    /// Secret the grants are on.
    secret_id: String,
    /// The person or service user losing the actions.
    principal: GrantPrincipal,
    /// Action names: `View`, `Reveal`, `Edit`, `Delete`, `Maintain`.
    actions: Vec<String>,
    /// Fresh authenticator code, required when the batch has Reveal or Maintain.
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let parsed = parse_secret_actions(&actions)?;
    require_fresh_code_if_needed(&parsed, totp_code, "revoke").await?;
    write_actions(&session_v, &secret_id, &principal, &parsed, false).await
}

/// Change what someone can do on a secret: give `add` and remove `remove` under
/// one step-up, so a single fresh code covers both halves.
#[uf_product_macros::server(permission = "SecretsGrantManage", step_up)]
pub async fn change_secret_actions(
    /// Secret the grants are on.
    secret_id: String,
    /// The person or service user whose actions change.
    principal: GrantPrincipal,
    /// Actions to give.
    add: Vec<String>,
    /// Actions to remove.
    remove: Vec<String>,
    /// Fresh authenticator code, required when either list has Reveal or Maintain.
    totp_code: Option<String>,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let session_v = session_valence_from_ctx(&ctx)?;
    let all: Vec<String> = add.iter().chain(remove.iter()).cloned().collect();
    let parsed_all = parse_secret_actions(&all)?;
    let to_add = if add.is_empty() {
        Vec::new()
    } else {
        parse_secret_actions(&add)?
    };
    let to_remove = if remove.is_empty() {
        Vec::new()
    } else {
        parse_secret_actions(&remove)?
    };
    require_fresh_code_if_needed(&parsed_all, totp_code, "change").await?;
    write_actions(&session_v, &secret_id, &principal, &to_add, true).await?;
    write_actions(&session_v, &secret_id, &principal, &to_remove, false).await
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    fn dto(kind: PrincipalKind, id: &str, owner: Option<&str>) -> PrincipalRefDto {
        PrincipalRefDto {
            kind,
            id: id.to_string(),
            label: "Label".to_string(),
            owner_label: owner.map(str::to_string),
        }
    }

    #[test]
    fn grants_list_keeps_people_and_service_users_happy() {
        let person = principal_to_grant(dto(PrincipalKind::User, "user:abc", None));
        assert_eq!(
            person,
            Some(SecretGrantPrincipal {
                user_id: "abc".into(),
                display_name: "Label".into(),
                owner_display_name: None,
                kind: GrantPrincipalKind::User,
            })
        );
        let svc = principal_to_grant(dto(PrincipalKind::ServiceUser, "svc-1", Some("Ines Okoro")));
        assert_eq!(
            svc.as_ref().map(|g| g.kind),
            Some(GrantPrincipalKind::ServiceUser)
        );
        assert_eq!(
            svc.and_then(|g| g.owner_display_name).as_deref(),
            Some("Ines Okoro")
        );
    }

    #[test]
    fn grants_list_drops_groups_sad() {
        assert!(principal_to_grant(dto(PrincipalKind::Group, "g", None)).is_none());
    }

    #[test]
    fn batch_actions_parse_and_dedupe_happy() {
        let parsed = parse_secret_actions(&["View".into(), "Reveal".into(), "View".into()]);
        assert_eq!(
            parsed.ok(),
            Some(vec![ResourceAction::View, ResourceAction::Reveal])
        );
    }

    #[test]
    fn batch_with_unknown_or_no_action_is_refused_before_writing_sad() {
        let unknown = parse_secret_actions(&["View".into(), "Own".into()]);
        assert!(unknown.is_err_and(|e| e.to_string().contains(error_tags::INVALID_INPUT)));
        assert!(parse_secret_actions(&[]).is_err());
    }

    #[test]
    fn grant_service_errors_are_tagged_sad() {
        let denied = map_grant_service_err("grant", "not authorized to maintain");
        assert!(denied.to_string().contains(error_tags::ACCESS_DENIED));
        let other = map_grant_service_err("grant secret action", "db locked at /tmp/x");
        assert!(other.to_string().contains(error_tags::GAUGE_FAILED));
        assert!(!other.to_string().contains("/tmp/x"));
    }
}

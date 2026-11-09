//! Lab-only seeding for the secret grant person picker specs. Do not mount on a production host.
//!
//! The host links Lepton's `lepton_people` search source through `lepton-shell`, so
//! `POST /api/test/person-lab` only gives seeded users profiles with display names. The
//! `lepton_service_users` source lives in a crate this host doesn't link, so it registers a
//! stand-in that lists the service users the caller owns, as the real one does.

use std::sync::{Arc, Once};

use async_trait::async_trait;
use lepton_identity::generated::{
    ServiceUserAccount, ServiceUserAccountOwnerKind, ServiceUserAccountStatus, User, UserProfile,
    UserStatus, UserUserType,
};
use lepton_identity::service_users::{
    install_service_user_gate, ServiceUserGate, ServiceUserGateError, ServiceUserId,
};
use uf_search_core::{SearchSourceFuture, SearchSourceItem};
use valence::{Model, RecordId, RecordPredicate, Valence};

/// Lets any signed-in user create service users and nobody manage another user's.
struct LabGate;

#[async_trait]
impl ServiceUserGate for LabGate {
    async fn may_create_service_user(&self, _v: &Valence) -> Result<bool, ServiceUserGateError> {
        Ok(true)
    }

    async fn may_manage_any_service_user(
        &self,
        _v: &Valence,
    ) -> Result<bool, ServiceUserGateError> {
        Ok(false)
    }
}

fn install_gate_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if install_service_user_gate(Arc::new(LabGate)).is_err() {
            log::warn!("person lab: a service user gate was already installed");
        }
    });
}

/// Stand-in for Lepton's service user search: the caller's own service users whose name or
/// handle contains the query.
pub struct LabServiceUserSearchSource;

uf_product_macros::define_search_sources! {
    enum LabServiceUserSearchSourceId {
        ServiceUsers => {
            id: "lepton_service_users",
            label: "Service users",
            description: "Lab stand-in for Lepton's service user search",
            provider: LabServiceUserSearchSource
        }
    }
}

impl uf_search_core::SearchSourceProvider for LabServiceUserSearchSource {
    fn query<'a>(
        &'a self,
        v: &'a Valence,
        query_text: &'a str,
        max_results: u32,
    ) -> SearchSourceFuture<'a> {
        Box::pin(async move {
            let needle = query_text.trim().to_lowercase();
            if v.actor().user_id().is_none() || needle.chars().count() < 2 {
                return Ok(Vec::new());
            }
            let rows = ServiceUserAccount::query(v, valence::use_!(r"**Test:** Lab host **lists the caller's service users** for the person picker specs. Valence's read rule limits it to service users the caller owns."))
                .limit(valence::MAX_QUERY_LIMIT)
                .await?;
            Ok(rows
                .iter()
                .filter(|r| {
                    r.display_name().to_lowercase().contains(&needle)
                        || r.handle().to_lowercase().contains(&needle)
                })
                .filter_map(|r| {
                    let rid = r.id()?;
                    let id = valence::extract_id_from_record(rid).ok()?;
                    Some(SearchSourceItem {
                        source_id: lepton_shell::people::sources::SERVICE_USERS.to_string(),
                        id,
                        title: r.display_name().clone(),
                        description: Some("Owned by you".to_string()),
                        kind: "service_user".to_string(),
                        image_url: None,
                    })
                })
                .take(max_results as usize)
                .collect())
        })
    }
}

/// Request body for [`person_lab`].
#[derive(serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PersonLabRequest {
    /// Make sure user `id` exists and has a profile named `display_name`.
    Person { id: String, display_name: String },
    /// Create a service user owned by user `owner`.
    ServiceUser {
        owner: String,
        handle: String,
        display_name: String,
    },
}

fn anyhow_msg(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

async fn seed_person(system: &Valence, id: &str, display_name: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        None,
        Some(UserStatus::Active),
        None,
        None,
        None,
        None,
        None,
        now,
        now,
    )
    .map_err(anyhow_msg)?;
    User::upsert(id, user, system, valence::use_!(r"**Test:** Lab host **saves a user** so the person picker specs can search for them. Only the lab harness uses this."))
        .await?;
    let user_thing = RecordId::new("user", id);
    let existing = UserProfile::query(system, valence::use_!(r"**Test:** Lab host **finds a seeded user's profile** so it can be renamed. Only the lab harness uses this."))
        .where_user(RecordPredicate::Equals(user_thing.clone()))
        .first()
        .await?;
    match existing {
        Some(profile) => {
            profile
                .get_mutable(system, valence::use_!(r"**Test:** Lab host **sets a seeded user's display name** so the person picker specs can search for them. Only the lab harness uses this."))
                .set_display_name(display_name.to_string())
                .map_err(anyhow_msg)?
                .commit()
                .await?;
        }
        None => {
            let profile = UserProfile::new(
                user_thing,
                display_name.to_string(),
                display_name.to_string(),
                now,
                now,
                None,
            )
            .map_err(anyhow_msg)?;
            UserProfile::create(profile, system, valence::use_!(r"**Test:** Lab host **creates a profile** with a display name for a seeded user so the person picker specs can search for them. Only the lab harness uses this."))
                .await?;
        }
    }
    Ok(())
}

async fn seed_service_user(
    system: &Valence,
    owner: &str,
    handle: &str,
    display_name: &str,
) -> anyhow::Result<String> {
    install_gate_once();
    let owner_ref = RecordId::new("user", owner);
    let id = ServiceUserId::new_v4();
    let now = chrono::Utc::now();
    let row = ServiceUserAccount::new(
        Some(owner_ref.clone()),
        ServiceUserAccountOwnerKind::User,
        Some(owner_ref),
        handle.to_string(),
        handle.replace('-', ""),
        display_name.to_string(),
        None,
        ServiceUserAccountStatus::Active,
        None,
        now,
        now,
    )
    .map_err(anyhow_msg)?;
    let owner_valence = system.with_actor(valence::Actor::User {
        user_id: owner.to_string(),
    });
    ServiceUserAccount::upsert(id.as_str(), row, &owner_valence, valence::use_!(r"**Test:** Lab host **creates a service user owned by a seeded user**, as that user, so the secret grant specs can grant to it. Only the lab harness uses this."))
        .await?;
    Ok(id.as_str().to_string())
}

/// Lab-only seed route for the secret grant person picker specs.
pub async fn person_lab(
    axum::Json(body): axum::Json<PersonLabRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let fail = |e: anyhow::Error| (axum::http::StatusCode::BAD_REQUEST, format!("{e:#}"));
    let system = crate::e2e_valence::e2e_system_valence();
    match body {
        PersonLabRequest::Person { id, display_name } => {
            seed_person(&system, &id, &display_name)
                .await
                .map_err(fail)?;
            Ok(axum::Json(serde_json::json!({ "user_id": id })))
        }
        PersonLabRequest::ServiceUser {
            owner,
            handle,
            display_name,
        } => {
            let id = seed_service_user(&system, &owner, &handle, &display_name)
                .await
                .map_err(fail)?;
            Ok(axum::Json(serde_json::json!({ "service_user_id": id })))
        }
    }
}

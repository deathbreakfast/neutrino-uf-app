//! Harness-only seed endpoint for Playwright.

use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;

use crate::e2e_valence::{e2e_fixtures, HARNESS_TOTP_SECRET};
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `admin` | `operator`/`requestor` | `outsider` | `unverified`
    #[serde(default = "default_auth")]
    pub auth: String,
    /// When true (default for authenticated seeds), open a valid TOTP sudo window.
    /// Prefer [`Self::step_up_window`] when you need `expired`.
    #[serde(default)]
    pub grant_step_up_window: Option<bool>,
    /// `valid` | `expired` | `none` | `other_user` — overrides [`Self::grant_step_up_window`] when set.
    /// `other_user` writes a time-valid window bound to a different user than the session (TM-3).
    #[serde(default)]
    pub step_up_window: Option<String>,
    /// Lab-only: `auto` (default) | `empty` — controls fresh-TOTP supply for reveal.
    #[serde(default)]
    pub fresh_totp: Option<String>,
    /// When true, put admin back on Super User so break-glass reveal can succeed.
    #[serde(default)]
    pub promote_super_user: Option<bool>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepUpWindowKind {
    None,
    Valid,
    Expired,
    /// Time-valid window whose `STEP_UP_USER_ID_KEY` does not match the session user.
    OtherUser,
}

fn resolve_window_kind(body: &SeedRequest, kind: E2eAuthKind) -> StepUpWindowKind {
    if let Some(raw) = body.step_up_window.as_deref() {
        return match raw.trim() {
            "valid" | "ok" | "true" => StepUpWindowKind::Valid,
            "expired" => StepUpWindowKind::Expired,
            "other_user" => StepUpWindowKind::OtherUser,
            "none" | "false" | "" => StepUpWindowKind::None,
            _ => StepUpWindowKind::None,
        };
    }
    match body.grant_step_up_window {
        Some(true) => StepUpWindowKind::Valid,
        Some(false) => StepUpWindowKind::None,
        None => {
            if matches!(
                kind,
                E2eAuthKind::Admin | E2eAuthKind::Requestor | E2eAuthKind::Outsider
            ) {
                StepUpWindowKind::Valid
            } else {
                StepUpWindowKind::None
            }
        }
    }
}

fn bound_user_for_window(kind: StepUpWindowKind, session_user_id: &str) -> String {
    match kind {
        StepUpWindowKind::OtherUser => {
            if session_user_id == "admin" {
                "requestor".to_string()
            } else {
                "admin".to_string()
            }
        }
        _ => session_user_id.to_string(),
    }
}

async fn clear_step_up_window(session: &tower_sessions::Session) {
    let _ = session
        .remove::<i64>(uf_product::permissions::STEP_UP_VERIFIED_AT_KEY)
        .await;
    let _ = session
        .remove::<i64>(uf_product::permissions::STEP_UP_EXPIRES_AT_KEY)
        .await;
    let _ = session
        .remove::<String>(uf_product::permissions::STEP_UP_USER_ID_KEY)
        .await;
    let _ = session
        .remove::<Vec<u8>>(uf_product::permissions::STEP_UP_AUTH_HASH_KEY)
        .await;
    let _ = session
        .remove::<String>(uf_product::permissions::STEP_UP_SCOPE_KEY)
        .await;
}

async fn write_step_up_window(
    session: &tower_sessions::Session,
    session_user_id: &str,
    kind: StepUpWindowKind,
) -> Result<(), StatusCode> {
    let now = Utc::now();
    let (verified_at, expires_at) = match kind {
        StepUpWindowKind::None => return Ok(()),
        StepUpWindowKind::Valid | StepUpWindowKind::OtherUser => (
            now,
            now + chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS),
        ),
        StepUpWindowKind::Expired => {
            let expired =
                now - chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS + 30);
            (expired, expired)
        }
    };
    let bound_user = bound_user_for_window(kind, session_user_id);
    session
        .insert(
            uf_product::permissions::STEP_UP_VERIFIED_AT_KEY,
            verified_at.timestamp(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_EXPIRES_AT_KEY,
            expires_at.timestamp(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(uf_product::permissions::STEP_UP_USER_ID_KEY, bound_user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_AUTH_HASH_KEY,
            b"e2e-auth-hash".to_vec(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_SCOPE_KEY,
            uf_product::permissions::STEP_UP_SCOPE_SENSITIVE.to_string(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    clear_step_up_window(&session).await;

    let window_kind = resolve_window_kind(&body, kind);
    let mut step_up_window = "none";
    if window_kind != StepUpWindowKind::None {
        if let Some(uid) = kind.session_user_id() {
            write_step_up_window(&session, uid, window_kind).await?;
            step_up_window = match window_kind {
                StepUpWindowKind::Valid => "valid",
                StepUpWindowKind::Expired => "expired",
                StepUpWindowKind::OtherUser => "other_user",
                StepUpWindowKind::None => "none",
            };
        }
    }

    let fresh_totp = match body.fresh_totp.as_deref().map(str::trim) {
        Some("empty") => "empty",
        _ => "auto",
    };
    neutrino_app::e2e_lab::set_fresh_totp_mode(Some(fresh_totp));

    if body.promote_super_user == Some(true) {
        let system = crate::e2e_valence::e2e_system_valence();
        crate::e2e_valence::promote_admin_to_super_user(&system).await;
    }

    let fixtures = e2e_fixtures();

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "step_up_window": step_up_window,
        "fresh_totp": fresh_totp,
        "promote_super_user": body.promote_super_user.unwrap_or(false),
        "totp_secret": HARNESS_TOTP_SECRET,
        "fixtures": {
            "outsider_secret_id": fixtures.outsider_secret_id,
            "outsider_secret_name": fixtures.outsider_secret_name,
            "admin_secret_id": fixtures.admin_secret_id,
            "admin_secret_name": fixtures.admin_secret_name,
        }
    })))
}

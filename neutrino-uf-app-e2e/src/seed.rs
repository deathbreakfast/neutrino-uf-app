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
    /// `valid` | `expired` | `none` — overrides [`Self::grant_step_up_window`] when set.
    #[serde(default)]
    pub step_up_window: Option<String>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepUpWindowKind {
    None,
    Valid,
    Expired,
}

fn resolve_window_kind(body: &SeedRequest, kind: E2eAuthKind) -> StepUpWindowKind {
    if let Some(raw) = body.step_up_window.as_deref() {
        return match raw.trim() {
            "valid" | "ok" | "true" => StepUpWindowKind::Valid,
            "expired" => StepUpWindowKind::Expired,
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
        StepUpWindowKind::Valid => (
            now,
            now + chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS),
        ),
        StepUpWindowKind::Expired => {
            let expired =
                now - chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS + 30);
            (expired, expired)
        }
    };
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
        .insert(
            uf_product::permissions::STEP_UP_USER_ID_KEY,
            session_user_id.to_string(),
        )
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
                StepUpWindowKind::None => "none",
            };
        }
    }

    let fixtures = e2e_fixtures();

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "step_up_window": step_up_window,
        "totp_secret": HARNESS_TOTP_SECRET,
        "fixtures": {
            "outsider_secret_id": fixtures.outsider_secret_id,
            "outsider_secret_name": fixtures.outsider_secret_name,
            "admin_secret_id": fixtures.admin_secret_id,
            "admin_secret_name": fixtures.admin_secret_name,
        }
    })))
}

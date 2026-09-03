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
    /// When true (default for authenticated seeds), open a TOTP sudo window.
    #[serde(default = "default_grant_step_up")]
    pub grant_step_up_window: Option<bool>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

fn default_grant_step_up() -> Option<bool> {
    None
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
) -> Result<(), StatusCode> {
    let now = Utc::now();
    let expires = now + chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS);
    session
        .insert(
            uf_product::permissions::STEP_UP_VERIFIED_AT_KEY,
            now.timestamp(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_EXPIRES_AT_KEY,
            expires.timestamp(),
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

    let grant_window = body.grant_step_up_window.unwrap_or(matches!(
        kind,
        E2eAuthKind::Admin | E2eAuthKind::Requestor | E2eAuthKind::Outsider
    ));
    let mut step_up_window = false;
    if grant_window {
        if let Some(uid) = kind.session_user_id() {
            write_step_up_window(&session, uid).await?;
            step_up_window = true;
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

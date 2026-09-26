//! Lab-only step-up helpers for `neutrino-uf-app-e2e` (feature `e2e-lab`).
//!
//! Playwright hosts seed Higgs session snapshots without axum-login. Fresh reveal
//! verifies against `TotpFactor` via Higgs + System Valence; the client obtains a
//! current code from [`e2e_lab_totp_code`] instead of mounting axum-login.

use std::sync::atomic::{AtomicU8, Ordering};

use leptos::prelude::*;

/// 0 = auto (generate harness TOTP), 1 = empty string (force fresh verify fail).
static FRESH_TOTP: AtomicU8 = AtomicU8::new(0);

/// `auto` | `empty` — controls [`e2e_lab_totp_code`] lab path (server-side).
pub fn set_fresh_totp_mode(mode: Option<&str>) {
    let v = match mode {
        Some("empty") => 1,
        _ => 0,
    };
    FRESH_TOTP.store(v, Ordering::SeqCst);
    // SAFETY: lab host only.
    unsafe {
        match mode {
            Some("empty") => std::env::set_var("NEUTRINO_E2E_FRESH_TOTP", "empty"),
            _ => std::env::set_var("NEUTRINO_E2E_FRESH_TOTP", "auto"),
        }
    }
}

fn fresh_totp_empty() -> bool {
    if FRESH_TOTP.load(Ordering::SeqCst) == 1 {
        return true;
    }
    matches!(
        std::env::var("NEUTRINO_E2E_FRESH_TOTP").as_deref(),
        Ok("empty")
    )
}

/// RFC 6238 fixture secret (base32). Must match e2e Valence `TotpFactor` seed.
#[cfg(feature = "ssr")]
pub const HARNESS_TOTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// Current 6-digit TOTP for the harness fixture secret (lab only).
/// When seed set `fresh_totp: "empty"`, returns an empty code so verify fails (TM-12).
#[server(E2eLabTotpCode)]
pub async fn e2e_lab_totp_code() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        if fresh_totp_empty() {
            return Ok(String::new());
        }
        use chrono::Utc;
        use totp_rs::{Algorithm, Secret, TOTP};

        let ctx = higgs::Higgs::from_request()
            .await
            .map_err(|_| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
        if ctx.session_user_id().is_none() {
            return Err(ServerFnError::new(
                "STEP_UP:auth_required: authentication required",
            ));
        }
        let secret = Secret::Encoded(HARNESS_TOTP_SECRET.to_string())
            .to_bytes()
            .map_err(|_| ServerFnError::new("STEP_UP:totp_secret: invalid harness secret"))?;
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret)
            .map_err(|_| ServerFnError::new("STEP_UP:totp_secret: totp build failed"))?;
        return Ok(totp.generate(Utc::now().timestamp() as u64));
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// Reset the session user's harness `TotpFactor` so `last_used_step` is clear.
///
/// Shared in-memory Valence + lepton same-step replay would otherwise fail a second
/// (or cross-test) fresh reveal within the same TOTP window. Lab harness only.
#[cfg(all(feature = "ssr", feature = "e2e-lab-ssr"))]
async fn clear_lab_totp_replay_guard(session_uid: &str) -> Result<(), ServerFnError> {
    use chrono::Utc;
    use valence::Model;

    let ctx = higgs::Higgs::from_request()
        .await
        .map_err(|_| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
    // TotpFactor is SYSTEM_ONLY — same path as lepton_auth verify. Test harness only.
    let system = ctx
        .unsafe_system_valence()
        .map_err(|_| ServerFnError::new("STEP_UP:store: system valence unavailable"))?;
    let bare = session_uid.split(':').next_back().unwrap_or(session_uid);
    let factor_id = format!("totp-{bare}");
    let now = Utc::now();
    let factor = lepton::generated::TotpFactor::new(
        valence::RecordId::new("user", bare),
        HARNESS_TOTP_SECRET.to_string(),
        None,
        None,
        None,
        Some(now),
        Some(now),
        now,
        now,
    )
    .map_err(|e| ServerFnError::new(format!("STEP_UP:store: {e}")))?;
    lepton::generated::TotpFactor::upsert_used(
        &factor_id,
        factor,
        &system,
        valence::use_!(r#"**Test:** Lab **Totp Factor** re-seed clears last_used_step so consecutive Playwright fresh reveals can reuse the fixture TOTP within one 30s window. CI and developers running the suite only."#),
    )
    .await
    .map_err(|e| ServerFnError::new(format!("STEP_UP:store: {e}")))?;
    Ok(())
}

/// Verify a fresh TOTP for the Higgs session user (lab substitute for axum-login).
#[cfg(feature = "ssr")]
pub async fn verify_fresh_totp(code: &str) -> Result<(), ServerFnError> {
    if code.trim().is_empty() {
        return Err(ServerFnError::new(
            "STEP_UP:invalid_totp: fresh TOTP code required",
        ));
    }
    let ctx = higgs::Higgs::from_request()
        .await
        .map_err(|_| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
    let session_uid = ctx
        .session_user_id()
        .ok_or_else(|| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
    #[cfg(feature = "e2e-lab-ssr")]
    clear_lab_totp_replay_guard(session_uid).await?;
    lepton_auth::verify_fresh_totp_for_session_user(session_uid, code)
        .await
        .map_err(|e| e.to_server_fn_error())
}

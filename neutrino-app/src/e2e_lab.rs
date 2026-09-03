//! Lab-only step-up helpers for `neutrino-uf-app-e2e` (feature `e2e-lab`).
//!
//! Playwright hosts seed Higgs session snapshots without axum-login. Fresh reveal
//! verifies against `TotpFactor` via Higgs + System Valence; the client obtains a
//! current code from [`e2e_lab_totp_code`] instead of mounting axum-login.

use leptos::prelude::*;

/// RFC 6238 fixture secret (base32). Must match e2e Valence `TotpFactor` seed.
#[cfg(feature = "ssr")]
pub const HARNESS_TOTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// Current 6-digit TOTP for the harness fixture secret (lab only).
#[server(E2eLabTotpCode)]
pub async fn e2e_lab_totp_code() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
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

/// Verify a fresh TOTP for the Higgs session user (lab substitute for axum-login).
#[cfg(feature = "ssr")]
pub async fn verify_fresh_totp(code: &str) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request()
        .await
        .map_err(|_| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
    let session_uid = ctx
        .session_user_id()
        .ok_or_else(|| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
    lepton_auth::verify_fresh_totp_for_session_user(session_uid, code)
        .await
        .map_err(|e| e.to_server_fn_error())
}

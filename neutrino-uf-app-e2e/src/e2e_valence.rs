//! Process-wide Valence + Higgs for Playwright (neutrino sqlite mem + Secrets* grants).
#![allow(dead_code)]

use std::sync::{Arc, Mutex, OnceLock};

use chrono::Utc;
use gauge::manifest_sync::{
    sync_permission_manifests, PermissionDomainInput, PermissionInput, PermissionManifestInput,
};
use gauge::service;
use gauge::super_user::SUPER_USER_GROUP_NAME;
use gauge::touch_schema_inventory;
use higgs::{HiggsConfig, HiggsValenceFactory};
use neutrino::create_initial_neutrino_groups;
use neutrino::vault::{create_vault_secret, store_from_valence_for_request};
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter, Model,
    RegisterBackendLogicalNamesOptions, RouterValenceFactory, RouterValenceFactoryConfig,
    SqliteBackend, Valence, ValenceFactory, SQLITE_ENGINE_ID,
};

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    default_backend_key: String,
    fixtures: Mutex<FixtureIds>,
}

/// Stable fixture ids exposed to seed JSON / Playwright.
#[derive(Clone, Debug, Default)]
pub struct FixtureIds {
    /// Pre-seeded secret owned by `outsider` (for reveal-denied sad path).
    pub outsider_secret_id: String,
    pub outsider_secret_name: String,
    /// Pre-seeded secret owned by `admin` (optional list fixture).
    pub admin_secret_id: String,
    pub admin_secret_name: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
        // Always pin a valid 64-hex key: parent shells often export a weak
        // placeholder that would otherwise skip this branch and panic at boot.
        std::env::set_var("NEUTRINO_MASTER_KEY", "0".repeat(64));
        std::env::remove_var("NEUTRINO_ALLOW_WEAK_MASTER_KEY");
        // Lab TOTP seal key for step-up verify / lazy re-seal.
        std::env::set_var("LEPTON_TOTP_ALLOW_TEST_SEAL_KEY", "1");
    }
}

/// RFC 6238 fixture secret seeded onto e2e users (matches `neutrino_app::e2e_lab`).
pub const HARNESS_TOTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

async fn seed_enabled_totp(user_id: &str, valence: &Valence) {
    let now = Utc::now();
    let factor_id = format!("totp-{user_id}");
    let user_rec = valence::RecordId::new("user", user_id);
    let factor = lepton::generated::TotpFactor::new(
        user_rec,
        HARNESS_TOTP_SECRET.to_string(),
        None,
        None,
        None,
        Some(now),
        Some(now),
        now,
        now,
    )
    .expect("totp factor");
    lepton::generated::TotpFactor::upsert_used(&factor_id, factor, valence, valence::use_!("upsert TotpFactor in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("upsert totp");
}

async fn seed_user(id: &str, email_verified: bool, valence: &Valence) {
    let now = Utc::now();
    let confirmed_at = email_verified.then_some(now);
    let user = lepton::generated::User::new(
        Some(lepton::generated::UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(lepton::generated::UserStatus::Active),
        None,
        None,
        confirmed_at,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    lepton::generated::User::upsert_used(id, user, valence, valence::use_!("upsert User in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("upsert user");
}

async fn add_user_to_creators_group(user_id: &str, v: &Valence) {
    let group = gauge::generated::PermissionGroup::get_used("neutrino.secret.creators", v, valence::use_!("get PermissionGroup in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("get creators group")
        .expect("neutrino.secret.creators");
    let user = lepton::generated::User::get_used(user_id, v, valence::use_!("get User in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("get user")
        .expect("user row");
    let principal = gauge::generated::PermissionUserPrincipal::upsert_used(
        &format!("user:{user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            user.id().expect("user id").clone(),
            user_id.to_string(),
        )
        .expect("principal"),
        v,
        valence::use_!("upsert PermissionUserPrincipal in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."),
    )
    .await
    .expect("upsert principal");
    group
        .relate_to_member_record(principal.id().expect("principal id"), v)
        .await
        .expect("relate member");
}

async fn seed_super_user_with_member(system: &Valence, member_user_id: &str) {
    let super_group = gauge::generated::PermissionGroup::new(
        SUPER_USER_GROUP_NAME.to_string(),
        Some("super users".to_string()),
        Utc::now(),
        Utc::now(),
    )
    .expect("build super user group");
    let created =
        gauge::generated::PermissionGroup::upsert_used("super_user_group", super_group, system, valence::use_!("upsert PermissionGroup in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
            .await
            .expect("upsert super user group");

    let member = lepton::generated::User::get_used(member_user_id, system, valence::use_!("get User in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("query member")
        .expect("member exists");
    let principal = gauge::generated::PermissionUserPrincipal::upsert_used(
        &format!("user:{member_user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            member.id().expect("member id").clone(),
            member_user_id.to_string(),
        )
        .expect("new principal"),
        system,
        valence::use_!("upsert PermissionUserPrincipal in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."),
    )
    .await
    .expect("upsert principal");
    created
        .relate_to_owner_record(principal.id().expect("principal id"), system)
        .await
        .expect("relate super owner");
    created
        .relate_to_member_record(principal.id().expect("principal id"), system)
        .await
        .expect("relate super member");
}

async fn demote_admin_from_super_user(system: &Valence) {
    let Some(super_group) = gauge::generated::PermissionGroup::get_used("super_user_group", system, valence::use_!("get PermissionGroup in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("get super user group")
    else {
        return;
    };
    let Some(principal) = gauge::generated::PermissionUserPrincipal::get_used("user:admin", system, valence::use_!("get PermissionUserPrincipal in neutrino-uf-app-e2e/src/e2e_valence.rs; Valence persistence for this feature path; typed store; visible to test harness."))
        .await
        .expect("get admin principal")
    else {
        return;
    };
    let pid = principal.id().expect("principal id").clone();
    let _ = super_group.unrelate_from_member_record(&pid, system).await;
    let _ = super_group.unrelate_from_owner_record(&pid, system).await;
}

/// Lab seed: put admin back on Super User (break-glass reveal paths).
pub async fn promote_admin_to_super_user(system: &Valence) {
    seed_super_user_with_member(system, "admin").await;
}

fn secrets_manifest() -> PermissionManifestInput {
    PermissionManifestInput {
        app_id: "secrets".into(),
        domains: vec![PermissionDomainInput {
            key: "secrets".into(),
            name: "Secrets".into(),
            description: "Neutrino encrypted secret vault".into(),
            permissions: vec![
                PermissionInput {
                    name: "SecretsRead".into(),
                    description: "View secret metadata".into(),
                },
                PermissionInput {
                    name: "SecretsReveal".into(),
                    description: "Reveal secret plaintext (audited)".into(),
                },
                PermissionInput {
                    name: "SecretsWrite".into(),
                    description: "Create or update secrets".into(),
                },
                PermissionInput {
                    name: "SecretsRotate".into(),
                    description: "Trigger rotation".into(),
                },
            ],
        }],
    }
}

async fn grant_named(admin_ctx: &Valence, name: &str, user_id: &str) {
    let perms = service::list_permissions(admin_ctx, None)
        .await
        .expect("list permissions");
    let perm = perms
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("missing permission {name} after sync"));
    service::grant_permission_to_user(&perm.id, user_id, admin_ctx)
        .await
        .unwrap_or_else(|e| panic!("grant {name} to {user_id}: {e}"));
}

/// Build shared Valence/Higgs once and seed baseline fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    // Neutrino + Gauge share LOGICAL_NAME "permissions" on SQLITE_ENGINE_ID.
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        neutrino::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions::default(),
    );
    let router = Arc::new(router);
    let default_key = router_key(neutrino::embedded_surreal::LOGICAL_NAME, SQLITE_ENGINE_ID);

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_neutrino_host".into(),
        })
        .build()
        .expect("e2e Valence");

    touch_schema_inventory();
    create_initial_neutrino_groups(&system)
        .await
        .expect("create_initial_neutrino_groups");

    seed_user("admin", true, &system).await;
    seed_user("requestor", true, &system).await;
    seed_user("outsider", true, &system).await;
    seed_user("unverified", false, &system).await;

    seed_enabled_totp("admin", &system).await;
    seed_enabled_totp("requestor", &system).await;
    seed_enabled_totp("outsider", &system).await;

    add_user_to_creators_group("admin", &system).await;
    add_user_to_creators_group("requestor", &system).await;
    add_user_to_creators_group("outsider", &system).await;

    seed_super_user_with_member(&system, "admin").await;

    sync_permission_manifests(&system, &[secrets_manifest()])
        .await
        .expect("sync Secrets* manifest");

    let admin_ctx = system.with_actor(Actor::User {
        user_id: "admin".to_string(),
    });

    for name in [
        "SecretsRead",
        "SecretsReveal",
        "SecretsWrite",
        "SecretsRotate",
    ] {
        grant_named(&admin_ctx, name, "admin").await;
    }
    // Operator (requestor): read+write+rotate for happy partial flows if needed.
    for name in ["SecretsRead", "SecretsWrite", "SecretsRotate"] {
        grant_named(&admin_ctx, name, "requestor").await;
    }
    // Outsider: metadata read only (reveal/write deny sad paths).
    grant_named(&admin_ctx, "SecretsRead", "outsider").await;

    let fixtures = bootstrap_fixtures(&system)
        .await
        .expect("bootstrap fixtures");

    demote_admin_from_super_user(&system).await;

    // Higgs SSR factory uses internal trust so `unsafe_system_valence` can mint
    // System for Neutrino SYSTEM_ONLY sealed-store ORM (same as embedded
    // ProcessValenceFactory::as_higgs_factory). Do not install
    // RejectExternalSystemActor here — that belongs on worker/enqueue factories.
    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone()),
    )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        default_backend_key: default_key,
        fixtures: Mutex::new(fixtures),
    });
    let _ = E2E_STATE.set(state);
}

async fn bootstrap_fixtures(system: &Valence) -> anyhow::Result<FixtureIds> {
    let admin_store = store_from_valence_for_request(system.clone(), "user:admin");
    let admin_row = create_vault_secret(
        &admin_store,
        "e2e-admin-seed".into(),
        "/e2e/admin".into(),
        "token".into(),
        "admin-seed-pt".into(),
        "user:admin".into(),
    )
    .await?;

    let outsider_store = store_from_valence_for_request(system.clone(), "user:outsider");
    let outsider_row = create_vault_secret(
        &outsider_store,
        "e2e-outsider-seed".into(),
        "/e2e/outsider".into(),
        "token".into(),
        "outsider-seed-pt".into(),
        "user:outsider".into(),
    )
    .await?;

    Ok(FixtureIds {
        outsider_secret_id: outsider_row.id,
        outsider_secret_name: "e2e-outsider-seed".into(),
        admin_secret_id: admin_row.id,
        admin_secret_name: "e2e-admin-seed".into(),
    })
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_fixtures() -> FixtureIds {
    state().fixtures.lock().expect("fixtures").clone()
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}

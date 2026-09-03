#![recursion_limit = "256"]
//! Encrypted secret vault admin UI (`/secrets`).
//!
//! Orbital app on top of the [`neutrino`] domain crate. Registers under
//! `/secrets` and requires an authenticated, verified session before rendering.
//! Hosts supply session chrome and identity; sealed-store crypto and Gauge
//! bootstrap live in `neutrino`.
//!
//! ## Features
//!
//! - **Secrets admin routes** — Nest [`NeutrinoRoutes`] under the host router to
//!   expose list, create, reveal, rotate, and delete pages at `/secrets`, gated
//!   by an authenticated verified session. Mount once when composing the host
//!   route tree at startup. [Get started](#mount-neutrino-routes).
//! - **Help spotlight tours** — Route-scoped Orbital spotlights that teach the
//!   vault list and ACL placeholder. Call [`ensure_help_steps_linked`] so inventory
//!   links into the host; enable `offering-help` on the product shell.
//!   [Get started](#help-spotlight-tours).
//!
//! Layout and guard: [`NeutrinoAppLayout`], [`NeutrinoVerifiedGuardRouteView`].
//! Pages: [`pages::SecretsListPage`], [`pages::AclManagePage`]. Server wrappers:
//! [`mod@server`]. Permissions: [`permissions::NeutrinoPermission`].
//! Help inventory: [`mod@help_steps`].
//!
//! ## Module map
//!
//! | Module | Role |
//! |--------|------|
//! | [`layout`] | Orbital shell (app bar + nav) wrapping routed pages |
//! | [`pages`] | Vault list and ACL placeholder route pages |
//! | [`mod@help_steps`] | Help spotlight tour inventory; call [`ensure_help_steps_linked`] |
//! | [`mod@server`] | Higgs `#[server]` wrappers over `neutrino::vault` |
//! | [`permissions`] | `Secrets*` permission manifest for host registration |
//! | [`shell`] | Re-exports shared `uf-integrations` shell components |
//!
//! ## Getting started
//!
//! Composite hosts mount [`NeutrinoRoutes`] inside `<Routes>` so operators reach
//! the vault UI without wiring pages by hand. Full sequence (Gauge bootstrap,
//! permission grants, hydrate variant):
//! [Mount Neutrino routes](#mount-neutrino-routes).
//!
//! ## Mount Neutrino routes
//!
//! [`NeutrinoRoutes`] is the Orbital route tree for the Secrets app: vault list and
//! ACL placeholder pages nested under an authenticated+verified guard. Mount it
//! once when building the host Axum/Leptos router at startup, after session chrome
//! and identity are available. Server functions are gated per-action by
//! [`permissions::NeutrinoPermission`] (`SecretsRead`, `SecretsReveal`,
//! `SecretsWrite`, `SecretsRotate`, plus reserved grant/audit/master-key
//! capabilities).
//!
//! **Prerequisites:** Valence, Lepton session, and Higgs on the host; `neutrino`
//! and `neutrino-app` with `feature = "ssr"` on the server binary (and `hydrate`
//! on the client bundle).
//!
//! 1. Depend on `neutrino-app` (and `neutrino`) with `ssr` / `hydrate` aligned to
//!    the host.
//! 2. At worker boot, call `neutrino::create_initial_neutrino_groups` so Gauge
//!    resource groups exist before vault or UI traffic. Domain teaching for that
//!    call lives under **Gauge bootstrap at boot** in the `neutrino` crate docs
//!    (`cargo doc -p neutrino --features ssr --open`).
//! 3. Mount `<NeutrinoRoutes />` under the host `<Routes>`.
//! 4. Grant operators the Gauge permissions in [`permissions::NeutrinoPermission`]
//!    that match their role (`SecretsRead` for list metadata; `SecretsReveal` for
//!    plaintext; `SecretsWrite` for create/delete; `SecretsRotate` for rotation).
//!    Sync the manifest with host permission registration (`secrets` / `/secrets`).
//!
//! ```rust,ignore
//! use neutrino_app::NeutrinoRoutes;
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//!
//! view! {
//!     <Routes fallback=|| "not found">
//!         <NeutrinoRoutes />
//!     </Routes>
//! }
//! ```
//!
//! On success `/secrets` resolves to the vault list (ACL placeholder at
//! `/secrets/acl`). Without `feature = "ssr"` on the server binary, server fns
//! do not compile into the host. Without an authenticated verified session, the
//! guard does not render vault pages (operators see the host login / verify
//! flow). Mutations without the matching `Secrets*` permission fail at the
//! Higgs `#[uf_product_macros::server(permission = "…")]` gate.
//!
//! Next: open `/secrets` in the running host, or exercise domain APIs without UI
//! via `cargo run -p vault-host` in the sibling `neutrino` workspace.
//!
//! ### Variant: hydrate-only client
//!
//! Client bundles use `feature = "hydrate"` without `ssr`. Mount still uses the same
//! [`NeutrinoRoutes`] tree; server fns execute on the SSR host. If the host omitted
//! `ssr`, list/create/reveal calls fail at the Leptos server-fn boundary rather than
//! at route mount.
//!
//! ## Help spotlight tours
//!
//! Secrets ships Orbital Help spotlights for the vault list (`/secrets`) and ACL
//! placeholder (`/secrets/acl`). Hosts that enable `offering-help` (or `full`) mount
//! `HelpTourPlayer`. Call [`ensure_help_steps_linked`] once at host startup (when
//! mounting routes) so `inventory` submissions from [`mod@help_steps`] are retained
//! and tours can run.
//!
//! **Prerequisites:** `uf-help` hydrate/ssr features on this crate; product host with
//! Help player mounted (`uf-integrations` `offering-help` or `full`); authenticated
//! session when Valence visit tracking is enabled.
//!
//! ```rust,ignore
//! use neutrino_app::{ensure_help_steps_linked, NeutrinoRoutes};
//!
//! ensure_help_steps_linked();
//! // Mount <NeutrinoRoutes /> under the host <Routes>.
//! ```
//!
//! On success, visiting `/secrets` (and `/secrets/acl`) can show pending spotlight
//! steps. Replay restarts the tour for the current route via the Help menu. If the
//! host omitted `offering-help`, `HelpTourPlayer` is absent and steps never appear
//! even when inventory linked. Skipping `ensure_help_steps_linked` drops inventory
//! submissions so the player has nothing to show.
//!
//! Next: open `/secrets` in a host with Help enabled, or follow
//! [Mount Neutrino routes](#mount-neutrino-routes) if the route tree is not mounted yet.
//!
//! ## Feature flags
//!
//! | Flag | What it enables |
//! |------|-----------------|
//! | `hydrate` | Client hydrate for Leptos / Orbital shell |
//! | `ssr` | Server functions, Valence, Higgs, Gauge, and `neutrino/ssr` |
//!
//! ## Examples
//!
//! - Mount path: [Mount Neutrino routes](#mount-neutrino-routes)
//! - Help tours: [Help spotlight tours](#help-spotlight-tours)
//! - Domain contracts: `cargo test -p neutrino --features ssr --test vault_crud_contract`
//! - Domain host without UI: `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-neutrino cargo run -p vault-host`

#![allow(missing_docs)] // uf_app! / routes macros emit undocumented associated items

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

/// Help spotlight tour inventory ([`mod@help_steps`]).
pub mod help_steps;
/// Shell layout wrapping routed pages ([`NeutrinoAppLayout`]).
pub mod layout;
mod lazy_routes;
pub mod pages;
pub mod permissions;
pub mod server;
pub mod shell;

/// IsolatedLab fresh-TOTP helpers (enabled with Cargo feature `e2e-lab`).
#[cfg(feature = "e2e-lab")]
pub mod e2e_lab;

pub use help_steps::ensure_help_steps_linked;
pub use layout::NeutrinoAppLayout;
pub use lazy_routes::{
    prefetch_family, AclManageRoute, NeutrinoVerifiedGuardRouteView, SecretsListRoute,
};
pub use pages::{AclManagePage, SecretsListPage};

// `uf_app!` expands public const fields on [`AppMetadata`] without per-field docs.
uf_app! {
    name: "Secrets",
    id: "secrets",
    description: "Encrypted secret vault (Neutrino)",
    icon: "🔐",
    version: "0.1.0",
    routes: NeutrinoRoutes,
    route_path: "/secrets",
    permission_manifest: permissions::NeutrinoPermission,
}

/// Route tree for the Secrets app: vault list and ACL placeholder pages, nested
/// under an authenticated+verified guard.
// `orbital_routes_extract` emits helper items without docs.
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn NeutrinoRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    crate::help_steps::ensure_help_steps_linked();
    view! {
        <ParentRoute path=path!("secrets") view=NeutrinoVerifiedGuardRouteView>
            <Route path=path!("") view={Lazy::<SecretsListRoute>::new()} />
            <Route path=path!("acl") view={Lazy::<AclManageRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}

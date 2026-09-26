//! Lazy-loaded route views for WASM code-splitting (`cargo leptos --split`).

#![allow(clippy::used_underscore_binding)]

use leptos::prelude::*;
use leptos_router::{lazy_route, LazyRoute};

use crate::layout::NeutrinoAppLayout;
use crate::pages::{AclManagePage, SecretsListPage};

/// Prefetch the secrets family WASM chunk (leaf pages share split modules).
pub async fn prefetch_family() {
    SecretsListRoute::preload().await;
}

/// Eager authenticated+verified guard shell for `/secrets/*` ParentRoute.
#[component]
pub fn NeutrinoVerifiedGuardRouteView() -> impl IntoView {
    view! {
        <uf_product::routes::RequireAuthenticated requires_email_verification=true>
            <NeutrinoAppLayout />
        </uf_product::routes::RequireAuthenticated>
    }
}

/// Lazy `/secrets` vault list page.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecretsListRoute;

#[lazy_route]
impl LazyRoute for SecretsListRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <SecretsListPage /> }.into_any()
    }
}

/// Lazy `/secrets/acl` ACL manage page.
#[derive(Clone, Copy, Debug, Default)]
pub struct AclManageRoute;

#[lazy_route]
impl LazyRoute for AclManageRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <AclManagePage /> }.into_any()
    }
}

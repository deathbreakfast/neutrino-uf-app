//! The Access page: pick a secret, then see and change who can do what with it.

use leptos::prelude::*;
use uf_product::components::{Caption1, ContentContainer, EmptyState, SpacingSize, Title3};
use uf_product::primitives::{
    Field, Flex, FlexAlign, FlexGap, FlexWrap, Link, Select, SelectAppearance,
};

use crate::components::{error_copy, ErrorCopyBar};
use crate::pages::access::AccessCard;
use crate::server::{list_vault_secrets, VaultSecretRow};

#[component]
/// Who can do what with each secret, one secret at a time. Nothing is picked
/// until the person chooses a secret.
pub fn AclManagePage() -> impl IntoView {
    let selected = RwSignal::new(Option::<VaultSecretRow>::None);
    let secrets = Resource::new(|| (), |()| list_vault_secrets());

    let on_select_secret = move |ev: leptos::ev::Event| {
        let value = select_event_value(&ev);
        let row = secrets.with_untracked(|s| {
            s.as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|rows| rows.iter().find(|r| r.id == value).cloned())
        });
        selected.set(row);
    };

    let secret_id = Signal::derive(move || {
        selected.with(|s| s.as_ref().map(|s| s.id.clone()).unwrap_or_default())
    });
    let secret_name = Signal::derive(move || {
        selected.with(|s| s.as_ref().map(|s| s.name.clone()).unwrap_or_default())
    });

    view! {
        <ContentContainer data_testid="neutrino-acl-page">
            <div id="secrets-acl-page">
                <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                    <div id="secrets-acl-title">
                        <Title3>"Access"</Title3>
                        <Caption1>
                            "Who can do what with each secret. You can also open a secret and use its Access tab."
                        </Caption1>
                    </div>
                    <Flex align=FlexAlign::End gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                        <Field label="Secret">
                            <div id="secrets-acl-secret-select">
                                <Select
                                    appearance=SelectAppearance { ..Default::default() }
                                    on:change=on_select_secret
                                >
                                    <option value="" selected=move || selected.with(Option::is_none)>
                                        "Choose a secret"
                                    </option>
                                    {move || secrets.get().map(|result| match result {
                                        Ok(rows) => rows.into_iter().map(|row: VaultSecretRow| {
                                            let id = row.id.clone();
                                            view! {
                                                <option value=id.clone() selected=move || {
                                                    selected.with(|s| s.as_ref().is_some_and(|s| s.id == id))
                                                }>{row.name}</option>
                                            }
                                        }).collect_view().into_any(),
                                        Err(_) => ().into_any(),
                                    })}
                                </Select>
                            </div>
                        </Field>
                        {move || selected.get().map(|row| view! {
                            <div data-testid="neutrino-acl-open-secret">
                                <Link href=format!("/secrets/{}", row.id)>"Open its page"</Link>
                            </div>
                        })}
                    </Flex>
                    <Transition fallback=|| ()>
                        {move || secrets.get().and_then(Result::err).map(|err| view! {
                            <ErrorCopyBar copy=error_copy(&err) on_retry=Callback::new(move |()| secrets.refetch()) />
                        })}
                    </Transition>
                    {move || if selected.with(Option::is_none) {
                        view! {
                            <div id="secrets-acl-empty" data-testid="neutrino-acl-choose">
                                <EmptyState
                                    message="Choose a secret"
                                    description="Pick one above to see who can see it, read it, change it, delete it or manage its access."
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! { <AccessCard secret_id=secret_id secret_name=secret_name /> }.into_any()
                    }}
                </Flex>
            </div>
        </ContentContainer>
    }
}

fn select_event_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}

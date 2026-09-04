//! Per-secret ACL management: list, grant, and revoke Gauge action grants.

use leptos::prelude::*;
use uf_product::components::{
    Caption1, ContentContainer, EmptyState, MessageBar, MessageBarIntent, SpacingSize, Title3,
};
use uf_product::primitives::{
    Button, ButtonAppearance, Field, Flex, FlexAlign, FlexJustify, Input, Select, SelectAppearance,
};

use crate::pages::step_up::{spawn_with_fresh_totp, spawn_with_step_up};
use crate::server::{
    grant_secret_action, list_secret_grants, list_vault_secrets, revoke_secret_action,
    SecretActionGrant, VaultSecretRow,
};

const SECRET_ACTIONS: &[&str] = &["View", "Reveal", "Edit", "Delete", "Maintain"];

fn action_needs_fresh_code(action: &str) -> bool {
    action == "Reveal" || action == "Maintain"
}

#[component]
/// Manage direct user grants for one vault secret at a time.
#[allow(clippy::too_many_lines)]
pub fn AclManagePage() -> impl IntoView {
    let selected_secret_id = RwSignal::new(Option::<String>::None);
    let grant_user_id = RwSignal::new(String::new());
    let grant_action = RwSignal::new("View".to_string());
    let page_error = RwSignal::new(Option::<String>::None);
    let grant_submitting = RwSignal::new(false);
    let revoke_busy = RwSignal::new(Option::<(String, String)>::None);
    let grants_refresh = RwSignal::new(0u32);

    let secrets = Resource::new(|| (), |_| list_vault_secrets());

    let grants = Resource::new(
        move || (selected_secret_id.get(), grants_refresh.get()),
        |(secret_id, _)| async move {
            let Some(id) = secret_id else {
                return Ok(Vec::<SecretActionGrant>::new());
            };
            list_secret_grants(id).await
        },
    );

    Effect::new(move |_| {
        if selected_secret_id.get().is_none() {
            if let Some(Ok(rows)) = secrets.get() {
                if let Some(first) = rows.first() {
                    selected_secret_id.set(Some(first.id.clone()));
                }
            }
        }
    });

    let on_select_secret = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        if value.is_empty() {
            selected_secret_id.set(None);
        } else {
            page_error.set(None);
            selected_secret_id.set(Some(value));
        }
    };

    let refresh_grants = move || {
        grants_refresh.update(|n| *n += 1);
    };

    let on_submit_grant = move |_| {
        let Some(secret_id) = selected_secret_id.get_untracked() else {
            page_error.set(Some("Pick a secret first.".into()));
            return;
        };
        let user_id = grant_user_id.get_untracked();
        if user_id.trim().is_empty() {
            page_error.set(Some("Enter a user id.".into()));
            return;
        }
        let action = grant_action.get_untracked();
        page_error.set(None);
        grant_submitting.set(true);
        let needs_fresh = action_needs_fresh_code(&action);

        let finish_ok = Callback::new(move |()| {
            grant_submitting.set(false);
            grant_user_id.set(String::new());
            refresh_grants();
        });

        let run_grant = move |totp_code: Option<String>| {
            let secret_id = secret_id.clone();
            let user_id = user_id.clone();
            let action = action.clone();
            async move { grant_secret_action(secret_id, user_id, action, totp_code).await }
        };

        if needs_fresh {
            spawn_with_fresh_totp(page_error, finish_ok, move |totp_code| {
                run_grant(Some(totp_code))
            });
        } else {
            spawn_with_step_up(page_error, finish_ok, move || run_grant(None));
        }
    };

    let on_revoke = move |secret_id: String, user_id: String, action: String| {
        page_error.set(None);
        revoke_busy.set(Some((user_id.clone(), action.clone())));
        let needs_fresh = action_needs_fresh_code(&action);

        let finish_ok = Callback::new(move |()| {
            revoke_busy.set(None);
            refresh_grants();
        });

        let run_revoke = move |totp_code: Option<String>| {
            let secret_id = secret_id.clone();
            let user_id = user_id.clone();
            let action = action.clone();
            async move { revoke_secret_action(secret_id, user_id, action, totp_code).await }
        };

        if needs_fresh {
            spawn_with_fresh_totp(page_error, finish_ok, move |totp_code| {
                run_revoke(Some(totp_code))
            });
        } else {
            spawn_with_step_up(page_error, finish_ok, move || run_revoke(None));
        }
    };

    view! {
        <ContentContainer data_testid="neutrino-acl-page">
            <div id="secrets-acl-page">
                <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                    <div id="secrets-acl-title">
                        <Title3>"Secret access grants"</Title3>
                        <Caption1>
                            "Grant or revoke who can View, Reveal, Edit, Delete, or Maintain each secret."
                        </Caption1>
                    </div>

                    {move || page_error.get().map(|msg| view! {
                        <MessageBar intent=MessageBarIntent::Error>{msg}</MessageBar>
                    })}

                    <Field label="Secret">
                        <div id="secrets-acl-secret-select">
                            <Select
                                appearance=SelectAppearance { ..Default::default() }
                                on:change=on_select_secret
                            >
                                <option value="">"Select a secret"</option>
                                {move || secrets.get().map(|result| match result {
                                    Ok(rows) if rows.is_empty() => view! {
                                        <option value="" disabled=true>"No secrets yet"</option>
                                    }.into_any(),
                                    Ok(rows) => rows.into_iter().map(|row: VaultSecretRow| {
                                        let id = row.id.clone();
                                        let label = format!("{} ({})", row.name, row.scope_path);
                                        view! {
                                            <option value=id.clone() selected=move || {
                                                selected_secret_id.get().as_deref() == Some(id.as_str())
                                            }>{label}</option>
                                        }
                                    }).collect_view().into_any(),
                                    Err(_) => view! {
                                        <option value="" disabled=true>"Could not load secrets"</option>
                                    }.into_any(),
                                })}
                            </Select>
                        </div>
                    </Field>

                    {move || {
                        if selected_secret_id.get().is_none() {
                            return view! {
                                <div id="secrets-acl-empty">
                                    <EmptyState
                                        message="Pick a secret to manage grants."
                                        description="You need SecretsGrantManage and maintain rights on that secret's permissions."
                                    />
                                </div>
                            }.into_any();
                        }
                        grants.get().map(|result| match result {
                            Ok(grant_rows) if grant_rows.is_empty() => view! {
                                <div id="secrets-acl-empty">
                                    <EmptyState
                                        message="No permission bundle found for this secret."
                                        description="Create the secret first, or confirm you can maintain its Gauge permissions."
                                    />
                                </div>
                            }.into_any(),
                            Ok(grant_rows) => view! {
                                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                                    <div id="secrets-acl-grants">
                                    {grant_rows.into_iter().map(|row| {
                                        view! {
                                            <section data-testid=format!("acl-action-{}", row.action)>
                                                <Title3>{format!("{} grants", row.action)}</Title3>
                                                <Caption1>{row.permission_name.clone()}</Caption1>
                                                {if row.user_grants.is_empty() {
                                                    view! {
                                                        <Caption1>"No direct user grants for this action."</Caption1>
                                                    }.into_any()
                                                } else {
                                                    row.user_grants.into_iter().map(|grant| {
                                                        let uid = grant.user_id.clone();
                                                        let uid_for_btn = grant.user_id.clone();
                                                        let action_for_btn = row.action.clone();
                                                        let action_for_disabled = row.action.clone();
                                                        let secret_for_btn = selected_secret_id.get().unwrap_or_default();
                                                        view! {
                                                            <Flex
                                                                align=FlexAlign::Center
                                                                justify=FlexJustify::SpaceBetween
                                                                gap=SpacingSize::Size120.flex_gap()
                                                            >
                                                                <span>{format!("{} ({})", grant.label, uid)}</span>
                                                                <Button
                                                                    appearance=ButtonAppearance::Secondary
                                                                    disabled=Signal::derive(move || {
                                                                        revoke_busy.get().as_ref()
                                                                            == Some(&(uid_for_btn.clone(), action_for_disabled.clone()))
                                                                    })
                                                                    on:click=move |_| {
                                                                        on_revoke(
                                                                            secret_for_btn.clone(),
                                                                            uid.clone(),
                                                                            action_for_btn.clone(),
                                                                        );
                                                                    }
                                                                >
                                                                    "Revoke"
                                                                </Button>
                                                            </Flex>
                                                        }
                                                    }).collect_view().into_any()
                                                }}
                                            </section>
                                        }
                                    }).collect_view()}
                                    </div>

                                    <section data-testid="secrets-acl-grant-form">
                                        <div id="secrets-acl-grant-form">
                                        <Title3>"Add grant"</Title3>
                                        <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                                            <Field label="User id">
                                                <Input bind=grant_user_id />
                                            </Field>
                                            <Field label="Action">
                                                <Select
                                                    bind=grant_action
                                                    appearance=SelectAppearance { ..Default::default() }
                                                >
                                                    {SECRET_ACTIONS.iter().map(|action| view! {
                                                        <option value=*action>{*action}</option>
                                                    }).collect_view()}
                                                </Select>
                                            </Field>
                                            <Flex justify=FlexJustify::End>
                                                <Button
                                                    appearance=ButtonAppearance::Primary
                                                    disabled=Signal::derive(move || grant_submitting.get())
                                                    on:click=on_submit_grant
                                                >
                                                    {move || if grant_submitting.get() { "Granting…" } else { "Grant access" }}
                                                </Button>
                                            </Flex>
                                        </Flex>
                                        </div>
                                    </section>
                                </Flex>
                            }.into_any(),
                            Err(err) => view! {
                                <MessageBar intent=MessageBarIntent::Error>
                                    {err.to_string()}
                                </MessageBar>
                            }.into_any(),
                        }).unwrap_or_else(|| view! {
                            <Caption1>"Loading grants…"</Caption1>
                        }.into_any())
                    }}
                </Flex>
            </div>
        </ContentContainer>
    }
}

fn event_target_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}

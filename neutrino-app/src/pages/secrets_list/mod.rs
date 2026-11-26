//! Secrets vault list page: create / read / rotate / delete dialogs + table.

mod dialogs;
mod table;

use dialogs::CreateSecretDialog;
use leptos::prelude::*;
use table::SecretsTable;
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Flex, FlexAlign, FlexJustify, MessageBar, MessageBarActions,
    MessageBarBody, MessageBarIntent, MessageBarLayout, MessageBarTitle,
};

use crate::components::RequestAccessButton;
use crate::help_steps::SecretsTourDialogs;
use crate::pages::secret_actions::{
    DeleteDialog, NotifyFailed, NotifyFailedBar, ReadValueDialog, RotateDialog,
};
use crate::pages::step_up::{spawn_with_fresh_totp, spawn_with_step_up, StepUpAsk, StepUpCall};
use crate::server::{
    create_vault_secret, get_vault_abilities, list_vault_secrets, reveal_vault_secret,
    RevealedVaultSecret, RotateOutcome, VaultSecretRow,
};

#[component]
/// Lists Neutrino vault secrets with create / read / rotate / delete flows.
#[allow(clippy::too_many_lines)] // dialog signal graph lives with the page shell
pub fn SecretsListPage() -> impl IntoView {
    let refresh_trigger = RwSignal::new(0u32);
    let secrets = Resource::new(move || refresh_trigger.get(), |_| list_vault_secrets());
    let abilities = Resource::new(|| (), |()| get_vault_abilities());

    let create_open = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_scope = RwSignal::new(String::new());
    let new_kind = RwSignal::new(String::new());
    let new_plaintext = RwSignal::new(String::new());
    let create_error = RwSignal::new(Option::<ServerFnError>::None);
    let create_submitting = RwSignal::new(false);

    let reveal_open = RwSignal::new(false);
    let reveal_target = RwSignal::new(Option::<VaultSecretRow>::None);
    let reveal_b64 = RwSignal::new(String::new());
    let reveal_error = RwSignal::new(Option::<ServerFnError>::None);
    let reveal_loading = RwSignal::new(false);

    let rotate_open = RwSignal::new(false);
    let rotate_target = RwSignal::new(Option::<VaultSecretRow>::None);
    let notify_failed = RwSignal::new(Option::<NotifyFailed>::None);

    let delete_open = RwSignal::new(false);
    let delete_target = RwSignal::new(Option::<VaultSecretRow>::None);

    let action_busy_id = RwSignal::new(Option::<String>::None);

    provide_context(SecretsTourDialogs {
        create_open,
        reveal_open,
        rotate_open,
        delete_open,
    });

    let refresh = move || refresh_trigger.update(|n| *n += 1);

    let on_cancel_create = move |_| {
        create_open.set(false);
        create_error.set(None);
        new_plaintext.set(String::new());
    };

    let on_submit_create = move |_| {
        let name = new_name.get_untracked();
        let scope_path = new_scope.get_untracked();
        let kind = new_kind.get_untracked();
        let plaintext = new_plaintext.get_untracked();
        let call = StepUpCall::new(
            create_error,
            create_submitting,
            Callback::new(move |_: VaultSecretRow| {
                create_open.set(false);
                new_plaintext.set(String::new());
                refresh();
            }),
        );
        let ask = if name.trim().is_empty() {
            StepUpAsk::vault("store this secret")
        } else {
            StepUpAsk::vault(format!("store {}", name.trim()))
        };
        spawn_with_step_up(ask, call, move || {
            create_vault_secret(
                name.clone(),
                scope_path.clone(),
                kind.clone(),
                plaintext.clone(),
            )
        });
    };

    let read_value = Callback::new(move |row: VaultSecretRow| {
        reveal_b64.set(String::new());
        let call = StepUpCall::new(
            reveal_error,
            reveal_loading,
            Callback::new(move |mut p: RevealedVaultSecret| {
                reveal_b64.set(std::mem::take(&mut p.plaintext_b64));
            }),
        );
        spawn_with_fresh_totp(
            StepUpAsk::vault(format!("read {}", row.name)),
            call,
            move |totp_code| reveal_vault_secret(row.id.clone(), totp_code),
        );
    });

    let on_reveal_row = Callback::new(move |row: VaultSecretRow| {
        reveal_target.set(Some(row.clone()));
        reveal_error.set(None);
        reveal_open.set(true);
        read_value.run(row);
    });

    let on_rotate_row = Callback::new(move |row: VaultSecretRow| {
        rotate_target.set(Some(row));
        rotate_open.set(true);
    });

    let on_rotated = Callback::new(move |(row, outcome): (VaultSecretRow, RotateOutcome)| {
        rotate_target.set(None);
        notify_failed.set(outcome.notify_error.map(|_| NotifyFailed {
            secret_id: row.id,
            name: row.name,
            version: outcome.version,
        }));
        refresh();
    });

    let on_delete_row = Callback::new(move |row: VaultSecretRow| {
        delete_target.set(Some(row));
        delete_open.set(true);
    });

    let on_deleted = Callback::new(move |()| {
        delete_target.set(None);
        refresh();
    });

    view! {
        <div data-testid="neutrino-secrets-list-page">
        <ContentContainer>
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <div id="secrets-page-title">
                        <Title3>"Secret vault"</Title3>
                    </div>
                    <Transition fallback=|| ()>
                        {move || abilities.get().map(|a| {
                            a.is_ok_and(|a| a.can_write).then(|| view! {
                                <div id="secrets-create-button" data-testid="neutrino-create-secret-btn">
                                    <Button
                                        appearance=ButtonAppearance::Primary
                                        on_click=Callback::new(move |_| {
                                            create_error.set(None);
                                            create_open.set(true);
                                        })
                                    >
                                        "Create secret"
                                    </Button>
                                </div>
                            })
                        })}
                    </Transition>
                </Flex>

                <Transition fallback=|| ()>
                    {move || abilities.get().map(|a| {
                        matches!(a, Ok(a) if !a.can_write).then(|| view! {
                            <div data-testid="neutrino-ask-create-bar">
                                <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                                    <MessageBarBody>
                                        <MessageBarTitle>"Adding secrets needs SecretsWrite"</MessageBarTitle>
                                        "Request it, or ask someone who can add secrets to store this one for you."
                                    </MessageBarBody>
                                    <MessageBarActions>
                                        <RequestAccessButton permission="SecretsWrite".to_string() />
                                    </MessageBarActions>
                                </MessageBar>
                            </div>
                        })
                    })}
                </Transition>

                <NotifyFailedBar notice=notify_failed />

                <CreateSecretDialog
                    open=create_open
                    new_name=new_name
                    new_scope=new_scope
                    new_kind=new_kind
                    new_plaintext=new_plaintext
                    create_error=create_error
                    create_submitting=create_submitting
                    on_cancel=Callback::new(on_cancel_create)
                    on_submit=Callback::new(on_submit_create)
                />
                <ReadValueDialog
                    open=reveal_open
                    target=reveal_target
                    value_b64=reveal_b64
                    loading=reveal_loading
                    error=reveal_error
                    on_read_again=read_value
                />
                <RotateDialog open=rotate_open target=rotate_target on_rotated=on_rotated />
                <DeleteDialog open=delete_open target=delete_target on_deleted=on_deleted />

                <SecretsTable
                    secrets=secrets
                    action_busy_id=action_busy_id
                    on_reveal=on_reveal_row
                    on_rotate=on_rotate_row
                    on_delete=on_delete_row
                />
            </Flex>
        </ContentContainer>
        </div>
    }
}

//! Create dialog for the secrets list page. Read value, Rotate and Delete live in
//! [`crate::pages::secret_actions`] so the secret's own page can share them.

use leptos::ev::MouseEvent;
use leptos::prelude::*;
use uf_product::components::SpacingSize;
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, Input, InputAppearance, InputType,
};

use crate::components::{ErrorBar, KeepsNotice, KeepsWhat};

#[component]
#[allow(clippy::too_many_lines)] // Orbital Dialog field markup
pub(super) fn CreateSecretDialog(
    open: RwSignal<bool>,
    new_name: RwSignal<String>,
    new_scope: RwSignal<String>,
    new_kind: RwSignal<String>,
    new_plaintext: RwSignal<String>,
    create_error: RwSignal<Option<ServerFnError>>,
    create_submitting: RwSignal<bool>,
    on_cancel: Callback<MouseEvent>,
    on_submit: Callback<MouseEvent>,
) -> impl IntoView {
    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-create-secret-dialog">
                <DialogBody>
                    <DialogTitle>"Create secret"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                            <Field label="Name">
                                <div id="secrets-create-name" data-testid="neutrino-create-name">
                                    <Input bind=new_name appearance=InputAppearance::with_placeholder("my-api-token") />
                                </div>
                            </Field>
                            <Field label="Scope path">
                                <div id="secrets-create-scope" data-testid="neutrino-create-scope">
                                    <Input bind=new_scope appearance=InputAppearance::with_placeholder("/gluon/provider_account/...") />
                                </div>
                            </Field>
                            <Field label="Kind">
                                <div id="secrets-create-kind" data-testid="neutrino-create-kind">
                                    <Input bind=new_kind appearance=InputAppearance::with_placeholder("provider_api_token") />
                                </div>
                            </Field>
                            <Field label="Plaintext">
                                <div id="secrets-create-plaintext" data-testid="neutrino-create-plaintext">
                                <Input
                                    bind=new_plaintext
                                    appearance=InputAppearance {
                                        input_type: Signal::from(InputType::Password),
                                        placeholder: "secret value".into(),
                                        ..Default::default()
                                    }
                                />
                                </div>
                            </Field>
                            <KeepsNotice what=KeepsWhat::Create />
                            <ErrorBar error=create_error testid="neutrino-create-error" />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <div id="secrets-create-cancel" data-testid="neutrino-create-cancel">
                            <Button appearance=ButtonAppearance::Secondary on_click=on_cancel>
                                "Cancel"
                            </Button>
                        </div>
                        <div id="secrets-create-submit" data-testid="neutrino-create-submit">
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=on_submit
                            disabled=Signal::derive(move || create_submitting.get())
                        >
                            {move || if create_submitting.get() { "Creating…" } else { "Create" }}
                        </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

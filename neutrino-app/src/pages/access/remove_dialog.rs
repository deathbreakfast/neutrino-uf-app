//! Remove access: say what the person loses, and that group access stays.

use leptos::prelude::*;
use uf_product::components::Caption1;
use uf_product::primitives::*;

use super::{action_words, needs_fresh_code, run_access_change, PersonAccess};
use crate::components::ErrorBar;
use crate::pages::step_up::{StepUpAsk, StepUpCall};
use crate::server::revoke_secret_actions;

/// Remove some or all of one person's direct actions on a secret.
#[component]
pub fn RemoveAccessDialog(
    /// Open flag.
    open: RwSignal<bool>,
    /// The secret the grants are on.
    #[prop(into)]
    secret_id: Signal<String>,
    /// Its name, for the title and the code dialog.
    #[prop(into)]
    secret_name: Signal<String>,
    /// Who loses which actions.
    target: RwSignal<Option<(PersonAccess, Vec<String>)>>,
    /// Runs after the grants are removed.
    on_saved: Callback<()>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<ServerFnError>::None);
    Effect::new(move |_| {
        if !open.get() {
            error.set(None);
        }
    });

    let on_remove = Callback::new(move |_| {
        let Some((person, actions)) = target.get_untracked() else {
            return;
        };
        let secret = secret_id.get_untracked();
        let name = secret_name.get_untracked();
        let call = StepUpCall::new(
            error,
            busy,
            Callback::new(move |()| {
                open.set(false);
                on_saved.run(());
            }),
        );
        let principal = person.principal.clone();
        run_access_change(
            StepUpAsk::access(format!("remove {}'s access to {name}", person.display_name)),
            call,
            needs_fresh_code(&actions),
            move |code| {
                revoke_secret_actions(secret.clone(), principal.clone(), actions.clone(), code)
            },
        );
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-remove-access-dialog">
                <DialogBody>
                    <DialogTitle>
                        {move || target.with(|t| t.as_ref().map(|(p, _)| {
                            format!("Remove {}'s access to {}?", p.display_name, secret_name.get())
                        }))}
                    </DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Medium>
                            {move || target.get().map(|(person, actions)| {
                                let first = person.first_name().to_string();
                                let loses = format!("{first} loses");
                                let groups = format!(
                                    "Only grants on this secret go. If one of {first}'s Gauge groups has access, {first} keeps that. You'll confirm with your authenticator code."
                                );
                                view! {
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Text weight=TextWeight::Semibold>{loses}</Text>
                                        <ul data-testid="neutrino-remove-loses">
                                            {actions.iter().map(|a| view! { <li>{action_words(a).0}</li> }).collect_view()}
                                        </ul>
                                    </Flex>
                                    <Caption1>{groups}</Caption1>
                                }
                            })}
                            <ErrorBar error=error testid="neutrino-remove-error" />
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=busy
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            "Cancel"
                        </Button>
                        <div data-testid="neutrino-remove-submit">
                            <Button appearance=ButtonAppearance::Primary disabled=busy on_click=on_remove>
                                {move || if busy.get() { "Removing…" } else { "Remove access" }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

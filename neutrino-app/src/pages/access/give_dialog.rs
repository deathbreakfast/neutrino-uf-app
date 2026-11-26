//! Give access: pick a person or service user, tick what they can do, confirm once.
//!
//! The same dialog changes what someone already on the list can do.

use lepton_shell::people::{PersonPicker, PickedPrincipal, PrincipalKind};
use leptos::prelude::*;
use uf_product::components::{Caption1, Persona, PersonaConfig, PersonaSecondaryText};
use uf_product::primitives::*;

use super::{
    action_words, needs_fresh_code, run_access_change, words_list, PersonAccess, SECRET_ACTIONS,
};
use crate::components::{error_copy, ErrorAction, ErrorCopy, ErrorCopyBar, ErrorTone};
use crate::pages::step_up::{StepUpAsk, StepUpCall};
use crate::server::{
    change_secret_actions, error_tags, grant_secret_actions, GrantPrincipal, GrantPrincipalKind,
};

/// The principal to send for a pick, or `None` for a kind secrets can't be granted to.
fn grant_target(pick: &PickedPrincipal) -> Option<GrantPrincipal> {
    let kind = match pick.kind {
        PrincipalKind::User => GrantPrincipalKind::User,
        PrincipalKind::ServiceUser => GrantPrincipalKind::ServiceUser,
        PrincipalKind::Team | PrincipalKind::Extra(_) => return None,
    };
    Some(GrantPrincipal {
        id: pick.id.clone(),
        kind,
    })
}

/// (actions to give, actions to remove) going from `current` to `ticked`.
fn diff_actions(current: &[String], ticked: &[String]) -> (Vec<String>, Vec<String>) {
    let add = ticked
        .iter()
        .filter(|a| !current.contains(a))
        .cloned()
        .collect();
    let remove = current
        .iter()
        .filter(|a| !ticked.contains(a))
        .cloned()
        .collect();
    (add, remove)
}

/// What the dialog says when the change didn't go through.
fn failure_copy(err: &ServerFnError, changing: bool) -> ErrorCopy {
    let copy = error_copy(err);
    if copy.action != ErrorAction::TryAgain {
        return copy;
    }
    let title = if changing {
        "Access wasn't changed"
    } else {
        "Access wasn't given"
    };
    let sentence = if err.to_string().contains(error_tags::GAUGE_FAILED) {
        "Gauge didn't save the grant. Nothing changed; try again.".to_string()
    } else {
        copy.sentence
    };
    ErrorCopy {
        title: title.to_string(),
        sentence,
        tone: ErrorTone::Error,
        action: ErrorAction::None,
    }
}

/// Give access, or (with `prefill`) change what someone can do, on one secret.
///
/// A failure keeps the dialog open with the button enabled; closing the code
/// dialog shows "Not given yet" and enables it again.
#[component]
#[allow(clippy::too_many_lines)] // picker, checkboxes and three result states in one dialog
pub fn GiveAccessDialog(
    /// Open flag.
    open: RwSignal<bool>,
    /// The secret the grants are on.
    #[prop(into)]
    secret_id: Signal<String>,
    /// Its name, for the dialog and the code dialog.
    #[prop(into)]
    secret_name: Signal<String>,
    /// Someone already on the list: change their actions instead of picking.
    prefill: RwSignal<Option<PersonAccess>>,
    /// Runs after a saved change.
    on_saved: Callback<()>,
) -> impl IntoView {
    let picked = RwSignal::new(Vec::<PickedPrincipal>::new());
    let ticked = SECRET_ACTIONS.map(|_| RwSignal::new(false));
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<ServerFnError>::None);
    let cancelled = RwSignal::new(false);
    let hint = RwSignal::new(Option::<&'static str>::None);

    // Clear on close, not on open: an open-time reset can land after the first
    // click and undo it. Openers set `prefill` before `open`, so the ticks
    // follow the person's current actions (or nothing) by the time it shows.
    Effect::new(move |_| {
        if open.get() {
            return;
        }
        for tick in ticked {
            tick.set(false);
        }
        picked.set(Vec::new());
        error.set(None);
        cancelled.set(false);
        hint.set(None);
    });
    Effect::new(move |_| {
        let current = prefill.with(|p| p.as_ref().map(|p| p.actions.clone()));
        for (action, tick) in SECRET_ACTIONS.iter().zip(ticked) {
            tick.set(
                current
                    .as_ref()
                    .is_some_and(|c| c.iter().any(|a| a == action)),
            );
        }
    });

    let changing = move || prefill.with(Option::is_some);

    let on_submit = Callback::new(move |_| {
        error.set(None);
        cancelled.set(false);
        hint.set(None);
        let actions: Vec<String> = SECRET_ACTIONS
            .iter()
            .zip(ticked)
            .filter(|(_, tick)| tick.get_untracked())
            .map(|(a, _)| (*a).to_string())
            .collect();
        let existing = prefill.get_untracked();
        let (principal, who) = match &existing {
            Some(person) => (person.principal.clone(), person.display_name.clone()),
            None => {
                let pick = picked.with_untracked(|p| p.first().cloned());
                let Some((principal, who)) = pick
                    .as_ref()
                    .and_then(|p| grant_target(p).map(|g| (g, p.label.clone())))
                else {
                    hint.set(Some("Pick a person or service user."));
                    return;
                };
                (principal, who)
            }
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
        )
        .with_cancel(Callback::new(move |()| cancelled.set(true)));

        match existing {
            Some(person) => {
                let (add, remove) = diff_actions(&person.actions, &actions);
                if add.is_empty() && remove.is_empty() {
                    open.set(false);
                    return;
                }
                let fresh = needs_fresh_code(&add) || needs_fresh_code(&remove);
                run_access_change(
                    StepUpAsk::access(format!("change what {who} can do on {name}")),
                    call,
                    fresh,
                    move |code| {
                        change_secret_actions(
                            secret.clone(),
                            principal.clone(),
                            add.clone(),
                            remove.clone(),
                            code,
                        )
                    },
                );
            }
            None => {
                if actions.is_empty() {
                    hint.set(Some("Pick at least one thing they can do."));
                    return;
                }
                let fresh = needs_fresh_code(&actions);
                run_access_change(
                    StepUpAsk::access(format!("give {who} {} on {name}", words_list(&actions))),
                    call,
                    fresh,
                    move |code| {
                        grant_secret_actions(
                            secret.clone(),
                            principal.clone(),
                            actions.clone(),
                            code,
                        )
                    },
                );
            }
        }
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-give-access-dialog">
                <DialogBody>
                    <DialogTitle>
                        {move || if changing() { "Change what they can do" } else { "Give access" }}
                    </DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Medium>
                            <Body1>{move || format!("on {}", secret_name.get())}</Body1>
                            // A fresh picker per open: the picker keeps its own chips, so
                            // clearing `picked` alone doesn't empty it for the next open.
                            {move || match (open.get(), prefill.get()) {
                                (false, _) => ().into_any(),
                                (true, Some(person)) => view! {
                                    <Persona config=PersonaConfig::named(person.display_name)>
                                        <PersonaSecondaryText slot>{person.who_line}</PersonaSecondaryText>
                                    </Persona>
                                }.into_any(),
                                (true, None) => view! {
                                    <div id="secrets-acl-grant-principal">
                                        <PersonPicker
                                            value=picked
                                            single=true
                                            people=true
                                            service_users=true
                                            label="Person or service user"
                                            placeholder="Search by name"
                                        />
                                    </div>
                                }.into_any(),
                            }}
                            <Flex vertical=true gap=FlexGap::Small>
                                <Text weight=TextWeight::Semibold>"What they can do"</Text>
                                {SECRET_ACTIONS.iter().zip(ticked).map(|(action, tick)| {
                                    let (label, allows) = action_words(action);
                                    view! {
                                        <div data-testid=format!("neutrino-give-action-{action}")>
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Checkbox checked=tick label=label.to_string() />
                                                <Caption1>{allows}</Caption1>
                                            </Flex>
                                        </div>
                                    }
                                }).collect_view()}
                            </Flex>
                            {move || hint.get().map(|text| view! {
                                <div data-testid="neutrino-give-hint">
                                    <FieldValidationMessage state=FieldValidationState::Error(text.to_string()) />
                                </div>
                            })}
                            {move || error.get().map(|err| view! {
                                <div data-testid="neutrino-give-error">
                                    <ErrorCopyBar copy=failure_copy(&err, changing()) />
                                </div>
                            })}
                            {move || cancelled.get().then(|| view! {
                                <div data-testid="neutrino-give-cancelled">
                                    <ErrorCopyBar copy=ErrorCopy {
                                        title: "Not given yet".to_string(),
                                        sentence: "You closed the code dialog, so nothing changed. Give access again when you're ready.".to_string(),
                                        tone: ErrorTone::Info,
                                        action: ErrorAction::None,
                                    } />
                                </div>
                            })}
                            <Caption1>
                                "You'll confirm with your authenticator code. The person gets a notification."
                            </Caption1>
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
                        <div data-testid="neutrino-give-submit">
                            <Button appearance=ButtonAppearance::Primary disabled=busy on_click=on_submit>
                                {move || match (busy.get(), changing()) {
                                    (true, _) => "Saving…",
                                    (false, true) => "Save changes",
                                    (false, false) => "Give access",
                                }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
                </div>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(kind: PrincipalKind) -> PickedPrincipal {
        PickedPrincipal {
            kind,
            id: "abc".into(),
            label: "Ada".into(),
            secondary: None,
            image_url: None,
        }
    }

    #[test]
    fn person_and_service_user_picks_map_to_grant_kinds_happy() {
        assert_eq!(
            grant_target(&pick(PrincipalKind::User)).map(|g| g.kind),
            Some(GrantPrincipalKind::User)
        );
        assert_eq!(
            grant_target(&pick(PrincipalKind::ServiceUser)).map(|g| g.kind),
            Some(GrantPrincipalKind::ServiceUser)
        );
    }

    #[test]
    fn team_and_extra_picks_have_no_grant_target_sad() {
        assert_eq!(grant_target(&pick(PrincipalKind::Team)), None);
        assert_eq!(grant_target(&pick(PrincipalKind::Extra("x".into()))), None);
    }

    #[test]
    fn change_diffs_ticked_against_current() {
        let current = vec!["View".to_string(), "Reveal".to_string()];
        let ticked = vec!["View".to_string(), "Edit".to_string()];
        assert_eq!(
            diff_actions(&current, &ticked),
            (vec!["Edit".to_string()], vec!["Reveal".to_string()])
        );
        assert_eq!(diff_actions(&current, &current), (vec![], vec![]));
    }

    #[test]
    fn failed_grant_says_access_wasnt_given() {
        let gauge = ServerFnError::new("gauge_failed::grant secret action");
        let copy = failure_copy(&gauge, false);
        assert_eq!(copy.title, "Access wasn't given");
        assert_eq!(
            copy.sentence,
            "Gauge didn't save the grant. Nothing changed; try again."
        );
        let code = ServerFnError::new("STEP_UP:step_up_invalid: x");
        assert_eq!(failure_copy(&code, false).title, "That code didn't work");
    }
}

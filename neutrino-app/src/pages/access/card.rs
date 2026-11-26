//! The Access card: grants on one secret, by person or by action.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use orbital_data::{DataRecord, DataValue};
use uf_product::components::{Caption1, Persona, PersonaConfig, PersonaSecondaryText};
use uf_product::primitives::*;

use super::{
    action_words, by_person, who_line, GiveAccessDialog, PersonAccess, RemoveAccessDialog,
    SECRET_ACTIONS,
};
use crate::components::{error_copy, ErrorCopyBar};
use crate::server::{list_secret_grants, GrantPrincipal, SecretActionGrant};

/// Which way the card lists access.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccessView {
    /// One row per person or service user, with everything they can do.
    #[default]
    ByPerson,
    /// One card per action, with who holds it.
    ByAction,
}

impl AccessView {
    const fn value(self) -> &'static str {
        match self {
            Self::ByPerson => "person",
            Self::ByAction => "action",
        }
    }
}

/// Plain-word badges for a person's actions.
fn can_badges(actions: &[String]) -> impl IntoView {
    let badges: Vec<(BadgeColor, &'static str)> = actions
        .iter()
        .map(|action| {
            let color = if action == "Reveal" || action == "Maintain" {
                BadgeColor::Brand
            } else {
                BadgeColor::Informative
            };
            (color, action_words(action).0)
        })
        .collect();
    view! {
        <Flex gap=FlexGap::Small wrap=FlexWrap::Wrap>
            {badges.into_iter().map(|(color, word)| view! {
                <Badge appearance=BadgeAppearance::Tint color=color size=BadgeSize::Medium>
                    {word}
                </Badge>
            }).collect_view()}
        </Flex>
    }
}

fn person_persona(name: String, line: String) -> impl IntoView {
    view! {
        <Persona config=PersonaConfig::named(name)>
            <PersonaSecondaryText slot>{line}</PersonaSecondaryText>
        </Persona>
    }
}

/// A `DataTable` record for one person: the name and plain words, so the
/// table's search finds people by either.
fn person_record(person: &PersonAccess) -> DataRecord {
    let words: Vec<&str> = person.actions.iter().map(|a| action_words(a).0).collect();
    let fields: HashMap<String, DataValue> = HashMap::from([
        ("who".into(), DataValue::Text(person.display_name.clone())),
        ("can".into(), DataValue::Text(words.join(", "))),
        ("menu".into(), DataValue::Text(String::new())),
    ]);
    DataRecord::new(person.key(), fields)
}

fn person_columns(
    people: RwSignal<Vec<PersonAccess>>,
    on_change: Callback<PersonAccess>,
    on_remove: Callback<PersonAccess>,
) -> Vec<DataTableColumnDef> {
    let find =
        move |key: &str| people.with_untracked(|p| p.iter().find(|p| p.key() == key).cloned());
    let who_view = Arc::new(move |record: DataRecord| {
        find(&record.id).map_or_else(
            || ().into_any(),
            |p| {
                let kind = p.principal.kind.as_str();
                view! {
                    <span data-testid="acl-person-row" data-kind=kind>
                        {person_persona(p.display_name, p.who_line)}
                    </span>
                }
                .into_any()
            },
        )
    });
    let can_view = Arc::new(move |record: DataRecord| {
        find(&record.id).map_or_else(
            || ().into_any(),
            |p| {
                view! { <span data-testid="acl-person-can">{can_badges(&p.actions)}</span> }
                    .into_any()
            },
        )
    });
    let menu_view = Arc::new(move |record: DataRecord| {
        let Some(person) = find(&record.id) else {
            return ().into_any();
        };
        let label = format!("More for {}", person.display_name);
        view! {
            <span data-testid="acl-person-menu">
                <Menu on_select=move |value: String| {
                    if value == "remove" {
                        on_remove.run(person.clone());
                    } else {
                        on_change.run(person.clone());
                    }
                }>
                    <MenuTrigger slot>
                        <Button appearance=ButtonAppearance::Subtle attr:aria-label=label>"⋯"</Button>
                    </MenuTrigger>
                    <MenuItem value="change".to_string()>"Change what they can do"</MenuItem>
                    <MenuItem value="remove".to_string()>"Remove access"</MenuItem>
                </Menu>
            </span>
        }
        .into_any()
    });
    vec![
        DataTableColumnDef::new("who", "Who")
            .with_sortable(false)
            .with_cell_view(who_view),
        DataTableColumnDef::new("can", "Can")
            .with_sortable(false)
            .with_cell_view(can_view),
        DataTableColumnDef::new("menu", "")
            .with_sortable(false)
            .with_cell_view(menu_view),
    ]
}

/// The person for a holder line in the By action view.
fn holder(grant: &crate::server::SecretGrantPrincipal) -> PersonAccess {
    PersonAccess {
        principal: GrantPrincipal {
            id: grant.user_id.clone(),
            kind: grant.kind,
        },
        display_name: grant.display_name.clone(),
        who_line: who_line(grant),
        actions: Vec::new(),
    }
}

/// Who has access to one secret, with Give access and per-person changes.
///
/// Opening a dialog or switching between By person and By action keeps the
/// loaded grants; only a saved change reloads them.
#[component]
#[allow(clippy::too_many_lines)] // both views and their dialogs share one signal graph
pub fn AccessCard(
    /// The secret whose grants to show.
    #[prop(into)]
    secret_id: Signal<String>,
    /// Its name, for headings and the code dialog.
    #[prop(into)]
    secret_name: Signal<String>,
    /// The view to open on.
    #[prop(optional)]
    view: AccessView,
) -> impl IntoView {
    let tab = RwSignal::new(view.value().to_string());
    let refresh = RwSignal::new(0u32);
    let grants = LocalResource::new(move || {
        let id = secret_id.get();
        refresh.track();
        async move { list_secret_grants(id).await }
    });

    let rows = RwSignal::new(Vec::<SecretActionGrant>::new());
    let people = RwSignal::new(Vec::<PersonAccess>::new());
    let table_rows = RwSignal::new(Vec::<DataTableRowModel>::new());
    let loading = RwSignal::new(true);
    let load_error = RwSignal::new(Option::<ServerFnError>::None);

    // A different secret starts from an empty, loading table.
    Effect::watch(
        move || secret_id.get(),
        move |_, _, _| {
            loading.set(true);
            load_error.set(None);
            rows.set(Vec::new());
            people.set(Vec::new());
            table_rows.set(Vec::new());
        },
        false,
    );
    Effect::new(move |_| match grants.get() {
        None => loading.set(true),
        Some(Ok(loaded)) => {
            let regrouped = by_person(&loaded);
            let records = regrouped
                .iter()
                .map(person_record)
                .map(DataTableRowModel::new)
                .collect();
            people.set(regrouped);
            table_rows.set(records);
            rows.set(loaded);
            load_error.set(None);
            loading.set(false);
        }
        Some(Err(err)) => {
            load_error.set(Some(err));
            loading.set(false);
        }
    });

    let give_open = RwSignal::new(false);
    let give_prefill = RwSignal::new(Option::<PersonAccess>::None);
    let remove_open = RwSignal::new(false);
    let remove_target = RwSignal::new(Option::<(PersonAccess, Vec<String>)>::None);
    let on_saved = Callback::new(move |()| refresh.update(|n| *n += 1));

    let on_change = Callback::new(move |person: PersonAccess| {
        give_prefill.set(Some(person));
        give_open.set(true);
    });
    let on_remove = Callback::new(move |person: PersonAccess| {
        let actions = person.actions.clone();
        remove_target.set(Some((person, actions)));
        remove_open.set(true);
    });

    let by_person_view = move || {
        if !loading.get() && load_error.with(Option::is_none) && people.with(Vec::is_empty) {
            return view! {
                <div data-testid="acl-no-grants">
                    <Caption1>"No one has direct access to this secret yet."</Caption1>
                </div>
            }
            .into_any();
        }
        view! {
            <Flex vertical=true gap=FlexGap::Small>
                <div data-testid="acl-by-person">
                    <DataTable
                        data_source=DataTableSource::Client(table_rows)
                        paging=PagingMode::None
                        columns=person_columns(people, on_change, on_remove)
                        sortable=false
                        loading=loading
                        toolbar_config=DataTableToolbarConfig {
                            quick_search: true,
                            filter_panel: false,
                            column_picker: false,
                            pivot: false,
                            export_menu: false,
                        }
                    />
                </div>
                <Caption1>
                    "These are grants on this secret. People can also get access through Gauge groups, which aren't listed here."
                </Caption1>
            </Flex>
        }
        .into_any()
    };

    let by_action_view = move || {
        let loaded = rows.get();
        view! {
            <Flex vertical=true gap=FlexGap::Medium>
                {SECRET_ACTIONS.iter().map(|action| {
                    let (name, allows) = action_words(action);
                    let row = loaded.iter().find(|r| r.action == *action).cloned();
                    let holders = row.as_ref().map(|r| r.user_grants.clone()).unwrap_or_default();
                    let permission_name = row.map(|r| r.permission_name).unwrap_or_default();
                    let action = (*action).to_string();
                    let card_testid = format!("acl-action-{action}");
                    view! {
                        <div data-testid=card_testid>
                            <Card>
                                <CardHeader>
                                    <CardHeaderDescription slot>{allows}</CardHeaderDescription>
                                    {name}
                                </CardHeader>
                                <CardContent>
                                    <Flex vertical=true gap=FlexGap::Medium>
                                        {if holders.is_empty() {
                                            view! { <Caption1>"No one has this directly."</Caption1> }.into_any()
                                        } else {
                                            holders.into_iter().map(|grant| {
                                                let person = holder(&grant);
                                                let kind = grant.kind.as_str();
                                                let action = action.clone();
                                                let remove_label = format!("Remove {} from {name}", person.display_name);
                                                view! {
                                                    <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                        <span data-testid="acl-grant-row" data-kind=kind>
                                                            {person_persona(person.display_name.clone(), person.who_line.clone())}
                                                        </span>
                                                        <Button
                                                            appearance=ButtonAppearance::Subtle
                                                            size=ButtonSize::Small
                                                            attr:aria-label=remove_label
                                                            on_click=Callback::new(move |_| {
                                                                remove_target.set(Some((person.clone(), vec![action.clone()])));
                                                                remove_open.set(true);
                                                            })
                                                        >
                                                            "Remove"
                                                        </Button>
                                                    </Flex>
                                                }
                                            }).collect_view().into_any()
                                        }}
                                        {(!permission_name.is_empty()).then(|| view! {
                                            <Accordion collapsible=true>
                                                <AccordionItem value=action.clone()>
                                                    <AccordionHeader slot>"Permission id"</AccordionHeader>
                                                    <Flex align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                                                        <Code text=permission_name.clone() />
                                                        <CopyButton text=permission_name.clone() />
                                                    </Flex>
                                                </AccordionItem>
                                            </Accordion>
                                        })}
                                    </Flex>
                                </CardContent>
                            </Card>
                        </div>
                    }
                }).collect_view()}
            </Flex>
        }
    };

    view! {
        <div data-testid="neutrino-access-card">
            <Card>
                <CardHeader>{move || format!("Who has access to {}", secret_name.get())}</CardHeader>
                <CardContent>
                    <Flex vertical=true gap=FlexGap::Medium>
                        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                            <TabList selected_value=tab>
                                <Tab value="person".to_string()>"By person"</Tab>
                                <Tab value="action".to_string()>"By action"</Tab>
                            </TabList>
                            <div id="secrets-acl-grant-form" data-testid="secrets-acl-grant-form">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=Callback::new(move |_| {
                                        give_prefill.set(None);
                                        give_open.set(true);
                                    })
                                >
                                    "Give access"
                                </Button>
                            </div>
                        </Flex>
                        {move || load_error.get().map(|err| view! {
                            <ErrorCopyBar copy=error_copy(&err) on_retry=on_saved />
                        })}
                        {move || if tab.get() == "action" {
                            by_action_view().into_any()
                        } else {
                            by_person_view()
                        }}
                    </Flex>
                </CardContent>
            </Card>
            <GiveAccessDialog
                open=give_open
                secret_id=secret_id
                secret_name=secret_name
                prefill=give_prefill
                on_saved=on_saved
            />
            <RemoveAccessDialog
                open=remove_open
                secret_id=secret_id
                secret_name=secret_name
                target=remove_target
                on_saved=on_saved
            />
        </div>
    }
}

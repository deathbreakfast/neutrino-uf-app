//! Secrets vault data table (loading / empty / error / rows).

use leptos::prelude::*;
use uf_product::components::{Card, EmptyState, SkeletonItemSize};
use uf_product::primitives::{
    Button, ButtonAppearance, ButtonSize, Flex, FlexGap, SkeletonItem, Table, TableBody, TableCell,
    TableHeader, TableHeaderCell, TableRow,
};

use crate::components::{error_copy, ErrorCopyBar};
use crate::server::VaultSecretRow;

#[component]
#[allow(clippy::too_many_lines)] // table + row action chrome
pub(super) fn SecretsTable(
    secrets: Resource<Result<Vec<VaultSecretRow>, ServerFnError>>,
    action_busy_id: RwSignal<Option<String>>,
    on_reveal: Callback<VaultSecretRow>,
    on_rotate: Callback<VaultSecretRow>,
    on_delete: Callback<VaultSecretRow>,
) -> impl IntoView {
    view! {
        <Flex vertical=true fill=true full_width=true gap=FlexGap::Size(0)>
        <Card>
            <div id="secrets-table">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>
                            <div id="secrets-col-name">"Name"</div>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <div id="secrets-col-scope">"Scope"</div>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <div id="secrets-col-kind">"Kind"</div>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <div id="secrets-col-version">"Version"</div>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <div id="secrets-col-created">"Created"</div>
                        </TableHeaderCell>
                        <TableHeaderCell>"Actions"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <Suspense fallback=move || view! {
                    <TableBody>
                        {(0..3).map(|_| view! {
                            <TableRow>
                                <TableCell attr:colspan="6">
                                    <SkeletonItem
                                        size=Signal::from(SkeletonItemSize::S32)
                                        width="100%".to_string()
                                    />
                                </TableCell>
                            </TableRow>
                        }).collect_view()}
                    </TableBody>
                }>
                    {move || match secrets.get() {
                        Some(Ok(rows)) => {
                            if rows.is_empty() {
                                return view! {
                                    <TableBody>
                                        <TableRow>
                                            <TableCell attr:colspan="6">
                                                <EmptyState message="No secrets in the vault yet." />
                                            </TableCell>
                                        </TableRow>
                                    </TableBody>
                                }.into_any();
                            }
                            view! {
                                <TableBody>
                                    {rows.into_iter().enumerate().map(|(idx, row)| {
                                        let row_name = row.name.clone();
                                        let row_name_ver = row.name.clone();
                                        let row_name_act = row.name.clone();
                                        let row_reveal = row.clone();
                                        let row_rotate = row.clone();
                                        let row_delete = row.clone();
                                        let is_busy = Signal::derive({
                                            let id = row.id.clone();
                                            move || action_busy_id.get().as_deref() == Some(id.as_str())
                                        });
                                        let spotlight_first = idx == 0;
                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <div data-testid=format!("neutrino-secret-row-{}", row_name)>
                                                        {row.name.clone()}
                                                    </div>
                                                </TableCell>
                                                <TableCell>{row.scope_path.clone()}</TableCell>
                                                <TableCell>{row.kind.clone()}</TableCell>
                                                <TableCell>
                                                    <div data-testid=format!("neutrino-secret-version-{}", row_name_ver)>
                                                        {row.current_version}
                                                    </div>
                                                </TableCell>
                                                <TableCell>{row.created_at.clone()}</TableCell>
                                                <TableCell>
                                                    <div data-testid=format!("neutrino-secret-actions-{}", row_name_act)>
                                                    <Flex gap=FlexGap::Size(4)>
                                                        {if spotlight_first {
                                                            view! {
                                                                <div id="secrets-action-reveal">
                                                                    <Button
                                                                        appearance=ButtonAppearance::Subtle
                                                                        size=ButtonSize::Small
                                                                        disabled=is_busy
                                                                        attr:aria-label="Reveal secret"
                                                                        on_click=Callback::new(move |_| {
                                                                            on_reveal.run(row_reveal.clone());
                                                                        })
                                                                    >
                                                                        "Reveal"
                                                                    </Button>
                                                                </div>
                                                                <div id="secrets-action-rotate">
                                                                    <Button
                                                                        appearance=ButtonAppearance::Subtle
                                                                        size=ButtonSize::Small
                                                                        disabled=is_busy
                                                                        attr:aria-label="Rotate secret"
                                                                        on_click=Callback::new(move |_| {
                                                                            on_rotate.run(row_rotate.clone());
                                                                        })
                                                                    >
                                                                        "Rotate"
                                                                    </Button>
                                                                </div>
                                                                <div id="secrets-action-delete">
                                                                    <Button
                                                                        appearance=ButtonAppearance::Subtle
                                                                        size=ButtonSize::Small
                                                                        disabled=is_busy
                                                                        attr:aria-label="Delete secret"
                                                                        on_click=Callback::new(move |_| {
                                                                            on_delete.run(row_delete.clone());
                                                                        })
                                                                    >
                                                                        "Delete"
                                                                    </Button>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <Button
                                                                    appearance=ButtonAppearance::Subtle
                                                                    size=ButtonSize::Small
                                                                    disabled=is_busy
                                                                    attr:aria-label="Reveal secret"
                                                                    on_click=Callback::new(move |_| {
                                                                        on_reveal.run(row_reveal.clone());
                                                                    })
                                                                >
                                                                    "Reveal"
                                                                </Button>
                                                                <Button
                                                                    appearance=ButtonAppearance::Subtle
                                                                    size=ButtonSize::Small
                                                                    disabled=is_busy
                                                                    attr:aria-label="Rotate secret"
                                                                    on_click=Callback::new(move |_| {
                                                                        on_rotate.run(row_rotate.clone());
                                                                    })
                                                                >
                                                                    "Rotate"
                                                                </Button>
                                                                <Button
                                                                    appearance=ButtonAppearance::Subtle
                                                                    size=ButtonSize::Small
                                                                    disabled=is_busy
                                                                    attr:aria-label="Delete secret"
                                                                    on_click=Callback::new(move |_| {
                                                                        on_delete.run(row_delete.clone());
                                                                    })
                                                                >
                                                                    "Delete"
                                                                </Button>
                                                            }.into_any()
                                                        }}
                                                    </Flex>
                                                    </div>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            }.into_any()
                        }
                        Some(Err(err)) => {
                            view! {
                                <TableBody>
                                    <TableRow>
                                        <TableCell attr:colspan="6">
                                            <div data-testid="neutrino-secrets-load-error">
                                                <ErrorCopyBar
                                                    copy=error_copy(&err)
                                                    on_retry=Callback::new(move |()| secrets.refetch())
                                                />
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                </TableBody>
                            }.into_any()
                        }
                        None => view! {
                            <TableBody>
                                <TableRow>
                                    <TableCell attr:colspan="6"></TableCell>
                                </TableRow>
                            </TableBody>
                        }.into_any(),
                    }}
                </Suspense>
            </Table>
            </div>
        </Card>
        </Flex>
    }
}

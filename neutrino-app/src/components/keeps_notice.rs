//! "What Neutrino keeps": the one-line notice on create, read and delete, and
//! the dialog it opens.

use leptos::prelude::*;
use uf_product::components::Caption1;
use uf_product::primitives::*;

/// Which action the notice sits next to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeepsWhat {
    /// Storing a new secret.
    Create,
    /// Reading a value.
    Read,
    /// Deleting a secret.
    Delete,
}

impl KeepsWhat {
    /// The notice's sentence for this action.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Create => "Neutrino records that you stored this secret and when.",
            Self::Read => "Neutrino records that you read this secret, which version, and when.",
            Self::Delete => {
                "The secret's activity stays after it's deleted, with your name on the delete."
            }
        }
    }
}

/// The rows of [`KeepsDialog`]: (heading, what's kept).
const KEEPS_ROWS: [(&str, &str); 5] = [
    (
        "When you store or rotate a secret",
        "Your user id on that version, and an activity entry with the time.",
    ),
    (
        "When you read a value, or try to",
        "An activity entry: your user id, the version, whether it worked, and when. Refused reads are kept too.",
    ),
    (
        "When you delete a secret",
        "The secret and its values go. Its activity stays, with your name on the delete.",
    ),
    (
        "Who can read the activity",
        "The secret's owners, and people they give See activity to. Access changes are kept by Gauge.",
    ),
    (
        "What's never kept",
        "The value itself in any log, and anything about you beyond your user id.",
    ),
];

/// Info bar saying what Neutrino records about `what`, with a link to [`KeepsDialog`].
#[component]
pub fn KeepsNotice(
    /// The action the notice describes.
    what: KeepsWhat,
) -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <div data-testid="neutrino-keeps-notice">
            <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                <MessageBarBody>
                    {what.sentence()}" "
                    <span data-testid="neutrino-keeps-link" on:click=move |_| open.set(true)>
                        <Link inline=true>"What Neutrino keeps"</Link>
                    </span>
                </MessageBarBody>
            </MessageBar>
            <KeepsDialog open=open />
        </div>
    }
}

/// What Neutrino keeps about the people who use a secret, in five rows.
#[component]
pub fn KeepsDialog(
    /// Open flag; Close sets it back to false.
    open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <Dialog open=open>
            <DialogSurface>
                <div data-testid="neutrino-keeps-dialog">
                <DialogBody>
                    <DialogTitle>"What Neutrino keeps"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Medium>
                            {KEEPS_ROWS.iter().map(|(heading, kept)| view! {
                                <Flex vertical=true gap=FlexGap::Size(0)>
                                    <Text weight=TextWeight::Semibold>{*heading}</Text>
                                    <Caption1>{*kept}</Caption1>
                                </Flex>
                            }).collect_view()}
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            "Close"
                        </Button>
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

    #[test]
    fn notices_and_rows_use_the_agreed_wording() {
        assert!(KeepsWhat::Read.sentence().contains("which version"));
        assert!(KeepsWhat::Delete
            .sentence()
            .contains("your name on the delete"));
        let who = KEEPS_ROWS
            .iter()
            .find(|(h, _)| *h == "Who can read the activity")
            .map(|(_, kept)| *kept);
        assert_eq!(
            who,
            Some("The secret's owners, and people they give See activity to. Access changes are kept by Gauge.")
        );
    }
}

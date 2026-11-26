//! Who can do what with one secret: the Access card, Give access and Remove
//! access. The Access page uses it today; the secret's own page reuses it.

mod card;
mod give_dialog;
mod remove_dialog;

pub use card::{AccessCard, AccessView};
pub use give_dialog::GiveAccessDialog;
pub use remove_dialog::RemoveAccessDialog;

use leptos::prelude::*;

use crate::pages::step_up::{spawn_with_fresh_totp, spawn_with_step_up, StepUpAsk, StepUpCall};
use crate::server::{GrantPrincipal, GrantPrincipalKind, SecretActionGrant, SecretGrantPrincipal};

/// Gauge actions on a secret, in the order the page lists them.
pub const SECRET_ACTIONS: [&str; 5] = ["View", "Reveal", "Edit", "Delete", "Maintain"];

/// Plain words for a Gauge action: (short name, what it allows).
#[must_use]
pub fn action_words(action: &str) -> (&'static str, &'static str) {
    match action {
        "View" => (
            "See it",
            "See its name, scope, versions and activity. Never the value.",
        ),
        "Reveal" => (
            "Read the value",
            "Read the current value. Every read is recorded.",
        ),
        "Edit" => ("Change it", "Rotate the value or rename it."),
        "Delete" => ("Delete it", "Delete it and every version."),
        "Maintain" => ("Manage access", "Give and remove access to it."),
        _ => ("Other access", "An action Neutrino doesn't name."),
    }
}

/// "Read the value", "See it and Read the value", "See it, Change it and Delete it".
fn words_list(actions: &[String]) -> String {
    let words: Vec<&str> = actions.iter().map(|a| action_words(a).0).collect();
    match words.as_slice() {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Reveal and Maintain need a fresh authenticator code for each change.
fn needs_fresh_code(actions: &[String]) -> bool {
    actions.iter().any(|a| a == "Reveal" || a == "Maintain")
}

/// One person or service user and every action they hold directly on the secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonAccess {
    /// Who, for grant and revoke calls.
    pub principal: GrantPrincipal,
    /// The name to show.
    pub display_name: String,
    /// The line under the name ("Person", "Service user · owned by …").
    pub who_line: String,
    /// Their actions, in [`SECRET_ACTIONS`] order.
    pub actions: Vec<String>,
}

impl PersonAccess {
    /// Stable row key: kind and id.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.principal.kind.as_str(), self.principal.id)
    }

    /// First word of the display name, for "{First} loses".
    #[must_use]
    pub fn first_name(&self) -> &str {
        self.display_name
            .split_whitespace()
            .next()
            .unwrap_or(&self.display_name)
    }
}

/// "Service user · owned by Ines Okoro" for a service user; "Person" otherwise.
#[must_use]
pub fn who_line(principal: &SecretGrantPrincipal) -> String {
    match (principal.kind, principal.owner_display_name.as_deref()) {
        (GrantPrincipalKind::ServiceUser, Some(owner)) => {
            format!("Service user · owned by {owner}")
        }
        (GrantPrincipalKind::ServiceUser, None) => "Service user".to_string(),
        (GrantPrincipalKind::User, _) => "Person".to_string(),
    }
}

/// Regroup the per-action grant lists by person, in the order people first appear.
#[must_use]
pub fn by_person(grants: &[SecretActionGrant]) -> Vec<PersonAccess> {
    let mut people: Vec<PersonAccess> = Vec::new();
    for action in SECRET_ACTIONS {
        let Some(row) = grants.iter().find(|g| g.action == action) else {
            continue;
        };
        for grant in &row.user_grants {
            let principal = GrantPrincipal {
                id: grant.user_id.clone(),
                kind: grant.kind,
            };
            if let Some(person) = people.iter_mut().find(|p| p.principal == principal) {
                person.actions.push(action.to_string());
            } else {
                people.push(PersonAccess {
                    principal,
                    display_name: grant.display_name.clone(),
                    who_line: who_line(grant),
                    actions: vec![action.to_string()],
                });
            }
        }
    }
    people
}

/// Run a grant change under the right step-up: a fresh code when Reveal or
/// Maintain is involved, else the confirmation window.
fn run_access_change<F, Fut>(ask: StepUpAsk, call: StepUpCall<()>, fresh: bool, change: F)
where
    F: Fn(Option<String>) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), ServerFnError>> + 'static,
{
    if fresh {
        spawn_with_fresh_totp(ask, call, move |code| change(Some(code)));
    } else {
        spawn_with_step_up(ask, call, move || change(None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(id: &str, name: &str, kind: GrantPrincipalKind) -> SecretGrantPrincipal {
        SecretGrantPrincipal {
            user_id: id.into(),
            display_name: name.into(),
            owner_display_name: (kind == GrantPrincipalKind::ServiceUser)
                .then(|| "Ines Okoro".to_string()),
            kind,
        }
    }

    fn grant(action: &str, who: Vec<SecretGrantPrincipal>) -> SecretActionGrant {
        SecretActionGrant {
            action: action.into(),
            permission_id: format!("p-{action}"),
            permission_name: format!("neutrino_secret.s1.{action}"),
            user_grants: who,
        }
    }

    #[test]
    fn grants_regroup_by_person_in_action_order_happy() {
        let mara = principal("u-mara", "Mara Lindqvist", GrantPrincipalKind::User);
        let bot = principal("svc-1", "Orders sync", GrantPrincipalKind::ServiceUser);
        let rows = vec![
            grant("Reveal", vec![mara.clone(), bot.clone()]),
            grant("View", vec![mara.clone()]),
            grant("Maintain", vec![]),
        ];
        let people = by_person(&rows);
        assert_eq!(people.len(), 2);
        assert_eq!(people[0].display_name, "Mara Lindqvist");
        assert_eq!(people[0].actions, vec!["View", "Reveal"]);
        assert_eq!(people[0].who_line, "Person");
        assert_eq!(people[1].who_line, "Service user · owned by Ines Okoro");
        assert_eq!(people[1].actions, vec!["Reveal"]);
        assert_eq!(people[0].first_name(), "Mara");
    }

    #[test]
    fn no_grants_means_no_people_sad() {
        assert!(by_person(&[grant("View", vec![])]).is_empty());
    }

    #[test]
    fn actions_read_in_plain_words() {
        assert_eq!(action_words("Reveal").0, "Read the value");
        assert_eq!(action_words("Maintain").0, "Manage access");
        assert_eq!(words_list(&["Reveal".into()]), "Read the value");
        assert_eq!(
            words_list(&["View".into(), "Edit".into(), "Delete".into()]),
            "See it, Change it and Delete it"
        );
        assert!(needs_fresh_code(&["View".into(), "Maintain".into()]));
        assert!(!needs_fresh_code(&["View".into(), "Edit".into()]));
    }
}

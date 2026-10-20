//! The New worktree picker shows a Jira ticket's status (feature 1101, US1, FR-002, FR-009).

mod support;

use micold_client::ui::issue_rows;
use micold_core::github::Issue;
use support::jira_picker::{form_with, state_with, ticket};
use support::layout::{painted_text_settled, renderer, view_of};

fn statuses(form: &micold_client::features::worktree_form::WorktreeForm) -> Vec<Option<String>> {
    issue_rows(form)
        .0
        .iter()
        .map(|r| r.status.as_ref().map(|s| s.name.clone()))
        .collect()
}

fn listed() -> Vec<Issue> {
    vec![
        ticket(1, "Alpha", Some(("To Do", "new"))),
        ticket(2, "Bravo", Some(("In Progress", "indeterminate"))),
        ticket(3, "Charlie", Some(("In Review", "indeterminate"))),
        ticket(4, "Delta", None),
    ]
}

#[test]
fn each_listed_row_carries_its_own_status_and_a_row_without_one_carries_none() {
    let mut got = statuses(&form_with(listed(), ""));
    got.sort();
    assert_eq!(
        got,
        [
            None,
            Some("In Progress".into()),
            Some("In Review".into()),
            Some("To Do".into())
        ]
    );
}

#[test]
fn search_results_show_their_status_too() {
    assert_eq!(
        statuses(&form_with(listed(), "Charlie")),
        [Some("In Review".to_string())]
    );
}

#[test]
fn the_chip_text_is_painted_only_for_rows_with_a_status() {
    let painted = |issues| -> Vec<String> {
        painted_text_settled(view_of(&state_with(form_with(issues, ""))), &mut renderer())
            .into_iter()
            .map(|t| t.content)
            .collect()
    };
    let with = painted(listed());
    for name in ["To Do", "In Progress", "In Review"] {
        assert!(with.iter().any(|t| t == name), "{name} painted: {with:?}");
    }
    let without = painted(vec![ticket(4, "Delta", None)]);
    assert!(!without.iter().any(|t| t == "To Do"), "{without:?}");
}

#[test]
fn github_rows_have_no_status() {
    let issue = Issue::new(7, "Sidebar".into(), Vec::new(), String::new());
    assert_eq!(statuses(&form_with(vec![issue], "")), [None]);
}

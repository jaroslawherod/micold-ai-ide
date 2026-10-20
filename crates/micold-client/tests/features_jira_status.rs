//! The Jira status feature in isolation (feature 1101, SC-004 of 021): builds only
//! `features::jira_status::State` and drives it with the events the shell sends.

use std::path::PathBuf;
use std::time::Duration;

use micold_client::features::jira_status::{linked_keys, State};
use micold_core::jira::{JiraConnection, ProjectKey};
use micold_core::jira_status::{TicketStatus, Trigger};
use micold_core::worktree::{Worktree, WorktreeStatus};

fn worktree(dir: &str) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from("/repo").join(dir),
        branch: None,
        status: WorktreeStatus::Valid,
        included: false,
    }
}

fn project() -> ProjectKey {
    ProjectKey::parse("ABC").unwrap()
}

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

#[test]
fn linked_keys_are_the_projects_own_each_once() {
    let worktrees = [
        worktree("fix-ABC-1_a"),
        worktree("feat-abc-1_b"),
        worktree("fix-XYZ-2_c"),
        worktree("plain"),
    ];
    assert_eq!(linked_keys(&worktrees, &project()), vec!["ABC-1"]);
}

#[test]
fn a_read_is_planned_once_and_the_answer_is_kept() {
    let mut state = State::default();
    let worktrees = [worktree("fix-ABC-1_a")];
    let read = state
        .plan(Trigger::Open, Duration::ZERO, &worktrees, &project())
        .expect("a ticket is linked");
    assert!(state
        .plan(Trigger::Open, Duration::ZERO, &worktrees, &project())
        .is_none());
    let done = TicketStatus::from_jira(Some("Done"), Some("done"));
    state.apply(
        read.epoch,
        &read.keys,
        Ok(vec![("ABC-1".to_string(), done)]),
        Duration::from_secs(1),
    );
    assert_eq!(state.board.status_of("ABC-1").unwrap().status.name, "Done");
}

#[test]
fn project_opened_and_connection_changes_reset_the_board() {
    let mut state = State::default();
    assert!(state.is_dormant());
    state.connected(&conn());
    assert!(state.open_pending);
    state.project_opened();
    assert!(state.open_pending);
    state.clear();
    assert!(state.is_dormant());
}

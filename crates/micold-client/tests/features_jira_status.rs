//! The Jira status feature in isolation (feature 1101, SC-004 of 021): builds only
//! `features::jira_status::State` and drives it with the events the shell sends.

use std::path::PathBuf;
use std::time::Duration;

use micold_client::features::jira_status::{branch_title_keys, linked_keys, State};
use micold_core::jira::{JiraConnection, ProjectKey};
use micold_core::jira_status::{KeyedIssue, TicketStatus, Trigger};
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
        Ok(vec![KeyedIssue {
            key: "ABC-1".to_string(),
            status: done,
            summary: None,
        }]),
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

// ---- Feature 1504 (T018, FR-002, FR-004, L6): Jira title tickets found only in a branch ----

fn on_branch(dir: &str, branch: &str) -> Worktree {
    Worktree {
        branch: Some(branch.to_string()),
        ..worktree(dir)
    }
}

#[test]
fn branch_title_keys_are_jira_keys_of_the_project_in_branches_of_folders_without_a_ticket() {
    let worktrees = [
        on_branch("project-thread-t2a600", "feat/ABC-42_login"),
        on_branch("other-thread", "fix/abc-43_lower"),
        // The folder's ticket wins: the branch's key is no title ticket.
        on_branch("fix-ABC-1_a", "feat/ABC-5_x"),
        on_branch("feat-813_b", "feat/ABC-6_y"),
        // Another project's key, a GitHub number, no ticket at all.
        on_branch("third-thread", "feat/XYZ-7_z"),
        on_branch("fourth-thread", "feat/1504-readable-titles"),
        on_branch("fifth-thread", "main-work"),
        on_branch("again-thread", "feat/ABC-42_again"),
    ];
    assert_eq!(
        branch_title_keys(&worktrees, &project()),
        vec!["ABC-42", "ABC-43"]
    );
}

#[test]
fn a_plan_asks_the_linked_keys_then_the_branch_only_keys_and_keeps_no_branch_status() {
    let mut state = State::default();
    let worktrees = [
        worktree("fix-ABC-1_a"),
        on_branch("project-thread-t2a600", "feat/ABC-42_login"),
        on_branch("third-thread", "feat/XYZ-7_z"),
    ];
    let read = state
        .plan(Trigger::Open, Duration::ZERO, &worktrees, &project())
        .expect("keys to read");
    assert_eq!(read.keys, vec!["ABC-1", "ABC-42"]);
    let todo = TicketStatus::from_jira(Some("To Do"), Some("new"));
    state.apply(
        read.epoch,
        &read.keys,
        Ok(vec![
            KeyedIssue {
                key: "ABC-1".into(),
                status: todo.clone(),
                summary: Some("Linked".into()),
            },
            KeyedIssue {
                key: "ABC-42".into(),
                status: todo,
                summary: Some("Branch only".into()),
            },
        ]),
        Duration::from_secs(1),
    );
    assert!(state.board.status_of("ABC-1").is_some());
    assert!(state.board.status_of("ABC-42").is_none());
    assert_eq!(state.board.summary_of("ABC-42"), Some("Branch only"));
}

#[test]
fn a_branch_only_key_alone_is_still_read_and_another_projects_is_not() {
    let mut state = State::default();
    let only_branch = [on_branch("project-thread-t2a600", "feat/ABC-42_login")];
    let read = state
        .plan(Trigger::Open, Duration::ZERO, &only_branch, &project())
        .expect("a summary-only read");
    assert_eq!(read.keys, vec!["ABC-42"]);
    let mut state = State::default();
    let other = [on_branch("third-thread", "feat/XYZ-7_z")];
    assert!(state
        .plan(Trigger::Open, Duration::ZERO, &other, &project())
        .is_none());
}

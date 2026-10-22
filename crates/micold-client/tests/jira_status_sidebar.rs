//! A Jira ticket's status on the sidebar and how it is kept fresh (feature 1101, M2: FR-003 to
//! FR-008, FR-010, FR-014, FR-015; SC-002, SC-003).
//!
//! The refresh driver lives in the binary; its own tests (the gate, the clear, the Focus throttle)
//! are `jira_status_driver` in `main_tests.rs`. These drive what the driver drives: the state's
//! `plan` and `apply`, `read_statuses` through a `FakeJiraSource`, and the painted sidebar.

mod support;

use std::time::Duration;

use iced::Element;
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::github::IssueLoadError;
use micold_core::jira::{ApiToken, FakeJiraSource, JiraConnection, ProjectKey};
use micold_core::jira_status::{read_statuses, KeyedStatus, TicketStatus, Trigger};
use support::layout as lay;
use support::sidebar_fixture::{deliver, listed, state_with_active};

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn token() -> ApiToken {
    ApiToken::parse("sentinel-token-0123").unwrap()
}

fn project() -> ProjectKey {
    conn().project_key
}

fn status(name: &str, category: &str) -> Option<TicketStatus> {
    TicketStatus::from_jira(Some(name), Some(category))
}

fn read(key: &str, name: &str, category: &str) -> KeyedStatus {
    (key.to_string(), status(name, category))
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// A state with worktrees named for `keys` (and `extra` dir names), as the client lists them.
fn state_with(dirs: &[&str]) -> State {
    let mut state = state_with_active();
    deliver(&mut state, dirs.iter().map(|d| listed(d)).collect());
    state
}

/// Run one read for `trigger` against `source` the way the shell does, and apply the answer.
fn refresh(state: &mut State, source: &FakeJiraSource, trigger: Trigger, now: Duration) -> bool {
    let worktrees = state.worktree.worktrees.clone();
    let Some(read) = state.jira_status.plan(trigger, now, &worktrees, &project()) else {
        return false;
    };
    let result = read_statuses(source, &conn(), &token(), &read.keys);
    state.jira_status.apply(read.epoch, &read.keys, result, now);
    true
}

fn painted(state: &State) -> Vec<String> {
    let view: Element<'_, Message> = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &Sandbox::default(),
    );
    let mut renderer = lay::renderer();
    lay::painted_text_settled(view, &mut renderer)
        .into_iter()
        .map(|t| t.content)
        .collect()
}

fn shows(state: &State, text: &str) -> bool {
    painted(state).iter().any(|t| t == text)
}

#[test]
fn the_chip_stands_beside_the_key_tag() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read(
        "ABC-1",
        "In Progress",
        "indeterminate",
    )]));
    assert!(refresh(&mut state, &source, Trigger::Open, secs(0)));
    assert!(shows(&state, "In Progress"));
    assert!(shows(&state, "ABC-1"));
}

#[test]
fn a_worktree_without_a_ticket_or_a_status_has_no_chip() {
    let mut state = state_with(&["fix-ABC-1_login", "chore-plain"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![("ABC-1".into(), None)]));
    let before = painted(&state);
    refresh(&mut state, &source, Trigger::Open, secs(0));
    assert!(state.jira_status.board.status_of("ABC-1").is_none());
    assert_eq!(source.status_calls(), vec![vec!["ABC-1".to_string()]]);
    // Nothing was added to the sidebar: the key tag stays and no chip joins it.
    assert_eq!(painted(&state), before);
    assert!(shows(&state, "ABC-1"));
}

#[test]
fn another_projects_key_is_never_asked_for() {
    let mut state = state_with(&["fix-XYZ-9_other"]);
    let source = FakeJiraSource::new();
    assert!(!refresh(&mut state, &source, Trigger::Open, secs(0)));
    assert!(source.status_calls().is_empty());
}

#[test]
fn a_focus_after_five_minutes_follows_the_ticket_and_a_recent_one_does_not_ask() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read(
        "ABC-1",
        "In Progress",
        "indeterminate",
    )]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    source.script_statuses(Ok(vec![read("ABC-1", "In Review", "indeterminate")]));
    assert!(!refresh(&mut state, &source, Trigger::Focus, secs(299)));
    assert!(shows(&state, "In Progress"));
    assert!(refresh(&mut state, &source, Trigger::Focus, secs(301)));
    assert!(shows(&state, "In Review"));
    assert!(!shows(&state, "In Progress"));
}

#[test]
fn a_done_ticket_keeps_its_chip() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "Done", "done")]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    assert!(shows(&state, "Done"));
}

#[test]
fn a_failed_refresh_keeps_the_last_status_marked_stale_and_asks_nothing_more() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "To Do", "new")]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    source.script_statuses(Err(IssueLoadError::RateLimited));
    refresh(&mut state, &source, Trigger::Focus, secs(400));
    let entry = state.jira_status.board.status_of("ABC-1").expect("kept");
    assert!(entry.stale);
    assert_eq!(entry.status.name, "To Do");
    assert!(shows(&state, "To Do"));
    // The failure raised no request of its own: only the open read and the focus read were made.
    assert_eq!(source.status_calls().len(), 2);
}

#[test]
fn a_ticket_jira_no_longer_returns_is_retired() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "To Do", "new")]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    source.script_statuses(Ok(vec![]));
    refresh(&mut state, &source, Trigger::Focus, secs(400));
    assert!(state.jira_status.board.status_of("ABC-1").is_none());
    assert!(!shows(&state, "To Do"));
}

#[test]
fn clearing_the_state_drops_the_chips_from_the_sidebar() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "To Do", "new")]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    assert!(shows(&state, "To Do"));
    // That switching off or removing the connection calls this is `jira_status_driver`'s test.
    state.jira_status.clear();
    assert!(state.jira_status.board.is_idle());
    assert!(!shows(&state, "To Do"));
}

#[test]
fn fifty_keys_are_one_call_and_a_hundred_and_one_are_two() {
    for (count, calls) in [(50usize, 1usize), (101, 2)] {
        let dirs: Vec<String> = (1..=count).map(|n| format!("fix-ABC-{n}_t")).collect();
        let refs: Vec<&str> = dirs.iter().map(String::as_str).collect();
        let mut state = state_with(&refs);
        let source = FakeJiraSource::new();
        refresh(&mut state, &source, Trigger::Open, secs(0));
        assert_eq!(source.status_calls().len(), calls, "{count} keys");
    }
}

#[test]
fn a_read_in_flight_coalesces_later_triggers() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let worktrees = state.worktree.worktrees.clone();
    let first = state
        .jira_status
        .plan(Trigger::Open, secs(0), &worktrees, &project());
    assert!(first.is_some());
    assert!(state
        .jira_status
        .plan(Trigger::Open, secs(1), &worktrees, &project())
        .is_none());
    assert!(state
        .jira_status
        .plan(Trigger::Focus, secs(1000), &worktrees, &project())
        .is_none());
}

#[test]
fn a_chip_names_no_token_or_site() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "To Do", "new")]));
    refresh(&mut state, &source, Trigger::Open, secs(0));
    for text in painted(&state) {
        assert!(!text.contains("sentinel-token"), "{text}");
        assert!(!text.contains("atlassian.net"), "{text}");
    }
    let entry = state.jira_status.board.status_of("ABC-1").unwrap();
    let said = entry.status.accessible_text(true);
    assert!(!said.contains("sentinel-token") && !said.contains("atlassian"));
}

#[test]
fn a_changed_connection_drops_the_old_statuses_and_reads_again() {
    let mut state = state_with(&["fix-ABC-1_login"]);
    let source = FakeJiraSource::new().with_statuses(Ok(vec![read("ABC-1", "Done", "done")]));
    state.jira_status.connected(&conn());
    refresh(&mut state, &source, Trigger::Open, secs(0));
    state.jira_status.open_pending = false;
    // The same connection again changes nothing.
    state.jira_status.connected(&conn());
    assert!(state.jira_status.board.status_of("ABC-1").is_some());
    assert!(!state.jira_status.open_pending);
    // Another account of the same site and project is another connection.
    let other = JiraConnection::parse("https://acme.atlassian.net", "ABC", "you@acme.com").unwrap();
    state.jira_status.connected(&other);
    assert!(state.jira_status.board.status_of("ABC-1").is_none());
    assert!(state.jira_status.open_pending);
}

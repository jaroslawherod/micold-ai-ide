//! A Jira ticket's status on the Overview cards (feature 1101, M3: US3 scenarios 1-2, FR-010).
//!
//! The card and the sidebar row read the one board, so a ticket shown in both shows the same
//! status. Driven through the state's `plan`/`apply` (the shell's refresh driver is in the binary).

mod support;

use std::time::Duration;

use iced::Element;
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_client::ui::overview;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::github::IssueLoadError;
use micold_core::jira::{ApiToken, FakeJiraSource, JiraConnection};
use micold_core::jira_status::{read_statuses, KeyedStatus, TicketStatus, Trigger};
use support::layout as lay;
use support::sidebar_fixture::{deliver, listed, state_with_active};

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn read(key: &str, name: &str, category: &str) -> KeyedStatus {
    (
        key.to_string(),
        TicketStatus::from_jira(Some(name), Some(category)),
    )
}

/// A state with worktrees for `dirs` and `statuses` read into the board.
fn state_with(dirs: &[&str], statuses: Vec<KeyedStatus>) -> State {
    let mut state = state_with_active();
    deliver(&mut state, dirs.iter().map(|d| listed(d)).collect());
    refresh(&mut state, Ok(statuses), Duration::ZERO, Trigger::Open);
    state
}

fn refresh(
    state: &mut State,
    answer: Result<Vec<KeyedStatus>, IssueLoadError>,
    now: Duration,
    trigger: Trigger,
) {
    let source = FakeJiraSource::new().with_statuses(answer);
    let token = ApiToken::parse("sentinel-token-0123").unwrap();
    let worktrees = state.worktree.worktrees.clone();
    let project = conn().project_key;
    let plan = state
        .jira_status
        .plan(trigger, now, &worktrees, &project)
        .expect("a read is due");
    let result = read_statuses(&source, &conn(), &token, &plan.keys);
    state.jira_status.apply(plan.epoch, &plan.keys, result, now);
}

/// Every text the window paints, with the Overview open or closed.
fn painted(state: &mut State, overview: bool) -> Vec<String> {
    state.overview.open = overview;
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

fn count(texts: &[String], text: &str) -> usize {
    texts.iter().filter(|t| t.as_str() == text).count()
}

#[test]
fn a_linked_worktrees_card_shows_the_ticket_status_as_the_sidebar_does() {
    let mut state = state_with(
        &["fix-ABC-1_login"],
        vec![read("ABC-1", "In Progress", "indeterminate")],
    );
    assert_eq!(count(&painted(&mut state, false), "In Progress"), 1);
    // Overview open: the sidebar row and the card both show it (US3 scenarios 1-2, FR-010).
    assert_eq!(count(&painted(&mut state, true), "In Progress"), 2);
}

#[test]
fn both_places_follow_a_changed_status_together() {
    let mut state = state_with(
        &["fix-ABC-1_login"],
        vec![read("ABC-1", "In Progress", "indeterminate")],
    );
    refresh(
        &mut state,
        Ok(vec![read("ABC-1", "In Review", "indeterminate")]),
        Duration::from_secs(600),
        Trigger::Focus,
    );
    let texts = painted(&mut state, true);
    assert_eq!(count(&texts, "In Review"), 2);
    assert_eq!(count(&texts, "In Progress"), 0);
}

#[test]
fn a_card_without_a_ticket_or_status_has_no_chip() {
    let mut state = state_with(
        &["fix-ABC-1_login", "feat-7_plain"],
        vec![read("ABC-1", "To Do", "new")],
    );
    // One chip on the linked worktree's row and one on its card; the numeric ticket has none.
    assert_eq!(count(&painted(&mut state, true), "To Do"), 2);
}

#[test]
fn a_lowercase_directory_name_still_finds_its_ticket() {
    // The app creates `feat-abc-1_...`; the sidebar shows `ABC-1` and so must the card.
    let mut state = state_with(
        &["feat-abc-1_login"],
        vec![read("ABC-1", "In Review", "indeterminate")],
    );
    assert_eq!(count(&painted(&mut state, true), "In Review"), 2);
}

/// The status the card is given, and whether it is marked stale (FR-008, FR-010).
fn card_status_of(state: &State) -> (String, bool) {
    let models = overview::cards(state);
    let model = models
        .iter()
        .find(|m| overview::card_status(state, m).is_some())
        .expect("the linked worktree's card has a status");
    let (status, stale) = overview::card_status(state, model).unwrap();
    (status.name.clone(), stale)
}

#[test]
fn a_failed_refresh_keeps_the_card_status_and_marks_it_stale_as_the_sidebar_does() {
    let mut state = state_with(
        &["fix-ABC-1_login"],
        vec![read("ABC-1", "In Progress", "indeterminate")],
    );
    assert_eq!(card_status_of(&state), ("In Progress".to_string(), false));
    refresh(
        &mut state,
        Err(IssueLoadError::RateLimited),
        Duration::from_secs(600),
        Trigger::Focus,
    );
    assert!(state.jira_status.board.status_of("ABC-1").unwrap().stale);
    assert_eq!(card_status_of(&state), ("In Progress".to_string(), true));
    // The last status stays painted on both surfaces.
    assert_eq!(count(&painted(&mut state, true), "In Progress"), 2);
}

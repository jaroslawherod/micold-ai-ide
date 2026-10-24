//! The selector's advanced JQL mode: the shell's half (feature 1186,
//! `specs/1186-jira-advanced-jql/contracts/selector-source-and-storage.md` §3).
//!
//! The reducer decides everything about the selector (`features/jira_query.rs`). What is left here
//! is running the scoped query it accepted, through the one Jira gate (`jira_request` in
//! `shell/issues.rs`), off the update thread.
//!
//! # When a query runs
//!
//! [`start_query`] is the only path to `JiraSource::query_page`. It runs when the form newly
//! awaits a run after a message: `Run` accepted, or the Jira source shown in advanced mode with a
//! last run query (FR-004a, FR-003a). Switching the mode and typing never run anything (FR-004).
//! `tests/issues_are_requested_only_on_named_events.rs` counts the callers.
//!
//! Nothing here logs or notifies: the query leaves the form only as the request (FR-016).

use iced::Task;
use micold_client::app::Message;
use micold_client::features::jira_query::Msg as QueryMsg;
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_core::github::IssueLoadError;
use micold_core::jira_query::{QueryFailure, ScopedQuery};

use super::issues::{jira_request, on_basic_shown, on_issue_row_picked};
use crate::App;

/// A message of the selector's advanced mode. A row pick resolves to its issue as a basic row's
/// does (FR-009); anything else goes to the reducer, and what it newly awaits starts: the basic
/// load on a switch back to a list never requested (FR-003), or the run it accepted.
pub fn on_message(app: &mut App, msg: QueryMsg) -> Task<Message> {
    if let QueryMsg::RowPicked(index) = msg {
        return on_issue_row_picked(app, index);
    }
    let before = app.core.worktree_form.issue_request_seq;
    let before_query = app.core.worktree_form.awaited_jira_query();
    app.core
        .update(Message::WorktreeForm(FormMsg::JiraQuery(msg)));
    Task::batch([
        on_basic_shown(app, before),
        run_if_newly_awaited(app, before_query),
    ])
}

/// Start the run the last message made the form await, comparing the awaited seq before it
/// (`before`) and after. A message the reducer dropped leaves the run as it was, already running.
pub(crate) fn run_if_newly_awaited(app: &mut App, before: Option<u64>) -> Task<Message> {
    let awaited = app.core.worktree_form.awaited_jira_query();
    let request = app
        .core
        .worktree_form
        .form
        .as_ref()
        .and_then(|form| form.jira_query.running())
        .filter(|_| awaited != before)
        .map(|(seq, scoped)| (seq, scoped.clone()));
    match request {
        Some((seq, scoped)) => start_query(app, seq, scoped),
        None => Task::none(),
    }
}

/// Run `scoped` on a blocking thread through the Jira gate, answering `Ran { seq }`.
fn start_query(app: &mut App, seq: u64, scoped: ScopedQuery) -> Task<Message> {
    let request = match jira_request(app) {
        Ok(request) => request,
        Err(why) => {
            return Task::done(ran(
                seq,
                Err(QueryFailure::Load(IssueLoadError::Other(why.to_string()))),
            ))
        }
    };
    Task::perform(
        request.run(move |source, connection, token| {
            Ok(micold_core::jira_query::load(
                source, connection, token, &scoped,
            ))
        }),
        move |joined| {
            let result = match joined {
                Ok(Ok(answered)) => answered,
                Ok(Err(refused)) => Err(QueryFailure::Load(refused)),
                Err(stopped) => Err(QueryFailure::Load(IssueLoadError::Other(format!(
                    "the query stopped unexpectedly: {stopped}"
                )))),
            };
            ran(seq, result)
        },
    )
}

/// The answer to run `seq`.
fn ran(seq: u64, result: Result<micold_core::github::IssueListing, QueryFailure>) -> Message {
    Message::WorktreeForm(FormMsg::JiraQuery(QueryMsg::Ran { seq, result }))
}

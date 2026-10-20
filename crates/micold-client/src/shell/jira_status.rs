//! Reading the linked Jira tickets' statuses: the shell's half (feature 1101).
//!
//! The reducer (`features/jira_status.rs`) decides when a read is due and what its answer
//! changes. What is left here is the request: behind feature 533's one gate
//! ([`jira_request`](super::issues::jira_request): the switch, the project's connection, the
//! token, the source), on a blocking thread, answering `Message::JiraStatusesRead`.
//!
//! # When Jira is contacted, and why only then
//!
//! [`trigger`] is the only path to the statuses request, and it is reached from two places:
//! [`on_focus`] (the window regained focus) and [`after_update`] (the project opened and its
//! worktrees are known). `tests/jira_is_requested_only_on_named_events.rs` counts the callers.
//! Nothing is raised on a failure (FR-008): the answer is an `Err` the board marks stale.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::jira_status::now;
use micold_core::github::IssueLoadError;
use micold_core::jira_status::{read_statuses, Trigger};

use super::issues::jira_request;
use crate::App;

/// The window regained focus (FR-005).
pub fn on_focus(app: &mut App) -> Task<Message> {
    trigger(app, Trigger::Focus)
}

/// After every message: statuses of a project whose Jira switch went off or whose connection was
/// removed are dropped (FR-008), and a project that just opened reads once its worktrees are known.
pub fn after_update(app: &mut App) -> Task<Message> {
    let gate_open = jira_request(app).is_ok();
    let state = &mut app.core.jira_status;
    if !gate_open {
        state.open_pending = false;
        if !state.board.is_idle() {
            state.clear();
        }
        return Task::none();
    }
    if state.open_pending && !app.core.worktree.worktrees.is_empty() {
        state.open_pending = false;
        return trigger(app, Trigger::Open);
    }
    Task::none()
}

/// Start a read for `trigger` when the gate passes and the board says it is due.
fn trigger(app: &mut App, trigger: Trigger) -> Task<Message> {
    let Ok(request) = jira_request(app) else {
        return Task::none();
    };
    let project = request.connection.project_key.clone();
    let Some(read) = app.core.jira_status.plan(
        trigger,
        now(),
        &app.core.worktree.worktrees,
        &project,
    ) else {
        return Task::none();
    };
    let keys = read.keys.clone();
    let epoch = read.epoch;
    let asked = keys.clone();
    Task::perform(
        request.run(move |source, connection, token| {
            read_statuses(source, connection, token, &asked)
        }),
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "the status read stopped unexpectedly: {stopped}"
                )))
            });
            Message::JiraStatusesRead {
                epoch,
                keys: keys.clone(),
                result,
                at: now(),
            }
        },
    )
}

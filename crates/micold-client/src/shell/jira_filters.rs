//! The Jira filters of the add-worktree form: the shell's half (feature 1185).
//!
//! The reducer (`features/jira_filters.rs`) decides everything about the filters. What is left
//! here is what it cannot do: ask Jira for a filter's choices, and reload the listing with the
//! filters now chosen, both on a blocking thread and both behind the one gate every Jira request
//! passes (`shell::issues::jira_request`).
//!
//! # When Jira is asked, and why only then
//!
//! A filter's choices are asked for when the reducer accepted the field's first focus or a
//! **Retry** of a failed request, and handed out a new filter seq for it (FR-006, research R7);
//! the listing is reloaded when the reducer accepted a pick, a clear or **Clear filters** that
//! changed the chosen filters and handed out a new issue seq (FR-008, research R6). Nothing else
//! here sends anything: not opening the form, not showing the Jira source, not a redraw.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::worktree_form::jira_filters::FilterMsg;
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_core::github::IssueLoadError;
use micold_core::jira::{ApiToken, JiraConnection, JiraSource};
use micold_core::jira_filter::FilterKind;

use crate::shell::issues::{jira_request, start_jira_load};
use crate::App;

/// A filter message: the reducer takes it, then the choice request or the listing reload it newly
/// awaits starts.
pub fn on_filter(app: &mut App, msg: FilterMsg) -> Task<Message> {
    let issue_before = app.core.worktree_form.issue_request_seq;
    let choice_before = app.core.worktree_form.filter_request_seq;
    app.core
        .update(Message::WorktreeForm(FormMsg::JiraFilter(msg)));
    let mut work = Vec::new();
    if let Some(seq) = app
        .core
        .worktree_form
        .awaited_issue_load()
        .filter(|seq| *seq > issue_before)
    {
        work.push(start_jira_load(app, seq));
    }
    if let Some((kind, seq)) = app
        .core
        .worktree_form
        .awaited_choices()
        .filter(|(_, seq)| *seq > choice_before)
    {
        work.push(start_choices(app, kind, seq));
    }
    Task::batch(work)
}

/// Ask Jira for `kind`'s choices under `seq`, on a blocking thread, answering the filter's
/// `…Loaded { seq }`.
fn start_choices(app: &mut App, kind: FilterKind, seq: u64) -> Task<Message> {
    match kind {
        FilterKind::Component => ask(
            app,
            |source, connection, token| source.components(connection, token),
            move |result| FilterMsg::ComponentsLoaded { seq, result },
        ),
        FilterKind::FixVersion => ask(
            app,
            |source, connection, token| source.versions(connection, token),
            move |result| FilterMsg::VersionsLoaded { seq, result },
        ),
        // The reporter and the label are not offered in this build: neither awaits choices.
        FilterKind::Reporter | FilterKind::Label => Task::none(),
    }
}

/// Run one choice request behind the Jira gate and answer with `answer` (a failure to start or a
/// stopped thread as an error, so the field shows it with **Retry**).
fn ask<T: Send + 'static>(
    app: &mut App,
    request_choices: impl FnOnce(&dyn JiraSource, &JiraConnection, &ApiToken) -> Result<Vec<T>, IssueLoadError>
        + Send
        + 'static,
    answer: impl Fn(Result<Vec<T>, IssueLoadError>) -> FilterMsg + Send + 'static,
) -> Task<Message> {
    let answer = move |result| Message::WorktreeForm(FormMsg::JiraFilter(answer(result)));
    let request = match jira_request(app) {
        Ok(request) => request,
        Err(why) => return Task::done(answer(Err(IssueLoadError::Other(why.to_string())))),
    };
    Task::perform(request.run(request_choices), move |joined| {
        answer(joined.unwrap_or_else(|stopped| {
            Err(IssueLoadError::Other(format!(
                "the choice request stopped unexpectedly: {stopped}"
            )))
        }))
    })
}

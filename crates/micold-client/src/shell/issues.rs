//! The GitHub issue source of the add-worktree form: the shell's half (feature 034).
//!
//! The reducer decides everything about the picker (`features/worktree_form.rs`). What is left
//! here is what it cannot do: ask the daemon for the repository's remotes when the form opens, and
//! run the load — locate `gh`, then read the open issues — off the update thread.
//!
//! # When GitHub is contacted, and why only then
//!
//! [`start_issue_load`], [`start_issue_search`] and [`start_issue_descriptions`] are the only paths
//! to the issue source. The load runs when the reducer accepts `SourceChanged(Issue)` or
//! `IssueRetry` from a failed load; the search beyond the loaded issues runs when the debounce
//! after a keystroke on a capped list ends with the reducer accepting `IssueSearchDue`, or on
//! `IssueRetry` from a failed search (FR-003, FR-005a). Opening the form asks the daemon for the
//! remotes, a local read, and nothing else.
//! `tests/issues_are_requested_only_on_named_events.rs` counts the callers.
//!
//! # Why the descriptions come after the list
//!
//! A list request that also carried every issue's body took almost twice as long on a repository
//! of 1,000 issues (feature 038, research R14), so the list is read without them and shown at
//! once. The descriptions follow in a second pass, a page at a time: it starts when the reducer
//! accepted a load, and goes on when it accepted a page and awaits another. Nothing else starts
//! it: not a view, not a timer, not a cursor resting on a row (038 FR-024). A page that fails ends
//! the pass and shows nothing; the next load starts a new one.
//!
//! # Why the search waits
//!
//! GitHub's search API allows 30 requests a minute, so a search waits [`ISSUE_SEARCH_DEBOUNCE`]
//! after the last keystroke (research R9). The local ranking does not wait: the reducer re-ranks
//! the held issues on every keystroke. Every keystroke that changes the trimmed text hands out a
//! fresh seq, so an older keystroke's timer, or an older search's answer, finds nothing to apply
//! to (FR-007a); one that changes only surrounding whitespace keeps the search as it is.
//!
//! # Why the environment include is resolved inside the load
//!
//! `gh` installed by a version manager is often only on the `PATH` the user's include script
//! builds, so the lookup searches that `PATH` first (research R3). Sourcing the script can take up
//! to its timeout, so a cache miss is resolved on the blocking thread with the rest of the load and
//! handed back in `IssuesLoaded` for the shell to cache, rather than stalling the update thread.

use std::path::PathBuf;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::worktree_form::{BranchSource, DescriptionRequest, Msg as FormMsg};
use micold_core::env_include::EnvIncludeSnapshot;
use micold_core::github::{
    env_include_path, load_listing, DescriptionPage, IssueListing, IssueLoadError,
};
use micold_core::protocol::messages::ClientMsg;

use crate::shell::daemon_sync::{on_add_worktree_source_changed, send_op, PendingOp};
use crate::shell::env_include::resolve_env_include;
use crate::App;

/// How long typing must pause before the search beyond the loaded issues runs (research R9).
pub(crate) const ISSUE_SEARCH_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

/// Why the remotes could not be read while the session service is unreachable.
pub(crate) const NOT_CONNECTED: &str = "not connected to the session service";

/// The form opened: ask the daemon for the active project's remotes (FR-002).
///
/// Not connected, the form says so in its caption. `send_op`'s toast would be raised under the
/// modal's scrim, unseen, and would call a read "a request that may not have taken effect".
pub fn on_form_opened(app: &mut App) -> Task<Message> {
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    let Some(project) = app.core.workspace.active.clone() else {
        app.core
            .update(Message::WorktreeForm(FormMsg::RemotesListed(Err(
                "no project is open".to_string(),
            ))));
        return Task::none();
    };
    if app.daemon.is_none() {
        app.core
            .update(Message::WorktreeForm(FormMsg::RemotesListed(Err(
                NOT_CONNECTED.to_string(),
            ))));
        return Task::none();
    }
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::RemoteList { project: asked_for },
        move |req| ClientMsg::RemoteList { req, project },
    );
    Task::none()
}

/// The branch source changed. Choosing the issue source starts a load when the reducer accepted
/// it — it refuses while no GitHub remote is known, and ignores a second press.
pub fn on_source_changed(app: &mut App, source: BranchSource) -> Task<Message> {
    let before = app.core.worktree_form.issue_request_seq;
    let listed = on_add_worktree_source_changed(app, source);
    match newly_awaited(app, before) {
        Some(chosen) => start_issue_load(app, chosen),
        None => listed,
    }
}

/// Retry a failed load or a failed search, when the reducer accepted the retry.
pub fn on_issue_retry(app: &mut App) -> Task<Message> {
    let before = app.core.worktree_form.issue_request_seq;
    app.core.update(Message::WorktreeForm(FormMsg::IssueRetry));
    match newly_awaited(app, before) {
        Some(retried) => start_issue_load(app, retried),
        None => retry_issue_search(app, before),
    }
}

/// Run the search again when the retry was the failed search's.
fn retry_issue_search(app: &mut App, before: u64) -> Task<Message> {
    match app
        .core
        .worktree_form
        .awaited_issue_search()
        .filter(|seq| *seq > before)
    {
        Some(retried) => start_issue_search(app, retried),
        None => Task::none(),
    }
}

/// The issue search text changed. When the reducer left a search pending, wait out the debounce.
pub fn on_issue_query_changed(app: &mut App, text: String) -> Task<Message> {
    let before = app.core.worktree_form.issue_request_seq;
    app.core
        .update(Message::WorktreeForm(FormMsg::IssueQueryChanged(text)));
    match app
        .core
        .worktree_form
        .pending_issue_search()
        .filter(|seq| *seq > before)
    {
        // Built inside the future: the timer needs the runtime the task runs on.
        Some(seq) => Task::perform(
            async move { tokio::time::sleep(ISSUE_SEARCH_DEBOUNCE).await },
            move |()| Message::WorktreeForm(FormMsg::IssueSearchDue { seq }),
        ),
        None => Task::none(),
    }
}

/// The debounce ended: search, when this was still the pending keystroke's.
pub fn on_issue_search_due(app: &mut App, seq: u64) -> Task<Message> {
    let was_pending = app.core.worktree_form.pending_issue_search() == Some(seq);
    app.core
        .update(Message::WorktreeForm(FormMsg::IssueSearchDue { seq }));
    match app
        .core
        .worktree_form
        .awaited_issue_search()
        .filter(|awaited| was_pending && *awaited == seq)
    {
        Some(due) => start_issue_search(app, due),
        None => Task::none(),
    }
}

/// A row of the shown results was picked: resolve it to its issue while the index and the results
/// are both in hand, and read the label-to-type mapping as it is stored now — the default with no
/// settings store (FR-014a).
pub fn on_issue_row_picked(app: &mut App, index: usize) -> Task<Message> {
    if let Some(number) = app.core.worktree_form.issue_number_at(index) {
        let mapping = app
            .caps
            .settings()
            .map_or_else(micold_core::issue_types::default_mapping, |store| {
                store.load().settings.issue_label_types
            });
        app.core.update(Message::WorktreeForm(FormMsg::IssuePicked {
            number,
            mapping,
        }));
    }
    Task::none()
}

/// A load finished: keep the environment-include snapshot it resolved, then let the reducer decide
/// whether the result is still the awaited one. When it was, and it listed issues, the pass that
/// reads their descriptions starts (feature 038, FR-024).
pub fn on_issues_loaded(
    app: &mut App,
    seq: u64,
    result: Result<(IssueListing, PathBuf), IssueLoadError>,
    resolved_env: Option<(PathBuf, EnvIncludeSnapshot)>,
) -> Task<Message> {
    // Keep an entry already there: a terminal restart or a Settings save may have refreshed it
    // while this load ran, and that snapshot is newer than the one resolved here.
    if let Some((cwd, snapshot)) = resolved_env {
        app.env_include_cache.entry(cwd).or_insert(snapshot);
    }
    let before = app.core.worktree_form.issue_description_request();
    app.core
        .update(Message::WorktreeForm(FormMsg::IssuesLoaded {
            seq,
            result,
            resolved_env: None,
        }));
    match newly_awaited_descriptions(app, before) {
        Some(first) => start_issue_descriptions(app, first),
        None => Task::none(),
    }
}

/// A page of descriptions arrived (feature 038, FR-024). The reducer decides whether it is the
/// awaited one; when it was and the pass awaits another, that page is read next.
pub fn on_issue_descriptions_loaded(
    app: &mut App,
    seq: u64,
    cursor: Option<String>,
    result: Result<DescriptionPage, IssueLoadError>,
) -> Task<Message> {
    let before = app.core.worktree_form.issue_description_request();
    app.core
        .update(Message::WorktreeForm(FormMsg::IssueDescriptionsLoaded {
            seq,
            cursor,
            result,
        }));
    match newly_awaited_descriptions(app, before) {
        Some(next) => start_issue_descriptions(app, next),
        None => Task::none(),
    }
}

/// Up or Down moved the highlight (feature 038, FR-007). The reducer moves the index and nothing
/// else; the list is then scrolled so the row it landed on is wholly in view, which only the
/// laid-out tree can answer now that an issue's row is as tall as its lines.
///
/// The issue list is the only picker that chains this (FR-029): the branch picker's rows and the
/// select's are all one base row tall.
pub fn on_issue_highlight_moved(
    app: &mut App,
    direction: micold_core::typeahead::Direction,
) -> Task<Message> {
    app.core
        .update(Message::WorktreeForm(FormMsg::IssueHighlightMoved(
            direction,
        )));
    micold_client::ui::picker_highlight_into_view()
}

/// The seq the form awaits, if the last message handed out a new one.
fn newly_awaited(app: &App, before: u64) -> Option<u64> {
    app.core
        .worktree_form
        .awaited_issue_load()
        .filter(|seq| *seq > before)
}

/// The page of descriptions the form awaits, if the last message is what made it await it.
///
/// A message the reducer dropped leaves the request as it was, and that request is already
/// running: starting it again would ask GitHub twice for one page.
fn newly_awaited_descriptions(
    app: &App,
    before: Option<DescriptionRequest>,
) -> Option<DescriptionRequest> {
    app.core
        .worktree_form
        .issue_description_request()
        .filter(|request| before.as_ref() != Some(request))
}

/// Locate `gh` and read the open issues on a blocking thread, answering `IssuesLoaded { seq }`.
fn start_issue_load(app: &mut App, seq: u64) -> Task<Message> {
    let Some(repo) = app
        .core
        .worktree_form
        .form
        .as_ref()
        .and_then(|form| form.github_repo().cloned())
    else {
        return Task::none();
    };
    let tooling = app.caps.issue_tooling();
    // The project root: the directory the include script is resolved in for this project.
    let cwd = app
        .core
        .workspace
        .active
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let cached = app.env_include_cache.get(&cwd).cloned();
    let resolver = app.caps.env_include_shared();
    let enabled = app.env_include_enabled;
    let script = app.env_include_script_path.clone();
    let timeout_secs = app.env_include_timeout_secs;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let resolved = match cached {
                    Some(_) => None,
                    None => {
                        let snapshot =
                            resolve_env_include(&*resolver, enabled, &script, timeout_secs, &cwd);
                        Some((cwd, snapshot))
                    }
                };
                let snapshot = cached.as_ref().or(resolved.as_ref().map(|(_, s)| s));
                let include_path = snapshot.and_then(|s| env_include_path(&s.vars));
                let result = match (tooling.locate_gh)(include_path) {
                    None => Err(IssueLoadError::ToolMissing),
                    Some(gh) => {
                        let source = (tooling.source)(gh.clone());
                        load_listing(&*source, &repo).map(|listing| (listing, gh))
                    }
                };
                (result, resolved)
            })
            .await
        },
        move |joined| {
            let (result, resolved_env) = joined.unwrap_or_else(|stopped| {
                (
                    Err(IssueLoadError::Other(format!(
                        "the issue load stopped unexpectedly: {stopped}"
                    ))),
                    None,
                )
            });
            Message::WorktreeForm(FormMsg::IssuesLoaded {
                seq,
                result,
                resolved_env,
            })
        },
    )
}

/// Search GitHub beyond the loaded issues on a blocking thread, answering `IssueSearched { seq }`.
///
/// The source runs the `gh` the load located: a search never looks for `gh` again (research R3).
fn start_issue_search(app: &mut App, seq: u64) -> Task<Message> {
    let Some((repo, text, gh)) = app.core.worktree_form.issue_search_request() else {
        return Task::none();
    };
    let source = app.caps.issue_tooling().source;
    Task::perform(
        async move { tokio::task::spawn_blocking(move || source(gh).search_open(&repo, &text)).await },
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "the issue search stopped unexpectedly: {stopped}"
                )))
            });
            Message::WorktreeForm(FormMsg::IssueSearched { seq, result })
        },
    )
}

/// Read one page of the listed issues' descriptions on a blocking thread, answering
/// `IssueDescriptionsLoaded` under the request's seq and cursor (feature 038, FR-024).
///
/// Like the search, it runs the `gh` the load located and looks for nothing.
fn start_issue_descriptions(app: &mut App, request: DescriptionRequest) -> Task<Message> {
    let describing = app.caps.issue_tooling().source;
    let DescriptionRequest {
        seq,
        repo,
        cursor,
        gh,
    } = request;
    let asked = cursor.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                describing(gh).describe_open(&repo, asked.as_deref())
            })
            .await
        },
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "reading the issue descriptions stopped unexpectedly: {stopped}"
                )))
            });
            Message::WorktreeForm(FormMsg::IssueDescriptionsLoaded {
                seq,
                cursor,
                result,
            })
        },
    )
}

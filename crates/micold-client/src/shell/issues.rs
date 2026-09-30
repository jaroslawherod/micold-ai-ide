//! The GitHub issue source of the add-worktree form: the shell's half (feature 034).
//!
//! The reducer decides everything about the picker (`features/worktree_form.rs`). What is left
//! here is what it cannot do: ask the daemon for the repository's remotes when the form opens, and
//! run the load — locate `gh`, then read the open issues — off the update thread.
//!
//! # When GitHub is contacted, and why only then
//!
//! [`start_issue_load`] is the only path to the issue source, and it has exactly two callers: the
//! reducer accepting `SourceChanged(Issue)`, and the reducer accepting `IssueRetry` from a failed
//! load (FR-003). Opening the form asks the daemon for the remotes, a local read, and nothing
//! else. `tests/issues_are_requested_only_on_named_events.rs` counts the callers.
//!
//! # Why the environment include is resolved inside the load
//!
//! `gh` installed by a version manager is often only on the `PATH` the user's include script
//! builds, so the lookup searches that `PATH` first (research R3). Sourcing the script can take up
//! to its timeout, so a cache miss is resolved on the blocking thread with the rest of the load and
//! handed back in `IssuesLoaded` for the shell to cache, rather than stalling the update thread.

use std::path::PathBuf;
use std::time::Duration;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::worktree_form::{BranchSource, Msg as FormMsg};
use micold_core::env_include::{self, EnvIncludeSnapshot};
use micold_core::github::{env_include_path, load_listing, IssueListing, IssueLoadError};
use micold_core::protocol::messages::ClientMsg;

use crate::shell::daemon_sync::{on_add_worktree_source_changed, send_op, PendingOp};
use crate::App;

/// Why the remotes could not be read while the session service is unreachable.
pub(crate) const NOT_CONNECTED: &str = "not connected to the session service";

/// The form opened: ask the daemon for the active project's remotes (FR-002).
///
/// Not connected, the form says so in its caption. `send_op`'s toast would be raised under the
/// modal's scrim, unseen, and would call a read "a request that may not have taken effect".
pub fn on_form_opened(app: &mut App) -> Task<Message> {
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    let Some(project) = app.core.workspace.active.clone() else {
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

/// Retry a failed load, when the reducer accepted the retry.
pub fn on_issue_retry(app: &mut App) -> Task<Message> {
    let before = app.core.worktree_form.issue_request_seq;
    app.core.update(Message::WorktreeForm(FormMsg::IssueRetry));
    match newly_awaited(app, before) {
        Some(retried) => start_issue_load(app, retried),
        None => Task::none(),
    }
}

/// A row of the shown results was picked: resolve it to its issue while the index and the results
/// are both in hand.
pub fn on_issue_row_picked(app: &mut App, index: usize) -> Task<Message> {
    if let Some(number) = app.core.worktree_form.issue_number_at(index) {
        app.core
            .update(Message::WorktreeForm(FormMsg::IssuePicked { number }));
    }
    Task::none()
}

/// A load finished: keep the environment-include snapshot it resolved, then let the reducer decide
/// whether the result is still the awaited one.
pub fn on_issues_loaded(
    app: &mut App,
    seq: u64,
    result: Result<(IssueListing, PathBuf), IssueLoadError>,
    resolved_env: Option<(PathBuf, EnvIncludeSnapshot)>,
) -> Task<Message> {
    if let Some((cwd, snapshot)) = resolved_env {
        app.env_include_cache.insert(cwd, snapshot);
    }
    app.core
        .update(Message::WorktreeForm(FormMsg::IssuesLoaded {
            seq,
            result,
            resolved_env: None,
        }));
    Task::none()
}

/// The seq the form awaits, if the last message handed out a new one.
fn newly_awaited(app: &App, before: u64) -> Option<u64> {
    app.core
        .worktree_form
        .awaited_issue_load()
        .filter(|seq| *seq > before)
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
    let timeout = Duration::from_secs(app.env_include_timeout_secs);
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let resolved = match cached {
                    Some(_) => None,
                    None => {
                        let snapshot =
                            env_include::snapshot_for(&*resolver, enabled, &script, timeout, &cwd);
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

//! The GitHub filters of the add-worktree form: the shell's half (feature 1400,
//! contracts/source-and-storage.md §3, §4).
//!
//! The reducer (`features/worktree_form/github_filter.rs`) decides everything about the filters.
//! What is left here is what it cannot do: run the filtered listing, the filtered search beyond it
//! and the label choices, each on a blocking thread.
//!
//! # When GitHub is asked, and why only then
//!
//! The filtered listing runs where 034's listing runs (`shell::issues::start_issue_load` branches
//! here while the form's filter is not the default), and when the reducer accepted a filter change
//! and handed out a new issue seq (FR-008). The filtered search runs where 034's search runs
//! (`start_issue_search` branches here). The label choices run when the reducer accepted the first
//! opening of a label field, or a **Retry** of a failed request, and handed out a new filter seq
//! (FR-005). Nothing else here sends anything: not opening the form, not a redraw.
//!
//! # What the project remembers
//!
//! The form opens with the project's remembered selector (`shell::issues::remembered_selector`),
//! read from `<data_dir>/github/<project id>.selector.json` at each open and nowhere else, whatever
//! the GitHub availability, so another window's later write leaves an open form alone. Each basic
//! filter change hands back `Outcome::GithubSelectorRemembered`, which
//! `shell::issues::remember_github_selector` writes as a read-modify-write of that one field
//! (FR-021). Forgetting the project deletes the file (`forget_github_selector` there, FR-022).
//! Those live in `shell::issues`, beside 1186's `remember_jira_query`, because their failure is a
//! notification and this file notifies nothing (below).
//!
//! Nothing here logs or notifies a filter value, the scoped search text or a typed label (FR-024):
//! `tests/issues_are_requested_only_on_named_events.rs` scans this file for it.

use std::path::PathBuf;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::worktree_form::github_filter::Msg as GithubMsg;
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_core::github::{GithubRepo, IssueLoadError};
use micold_core::github_filter::{load_filtered, load_label_choices, FilterFailure, ScopedSearch};

use crate::shell::issues::{remember_github_selector, search_after_pause, start_issue_load, GhJob};
use crate::App;

/// A filter message: the reducer takes it, then the listing, the label choices or the search it
/// newly awaits starts.
pub fn on_filter(app: &mut App, msg: GithubMsg) -> Task<Message> {
    let issue_before = app.core.worktree_form.issue_request_seq;
    let choice_before = app.core.worktree_form.filter_request_seq;
    let (msg, loaded) = match msg {
        GithubMsg::Loaded {
            seq,
            result,
            resolved_env,
        } => {
            // Keep an entry already there, as 034's load does: it may be newer than this one.
            if let Some((cwd, snapshot)) = resolved_env {
                app.env_include_cache.entry(cwd).or_insert(snapshot);
            }
            let msg = GithubMsg::Loaded {
                seq,
                result,
                resolved_env: None,
            };
            (msg, Some(seq))
        }
        other => (other, None),
    };
    let remembered = app
        .core
        .update_worktree_form_for_effects(FormMsg::GithubFilter(msg));
    remember_github_selector(app, remembered);
    let mut work = Vec::new();
    if let Some(seq) = app
        .core
        .worktree_form
        .awaited_issue_load()
        .filter(|seq| *seq > issue_before)
    {
        work.push(start_issue_load(app, seq));
    }
    if let Some(seq) = app
        .core
        .worktree_form
        .awaited_label_choices()
        .filter(|seq| *seq > choice_before)
    {
        work.push(start_label_choices(app, seq));
    }
    // A filtered list reloaded with text typed and more to find searches beyond it again; the
    // reducer left that search pending under the load's seq (FR-016).
    if let Some(seq) =
        loaded.filter(|seq| app.core.worktree_form.pending_issue_search() == Some(*seq))
    {
        work.push(search_after_pause(seq));
    }
    Task::batch(work)
}

/// Locate `gh` and read the issues `search` selects, on a blocking thread, answering
/// `GithubFilter(Loaded { seq })`.
pub(crate) fn start_filtered_load(
    app: &mut App,
    seq: u64,
    repo: GithubRepo,
    search: ScopedSearch,
) -> Task<Message> {
    Task::perform(
        GhJob::of(app).run(move |located| match located {
            None => Err(FilterFailure::Load(IssueLoadError::ToolMissing)),
            Some((source, gh)) => load_filtered(&*source, &repo, &search).map(|l| (l, gh)),
        }),
        move |joined| {
            let (result, resolved_env) = joined.unwrap_or_else(|stopped| {
                (
                    Err(FilterFailure::Load(IssueLoadError::Other(format!(
                        "the issue load stopped unexpectedly: {stopped}"
                    )))),
                    None,
                )
            });
            Message::WorktreeForm(FormMsg::GithubFilter(GithubMsg::Loaded {
                seq,
                result,
                resolved_env,
            }))
        },
    )
}

/// Search GitHub beyond the filtered list with `search` (the filter and the typed words), on a
/// blocking thread, answering `GithubFilter(Searched { seq })`. It runs the `gh` the load located,
/// as 034's search does.
pub(crate) fn start_filtered_search(
    app: &mut App,
    seq: u64,
    repo: GithubRepo,
    search: ScopedSearch,
    gh: PathBuf,
) -> Task<Message> {
    let source = app.caps.issue_tooling().source;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || source(gh).search_filtered(&repo, &search)).await
        },
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(FilterFailure::Load(IssueLoadError::Other(format!(
                    "the issue search stopped unexpectedly: {stopped}"
                ))))
            });
            Message::WorktreeForm(FormMsg::GithubFilter(GithubMsg::Searched { seq, result }))
        },
    )
}

/// Locate `gh` and read the repository's labels, up to ten pages, on a blocking thread, answering
/// `GithubFilter(LabelsLoaded { seq })`. Sends the repository's owner and name only (FR-023).
fn start_label_choices(app: &mut App, seq: u64) -> Task<Message> {
    let Some(repo) = app
        .core
        .worktree_form
        .form
        .as_ref()
        .and_then(|form| form.github_repo().cloned())
    else {
        return Task::none();
    };
    Task::perform(
        GhJob::of(app).run(move |located| match located {
            None => Err(IssueLoadError::ToolMissing),
            Some((source, _)) => load_label_choices(&*source, &repo),
        }),
        move |joined| {
            let result = joined.map(|(result, _)| result).unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "the label request stopped unexpectedly: {stopped}"
                )))
            });
            Message::WorktreeForm(FormMsg::GithubFilter(GithubMsg::LabelsLoaded {
                seq,
                result,
            }))
        },
    )
}

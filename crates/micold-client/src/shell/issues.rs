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
use micold_client::features::worktree_form::{
    BranchSource, DescriptionRequest, DetailsRequest, JiraAvailability, Msg as FormMsg,
};
use micold_core::env_include::EnvIncludeSnapshot;
use micold_core::github::{
    env_include_path, load_listing, DescriptionPage, IssueListing, IssueLoadError,
};
use micold_core::issue_details::DETAILS_BOUND;
use micold_core::jira::{connection_key, ApiToken, IssueProvider};
use micold_core::protocol::messages::ClientMsg;
use micold_core::store::jira_token_path;

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
    if app.active_outbox().is_none() {
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

/// The GitHub / Jira choice changed (feature 533, FR-016, FR-017). When the reducer accepted it, the
/// choice is written to the project's state file and, with the Issue source showing, the new
/// provider's load starts.
pub fn on_issue_provider_changed(app: &mut App, provider: IssueProvider) -> Task<Message> {
    let before = app.core.worktree_form.issue_request_seq;
    let project = app.core.workspace.active.clone();
    let previous = project
        .as_ref()
        .and_then(|project| app.core.workspace.issue_sources.get(project))
        .copied();
    app.core
        .update(Message::WorktreeForm(FormMsg::IssueProviderChanged(
            provider,
        )));
    if app.core.worktree_form.issue_request_seq == before {
        return Task::none();
    }
    if let (Some(project), Some(store)) = (project, app.caps.projects()) {
        if previous != Some(provider) {
            // A project with no state file yet cannot keep it on disk (the store refuses to create
            // one); the choice still holds for this run, so that is not worth a toast.
            if let Err(err) =
                store
                    .save_issue_source(&project, provider)
                    .or_else(|err| match err.kind() {
                        std::io::ErrorKind::NotFound => Ok(()),
                        _ => Err(err),
                    })
            {
                app.core
                    .notify_error(format!("Couldn't remember the issue source: {err}"));
            }
        }
    }
    match newly_awaited(app, before) {
        Some(chosen) => start_issue_load(app, chosen),
        None => Task::none(),
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
        return on_issue_picked(app, number, mapping);
    }
    Task::none()
}

/// An issue was picked: the reducer takes the pick, and the read of its details starts when the pick
/// is what made the form await them (837, form-details §2). Create never waits for it (FR-006).
///
/// The pick is the only message that starts a read: leaving the source, a provider change and a
/// closed form all clear the pick, which leaves the form awaiting nothing.
pub fn on_issue_picked(
    app: &mut App,
    number: u64,
    mapping: Vec<micold_core::issue_types::LabelTypeEntry>,
) -> Task<Message> {
    let before = app.core.worktree_form.issue_details_request();
    app.core.update(Message::WorktreeForm(FormMsg::IssuePicked {
        number,
        mapping,
    }));
    match app
        .core
        .worktree_form
        .issue_details_request()
        .filter(|request| before.as_ref() != Some(request))
    {
        Some(request) => start_issue_details(app, request),
        None => Task::none(),
    }
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
    update_then_follow(
        app,
        FormMsg::IssuesLoaded {
            seq,
            result,
            resolved_env: None,
        },
    )
}

/// Feed the form one message, then start the description read it newly awaits, if any.
fn update_then_follow(app: &mut App, msg: FormMsg) -> Task<Message> {
    let before = app.core.worktree_form.issue_description_request();
    app.core.update(Message::WorktreeForm(msg));
    match newly_awaited_descriptions(app, before) {
        Some(next) => start_issue_descriptions(app, next),
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
    update_then_follow(
        app,
        FormMsg::IssueDescriptionsLoaded {
            seq,
            cursor,
            result,
        },
    )
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

/// The settings now in force changed what Jira makes of the open form (feature 533, FR-005): tell
/// the form, which drops a Jira list and discards a load in flight when the switch went off or the
/// connection went away.
pub fn sync_jira_availability(app: &mut App) {
    if app.core.worktree_form.form.is_none() {
        return;
    }
    let jira = JiraAvailability::from_settings(
        app.core.settings.jira_enabled,
        &app.core.settings.jira_connections,
        app.core.workspace.active.as_deref(),
    );
    app.core
        .update(Message::WorktreeForm(FormMsg::JiraSettingsChanged(jira)));
}

/// The GitHub issues switch in force changed (feature 1102, FR-005): tell the open form, which
/// drops a GitHub list and discards a load in flight when it went off, and offers GitHub again
/// when it went on.
pub fn sync_github_issues_availability(app: &mut App) {
    if app.core.worktree_form.form.is_none() {
        return;
    }
    let enabled = app.core.settings.github_issues_enabled;
    app.core
        .update(Message::WorktreeForm(FormMsg::GithubIssuesChanged(enabled)));
}

/// Why a Jira request asked nothing: no token file on this host (plan risks, multi-daemon).
const NO_TOKEN: &str = "no API token is stored on this computer for this project: enter your token in Settings > Jira.";

/// What a Jira request needs, taken only once the integration's switch and the project's
/// connection allow one (FR-001, FR-020): the one gate every Jira request passes.
pub(crate) struct JiraRequest {
    pub(crate) project: PathBuf,
    pub(crate) connection: micold_core::jira::JiraConnection,
    dir: Option<PathBuf>,
    source: Option<std::sync::Arc<dyn micold_core::jira::JiraSource>>,
}

/// The gate: `Err` is the reason nothing is asked.
pub(crate) fn jira_request(app: &mut App) -> Result<JiraRequest, &'static str> {
    let Some(project) = app.core.workspace.active.clone() else {
        return Err("no project is open.");
    };
    let connection = app
        .core
        .settings
        .jira_connections
        .get(&connection_key(&project))
        .cloned();
    let (true, Some(connection)) = (app.core.settings.jira_enabled, connection) else {
        return Err("Jira isn't connected for this project: check Settings > Jira.");
    };
    let dir = app.caps.jira_token_dir().map(std::path::Path::to_path_buf);
    let source = app.caps.jira_source();
    Ok(JiraRequest {
        project,
        connection,
        dir,
        source,
    })
}

impl JiraRequest {
    /// Run `ask` on a blocking thread with the source and the token read from its file, which is
    /// done only here, inside the request's thread.
    pub(crate) async fn run<T: Send + 'static>(
        self,
        ask: impl FnOnce(
                &dyn micold_core::jira::JiraSource,
                &micold_core::jira::JiraConnection,
                &ApiToken,
            ) -> Result<T, IssueLoadError>
            + Send
            + 'static,
    ) -> Result<Result<T, IssueLoadError>, tokio::task::JoinError> {
        tokio::task::spawn_blocking(move || {
            let Some(source) = self.source else {
                return Err(IssueLoadError::Other(
                    "Jira isn't available in this build.".to_string(),
                ));
            };
            let token = self
                .dir
                .and_then(|dir| std::fs::read_to_string(jira_token_path(&dir, &self.project)).ok())
                .and_then(|text| ApiToken::parse(text.trim()).ok())
                .ok_or_else(|| IssueLoadError::Other(NO_TOKEN.to_string()))?;
            ask(&*source, &self.connection, &token)
        })
        .await
    }
}

/// Read the connected project's open Jira issues on a blocking thread, answering
/// `IssuesLoaded { seq }`.
fn start_jira_load(app: &mut App, seq: u64) -> Task<Message> {
    let request = match jira_request(app) {
        Ok(request) => request,
        Err(why) => {
            return Task::done(Message::WorktreeForm(FormMsg::IssuesLoaded {
                seq,
                result: Err(IssueLoadError::Other(why.to_string())),
                resolved_env: None,
            }))
        }
    };
    Task::perform(
        request.run(|source, connection, token| {
            micold_core::jira::load(source, connection, token)
                .map(|listing| (listing, PathBuf::new()))
        }),
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "the issue load stopped unexpectedly: {stopped}"
                )))
            });
            Message::WorktreeForm(FormMsg::IssuesLoaded {
                seq,
                result,
                resolved_env: None,
            })
        },
    )
}

/// Search the connected Jira project beyond the loaded issues on a blocking thread, answering
/// `IssueSearched { seq }` (FR-012). Behind the same gate as the load.
fn start_jira_search(app: &mut App, seq: u64, text: String) -> Task<Message> {
    let request = match jira_request(app) {
        Ok(request) => request,
        Err(why) => {
            return Task::done(Message::WorktreeForm(FormMsg::IssueSearched {
                seq,
                result: Err(IssueLoadError::Other(why.to_string())),
            }))
        }
    };
    Task::perform(
        request.run(move |source, connection, token| source.search_open(connection, token, &text)),
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

/// Locate `gh` and read the open issues on a blocking thread, answering `IssuesLoaded { seq }`.
fn start_issue_load(app: &mut App, seq: u64) -> Task<Message> {
    if app
        .core
        .worktree_form
        .form
        .as_ref()
        .is_some_and(|form| form.jira_site().is_some())
    {
        return start_jira_load(app, seq);
    }
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
    if let Some(text) = app.core.worktree_form.jira_search_text() {
        return start_jira_search(app, seq, text);
    }
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

/// Read the picked issue's details on a blocking thread, answering `IssueDetailsLoaded` under the
/// request's seq and number (837). Like the search, it runs the `gh` the load located, and it is
/// bound: the source is handed a deadline [`DETAILS_BOUND`] from now (FR-006, SC-005).
fn start_issue_details(app: &mut App, request: DetailsRequest) -> Task<Message> {
    let source = app.caps.issue_tooling().source;
    let DetailsRequest {
        seq,
        number,
        repo,
        gh,
    } = request;
    let deadline = std::time::Instant::now() + DETAILS_BOUND;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                source(gh)
                    .read_details(&repo, number, deadline)
                    .map(Box::new)
            })
            .await
        },
        move |joined| {
            let result = joined.unwrap_or_else(|stopped| {
                Err(IssueLoadError::Other(format!(
                    "reading the issue's details stopped unexpectedly: {stopped}"
                )))
            });
            Message::WorktreeForm(FormMsg::IssueDetailsLoaded {
                seq,
                number: u64::from(number),
                result,
            })
        },
    )
}

// ---- The Overview's issue titles (feature 813, research R8) ----

/// The Overview opened: read the title of every worktree's issue ([`request_titles`]).
pub fn on_overview_opened(app: &mut App) -> Task<Message> {
    let dirs = worktree_dirs(app);
    // The issue links need the repository even when no title is read (feature 1122, R2).
    if !request_titles(app, &dirs) {
        read_repo(app);
    }
    Task::none()
}

/// The worktree list changed, or titles became readable (feature 1125, FR-011): a sidebar row's
/// first line is its issue's title, so every worktree's title is read as the Overview would read
/// it, without the Overview having opened. Each title is read once ([`request_titles`]).
pub fn on_worktrees_changed(app: &mut App) {
    let dirs = worktree_dirs(app);
    request_titles(app, &dirs);
}

/// A fingerprint of the active project's worktree list, by directory name, so the shell can tell
/// that a message changed it without keeping a copy.
pub fn worktrees_fingerprint(app: &App) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for worktree in &app.core.worktree.worktrees {
        worktree.dir_name.hash(&mut hasher);
    }
    hasher.finish()
}

fn worktree_dirs(app: &App) -> Vec<String> {
    app.core
        .worktree
        .worktrees
        .iter()
        .map(|worktree| worktree.dir_name.clone())
        .collect()
}

/// The session area's header followed the worktree `dir` (feature 1126, R3): read its issue's
/// title, as an Overview card would, so the header shows it without the Overview having opened.
pub fn on_header_followed(app: &mut App, dir: &str) {
    request_titles(app, &[dir.to_string()]);
}

/// Seed titles of `dirs`' issues from the issues the New-worktree form already holds, then read
/// each numeric ticket still without an entry, once. The remotes come first (the repository is
/// whatever the daemon says); a window with no route to a daemon reads nothing (FR-019). Whether a
/// read was sent.
fn request_titles(app: &mut App, dirs: &[String]) -> bool {
    use micold_client::features::overview::{numeric_ticket, titles_to_fetch, Msg, TitleLookup};
    let seeds: Vec<(u64, String)> = dirs
        .iter()
        .filter_map(|dir| numeric_ticket(dir))
        .filter(|number| !app.core.overview.titles.contains_key(number))
        .filter_map(|number| {
            let held = app.core.worktree_form.form.as_ref()?.issues.held();
            let issue = held.into_iter().find(|issue| issue.number() == number)?;
            Some((number, issue.title().to_string()))
        })
        .collect();
    if let Some(project) = app.core.workspace.active.clone() {
        for (number, title) in seeds {
            app.core.update(Message::Overview(Msg::TitleRead {
                project: project.clone(),
                number,
                result: TitleLookup::Known(title),
            }));
        }
    }
    // Feature 1102: with the switch off no title is read, and none is recorded as unavailable, so
    // turning it back on reads them on the next open. The repository is read all the same: the
    // issue links need it, and remotes are a git read, not a GitHub one (feature 1122, R2).
    let routed = has_route(app);
    let wanted = if app.core.settings.github_issues_enabled {
        titles_to_fetch(&app.core.overview, dirs, routed)
    } else {
        Vec::new()
    };
    if wanted.is_empty() {
        return false;
    }
    let Some(project) = app.core.workspace.active.clone() else {
        return false;
    };
    // `wanted` is empty unless `routed` held just above (a route to a daemon with an outbox), which
    // is exactly what `send_op` needs, so the request below is always sent and marking the titles
    // pending first cannot strand them. A daemon lost afterwards resolves the op as unknown
    // (`on_title_remotes(None)`), which settles them as unavailable. Its answer also records the
    // repository, so no second read is asked for.
    app.core
        .update(Message::Overview(Msg::TitlesRequested(wanted.clone())));
    send_remotes_read(app, project, wanted);
    true
}

/// Whether an issue title can be read now: the GitHub issues switch is on and the window has a
/// route to a daemon (feature 1126: the header asks again when this becomes true).
pub fn titles_readable(app: &App) -> bool {
    app.core.settings.github_issues_enabled && has_route(app)
}

/// Whether the window has a route to a daemon with an outbox, which [`send_op`] needs.
fn has_route(app: &App) -> bool {
    let binding = app.active_binding();
    matches!(
        app.route(binding),
        micold_client::links::Route::To(id) if app.outbox_for(id).is_some()
    )
}

/// Ask the daemon for `project`'s remotes on behalf of the Overview: the titles `numbers` and,
/// while it is unknown, the repository the issue links need (feature 1122, R2).
fn send_remotes_read(app: &mut App, project: PathBuf, numbers: Vec<u64>) {
    use micold_client::features::overview::RepoLookup;
    if app.core.overview.repo == RepoLookup::Unknown {
        app.core.overview.repo = RepoLookup::Asked;
    }
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::TitleRemotes {
            project: asked_for,
            numbers,
        },
        move |req| ClientMsg::RemoteList { req, project },
    );
}

/// Read the active project's GitHub repository while the Overview does not know it (feature 1122,
/// R2): the titles' remotes request with no title numbers, so its answer records the repository
/// and reads nothing. Called when the Overview opens and when the active project's daemon answers
/// its attach (`Attached`, or `Refused` in a read-only window) with the Overview open (a project
/// switch made there). A window with no route asks nothing.
pub fn read_repo(app: &mut App) {
    use micold_client::features::overview::RepoLookup;
    if app.core.overview.repo != RepoLookup::Unknown || !has_route(app) {
        return;
    }
    if let Some(project) = app.core.workspace.active.clone() {
        send_remotes_read(app, project, Vec::new());
    }
}

/// The daemon answered the titles' `RemoteList` (`None`: it failed). Reads each title on a blocking
/// thread when the repository is on GitHub and `gh` is found; every other end leaves the branch as
/// the summary.
pub fn on_title_remotes(
    app: &mut App,
    project: PathBuf,
    numbers: Vec<u64>,
    remotes: Option<Vec<micold_core::git::GitRemote>>,
) -> Task<Message> {
    use micold_client::features::overview::{Msg, TitleLookup};
    let started_in = project.clone();
    let unavailable = |numbers: Vec<u64>| {
        Task::batch(numbers.into_iter().map(|number| {
            Task::done(Message::Overview(Msg::TitleRead {
                project: started_in.clone(),
                number,
                result: TitleLookup::Unavailable,
            }))
        }))
    };
    // An answer for a project no longer active: that project's titles were cleared on the switch.
    if app.core.workspace.active.as_deref() != Some(project.as_path()) {
        return Task::none();
    }
    // Feature 1122 (R2): the repository the issue links need, recorded before the switch's gate.
    let choice = remotes.as_deref().map(micold_core::github::choose_remote);
    match &choice {
        Some(choice) => {
            let repo = match choice {
                micold_core::github::RemoteChoice::Github { repo, .. } => Some(repo.clone()),
                micold_core::github::RemoteChoice::NoGithubRemote => None,
            };
            app.core.update(Message::Overview(Msg::RepoRead {
                project: project.clone(),
                repo,
            }));
        }
        // Not read: asked again on the next open or attach.
        None => {
            if app.core.overview.repo == micold_client::features::overview::RepoLookup::Asked {
                app.core.overview.repo = micold_client::features::overview::RepoLookup::Unknown;
            }
        }
    }
    if numbers.is_empty() {
        return Task::none();
    }
    // Feature 1102: an issue title is a GitHub issue read, so the switch gates it too.
    let Some(micold_core::github::RemoteChoice::Github { repo, .. }) =
        choice.filter(|_| app.core.settings.github_issues_enabled)
    else {
        return unavailable(numbers);
    };
    let tooling = app.caps.issue_tooling();
    let cached = app.env_include_cache.get(&project).cloned();
    let resolver = app.caps.env_include_shared();
    let enabled = app.env_include_enabled;
    let script = app.env_include_script_path.clone();
    let timeout_secs = app.env_include_timeout_secs;
    let asked = numbers.clone();
    let started_in = project.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let snapshot = cached.unwrap_or_else(|| {
                    resolve_env_include(&*resolver, enabled, &script, timeout_secs, &project)
                });
                let gh = (tooling.locate_gh)(env_include_path(&snapshot.vars))?;
                let gh = (tooling.source)(gh);
                Some(
                    numbers
                        .into_iter()
                        .map(|number| (number, read_title(&*gh, &repo, number)))
                        .collect::<Vec<_>>(),
                )
            })
            .await
            .ok()
            .flatten()
        },
        move |read| {
            read.unwrap_or_else(|| {
                asked
                    .iter()
                    .map(|number| (*number, TitleLookup::Unavailable))
                    .collect()
            })
        },
    )
    .then(move |read| {
        Task::batch(read.into_iter().map(|(number, result)| {
            Task::done(Message::Overview(Msg::TitleRead {
                project: started_in.clone(),
                number,
                result,
            }))
        }))
    })
}

/// One title through `gh`. A failure is logged by number alone and leaves the branch (FR-020).
fn read_title(
    gh: &(dyn micold_core::github::IssueSource + Send + Sync),
    repo: &micold_core::github::GithubRepo,
    number: u64,
) -> micold_client::features::overview::TitleLookup {
    use micold_client::features::overview::TitleLookup;
    let Ok(small) = u32::try_from(number) else {
        return TitleLookup::Unavailable;
    };
    match gh.read_issue(repo, small) {
        Ok(issue) => TitleLookup::Known(issue.title),
        Err(error) => {
            eprintln!("{}", title_failure_line(number, &error));
            TitleLookup::Unavailable
        }
    }
}

/// The log line for a title that could not be read. It names the issue number and a fixed reason,
/// never the error's own text, which can quote the issue (FR-020).
fn title_failure_line(number: u64, error: &micold_core::github::IssueLookupError) -> String {
    use micold_core::github::IssueLookupError;
    let reason = match error {
        IssueLookupError::NotOpenIssue { .. } => "it is not an open issue",
        IssueLookupError::Load(_) => "the read failed",
    };
    format!("micold: no title for issue #{number} on the Overview: {reason}")
}

#[cfg(test)]
mod title_tests {
    use super::*;
    use micold_core::github::{IssueLoadError, IssueLookupError};

    /// FR-020: the line names the number, and nothing the error carries.
    #[test]
    fn the_failure_line_names_the_number_and_never_a_title() {
        let quoted = "Fix the SECRET-TITLE crash";
        for error in [
            IssueLookupError::Load(IssueLoadError::Other(quoted.into())),
            IssueLookupError::Load(IssueLoadError::Unreachable(quoted.into())),
            IssueLookupError::NotOpenIssue { closed: true },
        ] {
            let line = title_failure_line(813, &error);
            assert!(line.contains("#813"), "{line}");
            assert!(!line.contains("SECRET"), "{line}");
        }
    }
}

//! The Changes view's shell half (feature 482, T021, T036, research R2): runs the list and diff
//! reads that `features::changes` asks for off the update thread, as `shell/pr_status.rs` runs its
//! reading, so a large diff never blocks input or redraws (FR-009).
//!
//! The reducer decides when a read starts and drops an answer that is no longer current; this
//! module only resolves the entry's directory, calls git in `spawn_blocking` (highlighting the
//! diff there too), sends the layout the user chose to the service, and answers
//! [`Msg::ListRead`] or [`Msg::DiffRead`] with the read's own `seq`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::text_editor;
use iced::Task;
use micold_client::app::Message;
use micold_client::features::changes::Effect;
use micold_client::features::changes::Msg;
use micold_client::features::changes::SourceKey;
use micold_client::features::settings;
use micold_client::ui::syntax;
use micold_core::context_header::ChangeSummary;
use micold_core::git::Git;
use micold_core::github::GithubRepo;
use micold_core::pull_request::{PullRequestSource, ReadingFailure, TreeListing};
use micold_core::review::base::{ReviewScope, Toggles};
use micold_core::review::browse::{self, Confined, FileReadFailure, FileSource, FileText};
use micold_core::review::changes::ChangeList;
use micold_core::review::diff::Spans;
use micold_core::review::RelPath;
use micold_core::session::SessionLocation;

use crate::shell::daemon_sync::PendingOp;
use crate::App;

/// Apply a Changes view message, and run the read the reducer asks for.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    if let Msg::ComposerAction(action) = msg {
        return composer_action(app, action.0);
    }
    // The reducer drops an answer whose read is no longer the current one, silently: say so in the
    // log, or a diff that never lands cannot be told from one that was dropped (#771).
    let stale = match &msg {
        Msg::DiffRead { seq, .. } => Some((*seq, !diff_read_is_current(app, *seq))),
        _ => None,
    };
    if let Some((seq, true)) = stale {
        crate::log_line(&format!(
            "changes: diff seq={seq} answered late, dropped as stale"
        ));
    }
    // GitHub's request limit named by a list read holds the status reading back too (R3).
    if let Msg::PullRequestsRead {
        result: Err(ReadingFailure::RateLimited { until }),
        ..
    } = &msg
    {
        let held = &mut app.core.pr_status.pause_until;
        *held = Some(held.map_or(*until, |current| current.max(*until)));
    }
    // The address of a too-large pull request is opened as the sidebar's is (FR-014).
    if let Msg::PullRequestOpenRequested(number) = &msg {
        return match pull_request_address(app, *number) {
            Some(address) => crate::shell::links::perform(
                app,
                micold_client::features::OpenRequest::Url(address),
            ),
            None => Task::none(),
        };
    }
    let effect = app.core.update_changes(msg);
    sync_composer(app);
    run(app, effect)
}

/// The address of pull request `number` of the open Changes view, when it is a GitHub one
/// (FR-014): the held address, never one built from the number.
pub fn pull_request_address(app: &App, number: u64) -> Option<String> {
    let view = app.core.changes.open.as_ref()?;
    let (prs, _) = view.pull_requests.shown()?;
    let address = &prs.iter().find(|pr| pr.number == number)?.url;
    address
        .starts_with(micold_client::features::sidebar::GITHUB_ADDRESS_PREFIX)
        .then(|| address.clone())
}

/// Whether the open view is waiting for diff read `seq`.
fn diff_read_is_current(app: &App, seq: u64) -> bool {
    app.core.changes.open.as_ref().is_some_and(|view| {
        matches!(view.diff, micold_client::features::changes::Load::Loading { seq: s, .. } if s == seq)
    })
}

/// Perform an action in the composer's editor; an edit reports the new text to the reducer (C2).
fn composer_action(app: &mut App, action: text_editor::Action) -> Task<Message> {
    let edit = action.is_edit();
    app.composer.perform(action);
    if edit {
        update(app, Msg::ComposerEdited(app.composer.text()))
    } else {
        Task::none()
    }
}

/// Keep the editor's text the reducer's: a composer that opened (Edit fills it with the comment,
/// cursor at the end), closed or was saved starts the editor over; typing leaves it alone.
fn sync_composer(app: &mut App) {
    let text = app
        .core
        .changes
        .open
        .as_ref()
        .and_then(|view| view.composer.as_ref())
        .map_or("", |composer| composer.text.as_str());
    if app.composer.text() != text {
        app.composer = text_editor::Content::with_text(text);
        app.composer
            .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
}

/// Run the counts read the header's follow left pending (feature 1126, R6), when the message
/// that moved the header did not already run it.
pub fn run_followed(app: &mut App) -> Task<Message> {
    match app.core.take_changes_effect() {
        Some(effect) => run(app, effect),
        None => Task::none(),
    }
}

/// Run the read the root left pending while it opened the view (V1).
pub fn run_pending(app: &mut App) -> Task<Message> {
    match app.core.take_changes_effect() {
        Some(effect) => {
            let list = run(app, effect);
            // The pull requests are asked for apart from the list: the list renders without them
            // (feature 775, FR-009).
            let branch = entry_branch(app);
            let pull_requests = update(app, Msg::PullRequestsWanted { branch });
            Task::batch([list, pull_requests])
        }
        None => Task::none(),
    }
}

/// The branch of the open view's entry: a listed worktree's own; none for the project root.
fn entry_branch(app: &App) -> Option<String> {
    let SessionLocation::Worktree(dir_name) = &app.core.changes.open.as_ref()?.entry else {
        return None;
    };
    app.core
        .worktree
        .worktrees
        .iter()
        .find(|w| &w.dir_name == dir_name)
        .and_then(|w| w.branch.clone())
}

/// Why no read can run here: the error each read answers with.
const NO_PROJECT: &str = "No project is open";
const NOT_LOCAL: &str =
    "This computer cannot read the worktree's files: the session service runs elsewhere";

/// Start `effect`'s read, answering `Msg::ListRead` or `Msg::DiffRead` when git is done.
fn run(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::None => Task::none(),
        Effect::Batch(effects) => Task::batch(effects.into_iter().map(|effect| run(app, effect))),
        Effect::ReadPullRequests {
            seq,
            project,
            entry,
            current,
        } => crate::shell::pr_status::list_requested(app, seq, project, entry, current),
        Effect::ReadPullRequestDiff {
            seq,
            project,
            number,
        } => crate::shell::pr_status::diff_requested(app, seq, project, number),
        Effect::ScrollDiff { offset } => iced::widget::operation::scroll_to(
            micold_client::ui::DIFF_SCROLL_ID.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: offset as f32,
            },
        ),
        Effect::SetLayout(layout) => {
            // Service-owned (R12): the daemon stores it and echoes `SettingsChanged` to every
            // window. Disconnected, the choice holds for this run only, as the reducer set it.
            if let Some(daemon) = app.active_outbox() {
                let req = app.next_req;
                app.next_req += 1;
                daemon.send(settings::diff_layout_set(req, layout));
                app.pending_ops.insert(req, PendingOp::SettingsSet);
                if let Some(id) = app.active_daemon() {
                    app.pending_daemon.insert(req, id);
                }
            }
            Task::none()
        }
        Effect::ReviewEdit {
            project,
            worktree_dir,
            edit,
        } => {
            // Service-owned (FR-020): the daemon stores it and pushes `ReviewChanged` to every
            // window, this one included; a refusal surfaces through the pending op.
            crate::shell::daemon_sync::send_op(app, PendingOp::ReviewEdit, |req| {
                micold_core::protocol::messages::ClientMsg::ReviewEdit {
                    req,
                    project,
                    worktree_dir,
                    edit,
                }
            });
            Task::none()
        }
        Effect::ReviewSend {
            project,
            worktree_dir,
            outdated,
            count,
            pr,
        } => {
            // W6: the service pushes `sending` to every window, then answers once the prompt was
            // typed or refused (S2).
            crate::shell::daemon_sync::send_op(app, PendingOp::ReviewSend { count }, |req| {
                micold_core::protocol::messages::ClientMsg::ReviewSend {
                    req,
                    project,
                    worktree_dir,
                    outdated,
                    pr,
                }
            });
            Task::none()
        }
        Effect::ReadCounts { seq, entry } => {
            // Feature 1126 R5: the view's own reader under the toggles it opens with, so the
            // header counts what the view would list; reduced off the update thread too.
            let answer = move |result| Message::Changes(Msg::HeaderCountsRead { seq, result });
            let (dir, git) = match local(app, &entry) {
                Ok(found) => found,
                Err(why) => return Task::done(answer(Err(why.into()))),
            };
            let label = format!("counts seq={seq} entry={entry:?}");
            off_thread(
                label,
                move || {
                    read_list(&*git, &dir, &entry, Toggles::default())
                        .map(|list| ChangeSummary::of(&list))
                },
                answer,
            )
        }
        Effect::ReadList {
            seq,
            entry,
            toggles,
        } => {
            let answer = move |result| Message::Changes(Msg::ListRead { seq, result });
            let (dir, git) = match local(app, &entry) {
                Ok(found) => found,
                Err(why) => return Task::done(answer(Err(why.into()))),
            };
            let label = format!("list seq={seq} entry={entry:?}");
            off_thread(
                label,
                move || read_list(&*git, &dir, &entry, toggles),
                answer,
            )
        }
        Effect::ReadTree {
            seq,
            project,
            entry,
            source,
            commit,
            address,
        } => read_tree_requested(app, seq, project, entry, source, commit, address),
        Effect::ReadFile {
            seq,
            project,
            entry,
            source,
            commit,
            address,
            path,
            force_large,
        } => read_file_requested(
            app,
            seq,
            project,
            entry,
            source,
            commit,
            address,
            path,
            force_large,
        ),
        Effect::ReadDiff {
            seq,
            entry,
            scope,
            toggles,
            path,
            from,
            force_large,
        } => {
            let answer = move |result| Message::Changes(Msg::DiffRead { seq, result });
            let (dir, git) = match local(app, &entry) {
                Ok(found) => found,
                Err(why) => return Task::done(answer(Err(why.into()))),
            };
            let label = format!("diff seq={seq} path={path} scope={scope:?}");
            off_thread(
                label,
                move || {
                    let mut loaded = git
                        .file_diff(&dir, &scope, toggles, &path, from.as_ref(), force_large)
                        .map_err(|error| error.to_string())?;
                    // Off the update thread with the read, both schemes at once (R10). Past the
                    // budget the plain diff shows: colour is not worth a pane that never lands.
                    match syntax::highlight_within(
                        &loaded,
                        &path.to_string(),
                        Some(HIGHLIGHT_BUDGET),
                    ) {
                        Some(spans) => loaded.spans = spans,
                        None => crate::log_line(&format!(
                            "changes: {path} not highlighted, over {}s",
                            HIGHLIGHT_BUDGET.as_secs()
                        )),
                    }
                    Ok(loaded)
                },
                answer,
            )
        }
    }
}

/// The repository a pull request address names: its first two path segments on GitHub.
fn repo_of_address(address: &str) -> Option<GithubRepo> {
    let rest = address.strip_prefix(micold_client::features::sidebar::GITHUB_ADDRESS_PREFIX)?;
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    GithubRepo::from_remote_url(&format!("https://github.com/{owner}/{name}"))
}

/// Read a pull request file's head version off the update thread (feature 1150, FR-005, FR-014):
/// local objects first, then GitHub. Anything that cannot run here answers `HeadUnavailable`.
#[allow(clippy::too_many_arguments)]
fn read_file_requested(
    app: &App,
    seq: u64,
    project: PathBuf,
    entry: SessionLocation,
    source: SourceKey,
    commit: String,
    address: String,
    path: RelPath,
    force_large: bool,
) -> Task<Message> {
    let answer = {
        let path = path.clone();
        move |(result, spans): (Result<FileText, FileReadFailure>, Spans)| {
            Message::Changes(Msg::FileRead {
                seq,
                source,
                path: path.clone(),
                result,
                spans,
            })
        }
    };
    if source == SourceKey::Worktree {
        return read_worktree_file(app, &entry, path, force_large, seq, answer);
    }
    let unavailable = || {
        Task::done(answer((
            Err(FileReadFailure::HeadUnavailable),
            Spans::default(),
        )))
    };
    let Ok((dir, _git)) = local(app, &entry) else {
        return unavailable();
    };
    let Some(repo) = repo_of_address(&address) else {
        return unavailable();
    };
    let gh = crate::shell::pr_status::pull_request_source(app, &project);
    let label = format!("file seq={seq} path={path}");
    Task::perform(
        async move {
            let started = Instant::now();
            let read = tokio::task::spawn_blocking(move || {
                let confined = Confined::new(path.as_str())?;
                let gh = gh();
                let source = FileSource::PrHead { dir, repo, commit };
                browse::read_file(
                    &source,
                    &confined,
                    force_large,
                    gh.as_deref().map(|gh| gh as &dyn PullRequestSource),
                    micold_core::unix_time::unix_secs(),
                )
            });
            let result = match tokio::time::timeout(READ_DEADLINE, read).await {
                Ok(joined) => joined.unwrap_or(Err(FileReadFailure::HeadUnavailable)),
                Err(_) => Err(FileReadFailure::HeadUnavailable),
            };
            crate::log_line(&format!(
                "changes: {label} {}ms {}",
                started.elapsed().as_millis(),
                if result.is_ok() { "ok" } else { "failed" }
            ));
            (result, Spans::default())
        },
        answer,
    )
}

/// List the repository's files at the shown source's version (feature 1150, FR-011, FR-014): the
/// worktree's own, or the pull request's head, read as a head file is (local objects first, then
/// one `gh` request).
fn read_tree_requested(
    app: &App,
    seq: u64,
    project: PathBuf,
    entry: SessionLocation,
    source: SourceKey,
    commit: String,
    address: String,
) -> Task<Message> {
    let answer = move |result: Result<Result<TreeListing, FileReadFailure>, String>| {
        Message::Changes(Msg::TreeRead {
            seq,
            result: result.unwrap_or(Err(FileReadFailure::Unreadable)),
        })
    };
    let Ok((dir, _git)) = local(app, &entry) else {
        let why = if source == SourceKey::Worktree {
            FileReadFailure::Unreadable
        } else {
            FileReadFailure::HeadUnavailable
        };
        return Task::done(answer(Ok(Err(why))));
    };
    let label = format!("tree seq={seq} entry={entry:?}");
    if source == SourceKey::Worktree {
        return off_thread(
            label,
            move || Ok(browse::list_tree(&FileSource::Worktree { dir }, None, 0)),
            answer,
        );
    }
    let Some(repo) = repo_of_address(&address) else {
        return Task::done(answer(Ok(Err(FileReadFailure::HeadUnavailable))));
    };
    let gh = crate::shell::pr_status::pull_request_source(app, &project);
    off_thread(
        label,
        move || {
            let gh = gh();
            let source = FileSource::PrHead { dir, repo, commit };
            Ok(browse::list_tree(
                &source,
                gh.as_deref().map(|gh| gh as &dyn PullRequestSource),
                micold_core::unix_time::unix_secs(),
            ))
        },
        answer,
    )
}

/// Read a browsed worktree file off the update thread and colour it there too (feature 1150,
/// FR-011): past [`HIGHLIGHT_BUDGET`] the lines show plain rather than wait.
fn read_worktree_file(
    app: &App,
    entry: &SessionLocation,
    path: RelPath,
    force_large: bool,
    seq: u64,
    answer: impl Fn((Result<FileText, FileReadFailure>, Spans)) -> Message + Send + 'static,
) -> Task<Message> {
    let Ok((dir, _git)) = local(app, entry) else {
        return Task::done(answer((Err(FileReadFailure::Unreadable), Spans::default())));
    };
    off_thread(
        format!("browse seq={seq} path={path}"),
        move || {
            let read = Confined::new(path.as_str()).and_then(|confined| {
                browse::read_file(
                    &FileSource::Worktree { dir },
                    &confined,
                    force_large,
                    None,
                    micold_core::unix_time::unix_secs(),
                )
            });
            let spans = match &read {
                Ok(text) => syntax::highlight_text(&text.lines, path.as_str(), HIGHLIGHT_BUDGET)
                    .unwrap_or_else(|| {
                        crate::log_line(&format!("changes: {path} not highlighted"));
                        Spans::default()
                    }),
                Err(_) => Spans::default(),
            };
            Ok((read, spans))
        },
        move |result| {
            answer(result.unwrap_or((Err(FileReadFailure::Unreadable), Spans::default())))
        },
    )
}

/// The entry's directory and this computer's git, or why a read cannot run here.
pub(crate) fn local(
    app: &App,
    entry: &SessionLocation,
) -> Result<(PathBuf, Arc<dyn Git + Send + Sync>), &'static str> {
    let dir = entry_dir(app, entry).ok_or(NO_PROJECT)?;
    // No local git when the daemon's filesystem is not this one (feature 027): the paths the
    // sidebar lists are the daemon's, so a local read would describe other directories.
    let git = app.caps.shared_git().ok_or(NOT_LOCAL)?;
    Ok((dir, git))
}

/// The most a whole read may take before the pane shows an error with a retry (#771): each git
/// command has its own, shorter deadline, but nothing a read does may leave the view waiting.
const READ_DEADLINE: Duration = Duration::from_secs(45);

/// How long a diff may spend being syntax-coloured before it shows plain.
const HIGHLIGHT_BUDGET: Duration = Duration::from_secs(5);

/// A read slower than this is logged as slow; every read is logged either way.
const SLOW_READ: Duration = Duration::from_secs(1);

/// Run `read` in `spawn_blocking` and answer with `answer`, or with a timeout error once
/// [`READ_DEADLINE`] passes (the blocking task cannot be cancelled; its late answer is dropped).
/// The read is logged with `label`, its duration and its outcome.
fn off_thread<T: Send + 'static>(
    label: String,
    read: impl FnOnce() -> Result<T, String> + Send + 'static,
    answer: impl Fn(Result<T, String>) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            let started = Instant::now();
            let result = match tokio::time::timeout(
                READ_DEADLINE,
                tokio::task::spawn_blocking(read),
            )
            .await
            {
                Ok(joined) => joined.unwrap_or_else(|joined| Err(joined.to_string())),
                Err(_) => Err(format!(
                    "timed out after {}s: the read did not finish",
                    READ_DEADLINE.as_secs()
                )),
            };
            let took = started.elapsed();
            let outcome = match &result {
                Ok(_) => "ok".to_owned(),
                Err(why) => format!("failed: {why}"),
            };
            let slow = if took >= SLOW_READ { " (slow)" } else { "" };
            crate::log_line(&format!(
                "changes: {label} {}ms {outcome}{slow}",
                took.as_millis()
            ));
            result
        },
        answer,
    )
}

/// The directory of `entry`: a listed worktree's own path (an included one lives outside the
/// project), else where a session there would run.
pub(crate) fn entry_dir(app: &App, entry: &SessionLocation) -> Option<PathBuf> {
    if let SessionLocation::Worktree(dir_name) = entry {
        if let Some(worktree) = app
            .core
            .worktree
            .worktrees
            .iter()
            .find(|w| &w.dir_name == dir_name)
        {
            return Some(worktree.path.clone());
        }
    }
    app.core.location_dir(entry)
}

/// Resolve the base (worktrees only) and list the changed files (R1, R7).
fn read_list(
    git: &(dyn Git + Send + Sync),
    dir: &Path,
    entry: &SessionLocation,
    toggles: Toggles,
) -> Result<ChangeList, String> {
    let scope = match entry {
        SessionLocation::Default => ReviewScope::RootUncommitted,
        SessionLocation::Worktree(_) => ReviewScope::Worktree {
            base: git.review_base(dir),
        },
    };
    git.change_list(dir, scope, toggles)
        .map_err(|error| error.to_string())
}

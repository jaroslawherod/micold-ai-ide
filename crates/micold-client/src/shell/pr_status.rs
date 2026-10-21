//! Reading the pull request status of the active project: the shell's half (feature 040).
//!
//! The reducer (`features/pr_status.rs`) decides when a reading starts and what its answer
//! changes. What is left here is the reading itself (contracts/reading-and-wire.md §2, "What one
//! reading does"): ask the daemon for the repository's remotes, locate `gh`, read the pull
//! requests off the update thread, and ask the daemon which merged branches hold nothing newer.
//!
//! # When GitHub is contacted, and why only then
//!
//! [`start`] is the only path to the pull request source, and it runs only when the reducer
//! answers `Effect::Read`: the first listing after the window came to hold its project (S1), the
//! switch turning on (S2), the interval (S3) or a list refresh ending (S4).
//! `tests/pr_status_is_read_only_on_named_events.rs` counts the callers. A repository without a GitHub remote, and a machine without `gh`, end the reading
//! before anything is sent (FR-026).
//!
//! # Why no failure is reported
//!
//! Every way a reading can fail ends as `Msg::Finished` with a `ReadingFailure`, which the reducer
//! turns into "keep what is shown" or "remove what is shown". Nothing here raises a notice
//! (FR-025, SC-004): the remotes request goes through `send_op` only while connected, so its
//! "not connected" notice cannot appear, and its pending entry is dropped silently on a
//! disconnect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::changes::{Msg as ChangesMsg, PrDiffLoad, PrList, Source};
use micold_client::features::pr_status::Msg;
use micold_client::features::pr_status::{Cause, Effect, Outcome, Phase};
use micold_client::features::OpenRequest;
use micold_core::git::GitRemote;
use micold_core::github::{choose_remote, env_include_path, RemoteChoice};
use micold_core::pr_list::BranchPrList;
use micold_core::protocol::messages::{BranchContainment, ClientMsg, MergedBranchQuery};
use micold_core::pull_request::{
    DiffReadFailure, PrState, PullRequestSource, PullRequestStatus, ReadingFailure,
};
use micold_core::review::pr_diff::split_pr_diff;
use micold_core::session::SessionLocation;
use micold_core::worktree_branches::read_branch_set;

use crate::shell::daemon_sync::{send_op, PendingOp};
use crate::shell::env_include::resolve_env_include;
use crate::App;

/// How long the daemon may take to answer the reading's `RemoteList` (FR-021).
pub(crate) const REMOTES_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the daemon may take to answer the reading's `MergedBranchCheck` (FR-021).
pub(crate) const MERGED_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// The most branches one `MergedBranchCheck` asks about; the daemon refuses a longer list. A
/// reading covers a project's worktrees, so more merged branches than this is far beyond the
/// ordinary: the rest get no removal suggestion rather than a refusal of all of them.
const MERGED_CHECK_LIMIT: usize = 50;

/// Unix seconds now. The reducer has no clock; every message that needs the time carries this.
fn now() -> u64 {
    micold_core::unix_time::unix_secs()
}

/// Apply a pull request status message, and start the reading when the reducer asks for one.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let msg = match msg {
        // S3: the interval's tick, stamped with the time here because the reducer has no clock.
        Msg::Tick => Msg::Trigger {
            cause: Cause::Interval,
            now: now(),
        },
        // The 10-second bound on the remotes ran out. Still unanswered: the reading ends as a
        // passing failure. Answered meanwhile: the entry is gone and there is nothing to do.
        Msg::RemotesTimedOut { seq, req } => {
            // A list read of the Changes view (feature 775) bounds its remotes the same way.
            if let Some(PendingOp::PrListRemotes {
                seq: asked,
                started,
                ..
            }) = app.pending_ops.get(&req)
            {
                if *asked != seq {
                    return Task::none();
                }
                let started = *started;
                app.pending_ops.remove(&req);
                return Task::done(Message::Changes(ChangesMsg::PullRequestsRead {
                    seq,
                    read_at: started,
                    result: Err(ReadingFailure::Passing),
                }));
            }
            // A pull request's diff read (feature 775) is bounded the same way.
            if let Some(PendingOp::PrDiffRemotes {
                seq: asked, number, ..
            }) = app.pending_ops.get(&req)
            {
                if *asked != seq {
                    return Task::none();
                }
                let number = *number;
                app.pending_ops.remove(&req);
                return Task::done(Message::Changes(ChangesMsg::PullRequestDiffRead {
                    seq,
                    number,
                    result: Err(DiffReadFailure::Failed(ReadingFailure::Passing)),
                }));
            }
            let unanswered = matches!(
                app.pending_ops.get(&req),
                Some(PendingOp::PrStatusRemotes { seq: asked, .. }) if *asked == seq
            );
            if !unanswered {
                return Task::none();
            }
            app.pending_ops.remove(&req);
            finished(seq, Outcome::Err(ReadingFailure::Passing))
        }
        // The 10-second bound on the merged-branch question ran out. Still unanswered: the
        // statuses are applied with no suggestion. Answered meanwhile: the entry is gone.
        Msg::MergedCheckTimedOut { seq, req } => {
            let unanswered = matches!(
                app.pending_ops.get(&req),
                Some(PendingOp::MergedBranchCheck { seq: asked, .. }) if *asked == seq
            );
            if !unanswered {
                return Task::none();
            }
            let Some(PendingOp::MergedBranchCheck {
                statuses,
                lists,
                started,
                ..
            }) = app.pending_ops.remove(&req)
            else {
                return Task::none();
            };
            finished(
                seq,
                Outcome::Ok {
                    statuses,
                    lists,
                    removable: BTreeSet::new(),
                    started_at: started,
                },
            )
        }
        // The source answered: ask which merged branches hold nothing newer, or end the reading.
        Msg::StatusesRead {
            seq,
            statuses,
            lists,
            started_at,
        } => return on_statuses_read(app, seq, (statuses, lists), started_at),
        other => other,
    };
    if let Msg::Finished { seq, outcome, .. } = &msg {
        log_outcome(app, *seq, outcome);
    }
    // The Changes view's pull request list follows the same triggers (feature 775, FR-007).
    let trigger_now = match &msg {
        Msg::Trigger { now, .. } => Some(*now),
        _ => None,
    };
    let status = match app.core.update_pr_status(msg) {
        Effect::Read { seq } => start(app, seq),
        Effect::None => Task::none(),
    };
    match trigger_now {
        Some(now) => Task::batch([status, refresh_list(app, now)]),
        None => status,
    }
}

/// A status trigger fired (S3, S4): refresh the open Changes view's pull request list. Nothing is
/// asked while no view is open, while GitHub's request limit pauses reads, or while a list read is
/// under way; a list that was `Unavailable` is asked again only here (R3).
fn refresh_list(app: &mut App, now: u64) -> Task<Message> {
    let Some(view) = app.core.changes.open.as_ref() else {
        return Task::none();
    };
    if matches!(view.pull_requests, PrList::Loading { .. })
        || micold_client::features::pr_status::paused(&app.core.pr_status, now)
    {
        return Task::none();
    }
    let branch = view.branch.clone();
    crate::shell::changes::update(app, ChangesMsg::PullRequestsWanted { branch })
}

/// **Open pull request** was chosen on the row `dir` (contract pull-request-ui §4).
///
/// Hands the address the reading found to the link opener, the way an activated link is, so an
/// opener failure is reported as a link's is (feature 031). Nothing is opened when the row lost its
/// status, or when the address is not a GitHub one (FR-014); nothing is sent to GitHub and the
/// application's state is not touched.
pub fn open_requested(app: &App, dir: &str) -> Task<Message> {
    let address = micold_client::features::sidebar::pull_request_address_to_open(
        &app.core.worktree.worktrees,
        &app.core.pr_status.statuses,
        dir,
    );
    match address {
        Some(address) => crate::shell::links::perform(app, OpenRequest::Url(address)),
        None => Task::none(),
    }
}

/// A list refresh ended (S4, FR-018a): read the branches the updated listing shows. The control's
/// busy state and its notice were settled before this runs and are not touched by it (FR-027).
pub fn refresh_ended(app: &mut App) -> Task<Message> {
    update(
        app,
        Msg::Trigger {
            cause: Cause::Refresh,
            now: now(),
        },
    )
}

/// A listing arrived from the daemon (`CatalogChanged`).
pub fn listing_arrived(app: &mut App) -> Task<Message> {
    update(app, Msg::ListingArrived { now: now() })
}

/// The live value of the switch, from `Welcome` or `SettingsChanged`.
pub fn enabled_changed(app: &mut App, enabled: bool) -> Task<Message> {
    // An open Settings page follows the service's value (feature 040, FR-029), unless its user
    // has switched it on this page: their edit stays until they save (BUG-475, FR-026b).
    if let Some(draft) = &mut app.core.settings.settings_draft {
        let untouched = draft
            .baseline
            .as_ref()
            .is_none_or(|opened| opened.pr_status_enabled == draft.github.pr_status_enabled);
        if untouched {
            draft.github.pr_status_enabled = enabled;
        }
    }
    // The Changes view's pull request section follows the same switch (feature 775, FR-014).
    let section =
        crate::shell::changes::update(app, ChangesMsg::PullRequestSettingChanged(enabled));
    let status = update(
        app,
        Msg::EnabledChanged {
            enabled,
            now: now(),
        },
    );
    Task::batch([section, status])
}

/// The daemon answered the reading's `RemoteList`: `None` when it failed. Reads the pull requests
/// on a blocking thread when the repository is on GitHub and `gh` is found.
pub fn on_remotes(
    app: &mut App,
    project: PathBuf,
    seq: u64,
    branches: Vec<String>,
    started: u64,
    remotes: Option<Vec<GitRemote>>,
) -> Task<Message> {
    // An answer that outlived its reading: the project was switched, or the hold was lost.
    let awaited =
        matches!(app.core.pr_status.phase, Phase::Reading { seq: current, .. } if current == seq);
    if !awaited || app.core.workspace.active.as_deref() != Some(project.as_path()) {
        return Task::none();
    }
    let Some(remotes) = remotes else {
        return update(app, finished(seq, Outcome::Err(ReadingFailure::Passing)));
    };
    // No GitHub remote: nothing is sent to GitHub (FR-026).
    let RemoteChoice::Github { repo, .. } = choose_remote(&remotes) else {
        return update(
            app,
            finished(seq, Outcome::Err(ReadingFailure::Unavailable)),
        );
    };
    let source = pull_request_source(app, &project);
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let Some(source) = source() else {
                    return finished(seq, Outcome::Err(ReadingFailure::Unavailable));
                };
                match source.read(&repo, &branches, now()) {
                    Ok(reading) => Msg::StatusesRead {
                        seq,
                        statuses: reading.statuses,
                        lists: reading.lists,
                        started_at: started,
                    },
                    Err(failure) => finished(seq, Outcome::Err(failure)),
                }
            })
            .await
        },
        move |joined| {
            Message::PrStatus(
                joined.unwrap_or_else(|_| finished(seq, Outcome::Err(ReadingFailure::Passing))),
            )
        },
    )
}

/// Whether a Changes view's GitHub read may start: the status reading is not paused by GitHub's
/// request limit (R3), and a daemon route is up, since `send_op` raises a notice when none is and
/// these reads never do (FR-008).
fn may_read_pull_requests(app: &App, started: u64) -> bool {
    let paused = micold_client::features::pr_status::paused(&app.core.pr_status, started);
    let binding = app.active_binding();
    let routed = matches!(
        app.route(binding),
        micold_client::links::Route::To(id) if app.outbox_for(id).is_some()
    );
    !paused && routed
}

/// A Changes view asked for the pull requests of `entry`'s branch set (feature 775, contracts §1;
/// feature 1003). The one entry of the list read: asks the daemon for the remotes, bounded by
/// [`REMOTES_TIMEOUT`], and answers `Msg::PullRequestsRead` of the Changes view. Held back while
/// the status reading is paused by GitHub's request limit (R3); every failure ends as a passing
/// one, never a notice (FR-008).
pub fn list_requested(
    app: &mut App,
    seq: u64,
    project: PathBuf,
    entry: SessionLocation,
    current: Option<String>,
) -> Task<Message> {
    let started = now();
    let passing = move || {
        Message::Changes(ChangesMsg::PullRequestsRead {
            seq,
            read_at: started,
            result: Err(ReadingFailure::Passing),
        })
    };
    if !may_read_pull_requests(app, started) {
        return Task::done(passing());
    }
    let req = app.next_req;
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::PrListRemotes {
            project: asked_for,
            seq,
            entry,
            current,
            started,
        },
        move |req| ClientMsg::RemoteList { req, project },
    );
    // The bound covers the remotes only: a late answer, or a timeout after the answer, finds the
    // pending operation gone and does nothing (`Msg::RemotesTimedOut`).
    Task::perform(
        async move { tokio::time::sleep(REMOTES_TIMEOUT).await },
        move |()| Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
    )
}

/// The daemon answered a list read's `RemoteList` (`None`: it failed). On a blocking thread, when
/// the repository is on GitHub and `gh` is found, reads the entry's branch set with this
/// computer's git (feature 1003, FR-002; no local git: `current` alone, with no notice), then the
/// pull requests of every branch of it.
pub fn on_list_remotes(
    app: &mut App,
    project: PathBuf,
    seq: u64,
    entry: SessionLocation,
    current: Option<String>,
    started: u64,
    remotes: Option<Vec<GitRemote>>,
) -> Task<Message> {
    let answer = move |result| {
        Message::Changes(ChangesMsg::PullRequestsRead {
            seq,
            read_at: started,
            result,
        })
    };
    let awaited = app.core.changes.open.as_ref().is_some_and(|view| {
        matches!(
            view.pull_requests,
            PrList::Loading { seq: current, .. } if current == seq
        )
    });
    if !awaited || app.core.workspace.active.as_deref() != Some(project.as_path()) {
        return Task::none();
    }
    let Some(remotes) = remotes else {
        return Task::done(answer(Err(ReadingFailure::Passing)));
    };
    let RemoteChoice::Github { repo, .. } = choose_remote(&remotes) else {
        return Task::done(answer(Err(ReadingFailure::Unavailable)));
    };
    let source = pull_request_source(app, &project);
    let local = crate::shell::changes::local(app, &entry).ok();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let Some(source) = source() else {
                    return Err(ReadingFailure::Unavailable);
                };
                let branches = match local {
                    Some((dir, git)) => read_branch_set(&*git, &dir, current.as_deref()),
                    None => current.into_iter().collect(),
                };
                source.list_branches(&repo, &branches, now())
            })
            .await
        },
        move |joined| answer(joined.unwrap_or(Err(ReadingFailure::Passing))),
    )
}

/// The pull request source running the user's `gh`, to be built on a blocking thread: `gh` from a
/// version manager is often only on the environment include's `PATH` (`shell/issues.rs`, "Why the
/// environment include is resolved inside the load"). `None` when no `gh` is found.
pub fn pull_request_source(
    app: &App,
    project: &std::path::Path,
) -> impl FnOnce() -> Option<Arc<dyn PullRequestSource + Send + Sync>> + Send + 'static {
    let tooling = app.caps.issue_tooling();
    let cached = app.env_include_cache.get(project).cloned();
    let resolver = app.caps.env_include_shared();
    let enabled = app.env_include_enabled;
    let script = app.env_include_script_path.clone();
    let timeout_secs = app.env_include_timeout_secs;
    let project = project.to_path_buf();
    move || {
        let snapshot = cached.unwrap_or_else(|| {
            resolve_env_include(&*resolver, enabled, &script, timeout_secs, &project)
        });
        let gh = (tooling.locate_gh)(env_include_path(&snapshot.vars))?;
        Some((tooling.pull_requests)(gh))
    }
}

/// A Changes view asked for pull request `number`'s diff (feature 775, contracts §2). Like
/// [`list_requested`]: the daemon's remotes first, bounded by [`REMOTES_TIMEOUT`], held back while
/// GitHub's request limit pauses reads, and a failure is a line in the pane, never a notice.
pub fn diff_requested(app: &mut App, seq: u64, project: PathBuf, number: u64) -> Task<Message> {
    let started = now();
    if !may_read_pull_requests(app, started) {
        return Task::done(Message::Changes(ChangesMsg::PullRequestDiffRead {
            seq,
            number,
            result: Err(DiffReadFailure::Failed(ReadingFailure::Passing)),
        }));
    }
    let req = app.next_req;
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::PrDiffRemotes {
            project: asked_for,
            seq,
            number,
            started,
        },
        move |req| ClientMsg::RemoteList { req, project },
    );
    Task::perform(
        async move { tokio::time::sleep(REMOTES_TIMEOUT).await },
        move |()| Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
    )
}

/// The daemon answered a diff read's `RemoteList` (`None`: it failed). Reads the diff on a
/// blocking thread when the repository is on GitHub and `gh` is found, and splits it per file.
pub fn on_diff_remotes(
    app: &mut App,
    project: PathBuf,
    seq: u64,
    number: u64,
    _started: u64,
    remotes: Option<Vec<GitRemote>>,
) -> Task<Message> {
    let answer = move |result| {
        Message::Changes(ChangesMsg::PullRequestDiffRead {
            seq,
            number,
            result,
        })
    };
    let awaited = app.core.changes.open.as_ref().is_some_and(|view| {
        matches!(
            &view.source,
            Source::PullRequest(pr) if pr.number == number
                && pr.diff == (PrDiffLoad::Loading { seq })
        )
    });
    if !awaited || app.core.workspace.active.as_deref() != Some(project.as_path()) {
        return Task::none();
    }
    let failed = |why| Task::done(answer(Err(DiffReadFailure::Failed(why))));
    let Some(remotes) = remotes else {
        return failed(ReadingFailure::Passing);
    };
    let RemoteChoice::Github { repo, .. } = choose_remote(&remotes) else {
        return failed(ReadingFailure::Unavailable);
    };
    let source = pull_request_source(app, &project);
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let Some(source) = source() else {
                    return Err(DiffReadFailure::Failed(ReadingFailure::Unavailable));
                };
                source
                    .diff(&repo, number, now())
                    .map(|raw| split_pr_diff(&raw))
            })
            .await
        },
        move |joined| {
            answer(joined.unwrap_or(Err(DiffReadFailure::Failed(ReadingFailure::Passing))))
        },
    )
}

/// The source of reading `seq` answered `statuses`. Asks the daemon, once, which of the merged
/// pull requests' branches hold nothing newer (reading-and-wire §2 step 4); with no merged one the
/// reading ends here and the daemon is not asked (FR-023). An answer that outlived its reading is
/// dropped. The daemon is read-only for this question, so nothing it is asked changes anything.
fn on_statuses_read(
    app: &mut App,
    seq: u64,
    (statuses, lists): (
        BTreeMap<String, PullRequestStatus>,
        BTreeMap<String, BranchPrList>,
    ),
    started: u64,
) -> Task<Message> {
    let awaited =
        matches!(app.core.pr_status.phase, Phase::Reading { seq: current, .. } if current == seq);
    if !awaited {
        return Task::none();
    }
    // A merged pull request is asked about only while it is the branch's shown one: a branch with a
    // newer open pull request holds that one, and is not asked about (story 3 scenario 6).
    let queries: Vec<MergedBranchQuery> = statuses
        .iter()
        .filter(|(_, status)| status.state == PrState::Merged)
        .map(|(branch, status)| MergedBranchQuery {
            branch: branch.clone(),
            head: status.head.clone(),
        })
        .take(MERGED_CHECK_LIMIT)
        .collect();
    let project = app.core.workspace.active.clone();
    let (Some(project), false, true) = (project, queries.is_empty(), app.active_outbox().is_some())
    else {
        let outcome = Outcome::Ok {
            statuses,
            lists,
            removable: BTreeSet::new(),
            started_at: started,
        };
        return update(app, finished(seq, outcome));
    };
    let asked = queries.iter().map(|query| query.branch.clone()).collect();
    let req = app.next_req;
    send_op(
        app,
        PendingOp::MergedBranchCheck {
            seq,
            asked,
            statuses,
            lists,
            started,
        },
        move |req| ClientMsg::MergedBranchCheck {
            req,
            project,
            checks: queries,
        },
    );
    // Built inside the future: the timer needs the runtime the task runs on.
    Task::perform(
        async move { tokio::time::sleep(MERGED_CHECK_TIMEOUT).await },
        move |()| Message::PrStatus(Msg::MergedCheckTimedOut { seq, req }),
    )
}

/// The daemon answered the reading's `MergedBranchCheck`: `None` when it was refused or failed.
/// Ends the reading with the statuses it waited with; `removable` holds the branches answered
/// `Contained`, and nothing when there is no usable answer (FR-017).
pub fn on_merged_branches(
    app: &mut App,
    seq: u64,
    asked: Vec<String>,
    (statuses, lists): (
        BTreeMap<String, PullRequestStatus>,
        BTreeMap<String, BranchPrList>,
    ),
    started: u64,
    answers: Option<Vec<BranchContainment>>,
) -> Task<Message> {
    // An answer that outlived its reading: the project was switched, or the hold was lost.
    let awaited =
        matches!(app.core.pr_status.phase, Phase::Reading { seq: current, .. } if current == seq);
    if !awaited {
        return Task::none();
    }
    let removable = answers.map_or_else(BTreeSet::new, |answers| {
        micold_client::features::pr_status::removable_from(&asked, &answers)
    });
    let outcome = Outcome::Ok {
        statuses,
        lists,
        removable,
        started_at: started,
    };
    update(app, finished(seq, outcome))
}

/// Reading `seq` ended with `outcome`, now.
fn finished(seq: u64, outcome: Outcome) -> Msg {
    Msg::Finished {
        seq,
        outcome,
        now: now(),
    }
}

/// The branches a reading covers: every listed worktree with a branch, deduplicated, in listing
/// order (reading-and-wire §1). The listing itself is never asked for (FR-018a).
fn branches(app: &App) -> Vec<String> {
    let mut seen = BTreeSet::new();
    app.core
        .worktree
        .worktrees
        .iter()
        .filter_map(|worktree| worktree.branch.clone())
        .filter(|branch| seen.insert(branch.clone()))
        .collect()
}

/// Start reading `seq`: ask the daemon for the remotes, bounded by [`REMOTES_TIMEOUT`].
fn start(app: &mut App, seq: u64) -> Task<Message> {
    let started = now();
    let branches = branches(app);
    let (Some(project), true) = (
        app.core.workspace.active.clone(),
        app.active_outbox().is_some(),
    ) else {
        // Unreachable while the window holds a project; ended rather than left under way.
        return update(app, finished(seq, Outcome::Err(ReadingFailure::Passing)));
    };
    let req = app.next_req;
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::PrStatusRemotes {
            project: asked_for,
            seq,
            branches,
            started,
        },
        move |req| ClientMsg::RemoteList { req, project },
    );
    // Built inside the future: the timer needs the runtime the task runs on.
    Task::perform(
        async move { tokio::time::sleep(REMOTES_TIMEOUT).await },
        move |()| Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
    )
}

/// One line per reading: how it ended and how many branches the listing shows. Never a pull
/// request's title or address.
fn log_outcome(app: &App, seq: u64, outcome: &Outcome) {
    let kind = match outcome {
        Outcome::Ok { .. } => "ok",
        Outcome::Err(ReadingFailure::Unavailable) => "unavailable",
        Outcome::Err(ReadingFailure::Passing) => "passing failure",
        Outcome::Err(ReadingFailure::RateLimited { .. }) => "rate limited",
    };
    crate::log_line(&format!(
        "pr status: reading {seq} ended {kind}, {} branch(es) listed",
        branches(app).len()
    ));
}

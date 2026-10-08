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
//! answers `Effect::Read`: the first listing after the window came to hold its project (S1), or
//! the switch turning on (S2). `tests/pr_status_is_read_only_on_named_events.rs` counts the
//! callers. A repository without a GitHub remote, and a machine without `gh`, end the reading
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
use std::time::Duration;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::pr_status::Msg;
use micold_client::features::pr_status::{Effect, Outcome, Phase};
use micold_client::features::OpenRequest;
use micold_core::git::GitRemote;
use micold_core::github::{choose_remote, env_include_path, RemoteChoice};
use micold_core::protocol::messages::{BranchContainment, ClientMsg, MergedBranchQuery};
use micold_core::pull_request::{PrState, PullRequestStatus, ReadingFailure};

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
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Apply a pull request status message, and start the reading when the reducer asks for one.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let msg = match msg {
        // The 10-second bound on the remotes ran out. Still unanswered: the reading ends as a
        // passing failure. Answered meanwhile: the entry is gone and there is nothing to do.
        Msg::RemotesTimedOut { seq, req } => {
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
                statuses, started, ..
            }) = app.pending_ops.remove(&req)
            else {
                return Task::none();
            };
            finished(
                seq,
                Outcome::Ok {
                    statuses,
                    removable: BTreeSet::new(),
                    started_at: started,
                },
            )
        }
        // The source answered: ask which merged branches hold nothing newer, or end the reading.
        Msg::StatusesRead {
            seq,
            statuses,
            started_at,
        } => return on_statuses_read(app, seq, statuses, started_at),
        other => other,
    };
    if let Msg::Finished { seq, outcome, .. } = &msg {
        log_outcome(app, *seq, outcome);
    }
    match app.core.update_pr_status(msg) {
        Effect::Read { seq } => start(app, seq),
        Effect::None => Task::none(),
    }
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
    update(
        app,
        Msg::EnabledChanged {
            enabled,
            now: now(),
        },
    )
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
    let tooling = app.caps.issue_tooling();
    let cached = app.env_include_cache.get(&project).cloned();
    let resolver = app.caps.env_include_shared();
    let enabled = app.env_include_enabled;
    let script = app.env_include_script_path.clone();
    let timeout_secs = app.env_include_timeout_secs;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                // `gh` from a version manager is often only on the include's `PATH`
                // (`shell/issues.rs`, "Why the environment include is resolved inside the load").
                let snapshot = cached.unwrap_or_else(|| {
                    resolve_env_include(&*resolver, enabled, &script, timeout_secs, &project)
                });
                let Some(gh) = (tooling.locate_gh)(env_include_path(&snapshot.vars)) else {
                    return finished(seq, Outcome::Err(ReadingFailure::Unavailable));
                };
                match (tooling.pull_requests)(gh).read(&repo, &branches, now()) {
                    Ok(statuses) => Msg::StatusesRead {
                        seq,
                        statuses,
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

/// The source of reading `seq` answered `statuses`. Asks the daemon, once, which of the merged
/// pull requests' branches hold nothing newer (reading-and-wire §2 step 4); with no merged one the
/// reading ends here and the daemon is not asked (FR-023). An answer that outlived its reading is
/// dropped. The daemon is read-only for this question, so nothing it is asked changes anything.
fn on_statuses_read(
    app: &mut App,
    seq: u64,
    statuses: BTreeMap<String, PullRequestStatus>,
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
    let (Some(project), false, true) = (project, queries.is_empty(), app.daemon.is_some()) else {
        let outcome = Outcome::Ok {
            statuses,
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
    statuses: BTreeMap<String, PullRequestStatus>,
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
    let (Some(project), true) = (app.core.workspace.active.clone(), app.daemon.is_some()) else {
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

//! The worktree and session operations, callable without a client (feature 034, research R9).
//!
//! The sidebar reaches these through protocol messages (`WorktreeCreate`, `WorktreeDelete`,
//! `WorktreeRename`, `SessionCreate`, `SessionStart`), and the tool server reaches the same
//! functions, so an agent's create is the dialog's create: the same validation, the same naming and
//! placement, the same provenance record, the same safeguards and the same catalog broadcast
//! (FR-009, FR-011). `server::route` wraps each one and keeps its protocol replies.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use alacritty_terminal::term::TermMode;
use micold_core::cli_reason::{self, AttemptDir, Explanation};
use micold_core::git::GitCli;
use micold_core::mcp::submission::encode_submission;
use micold_core::naming::DerivedNames;
use micold_core::project::{validate_rename, RenameError};
use micold_core::session::{AiCli, SessionId};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{
    create_worktree as git_create_worktree, preflight, remove_worktree, remove_worktree_dir,
    BranchSituation, CreateError, CreateMode, CreateProgressEvent, Leftover, ProvenanceView,
};

use crate::server::refresh_worktrees_and_broadcast;
use crate::state::DaemonState;
use crate::supervisor::PtySession;

/// Where a create's progress goes: the requesting window's `OperationProgress` frames, or nowhere
/// (the tool server passes none).
pub type ProgressSink = Box<dyn FnMut(CreateProgressEvent) + Send>;

/// Why a worktree create did not happen.
#[derive(Debug)]
pub enum CreateFailure {
    /// The project is unknown, or not a git repository.
    NotARepository,
    /// Refused before anything was created, or rolled back (FR-034).
    Create(CreateError),
    /// The blocking task itself failed (panicked or was cancelled).
    Task(String),
}

/// A delete that git carried out: whether the branch delete failed, and what could not be removed.
#[derive(Debug)]
pub struct Deleted {
    pub branch_delete_failed: bool,
    pub leftovers: Vec<Leftover>,
}

/// Why a worktree delete did not happen.
#[derive(Debug)]
pub enum DeleteFailure {
    /// The project is unknown, or not a git repository.
    NotARepository,
    /// The worktree hosts live sessions and stopping them was not requested (W2, T052).
    LiveSessions(Vec<SessionId>),
    /// git failed; the sessions are untouched (FR-023).
    Git(io::Error),
    /// The blocking task itself failed (panicked or was cancelled).
    Task(String),
}

/// Why a rename did not happen.
#[derive(Debug)]
pub enum RenameFailure {
    Invalid(RenameError),
    Io(io::Error),
}

/// What the create dialog's pre-flight finds for `names` (feature 016): whether the branch is
/// free, exists locally or on a remote, is checked out somewhere, or the directory is taken.
/// Never mutates; `None` when the project is not a git repository.
pub async fn branch_situation(
    state: &Arc<DaemonState>,
    project: &Path,
    names: &DerivedNames,
) -> Option<io::Result<BranchSituation>> {
    let Some((repo, true)) = state.project_repo(project) else {
        return None;
    };
    let included = state.included_worktrees(project);
    let (created, unreadable) = state.provenance(project);
    let names = names.clone();
    let situation = tokio::task::spawn_blocking(move || {
        let target = repo.join(".claude/worktrees").join(&names.dir_name);
        let target_exists = target.exists()
            && std::fs::read_dir(&target)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
        preflight(
            &GitCli::new(),
            &repo,
            &target,
            &names.branch,
            target_exists,
            &included,
            &ProvenanceView {
                records: &created,
                state_unreadable: unreadable,
            },
        )
    })
    .await
    .unwrap_or_else(|join| Err(io::Error::other(join.to_string())));
    Some(situation)
}

/// Create worktree `names.dir_name` on `names.branch` under `<project>/.claude/worktrees/`, record
/// it as app-created, and broadcast the refreshed catalog.
///
/// Serialized per project by `worktree_gate`, so two creates cannot interleave and a rolled-back
/// one cannot remove the directory a racing one just populated (T120). `git worktree add` runs on
/// the blocking pool and never under the state lock.
pub async fn create_worktree(
    state: &Arc<DaemonState>,
    project: PathBuf,
    names: DerivedNames,
    mode: CreateMode,
    progress: Option<ProgressSink>,
) -> Result<(), CreateFailure> {
    let Some((repo, true)) = state.project_repo(&project) else {
        return Err(CreateFailure::NotARepository);
    };
    let dir_name = names.dir_name.clone();
    // Read before the blocking task: the included set and the provenance records decide how a
    // blocked holder is described (016 BUG-002, FR-032; 029 FR-016), and the lock must not be held
    // across the git work.
    let included = state.included_worktrees(&project);
    let (created, unreadable) = state.provenance(&project);
    let gate = state.worktree_gate(&project);
    let _serialized = gate.lock().await;
    let result = tokio::task::spawn_blocking(move || {
        let root = repo.join(".claude/worktrees");
        let target = root.join(&names.dir_name);
        let _ = std::fs::create_dir_all(&root);
        let target_exists = target.exists()
            && std::fs::read_dir(&target)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
        let mut progress = progress;
        let mut on_progress = |event: CreateProgressEvent| {
            if let Some(sink) = progress.as_mut() {
                sink(event);
            }
        };
        let r = git_create_worktree(
            &GitCli::new(),
            &repo,
            &target,
            &names,
            target_exists,
            &mode,
            &included,
            &ProvenanceView {
                records: &created,
                state_unreadable: unreadable,
            },
            &mut on_progress,
        );
        // `RolledBack` is the only outcome in which this attempt created anything at `target`.
        // `DuplicateDir` in particular means the directory was already there — removing it would
        // destroy the user's files (feature 016).
        if matches!(r, Err(CreateError::RolledBack(_))) {
            let _ = std::fs::remove_dir_all(&target);
        }
        r
    })
    .await;
    match result {
        Ok(Ok(_worktree)) => {
            // Feature 029 FR-002/FR-010: the record is written *before* the broadcast, so the very
            // first snapshot a window renders already carries `user_created: true`. A persistence
            // failure is not fatal: the worktree exists, and the next successful write, or the
            // claim action, recovers the durable half.
            if let Err(e) = state.record_worktree_provenance(&project, &dir_name) {
                tracing::warn!(
                    project = %project.display(),
                    dir_name = %dir_name,
                    error = %e,
                    "could not record worktree provenance"
                );
            }
            refresh_worktrees_and_broadcast(state, project).await;
            Ok(())
        }
        Ok(Err(e)) => Err(CreateFailure::Create(e)),
        Err(join) => Err(CreateFailure::Task(join.to_string())),
    }
}

/// Delete worktree `dir_name`: refuse when it hosts live sessions and `stop_sessions` is false;
/// otherwise remove it through git, archive its sessions and kill their processes, forget its
/// provenance, and broadcast. `delete_branch` deletes the branch it had checked out.
pub async fn delete_worktree(
    state: &Arc<DaemonState>,
    project: PathBuf,
    dir_name: String,
    stop_sessions: bool,
    delete_branch: bool,
) -> Result<Deleted, DeleteFailure> {
    let Some((repo, true)) = state.project_repo(&project) else {
        return Err(DeleteFailure::NotARepository);
    };
    // FR-045: a worktree delete is destructive and often fails with a reason that lives entirely in
    // git's stderr, so the attempt, the refusal and the failure are all logged. Worktree and branch
    // names and git's own message are identity and error text, never terminal content (FR-047).
    tracing::info!(
        project = %project.display(),
        worktree = %dir_name,
        stop_sessions,
        delete_branch,
        "worktree delete requested"
    );
    // Never orphan a live process: a delete with a live session and `stop_sessions: false` fails
    // specifically instead (W2, T052). No mutation has happened yet.
    let live = state.worktree_live_sessions(&project, &dir_name);
    if !live.is_empty() && !stop_sessions {
        tracing::warn!(
            project = %project.display(),
            worktree = %dir_name,
            live_sessions = live.len(),
            "worktree delete refused: live sessions and stop_sessions not set"
        );
        return Err(DeleteFailure::LiveSessions(live));
    }
    // The same path a session in this worktree resolves as its `cwd`, so the env-include cache
    // entry for it can be dropped once the delete succeeds (BUG-003: a worktree recreated for the
    // same branch reuses this exact path).
    let cache_path = repo.join(".claude/worktrees").join(&dir_name);
    // Feature 013 (FR-011/FR-012): the keep/delete choice, resolved against the worktree's actual
    // bound branch from the discovery cache — `None` for an unbound worktree or a kept branch.
    let branch_to_delete = if delete_branch {
        state.worktree_branch(&project, &dir_name)
    } else {
        None
    };
    let dir2 = dir_name.clone();
    // Serialized with creates on the same project (T120/T124).
    let gate = state.worktree_gate(&project);
    let _serialized = gate.lock().await;
    let result = tokio::task::spawn_blocking(move || {
        let target = repo.join(".claude/worktrees").join(&dir2);
        let outcome = remove_worktree(&GitCli::new(), &repo, &target, branch_to_delete.as_deref())?;
        // Leftovers are NOT an error: git has already deregistered the worktree, so the delete
        // did partly succeed. Failing here skipped the session cleanup and left the directory to
        // come back as an unregistered orphan — the "I deleted it and it reappeared" report.
        let leftovers = remove_worktree_dir(&target);
        Ok::<(bool, Vec<Leftover>), io::Error>((outcome.branch_delete_failed, leftovers))
    })
    .await;
    match result {
        Ok(Ok((branch_delete_failed, mut leftovers))) => {
            // Gated on the git delete having succeeded (main `d88c7a1`): only now archive the
            // worktree's sessions durably and kill their live processes (outside the lock).
            let killed_any = match state.archive_and_remove_worktree_sessions(&project, &dir_name) {
                Ok(ptys) => {
                    let killed_any = !ptys.is_empty();
                    for pty in ptys {
                        let _ = pty.kill();
                    }
                    killed_any
                }
                Err(e) => {
                    tracing::warn!(%e, "archiving deleted worktree's sessions failed");
                    false
                }
            };
            // Windows cannot delete a directory a running process has as its working directory,
            // and a kill returns before the job's processes have exited, so retry with a bounded
            // backoff (about 3 s in all) rather than once.
            if killed_any {
                let mut delay = std::time::Duration::from_millis(100);
                for _ in 0..5 {
                    if leftovers.is_empty() {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                    let target = cache_path.clone();
                    match tokio::task::spawn_blocking(move || remove_worktree_dir(&target)).await {
                        Ok(retried) => leftovers = retried,
                        Err(_) => break,
                    }
                }
            }
            state.invalidate_env_include(&cache_path);
            // A confirmation still waiting to delete this worktree has nothing left to delete
            // (FR-014, EC-4): it fails `not_found` instead of reaching git.
            state.confirmations_worktree_gone(&project, &dir_name);
            // Feature 029 FR-018: the record dies with the worktree, and only once git has released
            // it. A directory name is reusable, so a record that outlived its worktree would hand
            // the next thing created at that path an ownership nobody granted it.
            // Feature 482 (W11, FR-020): its review comments go with it, everywhere.
            state.forget_review_worktree(&project, &dir_name);
            if let Err(e) = state.forget_worktree_provenance(&project, &dir_name) {
                tracing::warn!(
                    project = %project.display(),
                    worktree = %dir_name,
                    error = %e,
                    "could not forget the deleted worktree's provenance"
                );
            }
            if leftovers.is_empty() {
                tracing::info!(
                    project = %project.display(),
                    worktree = %dir_name,
                    branch_delete_failed,
                    "worktree deleted"
                );
            } else {
                // WARN, not ERROR: git released the worktree and the sessions are archived, so this
                // is a partial success. Naming the blockers and their owner is the point (FR-023).
                tracing::warn!(
                    project = %project.display(),
                    worktree = %dir_name,
                    branch_delete_failed,
                    leftovers = %describe_leftovers(&leftovers),
                    "worktree deregistered, but its directory could not be fully removed"
                );
            }
            refresh_worktrees_and_broadcast(state, project).await;
            Ok(Deleted {
                branch_delete_failed,
                leftovers,
            })
        }
        // Failed delete: sessions are left untouched (not killed, not archived) so a recoverable
        // failure never becomes permanent loss. Logged at ERROR so it also lands in the
        // recent-errors ring (FR-046).
        Ok(Err(e)) => {
            tracing::error!(
                project = %project.display(),
                worktree = %dir_name,
                error = %e,
                "worktree delete failed"
            );
            Err(DeleteFailure::Git(e))
        }
        Err(join) => {
            tracing::error!(
                project = %project.display(),
                worktree = %dir_name,
                error = %join,
                "worktree delete task failed"
            );
            Err(DeleteFailure::Task(join.to_string()))
        }
    }
}

/// Give worktree `dir_name` the display name `display_name` and broadcast. A display-name override
/// is durable catalog state; no git is involved.
pub fn rename_worktree(
    state: &Arc<DaemonState>,
    project: &Path,
    dir_name: &str,
    display_name: &str,
) -> Result<(), RenameFailure> {
    let name = validate_rename(display_name).map_err(RenameFailure::Invalid)?;
    state
        .set_worktree_display_name(project, dir_name, &name)
        .map_err(RenameFailure::Io)?;
    state.broadcast_catalog();
    Ok(())
}

/// Start `session`'s process off the async runtime and report whether it is running (T125,
/// feature 034). The caller has already called [`DaemonState::begin_start`], so input typed while
/// this runs is held and replayed.
///
/// Serialized per session by its gate. A failed start moves the catalog and nothing else says so,
/// so it is announced here (feature 026, T087, FR-010): `start_session` records the reason, which
/// fills the wire's `Failed { reason, attempts: 0 }`. A resume has no reply to carry it, and the
/// catalog is the surface both launch modes share. The held input is replayed before the task
/// ends, so whoever acts on its result finds the session already caught up.
pub fn start_session(
    state: &Arc<DaemonState>,
    session: SessionId,
    launch: LaunchMode,
) -> tokio::task::JoinHandle<bool> {
    let gate = queue_for_gate(state, session);
    let state = Arc::clone(state);
    tokio::spawn(async move {
        let _serialized = gate.await;
        let worker = Arc::clone(&state);
        let outcome = tokio::task::spawn_blocking(move || {
            worker.start_session_gated(session, launch)?;
            // Watch this session's own event log, for a provider that reports one (feature 026,
            // T064). In the same blocking hop as the spawn, and **only** for a session the daemon
            // has just started — that is what keeps a merely discovered session unwatched.
            worker.open_event_log_tail(session);
            Ok::<(), io::Error>(())
        })
        .await;
        let started = match outcome {
            Ok(Ok(())) => true,
            Ok(Err(err)) => {
                tracing::warn!(session = %session.0, %err, "session start failed");
                false
            }
            Err(join) => {
                tracing::warn!(session = %session.0, error = %join, "session start task failed");
                false
            }
        };
        state.finish_start(session);
        // After `finish_start`: while the start is in flight the catalog shows the session
        // `Starting`, so the failure is announced once that has ended.
        if !started {
            state.broadcast_catalog();
        }
        // Every window's connection hears of it, whoever asked for the start: one already viewing
        // the session builds its stream now (an agent's `create_session` has no connection).
        state.announce_session_started(session);
        started
    })
}

/// Stop `session` off the async runtime, as [`DaemonState::stop_session`] does, and report whether
/// the session is known (feature 041, R4).
///
/// Serialized per session by its gate, like a start: the stop waits for a start or a supervision
/// respawn in flight, and holds the gate until the processes have ended and the history is carried,
/// which can take up to `TEARDOWN_WAIT`. So it runs on a task of its own, and a connection's loop
/// that asks for it goes on to its next message at once.
pub fn stop_session(state: &Arc<DaemonState>, session: SessionId) -> tokio::task::JoinHandle<bool> {
    let gate = queue_for_gate(state, session);
    let state = Arc::clone(state);
    tokio::spawn(async move {
        let _serialized = gate.await;
        tokio::task::spawn_blocking(move || state.stop_session_gated(session))
            .await
            .unwrap_or_else(|join| {
                tracing::warn!(session = %session.0, error = %join, "session stop task failed");
                false
            })
    })
}

/// Take a place in the queue for `session`'s gate **now**, and return the wait for it.
///
/// The gate is fair, in the order its waiters were first polled, and a spawned task is first
/// polled whenever the runtime gets to it. Polling once here, on the caller, is what keeps a
/// window's `SessionStop` followed by its `SessionStart` in that order. The poll does not block:
/// it either takes the free gate or joins the queue. The waker it registers is replaced by the
/// task's at the task's first poll.
fn queue_for_gate(
    state: &DaemonState,
    session: SessionId,
) -> Pin<Box<dyn Future<Output = tokio::sync::OwnedMutexGuard<()>> + Send>> {
    let mut waiting = Box::pin(tokio::task::unconstrained(
        state.session_gate(session).lock_owned(),
    ));
    match waiting
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(held) => Box::pin(std::future::ready(held)),
        Poll::Pending => waiting,
    }
}

/// Render leftover paths for one log field: `path (uid N)`, comma-separated.
///
/// The owner is what makes the line actionable — a foreign uid means the daemon cannot unlink the
/// entry no matter how often the user retries, and points straight at the cause (typically a
/// container that wrote build output through a bind mount as root).
pub fn describe_leftovers(leftovers: &[Leftover]) -> String {
    leftovers
        .iter()
        .map(|l| match l.foreign_uid {
            Some(uid) => format!("{} (uid {uid})", l.path.display()),
            None => l.path.display().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why a submission was not typed into a session (feature 482, W9). Nothing reached the terminal.
#[derive(Debug)]
pub enum Undelivered {
    /// The session has no running primary process.
    NotRunning,
    /// The CLI has no record that it trusts the session's folder, so it may be asking about it, and
    /// the submission's Enter would answer that question (research R12).
    AsksTrust(AiCli),
    /// The terminal has not asked for bracketed paste, so a multi-line text would arrive as several
    /// submissions.
    NoBracketedPaste,
    /// Writing to the terminal failed.
    WriteFailed(io::Error),
}

impl Undelivered {
    /// What to tell the user: why nothing was typed into `session`.
    pub fn message(&self, session: SessionId) -> String {
        match self {
            Self::NotRunning => format!("session {} is not running", session.0),
            Self::AsksTrust(cli) => {
                let display = cli.provider().display_name();
                format!(
                    "{display} has no record that it trusts the folder of session {}, and may be \
                     asking about it, so nothing was typed; trust the project in {display} first",
                    session.0
                )
            }
            Self::NoBracketedPaste => format!(
                "the terminal of session {} does not accept a pasted text (no bracketed paste), \
                 so nothing was typed",
                session.0
            ),
            Self::WriteFailed(err) => format!(
                "the text could not be typed into session {}: {err}",
                session.0
            ),
        }
    }
}

/// Type `text` into `pty` as one submission (research R12): bracketed when the terminal asked for
/// it, and refused when it did not and `require_bracketed` says a plain one will not do.
pub fn write_submission(
    pty: &PtySession,
    text: &str,
    require_bracketed: bool,
) -> Result<(), Undelivered> {
    let bracketed = pty.term().lock().mode().contains(TermMode::BRACKETED_PASTE);
    if require_bracketed && !bracketed {
        return Err(Undelivered::NoBracketedPaste);
    }
    pty.write_input(&encode_submission(text, bracketed))
        .map_err(Undelivered::WriteFailed)
}

/// Type `text` into `session`'s running AI CLI as one bracketed submission (feature 482, W8, W9):
/// its primary process must be alive, its CLI must trust the session's folder, and its terminal
/// must accept a paste. Nothing is typed otherwise. The text is never logged (W12).
pub async fn type_submission(
    state: &Arc<DaemonState>,
    session: SessionId,
    text: &str,
) -> Result<(), Undelivered> {
    let pty = state
        .primary_pty(session)
        .filter(|pty| pty.is_alive())
        .ok_or(Undelivered::NotRunning)?;
    let (cwd, cli) = state
        .session_cwd_and_cli(session)
        .ok_or(Undelivered::NotRunning)?;
    let st = Arc::clone(state);
    let asks = tokio::task::spawn_blocking(move || st.cli_would_ask_trust(&cwd, cli))
        .await
        .map_err(|join| Undelivered::WriteFailed(io::Error::other(join.to_string())))?;
    if asks {
        return Err(Undelivered::AsksTrust(cli));
    }
    if !pty.is_alive() {
        return Err(Undelivered::NotRunning);
    }
    write_submission(&pty, text, true)
}

/// Why the AI CLI a new session would run cannot run where it would (037, FR-012): the reason and
/// what to do, in `cli_reason`'s words. `None` when it is available.
///
/// What is available and the state of the environment it was looked for in come from one
/// resolution of `cwd`, so the reason describes the attempt that did not find the CLI. A refusal
/// drops the cached environment, so the next attempt looks again (D18).
pub async fn cli_unavailable(state: &Arc<DaemonState>, cwd: &Path, cli: AiCli) -> Option<String> {
    let st = Arc::clone(state);
    let place = cwd.to_path_buf();
    let (available, env) =
        match tokio::task::spawn_blocking(move || st.availability_in(&place)).await {
            Ok(found) => found,
            Err(join) => return Some(format!("the AI CLIs could not be looked for: {join}")),
        };
    if available.contains(&cli) {
        return None;
    }
    let image = crate::state::image_reference();
    let Explanation { reason, action } =
        cli_reason::explain_one(cli, env, crate::state::place(&image), AttemptDir::Dir(cwd));
    state.invalidate_env_include(cwd);
    Some(format!("{reason} {action}"))
}

/// Where a new session goes and what it runs (see [`create_session_with_prompt`]).
#[derive(Debug, Clone, Copy)]
pub struct NewSession<'a> {
    /// The project it belongs to.
    pub project: &'a Path,
    /// Its entry: a worktree directory name, `""` for the project root.
    pub worktree_dir: &'a str,
    /// The directory it runs in (the entry's root).
    pub cwd: &'a Path,
    /// The AI CLI it runs.
    pub cli: AiCli,
}

/// A new session's first input (see [`create_session_with_prompt`]).
#[derive(Debug, Clone, Copy)]
pub struct FirstPrompt<'a> {
    /// What to type, as one submission.
    pub text: &'a str,
    /// When it was asked for: the first-prompt bound counts from here.
    pub asked: tokio::time::Instant,
    /// Refuse a terminal that has not asked for bracketed paste (see [`write_submission`]).
    pub require_bracketed: bool,
}

/// Why a new session's first prompt was not typed (034 FR-017, 482 W9). Nothing reached its
/// terminal; the session itself stays, as any session the user created would.
#[derive(Debug)]
pub enum FirstPromptUndelivered {
    /// The session's process did not start.
    NotStarted,
    /// The CLI would first ask whether to trust the folder; the prompt's Enter would answer it.
    AsksTrust(AiCli),
    /// The CLI was not ready for input within the first-prompt bound of the request.
    NotReady(AiCli, std::time::Duration),
    /// Typing it failed.
    Typing(Undelivered),
}

impl FirstPromptUndelivered {
    /// What to tell the user: why nothing was typed into `session`.
    pub fn message(&self, session: SessionId) -> String {
        match self {
            Self::NotStarted => "the session did not start, so the prompt was not typed".into(),
            Self::AsksTrust(cli) => {
                let display = cli.provider().display_name();
                format!(
                    "{display} would first ask whether to trust this folder, so the prompt was not \
                     typed; trust the project in {display} first"
                )
            }
            Self::NotReady(cli, bound) => format!(
                "{} was not ready for input within {} s of the request, so the prompt was not typed",
                cli.provider().display_name(),
                bound.as_secs()
            ),
            Self::Typing(why) => why.message(session),
        }
    }
}

/// Create a session in `new`'s entry, start it fresh (never a resume), and, given a `prompt`, type
/// it as the session's first input once its CLI is ready, if that happens within the first-prompt
/// bound of `prompt.asked` (034 FR-017, 482 W7). A ready signal after the bound types nothing: the
/// wait has ended, and nothing else writes the prompt.
///
/// `Err` only when the session record could not be created. Otherwise the session exists, whether
/// or not it started, and the inner result says whether the prompt was typed (`Ok` with no
/// prompt). The prompt is never logged.
pub async fn create_session_with_prompt(
    state: &Arc<DaemonState>,
    new: NewSession<'_>,
    prompt: Option<FirstPrompt<'_>>,
) -> io::Result<(SessionId, Result<(), FirstPromptUndelivered>)> {
    let session = state.create_session(new.project, new.worktree_dir, new.cli)?;
    state.begin_start(session);
    state.broadcast_catalog();
    let started = start_session(state, session, LaunchMode::Fresh)
        .await
        .unwrap_or(false);
    let Some(prompt) = prompt else {
        return Ok((session, Ok(())));
    };
    if !started {
        return Ok((session, Err(FirstPromptUndelivered::NotStarted)));
    }
    let st = Arc::clone(state);
    let place = new.cwd.to_path_buf();
    let cli = new.cli;
    // A failed check counts as asking: typing into a trust question would answer it.
    let asks = tokio::task::spawn_blocking(move || st.cli_would_ask_trust(&place, cli))
        .await
        .unwrap_or(true);
    if asks {
        return Ok((session, Err(FirstPromptUndelivered::AsksTrust(cli))));
    }
    let bound = state.first_prompt_bound();
    if !state
        .wait_ready_for_input(session, prompt.asked + bound)
        .await
    {
        return Ok((session, Err(FirstPromptUndelivered::NotReady(cli, bound))));
    }
    let Some(pty) = state.primary_pty(session) else {
        return Ok((
            session,
            Err(FirstPromptUndelivered::Typing(Undelivered::NotRunning)),
        ));
    };
    let typed = write_submission(&pty, prompt.text, prompt.require_bracketed)
        .map_err(FirstPromptUndelivered::Typing);
    Ok((session, typed))
}

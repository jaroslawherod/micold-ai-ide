//! The tool handlers (feature 034, contracts/mcp-tools.md).
//!
//! Every call is scoped to the caller's own project (FR-010): the caller is the session its
//! credential names, and a target outside that project is `not_found` with the same message as one
//! that does not exist at all, so the tools never reveal another project's contents.
//!
//! The read tools answer from the same catalog snapshot the sidebar renders, so an agent and the
//! user see the same worktrees and sessions. The mutating tools call the same operations the
//! sidebar's protocol messages do ([`crate::ops`]), so an agent's change is the user's (FR-009).
//!
//! The destructive tools (`stop_session`, `interrupt_session`, `delete_session`,
//! `delete_worktree`) check in the contract's order: scope, policy, state conflicts and no-ops,
//! then the user's confirmation in every window (FR-014), and only then the effect. So a refused,
//! conflicting or no-op request never shows a prompt.

use std::collections::BTreeMap;
use std::sync::Arc;

use alacritty_terminal::term::TermMode;
use micold_core::git::GitCli;
use micold_core::mcp::errors::{ErrorCategory, OpError};
use micold_core::mcp::policy::{
    self, Caller, ConfirmedOp, CrossSessionAccess, PolicyDecision, TargetFacts,
};
use micold_core::mcp::submission::encode_submission;
use micold_core::mcp::tools::{is_mutating_tool, parse_call, Operation, SessionRef, WorktreeRef};
use micold_core::naming::{self, DerivedNames, NamingError};
use micold_core::protocol::messages::{
    ActivitySignal, ConfirmOperation, ProjectSnapshot, SessionSummary, WireLifecycle,
    WorktreeSnapshot, WorktreeStatus,
};
use micold_core::session::{AiCli, SessionId, SessionLocation, TerminalMode};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{
    self, explain_directory_taken, BlockReason, BranchOrigin, BranchSituation, CreateError,
    CreateMode, ProvenanceView, Worktree, WorktreeOwner,
};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::mcp::confirm::{ConfirmOutcome, ConfirmRequest, ConfirmTarget};
use crate::ops;
use crate::state::DaemonState;

/// The sidebar's name for the project-root row.
const DEFAULT_DISPLAY_NAME: &str = "Default";

/// Validate and run one tool call made by `caller`. Read tools run on a blocking thread (they may
/// run git or fill the worktree cache); mutating tools await the shared operations in [`crate::ops`].
///
/// A mutating tool writes exactly one audit line here, whatever its outcome (FR-018): the caller,
/// the operation, its target and `ok` or the failure's category. Never its input text.
///
/// `hangup` is cancelled when the agent's connection closes: a request still waiting for the
/// user's confirmation is then abandoned, withdrawn from every window, and changes nothing.
pub async fn call(
    state: Arc<DaemonState>,
    caller: SessionId,
    name: String,
    arguments: Value,
    hangup: CancellationToken,
) -> Result<Value, OpError> {
    // The first-prompt bound is counted from here, the request (FR-017).
    let asked = tokio::time::Instant::now();
    let parsed = parse_call(&name, &arguments);
    let audited = is_mutating_tool(&name).then(|| {
        parsed
            .as_ref()
            .map(Operation::audit_target)
            .unwrap_or_default()
    });
    let result = match parsed {
        Ok(operation) => dispatch(state, caller, operation, asked, &hangup).await,
        Err(error) => Err(error),
    };
    if let Some(object) = audited {
        let outcome = match &result {
            Ok(_) => "ok",
            Err(error) => error.category.as_str(),
        };
        tracing::info!(
            target: "micold::mcp",
            caller = %caller.0,
            op = %name,
            target = %object,
            outcome = %outcome,
            "tool call"
        );
    }
    result
}

async fn dispatch(
    state: Arc<DaemonState>,
    caller: SessionId,
    operation: Operation,
    asked: tokio::time::Instant,
    hangup: &CancellationToken,
) -> Result<Value, OpError> {
    match operation {
        Operation::CreateWorktree { branch, name, mode } => {
            create_worktree(&state, caller, branch, name, mode).await
        }
        Operation::CreateSession {
            worktree,
            ai_cli,
            prompt,
        } => create_session(&state, caller, worktree, ai_cli, prompt, asked).await,
        Operation::StartSession { session } => start_session(&state, caller, session).await,
        Operation::RenameWorktree {
            worktree,
            display_name,
        } => rename_worktree(&state, caller, worktree, display_name).await,
        Operation::StopSession { session } => stop_session(&state, caller, session, hangup).await,
        Operation::InterruptSession { session } => {
            interrupt_session(&state, caller, session, hangup).await
        }
        Operation::DeleteSession { session } => {
            delete_session(&state, caller, session, hangup).await
        }
        Operation::DeleteWorktree {
            worktree,
            stop_sessions,
            delete_branch,
        } => {
            delete_worktree(
                &state,
                caller,
                worktree,
                stop_sessions,
                delete_branch,
                hangup,
            )
            .await
        }
        read => blocking(move || read_call(&state, caller, read)).await,
    }
}

/// Run `work` on the blocking pool; a task that panicked is a `service_error`.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, OpError> + Send + 'static,
) -> Result<T, OpError> {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|_| Err(OpError::service_error("the tool call failed unexpectedly")))
}

/// Answer a read-only tool from the catalog snapshot. **Blocking.**
fn read_call(
    state: &DaemonState,
    caller: SessionId,
    operation: Operation,
) -> Result<Value, OpError> {
    let project = caller_project(state, caller)?;
    let modes = state.session_modes(&project.path);
    let context = Context {
        caller,
        project: &project,
        modes: &modes,
    };
    match operation {
        Operation::Whoami => Ok(context.whoami()),
        Operation::ListWorktrees { include_hidden } => {
            Ok(json!({"worktrees": context.worktree_rows(state, include_hidden)}))
        }
        Operation::ListBranches => context.list_branches(state),
        Operation::ListSessions { worktree } => context.list_sessions(worktree.as_ref()),
        Operation::GetSession { session } => context.get_session(SessionId::from_uuid(session.0)),
        Operation::CreateWorktree { .. }
        | Operation::CreateSession { .. }
        | Operation::StartSession { .. }
        | Operation::RenameWorktree { .. }
        | Operation::StopSession { .. }
        | Operation::InterruptSession { .. }
        | Operation::DeleteSession { .. }
        | Operation::DeleteWorktree { .. } => {
            unreachable!("mutating tools are dispatched by call")
        }
    }
}

/// The caller as policy sees it, and its project. **Blocking.**
fn resolve_caller(
    state: &DaemonState,
    caller: SessionId,
) -> Result<(Caller, ProjectSnapshot), OpError> {
    let project = caller_project(state, caller)?;
    let me = project
        .sessions
        .iter()
        .find(|s| s.id == caller)
        .expect("caller_project found the caller in this project");
    let who = Caller {
        session: caller,
        project: project.path.clone(),
        location: match &me.worktree_dir {
            Some(dir) => SessionLocation::Worktree(dir.clone()),
            None => SessionLocation::Default,
        },
        provider: me.provider,
    };
    Ok((who, project))
}

/// What policy says about `operation` from `who`: `None` to go ahead, `Some` to ask the user
/// first (FR-014), or the refusal that is the call's failure.
fn policy_for(who: &Caller, operation: &Operation) -> Result<Option<ConfirmedOp>, OpError> {
    match policy::decide(
        who,
        operation,
        &TargetFacts::default(),
        CrossSessionAccess::default(),
    ) {
        PolicyDecision::Proceed => Ok(None),
        PolicyDecision::Confirm(op) => Ok(Some(op)),
        PolicyDecision::Refuse(error) => Err(error),
    }
}

/// [`policy_for`] for a tool that never waits for a confirmation; one that would is refused as
/// needing it, rather than performed unasked.
fn check_policy(who: &Caller, operation: &Operation) -> Result<(), OpError> {
    match policy_for(who, operation)? {
        None => Ok(()),
        Some(_) => Err(OpError::new(
            ErrorCategory::NeedsConfirmation,
            "this operation needs the user's confirmation in an app window",
        )),
    }
}

/// The wire form of a confirmed operation, as the prompt names it.
fn wire_operation(op: ConfirmedOp) -> ConfirmOperation {
    match op {
        ConfirmedOp::DeleteWorktree {
            stop_sessions,
            delete_branch,
        } => ConfirmOperation::DeleteWorktree {
            stop_sessions,
            delete_branch,
        },
        ConfirmedOp::DeleteSession => ConfirmOperation::DeleteSession,
        ConfirmedOp::StopSession => ConfirmOperation::StopSession,
        ConfirmedOp::InterruptSession => ConfirmOperation::InterruptSession,
    }
}

/// Ask the user, in every window, to allow `op` on `target` (FR-014), naming the calling session,
/// the operation and the target. `Ok` means allowed; every other answer is the call's failure. An
/// agent that hangs up meanwhile abandons the request: the prompt is withdrawn and nothing is done.
async fn ask_user(
    state: &DaemonState,
    who: &Caller,
    project: &ProjectSnapshot,
    op: ConfirmedOp,
    target: ConfirmTarget,
    target_label: String,
    hangup: &CancellationToken,
) -> Result<(), OpError> {
    let caller_label = project
        .sessions
        .iter()
        .find(|s| s.id == who.session)
        .map(|s| s.title.display().to_string())
        .unwrap_or_default();
    let request = ConfirmRequest {
        project: project.path.clone(),
        caller: who.session,
        caller_label,
        operation: wire_operation(op),
        target,
        target_label,
    };
    tokio::select! {
        outcome = state.confirm(request) => outcome.into_result(),
        () = hangup.cancelled() => ConfirmOutcome::Abandoned.into_result(),
    }
}

/// The caller, its project, and `session` in it (`not_found` outside it). **Blocking.**
fn resolve_session_target(
    state: &DaemonState,
    caller: SessionId,
    session: SessionId,
) -> Result<(Caller, ProjectSnapshot, SessionSummary), OpError> {
    let (who, project) = resolve_caller(state, caller)?;
    let target = project
        .sessions
        .iter()
        .find(|s| s.id == session)
        .cloned()
        .ok_or_else(|| OpError::not_found(format!("no session {} in this project", session.0)))?;
    Ok((who, project, target))
}

/// `stop_session` (contracts/mcp-tools.md): scope, then policy (a session may not stop itself,
/// FR-015), then the no-op for a session with no process (FR-012a), then the user's confirmation,
/// then the sidebar's own stop: processes end, the record is `Idle` in every window, and the
/// conversation stays resumable (US3 s2).
async fn stop_session(
    state: &Arc<DaemonState>,
    caller: SessionId,
    target: SessionRef,
    hangup: &CancellationToken,
) -> Result<Value, OpError> {
    let session = SessionId::from_uuid(target.0);
    let st = Arc::clone(state);
    let (who, project, summary) =
        blocking(move || resolve_session_target(&st, caller, session)).await?;
    let confirm = policy_for(&who, &Operation::StopSession { session: target })?;
    if state.session_ptys(session).is_empty() {
        return Ok(json!({ "lifecycle": lifecycle_name(&summary.lifecycle) }));
    }
    if let Some(op) = confirm {
        let label = summary.title.display().to_string();
        ask_user(
            state,
            &who,
            &project,
            op,
            ConfirmTarget::Session(session),
            label,
            hangup,
        )
        .await?;
    }
    let st = Arc::clone(state);
    if !blocking(move || Ok(st.stop_session(session))).await? {
        return Err(OpError::not_found(format!(
            "session {} was deleted while waiting for confirmation",
            session.0
        )));
    }
    Ok(json!({ "lifecycle": "idle" }))
}

/// `interrupt_session` (contracts/mcp-tools.md): scope, then policy (a session may not interrupt
/// itself, FR-015), then the conflict for a session that is not running (FR-012a), then the
/// user's confirmation, then `Ctrl-C` (`0x03`) typed into its primary terminal, as the sidebar's
/// interrupt does. The session keeps running (US3 s2).
async fn interrupt_session(
    state: &Arc<DaemonState>,
    caller: SessionId,
    target: SessionRef,
    hangup: &CancellationToken,
) -> Result<Value, OpError> {
    let session = SessionId::from_uuid(target.0);
    let st = Arc::clone(state);
    let (who, project, summary) =
        blocking(move || resolve_session_target(&st, caller, session)).await?;
    let confirm = policy_for(&who, &Operation::InterruptSession { session: target })?;
    let not_running = || {
        OpError::new(
            ErrorCategory::Conflict,
            format!(
                "session {} is not running, so there is nothing to interrupt",
                session.0
            ),
        )
    };
    if summary.lifecycle != WireLifecycle::Running {
        return Err(not_running());
    }
    // The process the user is asked about: a restart while the prompt waits is another process,
    // and the approval does not carry over to it.
    let asked_about = state.primary_pty(session).ok_or_else(not_running)?;
    if let Some(op) = confirm {
        let label = summary.title.display().to_string();
        ask_user(
            state,
            &who,
            &project,
            op,
            ConfirmTarget::Session(session),
            label,
            hangup,
        )
        .await?;
    }
    let pty = state.primary_pty(session).ok_or_else(not_running)?;
    if !Arc::ptr_eq(&pty, &asked_about) {
        return Err(not_running());
    }
    pty.write_input(&[0x03])
        .map_err(|e| OpError::service_error(format!("could not type the interrupt: {e}")))?;
    Ok(json!({}))
}

/// `delete_session` (contracts/mcp-tools.md): scope, then policy (a session may not delete
/// itself, FR-015), then the user's confirmation, then the sidebar's own delete: the record is
/// archived, its processes end, its credential is revoked, and every window is told.
async fn delete_session(
    state: &Arc<DaemonState>,
    caller: SessionId,
    target: SessionRef,
    hangup: &CancellationToken,
) -> Result<Value, OpError> {
    let session = SessionId::from_uuid(target.0);
    let st = Arc::clone(state);
    let (who, project, summary) =
        blocking(move || resolve_session_target(&st, caller, session)).await?;
    if let Some(op) = policy_for(&who, &Operation::DeleteSession { session: target })? {
        let label = summary.title.display().to_string();
        ask_user(
            state,
            &who,
            &project,
            op,
            ConfirmTarget::Session(session),
            label,
            hangup,
        )
        .await?;
    }
    let st = Arc::clone(state);
    let (owner, ptys) = blocking(move || {
        st.delete_session(session)
            .map_err(|e| OpError::service_error(format!("could not delete the session: {e}")))
    })
    .await?;
    for pty in ptys {
        let _ = pty.kill();
    }
    if owner.is_none() {
        return Err(OpError::not_found(format!(
            "session {} was deleted while waiting for confirmation",
            session.0
        )));
    }
    state.broadcast_catalog();
    Ok(json!({}))
}

/// `delete_worktree` (contracts/mcp-tools.md): scope, then policy (Principle III for a Default
/// caller, FR-015a; the caller's own worktree, FR-015), then the live-session conflict unless
/// `stop_sessions` (US3 s3), then the user's confirmation, then the sidebar's own delete.
async fn delete_worktree(
    state: &Arc<DaemonState>,
    caller: SessionId,
    worktree: WorktreeRef,
    stop_sessions: bool,
    delete_branch: bool,
    hangup: &CancellationToken,
) -> Result<Value, OpError> {
    let WorktreeRef::Named(dir_name) = worktree.clone() else {
        return Err(OpError::invalid_input(
            "\"default\" is the project root, not a worktree; it cannot be deleted",
        ));
    };
    let st = Arc::clone(state);
    let name = dir_name.clone();
    let (who, project, wt) = blocking(move || {
        let (who, project) = resolve_caller(&st, caller)?;
        let wt = project
            .worktrees
            .iter()
            .find(|wt| wt.dir_name == name)
            .cloned()
            .ok_or_else(|| OpError::not_found(format!("no worktree \"{name}\" in this project")))?;
        Ok((who, project, wt))
    })
    .await?;
    let confirm = policy_for(
        &who,
        &Operation::DeleteWorktree {
            worktree,
            stop_sessions,
            delete_branch,
        },
    )?;
    let live = state.worktree_live_sessions(&project.path, &dir_name);
    if !live.is_empty() && !stop_sessions {
        return Err(live_sessions_conflict(&project, &dir_name, &live));
    }
    if let Some(op) = confirm {
        ask_user(
            state,
            &who,
            &project,
            op,
            ConfirmTarget::Worktree {
                dir_name: dir_name.clone(),
            },
            wt.display_name.clone(),
            hangup,
        )
        .await?;
    }
    match ops::delete_worktree(
        state,
        project.path.clone(),
        dir_name.clone(),
        stop_sessions,
        delete_branch,
    )
    .await
    {
        Ok(deleted) => Ok(json!({
            "removed": dir_name,
            "branch_deleted": delete_branch && wt.branch.is_some() && !deleted.branch_delete_failed,
            "leftovers": deleted
                .leftovers
                .iter()
                .map(|l| native(&l.path))
                .collect::<Vec<_>>(),
        })),
        Err(ops::DeleteFailure::LiveSessions(live)) => {
            Err(live_sessions_conflict(&project, &dir_name, &live))
        }
        Err(ops::DeleteFailure::NotARepository) => Err(OpError::invalid_input(
            "this project is not a git repository, so it has no worktrees",
        )),
        Err(ops::DeleteFailure::Git(e)) => Err(OpError::service_error(format!(
            "git failed to remove the worktree: {e}"
        ))),
        Err(ops::DeleteFailure::Task(e)) => Err(OpError::service_error(format!(
            "the worktree delete failed: {e}"
        ))),
    }
}

/// The refusal for a worktree whose sessions are live, naming each (US3 s3).
fn live_sessions_conflict(
    project: &ProjectSnapshot,
    dir_name: &str,
    live: &[SessionId],
) -> OpError {
    let names: Vec<String> = live
        .iter()
        .map(|id| {
            let label = project
                .sessions
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.title.display().to_string())
                .unwrap_or_default();
            format!("\"{label}\" ({})", id.0)
        })
        .collect();
    OpError::new(
        ErrorCategory::Conflict,
        format!(
            "worktree \"{dir_name}\" has live sessions: {}; stop them first or pass stop_sessions: true",
            names.join(", ")
        ),
    )
}

/// `create_worktree` (contracts/mcp-tools.md): scope, then the naming rules and git's ref check,
/// then policy (FR-015a), then the dialog's pre-flight, then the dialog's own create (FR-009).
async fn create_worktree(
    state: &Arc<DaemonState>,
    caller: SessionId,
    branch: String,
    name: Option<String>,
    mode: CreateMode,
) -> Result<Value, OpError> {
    let st = Arc::clone(state);
    let (who, project) = blocking(move || resolve_caller(&st, caller)).await?;
    let Some((repo, true)) = state.project_repo(&project.path) else {
        return Err(OpError::invalid_input(
            "this project is not a git repository, so it has no worktrees",
        ));
    };

    let dir_name = match &name {
        Some(name) => checked_dir_name(name)?,
        None => {
            let derived = naming::dir_name_from_branch(&branch);
            if derived.is_empty() {
                return Err(OpError::invalid_input(
                    NamingError::EmptyNameAfterSlug.to_string(),
                ));
            }
            derived
        }
    };
    let checked = branch.clone();
    blocking(move || {
        GitCli::new()
            .check_branch_name(&repo, &checked)
            .map_err(|e| OpError::invalid_input(e.to_string()))
    })
    .await?;

    check_policy(
        &who,
        &Operation::CreateWorktree {
            branch: branch.clone(),
            name,
            mode: mode.clone(),
        },
    )?;

    let names = DerivedNames {
        dir_name: dir_name.clone(),
        branch: branch.clone(),
    };
    match ops::branch_situation(state, &project.path, &names).await {
        Some(Ok(situation)) if mode.is_compatible_with(&situation) => {}
        Some(Ok(situation)) => {
            return Err(OpError::new(
                ErrorCategory::Conflict,
                describe_situation(&branch, &situation),
            ))
        }
        Some(Err(e)) => {
            return Err(OpError::service_error(format!(
                "could not check the branch: {e}"
            )))
        }
        None => {
            return Err(OpError::invalid_input(
                "this project is not a git repository, so it has no worktrees",
            ))
        }
    }

    ops::create_worktree(state, project.path.clone(), names, mode, None)
        .await
        .map_err(create_failure)?;

    worktree_row(state, caller, dir_name).await
}

/// The `list_worktrees` row of worktree `dir_name`, as it is now, after a change to it. A row
/// missing then is the service's fault: the change was made.
async fn worktree_row(
    state: &Arc<DaemonState>,
    caller: SessionId,
    dir_name: String,
) -> Result<Value, OpError> {
    let st = Arc::clone(state);
    blocking(move || {
        let project = caller_project(&st, caller)?;
        let modes = st.session_modes(&project.path);
        let context = Context {
            caller,
            project: &project,
            modes: &modes,
        };
        context
            .worktree_rows(&st, true)
            .into_iter()
            .find(|row| row["ref"] == dir_name.as_str())
            .ok_or_else(|| {
                OpError::service_error(format!(
                    "worktree \"{dir_name}\" was changed but is no longer in the catalog"
                ))
            })
    })
    .await
}

/// `rename_worktree` (contracts/mcp-tools.md): scope, then policy (FR-015a), then the sidebar's own
/// rename, which every window receives (FR-009, SC-003). The name was checked and trimmed by
/// `parse_call`, as the rename dialog does.
async fn rename_worktree(
    state: &Arc<DaemonState>,
    caller: SessionId,
    worktree: WorktreeRef,
    display_name: String,
) -> Result<Value, OpError> {
    let WorktreeRef::Named(dir_name) = worktree.clone() else {
        return Err(OpError::invalid_input(
            "\"default\" is the project root, not a worktree; it cannot be renamed",
        ));
    };
    let st = Arc::clone(state);
    let name = dir_name.clone();
    let (who, project) = blocking(move || {
        let (who, project) = resolve_caller(&st, caller)?;
        if !project.worktrees.iter().any(|wt| wt.dir_name == name) {
            return Err(OpError::not_found(format!(
                "no worktree \"{name}\" in this project"
            )));
        }
        Ok((who, project))
    })
    .await?;
    check_policy(
        &who,
        &Operation::RenameWorktree {
            worktree,
            display_name: display_name.clone(),
        },
    )?;

    let st = Arc::clone(state);
    let name = dir_name.clone();
    blocking(move || {
        ops::rename_worktree(&st, &project.path, &name, &display_name).map_err(|e| match e {
            ops::RenameFailure::Invalid(_) => {
                OpError::invalid_input("display_name cannot be empty or only whitespace")
            }
            ops::RenameFailure::Io(e) => {
                OpError::service_error(format!("could not save the new name: {e}"))
            }
        })
    })
    .await?;
    worktree_row(state, caller, dir_name).await
}

/// `start_session` (contracts/mcp-tools.md): scope, then policy, then the no-op for a session that
/// is already starting, running or restarting (FR-012a), then the sidebar's own start (FR-009),
/// shown as `Starting` in every window until its process runs (US3 s1).
async fn start_session(
    state: &Arc<DaemonState>,
    caller: SessionId,
    target: SessionRef,
) -> Result<Value, OpError> {
    let session = SessionId::from_uuid(target.0);
    let st = Arc::clone(state);
    let (who, lifecycle) = blocking(move || {
        let (who, project) = resolve_caller(&st, caller)?;
        let lifecycle = project
            .sessions
            .iter()
            .find(|s| s.id == session)
            .map(|s| s.lifecycle.clone())
            .ok_or_else(|| {
                OpError::not_found(format!("no session {} in this project", session.0))
            })?;
        Ok((who, lifecycle))
    })
    .await?;
    check_policy(&who, &Operation::StartSession { session: target })?;

    if !matches!(
        lifecycle,
        WireLifecycle::Starting | WireLifecycle::Running | WireLifecycle::Restarting { .. }
    ) {
        // The sidebar's `SessionStart`: bringing an existing session back is a resume. Every
        // window sees it `Starting` while the start runs.
        state.begin_start(session);
        state.broadcast_catalog();
        let started = ops::start_session(state, session, LaunchMode::Resume)
            .await
            .unwrap_or(false);
        if !started {
            // A failed start is the call's failure (FR-013), with the reason the sidebar shows.
            let st = Arc::clone(state);
            let row = blocking(move || {
                let project = caller_project(&st, caller)?;
                let modes = st.session_modes(&project.path);
                Context {
                    caller,
                    project: &project,
                    modes: &modes,
                }
                .get_session(session)
            })
            .await?;
            let reason = row["failure_reason"]
                .as_str()
                .filter(|r| !r.is_empty())
                .unwrap_or("its process could not be started");
            return Err(OpError::service_error(format!(
                "the session did not start: {reason}"
            )));
        }
    }
    let st = Arc::clone(state);
    let lifecycle = blocking(move || {
        let project = caller_project(&st, caller)?;
        let modes = st.session_modes(&project.path);
        Context {
            caller,
            project: &project,
            modes: &modes,
        }
        .get_session(session)
    })
    .await?["lifecycle"]
        .clone();
    Ok(json!({ "lifecycle": lifecycle }))
}

/// `create_session` (contracts/mcp-tools.md): scope and the worktree, then the CLI's availability
/// where the session would run, then the sidebar's own create and start (FR-009), then the
/// optional first prompt once the CLI is ready for it (FR-017).
async fn create_session(
    state: &Arc<DaemonState>,
    caller: SessionId,
    worktree: WorktreeRef,
    ai_cli: Option<AiCli>,
    prompt: Option<String>,
    asked: tokio::time::Instant,
) -> Result<Value, OpError> {
    let st = Arc::clone(state);
    let place = worktree.clone();
    let (who, project, cwd) = blocking(move || {
        let (who, project) = resolve_caller(&st, caller)?;
        let cwd = match &place {
            WorktreeRef::Default => project.path.clone(),
            WorktreeRef::Named(name) => project
                .worktrees
                .iter()
                .find(|wt| &wt.dir_name == name)
                .map(|wt| wt.path.clone())
                .ok_or_else(|| {
                    OpError::not_found(format!("no worktree \"{name}\" in this project"))
                })?,
        };
        Ok((who, project, cwd))
    })
    .await?;
    let cli = match ai_cli {
        Some(cli) => cli,
        None => state.default_ai_cli(),
    };
    check_policy(
        &who,
        &Operation::CreateSession {
            worktree: worktree.clone(),
            ai_cli: Some(cli),
            prompt: None,
        },
    )?;

    // Checked before the record exists, so a missing CLI leaves nothing behind (US2 s5).
    let st = Arc::clone(state);
    let place = cwd.clone();
    let available = blocking(move || Ok(st.ai_clis_available_in(&place))).await?;
    if !available.contains(&cli) {
        let provider = cli.provider();
        return Err(OpError::service_error(format!(
            "{} is not installed where this session would run: `{}` is not on its PATH",
            provider.display_name(),
            provider.command()
        )));
    }

    let dir = match &worktree {
        WorktreeRef::Default => String::new(),
        WorktreeRef::Named(name) => name.clone(),
    };
    let session = state
        .create_session(&project.path, &dir, cli)
        .map_err(|e| OpError::service_error(format!("could not create the session: {e}")))?;
    state.begin_start(session);
    state.broadcast_catalog();
    let started = ops::start_session(state, session, LaunchMode::Fresh)
        .await
        .unwrap_or(false);

    let display = cli.provider().display_name();
    let prompt_given = prompt.is_some();
    let undelivered = match prompt {
        None => None,
        Some(_) if !started => {
            Some("the session did not start, so the prompt was not typed".to_string())
        }
        Some(text) => {
            let st = Arc::clone(state);
            let place = cwd.clone();
            if blocking(move || Ok(st.cli_would_ask_trust(&place, cli))).await? {
                // Its first screen is the trust question; the prompt's Enter would answer it.
                Some(format!(
                    "{display} would first ask whether to trust this folder, so the prompt was not \
                     typed; trust the project in {display} first"
                ))
            } else if deliver_first_prompt(state, session, &text, asked).await {
                None
            } else {
                Some(format!(
                    "{display} was not ready for input within {} s of the request, so the prompt \
                     was not typed",
                    state.first_prompt_bound().as_secs()
                ))
            }
        }
    };
    let st = Arc::clone(state);
    let lifecycle = blocking(move || {
        let project = caller_project(&st, caller)?;
        let modes = st.session_modes(&project.path);
        let context = Context {
            caller,
            project: &project,
            modes: &modes,
        };
        context.get_session(session)
    })
    .await?["lifecycle"]
        .clone();
    let mut out = json!({
        "session": session.0.to_string(),
        "lifecycle": lifecycle,
        "prompt_delivered": prompt_given.then_some(undelivered.is_none()),
    });
    if let Some(reason) = undelivered {
        out["prompt_reason"] = json!(reason);
    }
    Ok(out)
}

/// Type `text` into `session` as one submission once its CLI is ready for it, if that happens
/// within the first-prompt bound of `asked` (FR-017). A signal after the bound types nothing: the
/// wait has ended, and nothing else writes the prompt.
async fn deliver_first_prompt(
    state: &Arc<DaemonState>,
    session: SessionId,
    text: &str,
    asked: tokio::time::Instant,
) -> bool {
    let deadline = asked + state.first_prompt_bound();
    if !state.wait_ready_for_input(session, deadline).await {
        return false;
    }
    let Some(pty) = state.primary_pty(session) else {
        return false;
    };
    let bracketed = pty.term().lock().mode().contains(TermMode::BRACKETED_PASTE);
    pty.write_input(&encode_submission(text, bracketed)).is_ok()
}

/// A worktree directory name given by the agent: accepted only as the dialog would write it
/// (`naming::dir_name_from_branch` leaves it unchanged).
fn checked_dir_name(name: &str) -> Result<String, OpError> {
    let normal = naming::dir_name_from_branch(name);
    if normal.is_empty() {
        return Err(OpError::invalid_input(
            NamingError::EmptyNameAfterSlug.to_string(),
        ));
    }
    if normal != name {
        return Err(OpError::invalid_input(format!(
            "\"{name}\" is not a worktree name the create dialog would write; it would use \
             \"{normal}\" (lowercase letters, digits and '-')"
        )));
    }
    Ok(normal)
}

/// The pre-flight's situation in the dialog's words, with the mode that fits it (US2 s2).
fn describe_situation(branch: &str, situation: &BranchSituation) -> String {
    match situation {
        BranchSituation::Free => {
            format!("branch '{branch}' does not exist yet; use mode new_branch to create it")
        }
        BranchSituation::LocalAvailable { branch } => format!(
            "branch '{branch}' already exists locally and no worktree has it checked out; use \
             mode existing_local to check it out"
        ),
        BranchSituation::RemoteOnly { branch, remotes } => format!(
            "branch '{branch}' exists only on the remote(s) {}; use mode track_remote with one \
             of them as remote, or new_branch to start a fresh branch",
            remotes.join(", ")
        ),
        BranchSituation::Blocked { branch, reason } => reason.explain(branch),
        BranchSituation::DirectoryTaken { dir } => explain_directory_taken(dir).to_string(),
    }
}

/// A create the shared operation refused or could not complete.
fn create_failure(failure: ops::CreateFailure) -> OpError {
    match failure {
        ops::CreateFailure::NotARepository => {
            OpError::invalid_input("this project is not a git repository, so it has no worktrees")
        }
        ops::CreateFailure::Create(CreateError::BranchInUse { branch, reason }) => {
            OpError::new(ErrorCategory::Conflict, reason.explain(&branch))
        }
        ops::CreateFailure::Create(CreateError::SituationChanged) => OpError::new(
            ErrorCategory::Conflict,
            "the branch changed while the worktree was being created, so nothing was done",
        ),
        ops::CreateFailure::Create(CreateError::DuplicateDir { dir }) => OpError::new(
            ErrorCategory::Conflict,
            explain_directory_taken(&dir).to_string(),
        ),
        ops::CreateFailure::Create(CreateError::RolledBack(stderr)) => {
            OpError::service_error(format!("git failed to create the worktree: {stderr}"))
        }
        ops::CreateFailure::Task(e) => {
            OpError::service_error(format!("the worktree create failed: {e}"))
        }
    }
}

/// The project that holds `caller`, with its worktrees discovered. A caller no longer in any project
/// is `not_found`: its credential outlived its session only for the instant before revocation.
fn caller_project(state: &DaemonState, caller: SessionId) -> Result<ProjectSnapshot, OpError> {
    let find = |state: &DaemonState| {
        state
            .catalog_snapshot()
            .projects
            .into_iter()
            .find(|p| p.sessions.iter().any(|s| s.id == caller))
    };
    let project = find(state).ok_or_else(|| OpError::not_found("the calling session is gone"))?;
    state.ensure_worktrees(&project.path);
    find(state).ok_or_else(|| OpError::not_found("the calling session is gone"))
}

struct Context<'a> {
    caller: SessionId,
    project: &'a ProjectSnapshot,
    modes: &'a BTreeMap<SessionId, TerminalMode>,
}

impl Context<'_> {
    fn whoami(&self) -> Value {
        let me = self
            .project
            .sessions
            .iter()
            .find(|s| s.id == self.caller)
            .expect("caller_project found the caller in this project");
        json!({
            "session": self.caller.0.to_string(),
            "project": {
                "name": self.project.display_name,
                "path": native(&self.project.path),
            },
            "worktree": worktree_ref(me),
            "ai_cli": self.ai_cli(me),
        })
    }

    /// `default` first, then the sidebar's set; assistant-owned rows only when asked for.
    fn worktree_rows(&self, state: &DaemonState, include_hidden: bool) -> Vec<Value> {
        let (records, unreadable) = state.provenance(&self.project.path);
        let view = if unreadable {
            ProvenanceView::unreadable()
        } else {
            ProvenanceView::new(&records)
        };
        let root_count = self
            .project
            .sessions
            .iter()
            .filter(|s| s.worktree_dir.is_none())
            .count();
        let mut rows = vec![json!({
            "ref": WorktreeRef::DEFAULT,
            "display_name": DEFAULT_DISPLAY_NAME,
            "branch": Value::Null,
            "path": native(&self.project.path),
            "status": "clean",
            "app_created": false,
            "assistant_owned": false,
            "session_count": root_count,
        })];
        for wt in &self.project.worktrees {
            let assistant_owned = owner(wt, &view) == WorktreeOwner::Agent;
            if assistant_owned && !include_hidden {
                continue;
            }
            let session_count = self
                .project
                .sessions
                .iter()
                .filter(|s| s.worktree_dir.as_deref() == Some(wt.dir_name.as_str()))
                .count();
            rows.push(json!({
                "ref": wt.dir_name,
                "display_name": wt.display_name,
                "branch": wt.branch,
                "path": native(&wt.path),
                "status": status_name(wt.status),
                "app_created": wt.user_created,
                "assistant_owned": assistant_owned,
                "session_count": session_count,
            }));
        }
        rows
    }

    fn list_branches(&self, state: &DaemonState) -> Result<Value, OpError> {
        let path = &self.project.path;
        let Some((repo, true)) = state.project_repo(path) else {
            return Ok(json!({"branches": []}));
        };
        let included = state.included_worktrees(path);
        let (records, unreadable) = state.provenance(path);
        let view = if unreadable {
            ProvenanceView::unreadable()
        } else {
            ProvenanceView::new(&records)
        };
        let candidates =
            worktree::branch_candidates(&micold_core::git::GitCli::new(), &repo, &included, &view)
                .map_err(|e| OpError::service_error(format!("could not list the branches: {e}")))?;
        let branches: Vec<Value> = candidates
            .iter()
            .map(|c| {
                let checked_out_in = match &c.blocked_by {
                    Some(BlockReason::CheckedOutAt { path, .. }) => {
                        Some(self.worktree_ref_at(path))
                    }
                    Some(BlockReason::CheckedOutInProjectRoot) => {
                        Some(WorktreeRef::DEFAULT.to_string())
                    }
                    Some(BlockReason::CheckedOutOutsideApp { .. }) | None => None,
                };
                json!({
                    "name": c.name,
                    "kind": match c.origin {
                        BranchOrigin::Local => "local",
                        BranchOrigin::Remote { .. } => "remote",
                    },
                    "checked_out_in": checked_out_in,
                    "unavailable_reason": c.blocked_by.as_ref().map(|r| r.explain(&c.name)),
                })
            })
            .collect();
        Ok(json!({ "branches": branches }))
    }

    /// The ref of the worktree at `path`: its row's `dir_name`, which is the folder name unless
    /// discovery had to disambiguate it.
    fn worktree_ref_at(&self, path: &std::path::Path) -> String {
        self.project
            .worktrees
            .iter()
            .find(|wt| wt.path == path)
            .map(|wt| wt.dir_name.clone())
            .unwrap_or_else(|| worktree::folder_name(path))
    }

    fn list_sessions(&self, filter: Option<&WorktreeRef>) -> Result<Value, OpError> {
        if let Some(WorktreeRef::Named(name)) = filter {
            if !self.project.worktrees.iter().any(|wt| &wt.dir_name == name) {
                return Err(OpError::not_found(format!(
                    "no worktree \"{name}\" in this project"
                )));
            }
        }
        let sessions: Vec<Value> = self
            .project
            .sessions
            .iter()
            .filter(|s| filter.is_none_or(|f| worktree_ref(s) == f.as_str()))
            .map(|s| self.session_row(s))
            .collect();
        Ok(json!({ "sessions": sessions }))
    }

    fn get_session(&self, session: SessionId) -> Result<Value, OpError> {
        let summary = self
            .project
            .sessions
            .iter()
            .find(|s| s.id == session)
            .ok_or_else(|| {
                OpError::not_found(format!("no session {} in this project", session.0))
            })?;
        let mut row = self.session_row(summary);
        if let WireLifecycle::Failed { reason, .. } = &summary.lifecycle {
            row["failure_reason"] = json!(reason);
        }
        Ok(row)
    }

    fn session_row(&self, s: &SessionSummary) -> Value {
        json!({
            "ref": s.id.0.to_string(),
            "label": s.title.display(),
            "ai_cli": self.ai_cli(s),
            "lifecycle": lifecycle_name(&s.lifecycle),
            "activity": activity_name(&s.activity),
            "worktree": worktree_ref(s),
            "is_caller": s.id == self.caller,
        })
    }

    /// The CLI a session runs, or `regular_terminal` when its pane is a plain shell.
    fn ai_cli(&self, s: &SessionSummary) -> &'static str {
        match self.modes.get(&s.id) {
            Some(TerminalMode::Regular) => "regular_terminal",
            _ => s.provider.tool_name(),
        }
    }
}

/// A path in the platform's native form (EC-17). Git prints Windows paths with forward slashes;
/// re-assembling the components joins them with the platform separator.
fn native(path: &std::path::Path) -> String {
    path.components()
        .collect::<std::path::PathBuf>()
        .display()
        .to_string()
}

fn worktree_ref(s: &SessionSummary) -> &str {
    s.worktree_dir.as_deref().unwrap_or(WorktreeRef::DEFAULT)
}

/// Who owns a sidebar row, by the one classification the sidebar itself uses. Health does not enter
/// into it (`classify_owner` is health-blind), so the status given here is immaterial.
fn owner(wt: &WorktreeSnapshot, view: &ProvenanceView<'_>) -> WorktreeOwner {
    worktree::classify_owner(
        &Worktree {
            dir_name: wt.dir_name.clone(),
            path: wt.path.clone(),
            branch: wt.branch.clone(),
            status: worktree::WorktreeStatus::Valid,
            included: wt.included,
        },
        view,
    )
}

fn status_name(status: WorktreeStatus) -> &'static str {
    match status {
        WorktreeStatus::Clean => "clean",
        WorktreeStatus::Missing => "missing",
        WorktreeStatus::Locked => "locked",
        WorktreeStatus::Prunable => "prunable",
    }
}

fn lifecycle_name(lifecycle: &WireLifecycle) -> &'static str {
    match lifecycle {
        WireLifecycle::Idle => "idle",
        WireLifecycle::Starting => "starting",
        WireLifecycle::Running => "running",
        WireLifecycle::Restarting { .. } => "restarting",
        WireLifecycle::Failed { .. } => "failed",
        WireLifecycle::InterruptedResumable => "interrupted_resumable",
    }
}

fn activity_name(activity: &ActivitySignal) -> &'static str {
    match activity {
        ActivitySignal::Unknown => "unknown",
        ActivitySignal::Working => "working",
        ActivitySignal::AwaitingInput => "awaiting_input",
        ActivitySignal::Ended { .. } => "ended",
    }
}

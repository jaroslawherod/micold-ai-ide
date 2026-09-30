//! The tool handlers (feature 034, contracts/mcp-tools.md).
//!
//! Every call is scoped to the caller's own project (FR-010): the caller is the session its
//! credential names, and a target outside that project is `not_found` with the same message as one
//! that does not exist at all, so the tools never reveal another project's contents.
//!
//! The read tools answer from the same catalog snapshot the sidebar renders, so an agent and the
//! user see the same worktrees and sessions. **Blocking**: `list_branches` runs git and the worktree
//! cache may need filling, so the server calls [`call`] on a blocking thread.

use std::collections::BTreeMap;

use micold_core::mcp::errors::OpError;
use micold_core::mcp::tools::{parse_call, Operation, WorktreeRef};
use micold_core::protocol::messages::{
    ActivitySignal, ProjectSnapshot, SessionSummary, WireLifecycle, WorktreeSnapshot,
    WorktreeStatus,
};
use micold_core::session::{SessionId, TerminalMode};
use micold_core::worktree::{
    self, BlockReason, BranchOrigin, ProvenanceView, Worktree, WorktreeOwner,
};
use serde_json::{json, Value};

use crate::state::DaemonState;

/// The sidebar's name for the project-root row.
const DEFAULT_DISPLAY_NAME: &str = "Default";

/// Validate and run one tool call made by `caller`.
pub fn call(
    state: &DaemonState,
    caller: SessionId,
    name: &str,
    arguments: &Value,
) -> Result<Value, OpError> {
    let operation = parse_call(name, arguments)?;
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
        let candidates = worktree::branch_candidates(
            &micold_core::git::GitCli::new(),
            &repo,
            &included,
            &view,
        )
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
            .ok_or_else(|| OpError::not_found(format!("no session {} in this project", session.0)))?;
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

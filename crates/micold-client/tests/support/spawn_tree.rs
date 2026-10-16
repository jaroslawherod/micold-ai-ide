//! A project with one worktree and sessions that name their origins (feature 551).

use micold_client::app::State;
use micold_client::features::worktree;
use micold_core::origin::{OriginStatus, SessionOrigin};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

pub const PROJECT: &str = "/fixture/project";
pub const DIR: &str = "feat-a";

pub fn session(location: &SessionLocation, origin: Option<(SessionId, OriginStatus)>) -> Session {
    let mut s = Session::start_new(location.clone(), AiCli::ClaudeCode);
    s.origin = origin.map(|(id, status)| SessionOrigin { id, status });
    s
}

pub fn known(s: &Session) -> Option<(SessionId, OriginStatus)> {
    Some((s.id, OriginStatus::Known))
}

pub fn state_with(sessions: Vec<Session>) -> State {
    let mut workspace = super::workspace_with(vec![(PROJECT, sessions)]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    State {
        workspace,
        worktree: worktree::State {
            worktrees: vec![Worktree {
                dir_name: DIR.to_string(),
                path: PathBuf::from(PROJECT).join(".claude/worktrees").join(DIR),
                branch: Some("feat/a".to_string()),
                status: WorktreeStatus::Valid,
                included: false,
            }],
            ..Default::default()
        },
        ..State::default()
    }
}

pub fn wt() -> SessionLocation {
    SessionLocation::Worktree(DIR.to_string())
}

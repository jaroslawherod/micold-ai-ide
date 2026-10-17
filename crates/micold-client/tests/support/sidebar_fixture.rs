//! A project open at `/repo` with worktrees and sessions placed by the caller: the sidebar and
//! navigation tests' one builder (features 029, 803).

use std::path::PathBuf;

use micold_client::app::State;
use micold_core::project::{Availability, Project};
use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};

/// A state with the project `/repo` open and nothing else.
pub fn state_with_active() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state
}

/// A worktree at `/repo`, matching the paths `state_with_active` sets up.
pub fn listed(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/repo/.claude/worktrees").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// Replace the listing the way the client does when one arrives: apply, then drain, so the sidebar
/// prunes its own expansion set rather than the worktree feature reaching into it (T066).
pub fn deliver(state: &mut State, worktrees: Vec<Worktree>) {
    let outcomes = state.set_worktrees(worktrees);
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(state, o));
}

/// Start a session at `location` of `/repo` (the store only; nothing becomes current).
pub fn session_at(state: &mut State, location: SessionLocation) -> SessionId {
    let session = Session::start_new(location, AiCli::ClaudeCode);
    let id = session.id;
    state
        .workspace
        .sessions
        .entry(PathBuf::from("/repo"))
        .or_default()
        .push(session);
    id
}

/// A worktree location by name.
pub fn worktree(dir_name: &str) -> SessionLocation {
    SessionLocation::Worktree(dir_name.to_string())
}

/// A run group whose runs sit in the worktrees `dirs`, in order.
pub fn run_group(id: u128, name: &str, dirs: &[&str]) -> RunGroup {
    RunGroup::new(NewGroup {
        id: GroupId(uuid::Uuid::from_u128(id)),
        name: name.into(),
        naming: micold_core::naming::WorktreeNaming {
            type_: Some(micold_core::naming::ConventionalType::Feat),
            ticket: None,
            name: name.into(),
        },
        prompt: "p".into(),
        base_branch: "main".into(),
        base_commit: "abc".into(),
        created: std::time::SystemTime::UNIX_EPOCH,
        runs: dirs
            .iter()
            .zip(1u8..)
            .map(|(dir, number)| Run {
                number,
                provider: AiCli::ClaudeCode,
                names: micold_core::naming::DerivedNames {
                    dir_name: (*dir).to_string(),
                    branch: format!("feat/{dir}"),
                },
                session: None,
                status: RunStatus::Creating,
            })
            .collect(),
    })
    .expect("valid group")
}

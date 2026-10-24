//! A project open at `/repo` with worktrees and sessions placed by the caller: the sidebar and
//! navigation tests' one builder (features 029, 803).

use std::path::PathBuf;

use micold_client::app::State;
use micold_core::project::{Availability, Project};
use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};

/// A known project named `name`, at `/<name>`, with the given availability.
pub fn project(name: &str, availability: Availability) -> Project {
    Project {
        path: PathBuf::from(format!("/{name}")),
        display_name: name.to_string(),
        is_git_repo: true,
        availability,
    }
}

/// A session at the project root, for a project other than the active one (it is pushed into
/// `workspace.sessions` under that project's path by the caller).
pub fn foreign_session() -> Session {
    Session::start_new(SessionLocation::Default, AiCli::ClaudeCode)
}

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

/// Start a session at `location` of `/repo` (the store only; nothing becomes current). It is idle,
/// so it leaves its location in the Idle state group (feature 1125): every location these
/// fixtures build is listed in today's order unless a test sets a state.
pub fn session_at(state: &mut State, location: SessionLocation) -> SessionId {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.lifecycle = micold_core::session::SessionLifecycle::Idle;
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

/// Default with the current session, `feat-a` with one, `feat-b` empty (navigation and quick
/// switcher apply tests, features 803 and 968).
pub fn arranged() -> (State, SessionId, SessionId) {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    let default = session_at(&mut state, SessionLocation::Default);
    let a = session_at(&mut state, worktree("feat-a"));
    state.session.active = Some(default);
    (state, default, a)
}

/// How many sessions are running across every project.
pub fn session_count(state: &State) -> usize {
    state.workspace.sessions.values().map(Vec::len).sum()
}

/// Run a resolved step the way the binary does: apply, dispatch the follow-ups, then finish.
pub fn press(
    state: &mut State,
    step: micold_client::features::navigation::Step,
    terminal_had_focus: bool,
) {
    for message in state.apply_navigation(step.clone(), terminal_had_focus) {
        state.update(message);
    }
    state.finish_navigation(&step, terminal_had_focus);
}

/// Put a session at `location` of `/repo` in `lifecycle` and `activity` (feature 1125).
pub fn session_in(
    state: &mut State,
    location: SessionLocation,
    lifecycle: micold_core::session::SessionLifecycle,
    activity: micold_core::protocol::messages::ActivitySignal,
    paused_on_user: bool,
) -> SessionId {
    let id = session_at(state, location);
    let session = state
        .workspace
        .sessions
        .get_mut(&PathBuf::from("/repo"))
        .and_then(|sessions| sessions.iter_mut().find(|s| s.id == id))
        .expect("just added");
    session.lifecycle = lifecycle;
    session.activity = activity;
    session.paused_on_user = paused_on_user;
    id
}

/// Read pull request `pr` for `branch`, as a pull request reading does (feature 1125).
pub fn pr_read(
    state: &mut State,
    branch: &str,
    pr: micold_core::pull_request::PrState,
    removable: bool,
) {
    state.pr_status.enabled = true;
    state.pr_status.read_at = Some(100);
    state.pr_status.statuses.insert(
        branch.to_string(),
        micold_core::pull_request::PullRequestStatus {
            number: 7,
            title: "t".to_string(),
            url: "https://github.com/o/r/pull/7".to_string(),
            state: pr,
            review: micold_core::pull_request::ReviewState::None,
            head: "abc".to_string(),
            conflict: false,
        },
    );
    if removable {
        state.pr_status.removable.insert(branch.to_string());
    }
}

/// One worktree per card state (feature 1125), listed in this order: `idle` (no session), `done`
/// (a merged, removable pull request), `work` (a running session), `attn` (failing checks) and
/// `act` (a permission prompt). *Default* has no session, so it is Idle.
pub fn five_states() -> State {
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::pull_request::{CheckStatus, PrState};
    use micold_core::session::SessionLifecycle;
    let mut state = state_with_active();
    deliver(
        &mut state,
        ["idle", "done", "work", "attn", "act"]
            .into_iter()
            .map(listed)
            .collect(),
    );
    pr_read(&mut state, "feat/done", PrState::Merged, true);
    session_in(
        &mut state,
        worktree("work"),
        SessionLifecycle::Running,
        ActivitySignal::Working,
        false,
    );
    pr_read(
        &mut state,
        "feat/attn",
        PrState::Open {
            checks: CheckStatus::Failing,
        },
        false,
    );
    session_in(
        &mut state,
        worktree("act"),
        SessionLifecycle::Running,
        ActivitySignal::AwaitingInput,
        true,
    );
    state
}

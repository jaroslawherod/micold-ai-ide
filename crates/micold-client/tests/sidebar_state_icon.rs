//! The state icon on the sidebar's location rows (feature 905, US1; contract `state-icon.md` S3,
//! S4, A1, A7, A8).
//!
//! Each worktree row and the Default row carries its location's state, taken from every
//! un-archived session there by the per-session rules, whether the row is expanded, selected or
//! inside a run group. The state follows the service's catalog snapshots and nothing else.

use micold_client::app::State;
use micold_client::catalog_sync::reconcile_catalog;
use micold_client::features::sidebar::{
    arrange_groups, with_state_line, with_unread_line, worktree_tooltip, DefaultNode, RunRow,
    SidebarEntry, TagFilter, WorktreeNode, DEFAULT_LOCATION_LABEL,
};
use micold_core::daemons::DaemonId;
use micold_core::location_state::{BlockedReason, LocationState};
use micold_core::naming::{ConventionalType, DerivedNames, WorktreeNaming};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
};
use micold_core::runs::{GroupId, NewGroup, Run, RunGroup, RunStatus};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation,
};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::{Path, PathBuf};

const REPO: &str = "/repo";
const FEATURE_X: &str = "feat-x";
const FEATURE_Y: &str = "feat-y";

fn worktree(dir: &str, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: Path::new(REPO).join(".claude/worktrees").join(dir),
        branch: Some(format!("feat/{dir}")),
        status,
        included: false,
    }
}

/// A session at `location` in `lifecycle` reporting `activity`.
fn session_at(
    location: SessionLocation,
    lifecycle: SessionLifecycle,
    activity: ActivitySignal,
) -> Session {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.lifecycle = lifecycle;
    session.activity = activity;
    session
}

fn in_worktree(dir: &str, lifecycle: SessionLifecycle, activity: ActivitySignal) -> Session {
    session_at(
        SessionLocation::Worktree(dir.to_string()),
        lifecycle,
        activity,
    )
}

fn working(dir: &str) -> Session {
    in_worktree(dir, SessionLifecycle::Running, ActivitySignal::Working)
}

fn turn_finished(dir: &str) -> Session {
    in_worktree(
        dir,
        SessionLifecycle::Running,
        ActivitySignal::AwaitingInput,
    )
}

fn failed(dir: &str) -> Session {
    in_worktree(
        dir,
        SessionLifecycle::Failed {
            reason: "gave up after 3 attempts".into(),
            attempts: 3,
        },
        ActivitySignal::Unknown,
    )
}

/// The active project `/repo` with `recorded` worktrees and `sessions`.
fn state_with_worktrees(recorded: Vec<Worktree>, sessions: Vec<Session>) -> State {
    let mut state = State::default();
    let path = PathBuf::from(REPO);
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktree.worktrees = recorded;
    state.workspace.sessions.insert(path, sessions);
    state
}

/// The active project with the valid worktrees `feat-x` and `feat-y`, and `sessions`.
fn state_with(sessions: Vec<Session>) -> State {
    state_with_worktrees(
        vec![
            worktree(FEATURE_X, WorktreeStatus::Valid),
            worktree(FEATURE_Y, WorktreeStatus::Valid),
        ],
        sessions,
    )
}

fn node_of(state: &State, dir: &str) -> Option<WorktreeNode> {
    state.sidebar_entries().into_iter().find_map(|e| match e {
        SidebarEntry::Worktree(node) if node.worktree.dir_name == dir => Some(node),
        _ => None,
    })
}

fn state_of(state: &State, dir: &str) -> Option<LocationState> {
    node_of(state, dir).map(|node| node.state())
}

fn default_of(state: &State) -> DefaultNode {
    state
        .sidebar_entries()
        .into_iter()
        .find_map(|e| match e {
            SidebarEntry::Default(node) => Some(node),
            _ => None,
        })
        .expect("the Default row is listed")
}

/// US1 scenarios 1-3, US2 scenario 2, FR-003, FR-005.
#[test]
fn a_worktree_rows_state_follows_its_session() {
    let state = state_with(vec![working(FEATURE_X), turn_finished(FEATURE_Y)]);
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));
    assert_eq!(
        state_of(&state, FEATURE_Y),
        Some(LocationState::Idle),
        "a session that finished its turn is Idle / waiting"
    );

    let state = state_with(vec![]);
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Idle),
        "a worktree with no session is Idle / waiting"
    );

    let state = state_with(vec![
        in_worktree(
            FEATURE_X,
            SessionLifecycle::Running,
            ActivitySignal::Unknown,
        ),
        failed(FEATURE_Y),
    ]);
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Unknown),
        "an unreadable activity is Unknown, never Idle or Blocked"
    );
    assert_eq!(
        state_of(&state, FEATURE_Y),
        Some(LocationState::Blocked(BlockedReason::Failed))
    );
}

/// US1 scenario 1, A1: expanding or selecting a row does not change its state.
#[test]
fn the_state_is_the_same_expanded_or_collapsed_and_selected_or_not() {
    let a = working(FEATURE_X);
    let id = a.id;
    let mut state = state_with(vec![a, turn_finished(FEATURE_X)]);
    let collapsed = node_of(&state, FEATURE_X).expect("feat-x is listed");
    let mut expanded = collapsed.clone();
    expanded.expanded = !collapsed.expanded;
    assert_eq!(collapsed.state(), LocationState::Running);
    assert_eq!(expanded.state(), collapsed.state());

    state.session.active = Some(id);
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));
    state.session.active = None;
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));
}

fn run(number: u8, dir: &str) -> Run {
    Run {
        number,
        provider: AiCli::ClaudeCode,
        names: DerivedNames {
            dir_name: dir.to_string(),
            branch: format!("feat/{dir}"),
        },
        session: None,
        status: RunStatus::Prompted,
    }
}

fn group(runs: Vec<Run>) -> RunGroup {
    RunGroup::new(NewGroup {
        id: GroupId(uuid::Uuid::from_u128(7)),
        name: "login".into(),
        naming: WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: None,
            name: "login".into(),
        },
        prompt: "p".into(),
        base_branch: "main".into(),
        base_commit: "abc".into(),
        created: std::time::SystemTime::UNIX_EPOCH,
        runs,
    })
    .expect("valid group")
}

/// S3: a run row inside a group carries its own worktree's state.
#[test]
fn a_parallel_run_row_has_its_own_state() {
    let state = state_with(vec![working(FEATURE_X), failed(FEATURE_Y)]);
    let arranged = arrange_groups(
        state.sidebar_entries(),
        &[group(vec![run(1, FEATURE_X), run(2, FEATURE_Y)])],
        |_| true,
    );
    let runs: Vec<LocationState> = arranged
        .iter()
        .filter_map(|e| match e {
            SidebarEntry::Group(g) => Some(g),
            _ => None,
        })
        .flat_map(|g| g.runs.iter())
        .filter_map(|run| match &run.row {
            RunRow::Worktree(node) => Some(node.state()),
            RunRow::NoWorktree => None,
        })
        .collect();
    assert_eq!(
        runs,
        [
            LocationState::Running,
            LocationState::Blocked(BlockedReason::Failed)
        ]
    );
}

/// A worktree the tag filter hides has no row; a missing one keeps its row and its state.
#[test]
fn a_hidden_worktree_has_no_row_and_a_missing_one_has_a_state() {
    let mut state = state_with(vec![working(FEATURE_X)]);
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Fix));
    assert_eq!(state_of(&state, FEATURE_X), None);

    let state = state_with_worktrees(
        vec![worktree("feat-gone", WorktreeStatus::Missing)],
        vec![working("feat-gone")],
    );
    assert_eq!(state_of(&state, "feat-gone"), Some(LocationState::Running));
}

/// FR-002: the Default row follows the project root's sessions by the same rules.
#[test]
fn the_default_rows_state_follows_the_project_roots_sessions() {
    let state = state_with(vec![working(FEATURE_X)]);
    assert_eq!(default_of(&state).state(), LocationState::Idle);

    let state = state_with(vec![
        session_at(
            SessionLocation::Default,
            SessionLifecycle::InterruptedResumable,
            ActivitySignal::Unknown,
        ),
        session_at(
            SessionLocation::Default,
            SessionLifecycle::Running,
            ActivitySignal::Working,
        ),
    ]);
    assert_eq!(
        default_of(&state).state(),
        LocationState::Blocked(BlockedReason::Resumable),
        "Blocked outranks Running"
    );
}

/// A catalog snapshot of `REPO` holding `sessions` as `(id, worktree, lifecycle, activity)`.
fn catalog(sessions: &[(SessionId, &str, WireLifecycle, ActivitySignal)]) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from(REPO)),
        projects: vec![ProjectSnapshot {
            pane_layout: None,
            path: PathBuf::from(REPO),
            display_name: "repo".to_string(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: sessions
                .iter()
                .map(|(id, worktree, lifecycle, activity)| SessionSummary {
                    id: *id,
                    worktree_dir: Some(worktree.to_string()),
                    title: SessionLabel::Pending,
                    lifecycle: lifecycle.clone(),
                    activity: activity.clone(),
                    provider: AiCli::ClaudeCode,
                    kind: Default::default(),
                    permission_mode: Default::default(),
                    input_serial: 0,
                    live_shells: Vec::new(),
                    attention_seq: 1,
                    unread: false,
                    origin: None,
                    agents: Vec::new(),
                    main: false,
                    paused_on_user: false,
                    hook_activity: None,
                })
                .collect(),
        }],
        env_include_failures: Vec::new(),
    }
}

/// US1 scenario 4, FR-006, FR-011, A8, Edge (service restart): the row follows the snapshots, and
/// nothing but the snapshot changes it — no selection change, no notification.
#[test]
fn a_catalog_snapshot_changes_the_state_and_nothing_else() {
    let session = working(FEATURE_X);
    let id = session.id;
    let mut state = state_with(vec![session]);
    let selected_before = state.session.active;
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));

    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &catalog(&[(
            id,
            FEATURE_X,
            WireLifecycle::Running,
            ActivitySignal::AwaitingInput,
        )]),
        false,
    );
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Idle),
        "the turn finished: Running becomes Idle / waiting with no user action"
    );
    assert_eq!(
        state.session.active, selected_before,
        "a state change does not change the selection"
    );

    // The connection is lost: no snapshot arrives, and the row keeps the state it had.
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Idle));

    // The reconnect snapshot carries the service's current state, and replaces it.
    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &catalog(&[(
            id,
            FEATURE_X,
            WireLifecycle::Running,
            ActivitySignal::Working,
        )]),
        false,
    );
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));
}

/// The snapshot of [`catalog`] with every session's `paused_on_user` set to `paused`.
fn paused_catalog(
    sessions: &[(SessionId, &str, WireLifecycle, ActivitySignal)],
    paused: bool,
) -> CatalogSnapshot {
    let mut snapshot = catalog(sessions);
    for summary in &mut snapshot.projects[0].sessions {
        summary.paused_on_user = paused;
    }
    snapshot
}

fn ask_permission(dir: &str) -> Session {
    let mut session = turn_finished(dir);
    session.paused_on_user = true;
    session
}

/// US2 scenarios 1 and 4, W4: a session paused on the user makes its worktree Blocked, on both
/// catalog paths, and the next snapshot's `Working` makes it Running again.
#[test]
fn a_permission_stop_blocks_the_worktree_until_the_session_carries_on() {
    let rows = |id| {
        [(
            id,
            FEATURE_X,
            WireLifecycle::Running,
            ActivitySignal::AwaitingInput,
        )]
    };

    // New-session insert.
    let id = SessionId::new();
    let mut state = state_with(Vec::new());
    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &paused_catalog(&rows(id), true),
        false,
    );
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Blocked(BlockedReason::NeedsPermission))
    );
    let tooltip_line = state_of(&state, FEATURE_X).unwrap().tooltip_line();
    assert_eq!(tooltip_line, "Blocked: needs permission");

    // Existing-session update, then the grant.
    let carried_on = [(
        id,
        FEATURE_X,
        WireLifecycle::Running,
        ActivitySignal::Working,
    )];
    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &paused_catalog(&carried_on, false),
        false,
    );
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Running));
    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &paused_catalog(&rows(id), true),
        false,
    );
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Blocked(BlockedReason::NeedsPermission))
    );
    reconcile_catalog(
        &mut state,
        DaemonId(1),
        &paused_catalog(&rows(id), false),
        false,
    );
    assert_eq!(state_of(&state, FEATURE_X), Some(LocationState::Idle));
}

/// US2 scenario 1: a helper asking for a permission blocks the worktree beside an idle main.
#[test]
fn a_helper_asking_for_a_permission_blocks_beside_an_idle_main_session() {
    let state = state_with(vec![turn_finished(FEATURE_X), ask_permission(FEATURE_X)]);
    assert_eq!(
        state_of(&state, FEATURE_X),
        Some(LocationState::Blocked(BlockedReason::NeedsPermission))
    );
}

/// FR-009, A7: the state's words go on a line of their own, before the unread line.
#[test]
fn the_state_line_follows_the_tooltip_and_precedes_the_unread_line() {
    assert_eq!(
        with_state_line(DEFAULT_LOCATION_LABEL.to_string(), LocationState::Running),
        format!("{DEFAULT_LOCATION_LABEL}\nRunning")
    );
    let tooltip = worktree_tooltip(
        Some(Path::new(REPO)),
        &worktree(FEATURE_X, WorktreeStatus::Valid),
        "x",
        None,
    );
    assert_eq!(
        with_unread_line(
            with_state_line(
                tooltip.clone(),
                LocationState::Blocked(BlockedReason::Failed)
            ),
            2
        ),
        format!("{tooltip}\nBlocked: failed\n2 unread sessions")
    );
    assert_eq!(
        with_state_line(tooltip.clone(), LocationState::Unknown),
        format!("{tooltip}\nState unknown")
    );
}

//! Two daemons' catalog snapshots for one project fold per daemon (feature 491, T021, R11, FR-007).

use std::path::PathBuf;

use micold_client::app::State;
use micold_client::catalog_sync::{reconcile_catalog, session_daemon};
use micold_core::daemons::DaemonId;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
    WorktreeSnapshot, WorktreeStatus as Wire,
};
use micold_core::session::{AiCli, SessionId, SessionLabel, SessionLifecycle};
use micold_core::worktree::WorktreeStatus;

const PROJECT: &str = "/repo";
const A: DaemonId = DaemonId(1);
const B: DaemonId = DaemonId(2);

fn session(id: SessionId, dir: &str) -> SessionSummary {
    SessionSummary {
        id,
        worktree_dir: Some(dir.to_string()),
        title: SessionLabel::Pending,
        lifecycle: WireLifecycle::Running,
        activity: ActivitySignal::Unknown,
        input_serial: 0,
        live_shells: Vec::new(),
        attention_seq: 0,
        unread: false,
        provider: AiCli::ClaudeCode,
    }
}

fn worktree(dir: &str, status: Wire, name: &str, user_created: bool) -> WorktreeSnapshot {
    WorktreeSnapshot {
        dir_name: dir.to_string(),
        branch: Some(format!("feat/{dir}")),
        display_name: name.to_string(),
        status,
        path: PathBuf::from(format!("{PROJECT}/.claude/worktrees/{dir}")),
        included: false,
        user_created,
    }
}

fn snapshot(sessions: Vec<SessionSummary>, worktrees: Vec<WorktreeSnapshot>) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: None,
        projects: vec![ProjectSnapshot {
            pane_layout: None,
            path: PathBuf::from(PROJECT),
            display_name: "repo".into(),
            is_git_repo: true,
            available: true,
            worktrees,
            sessions,
        }],
        env_include_failures: Vec::new(),
    }
}

/// `feat-a` is bound to A, `feat-b` to B; the project is the active one.
fn state() -> State {
    let mut state = State::default();
    let path = PathBuf::from(PROJECT);
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.workspace.bindings.insert(
        path,
        [("feat-a".to_string(), A), ("feat-b".to_string(), B)]
            .into_iter()
            .collect(),
    );
    state
}

fn session_ids(state: &State) -> Vec<SessionId> {
    state.workspace.sessions[&PathBuf::from(PROJECT)]
        .iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn a_sessions_survive_bs_snapshot() {
    let (sa, sb) = (SessionId::new(), SessionId::new());
    let mut state = state();
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(vec![session(sa, "feat-a")], vec![]),
        false,
    );
    reconcile_catalog(
        &mut state,
        B,
        &snapshot(vec![session(sb, "feat-b")], vec![]),
        false,
    );
    let ids = session_ids(&state);
    assert!(ids.contains(&sa), "A's session survives B's snapshot");
    assert!(ids.contains(&sb));
    // And B's own omission still removes B's session.
    reconcile_catalog(&mut state, B, &snapshot(vec![], vec![]), false);
    let ids = session_ids(&state);
    assert!(ids.contains(&sa) && !ids.contains(&sb));
    let project = PathBuf::from(PROJECT);
    let kept = &state.workspace.sessions[&project][0];
    assert_eq!(session_daemon(&state, &project, kept), Some(A));
}

#[test]
fn the_worktree_list_is_the_union_with_status_from_the_bound_daemon() {
    let mut state = state();
    // A sees feat-a valid; it also discovers feat-b, which it reports missing (wrongly: not its).
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(
            vec![],
            vec![
                worktree("feat-a", Wire::Clean, "feat-a", false),
                worktree("feat-b", Wire::Missing, "feat-b", false),
            ],
        ),
        true,
    );
    // B reports feat-b valid, and feat-a locked (not its).
    reconcile_catalog(
        &mut state,
        B,
        &snapshot(
            vec![],
            vec![
                worktree("feat-a", Wire::Locked, "feat-a", false),
                worktree("feat-b", Wire::Clean, "feat-b", false),
            ],
        ),
        true,
    );
    let status = |dir: &str| {
        state
            .worktree
            .worktrees
            .iter()
            .find(|w| w.dir_name == dir)
            .map(|w| w.status)
    };
    assert_eq!(
        state.worktree.worktrees.len(),
        2,
        "the union, no duplicates"
    );
    assert_eq!(status("feat-a"), Some(WorktreeStatus::Valid), "A's word");
    assert_eq!(status("feat-b"), Some(WorktreeStatus::Valid), "B's word");
}

#[test]
fn provenance_and_display_names_are_scoped_to_the_snapshots_daemon() {
    let mut state = state();
    let project = PathBuf::from(PROJECT);
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(vec![], vec![worktree("feat-a", Wire::Clean, "Alpha", true)]),
        true,
    );
    // B's snapshot does not list feat-a at all; its records must remain.
    reconcile_catalog(
        &mut state,
        B,
        &snapshot(vec![], vec![worktree("feat-b", Wire::Clean, "Beta", true)]),
        true,
    );
    assert!(state.workspace.worktree_provenance[&project].contains("feat-a"));
    assert!(state.workspace.worktree_provenance[&project].contains("feat-b"));
    assert_eq!(state.workspace.worktree_names[&project]["feat-a"], "Alpha");
    assert_eq!(state.workspace.worktree_names[&project]["feat-b"], "Beta");
    // A's own later snapshot dropping feat-a removes A's record, and only that.
    reconcile_catalog(&mut state, A, &snapshot(vec![], vec![]), true);
    assert!(!state.workspace.worktree_provenance[&project].contains("feat-a"));
    assert!(state.workspace.worktree_provenance[&project].contains("feat-b"));
    assert!(!state.workspace.worktree_names[&project].contains_key("feat-a"));
    assert_eq!(state.workspace.worktree_names[&project]["feat-b"], "Beta");
}

#[test]
fn a_dropped_daemons_sessions_stay_unfinished() {
    let sa = SessionId::new();
    let mut state = state();
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(vec![session(sa, "feat-a")], vec![]),
        false,
    );
    // Nothing is folded when a daemon drops: the session is neither removed nor finished. What
    // marks it "lost to daemon A" is computed from the link state (see `daemon_version_mismatch`
    // and `worktree_unavailable_label`), so the lifecycle must stay what the daemon last said.
    let project = PathBuf::from(PROJECT);
    let kept = &state.workspace.sessions[&project][0];
    assert_eq!(kept.lifecycle, SessionLifecycle::Running);
    assert_eq!(session_daemon(&state, &project, kept), Some(A));
}

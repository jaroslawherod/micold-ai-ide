//! Two daemons' catalog snapshots for one project fold per daemon (feature 491, T021, R11, FR-007).

use std::path::PathBuf;

use iced::futures::channel::mpsc;
use micold_client::app::State;
use micold_client::catalog_sync::{reconcile_catalog, session_daemon};
use micold_client::daemon::Outbox;
use micold_client::links::DaemonLinks;
use micold_core::daemons::{ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime};
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
        main: false,
        provider: AiCli::ClaudeCode,
        kind: Default::default(),
        permission_mode: Default::default(),
        model: Default::default(),
        effort: Default::default(),
        origin: None,
        agents: Vec::new(),
        paused_on_user: false,
        hook_activity: None,
    }
}

fn worktree(dir: &str, status: Wire, name: &str) -> WorktreeSnapshot {
    WorktreeSnapshot {
        dir_name: dir.to_string(),
        branch: Some(format!("feat/{dir}")),
        display_name: name.to_string(),
        status,
        path: PathBuf::from(format!("{PROJECT}/.claude/worktrees/{dir}")),
        included: false,
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
                worktree("feat-a", Wire::Clean, "feat-a"),
                worktree("feat-b", Wire::Missing, "feat-b"),
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
                worktree("feat-a", Wire::Locked, "feat-a"),
                worktree("feat-b", Wire::Clean, "feat-b"),
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
fn display_names_are_scoped_to_the_snapshots_daemon() {
    let mut state = state();
    let project = PathBuf::from(PROJECT);
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(vec![], vec![worktree("feat-a", Wire::Clean, "Alpha")]),
        true,
    );
    // B's snapshot does not list feat-a at all; A's name must remain.
    reconcile_catalog(
        &mut state,
        B,
        &snapshot(vec![], vec![worktree("feat-b", Wire::Clean, "Beta")]),
        true,
    );
    assert_eq!(state.workspace.worktree_names[&project]["feat-a"], "Alpha");
    assert_eq!(state.workspace.worktree_names[&project]["feat-b"], "Beta");
    // A's own later snapshot dropping feat-a removes A's name, and only that.
    reconcile_catalog(&mut state, A, &snapshot(vec![], vec![]), true);
    assert!(!state.workspace.worktree_names[&project].contains_key("feat-a"));
    assert_eq!(state.workspace.worktree_names[&project]["feat-b"], "Beta");
}

#[test]
fn a_dropped_daemons_sessions_stay_unfinished_and_read_as_lost() {
    let sa = SessionId::new();
    let mut state = state();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    assert_eq!(registry.add("Alpha", DaemonRuntime::Host, true).unwrap(), A);
    assert_eq!(
        registry
            .add(
                "Beta",
                DaemonRuntime::Container(ContainerSettings {
                    profile: Default::default(),
                    container_name: "beta".into(),
                    port: 7001,
                    migration: None,
                }),
                true,
            )
            .unwrap(),
        B,
    );
    state.settings.links = DaemonLinks::at_launch(&registry);
    state.settings.daemons = registry;
    for id in [A, B] {
        state
            .settings
            .links
            .connected(id, Outbox::new(mpsc::unbounded().0));
    }
    reconcile_catalog(
        &mut state,
        A,
        &snapshot(vec![session(sa, "feat-a")], vec![]),
        false,
    );
    assert_eq!(state.daemon_row_label("feat-a"), "Alpha");

    // The real drop: the link goes to reconnecting. Nothing is folded into the session, which stays
    // what the daemon last said; the row reads "disconnected" from the link state alone (FR-019).
    state.settings.links.lost(A, "gone");

    let project = PathBuf::from(PROJECT);
    let kept = &state.workspace.sessions[&project][0];
    assert_eq!(kept.lifecycle, SessionLifecycle::Running);
    assert_eq!(session_daemon(&state, &project, kept), Some(A));
    assert_eq!(state.daemon_row_label("feat-a"), "disconnected");
    assert_eq!(state.daemon_row_label("feat-b"), "Beta", "B is unaffected");
}

/// Bug 1467: a location with no stored binding is the legacy default's (A here), which is where the
/// router sends its sessions. B lists the project too (it has `feat-b`, and since #1423 a create
/// registers the project there), so B's snapshots, which never hold A's unbound sessions and may
/// discover fewer worktrees, must not drop them. Each of B's pushes did, and A's next one put them
/// back: the sidebar's rows disappeared and reappeared while the sessions ran.
#[test]
fn unbound_locations_survive_another_daemons_snapshots_and_reconnects() {
    use micold_client::catalog_sync::note_lost_sessions;
    let (s_default, s_c, sa, sb) = (
        SessionId::new(),
        SessionId::new(),
        SessionId::new(),
        SessionId::new(),
    );
    let mut state = state();
    let project = PathBuf::from(PROJECT);
    let a_snapshot = || {
        snapshot(
            vec![
                SessionSummary {
                    worktree_dir: None,
                    ..session(s_default, "")
                },
                session(s_c, "feat-c"),
                session(sa, "feat-a"),
            ],
            vec![
                worktree("feat-a", Wire::Clean, "feat-a"),
                worktree("feat-b", Wire::Clean, "feat-b"),
                worktree("feat-c", Wire::Clean, "feat-c"),
            ],
        )
    };
    // B holds only its own session, and its discovery found only its own worktree.
    let b_snapshot = || {
        snapshot(
            vec![session(sb, "feat-b")],
            vec![worktree("feat-b", Wire::Clean, "feat-b")],
        )
    };
    let assert_full = |state: &State, step: &str| {
        let ids = session_ids(state);
        for (id, what) in [
            (s_default, "the Default location's"),
            (s_c, "unbound feat-c's"),
            (sa, "feat-a's"),
            (sb, "feat-b's"),
        ] {
            assert!(ids.contains(&id), "{step}: {what} session left the list");
        }
        let mut dirs: Vec<&str> = state
            .worktree
            .worktrees
            .iter()
            .map(|w| w.dir_name.as_str())
            .collect();
        dirs.sort_unstable();
        assert_eq!(dirs, ["feat-a", "feat-b", "feat-c"], "{step}: worktrees");
    };

    // Welcome from A, then A's first push (the welcome does not sync worktrees).
    note_lost_sessions(&mut state, A, &a_snapshot());
    reconcile_catalog(&mut state, A, &a_snapshot(), false);
    reconcile_catalog(&mut state, A, &a_snapshot(), true);
    // B's welcome, B's pushes interleaved with A's, and B reconnecting.
    note_lost_sessions(&mut state, B, &b_snapshot());
    reconcile_catalog(&mut state, B, &b_snapshot(), false);
    assert_full(&state, "after B's welcome");
    for round in 0..3 {
        reconcile_catalog(&mut state, B, &b_snapshot(), true);
        assert_full(&state, &format!("after B's push {round}"));
        reconcile_catalog(&mut state, A, &a_snapshot(), true);
        assert_full(&state, &format!("after A's push {round}"));
    }
    note_lost_sessions(&mut state, B, &b_snapshot());
    reconcile_catalog(&mut state, B, &b_snapshot(), false);
    reconcile_catalog(&mut state, B, &b_snapshot(), true);
    assert_full(&state, "after B reconnected");
    assert!(
        state.settings.lost_sessions.is_empty(),
        "nothing is lost: every session is still listed by its own daemon"
    );

    // A's own omission still removes A's unbound session and worktree.
    let mut fewer = a_snapshot();
    fewer.projects[0]
        .sessions
        .retain(|s| s.id != s_default && s.id != s_c);
    fewer.projects[0]
        .worktrees
        .retain(|w| w.dir_name != "feat-c");
    reconcile_catalog(&mut state, A, &fewer, true);
    let ids = session_ids(&state);
    assert!(
        !ids.contains(&s_default) && !ids.contains(&s_c),
        "A archived them"
    );
    assert!(
        !state
            .worktree
            .worktrees
            .iter()
            .any(|w| w.dir_name == "feat-c"),
        "A removed feat-c"
    );
    assert!(state.workspace.sessions[&project]
        .iter()
        .any(|s| s.id == sb));
}

/// Bug 1538: B reports a session at a location with no stored binding, which the router reads as
/// the legacy default's (A here). Its row is B's to keep and B's to drop: A's snapshots, which never
/// list it, must not remove it (B's next push brought it back), and B's own omission must.
#[test]
fn another_daemons_session_at_an_unbound_location_stays_while_it_lists_it() {
    use micold_client::catalog_sync::note_lost_sessions;
    let (sa, s_u) = (SessionId::new(), SessionId::new());
    let mut state = state();
    let a_snapshot = || snapshot(vec![session(sa, "feat-a")], vec![]);
    let b_snapshot = || snapshot(vec![session(s_u, "feat-u")], vec![]);
    let listed = |state: &State, step: &str| {
        assert!(
            session_ids(state).contains(&s_u),
            "{step}: B's unbound session left the list"
        );
        assert!(
            !state.settings.lost_sessions.contains(&s_u),
            "{step}: B still lists it, so it is not lost"
        );
    };

    note_lost_sessions(&mut state, A, &a_snapshot());
    reconcile_catalog(&mut state, A, &a_snapshot(), false);
    note_lost_sessions(&mut state, B, &b_snapshot());
    reconcile_catalog(&mut state, B, &b_snapshot(), false);
    listed(&state, "after B's welcome");
    for round in 0..3 {
        reconcile_catalog(&mut state, A, &a_snapshot(), true);
        listed(&state, &format!("after A's push {round}"));
        reconcile_catalog(&mut state, B, &b_snapshot(), true);
        listed(&state, &format!("after B's push {round}"));
    }
    // A reconnects: its welcome does not list B's session, which is not A's to call lost.
    note_lost_sessions(&mut state, A, &a_snapshot());
    reconcile_catalog(&mut state, A, &a_snapshot(), false);
    listed(&state, "after A reconnected");
    note_lost_sessions(&mut state, B, &b_snapshot());
    reconcile_catalog(&mut state, B, &b_snapshot(), false);
    listed(&state, "after B reconnected");

    // B restarts without it: the live row stays, lost to B rather than dropped (FR-019).
    note_lost_sessions(&mut state, B, &snapshot(vec![], vec![]));
    reconcile_catalog(&mut state, B, &snapshot(vec![], vec![]), false);
    assert!(session_ids(&state).contains(&s_u), "kept while lost");
    assert!(state.settings.lost_sessions.contains(&s_u), "lost to B");
    reconcile_catalog(&mut state, B, &b_snapshot(), true);
    listed(&state, "after B listed it again");

    // B pushes a catalog that has not loaded the project yet: what it listed there still stands.
    let unloaded = CatalogSnapshot {
        projects: vec![],
        ..b_snapshot()
    };
    reconcile_catalog(&mut state, B, &unloaded, true);
    reconcile_catalog(&mut state, A, &a_snapshot(), true);
    listed(&state, "after B omitted the project");

    // B stops listing it (archived there): the row leaves, and A's next push keeps it gone.
    reconcile_catalog(&mut state, B, &snapshot(vec![], vec![]), true);
    assert!(!session_ids(&state).contains(&s_u), "B archived it");
    reconcile_catalog(&mut state, A, &a_snapshot(), true);
    let ids = session_ids(&state);
    assert!(!ids.contains(&s_u) && ids.contains(&sa));
}

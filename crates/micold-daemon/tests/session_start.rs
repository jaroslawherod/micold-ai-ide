//! Phase 4 (US2) — `ClientMsg::SessionStart` brings a durable session to life: the daemon spawns its
//! process from the catalog (cwd from the session's location, mode = which process) and adopts it
//! into the live registry, so a client can then view and drive it (FR-006, data-model §Session).
//!
//! Uses a Regular (shell) session so the test spawns the platform shell — no `claude` binary needed.
//! The AI-CLI spawn path is compile-covered by the same `start_session` code.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::io::Write;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use uuid::Uuid;

/// The visible screen as one string, for content assertions (mirrors `drive_loop.rs`'s helper).
fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let (cols, rows) = (grid.columns(), grid.screen_lines());
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

/// A catalog holding one Regular-mode session at the project root, whose path is a real directory so
/// the spawned shell can `cwd` into it.
fn catalog_with_shell_session(
    project_dir: &std::path::Path,
    store_dir: &std::path::Path,
) -> Catalog {
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_dir.to_path_buf(), vec![session]);

    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };

    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

#[test]
fn session_start_spawns_and_registers_a_durable_session() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    assert!(
        state.live_session(id).is_none(),
        "the session is not live until started"
    );

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");

    let live = state
        .live_session(id)
        .expect("the session is now in the registry");
    assert!(
        wait_until(Duration::from_secs(5), || live.is_alive()),
        "the spawned shell process must be running"
    );

    // Idempotent: a second Start is a no-op, not a double-spawn.
    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("a redundant start is a no-op");
    assert!(state.live_session(id).is_some());

    // Test-owned process: stop it so nothing leaks.
    live.kill().expect("kill");
}

/// T110/FR-028a (BUG-006): the catalog snapshot must publish each live session's expected input
/// serial, so a client process that did not start the session can resume its counter there.
///
/// This is the wire half of the fix; `micold-core`'s `input_ordering` tests prove what a client does
/// with the number once it has it. Before the fix the field did not exist, a restarted UI stamped
/// `0` into a receiver already at `N`, and every keystroke was discarded as stale.
#[test]
fn the_snapshot_publishes_a_live_sessions_expected_input_serial() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    // The id `catalog_with_shell_session` records.
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    let summary_for = |state: &DaemonState| {
        state
            .welcome_payload()
            .0
            .projects
            .into_iter()
            .flat_map(|p| p.sessions)
            .find(|s| s.id == id)
            .expect("the session is in the snapshot")
    };

    // Not yet hosted: no receiver exists, so the catalog's default stands — and it is the right
    // answer, since the daemon has accepted no input and a client starting at 0 is in step.
    assert_eq!(
        summary_for(&state).input_serial,
        0,
        "a session with no live entry reports 0"
    );

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");
    let live = state.live_session(id).expect("registered");

    // Drive some input, exactly as a client would.
    for serial in 0..7 {
        state.session_input(id, serial, b"");
    }

    assert_eq!(
        summary_for(&state).input_serial,
        7,
        "the snapshot reports the receiver's high-water mark, not the durable record's"
    );

    // A stale serial is dropped and must not move the published mark — otherwise a client would
    // resume behind the daemon and lose input all over again.
    state.session_input(id, 2, b"");
    assert_eq!(summary_for(&state).input_serial, 7);

    live.kill().expect("kill");
}

/// BUG-003: a session the daemon spawns must see variables sourced from the configured
/// environment-include script (feature 011) — the same as a `micold-client`-spawned session would.
/// Before the fix, all three of the daemon's spawn sites (`start_session`, `respawn_primary`,
/// `open_shell`) hardcoded `env = vec![("TERM", ...)]` and never called
/// `micold_core::env_include::resolve`, so no such variable could ever reach the spawned process
/// regardless of what the configured script exported.
#[test]
fn a_daemon_spawned_session_sees_env_include_resolved_variables() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();

    // A plain, unconditional export — no interactive guard (BUG-001), no directory-dependent hook
    // (BUG-002): the simplest case env-include is supposed to handle unconditionally.
    let script_path = project.path().join("env-include-script.sh");
    std::fs::File::create(&script_path)
        .unwrap()
        .write_all(b"export BUG003_MARKER=daemon_env_include_works\n")
        .unwrap();

    JsonFileSettingsStore::at(store.path().join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script_path.to_string_lossy().into_owned(),
            ..Settings::default()
        })
        .unwrap();

    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");
    let live = state.live_session(id).expect("session is now live");
    assert!(
        wait_until(Duration::from_secs(5), || live.is_alive()),
        "the spawned shell process must be running"
    );

    // Drive the live shell to echo the variable back, proving it is actually in the spawned
    // process's own environment (not just resolvable in the abstract).
    state.session_input(id, 0, b"echo SEEN:$BUG003_MARKER\n");
    assert!(
        wait_until(Duration::from_secs(5), || visible_text(&live)
            .contains("SEEN:daemon_env_include_works")),
        "the daemon-spawned session must see the env-include-resolved variable:\n{}",
        visible_text(&live)
    );

    live.kill().expect("kill");
}

#[test]
fn starting_an_unknown_session_is_an_error_not_a_panic() {
    let store = tempfile::tempdir().unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));
    let err = state
        .start_session(SessionId::new(), micold_core::terminal::LaunchMode::Resume)
        .expect_err("an unknown session cannot be started");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn create_session_adds_a_daemon_owned_session_to_the_catalog() {
    use micold_core::project::{Availability, Project};
    use micold_core::store::ProjectStore;
    use micold_core::workspace::Workspace;

    let project = std::path::PathBuf::from("/repo/alpha");
    let store = tempfile::tempdir().unwrap();
    // Seed a catalog with a project but no sessions.
    let workspace = Workspace {
        projects: vec![Project::new(project.clone(), true, Availability::Available)],
        active: Some(project.clone()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));

    // The daemon assigns the id and records the session at the project root (empty worktree_dir).
    let id = state
        .create_session(&project, "")
        .expect("create must succeed");

    let snapshot = state.welcome_payload().0;
    let proj = snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .expect("project in snapshot");
    assert_eq!(proj.sessions.len(), 1, "the created session appears");
    assert_eq!(proj.sessions[0].id, id);
    assert_eq!(
        proj.sessions[0].worktree_dir, None,
        "an empty worktree_dir is the Default (root) location"
    );
}

/// BUG-003 (`006-real-terminal-emulator` FR-014/FR-014a): a size reported for a session that is not
/// live yet must be *remembered* and used to seed the spawn — not dropped.
///
/// Before the fix, `SessionResize` only walked `session_ptys`, which is empty until the process
/// exists, and every spawn site passed `initial_size: None`. So a session started right after the
/// client reported its pane size came up at the 100×30 seed and stayed there — the whole visible
/// symptom: a terminal occupying a fixed patch of an otherwise empty pane until the next window
/// resize happened to produce a fresh `SessionResize`.
#[test]
fn a_session_spawns_at_the_size_last_requested_for_it() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    // The client reports the pane size while the session has no process at all.
    state.resize_session(id, 220, 60);

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");
    let live = state.live_session(id).expect("registered");

    let (cols, rows) = {
        let term = live.term().lock();
        (term.grid().columns(), term.grid().screen_lines())
    };
    assert_eq!(
        (cols, rows),
        (220, 60),
        "the spawn must adopt the last requested size, not the 100×30 seed"
    );

    // A second terminal instance is displayed in the same pane, so it starts at the same size —
    // `open_shell` is the third spawn site and had the same hardcoded `None`.
    let instance = micold_core::session::ShellInstanceId(0);
    state
        .open_shell(id, instance)
        .expect("open a shell instance");
    for pty in state.session_ptys(id) {
        let term = pty.term().lock();
        assert_eq!(
            (term.grid().columns(), term.grid().screen_lines()),
            (220, 60),
            "every one of the session's processes starts at the recorded size"
        );
    }

    for pty in state.session_ptys(id) {
        pty.kill().expect("kill");
    }
    drop(live);
}

// ---------------------------------------------------------------------------------------
// BUG-011 — a session whose process the daemon has started must be reported as running.
//
// These sit at the seam nothing covered. Every existing lifecycle test drives the FSM directly
// (`session_lifecycle.rs`, `session_crash_restart.rs`, `supervision_restart.rs` all call `start()`
// and `mark_running()` themselves), which is why the state machine is provably correct and was
// provably unreached: `Session::start` had no production callers at all, and `mark_running` had one
// — `mark_running_if_restarting`, gated to `Restarting`. So they assert on **the snapshot a client
// would receive**, not on the record.
// ---------------------------------------------------------------------------------------

/// The reported lifecycle for `id`, as a connected client would read it.
fn reported_lifecycle(state: &DaemonState, id: SessionId) -> WireLifecycle {
    state
        .welcome_payload()
        .0
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .expect("the session is in the snapshot")
        .lifecycle
}

#[test]
fn the_snapshot_reports_a_started_session_as_running() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    assert_ne!(
        reported_lifecycle(&state, id),
        WireLifecycle::Running,
        "nothing has been started yet"
    );

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");
    let live = state.live_session(id).expect("registered");
    assert!(wait_until(Duration::from_secs(5), || live.is_alive()));

    assert_eq!(
        reported_lifecycle(&state, id),
        WireLifecycle::Running,
        "a session the daemon is hosting must be reported as running — before this, the durable \
         record kept whatever it held before the spawn, so the bar read `interrupted` (or \
         `starting…`) beside a live terminal and offered `restart` for a running agent"
    );

    live.kill().expect("kill");
}

#[test]
fn starting_a_session_announces_the_new_lifecycle_to_connected_clients() {
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::DaemonMsg;

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));
    let (_client, mut rx) = state.register("test".to_string());

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the shell session");
    let live = state.live_session(id).expect("registered");
    assert!(wait_until(Duration::from_secs(5), || live.is_alive()));

    // `SessionStart` carries no reply, and the only `broadcast_catalog` on that path used to sit
    // inside `if let Some(reply)` — so a resume changed the world and told nobody. A client that
    // never re-attaches would keep the stale value forever.
    let mut announced = None;
    while let Ok(frame) = rx.try_recv() {
        if let Frame::Control(DaemonMsg::CatalogChanged { catalog }) = frame {
            if let Some(summary) = catalog
                .projects
                .iter()
                .flat_map(|p| &p.sessions)
                .find(|s| s.id == id)
            {
                announced = Some(summary.lifecycle.clone());
            }
        }
    }
    assert_eq!(
        announced,
        Some(WireLifecycle::Running),
        "starting a session must broadcast the catalog, carrying the session as running"
    );

    live.kill().expect("kill");
}

/// The headline case, at the catalog level so it needs no `claude` on `PATH`: a session presented
/// as interrupted-resumable at service startup (FR-006a) must leave that state once the user's one
/// explicit action has actually started it. FR-006a said how a session *enters*; nothing said how
/// it leaves, which is the gap BUG-011 sits in.
#[test]
fn a_resumed_session_leaves_the_interrupted_resumable_state() {
    use micold_core::session::SessionLifecycle;

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0xA11E));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Agent".into()),
        TerminalMode::AiCli,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.path().to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.path().to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.path().to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    let mut catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    );

    // As the daemon does at startup for a session with a recorded conversation.
    assert_eq!(catalog.present_interrupted_resumable(|_, _, _| true), 1);
    let lifecycle_of = |catalog: &Catalog| {
        catalog
            .workspace()
            .sessions
            .values()
            .flatten()
            .find(|s| s.id == id)
            .expect("session")
            .lifecycle
    };
    assert_eq!(
        lifecycle_of(&catalog),
        SessionLifecycle::InterruptedResumable
    );

    let owner = catalog.mark_session_running(id);
    assert_eq!(
        owner,
        Some(project.path().to_path_buf()),
        "the transition names its project, so the caller knows what to broadcast"
    );
    assert_eq!(lifecycle_of(&catalog), SessionLifecycle::Running);

    // Already running: no transition, so nothing to announce. A steady `Running` must not
    // re-broadcast on every redundant start.
    assert_eq!(catalog.mark_session_running(id), None);
    assert_eq!(lifecycle_of(&catalog), SessionLifecycle::Running);

    // An unknown id is not a panic.
    assert_eq!(catalog.mark_session_running(SessionId::new()), None);
}

//! Phase 4 (US2) — `ClientMsg::SessionStart` brings a durable session to life: the daemon spawns its
//! process from the catalog (cwd from the session's location, mode = which process) and adopts it
//! into the live registry, so a client can then view and drive it (FR-006, data-model §Session).
//!
//! Uses a Regular (shell) session so the test spawns the platform shell — no `claude` binary needed.
//! The AI-CLI spawn path is compile-covered by the same `start_session` code.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
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
        AiCli::ClaudeCode,
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
        .create_session(&project, "", AiCli::ClaudeCode)
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
        micold_core::session::AiCli::ClaudeCode,
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
    assert_eq!(catalog.present_interrupted_resumable(|_, _, _, _| true), 1);
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
// Feature 026 (T024) — a created session runs the CLI the client asked for
// ---------------------------------------------------------------------------------------

/// What this can and cannot assert, stated plainly.
///
/// It cannot spawn either CLI. This file's own module doc says why it uses a Regular shell session:
/// CI runners have no `claude`, and adding `copilot` to that list would make the whole suite
/// dependent on two vendors' installers. The property "spawning uses `spec.provider`" is held
/// structurally instead — `PtySession::spawn_ai_cli` builds its `CommandBuilder` from
/// `spec.provider.provider().command()` and its arguments from `terminal::launch_args(spec)`, and
/// `micold-core/tests/terminal_backend.rs` pins what those produce for each provider.
///
/// What it *can* assert is the half that only exists here: the provider the client sent survives
/// the three hops from the wire to the durable record, and comes back out on the snapshot the
/// sidebar reads. That is where a `SessionCreate` carrying `Copilot` would have quietly become a
/// Claude session — the daemon's `create_session` used to name no provider at all.
#[test]
fn a_created_session_records_the_cli_the_client_chose() {
    let store = tempfile::tempdir().unwrap();
    let project = std::path::PathBuf::from("/repo/alpha");
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

    let claude = state
        .create_session(&project, "", AiCli::ClaudeCode)
        .expect("create must succeed");
    let copilot = state
        .create_session(&project, "feat-x", AiCli::Copilot)
        .expect("create must succeed");

    let summaries = state.sessions_for(&project);
    let provider_of = |id: SessionId| {
        summaries
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.provider)
            .expect("session in the snapshot")
    };
    assert_eq!(provider_of(claude), AiCli::ClaudeCode);
    assert_eq!(
        provider_of(copilot),
        AiCli::Copilot,
        "the daemon recorded what the client resolved, and did not re-decide it"
    );

    // And the argv each record implies is that CLI's, through the same function the spawn uses.
    let argv = |id: SessionId, provider: AiCli| {
        micold_core::terminal::launch_args(&micold_core::terminal::LaunchSpec {
            cwd: project.clone(),
            session_id: id.0,
            provider,
            mode: micold_core::terminal::LaunchMode::Fresh,
            env: Vec::new(),
        })
    };
    assert_eq!(
        argv(claude, provider_of(claude)),
        vec!["--session-id".to_string(), claude.0.to_string()]
    );
    assert_eq!(
        argv(copilot, provider_of(copilot)),
        vec![
            "--session-id".to_string(),
            copilot.0.to_string(),
            "--no-remote".to_string()
        ],
        "the Copilot session would be spawned with Copilot's argv, `--no-remote` included"
    );
}

// ---------------------------------------------------------------------------------------
// T046 [US2] — resuming a conversation the CLI no longer has (Clarifications 2026-08-16)
// ---------------------------------------------------------------------------------------

/// Serialises the tests that reach for `PATH` and `COPILOT_HOME`.
///
/// Both are process-global and this binary runs its tests on threads, so two of them installing
/// stubs at once would restore each other's `PATH` on drop and leave the survivor looking at a
/// directory that no longer exists. Held for the lifetime of a [`StubOnPath`], which is also the
/// lifetime of the `COPILOT_HOME` each of these tests sets.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A scratch `PATH` with the real one still behind it, plus a stub for one command.
///
/// The stub is there first of all so `AiCliProvider::is_available` says yes — `start_session`
/// checks that before anything else (FR-010), and on a runner with no `copilot` these tests would
/// be asserting the missing-CLI message instead of their own. It also **records the argv it was
/// given**, so a test can say what the daemon ran, how it ran it, and how many times. The real
/// `PATH` stays appended so the shell-session tests running beside these still find `sh`.
struct StubOnPath {
    _dir: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    log: std::path::PathBuf,
    /// Dropped last (declaration order), after `Drop for StubOnPath` has put `PATH` back.
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl StubOnPath {
    /// A stub that succeeds — for tests where what matters is that it was (or was not) run.
    fn new(command: &str) -> Self {
        Self::with_body(command, "exit 0")
    }

    /// A stub whose body is `body`, so a test can make the CLI refuse the way a real one would.
    fn with_body(command: &str, body: &str) -> Self {
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("argv.log");
        let stub = dir.path().join(command);
        // One line per invocation, so "was it run twice?" is a question the test can ask.
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n{body}\n",
                log.display()
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let previous = std::env::var_os("PATH");
        let joined = match &previous {
            Some(existing) => format!("{}:{}", dir.path().display(), existing.to_string_lossy()),
            None => dir.path().display().to_string(),
        };
        std::env::set_var("PATH", joined);
        Self {
            _dir: dir,
            previous,
            log,
            _guard: guard,
        }
    }

    /// One line per invocation, in order. Empty if the stub was never run.
    fn invocations(&self) -> Vec<String> {
        match std::fs::read_to_string(&self.log) {
            Ok(text) => text.lines().map(str::to_string).collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl Drop for StubOnPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// Resuming a Copilot session whose conversation is gone reports it and starts **nothing**.
///
/// The clarification chose this over the alternative it was specified against: "a clearly-reported
/// failure *or* a fresh session". A fresh session is what `--session-id` would give — a brand-new,
/// empty conversation running under the old session's identity, behind a row whose recorded title
/// still describes the conversation that is not there. That is worse than an error, because nothing
/// about the row says what happened.
///
/// Reported through the path a missing CLI already takes (FR-010): a reason on the session's
/// summary, `attempts: 0`, and no spawn. The reason names the CLI in its **display** register —
/// this is a sentence a user reads, and "copilot no longer has…" reads as a shell error rather than
/// as something that happened to their conversation.
#[test]
fn resuming_a_conversation_the_cli_no_longer_has_reports_it_and_starts_nothing() {
    let _stub = StubOnPath::new("copilot");

    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());

    let store = tempfile::tempdir().unwrap();
    let project = std::path::PathBuf::from("/repo/alpha");
    let id = SessionId::from_uuid(Uuid::from_u128(0xC0FFEE));
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project.clone(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("Refactor the parser".into()),
            TerminalMode::AiCli,
            AiCli::Copilot,
        )],
    );
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));

    // The conversation exists first, and the application learns of it the way it really does — the
    // startup pass, asking Copilot. The order is the point: the edge case is a store entry that was
    // **removed**, and the refusal below applies only to a session the daemon has already judged
    // resumable. A session the daemon never saw a conversation for keeps FR-008's ordinary
    // behaviour — attempt the resume, and report whatever the CLI says (T046a).
    let session_dir = home.path().join("session-state").join(id.0.to_string());
    std::fs::create_dir_all(&session_dir).unwrap();
    std::fs::write(session_dir.join("events.jsonl"), "{}\n").unwrap();
    assert_eq!(
        state.present_interrupted_resumable_at_startup(),
        1,
        "the session is offered for resume — the state a user clicks Start from"
    );

    // …and then it is gone: `copilot` dropping the conversation, or a user clearing out their
    // Copilot home. Nothing tells the application; the row still reads "Refactor the parser".
    std::fs::remove_dir_all(&session_dir).unwrap();

    let result = state.start_session(id, micold_core::terminal::LaunchMode::Resume);

    assert!(
        result.is_err(),
        "the start has to fail: a caller that got `Ok` would tell the client the session is coming \
         up, and nothing is"
    );
    assert!(
        state.live_session(id).is_none(),
        "and nothing was spawned — not an empty terminal, and not a fresh conversation"
    );

    // Read from the snapshot a client actually receives, not from `sessions_for` — the reason is
    // runtime state, overlaid onto the durable record on the way out (FR-010's path).
    let summary = state
        .catalog_snapshot()
        .projects
        .into_iter()
        .find(|p| p.path == project)
        .expect("the project is still there")
        .sessions
        .into_iter()
        .find(|s| s.id == id)
        .expect("the session is still in the catalog — reporting is not closing it");
    let WireLifecycle::Failed { reason, attempts } = summary.lifecycle else {
        panic!("expected a reported failure, got {:?}", summary.lifecycle);
    };
    assert!(
        reason.contains("GitHub Copilot"),
        "the reason names the CLI in the register a sentence wants, got {reason:?}"
    );
    assert!(
        reason.to_lowercase().contains("conversation"),
        "and says what is missing, so the row explains itself rather than just going red: {reason:?}"
    );
    assert_eq!(
        attempts, 0,
        "a conversation that is gone is not a crash loop — retrying it three times would spend the \
         budget on something that cannot change and make the message arrive late (FR-010's rule)"
    );

    // The negative that matters most: no conversation was begun under this id.
    assert!(
        !session_dir.exists(),
        "resuming a conversation that is gone must not create one"
    );
}

/// The other half of the same rule: a session that never had a conversation is **not** told one is
/// gone.
///
/// The refusal above is gated on the record having been marked resumable — a conversation was found
/// for it at startup. Ungated, it would fire for every durable session a resume is asked of,
/// including one created and never used: no `events.jsonl` exists for that either, and the row
/// would report "GitHub Copilot no longer has this conversation" about a conversation that never
/// existed. A message that is false is worse than the CLI's own error, which at least describes
/// what actually happened.
///
/// So the gate is behaviour, not an optimisation, and this test is what says so — dropping
/// `plan.resumable` from the condition passes every other test in the workspace.
#[test]
fn a_session_that_never_recorded_a_conversation_is_not_told_its_conversation_is_gone() {
    let _stub = StubOnPath::new("copilot");

    // Empty, and it stays empty: this session was created and never used, which is exactly what
    // "no recorded conversation" looks like for a session that had nothing to lose.
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());

    let store = tempfile::tempdir().unwrap();
    // A directory that exists, unlike the test above — this session gets as far as a real spawn.
    let project_dir = tempfile::tempdir().unwrap();
    let project = project_dir.path().to_path_buf();
    let id = SessionId::from_uuid(Uuid::from_u128(0xDECAF));
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project.clone(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Pending,
            TerminalMode::AiCli,
            AiCli::Copilot,
        )],
    );
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    ));

    // The startup pass finds nothing to offer — the record stays `Idle`, which is the state this
    // test is about.
    assert_eq!(
        state.present_interrupted_resumable_at_startup(),
        0,
        "there is no conversation to offer"
    );

    let result = state.start_session(id, micold_core::terminal::LaunchMode::Resume);

    assert!(
        result.is_ok(),
        "the spawn is attempted: whether `copilot` can resume this id is the CLI's answer to give, \
         not ours to pre-empt — got {result:?}"
    );
    let live = state.live_session(id).expect("the process was started");
    let summary = state
        .catalog_snapshot()
        .projects
        .into_iter()
        .find(|p| p.path == project)
        .expect("the project is still there")
        .sessions
        .into_iter()
        .find(|s| s.id == id)
        .expect("the session is still in the catalog");
    assert!(
        !matches!(summary.lifecycle, WireLifecycle::Failed { .. }),
        "nothing was reported about a conversation this session never had, got {:?}",
        summary.lifecycle
    );

    let _ = live.kill();
}

// ---------------------------------------------------------------------------------------
// T046a [US2] — a conversation another process may already hold (FR-008, amended 2026-08-18)
// ---------------------------------------------------------------------------------------

/// A Copilot session with a recorded conversation, in a real directory, already offered for resume.
///
/// The same shape as a session discovered under FR-014: the conversation is on disk, the record is
/// `InterruptedResumable`, and **nothing distinguishes it from one a `copilot` is attached to in
/// another terminal** — which is the whole difficulty the tests below are about.
fn offered_copilot_session(
    home: &Path,
    project: &Path,
    store: &Path,
    id: SessionId,
) -> DaemonState {
    let session_dir = home.join("session-state").join(id.0.to_string());
    std::fs::create_dir_all(&session_dir).unwrap();
    std::fs::write(session_dir.join("events.jsonl"), "{}\n").unwrap();

    let mut sessions = BTreeMap::new();
    sessions.insert(
        project.to_path_buf(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("Refactor the parser".into()),
            TerminalMode::AiCli,
            AiCli::Copilot,
        )],
    );
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project.to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    ));
    assert_eq!(
        state.present_interrupted_resumable_at_startup(),
        1,
        "the conversation is there, so the session is offered"
    );
    state
}

/// Every path under `root`, so a test can say "nothing was written here" rather than "the one path
/// I thought to check is absent".
fn tree(root: &Path) -> std::collections::BTreeSet<std::path::PathBuf> {
    let mut out = std::collections::BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(tree(&path));
        }
        out.insert(path);
    }
    out
}

/// Resuming a conversation another terminal may already hold is attempted like any other resume.
///
/// The clarification (2026-08-18) settled this against the tempting alternative: check first, and
/// refuse if the conversation looks busy. Research found neither CLI writes a lock or a liveness
/// marker, so such a check could only be a guess — and a guess that says "in use" when it is not
/// blocks a resume the user is entitled to, with no way for them to override it.
///
/// So the assertions here are mostly negative, and they are the point of the test: the daemon runs
/// the CLI **once**, with the ordinary resume argv, and writes nothing of its own into the
/// provider's store on the way. No lock file, no sentinel, no second invocation to ask whether the
/// conversation is free.
#[test]
fn resuming_a_conversation_another_terminal_may_hold_is_attempted_like_any_other() {
    let stub = StubOnPath::new("copilot");

    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());
    let store = tempfile::tempdir().unwrap();
    let project_dir = tempfile::tempdir().unwrap();
    let project = project_dir.path().to_path_buf();
    let id = SessionId::from_uuid(Uuid::from_u128(0xB0A7));
    let state = offered_copilot_session(home.path(), &project, store.path(), id);

    let before = tree(home.path());
    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect("the resume is attempted — whether the conversation is free is the CLI's answer");
    assert!(
        state.live_session(id).is_some(),
        "the process was started; nothing pre-empted it"
    );

    assert!(
        wait_until(Duration::from_secs(5), || !stub.invocations().is_empty()),
        "the CLI must actually run for the rest of this test to be testing anything"
    );
    assert_eq!(
        stub.invocations(),
        vec![format!("--resume={} --no-remote", id.0)],
        "run once, with the resume every Copilot session gets — not a probe run first, and not a \
         different command because the conversation might be busy"
    );
    assert_eq!(
        tree(home.path()),
        before,
        "and the daemon wrote nothing of its own into Copilot's store: no lock, no in-use marker, \
         nothing a second application would have to learn to respect"
    );
}

/// …and when the CLI refuses, that is reported and nothing is left running.
///
/// The other half of the same clarification. `copilot` here exits immediately with a message, which
/// is what a CLI that will not attach twice to one conversation does — the daemon cannot tell that
/// apart from any other immediate exit, and does not try to. It supervises the process it started
/// like any other, and the crash-loop policy settles `Failed` with the session's process dropped.
///
/// `Failed`'s `reason` is empty on this route, deliberately unasserted: the domain lifecycle is a
/// unit variant, and the message a user reads for an exiting process is the terminal's own output,
/// not a string the daemon invents. T046's route — a refusal the daemon itself decides — is the one
/// that carries a reason.
#[test]
fn a_cli_that_refuses_the_resume_is_reported_and_leaves_nothing_running() {
    let stub = StubOnPath::with_body("copilot", "echo 'session is in use' >&2\nexit 1");

    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());
    let store = tempfile::tempdir().unwrap();
    let project_dir = tempfile::tempdir().unwrap();
    let project = project_dir.path().to_path_buf();
    let id = SessionId::from_uuid(Uuid::from_u128(0xB0A8));
    let state = offered_copilot_session(home.path(), &project, store.path(), id);

    state
        .start_session(id, micold_core::terminal::LaunchMode::Resume)
        .expect(
            "the attempt is made: the refusal is the CLI's to give, and it gives it by exiting",
        );

    let lifecycle = || {
        state
            .sessions_for(&project)
            .into_iter()
            .find(|s| s.id == id)
            .map(|s| s.lifecycle)
    };
    let settled = wait_until(Duration::from_secs(20), || {
        state.supervise_exited_sessions();
        matches!(lifecycle(), Some(WireLifecycle::Failed { .. }))
    });
    assert!(
        settled,
        "a CLI that exits every time must end reported, not retried forever: {:?}",
        lifecycle()
    );
    assert!(
        state.live_session(id).is_none(),
        "and nothing is left running — a dead process kept in the registry would read as alive"
    );
    assert!(
        stub.invocations().len() > 1,
        "the refusal was taken as a crash and retried within the budget, which is exactly the \
         missing-detection posture: the daemon has no way to know this exit means `in use`"
    );
}

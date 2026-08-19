//! The join: daemon → wire snapshot → client state.
//!
//! Every other test in this repository proves one side. The daemon's tests call the transitions
//! themselves and assert on `DaemonState`; the client's build a `CatalogSnapshot` by hand and
//! assert on `State`. Both are useful and neither can fail when the two sides simply do not meet,
//! which is the only place three separate bugs actually lived:
//!
//! - `010` BUG-011 — `Session::start`/`mark_running` were correct and had no production caller, so
//!   `Running` was reachable only by crash-respawn.
//! - `012` BUG-003 — the daemon knew which shell instances had live processes and never published
//!   it. The **first** fix shipped still broken: the daemon-side test closed a shell with
//!   `close_shell`, which removes the process, so no test ever let a shell die on its own and
//!   `live_shells` went on reporting *presence in the registry* rather than *liveness*.
//! - `012` BUG-004 — the bar's restart control decided correctly and its button never asked.
//!
//! So the snapshot here is not written by this file. It is whatever `DaemonState` would actually
//! hand a connecting client, taken after driving a real PTY, and fed to the real
//! `micold_client::catalog_sync::reconcile_catalog`. If either side stops holding up its end, this
//! fails — which is the property none of the per-side tests have.
//!
//! `micold-daemon` dev-depends on `micold-client` (see the manifest; not a cycle — the client never
//! depends on the daemon), which is what lets one test hold both ends.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use micold_client::app::State;
use micold_client::catalog_sync::reconcile_catalog;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::SessionProcess;
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, ShellInstanceId,
    ShellLifecycle, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use uuid::Uuid;

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

/// A catalog holding one session at the project root (a real dir so a spawned shell can `cwd`).
fn catalog_with_session(
    project: &std::path::Path,
    store: &std::path::Path,
    id: SessionId,
) -> Catalog {
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("S".into()),
        TerminalMode::AiCli,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// The same, but a `Regular` session at a fixed id — `start_session` spawns a shell for this one,
/// so the second test can drive the real start path rather than registering a PTY by hand.
fn catalog_with_shell_session(project: &std::path::Path, store: &std::path::Path) -> Catalog {
    let session = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(0x5E55)),
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// What the client's session says about one instance, after folding in the daemon's own snapshot.
fn client_lifecycle_after_reconcile(
    core: &mut State,
    state: &DaemonState,
    id: SessionId,
    instance: ShellInstanceId,
) -> ShellLifecycle {
    // `welcome_payload` is the snapshot a client receives on attach — taken, not constructed.
    reconcile_catalog(core, &state.welcome_payload().0, false);
    core.workspace
        .sessions
        .values()
        .flatten()
        .find(|s| s.id == id)
        .expect("the session survives reconciliation")
        .shells
        .iter()
        .find(|i| i.id == instance)
        .expect("the instance is the client's own — reconciliation must not drop it")
        .lifecycle
}

/// `012` FR-008 / BUG-003, end to end.
///
/// A shell instance's whole observable life, driven on the daemon and read off the client: up,
/// then dead by the user's own `exit`. The second half is the one that shipped broken — and it
/// shipped broken *past* a daemon test and a client test that both passed, because the daemon test
/// closed the shell explicitly (removing the process) and the client test was handed a snapshot
/// that already said what it wanted to prove.
#[test]
fn a_shell_the_daemon_hosts_reaches_the_clients_state_alive_and_then_exited() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let sid = SessionId::new();
    let state = DaemonState::new(catalog_with_session(project.path(), store.path(), sid));

    // The session's AI CLI primary. `cat` so it stays up and is plainly a different process.
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let primary = PtySession::spawn(sid, cmd, 1_000, Some((80, 24))).expect("spawn primary");
    let primary = state.register_session(primary);

    // The client allocates instance ids and owns the set; the daemon owns which have a process.
    // Both sides must independently arrive at the same id for the join to mean anything, so this
    // takes the client's id and hands *that* to the daemon rather than agreeing on a literal.
    let mut core = State::default();
    reconcile_catalog(&mut core, &state.welcome_payload().0, false);
    let instance = {
        let (_, session) = core
            .workspace
            .find_session_mut(sid)
            .expect("the daemon's session reaches the client");
        session.open_shell_instance()
    };

    state
        .open_shell(sid, instance)
        .expect("open shell instance");
    let (shell, _) = state
        .attach_process(sid, SessionProcess::Shell(instance))
        .expect("attach the shell instance");
    assert!(
        wait_until(Duration::from_secs(5), || shell.is_alive()),
        "the shell must actually start for this test to be testing anything"
    );

    assert_eq!(
        client_lifecycle_after_reconcile(&mut core, &state, sid, instance),
        ShellLifecycle::Running,
        "a shell the daemon is hosting must read `running` on the client — this is the whole of \
         `012` BUG-003's first half, and it failed because nothing put liveness on the wire"
    );

    // End it the way a user ends one: not `close_shell`, which removes the process and is exactly
    // the shortcut that let the incomplete fix through.
    state.session_input(sid, 0, b"exit\n");
    assert!(
        wait_until(Duration::from_secs(10), || !shell.is_alive()),
        "the shell must actually exit for this test to be testing anything"
    );

    assert_eq!(
        client_lifecycle_after_reconcile(&mut core, &state, sid, instance),
        ShellLifecycle::Exited,
        "a shell whose process has ended must read `exited` on the client. Reporting presence in \
         the daemon's registry rather than liveness kept this unreachable, and the bar therefore \
         never offered `restart` — which is what hid `012` BUG-004 behind it"
    );

    for pty in state.session_ptys(sid) {
        pty.kill().ok();
    }
    drop(primary);
}

/// The other half of the same seam: the session's own lifecycle (`010` BUG-011).
///
/// `start_session` is what a client's `SessionStart` reaches. Before BUG-011 nothing on that path
/// told the catalog, so a started session stayed at whatever the durable record held and the bar
/// read `idle`/`interrupted` beside a live process. As with the shell above, both sides had tests
/// and neither could see it: the daemon's called `mark_running()` itself, and the client's fixtures
/// did too.
#[test]
fn a_session_the_daemon_starts_reaches_the_clients_state_as_running() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    // The id `catalog_with_shell_session` writes; the client learns it from the snapshot below.
    let sid = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(project.path(), store.path()));

    let lifecycle_of = |core: &State| {
        core.workspace
            .sessions
            .values()
            .flatten()
            .find(|s| s.id == sid)
            .expect("session")
            .lifecycle
            .clone()
    };

    let mut core = State::default();
    reconcile_catalog(&mut core, &state.welcome_payload().0, false);
    assert_ne!(
        lifecycle_of(&core),
        SessionLifecycle::Running,
        "nothing has been started yet"
    );

    state
        .start_session(sid, micold_core::terminal::LaunchMode::Resume)
        .expect("start must spawn the session");
    let live = state.live_session(sid).expect("registered");
    assert!(
        wait_until(Duration::from_secs(5), || live.is_alive()),
        "the process must actually start for this test to be testing anything"
    );

    reconcile_catalog(&mut core, &state.welcome_payload().0, false);
    assert_eq!(
        lifecycle_of(&core),
        SessionLifecycle::Running,
        "a session the daemon is hosting must read `running` on the client — `Session::start` and          `mark_running` were both correct and neither had a caller on this path"
    );

    live.kill().expect("kill");
}

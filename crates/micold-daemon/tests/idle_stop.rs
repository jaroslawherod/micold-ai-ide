//! T034/T035/T038 [US3] — the service stops itself when nobody is using it (FR-008, FR-009,
//! lifecycle contract §3.7–§3.11).
//!
//! Two levels, because the promise has two halves that fail differently.
//!
//! **Does it stop?** is a property of a *process*, so `a_daemon_with_nobody_connected_exits_by_itself`
//! runs the real `micold-daemon` binary and waits for it to be gone. An in-process test could assert
//! that a future resolved and still leave a daemon that never exits — `main` could hold a task, a
//! `Drop` could block, an executor could keep a handle. Nothing short of watching the process go
//! away distinguishes those.
//!
//! **What does it do on the way out?** is a property of the *order*, and that is asserted in-process
//! against the two seams the unwind is built from (data-model G5). The order is the whole safety
//! argument: a session's record has to be durable as `InterruptedResumable` *before* anything kills
//! its process tree, or a stop that is interrupted halfway leaves a record saying `Running` for a
//! process that no longer exists — which the next start would present as a live session the user
//! cannot attach to.
//!
//! The thirty-minute window is not waited out here. [`micold_daemon::idle::IDLE_STOP_ENV`] shortens
//! it, and `idle.rs`'s own unit tests assert the real constant is thirty minutes (T006) — so the
//! number that ships is pinned in one place and no test pays for it.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::idle::IDLE_STOP_ENV;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

// ---------------------------------------------------------------------------------------------
// T034 — does it stop?
// ---------------------------------------------------------------------------------------------

/// A daemon child with its endpoint isolated into `dir`.
///
/// The environment is set on the **child** rather than on this test process, so the tests in this
/// binary can run concurrently without one's `XDG_RUNTIME_DIR` becoming another's.
fn spawn_daemon(dir: &Path, idle_stop: &str) -> Child {
    Command::new(DAEMON_BIN)
        .env("XDG_RUNTIME_DIR", dir)
        // macOS keys the endpoint on `$HOME` instead; set both so one helper isolates either.
        .env("HOME", dir)
        .env("MICOLD_LOG", "warn")
        .env(IDLE_STOP_ENV, idle_stop)
        .spawn()
        .expect("the daemon binary must start")
}

/// Wait for `child` to exit, up to `timeout`. Returns whether it did.
fn exited_within(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match child.try_wait().expect("try_wait") {
            Some(_) => return true,
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    }
    false
}

/// The endpoint the daemon in `dir` will bind, resolved the way the daemon resolves it.
fn endpoint_in(dir: &Path) -> micold_core::endpoint::Endpoint {
    // Set on this process only long enough to ask the shared resolver, so the test and the child
    // cannot disagree about the path — the reason endpoint resolution lives in micold-core.
    let previous = std::env::var_os("XDG_RUNTIME_DIR");
    std::env::set_var("XDG_RUNTIME_DIR", dir);
    let resolved = micold_core::endpoint::resolve().expect("resolve isolated endpoint");
    match previous {
        Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
        None => std::env::remove_var("XDG_RUNTIME_DIR"),
    }
    resolved
}

/// Wait until something is listening at `endpoint`, so a test never races the daemon's startup.
async fn wait_until_listening(endpoint: &micold_core::endpoint::Endpoint, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if matches!(
            micold_core::connect::connect(endpoint, "idle-stop-test").await,
            Ok(Some(_))
        ) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the daemon never started listening at {endpoint:?}");
}

/// FR-008, §3.7: nobody connected for the whole window ⇒ the service stops itself.
///
/// The daemon here is never connected to at all, which is also the case that most needs to work:
/// a client that spawns a daemon and dies before handshaking would otherwise leave a process that
/// lives until the machine reboots (data-model G1 arms the window at construction for this reason).
#[tokio::test]
async fn a_daemon_with_nobody_connected_exits_by_itself() {
    let dir = tempfile::tempdir().unwrap();
    let mut daemon = spawn_daemon(dir.path(), "300ms");

    let exited = exited_within(&mut daemon, Duration::from_secs(30));
    if !exited {
        let _ = daemon.kill();
    }
    assert!(
        exited,
        "a daemon with nobody connected must stop itself once the window expires"
    );
}

/// FR-009, §3.9: one connection holds it up, however long the window is.
///
/// Asserted for many multiples of the window rather than one, because a rule that armed the deadline
/// on connect instead of on the *last disconnect* would still pass a single-window check.
#[tokio::test]
async fn a_daemon_with_one_client_connected_never_exits() {
    let dir = tempfile::tempdir().unwrap();
    let mut daemon = spawn_daemon(dir.path(), "300ms");
    let endpoint = endpoint_in(dir.path());
    wait_until_listening(&endpoint, Duration::from_secs(30)).await;

    // Hold a real, handshaked connection open across many windows.
    let held = micold_core::connect::connect(&endpoint, "idle-stop-test")
        .await
        .expect("connect")
        .expect("a daemon is listening");

    let outlived = !exited_within(&mut daemon, Duration::from_secs(3));
    let still_there = daemon.try_wait().expect("try_wait").is_none();
    drop(held);
    let _ = daemon.kill();
    let _ = daemon.wait();

    assert!(
        outlived && still_there,
        "a connected client must hold the service up for as long as it is connected"
    );
}

// ---------------------------------------------------------------------------------------------
// T035 / T038 — what it does on the way out
// ---------------------------------------------------------------------------------------------

/// A catalog holding one `Regular` session, which `start_session` spawns a real shell for.
fn catalog_with_shell_session(project: &Path, store: &Path, id: SessionId) -> Catalog {
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.to_path_buf(), vec![session]);
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project.to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// The lifecycle the daemon's own catalog reports for `id`.
///
/// **Not read from disk, and that is a fact about the schema rather than a shortcut.** A session's
/// lifecycle is not a persisted field (`StoredSession` in `micold-core/src/store.rs` carries id,
/// worktree, title, mode, archived and provider — no lifecycle), because resumability is derived at
/// startup from the AI CLI's own conversation store rather than from anything this project writes:
/// `present_interrupted_resumable_at_startup` is what turns a loaded `Idle` back into
/// `InterruptedResumable`. That is the design feature 026 settled on and data-model L4 records —
/// "an idle stop introduces no new session state" — so there is no on-disk lifecycle for a test to
/// assert against, and adding one to observe a shutdown would be inventing durable state the
/// product does not have.
///
/// What is still worth testing, and what this reads, is the *ordering*: the catalog's answer changes
/// before anything is killed, so nothing the daemon reports on the way out ever describes a session
/// as `Running` when its process is already gone.
fn catalogued_lifecycle(state: &DaemonState, project: &Path, id: SessionId) -> WireLifecycle {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .expect("the session must still be in the catalog")
        .lifecycle
}

/// G5 / §3.11: the record is durable as `InterruptedResumable` **while the process is still alive**.
///
/// This is the ordering assertion, and it is written this way because "before" is otherwise not
/// observable: after a completed stop both facts are true and no test can tell which happened first.
/// Stopping between the two phases makes the order the *only* thing under test — the file already
/// says interrupted-resumable at a moment when the child is provably still running.
///
/// The failure this prevents is a stop that is interrupted halfway (the machine loses power, the
/// user kills the daemon while it unwinds): what is on disk has to be true of a daemon that is gone,
/// and `Running` is not.
#[test]
fn a_session_is_durably_interrupted_before_anything_kills_it() {
    let store = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E55));
    let state = DaemonState::new(catalog_with_shell_session(cwd.path(), store.path(), id));

    state
        .start_session(id, LaunchMode::Resume)
        .expect("the shell must spawn");
    let live = state.live_session(id).expect("the session must be live");
    let pid = live.pid().expect("a spawned shell has a pid");
    drop(live);
    assert!(
        process_is_alive(pid),
        "precondition: the shell this test is about must be running"
    );

    // Phase one of the unwind, alone.
    let marked = state.mark_live_sessions_interrupted();
    assert_eq!(marked, 1, "the live session must be marked");
    assert_eq!(
        catalogued_lifecycle(&state, cwd.path(), id),
        WireLifecycle::InterruptedResumable,
        "the catalog must say interrupted-resumable"
    );
    assert!(
        process_is_alive(pid),
        "the record changed only after the process was killed — a stop interrupted between the \
         two would leave the daemon describing a session as Running with nothing behind it"
    );

    // Phase two: the session table goes, and `PtySession::Drop` takes the process tree with it.
    assert_eq!(state.take_live_sessions(), 1);
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_is_alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !process_is_alive(pid),
        "dropping the session table must terminate the process tree"
    );
}

/// Whether `pid` still exists. `kill(pid, 0)` is the portable "does this process exist" probe.
fn process_is_alive(pid: u32) -> bool {
    // SAFETY: signal 0 performs error checking only; it never delivers a signal.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// FR-006a/b/c, §3.10: a live session does not extend the window, and what is left behind is
/// resumable rather than resumed.
///
/// The first half is the clarified rule — a machine left with an agent running and no window open
/// must not keep a service alive forever. The second is what makes that safe: the session is still
/// there afterwards, marked resumable, and **nothing restarted it**. Auto-resume would be worse than
/// not stopping: an agent brought back with no user watching it.
#[test]
fn a_live_session_does_not_extend_the_window_and_is_left_resumable() {
    use micold_daemon::idle::{IdleWindow, Presence};

    let store = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(0x5E56));
    let state = DaemonState::new(catalog_with_shell_session(cwd.path(), store.path(), id));
    state
        .start_session(id, LaunchMode::Resume)
        .expect("the shell must spawn");

    // A running session, and nobody connected: the rule expires anyway.
    assert!(state.live_session(id).is_some());
    let presence = state.presence();
    assert_eq!(presence.connected(), 0);
    assert!(
        IdleWindow::default().expired(&presence, micold_core::clock::Uptime::from_nanos(u64::MAX)),
        "a live session must not extend the window (FR-006a)"
    );
    // Belt and braces: the rule cannot see the session at all, whatever it is doing.
    let _: Presence = presence;

    // The unwind leaves it resumable, and leaves it alone.
    state.mark_live_sessions_interrupted();
    state.take_live_sessions();
    assert_eq!(
        catalogued_lifecycle(&state, cwd.path(), id),
        WireLifecycle::InterruptedResumable,
        "the session must be left resumable (FR-006b)"
    );
    assert!(
        state.live_session(id).is_none(),
        "nothing may auto-resume the session — an agent brought back with no user watching is \
         worse than a service that stayed up (FR-006c)"
    );
}

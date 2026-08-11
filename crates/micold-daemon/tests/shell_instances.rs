//! Feature 011 (daemon shell instances) — a session hosts a `Primary` process plus additional shell
//! instances; exactly one is *attached* (streamed + driven) at a time, and `SessionId`-addressed
//! input routes to the attached one (data-model §Session, contracts/shell-instance-lifecycle.md).

#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::SessionProcess;
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLocation, ShellInstanceId, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

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

#[test]
fn a_shell_instance_can_be_opened_attached_and_driven_independently_of_the_primary() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let sid = SessionId::new();
    let state = DaemonState::new(catalog_with_session(project.path(), store.path(), sid));

    // Primary process: a `cat` (echoes input) so we can tell processes apart by content.
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let primary = PtySession::spawn(sid, cmd, 1_000, Some((80, 24))).expect("spawn primary");
    let primary = state.register_session(primary);

    // Open a shell instance; the Primary stays attached until we switch.
    let inst = ShellInstanceId(0);
    state.open_shell(sid, inst).expect("open shell instance");

    // Attach the shell instance.
    let (shell, _) = state
        .attach_process(sid, SessionProcess::Shell(inst))
        .expect("attach the shell instance");

    // Input (SessionId-addressed) routes to the ATTACHED process — the shell echoes it (cooked-mode
    // line discipline). The Primary `cat` must never see it: proof the two processes are distinct.
    state.session_input(sid, 0, b"feature011_marker\n");
    assert!(
        wait_until(Duration::from_secs(5), || visible_text(&shell)
            .contains("feature011_marker")),
        "input must drive the attached shell instance"
    );
    assert!(
        !visible_text(&primary).contains("feature011_marker"),
        "the un-attached primary must not receive the input"
    );

    // Closing the attached instance falls attachment back to the Primary.
    let reattach = state.close_shell(sid, inst);
    assert!(
        reattach.is_some(),
        "closing the attached instance reattaches primary"
    );

    // Test-owned processes: stop them.
    for p in state.remove_session(sid) {
        let _ = p.kill();
    }
}

/// BUG-001 (feature 010-regular-terminal-mode, FR-003) — opening a shell instance for a session
/// whose primary process is not running must still produce a shell.
///
/// `open_shell` spawned the PTY and then inserted it only `if let Some(live)` — so for a session
/// with no live entry the `Arc<PtySession>` fell out of scope and `Drop` killed the child it had
/// just started, while the call still returned `Ok(())`. `attach_process` then returned `None` for
/// the same reason and its handler treated that as "nothing to do". Every layer reported success,
/// so switching a session to Regular Terminal mode did nothing at all, silently.
///
/// A session is not live after its primary exits, after a failed start, or after a daemon restart
/// leaves it `InterruptedResumable` — the state most durable sessions are in right after a restart.
#[test]
fn opening_a_shell_on_a_session_whose_primary_is_not_running_still_attaches_a_shell() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::new();
    let state = DaemonState::new(catalog_with_session(project.path(), store.path(), id));

    // Deliberately never started: this is a durable record with no live process, exactly as after
    // a daemon restart.
    assert!(
        state.live_session(id).is_none(),
        "precondition: the session has no live process"
    );

    let instance = ShellInstanceId(1);
    state
        .open_shell(id, instance)
        .expect("opening a shell instance must not fail for a not-live session");

    let attached = state.attach_process(id, SessionProcess::Shell(instance));
    assert!(
        attached.is_some(),
        "the shell instance must exist and be attachable — otherwise the mode toggle silently \
         does nothing (FR-003, FR-007)"
    );
    assert!(
        attached.unwrap().0.is_alive(),
        "the spawned shell must still be running, not killed by being dropped on the floor"
    );
}

/// T032 (BUG-001) — a shell opened for a session that never had a primary is still torn down with
/// the session, so the new "create a live entry around the shell" path cannot leak a process.
///
/// `remove_session` returns only the *primary* handle for the caller to `kill()`, and there is no
/// primary here. What actually reclaims the shell is the removal of the whole `LiveSession`: each
/// `Proc`'s `Drop` kills and joins its child. This pins that, since a future refactor that leaned
/// on the returned handle instead would leak a shell per toggle.
#[test]
fn a_shell_opened_without_a_primary_is_reclaimed_when_the_session_is_removed() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::new();
    let state = DaemonState::new(catalog_with_session(project.path(), store.path(), id));

    let instance = ShellInstanceId(1);
    state.open_shell(id, instance).unwrap();
    let shell = state
        .attach_process(id, SessionProcess::Shell(instance))
        .expect("the shell instance is live")
        .0;
    assert!(shell.is_alive(), "precondition: the shell is running");

    // Teardown must hand back the shell even though the session has no primary — `Drop` alone is
    // not enough while another `Arc` (in production, a view-stream task) still holds the process.
    let removed = state.remove_session(id);
    assert_eq!(
        removed.len(),
        1,
        "the shell handle must come back for an explicit kill"
    );
    for pty in removed {
        let _ = pty.kill();
    }
    assert!(
        state.live_session(id).is_none(),
        "the live entry is gone from the registry"
    );
    assert!(
        wait_until(Duration::from_secs(5), || !shell.is_alive()),
        "the shell process must be reclaimed with the session, not leaked"
    );
}

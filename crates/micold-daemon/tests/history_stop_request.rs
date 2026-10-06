//! Feature 041, milestone M4: an orderly stop of the session service saves every running terminal
//! first (story 1 scenarios 1 and 8, FR-002, FR-004, SC-001; stop-request contract §1, §4, §6).
//!
//! The unwind is run directly, as the idle stop runs it; the stop request is a real service
//! process sent a signal. The saved files are read by a second `DaemonState` on the same
//! directory, which is a service started after the stop.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

#[path = "support/history.rs"]
mod history;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use history::{
    ai_session, fake_cli, history_file, history_showing, script, service_saving, texts, wait_file,
};
use micold_core::session::SessionId;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{HistoryStore, LoadOutcome};
use micold_daemon::idle::StopReason;
use micold_daemon::server::unwind;
use micold_daemon::state::DaemonState;

/// The lines of the file of `id` in `dir`, as a service started later reads them.
fn saved_lines(dir: &Path, id: SessionId) -> Vec<String> {
    match HistoryStore::new(dir.to_path_buf(), true).load(id) {
        LoadOutcome::History(snapshot) => texts(&snapshot),
        other => panic!("no saved history: {other:?}"),
    }
}

/// Start `id` with the stand-in printing `lines` numbered lines, and wait until the terminal holds
/// the last of them.
fn running_with_lines(state: &DaemonState, project: &Path, id: SessionId, lines: u32) {
    script(project, &format!("lines {lines} L\nwait\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(state, id, &format!("L{lines}"));
}

/// U79, A1, A8, SC-001: the idle unwind saves a running terminal, and a service started after it
/// restores all 200 lines.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_idle_unwind_saves_every_line_and_a_restart_restores_them() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    running_with_lines(&state, project.path(), id, 200);

    unwind(&state, StopReason::Idle).await;

    let lines = saved_lines(saved.path(), id);
    for n in 1..=200 {
        assert!(
            lines.iter().any(|l| l == &format!("L{n}")),
            "L{n} is missing"
        );
    }
    let again = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "wait\n");
    again.start_session(id, LaunchMode::Fresh).expect("starts");
    let shown = history_showing(&again, id, "L200");
    assert!(shown.iter().any(|l| l == "L1"), "{shown:?}");
    again.stop_session(id);
}

/// U80: a terminal with no output since its last save is not rewritten by the unwind (FR-004).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_terminal_with_nothing_new_is_not_rewritten_by_the_unwind() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    running_with_lines(&state, project.path(), id, 20);
    state.save_due_at(Instant::now());
    let first = std::fs::metadata(history_file(saved.path(), id))
        .and_then(|m| m.modified())
        .expect("saved by the saver");
    std::thread::sleep(Duration::from_millis(60));

    unwind(&state, StopReason::Idle).await;

    let after = std::fs::metadata(history_file(saved.path(), id))
        .and_then(|m| m.modified())
        .unwrap();
    assert_eq!(after, first, "the file was rewritten");
}

/// U81: a store that refuses to save (no directory, and it does not create one) writes nothing,
/// and the unwind goes on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_store_that_does_not_save_leaves_nothing_on_disk() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let missing = parent.path().join("terminal-history");
    let session = ai_session();
    let id = session.id;
    let state = history::service(project.path(), vec![session]);
    state.set_history_store(HistoryStore::new(missing.clone(), false));
    running_with_lines(&state, project.path(), id, 5);

    unwind(&state, StopReason::Idle).await;

    assert!(!missing.exists(), "the unwind made the directory");
}

/// U82, SR §4: a save that blocks does not hold the unwind longer than 3 s, and the previous
/// file stays.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_save_that_blocks_holds_the_unwind_for_at_most_three_seconds() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    running_with_lines(&state, project.path(), id, 5);
    state.save_due_at(Instant::now());
    let old = saved_lines(saved.path(), id);
    state.session_input(id, 1, b"newer\n");
    history_showing(&state, id, "newer");

    let gate = state.session_gate(id).lock_owned().await;
    let started = Instant::now();
    unwind(&state, StopReason::Idle).await;
    let took = started.elapsed();
    drop(gate);

    assert!(
        took >= Duration::from_millis(2900),
        "returned early: {took:?}"
    );
    assert!(
        took < Duration::from_millis(4500),
        "held the unwind: {took:?}"
    );
    assert_eq!(
        saved_lines(saved.path(), id),
        old,
        "the previous file stays"
    );
}

/// U83: the unwind is not over until the saves are: it waits for a save that finishes in time,
/// and the file is whole when it returns. The endpoint is released only after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_unwind_returns_only_after_the_saves() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    running_with_lines(&state, project.path(), id, 50);

    let gate = state.session_gate(id).lock_owned().await;
    let stopping = {
        let state = Arc::clone(&state);
        tokio::spawn(async move { unwind(&state, StopReason::Idle).await })
    };
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!stopping.is_finished(), "returned before the save");
    assert!(!history_file(saved.path(), id).exists());
    drop(gate);
    stopping.await.unwrap();

    assert!(saved_lines(saved.path(), id).iter().any(|l| l == "L50"));
}

/// U84, SR §4, FR-002, SC-001: ten sessions of 10,000 lines of 100 characters are all saved by
/// one unwind within its bound.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_busy_sessions_are_all_saved_within_the_bound() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions: Vec<_> = (0..10).map(|_| ai_session()).collect();
    let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
    let state = service_saving(project.path(), sessions, saved.path());
    let prefix = "x".repeat(90);
    script(project.path(), &format!("lines 10000 {prefix}\nwait\n"));
    for id in &ids {
        state.start_session(*id, LaunchMode::Fresh).expect("starts");
    }
    for id in &ids {
        history_showing(&state, *id, &format!("{prefix}10000"));
    }

    let started = Instant::now();
    unwind(&state, StopReason::Idle).await;
    let took = started.elapsed();

    assert!(took < Duration::from_millis(3500), "took {took:?}");
    for id in &ids {
        let lines = saved_lines(saved.path(), *id);
        assert!(lines.iter().any(|l| l == &format!("{prefix}10000")));
    }
}

/// U79 on a real service: `SIGTERM`, `SIGINT` and `SIGHUP` each make it exit within 5 s with the
/// last line it printed in the session's file (SR §6).
#[cfg(unix)]
#[tokio::test]
async fn a_signal_stops_a_real_service_within_five_seconds_with_its_terminals_saved() {
    use std::collections::BTreeMap;
    use std::process::{Command, Stdio};

    use futures_util::SinkExt;
    use micold_core::connect::{connect, Connected};
    use micold_core::project::{Availability, Project};
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::ClientMsg;
    use micold_core::session::{AiCli, Session, SessionLabel, SessionLocation, TerminalMode};
    use micold_core::store::{JsonFileStore, ProjectStore};
    use micold_core::workspace::Workspace;

    fake_cli();
    for (name, signal) in [
        ("SIGTERM", libc::SIGTERM),
        ("SIGINT", libc::SIGINT),
        ("SIGHUP", libc::SIGHUP),
    ] {
        let runtime = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_RUNTIME_DIR", runtime.path());
        std::env::set_var("HOME", runtime.path());
        std::env::set_var("XDG_DATA_HOME", data.path());
        std::env::set_var("MICOLD_LOG", "warn");

        let id = SessionId::from_uuid(uuid::Uuid::new_v4());
        let session = Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("AI".into()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        );
        let workspace = Workspace {
            projects: vec![Project::new(
                project.path().to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project.path().to_path_buf()),
            sessions: BTreeMap::from([(project.path().to_path_buf(), vec![session])]),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        };
        JsonFileStore::default_location()
            .unwrap()
            .save(&workspace)
            .unwrap();
        let ready = project.path().join("ready");
        script(
            project.path(),
            &format!(
                "print before\nprint last-line-{name}\ntouch {}\nwait\n",
                ready.display()
            ),
        );

        let endpoint = micold_core::endpoint::resolve().unwrap();
        let mut daemon = Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the service");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut conn = loop {
            if let Ok(Some(Connected::Ready(conn, _))) = connect(&endpoint, "stop-test").await {
                break conn;
            }
            assert!(Instant::now() < deadline, "the service never listened");
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        conn.send(Frame::Control(ClientMsg::SessionStart { session: id }))
            .await
            .expect("start the session");
        wait_file(&ready);
        tokio::time::sleep(Duration::from_millis(300)).await;

        // SAFETY: signals the child this test started.
        unsafe { libc::kill(daemon.id() as libc::pid_t, signal) };
        let stopped_at = Instant::now();
        let status = loop {
            if let Some(status) = daemon.try_wait().unwrap() {
                break status;
            }
            if stopped_at.elapsed() > Duration::from_secs(5) {
                let _ = daemon.kill();
                panic!("{name}: the service did not exit within 5 s");
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        assert!(status.success(), "{name}: {status:?}");

        let dir = micold_core::terminal_history::history_dir().unwrap();
        let lines = saved_lines(&dir, id);
        assert!(
            lines.iter().any(|l| l == &format!("last-line-{name}")),
            "{name}: {lines:?}"
        );
    }
}

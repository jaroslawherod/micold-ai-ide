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
/// one unwind within its bound. ConPTY passes only about 50 lines a second per session when ten
/// are busy (CI run 37519388539 reached line 565 in 10 s), so on Windows the sessions print 500
/// lines: the bound under test is the unwind's, not the pseudoterminal's throughput.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_busy_sessions_are_all_saved_within_the_bound() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions: Vec<_> = (0..10).map(|_| ai_session()).collect();
    let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
    let state = service_saving(project.path(), sessions, saved.path());
    let prefix = "x".repeat(90);
    let count = if cfg!(windows) { 500 } else { 10_000 };
    script(project.path(), &format!("lines {count} {prefix}\nwait\n"));
    for id in &ids {
        state.start_session(*id, LaunchMode::Fresh).expect("starts");
    }
    for id in &ids {
        history_showing(&state, *id, &format!("{prefix}{count}"));
    }

    let started = Instant::now();
    unwind(&state, StopReason::Idle).await;
    let took = started.elapsed();

    assert!(took < Duration::from_millis(3500), "took {took:?}");
    for id in &ids {
        let lines = saved_lines(saved.path(), *id);
        assert!(lines.iter().any(|l| l == &format!("{prefix}{count}")));
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

/// The stop request on Windows (T061, SR §1, §2, §3, §6): the named event and the end-of-session
/// window of a real service process. The event's name is one per user and so is the pipe, so the
/// cases take `SERIAL` as `tests/daemon_stop.rs` does, and run against the signed-in user's real
/// endpoint and data directories: they are for CI's Windows job, not a desktop with the app open.
#[cfg(windows)]
mod windows {
    use std::collections::BTreeMap;
    use std::process::{Child, Command, Stdio};

    use futures_util::SinkExt;
    use micold_core::connect::{connect, Connected};
    use micold_core::project::{Availability, Project};
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::ClientMsg;
    use micold_core::session::{AiCli, Session, SessionLabel, SessionLocation, TerminalMode};
    use micold_core::store::{JsonFileStore, ProjectStore};
    use micold_core::workspace::Workspace;
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetSecurityInfo, SE_KERNEL_OBJECT,
    };
    use windows_sys::Win32::Security::{
        GetAce, GetSecurityDescriptorControl, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
    };
    use windows_sys::Win32::Storage::FileSystem::READ_CONTROL;
    use windows_sys::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_ENDSESSION};

    use super::*;

    /// One at a time: the event, the pipe and the data directories are the user's own.
    static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A real service with one session that has printed its last line and is waiting.
    struct Service {
        child: Child,
        id: SessionId,
        last_line: String,
        _project: tempfile::TempDir,
    }

    impl Drop for Service {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(dir) = micold_core::terminal_history::history_dir() {
                let _ = std::fs::remove_file(history_file(&dir, self.id));
            }
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// The stop event with `access`, or `None` while no service has created it.
    fn open_event(access: u32) -> Option<HANDLE> {
        let name = wide(&micold_core::endpoint::stop_event_name().expect("the event's name"));
        // SAFETY: `name` is NUL-terminated and outlives the call; null is the failure signal.
        let handle = unsafe { OpenEventW(access, 0, name.as_ptr()) };
        (!handle.is_null()).then_some(handle)
    }

    /// The event once the service has created it, within 10 s.
    async fn wait_event(access: u32) -> HANDLE {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(handle) = open_event(access) {
                return handle;
            }
            assert!(
                Instant::now() < deadline,
                "the service never created the stop event"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Start the real service, then a session in it, and wait until it has printed `last-line-<tag>`.
    /// `None` when a service of this user is already running, whose endpoint and event these cases
    /// would otherwise take over.
    async fn start_service(tag: &str) -> Option<Service> {
        fake_cli();
        if let Some(event) = open_event(EVENT_MODIFY_STATE) {
            // SAFETY: closing the handle opened above.
            unsafe { CloseHandle(event) };
            eprintln!("skipped: a service of this user is running and owns the stop event");
            return None;
        }
        let project = tempfile::tempdir().unwrap();
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
        let last_line = format!("last-line-{tag}");
        script(
            project.path(),
            &format!(
                "print before\nprint {last_line}\ntouch {}\nwait\n",
                ready.display()
            ),
        );

        let endpoint = micold_core::endpoint::resolve().unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
            .env("MICOLD_LOG", "warn")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the service");
        let service = Service {
            child,
            id,
            last_line,
            _project: project,
        };
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
        wait_file(&service._project.path().join("ready"));
        tokio::time::sleep(Duration::from_millis(300)).await;
        Some(service)
    }

    /// The service's exit code once it has exited, within 5 s of `from`.
    async fn exit_code_within_five_seconds(service: &mut Service, from: Instant) -> Option<i32> {
        loop {
            if let Some(status) = service.child.try_wait().unwrap() {
                return status.code();
            }
            assert!(
                from.elapsed() <= Duration::from_secs(5),
                "the service did not exit within 5 s"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// The saved file of the service's session holds its last line.
    fn assert_saved(service: &Service) {
        let dir = micold_core::terminal_history::history_dir().unwrap();
        let lines = saved_lines(&dir, service.id);
        assert!(
            lines.iter().any(|l| l == &service.last_line),
            "the file lacks {:?}: {lines:?}",
            service.last_line
        );
    }

    /// U115, A1: setting the event makes a real service exit within 5 s, its file holding the last
    /// line.
    #[tokio::test]
    async fn setting_the_stop_event_stops_a_real_service_with_its_terminals_saved() {
        let _serial = SERIAL.lock().await;
        let Some(mut service) = start_service("event").await else {
            return;
        };

        let event = wait_event(EVENT_MODIFY_STATE).await;
        // SAFETY: `event` is the live handle opened above, closed once after the call.
        unsafe {
            assert_ne!(SetEvent(event), 0, "set the event");
            CloseHandle(event);
        }
        let code = exit_code_within_five_seconds(&mut service, Instant::now()).await;

        assert_eq!(code, Some(0), "an orderly exit");
        assert_saved(&service);
    }

    /// U116: `WM_ENDSESSION` sent to the service's hidden window raises the same request.
    #[tokio::test]
    async fn an_end_of_session_message_stops_a_real_service_with_its_terminals_saved() {
        let _serial = SERIAL.lock().await;
        let Some(mut service) = start_service("window").await else {
            return;
        };

        let class = wide(micold_daemon::platform::STOP_WINDOW_CLASS);
        let deadline = Instant::now() + Duration::from_secs(10);
        let window = loop {
            // SAFETY: `class` is NUL-terminated and outlives the call.
            let window = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
            if !window.is_null() {
                break window;
            }
            assert!(
                Instant::now() < deadline,
                "the service has no end-of-session window"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        // Posted, not sent: the window holds Windows back until the unwind is over, and a send
        // would wait for that.
        // SAFETY: `window` was just found; the message carries no pointers.
        assert_ne!(
            unsafe { PostMessageW(window, WM_ENDSESSION, 1, 0) },
            0,
            "post WM_ENDSESSION"
        );
        let code = exit_code_within_five_seconds(&mut service, Instant::now()).await;

        assert_eq!(code, Some(0), "an orderly exit");
        assert_saved(&service);
    }

    /// U117: the event's DACL is protected and has one entry, for the current user.
    #[tokio::test]
    async fn the_stop_events_dacl_has_one_entry_for_the_current_user() {
        let _serial = SERIAL.lock().await;
        let Some(service) = start_service("dacl").await else {
            return;
        };

        let event = wait_event(READ_CONTROL).await;
        // SAFETY: `event` is a live handle with READ_CONTROL; every out pointer is a local, and
        // the handle and the descriptor are released below.
        let (protected, ace_count, owner) = unsafe {
            let mut dacl: *mut ACL = std::ptr::null_mut();
            let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            let status = GetSecurityInfo(
                event,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            );
            CloseHandle(event);
            assert_eq!(status, ERROR_SUCCESS, "GetSecurityInfo on the event");
            assert!(!dacl.is_null(), "a NULL DACL grants everyone full access");
            let mut control = 0u16;
            let mut revision = 0u32;
            assert_ne!(
                GetSecurityDescriptorControl(descriptor, &mut control, &mut revision),
                0
            );
            let count = (*dacl).AceCount;
            let mut owner = None;
            let mut ace: *mut core::ffi::c_void = std::ptr::null_mut();
            if count > 0 && GetAce(dacl, 0, &mut ace) != 0 {
                let header = &*(ace as *const ACE_HEADER);
                // ACCESS_ALLOWED_ACE_TYPE
                if header.AceType == 0 {
                    let allowed = ace as *const ACCESS_ALLOWED_ACE;
                    let sid = std::ptr::addr_of!((*allowed).SidStart) as *mut core::ffi::c_void;
                    let mut text: *mut u16 = std::ptr::null_mut();
                    if ConvertSidToStringSidW(sid, &mut text) != 0 {
                        let len = (0..).take_while(|&i| *text.add(i) != 0).count();
                        owner = String::from_utf16(std::slice::from_raw_parts(text, len)).ok();
                        LocalFree(text.cast());
                    }
                }
            }
            LocalFree(descriptor);
            (control & SE_DACL_PROTECTED != 0, count, owner)
        };
        drop(service);

        assert!(protected, "nothing may be inherited into the event's DACL");
        assert_eq!(ace_count, 1, "exactly one entry");
        assert_eq!(
            owner,
            Some(micold_core::endpoint::user_sid().unwrap()),
            "the entry is for the current user"
        );
    }

    /// U118, SR §2: `stop_running_daemon` against the real service makes it exit with code 0, the
    /// orderly exit after the unwind and not the 1 of `TerminateProcess`, with its file holding the
    /// last line.
    #[tokio::test]
    async fn restart_service_stops_a_real_service_in_order() {
        let _serial = SERIAL.lock().await;
        let Some(mut service) = start_service("restart").await else {
            return;
        };
        let endpoint = micold_core::endpoint::resolve().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while micold_core::spawn::running_daemon_pid(&endpoint) != Some(service.child.id()) {
            assert!(
                Instant::now() < deadline,
                "the service never recorded its pid"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let event = wait_event(EVENT_MODIFY_STATE).await;
        // SAFETY: closing the handle opened above; an open handle would keep the event alive.
        unsafe { CloseHandle(event) };

        let started = Instant::now();
        let stopped = micold_core::spawn::stop_running_daemon(&endpoint);
        let code = exit_code_within_five_seconds(&mut service, started).await;

        assert!(matches!(stopped, Ok(true)), "got {stopped:?}");
        assert_eq!(
            code,
            Some(0),
            "the orderly exit, not the 1 of TerminateProcess"
        );
        assert_saved(&service);
    }
}

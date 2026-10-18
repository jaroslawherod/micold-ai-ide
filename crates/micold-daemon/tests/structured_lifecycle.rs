//! Feature 938, milestone M2 (T015): a structured session's lifecycle is a terminal session's (US5
//! scenarios 1–2, FR-009). Each case runs the same steps against a terminal and a structured session
//! of the stand-in of `support/structured.rs` and compares what it observes.
//!
//! The cases share this process's `PATH`, and one of them empties it: they run one at a time.
#![cfg(unix)]

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use micold_core::protocol::messages::WireLifecycle;
use micold_core::session::{Session, SessionId, SessionLocation};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use history::{ai_session, service};
use structured::{
    events_of, events_until, fake_stream_cli, fixture, fixture_events, record, starts,
    stream_script, structured_session, viewer,
};

fn one_at_a_time() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The two kinds of session a case compares, terminal first.
fn both() -> [Session; 2] {
    [ai_session(), structured_session(SessionLocation::Default)]
}

fn lifecycle(state: &DaemonState, project: &Path, id: SessionId) -> WireLifecycle {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.lifecycle)
        .expect("listed")
}

/// The lifecycle once it has stayed the same for 300 ms, within 10 s.
fn settled(state: &DaemonState, project: &Path, id: SessionId) -> WireLifecycle {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last = lifecycle(state, project, id);
    let mut since = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(20));
        let now = lifecycle(state, project, id);
        if now != last {
            (last, since) = (now, Instant::now());
        } else if since.elapsed() >= Duration::from_millis(300) {
            return last;
        }
        assert!(Instant::now() < deadline, "never settled: {last:?}");
    }
}

/// Whether `pid` runs: it exists and is not a zombie (the daemon reaps an exited child only at
/// kill).
fn pid_alive(pid: &str) -> bool {
    std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .output()
        .is_ok_and(|out| {
            let stat = String::from_utf8_lossy(&out.stdout);
            !stat.trim().is_empty() && !stat.trim_start().starts_with('Z')
        })
}

/// Start, stop, start, a crash and its restart, kill: the states each step leaves, for one session.
fn walk(session: Session) -> Vec<WireLifecycle> {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let id = session.id;
    let state = service(dir, vec![session]);
    let mut seen = Vec::new();
    stream_script(dir, "");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    record(dir, "pid-record");
    seen.push(settled(&state, dir, id));
    assert!(state.stop_session(id));
    seen.push(settled(&state, dir, id));
    stream_script(dir, "pause 300\nexit 1\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    seen.push(settled(&state, dir, id));
    stream_script(dir, "");
    let deadline = Instant::now() + Duration::from_secs(10);
    while starts(dir) < 3 {
        state.supervise_exited_sessions();
        assert!(Instant::now() < deadline, "the crash was never restarted");
        std::thread::sleep(Duration::from_millis(50));
    }
    seen.push(settled(&state, dir, id));
    let pid = record(dir, "pid-record");
    for pty in state.remove_session(id) {
        let _ = pty.kill();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while pid_alive(&pid) {
        assert!(Instant::now() < deadline, "kill left the stand-in running");
        std::thread::sleep(Duration::from_millis(20));
    }
    seen
}

/// US5 scenario 1, FR-009: stop, start, crash restart and kill report the same states in the same
/// order for both kinds, and kill ends the stand-in.
#[test]
fn stop_start_restart_and_kill_report_a_terminal_sessions_states() {
    let _serial = one_at_a_time();
    fake_stream_cli();
    let [terminal, structured] = both();
    assert_eq!(walk(structured), walk(terminal));
}

/// Supervise until the session gives up; the number of starts it took.
fn starts_until_given_up(session: Session) -> (usize, WireLifecycle) {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let id = session.id;
    let state = service(dir, vec![session]);
    stream_script(dir, "exit 1\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        std::thread::sleep(Duration::from_millis(80));
        state.supervise_exited_sessions();
        if let failed @ WireLifecycle::Failed { .. } = lifecycle(&state, dir, id) {
            return (starts(dir), failed);
        }
        assert!(Instant::now() < deadline, "never gave up");
    }
}

/// US5 scenario 2: a CLI that exits at once on every start is given up on after as many starts as
/// a terminal session.
#[test]
fn crash_loop_protection_stops_after_a_terminal_sessions_starts() {
    let _serial = one_at_a_time();
    fake_stream_cli();
    let [terminal, structured] = both();
    let (terminal_starts, terminal_end) = starts_until_given_up(terminal);
    let (structured_starts, structured_end) = starts_until_given_up(structured);
    assert_eq!(structured_starts, terminal_starts);
    assert!(terminal_starts >= 2, "a crash was restarted at least once");
    assert_eq!(structured_end, terminal_end);
}

/// Edge case: a missing `claude` fails a structured start as it fails a terminal one.
#[test]
fn a_missing_cli_fails_both_kinds_alike() {
    let _serial = one_at_a_time();
    fake_stream_cli();
    let empty = tempfile::tempdir().unwrap();
    let path = std::env::var_os("PATH").unwrap();
    std::env::set_var("PATH", empty.path());
    let outcomes: Vec<(String, WireLifecycle)> = both()
        .into_iter()
        .map(|session| {
            let project = tempfile::tempdir().unwrap();
            let id = session.id;
            let state = service(project.path(), vec![session]);
            let error = match state.start_session(id, LaunchMode::Fresh) {
                Ok(()) => "started".to_string(),
                Err(e) => e.to_string(),
            }
            .replace(&project.path().display().to_string(), "<project>");
            (error, settled(&state, project.path(), id))
        })
        .collect();
    std::env::set_var("PATH", path);
    assert_eq!(outcomes[1], outcomes[0]);
    assert_ne!(outcomes[0].0, "started");
}

/// FR-009: the events written before an exit stay in the history a viewer receives.
#[tokio::test(flavor = "multi_thread")]
// The guard serialises this case with the others that change `PATH`; it is meant to be held
// across the awaits, and nothing else on this runtime takes it.
#[allow(clippy::await_holding_lock)]
async fn events_written_before_an_exit_stay_in_the_history() {
    let _serial = one_at_a_time();
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state: Arc<DaemonState> = service(project.path(), vec![session]);
    let name = "text-turn.jsonl";
    stream_script(
        project.path(),
        &format!("replay {}\nexit 1\n", fixture(name).display()),
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let pid = record(project.path(), "pid-record");
    while pid_alive(&pid) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let expected = fixture_events(name);
    let mut client = viewer(&state, project.path(), id).await;
    let got = events_until(&mut client, id, |got| got.len() >= expected.len()).await;
    assert_eq!(events_of(&got), expected);
    assert!(got.iter().all(|r| !r.live), "history, not live events");
}

//! Feature 041, milestone M5: the setting that keeps terminal output off the disk (story 2,
//! FR-026 to FR-029, FR-033, SC-008; contracts/setting.md §3).
//!
//! The saver is driven by hand with `save_due_at(now)`, as in `history_periodic_save.rs`, and the
//! setting is changed through `DaemonState::set_save_terminal_history`, which is what the
//! service's `SettingsSet` handler calls; one case sends the message over a real connection.
//! These run on Unix, where a typed line is echoed.
// unix-only: the cases print by typing into the session, which a Unix terminal echoes
#![cfg(unix)]

#[path = "support/attention.rs"]
mod attention;
#[path = "support/history.rs"]
mod history;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use history::{
    ai_session, fake_cli, history_file, history_showing, script, separators, service, texts,
    wait_file,
};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::schedule::SAVE_SPACING;
use micold_core::terminal_history::{HistoryStore, LoadOutcome};
use micold_daemon::state::DaemonState;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run `f` and return the warnings logged on this thread meanwhile, one per line.
fn warnings_of<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    let buffer = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    (result, text.lines().map(str::to_string).collect())
}

const SECOND: Duration = Duration::from_secs(1);

/// A service whose catalog holds `sessions`, with saving `on` or off as the stored setting says
/// when it starts, and its histories in `dir`.
fn service_with(
    project: &Path,
    sessions: Vec<micold_core::session::Session>,
    dir: &Path,
    on: bool,
) -> Arc<DaemonState> {
    let state = service(project, sessions);
    state.set_save_terminal_history(on).unwrap();
    state.set_history_store(HistoryStore::new(dir.to_path_buf(), true));
    state
}

/// Run `id` until the stand-in has printed `lines`, then leave it running.
fn run_printing(state: &DaemonState, project: &Path, id: SessionId, lines: &[&str]) {
    let ready = project.join(format!("ready-{}", id.0));
    let _ = std::fs::remove_file(&ready);
    let mut directives: String = lines.iter().map(|l| format!("print {l}\n")).collect();
    directives.push_str(&format!("touch {}\nwait\n", ready.display()));
    script(project, &directives);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&ready);
    for line in lines {
        history_showing(state, id, line);
    }
}

fn serial() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Type `text` into the session and wait until its echo is in the history.
fn type_line(state: &DaemonState, id: SessionId, text: &str) {
    state.session_input(id, serial(), format!("{text}\n").as_bytes());
    history_showing(state, id, text);
}

/// The saved lines of `id` in `dir`, as a service that starts later would read them.
fn saved_lines(dir: &Path, id: SessionId) -> Vec<String> {
    match HistoryStore::new(dir.to_path_buf(), true).load(id) {
        LoadOutcome::History(snapshot) => texts(&snapshot),
        other => panic!("no saved history: {other:?}"),
    }
}

fn holds(lines: &[String], text: &str) -> bool {
    lines.iter().any(|l| l == text)
}

/// Every file in `dir`, sorted; none when the directory is not there.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

/// A directory whose files cannot be deleted, or `None` when this process is not stopped by that
/// (root).
fn undeletable(dir: &Path) -> Option<impl Fn(bool) + '_> {
    use std::os::unix::fs::PermissionsExt;
    let lock = move |on: bool| {
        let mode = if on { 0o500 } else { 0o700 };
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    lock(true);
    let stopped = std::fs::File::create(dir.join("probe")).is_err();
    lock(false);
    stopped.then_some(lock)
}

/// Story 2 scenario 2, FR-028 (U98): with the setting off, a session's output and a service
/// restart leave no file, and the terminal starts empty with no separator.
#[test]
fn with_saving_off_output_and_a_restart_leave_no_file_and_an_empty_terminal() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session.clone()], saved.path(), false);
    let t0 = Instant::now();
    run_printing(&state, project.path(), id, &["secret one"]);
    state.save_due_at(t0 + 5 * SECOND);
    state.save_due_at(t0 + 60 * SECOND);
    assert!(state.stop_session(id));
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());

    let restarted = service_with(project.path(), vec![session], saved.path(), false);
    run_printing(&restarted, project.path(), id, &["fresh"]);
    let lines = texts(&history::snapshot(&restarted, id));
    assert!(!holds(&lines, "secret one"), "{lines:#?}");
    assert!(separators(&lines).is_empty(), "{lines:#?}");
    restarted.stop_session(id);
}

/// Story 2 scenario 3 (U98): turning it off while a session prints stops every later write,
/// without a restart.
#[test]
fn turning_saving_off_stops_every_later_write() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session], saved.path(), true);
    let t0 = Instant::now();
    run_printing(&state, project.path(), id, &["before"]);
    state.save_due_at(t0 + 5 * SECOND);
    assert!(holds(&saved_lines(saved.path(), id), "before"));

    state.set_save_terminal_history(false).unwrap();
    type_line(&state, id, "after");
    for tick in (10..=120).step_by(5) {
        state.save_due_at(t0 + Duration::from_secs(tick));
    }
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
    assert!(state.stop_session(id));
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new(), "also at a stop");
}

/// Story 2 scenario 4 (U99): turning it on saves the running session's history, with what it
/// printed while off, within 60 s.
#[test]
fn turning_saving_on_saves_the_running_history_including_what_was_printed_while_off() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session], saved.path(), false);
    let t0 = Instant::now();
    run_printing(&state, project.path(), id, &["first"]);
    type_line(&state, id, "while off");
    state.save_due_at(t0 + 5 * SECOND);
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());

    state.set_save_terminal_history(true).unwrap();
    let landed = (10..=70).step_by(5).find(|s| {
        state.save_due_at(t0 + Duration::from_secs(*s));
        history_file(saved.path(), id).exists()
    });
    assert!(landed.is_some_and(|s| s - 10 <= 60), "saved: {landed:?}");
    let lines = saved_lines(saved.path(), id);
    assert!(holds(&lines, "first") && holds(&lines, "while off"), "{lines:#?}");
    state.stop_session(id);
}

/// Story 2 scenarios 5 and 6, SC-008 (U98): turning it off deletes the files of a running and of
/// a stopped session before the call returns, and the running terminal still shows its history.
#[test]
fn turning_saving_off_deletes_every_file_at_once_and_leaves_the_running_terminal() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (running, stopped) = (ai_session(), ai_session());
    let (run_id, stop_id) = (running.id, stopped.id);
    let state = service_with(
        project.path(),
        vec![running, stopped],
        saved.path(),
        true,
    );
    let t0 = Instant::now();
    run_printing(&state, project.path(), stop_id, &["stopped session text"]);
    assert!(state.stop_session(stop_id));
    run_printing(&state, project.path(), run_id, &["running session text"]);
    state.save_due_at(t0 + 5 * SECOND);
    assert_eq!(files(saved.path()).len(), 2, "both are on disk");

    state.set_save_terminal_history(false).unwrap();

    assert_eq!(
        files(saved.path()),
        Vec::<PathBuf>::new(),
        "gone when the call returns, which is before SettingsSet is answered"
    );
    let lines = texts(&history::snapshot(&state, run_id));
    assert!(holds(&lines, "running session text"), "{lines:#?}");
    state.stop_session(run_id);
}

/// Story 2 scenario 7 (U98): off then on then a service restart shows nothing of what was saved
/// before; the deleted histories do not come back.
#[test]
fn off_then_on_then_a_restart_brings_back_nothing_that_was_deleted() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session.clone()], saved.path(), true);
    run_printing(&state, project.path(), id, &["old output"]);
    assert!(state.stop_session(id));
    assert_eq!(files(saved.path()).len(), 1);

    state.set_save_terminal_history(false).unwrap();
    state.set_save_terminal_history(true).unwrap();
    assert_eq!(
        files(saved.path()),
        Vec::<PathBuf>::new(),
        "a stopped session's carried history is not written until it runs again"
    );

    let restarted = service_with(project.path(), vec![session], saved.path(), true);
    run_printing(&restarted, project.path(), id, &["new output"]);
    let lines = texts(&history::snapshot(&restarted, id));
    assert!(!holds(&lines, "old output"), "{lines:#?}");
    assert!(separators(&lines).is_empty(), "{lines:#?}");
    restarted.stop_session(id);
}

/// Story 2 scenario 8, FR-015 (U98): with saving off a stop and start in one service run still
/// shows the earlier output above the separator, and nothing is written.
#[test]
fn with_saving_off_a_stop_and_start_still_shows_the_history_and_writes_nothing() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session], saved.path(), false);
    run_printing(&state, project.path(), id, &["earlier"]);
    assert!(state.stop_session(id));

    run_printing(&state, project.path(), id, &["later"]);
    let lines = texts(&history::snapshot(&state, id));
    let separator = separators(&lines);
    assert_eq!(separator.len(), 1, "{lines:#?}");
    assert!(history::at(&lines, "earlier") < separator[0], "{lines:#?}");
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
    state.stop_session(id);
}

/// Edge case *Setting turned off while the service is not running* (U98): a service that starts
/// with the setting off deletes every file before a session can start.
#[test]
fn a_service_that_starts_with_saving_off_deletes_every_file_first() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    HistoryStore::new(saved.path().to_path_buf(), true)
        .save(
            id,
            &micold_core::terminal_history::HistorySnapshot {
                lines: vec![micold_core::terminal_history::LogicalLine {
                    text: "left behind".into(),
                    runs: vec![],
                }],
            },
        )
        .unwrap();
    std::fs::write(saved.path().join(format!(".{}.history.tmp", id.0)), b"half").unwrap();
    assert_eq!(files(saved.path()).len(), 2);

    let state = service_with(project.path(), vec![session], saved.path(), false);

    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
    run_printing(&state, project.path(), id, &["fresh"]);
    let lines = texts(&history::snapshot(&state, id));
    assert!(!holds(&lines, "left behind") && separators(&lines).is_empty());
    state.stop_session(id);
}

/// FR-033 (U100): a deletion that fails is logged once as a warning with the session and the
/// reason, and tried again every 30 s until it succeeds.
#[test]
fn a_failed_deletion_is_logged_once_and_retried_every_30_seconds() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_with(project.path(), vec![session], saved.path(), true);
    run_printing(&state, project.path(), id, &["kept"]);
    assert!(state.stop_session(id));
    let file = history_file(saved.path(), id);
    assert!(file.exists());
    let Some(lock) = undeletable(saved.path()) else {
        return; // root: a read-only directory stops nothing
    };

    lock(true);
    let t0 = Instant::now();
    let ((), log) = warnings_of(|| {
        state.set_save_terminal_history(false).unwrap();
        state.save_due_at(t0);
        state.save_due_at(t0 + SAVE_SPACING);
        state.save_due_at(t0 + 2 * SAVE_SPACING);
    });
    let named: Vec<_> = log.iter().filter(|l| l.contains(&id.0.to_string())).collect();
    assert_eq!(named.len(), 1, "one warning for the session: {log:#?}");
    assert!(
        named[0].contains("not deleted") && named[0].contains("ermission"),
        "it names the reason: {named:#?}"
    );
    assert!(file.exists(), "kept while it cannot be deleted");

    lock(false);
    state.save_due_at(t0 + 2 * SAVE_SPACING + SECOND);
    assert!(file.exists(), "not retried before 30 s have passed");
    state.save_due_at(t0 + 3 * SAVE_SPACING);
    assert!(!file.exists(), "retried at the next 30 s and deleted");
}

/// FR-027 (U99): the setting reaches the service as a `SettingsSet`, the files are gone before it
/// is answered, and every window is told.
#[tokio::test]
async fn a_settings_set_deletes_the_files_before_it_is_answered_and_is_broadcast() {
    let service = attention::Service::with_sessions(&[]);
    let saved = tempfile::tempdir().unwrap();
    let id = attention::session_id(0x41);
    let store = HistoryStore::new(saved.path().to_path_buf(), true);
    store
        .save(
            id,
            &micold_core::terminal_history::HistorySnapshot { lines: vec![] },
        )
        .unwrap();
    service.state.set_history_store(store);
    let mut window = attention::connect(&service.state, "window").await;
    let mut other = attention::connect(&service.state, "other").await;
    assert_eq!(files(saved.path()).len(), 1);

    window
        .send(Frame::Control(ClientMsg::SettingsSet {
            req: 5,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: Some(false),
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        }))
        .await
        .unwrap();
    // `SettingsChanged` is pushed before the answer, so it is among what comes up to it.
    let mut pushed = Vec::new();
    loop {
        match attention::next_frame(&mut window).await {
            Some(Frame::Control(DaemonMsg::OperationOk { req: 5, .. })) => break,
            Some(Frame::Control(DaemonMsg::SettingsChanged { settings })) => pushed.push(settings),
            Some(_) => {}
            None => panic!("the service closed the connection"),
        }
    }
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new(), "gone at the answer");
    assert!(
        pushed.iter().any(|s| !s.save_terminal_history),
        "the sender is told: {pushed:?}"
    );
    loop {
        match attention::next_frame(&mut other).await {
            Some(Frame::Control(DaemonMsg::SettingsChanged { settings })) => {
                assert!(!settings.save_terminal_history);
                break;
            }
            Some(_) => {}
            None => panic!("no SettingsChanged"),
        }
    }
}

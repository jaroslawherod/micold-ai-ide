//! Feature 041, milestone M3: a running terminal is saved at most every 30 seconds, and an idle
//! one never (story 1 scenario 7, story 3 scenario 6 as far as the retry; FR-003, FR-004, FR-005,
//! FR-007, FR-014, SC-002).
//!
//! The saver is driven by hand: `save_due_at(now)` is called with readings of a clock the test
//! moves, so no test waits 30 seconds. Output after the start is typed into the session, which
//! the terminal echoes. These run on Unix, where the echo is.
#![cfg(unix)]

#[path = "support/history.rs"]
mod history;

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use history::{
    ai_session, fake_cli, history_file, history_showing, script, service_saving, wait_file,
};
use micold_core::session::{SessionId, TerminalMode};
use micold_core::terminal::LaunchMode;
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

/// A running session in `project` whose stand-in has printed `first` and then waits on stdin; the
/// saver has been shown it once, at `at`, so its schedule is that of a terminal that just started.
fn running(state: &DaemonState, project: &Path, id: SessionId, first: &str, at: Instant) {
    let ready = project.join(format!("ready-{}", id.0));
    script(
        project,
        &format!("print {first}\ntouch {}\nwait\n", ready.display()),
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&ready);
    history_showing(state, id, first);
    state.save_due_at(at);
}

/// The next input serial: one counter for every session, so each session's serials only rise.
fn serial() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Type `text` into the session and wait until its echo is in the terminal's history.
fn type_line(state: &DaemonState, id: SessionId, text: &str) {
    state.session_input(id, serial(), format!("{text}\n").as_bytes());
    history_showing(state, id, text);
}

/// The lines of the saved history of `id` in `dir`, as a service that starts later would read it.
fn saved_lines(dir: &Path, id: SessionId) -> Vec<String> {
    match HistoryStore::new(dir.to_path_buf(), true).load(id) {
        LoadOutcome::History(snapshot) => history::texts(&snapshot),
        other => panic!("no saved history: {other:?}"),
    }
}

fn holds(lines: &[String], text: &str) -> bool {
    lines.iter().any(|l| l == text)
}

fn modified(dir: &Path, id: SessionId) -> std::time::SystemTime {
    std::fs::metadata(history_file(dir, id))
        .and_then(|m| m.modified())
        .expect("the file is there")
}

const SECOND: Duration = Duration::from_secs(1);

/// FR-003 (A7, U72): a printing terminal's file changes once per 30 s and holds what was printed
/// before the tick.
#[test]
fn u72_a_printing_terminal_is_saved_once_per_30_seconds_with_what_it_printed() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);

    type_line(&state, id, "one");
    state.save_due_at(t0 + 5 * SECOND);
    assert!(holds(&saved_lines(saved.path(), id), "one"));
    let first = modified(saved.path(), id);

    std::thread::sleep(Duration::from_millis(50));
    type_line(&state, id, "two");
    state.save_due_at(t0 + 34 * SECOND);
    assert_eq!(
        modified(saved.path(), id),
        first,
        "not 30 s after the last save"
    );
    assert!(!holds(&saved_lines(saved.path(), id), "two"));

    state.save_due_at(t0 + 35 * SECOND);
    assert!(
        holds(&saved_lines(saved.path(), id), "two"),
        "30 s after it"
    );
    assert_ne!(modified(saved.path(), id), first);
    state.stop_session(id);
}

/// FR-003 (A24, U73): output is on disk no later than 60 s after it was printed, with the saver
/// ticking every 5 s.
#[test]
fn u73_output_is_on_disk_within_60_seconds_of_being_printed() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);
    type_line(&state, id, "early");
    state.save_due_at(t0 + 5 * SECOND);

    // Printed at t0 + 6 s, just after a save, which is the worst case.
    type_line(&state, id, "late");
    let printed = 6;
    let landed = (printed..=printed + 60)
        .step_by(5)
        .find(|s| {
            state.save_due_at(t0 + Duration::from_secs(*s));
            holds(&saved_lines(saved.path(), id), "late")
        })
        .expect("on disk within 60 s");
    assert!(
        landed - printed <= 60,
        "landed {} s after it was printed",
        landed - printed
    );
    state.stop_session(id);
}

/// FR-004 (U74): an idle terminal's file is not rewritten.
#[test]
fn u74_an_idle_terminal_is_not_rewritten() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);
    type_line(&state, id, "once");
    state.save_due_at(t0 + 5 * SECOND);
    let written = modified(saved.path(), id);

    std::thread::sleep(Duration::from_millis(1100));
    for tick in (10..=600).step_by(5) {
        state.save_due_at(t0 + Duration::from_secs(tick));
    }
    assert_eq!(
        modified(saved.path(), id),
        written,
        "10 minutes without output"
    );
    state.stop_session(id);
}

/// Edge case *Several busy sessions* (U75): each is saved on its own schedule, with only its own
/// lines.
#[test]
fn u75_two_printing_sessions_are_each_saved_on_their_own_schedule() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions = vec![ai_session(), ai_session()];
    let (a, b) = (sessions[0].id, sessions[1].id);
    let state = service_saving(project.path(), sessions, saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), a, "start", t0);
    running(&state, project.path(), b, "start", t0);

    type_line(&state, a, "only-a");
    state.save_due_at(t0 + 5 * SECOND);
    assert!(holds(&saved_lines(saved.path(), a), "only-a"));
    assert!(!history_file(saved.path(), b).exists(), "b printed nothing");

    type_line(&state, b, "only-b");
    state.save_due_at(t0 + 10 * SECOND);
    assert!(holds(&saved_lines(saved.path(), b), "only-b"));
    assert!(!holds(&saved_lines(saved.path(), b), "only-a"));
    assert!(!holds(&saved_lines(saved.path(), a), "only-b"));
    for id in [a, b] {
        state.stop_session(id);
    }
}

/// FR-014 (U76): a Regular Terminal is never saved.
#[test]
fn u76_a_regular_terminal_is_never_saved() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let mut session = ai_session();
    session.set_mode(TerminalMode::Regular);
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    state.save_due_at(t0);
    state
        .primary_pty(id)
        .unwrap()
        .write_input(b"echo MARK-$((40+2))\r")
        .unwrap();
    history::history_once(&state, id, "MARK-42", |lines| {
        lines
            .iter()
            .any(|l| l.ends_with("MARK-42") && !l.contains("echo"))
    });
    for tick in [5, 40, 80] {
        state.save_due_at(t0 + Duration::from_secs(tick));
    }
    assert!(!history_file(saved.path(), id).exists());
    state.stop_session(id);
}

/// Story 1 scenario 7 (A7, U77, SC-002): the service is lost without an orderly stop; a restart
/// restores the history up to the last save.
#[test]
fn u77_a_service_lost_without_a_stop_restores_the_history_up_to_the_last_save() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);
    type_line(&state, id, "before the save");
    state.save_due_at(t0 + 5 * SECOND);
    type_line(&state, id, "after the save");
    state.save_due_at(t0 + 10 * SECOND);
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print back\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let lines = history_showing(&state, id, "back");
    assert!(holds(&lines, "before the save"), "{lines:#?}");
    assert!(!holds(&lines, "after the save"), "{lines:#?}");
    state.stop_session(id);
}

/// Story 3 scenario 6 (A24, U78): a save that fails leaves the session running, is one warning,
/// and is tried again 30 s later.
#[test]
fn u78_a_failed_save_is_one_warning_and_is_tried_again_30_seconds_later() {
    use std::os::unix::fs::PermissionsExt;
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);
    type_line(&state, id, "kept");

    let set_mode = |mode| {
        std::fs::set_permissions(saved.path(), std::fs::Permissions::from_mode(mode)).unwrap()
    };
    set_mode(0o500);
    if std::fs::File::create(saved.path().join("probe")).is_ok() {
        set_mode(0o700);
        eprintln!("skipped: the directory stays writable (running as root)");
        state.stop_session(id);
        return;
    }
    let ((), warnings) = warnings_of(|| {
        state.save_due_at(t0 + 5 * SECOND);
        state.save_due_at(t0 + 40 * SECOND);
    });
    assert!(state.primary_pty(id).unwrap().is_alive(), "still running");
    assert!(!history_file(saved.path(), id).exists());
    let named: Vec<_> = warnings
        .iter()
        .filter(|l| l.contains(&id.0.to_string()))
        .collect();
    assert_eq!(
        named.len(),
        1,
        "one warning naming the session, though two tries failed: {warnings:#?}"
    );

    set_mode(0o700);
    state.save_due_at(t0 + 69 * SECOND);
    assert!(
        !history_file(saved.path(), id).exists(),
        "not yet 30 s after the try"
    );
    state.save_due_at(t0 + 70 * SECOND);
    assert!(
        holds(&saved_lines(saved.path(), id), "kept"),
        "tried again after 30 s"
    );
    state.stop_session(id);
}

/// FR-005: input written and a resize sent while saves go on reach the stand-in.
#[test]
fn input_and_a_resize_during_saves_reach_the_process() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let t0 = Instant::now();
    running(&state, project.path(), id, "start", t0);

    let saver = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            for i in 1..=40u64 {
                state.save_due_at(t0 + Duration::from_secs(31 * i));
            }
        })
    };
    for i in 0..20 {
        state.session_input(id, serial(), format!("typed-{i}\n").as_bytes());
    }
    state.resize_session(id, 133, 41);
    saver.join().unwrap();

    let record = project.path().join("stdin-record");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let got = std::fs::read_to_string(&record).unwrap_or_default();
        if got.contains("typed-19") || Instant::now() >= deadline {
            let expected: String = (0..20).map(|i| format!("typed-{i}\n")).collect();
            assert_eq!(got, expected, "every line, in order");
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let columns = state.primary_pty(id).unwrap().term().lock().columns();
    assert_eq!(columns, 133, "the resize arrived");
    state.stop_session(id);
}

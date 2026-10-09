//! Feature 041, milestone M6: a damaged saved history never stops a session (story 3 scenarios 1
//! to 6; FR-007, FR-016, FR-017, FR-018, SC-006).
//!
//! "The service restarted" is a second `DaemonState` on the same history directory, as in
//! `history_service_restart.rs`. The saver is driven by hand, as in `history_periodic_save.rs`.
// unix-only: the stand-in and the typed echo are Unix ones, and the cases use file modes
#![cfg(unix)]

#[path = "support/history.rs"]
mod history;

use alacritty_terminal::grid::Dimensions;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use history::{
    ai_session, fake_cli, history_file, history_showing, script, separators, service_saving,
    snapshot, texts, wait_exited, wait_file,
};
use micold_core::session::SessionId;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::schedule::SAVE_SPACING;
use micold_core::terminal_history::text::notice_line;
use micold_daemon::logging::Logging;
use micold_daemon::state::DaemonState;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

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

/// Run `f` and return the warnings logged on this thread meanwhile, one per line, and the
/// diagnostics handle whose recent errors the same events filled.
fn warnings_of<T>(f: impl FnOnce() -> T) -> (T, Vec<String>, Logging) {
    let buffer = LogBuffer::default();
    let logging = Logging::in_memory();
    let subscriber = tracing_subscriber::registry()
        .with(logging.capture_layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(buffer.clone())
                .with_ansi(false)
                .with_filter(tracing_subscriber::filter::LevelFilter::WARN),
        );
    let result = tracing::subscriber::with_default(subscriber, f);
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    (result, text.lines().map(str::to_string).collect(), logging)
}

use tracing_subscriber::Layer as _;

/// The warnings among `log` that name `id`.
fn naming(log: &[String], id: SessionId) -> Vec<&String> {
    let id = id.0.to_string();
    log.iter().filter(|line| line.contains(&id)).collect()
}

/// Run `id` once in `project`, printing `text`, and stop it: its history is saved at the end.
fn run_and_stop(state: &DaemonState, project: &Path, id: SessionId, text: &str) {
    let printed = project.join("printed");
    let _ = std::fs::remove_file(&printed);
    script(project, &format!("print {text}\ntouch printed\nwait\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&printed);
    history_showing(state, id, text);
    assert!(state.stop_session(id));
}

/// 4 KiB of bytes that are not a history and hold a marker that must never be shown.
fn garbage() -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..4096u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    bytes[0] = b'X';
    bytes.extend_from_slice(b"GARBAGE-MARKER");
    bytes
}

/// Start `id` on a service over `saved` that is to skip its file: the session runs, the notice is
/// the only thing on its terminal, and one warning names the session and `reason`, which the
/// recent errors hold too. Returns the service.
fn assert_skipped(
    project: &Path,
    saved: &Path,
    session: &micold_core::session::Session,
    reason: &str,
) -> Arc<DaemonState> {
    let id = session.id;
    let state = service_saving(project, vec![session.clone()], saved);
    let ready = project.join("ready");
    let _ = std::fs::remove_file(&ready);
    script(project, &format!("touch {}\nwait\n", ready.display()));
    let (started, warnings, logging) = warnings_of(|| state.start_session(id, LaunchMode::Fresh));
    started.expect("starts as a session with no saved history does");
    wait_file(&ready);

    assert!(state.primary_pty(id).unwrap().is_alive(), "it runs");
    let lines = texts(&snapshot(&state, id));
    let notice = notice_line(state.primary_pty(id).unwrap().term().lock().columns());
    assert_eq!(
        lines.iter().filter(|l| !l.is_empty()).collect::<Vec<_>>(),
        [&notice],
        "one notice line and nothing else: {lines:#?}"
    );
    assert!(separators(&lines).is_empty(), "no separator");
    assert!(
        !lines.iter().any(|l| l.contains("GARBAGE")),
        "none of the file shows"
    );

    let named = naming(&warnings, id);
    assert_eq!(
        named.len(),
        1,
        "one warning naming the session: {warnings:#?}"
    );
    assert!(named[0].contains(reason), "{reason:?} in {:?}", named[0]);
    let recent = logging.recent_errors(10);
    assert_eq!(recent.len(), 1, "the diagnostics hold it: {recent:#?}");
    state
}

/// Story 3 scenarios 1 and 2 (A19, A20, SC-006): random bytes in place of the file.
#[test]
fn a19_a20_a_file_of_random_bytes_is_skipped_with_a_notice_and_one_warning() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    // The directory is made by the store on the first use; make it by saving once.
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "before");
    drop(state);
    std::fs::write(history_file(saved.path(), id), garbage()).unwrap();

    let state = assert_skipped(project.path(), saved.path(), &session, "not a history file");
    state.stop_session(id);
}

/// Story 3 scenario 3 (A21): a file the service may not read gives the same outcome.
#[test]
fn a21_an_unreadable_file_is_skipped_the_same_way() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "before");
    drop(state);
    let file = history_file(saved.path(), id);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&file).is_ok() {
        eprintln!("skipped: the file stays readable (running as root)");
        return;
    }

    let state = assert_skipped(project.path(), saved.path(), &session, "could not be read");
    state.stop_session(id);
}

/// A22: a file of another format version.
#[test]
fn a22_a_file_of_another_version_is_skipped_with_that_reason() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "before");
    drop(state);
    let file = history_file(saved.path(), id);
    let mut bytes = std::fs::read(&file).unwrap();
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    bytes[8..12].copy_from_slice(&(version + 1).to_le_bytes());
    std::fs::write(&file, bytes).unwrap();

    let state = assert_skipped(
        project.path(),
        saved.path(),
        &session,
        "written by another version",
    );
    state.stop_session(id);
}

/// Story 3 scenario 4 (A23): a damaged file of one session does not touch another's.
#[test]
fn a23_a_second_session_with_an_intact_file_shows_its_full_history() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (damaged, intact) = (ai_session(), ai_session());
    let state = service_saving(
        project.path(),
        vec![damaged.clone(), intact.clone()],
        saved.path(),
    );
    run_and_stop(&state, project.path(), damaged.id, "first");
    run_and_stop(&state, project.path(), intact.id, "second");
    drop(state);
    std::fs::write(history_file(saved.path(), damaged.id), garbage()).unwrap();

    let state = service_saving(
        project.path(),
        vec![damaged.clone(), intact.clone()],
        saved.path(),
    );
    script(project.path(), "print again\nwait\n");
    let ((), _, _) = warnings_of(|| {
        state.start_session(damaged.id, LaunchMode::Fresh).unwrap();
        state.start_session(intact.id, LaunchMode::Fresh).unwrap();
    });
    let lines = history_showing(&state, intact.id, "again");
    assert!(lines.iter().any(|l| l == "second"), "{lines:#?}");
    assert_eq!(separators(&lines).len(), 1);
    let lines = history_showing(&state, damaged.id, "again");
    assert!(!lines.iter().any(|l| l == "first"));
    state.stop_session(damaged.id);
    state.stop_session(intact.id);
}

/// Story 3 scenario 5 (A24, FR-018): the skipped file is replaced at the next tick with no new
/// output, and a later restart restores the notice and the new output.
#[test]
fn a24_after_a_skip_the_terminal_is_saved_at_the_next_tick_and_restored_later() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "before");
    drop(state);
    let file = history_file(saved.path(), id);
    std::fs::write(&file, garbage()).unwrap();

    let state = assert_skipped(project.path(), saved.path(), &session, "not a history file");
    state.save_due_at(Instant::now());
    let bytes = std::fs::read(&file).unwrap();
    assert!(
        bytes.starts_with(b"MICOLDTH"),
        "the file was replaced by a history, with no new output"
    );
    // New output, saved at the stop.
    static SERIAL: AtomicU64 = AtomicU64::new(1);
    state.session_input(id, SERIAL.fetch_add(1, Ordering::Relaxed), b"later\n");
    history_showing(&state, id, "later");
    assert!(state.stop_session(id));
    wait_exited_or_gone(&state, id);
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print newest\nwait\n");
    let ((), warnings, _) = warnings_of(|| {
        state.start_session(id, LaunchMode::Fresh).unwrap();
    });
    assert!(warnings.is_empty(), "nothing damaged now: {warnings:#?}");
    let lines = history_showing(&state, id, "newest");
    let notice = lines
        .iter()
        .position(|l| l.contains("earlier output"))
        .unwrap();
    let later = lines.iter().position(|l| l == "later").unwrap();
    assert!(notice < later, "the notice then the new output: {lines:#?}");
    state.stop_session(id);
}

fn wait_exited_or_gone(state: &DaemonState, id: SessionId) {
    if state.primary_pty(id).is_some() {
        wait_exited(state, id);
    }
}

/// Story 3 scenario 6 (U106, U107, FR-007): the same reason is logged once per service run, a
/// different reason again.
#[test]
fn u106_u107_a_save_failing_for_one_reason_is_logged_once_and_another_reason_again() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let saved = parent.path().join("history");
    std::fs::create_dir(&saved).unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], &saved);
    let ready = project.path().join("ready");
    script(
        project.path(),
        &format!("print start\ntouch {}\nwait\n", ready.display()),
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&ready);
    history_showing(&state, id, "start");
    let t0 = Instant::now();
    state.save_due_at(t0 - SAVE_SPACING);

    std::fs::set_permissions(&saved, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::File::create(saved.join("probe")).is_ok() {
        std::fs::set_permissions(&saved, std::fs::Permissions::from_mode(0o700)).unwrap();
        eprintln!("skipped: the directory stays writable (running as root)");
        state.stop_session(id);
        return;
    }
    let mut serial = 1000;
    let mut print = |text: &str| {
        serial += 1;
        state.session_input(id, serial, format!("{text}\n").as_bytes());
        history_showing(&state, id, text);
    };
    print("one");
    let ((), same, _) = warnings_of(|| {
        state.save_due_at(t0 + 5 * Duration::from_secs(1));
        state.save_due_at(t0 + 40 * Duration::from_secs(1));
    });
    assert_eq!(
        naming(&same, id).len(),
        1,
        "once for the same reason: {same:#?}"
    );

    // Another reason: the history directory is replaced by a file.
    std::fs::set_permissions(&saved, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(&saved).unwrap();
    std::fs::write(&saved, b"not a directory").unwrap();
    print("two");
    let ((), other, _) = warnings_of(|| {
        state.save_due_at(t0 + 80 * Duration::from_secs(1));
        state.save_due_at(t0 + 120 * Duration::from_secs(1));
    });
    assert_eq!(
        naming(&other, id).len(),
        1,
        "a different reason is logged again, once: {other:#?}"
    );
    assert!(state.primary_pty(id).unwrap().is_alive(), "still running");
    state.stop_session(id);
}

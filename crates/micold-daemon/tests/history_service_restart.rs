//! Feature 041, milestone M2: a terminal history saved at a process end is restored when the
//! session is started under a new session service (story 1 scenarios 2 to 6, FR-002, FR-007,
//! FR-014, FR-016, data-model §6).
//!
//! "The service restarted" is a second `DaemonState` with the same sessions in its catalog and a
//! new `HistoryStore` on the same directory: nothing of the first one's memory reaches it, only
//! the files. The directory is a temporary one in every test.
//!
//! As in `history_restart_in_run.rs`, the first scenario and the self-exit case run on every
//! platform and the rest on Unix.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

#[path = "support/history.rs"]
mod history;

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use history::{
    ai_session, at, fake_cli, history_file, history_showing, script, separators, service,
    service_saving, snapshot, texts, wait_exited, wait_file,
};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::ClientMsg;
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{SessionId, TerminalMode};
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{HistoryColor, HistoryStore, LoadOutcome};
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
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

/// Run `f` and return the warnings and errors logged on this thread meanwhile, one per line.
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

/// The warnings among `log` that name `id`.
fn naming(log: &[String], id: SessionId) -> Vec<&String> {
    let id = id.0.to_string();
    log.iter().filter(|line| line.contains(&id)).collect()
}

/// Run `id` once in `project` with `directives`, wait for the file `printed` its script touches,
/// and stop it.
fn run_and_stop(state: &DaemonState, project: &Path, id: SessionId, directives: &str) {
    let printed = project.join("printed");
    let _ = std::fs::remove_file(&printed);
    script(project, &format!("{directives}touch printed\nwait\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&printed);
    assert!(state.stop_session(id));
}

/// The terminal's history is exactly `limit` rows: the separator, and above it the most recent
/// `limit - 1` of the lines up to `line <last>`, in order and without a gap. The older ones are
/// absent, as they would be in a terminal that had printed them with this limit.
fn assert_most_recent(state: &DaemonState, id: SessionId, last: usize, limit: usize) {
    let lines = texts(&snapshot(state, id));
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator");
    let restored = &lines[..seps[0]];
    assert_eq!(
        restored.len() + 1,
        limit,
        "the restored lines and the separator fill the limit"
    );
    let first = last + 1 - restored.len();
    let expected: Vec<String> = (first..=last).map(|i| format!("line {i}")).collect();
    assert_eq!(restored, &expected[..], "the most recent lines, in order");
}

/// Story 1 scenarios 1 and 2 (A1, A2): 200 styled lines, a stop, a service restart and a start.
#[test]
fn a1_a2_after_a_service_restart_a_start_shows_the_lines_one_separator_and_the_new_output() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "lines 200 \\e[1;31mline \n");
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print new output\nwait\n");
    let before = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let after = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let lines = history_showing(&state, id, "new output");

    let first = at(&lines, "line 1");
    let expected: Vec<String> = (1..=200).map(|i| format!("line {i}")).collect();
    assert_eq!(
        lines[first..first + 200],
        expected[..],
        "all 200 lines, in order"
    );
    assert_eq!(
        separators(&lines),
        vec![first + 200],
        "one separator, right after them"
    );
    let separator = &lines[first + 200];
    assert!(
        separator.contains(&format!("session restarted at {before}"))
            || separator.contains(&format!("session restarted at {after}")),
        "the separator carries the time of this start ({before}): {separator}"
    );
    assert!(
        at(&lines, "new output") > first + 200,
        "the new output below"
    );
    let styled = &snapshot(&state, id).lines[first + 199].runs[0].style;
    assert_eq!(styled.fg, HistoryColor::Basic(1), "the colour is kept");
    assert!(
        styled
            .flags
            .contains(micold_core::terminal_history::StyleFlags::BOLD),
        "the style is kept"
    );
    state.stop_session(id);
}

/// Story 1 scenario 3 (A3): nothing was printed, so nothing is restored and no separator shows.
#[cfg(unix)]
#[test]
fn a3_a_session_that_printed_nothing_shows_no_separator_after_a_restart() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "");
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "touch again\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&project.path().join("again"));

    let lines = texts(&snapshot(&state, id));
    assert!(
        lines.iter().all(String::is_empty),
        "no separator and no blank history: {lines:#?}"
    );
    state.stop_session(id);
}

/// Story 1 scenario 4 (A4): two service restarts.
#[cfg(unix)]
#[test]
fn a4_a_second_restart_shows_output_separator_output_separator_in_order() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    for text in ["one", "two"] {
        let state = service_saving(project.path(), vec![session.clone()], saved.path());
        run_and_stop(&state, project.path(), id, &format!("print {text}\n"));
    }
    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print three\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let lines = history_showing(&state, id, "three");

    let seps = separators(&lines);
    assert_eq!(seps.len(), 2, "{lines:#?}");
    let (one, two, three) = (at(&lines, "one"), at(&lines, "two"), at(&lines, "three"));
    assert!(
        one < seps[0] && seps[0] < two && two < seps[1] && seps[1] < three,
        "{lines:#?}"
    );
    state.stop_session(id);
}

/// Story 1 scenario 5 (A5): the history was longer than the scrollback limit.
#[cfg(unix)]
#[test]
fn a5_a_history_longer_than_the_limit_restores_the_most_recent_lines() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    state.set_scrollback(100).unwrap();
    run_and_stop(&state, project.path(), id, "lines 400 line \n");
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    state.set_scrollback(100).unwrap();
    script(project.path(), "wait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");

    assert_most_recent(&state, id, 400, 100);
    state.stop_session(id);
}

/// Edge case *Scrollback limit changed* (U60): the whole history was saved, and a smaller limit is
/// in force at the restore.
#[cfg(unix)]
#[test]
fn u60_a_smaller_limit_at_the_restore_shows_the_most_recent_lines_up_to_it() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, "lines 400 line \n");
    drop(state);
    let LoadOutcome::History(file) = HistoryStore::new(saved.path().to_path_buf(), false).load(id)
    else {
        panic!("the stop saved no readable history");
    };
    assert_eq!(
        at(&texts(&file), "line 400") - at(&texts(&file), "line 1"),
        399
    );

    let state = service_saving(project.path(), vec![session], saved.path());
    state.set_scrollback(100).unwrap();
    script(project.path(), "wait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");

    assert_most_recent(&state, id, 400, 100);
    state.stop_session(id);
}

/// Story 1 scenario 6, FR-025 (A6).
#[cfg(unix)]
#[test]
fn a6_two_sessions_each_show_only_their_own_history_after_a_restart() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions = vec![ai_session(), ai_session()];
    let (a, b) = (sessions[0].id, sessions[1].id);
    let own =
        |id: SessionId| move |l: &String| l.starts_with("args") && l.contains(&id.0.to_string());

    let state = service_saving(project.path(), sessions.clone(), saved.path());
    for id in [a, b] {
        run_and_stop(&state, project.path(), id, "args\n");
    }
    drop(state);

    let state = service_saving(project.path(), sessions, saved.path());
    script(project.path(), "print restarted\nwait\n");
    for id in [a, b] {
        state.start_session(id, LaunchMode::Fresh).expect("starts");
    }
    for (id, other) in [(a, b), (b, a)] {
        let lines = history_showing(&state, id, "restarted");
        let sep = separators(&lines);
        assert_eq!(sep.len(), 1, "{lines:#?}");
        assert!(
            lines[..sep[0]].iter().any(own(id)),
            "its own earlier line above the separator: {lines:#?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains(&other.0.to_string())),
            "nothing of the other session: {lines:#?}"
        );
    }
    for id in [a, b] {
        state.stop_session(id);
    }
}

/// FR-002 (U61): a process that exits by itself is saved at that exit.
#[test]
fn u61_a_process_that_exits_by_itself_is_saved_at_the_exit_and_restored() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;

    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    script(
        project.path(),
        "print before exit A\nprint the last line\nexit 0\n",
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_exited(&state, id);
    state.supervise_exited_sessions();
    assert!(
        history_file(saved.path(), id).is_file(),
        "saved by the tick that saw the exit, with no stop request"
    );
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print after restart\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let lines = history_showing(&state, id, "after restart");

    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(
        at(&lines, "before exit A") < at(&lines, "the last line")
            && at(&lines, "the last line") < seps[0]
            && seps[0] < at(&lines, "after restart"),
        "earlier output with its last line, separator, new output: {lines:#?}"
    );
    state.stop_session(id);
}

/// FR-014 (U62): a Regular Terminal has no saved history; an AI CLI session beside it has one.
#[cfg(unix)]
#[test]
fn u62_a_regular_terminal_has_no_file_after_a_stop() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let mut regular = ai_session();
    regular.set_mode(TerminalMode::Regular);
    let ai = ai_session();
    let (regular_id, ai_id) = (regular.id, ai.id);
    let state = service_saving(project.path(), vec![regular, ai], saved.path());

    state
        .start_session(regular_id, LaunchMode::Fresh)
        .expect("starts");
    state
        .primary_pty(regular_id)
        .unwrap()
        .write_input(b"echo MARK-$((40+2))\r")
        .unwrap();
    history::history_once(&state, regular_id, "MARK-42", |lines| {
        lines
            .iter()
            .any(|l| l.ends_with("MARK-42") && !l.contains("echo"))
    });
    assert!(state.stop_session(regular_id));
    run_and_stop(&state, project.path(), ai_id, "print hi\n");

    let files: Vec<_> = std::fs::read_dir(saved.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(
        files,
        vec![history_file(saved.path(), ai_id)],
        "one file, the AI CLI session's"
    );
}

/// 4 KiB that are no saved history, with a readable marker among them.
fn random_bytes() -> Vec<u8> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut bytes: Vec<u8> = (0..4096)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect();
    bytes.extend_from_slice(b"\r\nDAMAGED-CONTENT\r\n");
    bytes
}

/// Story 3 scenario 1, FR-016 (A19): a saved history of random bytes.
#[cfg(unix)]
#[test]
fn a19_a_file_of_random_bytes_starts_the_session_with_no_history_and_one_warning() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    std::fs::write(history_file(saved.path(), id), random_bytes()).unwrap();

    let state = service_saving(project.path(), vec![session], saved.path());
    script(project.path(), "print new output\nwait\n");
    let (started, log) = warnings_of(|| state.start_session(id, LaunchMode::Fresh));
    started.expect("the session starts");
    let lines = history_showing(&state, id, "new output");

    assert_eq!(
        lines.iter().filter(|l| !l.is_empty()).collect::<Vec<_>>(),
        vec!["new output"],
        "as a session with no saved history: no separator, nothing of the file"
    );
    let warned = naming(&log, id);
    assert_eq!(warned.len(), 1, "one warning naming the session: {log:#?}");
    assert!(
        warned[0].contains("WARN") && warned[0].contains("saved terminal history"),
        "{warned:#?}"
    );
    state.stop_session(id);
}

/// Data-model §6, R4 (U63): while a carried history exists the file is not read. The file is
/// replaced by one that cannot be read; the start shows the carried lines and warns of nothing.
#[cfg(unix)]
#[test]
fn u63_a_file_is_not_read_while_a_carried_history_exists() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());

    run_and_stop(&state, project.path(), id, "print carried line\n");
    let file = history_file(saved.path(), id);
    assert!(file.is_file(), "the stop saved the history");
    std::fs::write(&file, random_bytes()).unwrap();

    script(project.path(), "print new output\nwait\n");
    let (started, log) = warnings_of(|| state.start_session(id, LaunchMode::Fresh));
    started.expect("starts");
    let lines = history_showing(&state, id, "new output");

    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(at(&lines, "carried line") < seps[0], "{lines:#?}");
    assert!(
        naming(&log, id).is_empty(),
        "the file was not read: {log:#?}"
    );
    state.stop_session(id);
}

/// FR-007: a save that fails is one warning naming the session and the reason, and stops nothing:
/// the stop succeeds and the next start in the same run still shows the history.
#[cfg(unix)]
#[test]
fn a_failed_save_is_one_warning_and_the_stop_and_the_next_start_go_on() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    // The history directory cannot be made: a file has its name.
    let blocked = saved.path().join("terminal-history");
    std::fs::write(&blocked, b"").unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], &blocked);

    let printed = project.path().join("printed");
    script(project.path(), "print kept\ntouch printed\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&printed);
    let (stopped, log) = warnings_of(|| state.stop_session(id));
    assert!(stopped, "the stop is not failed by the save");
    let warned = naming(&log, id);
    assert_eq!(warned.len(), 1, "one warning naming the session: {log:#?}");
    assert!(
        warned[0].contains("WARN")
            && warned[0].contains("terminal history was not saved")
            && warned[0].contains("reason="),
        "{warned:#?}"
    );

    script(project.path(), "print new output\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = history_showing(&state, id, "new output");
    assert!(at(&lines, "kept") < separators(&lines)[0], "{lines:#?}");
    state.stop_session(id);
}

/// R15: in a container whose history directory was not made, a save is skipped. That is not a
/// failure: nothing is logged for the session and nothing is created.
#[cfg(unix)]
#[test]
fn a_skipped_save_is_not_a_warning() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let absent = saved.path().join("terminal-history");
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);
    state.set_history_store(HistoryStore::new(absent.clone(), false));

    let printed = project.path().join("printed");
    script(project.path(), "print hi\ntouch printed\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&printed);
    let (stopped, log) = warnings_of(|| state.stop_session(id));

    assert!(stopped);
    assert!(naming(&log, id).is_empty(), "{log:#?}");
    assert!(!absent.exists(), "the directory was not created");
}

/// A window's `SessionStop` followed at once by its `SessionStart`, over a real connection: the
/// stop saves and the start seeds under the session's gate, each on a blocking thread, and the
/// second waits for the first (D17). On a current-thread runtime, where a stop or a start that
/// blocked the runtime's one thread while the other held the gate would hang for good.
#[tokio::test(flavor = "current_thread")]
async fn a_stop_then_a_start_over_a_connection_saves_and_restores_in_that_order() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());

    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();

    let printed = project.path().join("printed");
    script(project.path(), "print first run\ntouch printed\nwait\n");
    client
        .send(Frame::Control(ClientMsg::SessionStart { session: id }))
        .await
        .unwrap();
    until("the first run printed", || printed.exists(), &mut client).await;

    script(project.path(), "print second run\nwait\n");
    client
        .send(Frame::Control(ClientMsg::SessionStop { session: id }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::SessionStart { session: id }))
        .await
        .unwrap();
    let shown = |state: &DaemonState| {
        state
            .primary_pty(id)
            .map(|pty| texts(&micold_daemon::history::capture(&pty.term().lock())))
            .filter(|lines| lines.iter().any(|l| l == "second run"))
    };
    until(
        "the second run printed",
        || shown(&state).is_some(),
        &mut client,
    )
    .await;

    let lines = shown(&state).unwrap();
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(
        at(&lines, "first run") < seps[0] && seps[0] < at(&lines, "second run"),
        "{lines:#?}"
    );
    let LoadOutcome::History(file) = HistoryStore::new(saved.path().to_path_buf(), false).load(id)
    else {
        panic!("the stop saved no readable history");
    };
    assert!(texts(&file).iter().any(|l| l == "first run"));
    // Off the runtime's one thread: `stop_session` blocks on the session's gate, and the start's
    // continuation that releases the gate runs on this thread. Called here directly, a stop that
    // arrives before that continuation has run waits on it forever (#617).
    tokio::task::spawn_blocking(move || state.stop_session(id))
        .await
        .unwrap();
}

/// Read what the service sends until `done` holds, within 20 s.
async fn until(
    what: &str,
    mut done: impl FnMut() -> bool,
    client: &mut Framed<tokio::io::DuplexStream, ClientCodec>,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(tokio::time::Instant::now() < deadline, "{what}: never");
        match tokio::time::timeout(Duration::from_millis(20), client.next()).await {
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
            Ok(Some(Ok(_))) | Err(_) => {}
        }
    }
}

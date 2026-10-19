//! Feature 938, milestone M7 (T054): a structured session's event history is saved under **Save
//! terminal history** and restored after a daemon restart (US7, FR-021a–FR-021c, FR-025, SC-007).
//!
//! The session runs the idle stand-in of `support/structured.rs`; its events are pushed straight
//! into its stream, the call its stdout reader makes for each line. "The daemon restarted" is a
//! second `DaemonState` with the same session in its catalog and the same history directory.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::session::{Session, SessionId, SessionLocation};
use micold_core::structured::{StructuredEvent, MAX_EVENTS};
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{EventsLoad, HistoryStore};
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use history::{logged_warnings, service, service_saving};
use structured::{
    events_until, fake_stream_cli, stream_script, structured_session, viewer, Received,
};

fn line(text: &str) -> StructuredEvent {
    StructuredEvent::Unreadable {
        text: text.to_owned(),
    }
}

fn texts(events: &[Received]) -> Vec<String> {
    events
        .iter()
        .map(|r| match &r.event.event {
            StructuredEvent::Unreadable { text } => text.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn seqs(events: &[Received]) -> Vec<u64> {
    events.iter().map(|r| r.event.seq).collect()
}

fn events_file(dir: &Path, id: SessionId) -> PathBuf {
    dir.join(format!("{}.events", id.0))
}

/// The `.events` files in `dir`, saved or temporary.
fn event_files(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(".events"))
        .collect()
}

/// One session, its project and the history directory every daemon of a case shares.
struct Case {
    project: tempfile::TempDir,
    history: tempfile::TempDir,
    session: Session,
}

impl Case {
    fn new() -> Case {
        fake_stream_cli();
        let case = Case {
            project: tempfile::tempdir().unwrap(),
            history: tempfile::tempdir().unwrap(),
            session: structured_session(SessionLocation::Default),
        };
        stream_script(case.project.path(), "");
        case
    }

    fn id(&self) -> SessionId {
        self.session.id
    }

    fn dir(&self) -> &Path {
        self.history.path()
    }

    /// A daemon over the case's catalog and history directory, saving on.
    fn daemon(&self) -> Arc<DaemonState> {
        service_saving(
            self.project.path(),
            vec![self.session.clone()],
            self.history.path(),
        )
    }

    /// A daemon whose setting is off from its start (every file purged), over the same directory.
    fn daemon_not_saving(&self) -> Arc<DaemonState> {
        let state = service(self.project.path(), vec![self.session.clone()]);
        state.set_save_terminal_history(false).unwrap();
        state.set_history_store(HistoryStore::new(self.history.path().to_path_buf(), true));
        state
    }

    fn start(&self, state: &DaemonState) -> Arc<micold_daemon::structured::EventStream> {
        state
            .start_session(self.id(), LaunchMode::Fresh)
            .expect("starts");
        state.event_stream(self.id()).expect("a stream")
    }

    /// What a store on the directory reads for the session.
    fn saved(&self) -> EventsLoad {
        HistoryStore::new(self.history.path().to_path_buf(), false).load_events(self.id())
    }
}

fn push(stream: &micold_daemon::structured::EventStream, texts: &[&str]) {
    for text in texts {
        stream.push(line(text));
    }
}

/// A viewer of `id` in `state`, and the `n` earliest events it receives.
async fn view(state: &Arc<DaemonState>, case: &Case, n: usize) -> Vec<Received> {
    let mut client = viewer(state, case.project.path(), case.id()).await;
    events_until(&mut client, case.id(), |got| got.len() >= n).await
}

// US7 scenario 1, SC-007: an orderly stop saves; a new daemon's viewer gets the events with their
// seqs, then the resumed run's follow with the numbering continuing.
#[tokio::test(flavor = "multi_thread")]
async fn an_orderly_stop_saves_and_a_restart_restores_with_seq_continuing() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a", "b", "c"]);
    micold_daemon::history::save_all_live(&state, micold_daemon::history::STOP_SAVE_BOUND).await;
    state.stop_session(case.id());
    drop(state);

    let restarted = case.daemon();
    let mut client = viewer(&restarted, case.project.path(), case.id()).await;
    let got = events_until(&mut client, case.id(), |got| got.len() >= 3).await;
    assert_eq!(texts(&got), ["a", "b", "c"]);
    assert_eq!(seqs(&got), [0, 1, 2]);
    assert!(got.iter().all(|r| !r.live));

    let stream = case.start(&restarted);
    push(&stream, &["d", "e"]);
    let more = events_until(&mut client, case.id(), |got| got.len() >= 2).await;
    assert_eq!(texts(&more), ["d", "e"]);
    assert_eq!(seqs(&more), [3, 4], "no gap and no duplicate at the seam");
    restarted.stop_session(case.id());
}

// US7 scenario 2: a process end saves; the daemon dropped without an orderly stop is restored.
#[tokio::test(flavor = "multi_thread")]
async fn a_process_end_saves_and_a_daemon_dropped_without_a_stop_is_restored() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a", "b"]);
    assert!(state.stop_session(case.id()));
    assert!(events_file(case.dir(), case.id()).exists());
    drop(state);

    let got = view(&case.daemon(), &case, 2).await;
    assert_eq!(texts(&got), ["a", "b"]);
    assert_eq!(seqs(&got), [0, 1]);
}

// US7 scenario 3, SC-007: saving off writes nothing, and a restart starts at the resume.
#[tokio::test(flavor = "multi_thread")]
async fn with_saving_off_nothing_is_written_and_a_restart_starts_at_the_resume() {
    let case = Case::new();
    let state = case.daemon_not_saving();
    let stream = case.start(&state);
    push(&stream, &["a", "b"]);
    state.save_due_at(Instant::now());
    micold_daemon::history::save_all_live(&state, micold_daemon::history::STOP_SAVE_BOUND).await;
    state.stop_session(case.id());
    assert!(
        event_files(case.dir()).is_empty(),
        "{:?}",
        event_files(case.dir())
    );
    drop(state);

    let restarted = case.daemon_not_saving();
    let stream = case.start(&restarted);
    push(&stream, &["fresh"]);
    let got = view(&restarted, &case, 1).await;
    assert_eq!(
        (texts(&got), seqs(&got)),
        (vec!["fresh".to_owned()], vec![0])
    );
    restarted.stop_session(case.id());
    assert!(event_files(case.dir()).is_empty());
}

// US7 scenarios 4 and 9: turning the setting off deletes every file at once and leaves the history
// in memory as it is; turning it on again saves the whole running history.
#[tokio::test(flavor = "multi_thread")]
async fn turning_saving_off_deletes_the_files_and_on_again_saves_the_running_history() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a", "b"]);
    state.save_due_at(Instant::now());
    assert!(events_file(case.dir(), case.id()).exists());

    state.set_save_terminal_history(false).unwrap();
    assert!(event_files(case.dir()).is_empty());
    let got = view(&state, &case, 2).await;
    assert_eq!(texts(&got), ["a", "b"], "the memory history is unchanged");

    push(&stream, &["c"]);
    state.save_due_at(Instant::now() + Duration::from_secs(120));
    assert!(event_files(case.dir()).is_empty(), "off writes nothing");

    state.set_save_terminal_history(true).unwrap();
    state.save_due_at(Instant::now() + Duration::from_secs(240));
    match case.saved() {
        EventsLoad::Events { omitted, events } => {
            assert_eq!(
                (omitted, events.len()),
                (0, 3),
                "the whole history, earlier events too"
            );
        }
        other => panic!("saved: {other:?}"),
    }
    state.stop_session(case.id());
}

// US7 scenario 5.
#[tokio::test(flavor = "multi_thread")]
async fn removing_the_session_deletes_its_events_file() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a"]);
    state.save_due_at(Instant::now());
    assert!(events_file(case.dir(), case.id()).exists());
    let (_, ptys) = state.delete_session(case.id()).expect("deleted");
    drop(ptys);
    assert!(event_files(case.dir()).is_empty());
    // A save racing the removal leaves nothing either (scenario 8).
    state.save_due_at(Instant::now() + Duration::from_secs(120));
    assert!(event_files(case.dir()).is_empty());
}

// US7 scenario 6, FR-021c: a damaged file gives an empty history, one warning naming the session and
// the reason, and the next save replaces it.
#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_file_gives_an_empty_history_one_warning_and_is_replaced() {
    let case = Case::new();
    std::fs::write(
        events_file(case.dir(), case.id()),
        b"MICOLDSE not a real file at all.....",
    )
    .unwrap();
    let state = case.daemon();
    let (stream, log) = logged_warnings(|| case.start(&state));
    let named: Vec<_> = log
        .iter()
        .filter(|l| l.contains(&case.id().0.to_string()))
        .collect();
    assert_eq!(named.len(), 1, "{log:?}");
    assert!(named[0].contains("not a history file"), "{named:?}");
    push(&stream, &["a"]);
    let got = view(&state, &case, 1).await;
    assert_eq!(seqs(&got), [0], "an empty history: the first event is 0");
    state.save_due_at(Instant::now());
    assert_eq!(
        case.saved(),
        EventsLoad::Events {
            omitted: 0,
            events: vec![line("a")]
        }
    );
    state.stop_session(case.id());
}

// US7 scenario 6, through the view path: a damaged file of a stopped session is not shown either.
#[tokio::test(flavor = "multi_thread")]
async fn a_viewer_of_a_stopped_session_with_a_damaged_file_gets_no_events() {
    let case = Case::new();
    std::fs::write(events_file(case.dir(), case.id()), b"junk").unwrap();
    let state = case.daemon();
    let mut client = viewer(&state, case.project.path(), case.id()).await;
    let messages = structured::messages_until(&mut client, case.id(), |m| !m.is_empty()).await;
    assert!(messages.iter().all(|m| m.events.is_empty()));
}

// US7 scenario 7, FR-021b: at the daemon's start the file of a session not in the catalog and its
// leftover temporary file are deleted; with saving off at start every file is.
#[tokio::test(flavor = "multi_thread")]
async fn the_start_deletes_the_events_of_unknown_sessions_and_all_of_them_with_saving_off() {
    let case = Case::new();
    let other = SessionId(uuid::Uuid::from_u128(
        0xffff_2c3d_4e5f_4a6b_8c7d_9e0f_1a2b_3c4d,
    ));
    let kept = events_file(case.dir(), case.id());
    let orphan = events_file(case.dir(), other);
    let orphan_tmp = case.dir().join(format!(".{}.events.tmp", other.0));
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a"]);
    state.save_due_at(Instant::now());
    state.stop_session(case.id());
    drop(state);
    std::fs::copy(&kept, &orphan).unwrap();
    std::fs::write(&orphan_tmp, b"x").unwrap();

    let restarted = case.daemon();
    restarted.sweep_saved_histories();
    assert!(kept.exists() && !orphan.exists() && !orphan_tmp.exists());
    drop(restarted);

    let _off = case.daemon_not_saving();
    assert!(event_files(case.dir()).is_empty());
}

// One saver pass writes the file of a running session after a new event and none for a session with
// no new event since its last save; a terminal session writes no events file and a structured one
// no history file.
#[tokio::test(flavor = "multi_thread")]
async fn a_saver_pass_writes_only_after_a_new_event_and_only_the_events_kind() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    state.save_due_at(Instant::now());
    push(&stream, &["a"]);
    state.save_due_at(Instant::now() + Duration::from_secs(60));
    let file = events_file(case.dir(), case.id());
    assert!(file.exists());
    std::fs::remove_file(&file).unwrap();
    state.save_due_at(Instant::now() + Duration::from_secs(120));
    assert!(!file.exists(), "no new event since the last save");
    push(&stream, &["b"]);
    state.save_due_at(Instant::now() + Duration::from_secs(180));
    assert!(file.exists());
    let names = std::fs::read_dir(case.dir()).unwrap().count();
    assert_eq!(names, 1, "no .history file for a structured session");
    state.stop_session(case.id());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_history_over_the_bound_is_restored_under_the_bounds() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    for n in 0..MAX_EVENTS + 3 {
        stream.push(line(&n.to_string()));
    }
    state.stop_session(case.id());
    drop(state);
    match case.saved() {
        EventsLoad::Events { omitted, events } => {
            assert_eq!((omitted, events.len()), (3, MAX_EVENTS));
        }
        other => panic!("{other:?}"),
    }
}

// A restored history the file already holds is not written again until a new event comes.
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_history_is_saved_again_only_after_a_new_event() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a"]);
    state.stop_session(case.id());
    drop(state);

    let restarted = case.daemon();
    let got = view(&restarted, &case, 1).await;
    assert_eq!(texts(&got), ["a"]);
    let file = events_file(case.dir(), case.id());
    std::fs::remove_file(&file).unwrap();
    restarted.save_due_at(Instant::now() + Duration::from_secs(60));
    assert!(!file.exists(), "nothing new since the file was read");
    let stream = case.start(&restarted);
    push(&stream, &["b"]);
    restarted.save_due_at(Instant::now() + Duration::from_secs(120));
    assert!(file.exists());
    restarted.stop_session(case.id());
}

// A session removed does not get a stream made for it, from a file or empty.
#[tokio::test(flavor = "multi_thread")]
async fn no_stream_is_made_for_a_removed_session() {
    let case = Case::new();
    let state = case.daemon();
    let stream = case.start(&state);
    push(&stream, &["a"]);
    state.stop_session(case.id());
    let (_, ptys) = state.delete_session(case.id()).expect("deleted");
    drop(ptys);
    assert!(state.event_stream_loading(case.id(), true).is_none());
    assert!(state.event_stream_loading(case.id(), false).is_none());
    assert!(state.event_stream(case.id()).is_none());
}

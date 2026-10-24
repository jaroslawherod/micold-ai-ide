//! Feature 941, milestone M3 (T036): with **Save terminal history** on, a structured session's queue
//! survives a daemon restart, restored on hold (US4 AS4, FR-021, FR-022, research R9); off, it lives
//! only in the daemon (Q4).
//!
//! "The daemon restarted" is a second `DaemonState` with the same session in its catalog and the
//! same history directory, as in `structured_saved_history.rs`.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, OperationResult, QueueEntryId, QueuedSummary, TurnPhase,
};
use micold_core::session::{Session, SessionId, SessionLocation};
use micold_core::structured::attachment::{admit, ImageAttachment};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{HistoryStore, QueueLoad};
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use conn::Client;
use history::{service, service_saving};
use structured::{fake_stream_cli, fixture, stream_script, structured_session};

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
        // Each message opens a turn the test never ends: every later send is queued.
        stream_script(
            case.project.path(),
            &format!(
                "on-message-gated {}\n",
                fixture("text-turn.jsonl").display()
            ),
        );
        case
    }

    fn id(&self) -> SessionId {
        self.session.id
    }

    /// A daemon over the case's catalog and history directory, saving on, its session started.
    fn daemon(&self) -> Arc<DaemonState> {
        let state = service_saving(
            self.project.path(),
            vec![self.session.clone()],
            self.history.path(),
        );
        self.start(&state);
        state
    }

    /// A daemon whose setting is off from its start, over the same directory, its session started.
    fn daemon_not_saving(&self) -> Arc<DaemonState> {
        let state = service(self.project.path(), vec![self.session.clone()]);
        state.set_save_terminal_history(false).unwrap();
        state.set_history_store(HistoryStore::new(self.history.path().to_path_buf(), true));
        self.start(&state);
        state
    }

    fn start(&self, state: &Arc<DaemonState>) {
        state
            .start_session(self.id(), LaunchMode::Fresh)
            .expect("starts");
        micold_daemon::server::spawn_input_driver(Arc::clone(state));
    }

    fn queue_file(&self) -> PathBuf {
        self.history.path().join(format!("{}.queue", self.id().0))
    }

    /// What a store on the directory reads for the session's queue.
    fn saved(&self) -> QueueLoad {
        HistoryStore::new(self.history.path().to_path_buf(), false).load_queue(self.id())
    }

    /// The saved queue's texts, `None` for no file.
    fn saved_texts(&self) -> Option<Vec<String>> {
        match self.saved() {
            QueueLoad::Queue(entries) => Some(entries.into_iter().map(|m| m.text).collect()),
            QueueLoad::None => None,
            QueueLoad::Damaged(reason) => panic!("the saved queue is damaged: {reason}"),
        }
    }

    /// Wait, within 10 s, until the saved queue's texts are `expected` (`None`: no file).
    async fn saved_is(&self, expected: Option<&[&str]>) {
        let expected: Option<Vec<String>> =
            expected.map(|texts| texts.iter().map(|t| t.to_string()).collect());
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let got = self.saved_texts();
            if got == expected {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "saved {got:?}, expected {expected:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

/// A PNG picture named `name` (the bytes only need its magic number).
fn png(name: &str, fill: u8) -> ImageAttachment {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend(std::iter::repeat_n(fill, 64));
    admit(&[], name, bytes).expect("a PNG is admitted")
}

fn message(req: u64, session: SessionId, text: &str, images: Vec<ImageAttachment>) -> ClientMsg {
    ClientMsg::SessionUserMessage {
        req,
        session,
        text: text.into(),
        images,
    }
}

/// Send `msg` and wait for its answer.
async fn request(client: &mut Client, msg: ClientMsg, req: u64) -> DaemonMsg {
    client.send(Frame::Control(msg)).await.expect("sent");
    conn::answer(client, req).await
}

async fn ack(client: &mut Client, msg: ClientMsg, req: u64) {
    let answer = request(client, msg, req).await;
    assert!(
        matches!(
            answer,
            DaemonMsg::OperationOk {
                result: OperationResult::Ack,
                ..
            }
        ),
        "not acknowledged: {answer:?}"
    );
}

/// The first `StructuredInput` of `id` that satisfies `done`, as `(phase, hold, queue)`, within
/// 10 s.
async fn input_until(
    client: &mut Client,
    id: SessionId,
    what: &str,
    done: impl Fn(&TurnPhase, bool, &[QueuedSummary]) -> bool,
) -> (TurnPhase, bool, Vec<QueuedSummary>) {
    let wait = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Frame::Control(DaemonMsg::StructuredInput {
                session,
                phase,
                hold,
                queue,
            }) = client.next().await.expect("stream open").unwrap()
            {
                if session == id && done(&phase, hold, &queue) {
                    return (phase, hold, queue);
                }
            }
        }
    });
    wait.await
        .unwrap_or_else(|_| panic!("{what}: never happened"))
}

fn texts(queue: &[QueuedSummary]) -> Vec<&str> {
    queue.iter().map(|q| q.text.as_str()).collect()
}

/// A window on `state` that sent `m1` (delivered, its turn kept open) and queued `queued`.
async fn queued(
    state: &Arc<DaemonState>,
    id: SessionId,
    queued: &[(&str, Vec<ImageAttachment>)],
) -> Client {
    let mut client = conn::connect(state).await;
    input_until(&mut client, id, "idle", |p, _, _| *p == TurnPhase::Idle).await;
    ack(&mut client, message(1, id, "m1", vec![]), 1).await;
    for (n, (text, images)) in queued.iter().enumerate() {
        let req = n as u64 + 2;
        ack(&mut client, message(req, id, text, images.clone()), req).await;
    }
    client
}

/// The stand-in's user lines in `dir`.
fn delivered(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("stdin-record"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("\"type\":\"user\""))
        .count()
}

/// US4 AS4, FR-021, Q1: with saving on, a queue with pictures survives a daemon restart, restored
/// on hold, nothing delivered, its entries with fresh ids and their pictures.
#[tokio::test(flavor = "multi_thread")]
async fn a_saved_queue_survives_a_restart_on_hold() {
    let case = Case::new();
    let state = case.daemon();
    let shot = png("shot.png", 7);
    let _client = queued(
        &state,
        case.id(),
        &[("q1", vec![shot.clone()]), ("q2", vec![])],
    )
    .await;
    case.saved_is(Some(&["q1", "q2"])).await;
    state.stop_session(case.id());
    drop(state);
    structured::clear_records(case.project.path());

    let restarted = case.daemon();
    let mut client = conn::connect(&restarted).await;
    let (phase, hold, queue) = input_until(&mut client, case.id(), "running, held", |p, _, q| {
        *p == TurnPhase::Idle && !q.is_empty()
    })
    .await;
    assert_eq!(phase, TurnPhase::Idle);
    assert!(hold, "restored on hold");
    assert_eq!(texts(&queue), ["q1", "q2"], "in order");
    assert_eq!(queue[0].attachments[0].name, "shot.png");
    assert_ne!(queue[0].id, queue[1].id, "fresh ids, one each");

    let answer = request(
        &mut client,
        ClientMsg::QueueTake {
            req: 9,
            session: case.id(),
            entry: queue[0].id,
        },
        9,
    )
    .await;
    match answer {
        DaemonMsg::OperationOk {
            result: OperationResult::Taken { text, images },
            ..
        } => {
            assert_eq!(text, "q1");
            assert_eq!(images, [shot], "the picture's bytes are restored");
        }
        other => panic!("not taken: {other:?}"),
    }
    // The take's answer orders after the restart's pump: the hold seen above is what kept the
    // head in, so no sleep is needed. The hold is also the state a release would clear.
    assert_eq!(
        delivered(case.project.path()),
        0,
        "nothing delivered on hold"
    );
    restarted.stop_session(case.id());
}

/// FR-022, Q1: the hold is restored with the queue itself, not only by the process start that
/// follows (`on_process_start` holds a non-empty queue too, so the restart test above cannot tell
/// the two apart): a daemon whose session is not started shows the saved queue already held.
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_queue_is_held_before_any_process_starts() {
    let case = Case::new();
    let state = case.daemon();
    let _client = queued(&state, case.id(), &[("q1", vec![]), ("q2", vec![])]).await;
    case.saved_is(Some(&["q1", "q2"])).await;
    state.stop_session(case.id());
    drop(state);

    let restarted = service_saving(
        case.project.path(),
        vec![case.session.clone()],
        case.history.path(),
    );
    // Loading the session's saved stream is what restores its queue; no process is started.
    let loader = Arc::clone(&restarted);
    let id = case.id();
    tokio::task::spawn_blocking(move || loader.event_stream_loading(id, true))
        .await
        .unwrap()
        .expect("a stream");
    let mut client = conn::connect(&restarted).await;
    let (phase, hold, queue) = input_until(&mut client, case.id(), "restored, held", |_, _, q| {
        !q.is_empty()
    })
    .await;
    assert_eq!(phase, TurnPhase::NotRunning, "no process has started");
    assert_eq!(texts(&queue), ["q1", "q2"]);
    assert!(hold, "restored on hold by the restore itself");
}

/// Q4: with saving off nothing is written, and after a restart the queue is empty.
#[tokio::test(flavor = "multi_thread")]
async fn with_saving_off_the_queue_is_not_kept() {
    let case = Case::new();
    let state = case.daemon_not_saving();
    let _client = queued(&state, case.id(), &[("q1", vec![])]).await;
    // A wait, kept: a negative (no write ever comes) has no event to order after.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!case.queue_file().exists(), "nothing written");
    state.stop_session(case.id());
    drop(state);

    let restarted = case.daemon_not_saving();
    let mut client = conn::connect(&restarted).await;
    let (_, hold, queue) = input_until(&mut client, case.id(), "idle", |p, _, _| {
        *p == TurnPhase::Idle
    })
    .await;
    assert!(
        queue.is_empty() && !hold,
        "an empty queue after the restart"
    );
    restarted.stop_session(case.id());
}

/// R9: the `.queue` file goes when the queue empties, and with the session's delete and forget.
#[tokio::test(flavor = "multi_thread")]
async fn the_queue_file_goes_when_the_queue_empties_and_with_the_session() {
    let case = Case::new();
    let state = case.daemon();
    let _sender = queued(&state, case.id(), &[("q1", vec![])]).await;
    case.saved_is(Some(&["q1"])).await;
    // A new connection is replayed the queue at its handshake.
    let mut client = conn::connect(&state).await;
    let (_, _, queue) =
        input_until(&mut client, case.id(), "q1 queued", |_, _, q| q.len() == 1).await;
    ack(
        &mut client,
        ClientMsg::QueueRemove {
            req: 7,
            session: case.id(),
            entry: queue[0].id,
        },
        7,
    )
    .await;
    case.saved_is(None).await;
    ack(&mut client, message(8, case.id(), "q2", vec![]), 8).await;
    case.saved_is(Some(&["q2"])).await;
    let (_, ptys) = state.delete_session(case.id()).expect("deleted");
    for pty in ptys {
        let _ = pty.kill();
    }
    assert!(!case.queue_file().exists(), "deleted with the session");
    // A wait, kept: a negative (no write ever comes) has no event to order after.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!case.queue_file().exists(), "and not written again");

    let other = Case::new();
    let state = other.daemon();
    let _client = queued(&state, other.id(), &[("q1", vec![])]).await;
    other.saved_is(Some(&["q1"])).await;
    for pty in state
        .forget_project(other.project.path())
        .expect("forgotten")
    {
        let _ = pty.kill();
    }
    assert!(!other.queue_file().exists(), "forgotten with the project");
}

/// FR-021c's rule for the queue: a damaged `.queue` file restores an empty queue, and the session's
/// events still load.
#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_queue_file_restores_an_empty_queue_and_the_events_load() {
    let case = Case::new();
    let state = case.daemon();
    let _client = queued(&state, case.id(), &[("q1", vec![])]).await;
    case.saved_is(Some(&["q1"])).await;
    micold_daemon::history::save_all_live(&state, micold_daemon::history::STOP_SAVE_BOUND).await;
    state.stop_session(case.id());
    drop(state);
    std::fs::write(case.queue_file(), b"not a queue").unwrap();

    let restarted = case.daemon();
    let mut client = conn::connect(&restarted).await;
    let (_, hold, queue) = input_until(&mut client, case.id(), "idle", |p, _, _| {
        *p == TurnPhase::Idle
    })
    .await;
    assert!(
        queue.is_empty() && !hold,
        "the damaged queue is not restored"
    );
    let stream = restarted.event_stream(case.id()).expect("a stream");
    let (_, events, _) = stream.saved_snapshot();
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.event, StructuredEvent::UserSent { text, .. } if text == "m1")),
        "the saved events are restored"
    );
    restarted.stop_session(case.id());
}

/// R9: quick edits are saved in the order they were made: the file ends with the last one.
#[tokio::test(flavor = "multi_thread")]
async fn quick_edits_are_saved_in_order() {
    let case = Case::new();
    let state = case.daemon();
    let _sender = queued(&state, case.id(), &[("q1", vec![])]).await;
    let mut client = conn::connect(&state).await;
    let (_, _, queue) =
        input_until(&mut client, case.id(), "q1 queued", |_, _, q| q.len() == 1).await;
    let entry: QueueEntryId = queue[0].id;
    for n in 0..20u64 {
        client
            .send(Frame::Control(ClientMsg::QueueEdit {
                req: 100 + n,
                session: case.id(),
                entry,
                text: format!("edit {n}"),
                keep: vec![],
            }))
            .await
            .expect("sent");
    }
    conn::answer(&mut client, 119).await;
    case.saved_is(Some(&["edit 19"])).await;
    // A wait, kept: a negative (no write ever comes) has no event to order after.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(case.saved_texts(), Some(vec!["edit 19".to_string()]));
    state.stop_session(case.id());
}

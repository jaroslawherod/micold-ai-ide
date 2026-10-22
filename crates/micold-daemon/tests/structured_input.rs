//! Feature 941, milestone M2 (T024, T025): a structured session's messages are delivered once to
//! an idle CLI and queued while it is busy; every window sees the turn's phase, the queue and its
//! hold; the sidebar's activity follows the turn.
//!
//! Every case runs the stand-in of `support/structured.rs`, first on `PATH` for the whole test
//! process, scripted per project through `.fake-stream`.
#![cfg(unix)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ActivitySignal, ClientMsg, DaemonMsg, ErrorKind, InterruptAsk, OperationResult, QueueEntryId,
    QueuedSummary, TurnPhase,
};
use micold_core::session::{SessionId, SessionLocation};
use micold_core::structured::attachment::{admit, ImageAttachment};
use micold_core::structured::{
    encode_user_message, SequencedEvent, StructuredEvent, UserMessageRefusal,
};
use micold_core::terminal::LaunchMode;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use conn::Client;
use history::service;
use structured::{clear_records, fake_stream_cli, fixture, stream_script, structured_session};

/// One turn per message, each `result` written once the test creates `turn-<n>`.
fn gated() -> String {
    format!(
        "on-message-gated {}\n",
        fixture("text-turn.jsonl").display()
    )
}

/// One turn per message, answered at once.
fn answering() -> String {
    format!("on-message {}\n", fixture("text-turn.jsonl").display())
}

/// Let the stand-in in `dir` write the `n`-th turn's result.
fn end_turn(dir: &Path, n: usize) {
    std::fs::write(dir.join(format!("turn-{n}")), "").expect("write the turn gate");
}

/// A service with one structured session at `project`'s root, started with `script`, its input
/// driver running.
fn started(project: &Path, script: &str) -> (Arc<DaemonState>, SessionId) {
    let (state, id) = started_without_driver(project, script);
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    (state, id)
}

fn started_without_driver(project: &Path, script: &str) -> (Arc<DaemonState>, SessionId) {
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project, vec![session]);
    stream_script(project, script);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    (state, id)
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

/// One `StructuredInput` as a window received it.
#[derive(Debug, Clone)]
struct Seen {
    phase: TurnPhase,
    hold: bool,
    queue: Vec<QueuedSummary>,
    at: Instant,
}

impl Seen {
    fn texts(&self) -> Vec<&str> {
        self.queue.iter().map(|q| q.text.as_str()).collect()
    }
    fn busy(&self) -> bool {
        matches!(self.phase, TurnPhase::Busy { .. })
    }
}

/// A window: a connection, and everything it received of the kinds these tests read.
struct Window {
    client: Client,
    answers: HashMap<u64, DaemonMsg>,
    inputs: HashMap<SessionId, Vec<Seen>>,
    events: Vec<(SequencedEvent, Instant)>,
}

impl Window {
    async fn connect(state: &Arc<DaemonState>) -> Self {
        Self::of(conn::connect(state).await)
    }

    /// A window viewing `id`, so it is also sent its events.
    async fn viewing(state: &Arc<DaemonState>, project: &Path, id: SessionId) -> Self {
        Self::of(structured::viewer(state, project, id).await)
    }

    fn of(client: Client) -> Self {
        Self {
            client,
            answers: HashMap::new(),
            inputs: HashMap::new(),
            events: Vec::new(),
        }
    }

    async fn send(&mut self, msg: ClientMsg) {
        self.client.send(Frame::Control(msg)).await.expect("sent");
    }

    /// Receive until `done` holds, within 15 s.
    async fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !done(self) {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            assert!(
                !left.is_zero(),
                "{what}: never happened; inputs {:#?}",
                self.inputs
            );
            if let Ok(frame) = tokio::time::timeout(left, self.client.next()).await {
                self.take(frame.expect("the connection is open").expect("a frame"));
            }
        }
    }

    /// Receive whatever arrives within `quiet`.
    async fn drain(&mut self, quiet: Duration) {
        while let Ok(Some(Ok(frame))) = tokio::time::timeout(quiet, self.client.next()).await {
            self.take(frame);
        }
    }

    fn take(&mut self, frame: Frame<DaemonMsg>) {
        match frame {
            Frame::Control(
                m @ (DaemonMsg::OperationOk { req, .. } | DaemonMsg::OperationError { req, .. }),
            ) => {
                self.answers.insert(req, m);
            }
            Frame::Control(DaemonMsg::StructuredInput {
                session,
                phase,
                hold,
                queue,
            }) => self.inputs.entry(session).or_default().push(Seen {
                phase,
                hold,
                queue,
                at: Instant::now(),
            }),
            Frame::Control(DaemonMsg::StructuredEvents { events, .. }) => {
                let at = Instant::now();
                self.events.extend(events.into_iter().map(|e| (e, at)));
            }
            _ => {}
        }
    }

    async fn answer(&mut self, req: u64) -> DaemonMsg {
        self.until(&format!("the answer to {req}"), |w| {
            w.answers.contains_key(&req)
        })
        .await;
        self.answers.remove(&req).expect("answered")
    }

    async fn request(&mut self, msg: ClientMsg, req: u64) -> DaemonMsg {
        self.send(msg).await;
        self.answer(req).await
    }

    fn last(&self, id: SessionId) -> Option<&Seen> {
        self.inputs.get(&id).and_then(|seen| seen.last())
    }

    /// Receive until `id`'s latest `StructuredInput` satisfies `done`.
    async fn sees(&mut self, id: SessionId, what: &str, done: impl Fn(&Seen) -> bool) -> Seen {
        self.until(what, |w| w.last(id).is_some_and(&done)).await;
        self.last(id).cloned().expect("seen")
    }

    fn user_sent(&self) -> Vec<(String, Vec<String>)> {
        self.events
            .iter()
            .filter_map(|(e, _)| match &e.event {
                StructuredEvent::UserSent { text, attachments } => {
                    Some((text.clone(), attachments.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn results(&self) -> Vec<Instant> {
        self.events
            .iter()
            .filter(|(e, _)| matches!(e.event, StructuredEvent::Result { .. }))
            .map(|(_, at)| *at)
            .collect()
    }
}

fn assert_ack(answer: &DaemonMsg, what: &str) {
    assert!(
        matches!(
            answer,
            DaemonMsg::OperationOk {
                result: OperationResult::Ack,
                ..
            }
        ),
        "{what}: not acknowledged: {answer:?}"
    );
}

fn assert_refused(answer: &DaemonMsg, sentence: &str, what: &str) {
    match answer {
        DaemonMsg::OperationError {
            kind: ErrorKind::Refused,
            message,
            ..
        } => assert_eq!(message, sentence, "{what}: the refusal sentence"),
        other => panic!("{what}: not refused: {other:?}"),
    }
}

/// The lines the stand-in in `dir` read on stdin so far.
fn stdin_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("stdin-record"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The text block of each user message the stand-in read (empty for a message of pictures only).
fn delivered(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .iter()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            let first = &value["message"]["content"][0];
            first["text"].as_str().unwrap_or_default().to_owned()
        })
        .collect()
}

/// Wait, within 10 s, until the stand-in in `dir` has read `expected`, and nothing else.
async fn delivered_is(dir: &Path, expected: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let got = delivered(dir);
        if got == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "delivered {got:?}, expected {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// And that it stays so for a moment: nothing more is written.
async fn delivered_stays(dir: &Path, expected: &[&str]) {
    delivered_is(dir, expected).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(delivered(dir), expected, "nothing more is delivered");
}

fn activity(state: &DaemonState, id: SessionId) -> ActivitySignal {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .map(|s| s.activity)
        .expect("the session is in the snapshot")
}

/// Wait, within 1 s (SC-002), until `id`'s activity is `expected`.
async fn activity_is(state: &DaemonState, id: SessionId, expected: ActivitySignal, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let now = activity(state, id);
        if now == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{what}: activity {now:?}, expected {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn idle(seen: &Seen) -> bool {
    seen.phase == TurnPhase::Idle
}

/// SC-001, FR-002, FR-003, FR-027: each send to an idle session is one stdin line equal to its
/// encoding — pictures as image blocks in attach order, no text block for pictures alone — answered
/// `Ack`, and reaches two viewing windows once as `UserSent` with the pictures' names.
#[tokio::test(flavor = "multi_thread")]
async fn a_send_to_an_idle_session_is_one_line_equal_to_its_encoding() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &answering());
    let mut one = Window::viewing(&state, project.path(), id).await;
    let mut two = Window::viewing(&state, project.path(), id).await;
    let cases: Vec<(&str, Vec<ImageAttachment>)> = vec![
        ("one line", vec![]),
        ("first line\nsecond line\n\nfourth", vec![]),
        ("zażółć gęślą jaźń — 日本語 ✓", vec![]),
        ("\tindented\twith tabs\t", vec![]),
        (
            "look at these",
            vec![png("first.png", 1), png("second.png", 2)],
        ),
        ("", vec![png("only-a.png", 3), png("only-b.png", 4)]),
    ];
    for (n, (text, images)) in cases.iter().enumerate() {
        let req = n as u64 + 1;
        one.sees(id, "idle before the send", idle).await;
        let got = one
            .request(message(req, id, text, images.clone()), req)
            .await;
        assert_ack(&got, text);
        let lines = {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let lines = stdin_lines(project.path());
                if lines.len() > n {
                    break lines;
                }
                assert!(Instant::now() < deadline, "{text:?} never reached stdin");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        };
        assert_eq!(lines.len(), n + 1, "one line per send");
        assert_eq!(
            format!("{}\n", lines[n]),
            encode_user_message(text, images),
            "the line is the message's encoding"
        );
        // Its turn's result, so the next send finds the session idle again.
        one.until("the turn's result", |w| w.results().len() > n)
            .await;
    }
    let expected: Vec<(String, Vec<String>)> = cases
        .iter()
        .map(|(text, images)| {
            (
                text.to_string(),
                images.iter().map(|i| i.name().to_owned()).collect(),
            )
        })
        .collect();
    two.until("every UserSent", |w| w.user_sent().len() >= expected.len())
        .await;
    assert_eq!(one.user_sent(), expected, "the first window: each once");
    assert_eq!(two.user_sent(), expected, "the second window: each once");
    state.stop_session(id);
}

/// SC-002, FR-009: both windows see `Busy` then `Idle` within 1 s of the result, and a window that
/// connects mid-turn is shown `Busy` right after its handshake.
#[tokio::test(flavor = "multi_thread")]
async fn every_window_sees_busy_then_idle_within_a_second_of_the_result() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut one = Window::viewing(&state, project.path(), id).await;
    let mut two = Window::viewing(&state, project.path(), id).await;
    one.sees(id, "idle at first", idle).await;
    assert_ack(
        &one.request(message(1, id, "hello", vec![]), 1).await,
        "the send",
    );
    let busy = one.sees(id, "busy", Seen::busy).await;
    two.sees(id, "busy in the other window", Seen::busy).await;
    assert!(matches!(
        busy.phase,
        TurnPhase::Busy {
            interrupt: InterruptAsk::None,
            ..
        }
    ));

    let mut late = Window::connect(&state).await;
    late.until("the handshake's replay", |w| w.last(id).is_some())
        .await;
    assert_eq!(
        late.inputs[&id][0].phase, busy.phase,
        "a window connecting mid-turn is shown the open turn first"
    );

    end_turn(project.path(), 1);
    for window in [&mut one, &mut two] {
        window
            .until("the result and idle after it", |w| {
                !w.results().is_empty() && w.last(id).is_some_and(idle)
            })
            .await;
        let result_at = window.results()[0];
        let idle_at = window.last(id).unwrap().at;
        assert!(
            idle_at.saturating_duration_since(result_at) < Duration::from_secs(1),
            "idle within 1 s of the result"
        );
    }
    state.stop_session(id);
}

/// FR-011: the session's activity is Working from the send and AwaitingInput from the result; the
/// CLI's own `UserPromptSubmit` and `Stop` hooks do not move it, and #942's waiting state still wins.
#[tokio::test(flavor = "multi_thread")]
async fn the_sessions_activity_follows_the_turn() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut window = Window::viewing(&state, project.path(), id).await;
    window.sees(id, "idle", idle).await;
    state.note_activity(id, ActivityEvent::Hook(HookKind::UserPromptSubmit));
    assert_ne!(
        activity(&state, id),
        ActivitySignal::Working,
        "the CLI's prompt hook does not start a turn"
    );

    assert_ack(
        &window.request(message(1, id, "work", vec![]), 1).await,
        "the send",
    );
    activity_is(&state, id, ActivitySignal::Working, "from the send").await;
    state.note_activity(id, ActivityEvent::Hook(HookKind::Stop));
    assert_eq!(
        activity(&state, id),
        ActivitySignal::Working,
        "the CLI's Stop hook mid-turn does not end it"
    );
    state.note_activity(id, ActivityEvent::Hook(HookKind::Notification));
    assert_eq!(
        activity(&state, id),
        ActivitySignal::AwaitingInput,
        "the waiting state still wins"
    );
    state.note_activity(id, ActivityEvent::Hook(HookKind::PreToolUse));

    end_turn(project.path(), 1);
    activity_is(&state, id, ActivitySignal::AwaitingInput, "from the result").await;
    assert_ack(
        &window.request(message(2, id, "again", vec![]), 2).await,
        "the second send",
    );
    activity_is(&state, id, ActivitySignal::Working, "from the second send").await;
    end_turn(project.path(), 2);
    activity_is(
        &state,
        id,
        ActivitySignal::AwaitingInput,
        "from the second result",
    )
    .await;
    state.stop_session(id);
}

/// FR-005: a write that fails is answered `IoFailed`, and nothing is queued.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_write_is_answered_io_failed_and_queues_nothing() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut window = Window::viewing(&state, project.path(), id).await;
    window.sees(id, "idle", idle).await;
    assert_ack(
        &window
            .request(message(1, id, "CLOSE-STDIN", vec![]), 1)
            .await,
        "the message that closes the stand-in's stdin",
    );
    end_turn(project.path(), 1);
    window
        .until("idle after the result", |w| {
            !w.results().is_empty() && w.last(id).is_some_and(idle)
        })
        .await;
    let got = window.request(message(2, id, "lost", vec![]), 2).await;
    assert!(
        matches!(
            got,
            DaemonMsg::OperationError {
                kind: ErrorKind::IoFailed,
                ..
            }
        ),
        "a failed write: {got:?}"
    );
    let seen = window.sees(id, "idle again", idle).await;
    assert!(seen.queue.is_empty() && !seen.hold, "nothing queued");
    assert_eq!(delivered(project.path()), ["CLOSE-STDIN"]);
    state.stop_session(id);
}

/// FR-031: a send, a queue and a busy turn of session A never reach session B.
#[tokio::test(flavor = "multi_thread")]
async fn one_sessions_input_never_reaches_another() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    let a_dir = mcp::add_worktree(project.path(), "a");
    let b_dir = mcp::add_worktree(project.path(), "b");
    let sessions = vec![
        structured_session(SessionLocation::Worktree("a".into())),
        structured_session(SessionLocation::Worktree("b".into())),
    ];
    let (a, b) = (sessions[0].id, sessions[1].id);
    let state = service(project.path(), sessions);
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    for (id, dir) in [(a, &a_dir), (b, &b_dir)] {
        stream_script(dir, &gated());
        state
            .start_session_gated(
                id,
                LaunchMode::Fresh,
                micold_core::occupancy::Additional::Yes,
            )
            .expect("starts");
    }
    let mut window = Window::connect(&state).await;
    window
        .until("both sessions idle", |w| {
            w.last(a).is_some_and(idle) && w.last(b).is_some_and(idle)
        })
        .await;
    assert_ack(
        &window.request(message(1, a, "to a", vec![]), 1).await,
        "a's send",
    );
    assert_ack(
        &window
            .request(message(2, a, "queued on a", vec![]), 2)
            .await,
        "a's queued send",
    );
    window
        .sees(a, "a busy with a queue", |s| s.busy() && s.queue.len() == 1)
        .await;
    window.drain(Duration::from_millis(300)).await;
    let b_seen = window.last(b).expect("b seen");
    assert!(
        idle(b_seen) && b_seen.queue.is_empty() && !b_seen.hold,
        "b is untouched: {b_seen:?}"
    );
    delivered_is(&a_dir, &["to a"]).await;
    assert!(stdin_lines(&b_dir).is_empty(), "nothing reached b's CLI");
    state.stop_session(a);
    state.stop_session(b);
}

/// FR-018, FR-019, FR-021: a busy send writes nothing, is acknowledged and joins every window's
/// queue; queued messages go out alone, one per result, a picture with its image block.
#[tokio::test(flavor = "multi_thread")]
async fn a_busy_send_is_queued_and_each_result_delivers_one() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut one = Window::connect(&state).await;
    let mut two = Window::connect(&state).await;
    one.sees(id, "idle", idle).await;
    assert_ack(&one.request(message(1, id, "m1", vec![]), 1).await, "m1");
    let picture = png("shot.png", 9);
    assert_ack(&one.request(message(2, id, "q1", vec![]), 2).await, "q1");
    assert_ack(
        &one.request(message(3, id, "q2", vec![picture.clone()]), 3)
            .await,
        "q2",
    );
    for window in [&mut one, &mut two] {
        let seen = window.sees(id, "two queued", |s| s.queue.len() == 2).await;
        assert_eq!(seen.texts(), ["q1", "q2"], "in send order");
        assert_eq!(seen.queue[1].attachments[0].name, "shot.png");
        assert!(!seen.hold);
    }
    delivered_stays(project.path(), &["m1"]).await;

    end_turn(project.path(), 1);
    delivered_is(project.path(), &["m1", "q1"]).await;
    let seen = two.sees(id, "q1 left", |s| s.queue.len() == 1).await;
    assert_eq!(seen.texts(), ["q2"]);
    assert!(seen.busy(), "q1's turn is open");
    delivered_stays(project.path(), &["m1", "q1"]).await;

    end_turn(project.path(), 2);
    delivered_is(project.path(), &["m1", "q1", "q2"]).await;
    assert_eq!(
        format!("{}\n", stdin_lines(project.path())[2]),
        encode_user_message("q2", &[picture]),
        "the queued picture goes out with its image block"
    );
    two.sees(id, "the queue empty", |s| s.queue.is_empty() && s.busy())
        .await;
    end_turn(project.path(), 3);
    two.sees(id, "idle", idle).await;
    state.stop_session(id);
}

/// R2: a send that opened the next turn before the driver handled the last one's end leaves the
/// queue for that turn's result; nothing is written during it.
#[tokio::test(flavor = "multi_thread")]
async fn a_turn_opened_before_the_driver_woke_keeps_the_queue_for_its_result() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started_without_driver(project.path(), &gated());
    let mut window = Window::viewing(&state, project.path(), id).await;
    window.sees(id, "idle", idle).await;
    assert_ack(&window.request(message(1, id, "m1", vec![]), 1).await, "m1");
    assert_ack(&window.request(message(2, id, "q1", vec![]), 2).await, "q1");
    end_turn(project.path(), 1);
    window
        .until("t1's result", |w| !w.results().is_empty())
        .await;
    // No driver: the turn is idle with q1 waiting. This send queues behind q1, and its own lock
    // section delivers q1 as the next turn.
    assert_ack(&window.request(message(3, id, "m2", vec![]), 3).await, "m2");
    delivered_is(project.path(), &["m1", "q1"]).await;
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    delivered_stays(project.path(), &["m1", "q1"]).await;
    let seen = window.sees(id, "m2 waits", |s| s.texts() == ["m2"]).await;
    assert!(seen.busy());
    end_turn(project.path(), 2);
    delivered_is(project.path(), &["m1", "q1", "m2"]).await;
    state.stop_session(id);
}

/// FR-020, FR-021: an edit keeps the entry's place, a removed entry is never delivered, a take
/// answers the requester alone; an entry delivered or removed is refused with the sentence; a
/// window's disconnect keeps its entries.
#[tokio::test(flavor = "multi_thread")]
async fn queue_edits_name_their_entry() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut one = Window::connect(&state).await;
    let mut two = Window::connect(&state).await;
    one.sees(id, "idle", idle).await;
    assert_ack(&one.request(message(1, id, "m1", vec![]), 1).await, "m1");
    for (req, text) in [(2, "q1"), (3, "q2"), (4, "q3")] {
        let images = if text == "q1" {
            vec![png("q1.png", 5)]
        } else {
            vec![png("keep.png", 6), png("drop.png", 7)]
        };
        assert_ack(
            &one.request(message(req, id, text, images), req).await,
            text,
        );
    }
    let seen = two.sees(id, "three queued", |s| s.queue.len() == 3).await;
    let ids: Vec<QueueEntryId> = seen.queue.iter().map(|q| q.id).collect();

    let edit = |req, entry, text: &str, keep: Vec<u32>| ClientMsg::QueueEdit {
        req,
        session: id,
        entry,
        text: text.into(),
        keep,
    };
    assert_ack(
        &one.request(edit(5, ids[1], "q2 edited", vec![0]), 5).await,
        "the edit",
    );
    let seen = two
        .sees(id, "the edit", |s| s.texts() == ["q1", "q2 edited", "q3"])
        .await;
    assert_eq!(
        seen.queue[1].id, ids[1],
        "the edit keeps the entry and place"
    );
    let names: Vec<&str> = seen.queue[1]
        .attachments
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(names, ["keep.png"], "only the kept picture stays");
    assert_refused(
        &one.request(edit(6, ids[1], "  ", vec![]), 6).await,
        &UserMessageRefusal::Blank.to_string(),
        "an edit that leaves nothing",
    );

    let remove = |req, entry| ClientMsg::QueueRemove {
        req,
        session: id,
        entry,
    };
    assert_ack(&one.request(remove(7, ids[2]), 7).await, "the remove");
    let take = ClientMsg::QueueTake {
        req: 8,
        session: id,
        entry: ids[0],
    };
    match one.request(take, 8).await {
        DaemonMsg::OperationOk {
            result: OperationResult::Taken { text, images },
            ..
        } => {
            assert_eq!(text, "q1");
            assert_eq!(images, vec![png("q1.png", 5)], "the picture's bytes");
        }
        other => panic!("the take: {other:?}"),
    }
    let seen = two
        .sees(id, "only the edit left", |s| s.texts() == ["q2 edited"])
        .await;
    assert!(seen.busy());
    two.drain(Duration::from_millis(100)).await;
    assert!(
        !two.answers.values().any(|a| matches!(
            a,
            DaemonMsg::OperationOk {
                result: OperationResult::Taken { .. },
                ..
            }
        )),
        "the take is answered to its requester only"
    );

    // The window that queued it leaves; its entry stays and goes out.
    drop(one);
    end_turn(project.path(), 1);
    delivered_is(project.path(), &["m1", "q2 edited"]).await;
    two.sees(id, "the queue empty", |s| s.queue.is_empty())
        .await;
    let gone = "That message was already sent or removed";
    assert_refused(
        &two.request(edit(9, ids[1], "late", vec![]), 9).await,
        gone,
        "an edit of a delivered entry",
    );
    assert_refused(
        &two.request(remove(10, ids[1]), 10).await,
        gone,
        "a remove of a delivered entry",
    );
    let take = ClientMsg::QueueTake {
        req: 11,
        session: id,
        entry: ids[2],
    };
    assert_refused(
        &two.request(take, 11).await,
        gone,
        "a take of a removed entry",
    );
    end_turn(project.path(), 2);
    delivered_stays(project.path(), &["m1", "q2 edited"]).await;
    state.stop_session(id);
}

/// FR-022, Q1, Q5: a process exit and a stop hold a non-empty queue; a start keeps it held and
/// nothing goes out until one `QueueRelease`; a send while idle and held goes out at once and the
/// queue stays held; a release while busy waits for the result; two releases at once deliver once.
#[tokio::test(flavor = "multi_thread")]
async fn exit_and_stop_hold_the_queue_until_one_release() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, "await-stdin\nawait-file exit-now\nexit 1\n");
    let mut one = Window::connect(&state).await;
    let mut two = Window::connect(&state).await;
    one.sees(id, "idle", idle).await;
    assert_ack(&one.request(message(1, id, "m1", vec![]), 1).await, "m1");
    one.sees(id, "busy", Seen::busy).await;
    assert_ack(&one.request(message(2, id, "q1", vec![]), 2).await, "q1");
    assert_ack(&one.request(message(3, id, "q2", vec![]), 3).await, "q2");
    std::fs::write(dir.join("exit-now"), "").unwrap();
    let seen = one
        .sees(id, "not running", |s| s.phase == TurnPhase::NotRunning)
        .await;
    assert!(seen.hold, "the exit holds the queue");
    assert_eq!(seen.texts(), ["q1", "q2"]);

    // Supervision restarts the crash; the test drives its sweep, as the daemon's timer does.
    let _ = std::fs::remove_file(dir.join("exit-now"));
    clear_records(dir);
    stream_script(dir, &gated());
    let deadline = Instant::now() + Duration::from_secs(10);
    while structured::starts(dir) < 2 {
        state.supervise_exited_sessions();
        assert!(Instant::now() < deadline, "the crash was never restarted");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let seen = one.sees(id, "running again", idle).await;
    assert!(seen.hold, "a start keeps the queue held");
    assert_eq!(seen.texts(), ["q1", "q2"]);
    delivered_stays(dir, &[]).await;

    assert_ack(&one.request(message(4, id, "now", vec![]), 4).await, "now");
    delivered_is(dir, &["now"]).await;
    let seen = one.sees(id, "busy with now", Seen::busy).await;
    assert!(seen.hold, "the held queue stays held");
    assert_eq!(seen.texts(), ["q1", "q2"], "and unchanged");

    let release = |req| ClientMsg::QueueRelease { req, session: id };
    assert_ack(&two.request(release(5), 5).await, "a release while busy");
    let seen = two.sees(id, "released", |s| !s.hold).await;
    assert!(seen.busy());
    delivered_stays(dir, &["now"]).await;
    end_turn(dir, 1);
    delivered_is(dir, &["now", "q1"]).await;

    state.stop_session(id);
    let seen = one
        .sees(id, "stopped", |s| s.phase == TurnPhase::NotRunning)
        .await;
    assert!(seen.hold, "the stop holds the queue");
    assert_eq!(seen.texts(), ["q2"]);
    clear_records(dir);
    for n in 1..=3 {
        let _ = std::fs::remove_file(dir.join(format!("turn-{n}")));
    }
    state.start_session(id, LaunchMode::Resume).expect("starts");
    let seen = one.sees(id, "started", idle).await;
    assert!(seen.hold);
    delivered_stays(dir, &[]).await;

    one.send(release(6)).await;
    two.send(release(7)).await;
    assert_ack(&one.answer(6).await, "the first release");
    assert_ack(&two.answer(7).await, "the second release");
    delivered_stays(dir, &["q2"]).await;
    let seen = two
        .sees(id, "q2 out", |s| s.queue.is_empty() && s.busy())
        .await;
    assert!(!seen.hold);
    state.stop_session(id);
}

/// Edge case *Refusal or delivery failure*: a queued message whose write fails goes back to the
/// head of the queue, held.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_delivery_returns_to_the_head_held() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut window = Window::connect(&state).await;
    window.sees(id, "idle", idle).await;
    assert_ack(
        &window
            .request(message(1, id, "CLOSE-STDIN", vec![]), 1)
            .await,
        "the first",
    );
    assert_ack(&window.request(message(2, id, "q1", vec![]), 2).await, "q1");
    assert_ack(&window.request(message(3, id, "q2", vec![]), 3).await, "q2");
    end_turn(project.path(), 1);
    let seen = window
        .sees(id, "held after the failure", |s| s.hold && idle(s))
        .await;
    assert_eq!(seen.texts(), ["q1", "q2"], "q1 back at the head");
    assert_eq!(delivered(project.path()), ["CLOSE-STDIN"]);
    state.stop_session(id);
}

/// FR-032: two windows sending to an idle session at once, 50 times: one delivered, one queued,
/// both windows shown the same queue; nothing refused, merged or lost.
#[tokio::test(flavor = "multi_thread")]
async fn two_windows_sending_at_once_deliver_one_and_queue_the_other() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, &gated());
    let mut one = Window::connect(&state).await;
    let mut two = Window::connect(&state).await;
    one.sees(id, "idle", idle).await;
    for i in 0..50u64 {
        let (a, b) = (format!("a{i}"), format!("b{i}"));
        let (ra, rb) = (2 * i + 1, 2 * i + 2);
        one.send(message(ra, id, &a, vec![])).await;
        two.send(message(rb, id, &b, vec![])).await;
        assert_ack(&one.answer(ra).await, &a);
        assert_ack(&two.answer(rb).await, &b);
        let mine = one
            .sees(id, "one queued", |s| s.busy() && s.queue.len() == 1)
            .await;
        let theirs = two
            .sees(id, "one queued", |s| s.busy() && s.queue.len() == 1)
            .await;
        assert_eq!(mine.queue, theirs.queue, "the same queue in both windows");
        let queued = mine.queue[0].text.clone();
        assert!(queued == a || queued == b);
        end_turn(dir, (2 * i + 1) as usize);
        one.sees(id, "the queued one out", |s| s.queue.is_empty() && s.busy())
            .await;
        end_turn(dir, (2 * i + 2) as usize);
        one.sees(id, "idle", |s| idle(s) && s.queue.is_empty())
            .await;
        two.sees(id, "idle", |s| idle(s) && s.queue.is_empty())
            .await;
    }
    let got = delivered(dir);
    assert_eq!(got.len(), 100, "every message delivered once");
    for (i, pair) in got.chunks(2).enumerate() {
        let mut pair = pair.to_vec();
        pair.sort();
        assert_eq!(pair, [format!("a{i}"), format!("b{i}")]);
    }
    state.stop_session(id);
}

/// The session's delete drops its input state: a window connecting afterwards is not shown it.
#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_session_drops_its_input_state() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &gated());
    let mut window = Window::connect(&state).await;
    window.sees(id, "idle", idle).await;
    assert_ack(&window.request(message(1, id, "m1", vec![]), 1).await, "m1");
    assert_ack(&window.request(message(2, id, "q1", vec![]), 2).await, "q1");
    window.sees(id, "queued", |s| s.queue.len() == 1).await;
    let mut before = Window::connect(&state).await;
    before.until("the replay", |w| w.last(id).is_some()).await;

    let (_, ptys) = state.delete_session(id).expect("deleted");
    for pty in ptys {
        let _ = pty.kill();
    }
    let mut after = Window::connect(&state).await;
    after.drain(Duration::from_millis(500)).await;
    assert!(
        !after.inputs.contains_key(&id),
        "no input state replayed for a deleted session"
    );
}

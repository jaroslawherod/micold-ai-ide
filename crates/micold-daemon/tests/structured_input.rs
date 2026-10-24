//! Feature 941, milestones M2 (T024, T025) and M3 (T035): a structured session's messages are
//! delivered once to an idle CLI and queued while it is busy; every window sees the turn's phase,
//! the queue and its hold; the sidebar's activity follows the turn; an interrupt ends the open turn
//! with the process running on, is reported overdue when ignored, and holds the queue; and M7
//! (T063): an interrupt withdraws the turn's pending #942 permission request.
//!
//! Every case runs the stand-in of `support/structured.rs`, first on `PATH` for the whole test
//! process, scripted per project through `.fake-stream`.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ActivitySignal, ClientMsg, DaemonMsg, ErrorKind, InterruptAsk, OperationResult, PromptChoice,
    PromptId, QueueEntryId, QueuedSummary, Resolution, TurnPhase, WithdrawReason,
};
use micold_core::session::{SessionId, SessionLocation};
use micold_core::structured::attachment::{admit, ImageAttachment};
use micold_core::structured::control::encode_interrupt;
use micold_core::structured::transcript::{EntryKind, Transcript};
use micold_core::structured::{
    encode_user_message, SequencedEvent, StructuredEvent, TurnId, UserMessageRefusal,
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
use structured::{
    clear_records, fake_stream_cli, fixture, prompt_answers, prompt_call_file, stdin_lines,
    stream_script, structured_session,
};

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
    /// Each `PermissionRequested` (#942), as the request's id.
    requested: Vec<PromptId>,
    /// Each `PermissionResolved` (#942), as the request's id and how it ended.
    resolved: Vec<(PromptId, Resolution)>,
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
            requested: Vec::new(),
            resolved: Vec::new(),
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
            Frame::Control(DaemonMsg::PermissionRequested { request }) => {
                self.requested.push(request.id);
            }
            Frame::Control(DaemonMsg::PermissionResolved { id, resolution, .. }) => {
                self.resolved.push((id, resolution));
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

/// The text block of each user message the stand-in read (empty for a message of pictures only).
fn delivered(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .iter()
        .filter(|line| line.contains("\"type\":\"user\""))
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

/// The interrupt control requests the stand-in in `dir` read, as lines.
fn interrupts(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .into_iter()
        .filter(|line| line.contains("\"control_request\"") && line.contains("\"interrupt\""))
        .collect()
}

/// The open turn and its interrupt, when `seen` is busy.
fn busy_with(seen: &Seen) -> Option<(TurnId, InterruptAsk)> {
    match seen.phase {
        TurnPhase::Busy { turn, interrupt } => Some((turn, interrupt)),
        _ => None,
    }
}

fn asked(seen: &Seen) -> bool {
    busy_with(seen).is_some_and(|(_, ask)| ask == InterruptAsk::Asked)
}

fn overdue(seen: &Seen) -> bool {
    busy_with(seen).is_some_and(|(_, ask)| ask == InterruptAsk::Overdue)
}

fn interrupt(session: SessionId, turn: TurnId) -> ClientMsg {
    ClientMsg::TurnInterrupt { session, turn }
}

/// The stand-in answers each interrupt with P-4's lines: the turn ends interrupted.
fn honouring() -> String {
    format!(
        "on-interrupt {}\n",
        fixture("interrupt-turn.jsonl").display()
    )
}

/// The stand-in records each interrupt and does nothing; `turn-<n>` ends the n-th turn.
fn ignoring() -> String {
    format!(
        "ignore-interrupt {}\n",
        fixture("text-turn.jsonl").display()
    )
}

/// The stand-in's pid, as it recorded it at its start in `dir`.
fn stand_in_pid(dir: &Path) -> String {
    structured::record(dir, "pid-record")
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

// --- M3: interrupt (US3) ------------------------------------------------------------------------

/// SC-003, FR-013, US3 AS1–AS2: an interrupt of the open turn writes one `encode_interrupt` line,
/// pushes `InterruptAsked`, shows `Busy { Asked }`; the stand-in honouring it ends the turn and the
/// session is idle within 3 s, 20 times over, with the same process; the transcript closes each
/// turn `Interrupted`.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_ends_the_turn_and_the_same_process_runs_on() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, &honouring());
    let mut window = Window::viewing(&state, dir, id).await;
    window.sees(id, "idle", idle).await;
    let pid = stand_in_pid(dir);
    let mut slow = 0;
    for run in 1..=20u64 {
        assert_ack(
            &window
                .request(message(run, id, &format!("m{run}"), vec![]), run)
                .await,
            "the message",
        );
        let seen = window.sees(id, "busy", Seen::busy).await;
        let (turn, ask) = busy_with(&seen).expect("busy");
        assert_eq!(ask, InterruptAsk::None, "no interrupt asked yet");
        let asked_at = Instant::now();
        window.send(interrupt(id, turn)).await;
        window.sees(id, "the interrupt asked", asked).await;
        window.sees(id, "idle after the interrupt", idle).await;
        if asked_at.elapsed() > Duration::from_secs(3) {
            slow += 1;
        }
        let lines = interrupts(dir);
        assert_eq!(lines.len() as u64, run, "one interrupt line per turn");
        let request: serde_json::Value =
            serde_json::from_str(lines.last().unwrap()).expect("a JSON line");
        let request_id = request["request_id"].as_str().expect("a request id");
        assert_eq!(
            format!("{}\n", lines.last().unwrap()),
            encode_interrupt(request_id),
            "the line is encode_interrupt's"
        );
    }
    assert!(
        slow <= 1,
        "{slow} of 20 interrupts took over 3 s (SC-003: 95%)"
    );
    assert_eq!(stand_in_pid(dir), pid, "the same process before and after");
    assert_eq!(structured::starts(dir), 1, "never restarted");
    assert!(state.event_stream(id).is_some());

    window
        .until("20 results", |w| w.results().len() == 20)
        .await;
    let events: Vec<SequencedEvent> = window.events.iter().map(|(e, _)| e.clone()).collect();
    let notes = events
        .iter()
        .filter(|e| e.event == StructuredEvent::InterruptAsked)
        .count();
    assert_eq!(notes, 20, "one InterruptAsked note per interrupt");
    let mut transcript = Transcript::new();
    transcript.apply(0, &events, false);
    let interrupted = transcript
        .entries()
        .filter(|e| matches!(e.kind(), EntryKind::Interrupted))
        .count();
    assert_eq!(interrupted, 20, "each turn closes Interrupted");
    assert!(
        matches!(
            transcript.entries().last().map(|e| e.kind()),
            Some(EntryKind::Interrupted)
        ),
        "the transcript ends Interrupted"
    );
    state.stop_session(id);
}

/// FR-015, FR-016, US3 AS4–AS5: two windows interrupting the same turn write one line; an ignored
/// interrupt becomes `Overdue` after the bound with the session still running; a second interrupt
/// writes again and is `Asked`; the turn's own end clears it and the session is idle.
#[tokio::test(flavor = "multi_thread")]
async fn an_ignored_interrupt_goes_overdue_and_may_be_asked_again() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(dir, vec![session]);
    // The test's clock: 10 s shortened, never slept.
    state.set_interrupt_overdue(Duration::from_millis(400));
    stream_script(dir, &ignoring());
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    let mut one = Window::connect(&state).await;
    let mut two = Window::connect(&state).await;
    one.sees(id, "idle", idle).await;
    assert_ack(&one.request(message(1, id, "m1", vec![]), 1).await, "m1");
    let (turn, _) = busy_with(&one.sees(id, "busy", Seen::busy).await).unwrap();

    one.send(interrupt(id, turn)).await;
    two.send(interrupt(id, turn)).await;
    one.sees(id, "asked", asked).await;
    two.sees(id, "asked", asked).await;
    let seen = one.sees(id, "overdue", overdue).await;
    assert_eq!(
        busy_with(&seen).map(|(t, _)| t),
        Some(turn),
        "the same turn"
    );
    two.sees(id, "overdue", overdue).await;
    assert_eq!(interrupts(dir).len(), 1, "two windows, one interrupt line");
    assert!(
        one.last(id).unwrap().busy(),
        "an overdue interrupt stops nothing: the session runs, its turn open"
    );

    two.send(interrupt(id, turn)).await;
    two.sees(id, "asked again", asked).await;
    two.sees(id, "overdue again", overdue).await;
    assert_eq!(interrupts(dir).len(), 2, "an overdue ask is written again");

    end_turn(dir, 1);
    let seen = one.sees(id, "idle at the turn's end", idle).await;
    assert!(seen.queue.is_empty());
    two.sees(id, "idle", idle).await;
    state.stop_session(id);
}

/// FR-016, FR-017, US3 AS3: an interrupt of an idle, a not-running or an unknown session, or
/// naming a turn that is not the open one, writes nothing and answers nothing; `SessionInterrupt`
/// on a structured primary is still refused.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_that_names_no_open_turn_writes_nothing() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, &ignoring());
    let mut window = Window::connect(&state).await;
    window.sees(id, "idle", idle).await;
    window.send(interrupt(id, TurnId(1))).await;
    window.send(interrupt(SessionId::new(), TurnId(1))).await;
    assert_ack(&window.request(message(1, id, "m1", vec![]), 1).await, "m1");
    let (turn, _) = busy_with(&window.sees(id, "busy", Seen::busy).await).unwrap();
    window.send(interrupt(id, TurnId(turn.0 + 1))).await;
    window.send(interrupt(id, TurnId(turn.0 - 1))).await;
    window.drain(Duration::from_millis(300)).await;
    let seen = window.last(id).cloned().expect("seen");
    assert_eq!(
        busy_with(&seen),
        Some((turn, InterruptAsk::None)),
        "nothing asked"
    );
    end_turn(dir, 1);
    window.sees(id, "idle", idle).await;
    window.send(interrupt(id, turn)).await;
    window.drain(Duration::from_millis(300)).await;
    assert!(
        interrupts(dir).is_empty(),
        "nothing written: {:?}",
        interrupts(dir)
    );
    assert!(
        window.answers.is_empty(),
        "nothing answered: {:?}",
        window.answers
    );
    assert!(!window.last(id).unwrap().hold, "no queue, nothing held");

    assert!(
        state.refuses_terminal_request(id, None),
        "SessionInterrupt on the structured primary is refused (FR-017)"
    );
    window
        .send(ClientMsg::SessionInterrupt { session: id })
        .await;
    state.stop_session(id);
    window
        .sees(id, "not running", |s| s.phase == TurnPhase::NotRunning)
        .await;
    window.send(interrupt(id, turn)).await;
    window.drain(Duration::from_millis(300)).await;
    assert!(interrupts(dir).is_empty(), "nothing written");
    assert_eq!(delivered(dir), ["m1"]);
}

/// R4, contracts/claude-input.md *Interrupt*: an interrupt asked after a message opened its turn,
/// but before that message's write ran, waits for it: the request is written after the message it
/// interrupts, never to an idle CLI ahead of it.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_asked_before_its_message_is_written_follows_it() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, &honouring());
    let mut window = Window::connect(&state).await;
    window.sees(id, "idle", idle).await;
    let (client, _rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    let delivery = state
        .submit_user_message(
            client,
            id,
            micold_core::structured::input::Message {
                text: "m1".into(),
                images: vec![],
            },
        )
        .expect("accepted")
        .expect("opened as the next turn");
    let (turn, _) = busy_with(&window.sees(id, "busy", Seen::busy).await).unwrap();
    let pending = state
        .ask_interrupt(id, turn)
        .expect("an interrupt to write");
    let writer = tokio::spawn(micold_daemon::input_driver::interrupt(
        Arc::clone(&state),
        pending,
    ));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        stdin_lines(dir).is_empty(),
        "nothing written before the message: {:?}",
        stdin_lines(dir)
    );
    let deliverer = Arc::clone(&state);
    tokio::task::spawn_blocking(move || deliverer.deliver(delivery))
        .await
        .unwrap()
        .expect("written");
    window.sees(id, "idle after the interrupt", idle).await;
    let lines = stdin_lines(dir);
    assert_eq!(lines.len(), 2, "the message, then the request: {lines:?}");
    assert!(lines[0].contains("\"type\":\"user\""), "first the message");
    assert_eq!(interrupts(dir), [lines[1].clone()], "then the interrupt");
    writer.abort();
    state.stop_session(id);
}

/// Q2, US4 AS5: an interrupt with a queue holds it unchanged; the interrupted turn's end delivers
/// nothing; one release delivers the head.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_holds_the_queue_and_its_end_delivers_nothing() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started(dir, &honouring());
    let mut window = Window::connect(&state).await;
    window.sees(id, "idle", idle).await;
    assert_ack(&window.request(message(1, id, "m1", vec![]), 1).await, "m1");
    assert_ack(&window.request(message(2, id, "q1", vec![]), 2).await, "q1");
    assert_ack(&window.request(message(3, id, "q2", vec![]), 3).await, "q2");
    let seen = window.sees(id, "two queued", |s| s.queue.len() == 2).await;
    let (turn, _) = busy_with(&seen).expect("busy");
    window.send(interrupt(id, turn)).await;
    let seen = window.sees(id, "idle, held", |s| idle(s) && s.hold).await;
    assert_eq!(seen.texts(), ["q1", "q2"], "kept unchanged");
    delivered_stays(dir, &["m1"]).await;
    assert_ack(
        &window
            .request(
                ClientMsg::QueueRelease {
                    req: 4,
                    session: id,
                },
                4,
            )
            .await,
        "the release",
    );
    delivered_is(dir, &["m1", "q1"]).await;
    let seen = window.sees(id, "q1 out", |s| s.texts() == ["q2"]).await;
    assert!(!seen.hold && seen.busy());
    state.stop_session(id);
}

/// R4, edge case *Interrupt racing the turn's end*: an interrupt naming the turn that just ended,
/// sent between its `result` and the driver's wake-up, holds the queue, and never interrupts the
/// turn the queue would have started.
#[tokio::test(flavor = "multi_thread")]
async fn a_late_interrupt_naming_the_last_turn_holds_the_queue() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = started_without_driver(dir, &gated());
    let mut window = Window::viewing(&state, dir, id).await;
    window.sees(id, "idle", idle).await;
    assert_ack(&window.request(message(1, id, "m1", vec![]), 1).await, "m1");
    assert_ack(&window.request(message(2, id, "q1", vec![]), 2).await, "q1");
    let seen = window.sees(id, "q1 queued", |s| s.queue.len() == 1).await;
    let (turn, _) = busy_with(&seen).expect("busy");
    end_turn(dir, 1);
    window
        .until("t1's result", |w| !w.results().is_empty())
        .await;
    window.send(interrupt(id, turn)).await;
    window.sees(id, "held", |s| s.hold).await;
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    delivered_stays(dir, &["m1"]).await;
    assert!(interrupts(dir).is_empty(), "no turn was interrupted");
    let seen = window.last(id).cloned().unwrap();
    assert!(idle(&seen) && seen.hold);
    assert_eq!(seen.texts(), ["q1"]);
    state.stop_session(id);
}

// --- M7: an interrupt withdraws a pending permission request (US3 AS6, I-3) --------------------

/// T063, US3 AS6 (I-3): a turn whose CLI waits on a #942 permission request is interrupted; the
/// turn ends `Interrupted` with the same process running on and the session idle, and #942's rules
/// withdraw the request from both windows (`TurnEnded`): the waiting call is answered deny and a
/// decision after it is refused.
#[tokio::test(flavor = "multi_thread")]
async fn an_interrupt_withdraws_the_turns_pending_permission_request() {
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let call = prompt_call_file(
        dir,
        "p1",
        "Bash",
        serde_json::json!({"command": "make"}),
        "toolu_p1",
    );
    let script = format!(
        "await-stdin\nprompt-async {}\n{}",
        call.display(),
        honouring()
    );
    // As `started`, with #942's tool server bound first, so the CLI's prompt-tool call is served.
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(dir, vec![session]);
    let bindings = tempfile::tempdir().unwrap();
    mcp::serve_tool_server(&state, bindings.path().to_path_buf()).await;
    stream_script(dir, &script);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    let mut one = Window::viewing(&state, dir, id).await;
    let mut two = Window::viewing(&state, dir, id).await;
    one.sees(id, "idle", idle).await;
    let pid = stand_in_pid(dir);
    assert_ack(
        &one.request(message(1, id, "go", vec![]), 1).await,
        "the message",
    );
    let seen = one.sees(id, "busy", Seen::busy).await;
    let (turn, _) = busy_with(&seen).expect("busy");
    one.until("the request shown", |w| !w.requested.is_empty())
        .await;
    two.until("the request shown", |w| !w.requested.is_empty())
        .await;
    let request = one.requested[0];
    assert_eq!(
        two.requested,
        [request],
        "one request, shown in both windows"
    );
    assert!(one.resolved.is_empty(), "pending until the interrupt");

    one.send(interrupt(id, turn)).await;
    let withdrawn = (
        request,
        Resolution::Withdrawn {
            reason: WithdrawReason::TurnEnded,
        },
    );
    for window in [&mut one, &mut two] {
        window
            .until("the request withdrawn", |w| !w.resolved.is_empty())
            .await;
        assert_eq!(window.resolved, [withdrawn], "withdrawn once, TurnEnded");
        window.sees(id, "idle after the interrupt", idle).await;
    }
    assert_eq!(interrupts(dir).len(), 1, "one interrupt line");
    assert_eq!(
        prompt_answers(dir, 1),
        [serde_json::json!({"behavior": "deny", "message": "The turn ended before you answered."})],
        "the waiting call is answered deny"
    );
    let late = one
        .request(
            ClientMsg::PermissionDecision {
                req: 2,
                session: id,
                id: request,
                choice: PromptChoice::Allow,
            },
            2,
        )
        .await;
    assert!(
        matches!(late, DaemonMsg::OperationError { .. }),
        "a decision after the withdrawal is refused: {late:?}"
    );

    let events: Vec<SequencedEvent> = one.events.iter().map(|(e, _)| e.clone()).collect();
    let mut transcript = Transcript::new();
    transcript.apply(0, &events, false);
    assert!(
        matches!(
            transcript.entries().last().map(|e| e.kind()),
            Some(EntryKind::Interrupted)
        ),
        "the turn closes Interrupted"
    );
    assert_eq!(stand_in_pid(dir), pid, "the same process runs on");
    assert_eq!(structured::starts(dir), 1, "never restarted");
    state.stop_session(id);
}

//! Feature 938, milestone M2 (T014): a structured Claude Code session spawned over pipes streams the
//! stand-in's stdout to viewing clients as typed events, with a terminal session's launch.
//!
//! Every case runs the stand-in of `support/structured.rs` (R12), first on `PATH` for the whole test
//! process, scripted per project through `.fake-stream`.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures_util::SinkExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult, WireLifecycle};
use micold_core::session::{AiCli, KindRefusal, SessionId, SessionKind, SessionLocation};
use micold_core::structured::{StructuredEvent, UserMessageRefusal};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;
use tracing_subscriber::fmt::MakeWriter;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use conn::{answer, connect, Client};
use history::{ai_session, service};
use structured::{
    events_of, events_until, fake_stream_cli, fixture, fixture_events, record, seqs_increase,
    stream_script, structured_session, viewer, RECORDED,
};

/// The seven structured arguments, in order (FR-005).
const STRUCTURED_ARGS: [&str; 7] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
];

/// Arguments a structured session must never carry (FR-006a).
const PERMISSION_OVERRIDES: [&str; 5] = [
    "--permission-mode",
    "--allowedTools",
    "--disallowedTools",
    "--dangerously-skip-permissions",
    "--permission-prompt-tool",
];

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

/// The daemon's log for this test process: a global subscriber, because the stderr reader logs from
/// its own thread.
fn daemon_log() -> &'static LogBuffer {
    static LOG: OnceLock<LogBuffer> = OnceLock::new();
    LOG.get_or_init(|| {
        let buffer = LogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("one global subscriber");
        buffer
    })
}

fn log_text() -> String {
    String::from_utf8_lossy(&daemon_log().0.lock().unwrap()).into_owned()
}

/// A service with one structured session at `project`'s root, started with `script`.
fn started(project: &Path, script: &str) -> (Arc<DaemonState>, SessionId) {
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project, vec![session]);
    stream_script(project, script);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    (state, id)
}

/// `text` with every form of `id` replaced by `<id>`, so two sessions' records compare.
fn without_id(text: &str, id: SessionId) -> String {
    text.replace(&id.0.to_string(), "<id>")
        .replace(&id.0.simple().to_string(), "<id>")
}

fn replay(name: &str) -> String {
    format!("replay {}\n", fixture(name).display())
}

fn complete(expected: usize) -> impl Fn(&[structured::Received]) -> bool {
    move |got| got.len() >= expected
}

/// (a) FR-005, FR-006, FR-006a: the argv, cwd and environment of a terminal session at the same
/// location, behind the seven structured arguments, with no permission override.
#[test]
fn a_structured_launch_is_the_terminal_launch_behind_the_structured_args() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let terminal = ai_session();
    let structured = structured_session(SessionLocation::Default);
    let (t, s) = (terminal.id, structured.id);
    let state = service(dir, vec![terminal, structured]);
    stream_script(dir, "");

    state
        .start_session(t, LaunchMode::Fresh)
        .expect("terminal starts");
    let records: Vec<String> = ["argv-record", "cwd-record", "env-record"]
        .iter()
        .map(|name| without_id(&record(dir, name), t))
        .collect();
    assert!(state.stop_session(t));
    structured::clear_records(dir);

    state
        .start_session_gated(
            s,
            LaunchMode::Fresh,
            micold_core::occupancy::Additional::Yes,
        )
        .expect("structured starts");
    let argv = record(dir, "argv-record");
    let args: Vec<&str> = argv.lines().collect();
    assert_eq!(
        args[..7],
        STRUCTURED_ARGS,
        "the structured args first: {args:#?}"
    );
    let id_at = args
        .iter()
        .position(|a| *a == "--session-id")
        .expect("--session-id");
    assert_eq!(args[id_at + 1], s.0.to_string(), "its own identity handle");
    assert_eq!(
        args.contains(&"--settings"),
        records[0].lines().any(|a| a == "--settings"),
        "the hook file, the one source of activity and attention, exactly when a terminal has it"
    );
    for flag in PERMISSION_OVERRIDES {
        assert!(!args.contains(&flag), "no {flag}: {args:#?}");
    }
    let rest = args[7..].join("\n") + "\n";
    assert_eq!(
        without_id(&rest, s),
        records[0],
        "the rest is the terminal session's argv, --mcp-config last as there"
    );
    assert_eq!(without_id(&record(dir, "cwd-record"), s), records[1]);
    assert_eq!(without_id(&record(dir, "env-record"), s), records[2]);
    state.stop_session(s);
}

/// Feature 944 T018 (research R5, D8): a structured Claude Code session runs with its subagents'
/// text forwarded, behind #938's seven structured arguments unchanged; a terminal session at the
/// same location does not.
#[test]
fn a_structured_launch_forwards_subagent_text() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let terminal = ai_session();
    let structured = structured_session(SessionLocation::Default);
    let (t, s) = (terminal.id, structured.id);
    let state = service(dir, vec![terminal, structured]);
    stream_script(dir, "");

    state
        .start_session(t, LaunchMode::Fresh)
        .expect("terminal starts");
    assert_eq!(
        record(dir, "forward-record"),
        "CLAUDE_CODE_FORWARD_SUBAGENT_TEXT\n",
        "a terminal session is launched as before"
    );
    assert!(state.stop_session(t));
    structured::clear_records(dir);

    state
        .start_session_gated(
            s,
            LaunchMode::Fresh,
            micold_core::occupancy::Additional::Yes,
        )
        .expect("structured starts");
    assert_eq!(
        record(dir, "forward-record"),
        "CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1\n"
    );
    let argv = record(dir, "argv-record");
    let args: Vec<&str> = argv.lines().collect();
    assert_eq!(args[..7], STRUCTURED_ARGS, "{args:#?}");
    state.stop_session(s);
}

/// (b) SC-001, FR-016: every recorded fixture's lines reach a viewer as one typed event each, in
/// order, with strictly increasing seq; a second viewer receives the same.
#[tokio::test(flavor = "multi_thread")]
async fn every_recorded_fixture_reaches_two_viewers_in_order() {
    for name in RECORDED {
        let project = tempfile::tempdir().unwrap();
        let (state, id) = started(project.path(), &replay(name));
        let expected = fixture_events(name);
        let mut first = viewer(&state, project.path(), id).await;
        let mut second = viewer(&state, project.path(), id).await;
        for client in [&mut first, &mut second] {
            let got = events_until(client, id, complete(expected.len())).await;
            assert_eq!(events_of(&got), expected, "{name}");
            assert!(seqs_increase(&got), "{name}: seqs strictly increase");
        }
        state.stop_session(id);
    }
}

/// (c) SC-006: an event written while the turn runs reaches the client before the result is
/// written.
#[tokio::test(flavor = "multi_thread")]
async fn an_assistant_event_arrives_before_the_turns_result_is_written() {
    let project = tempfile::tempdir().unwrap();
    let lines = structured::fixture_lines("text-turn.jsonl");
    let result_at = lines
        .iter()
        .position(|l| l.contains("\"type\":\"result\""))
        .expect("the fixture ends its turn with a result");
    let mut script: Vec<String> = lines[..result_at].to_vec();
    script.push("pause 1500".into());
    script.extend(lines[result_at..].iter().cloned());
    let (state, id) = started(project.path(), &(script.join("\n") + "\n"));
    let mut client = viewer(&state, project.path(), id).await;
    let got = events_until(&mut client, id, |got| {
        got.iter()
            .any(|r| matches!(r.event.event, StructuredEvent::Result { .. }))
    })
    .await;
    let assistant = got
        .iter()
        .find(|r| matches!(r.event.event, StructuredEvent::Assistant { .. }))
        .expect("an assistant event");
    let result = got
        .iter()
        .find(|r| matches!(r.event.event, StructuredEvent::Result { .. }))
        .unwrap();
    assert!(
        result.at.duration_since(assistant.at) >= Duration::from_millis(1000),
        "the assistant event came as it was written, not with the result"
    );
    state.stop_session(id);
}

/// (d) FR-014, SC-002: every line of `tolerance.jsonl` is delivered, the trailing partial line at
/// exit, and the session runs until the stand-in exits.
#[tokio::test(flavor = "multi_thread")]
async fn no_line_of_the_tolerance_fixture_ends_the_session_or_is_lost() {
    let project = tempfile::tempdir().unwrap();
    let script = replay("tolerance.jsonl") + "pause 1500\nexit 0\n";
    let (state, id) = started(project.path(), &script);
    let expected = fixture_events("tolerance.jsonl");
    let mut client = viewer(&state, project.path(), id).await;
    let before_exit = events_until(&mut client, id, complete(expected.len() - 1)).await;
    assert_eq!(
        events_of(&before_exit)[..],
        expected[..expected.len() - 1],
        "every complete line, the unknown and unreadable ones kept"
    );
    assert_eq!(
        state
            .sessions_for(project.path())
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.lifecycle.clone()),
        Some(WireLifecycle::Running),
        "still running"
    );
    // The first call took every complete line; what is left is the partial one.
    let all = events_until(&mut client, id, complete(1)).await;
    assert_eq!(
        all.last().map(|r| r.event.event.clone()),
        expected.last().cloned(),
        "the trailing partial line, as an event at exit"
    );
    state.stop_session(id);
}

/// (e) FR-010, US2 scenario 5, US5 scenario 6: stderr goes to the daemon log, never into an event.
#[tokio::test(flavor = "multi_thread")]
async fn the_clis_stderr_is_logged_and_never_an_event() {
    daemon_log();
    let project = tempfile::tempdir().unwrap();
    let script = "stderr claude-stderr-marker\n".to_string() + &replay("text-turn.jsonl");
    let (state, id) = started(project.path(), &script);
    let expected = fixture_events("text-turn.jsonl");
    let mut client = viewer(&state, project.path(), id).await;
    let got = events_until(&mut client, id, complete(expected.len())).await;
    assert_eq!(events_of(&got), expected, "stderr did not enter the stream");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !log_text().contains("claude-stderr-marker") {
        assert!(std::time::Instant::now() < deadline, "never logged");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        log_text()
            .lines()
            .any(|l| l.contains("claude-stderr-marker") && l.contains(&id.0.to_string())),
        "logged with its session"
    );
    state.stop_session(id);
}

fn create(req: u64, project: &Path, provider: AiCli, kind: Option<SessionKind>) -> ClientMsg {
    ClientMsg::SessionCreate {
        req,
        project: project.to_path_buf(),
        worktree_dir: String::new(),
        provider,
        allow_additional_session: true,
        kind: kind.unwrap_or_default(),
    }
}

async fn created(client: &mut Client, req: u64) -> SessionId {
    match answer(client, req).await {
        DaemonMsg::OperationOk {
            result: OperationResult::SessionCreated { session },
            ..
        } => session,
        other => panic!("not created: {other:?}"),
    }
}

fn kind_of(state: &DaemonState, project: &Path, id: SessionId) -> SessionKind {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .expect("listed")
        .kind
}

/// (f) US1 scenario 3, (h) FR-020, US5 scenario 5: a `SessionCreate` without a kind is a terminal
/// session on the PTY path; one with `Structured` is listed as structured.
#[tokio::test(flavor = "multi_thread")]
async fn session_create_keeps_terminal_the_default_and_lists_each_kind() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    stream_script(project.path(), "");
    let state = service(project.path(), vec![]);
    let mut client = connect(&state).await;
    let json = serde_json::json!({ "SessionCreate": {
        "req": 1, "project": project.path(), "worktree_dir": "", "provider": "ClaudeCode",
        "allow_additional_session": true
    }});
    let without_kind: ClientMsg = serde_json::from_value(json).expect("decodes without a kind");
    client.send(Frame::Control(without_kind)).await.unwrap();
    let terminal = created(&mut client, 1).await;
    assert_eq!(
        kind_of(&state, project.path(), terminal),
        SessionKind::Terminal
    );
    assert!(state.primary_pty(terminal).is_some(), "on the PTY path");

    client
        .send(Frame::Control(create(
            2,
            project.path(),
            AiCli::ClaudeCode,
            Some(SessionKind::Structured),
        )))
        .await
        .unwrap();
    let structured = created(&mut client, 2).await;
    assert_eq!(
        kind_of(&state, project.path(), structured),
        SessionKind::Structured
    );
    assert!(state.primary_pty(structured).is_none(), "no PTY");
    for id in [terminal, structured] {
        state.stop_session(id);
    }
}

/// (g) FR-003, US5 scenario 3: a structured Copilot session is refused with the `KindRefusal`
/// sentence and nothing is written.
#[tokio::test(flavor = "multi_thread")]
async fn a_structured_copilot_session_is_refused_and_not_recorded() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    let state = service(project.path(), vec![]);
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create(
            1,
            project.path(),
            AiCli::Copilot,
            Some(SessionKind::Structured),
        )))
        .await
        .unwrap();
    match answer(&mut client, 1).await {
        DaemonMsg::OperationError { message, .. } => assert_eq!(
            message,
            KindRefusal::NotSupported(AiCli::Copilot).to_string()
        ),
        other => panic!("refused: {other:?}"),
    }
    assert!(state.sessions_for(project.path()).is_empty(), "no record");
}

/// Start `id` with `fixture` as its script, and return what a viewer of it receives.
async fn own_events(
    state: &Arc<DaemonState>,
    project: &Path,
    cwd: &Path,
    id: SessionId,
    fixture_name: &str,
) -> Client {
    stream_script(cwd, &replay(fixture_name));
    state
        .start_session_gated(
            id,
            LaunchMode::Fresh,
            micold_core::occupancy::Additional::Yes,
        )
        .expect("starts");
    let mut client = viewer(state, project, id).await;
    // The stand-in reads its script at start: wait for its first event before the next rewrite.
    events_until(&mut client, id, complete(1)).await;
    client
}

/// (i) Principle II: two structured sessions at once, in one worktree and in two, each deliver only
/// their own fixture's events.
#[tokio::test(flavor = "multi_thread")]
async fn two_structured_sessions_each_deliver_only_their_own_events() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    let one = mcp::add_worktree(project.path(), "one");
    let two = mcp::add_worktree(project.path(), "two");
    let wt = |dir: &str| SessionLocation::Worktree(dir.into());
    let sessions = vec![
        structured_session(wt("one")),
        structured_session(wt("one")),
        structured_session(wt("two")),
    ];
    let ids: Vec<SessionId> = sessions.iter().map(|s| s.id).collect();
    let state = service(project.path(), sessions);
    let plan = [
        (ids[0], one.as_path(), "text-turn.jsonl"),
        (ids[1], one.as_path(), "tool-turn.jsonl"),
        (ids[2], two.as_path(), "error-result.jsonl"),
    ];
    let mut clients = Vec::new();
    for (id, cwd, name) in plan {
        clients.push(own_events(&state, project.path(), cwd, id, name).await);
    }
    for ((id, _, name), client) in plan.into_iter().zip(clients.iter_mut()) {
        let expected = fixture_events(name);
        // The viewer was attached from the start: everything again, from a fresh view.
        let mut fresh = viewer(&state, project.path(), id).await;
        let got = events_until(&mut fresh, id, complete(expected.len())).await;
        assert_eq!(events_of(&got), expected, "{name}: only its own events");
        let _ = client;
    }
    for id in ids {
        state.stop_session(id);
    }
}

// --- M3 (T024): the first prompt and user messages (FR-007, FR-008, US1 scenarios 4–5) ---

/// `SessionUserMessage` `req` for `id` with `text`.
fn user_message(req: u64, id: SessionId, text: &str) -> ClientMsg {
    ClientMsg::SessionUserMessage {
        req,
        session: id,
        text: text.into(),
    }
}

/// Send `text` to `id` and return the answer.
async fn send_text(client: &mut Client, req: u64, id: SessionId, text: &str) -> DaemonMsg {
    client
        .send(Frame::Control(user_message(req, id, text)))
        .await
        .expect("sent");
    answer(client, req).await
}

fn assert_ok(answer: &DaemonMsg, what: &str) {
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "{what}: not accepted: {answer:?}"
    );
}

fn assert_refused(answer: &DaemonMsg, why: UserMessageRefusal, what: &str) {
    match answer {
        DaemonMsg::OperationError { message, .. } => {
            assert_eq!(message, &why.to_string(), "{what}: the refusal sentence")
        }
        other => panic!("{what}: not refused: {other:?}"),
    }
}

/// The texts of the user messages the stand-in read in `dir`, one per stdin line.
fn delivered(dir: &Path) -> Vec<String> {
    record(dir, "stdin-record")
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            value["message"]["content"][0]["text"]
                .as_str()
                .expect("a text block")
                .to_owned()
        })
        .collect()
}

/// Wait, within 10 s, until `delivered(dir)` equals `expected`: the stand-in records a message
/// asynchronously, so the daemon's `OperationOk` can arrive before the record has the line.
async fn delivered_is<const N: usize>(dir: &Path, expected: [&str; N]) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let got = delivered(dir);
        if got == expected {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "delivered {got:?}, expected {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// `await-stdin`, then `text-turn.jsonl` with a pause of `pause_ms` before its result.
fn answering_turn(pause_ms: u64) -> String {
    let lines = structured::fixture_lines("text-turn.jsonl");
    let result_at = lines
        .iter()
        .position(|l| l.contains("\"type\":\"result\""))
        .expect("the fixture ends its turn with a result");
    let mut script = vec!["await-stdin".to_owned()];
    script.extend(lines[..result_at].iter().cloned());
    script.push(format!("pause {pause_ms}"));
    script.extend(lines[result_at..].iter().cloned());
    script.join("\n") + "\n"
}

fn is_result(r: &structured::Received) -> bool {
    matches!(r.event.event, StructuredEvent::Result { .. })
}

/// Wait, within 10 s, until the stand-in has started `n` times in `dir`.
async fn started_times(dir: &Path, n: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while structured::starts(dir) < n {
        assert!(
            std::time::Instant::now() < deadline,
            "never started {n} times"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// US1 scenario 4, FR-007: a first prompt is the first stdin line, and a stop/start and a
/// `SessionRestart` do not send it again.
#[tokio::test(flavor = "multi_thread")]
async fn the_first_prompt_is_delivered_once_in_the_sessions_life() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project.path(), vec![session]);
    stream_script(project.path(), &answering_turn(0));
    let prompt = "first \"prompt\"\nzażółć";
    let (_, typed) = micold_daemon::ops::start_with_prompt(
        &state,
        id,
        project.path(),
        AiCli::ClaudeCode,
        LaunchMode::Fresh,
        micold_core::occupancy::Additional::Yes,
        Some(micold_daemon::ops::FirstPrompt {
            text: prompt,
            asked: tokio::time::Instant::now(),
            require_bracketed: false,
        }),
    )
    .await;
    assert!(typed.is_ok(), "the prompt was delivered: {typed:?}");
    delivered_is(project.path(), [prompt]).await;

    state.stop_session(id);
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    started_times(project.path(), 2).await;
    state.stop_session(id);
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::SessionRestart {
            session: id,
            allow_additional_session: true,
        }))
        .await
        .expect("sent");
    started_times(project.path(), 3).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(delivered(project.path()), [prompt], "never sent again");
    state.stop_session(id);
}

/// US1 scenario 5, FR-008: a message is one stdin line with its text unchanged, answered
/// `OperationOk`; the response reaches two viewers; the text is never logged.
#[tokio::test(flavor = "multi_thread")]
async fn a_user_message_is_one_line_and_its_answer_reaches_every_viewer() {
    daemon_log();
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), &answering_turn(0));
    let mut first = viewer(&state, project.path(), id).await;
    let mut second = viewer(&state, project.path(), id).await;
    let text = "two lines\n\"quoted\" — zażółć 🦀 m3-secret-marker";
    assert_ok(&send_text(&mut first, 1, id, text).await, "the message");
    delivered_is(project.path(), [text]).await;
    let expected = fixture_events("text-turn.jsonl");
    for client in [&mut first, &mut second] {
        let got = events_until(client, id, complete(expected.len())).await;
        assert_eq!(events_of(&got), expected, "the answer reached the viewer");
    }
    assert!(
        !log_text().contains("m3-secret-marker"),
        "the text of a message is never logged"
    );
    state.stop_session(id);
}

/// FR-008: blank, mid-turn and back-to-back messages are refused with their sentence; after the
/// result a message is accepted again; a stopped session and a terminal session refuse.
#[tokio::test(flavor = "multi_thread")]
async fn messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal() {
    let project = tempfile::tempdir().unwrap();
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let terminal = ai_session();
    let (id, terminal_id) = (session.id, terminal.id);
    let state = service(project.path(), vec![session, terminal]);
    stream_script(
        project.path(),
        &(answering_turn(1500) + "await-stdin\n" + &replay("text-turn.jsonl")),
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let mut client = viewer(&state, project.path(), id).await;

    for (req, blank) in [(1, ""), (2, "  \n\t")] {
        let got = send_text(&mut client, req, id, blank).await;
        assert_refused(&got, UserMessageRefusal::Blank, "a blank message");
    }
    // Back to back: the second arrives before the first's result.
    client
        .send(Frame::Control(user_message(3, id, "one")))
        .await
        .unwrap();
    client
        .send(Frame::Control(user_message(4, id, "two")))
        .await
        .unwrap();
    assert_ok(&answer(&mut client, 3).await, "the first of two");
    let got = answer(&mut client, 4).await;
    assert_refused(
        &got,
        UserMessageRefusal::TurnInProgress,
        "the second of two",
    );
    let got = send_text(&mut client, 5, id, "before the result").await;
    assert_refused(&got, UserMessageRefusal::TurnInProgress, "mid-turn");

    events_until(&mut client, id, |got| got.iter().any(is_result)).await;
    assert_ok(
        &send_text(&mut client, 6, id, "after").await,
        "after the result",
    );
    delivered_is(project.path(), ["one", "after"]).await;

    state.stop_session(id);
    let got = send_text(&mut client, 7, id, "stopped").await;
    assert_refused(&got, UserMessageRefusal::NotRunning, "a stopped session");
    let got = send_text(&mut client, 8, terminal_id, "terminal").await;
    assert_refused(
        &got,
        UserMessageRefusal::NotStructured,
        "a terminal session",
    );
}

/// Edge case, FR-014: the CLI ends mid-turn: a `TurnEndedWithoutResult` note, and once restarted
/// the next message is accepted.
#[tokio::test(flavor = "multi_thread")]
async fn an_exit_mid_turn_ends_the_turn_with_a_note() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), "await-stdin\nexit 1\n");
    let mut client = viewer(&state, project.path(), id).await;
    assert_ok(&send_text(&mut client, 1, id, "crash").await, "the message");
    events_until(&mut client, id, |got| {
        got.iter()
            .any(|r| r.event.event == StructuredEvent::TurnEndedWithoutResult)
    })
    .await;
    // Supervision restarts the crash; the test drives its sweep, as the daemon's timer does.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while structured::starts(project.path()) < 2 {
        state.supervise_exited_sessions();
        assert!(
            std::time::Instant::now() < deadline,
            "the crash was never restarted"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut req = 2;
    loop {
        // The respawn is registered a moment after the stand-in records its start.
        let got = send_text(&mut client, req, id, "again").await;
        if matches!(got, DaemonMsg::OperationOk { .. }) {
            break;
        }
        assert_refused(&got, UserMessageRefusal::NotRunning, "before the respawn");
        assert!(std::time::Instant::now() < deadline, "never accepted again");
        req += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    state.stop_session(id);
}

/// Principle II: a message to one of two running structured sessions reaches only that one.
#[tokio::test(flavor = "multi_thread")]
async fn a_message_reaches_only_its_own_session() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    let one = mcp::add_worktree(project.path(), "one");
    let two = mcp::add_worktree(project.path(), "two");
    let sessions = vec![
        structured_session(SessionLocation::Worktree("one".into())),
        structured_session(SessionLocation::Worktree("two".into())),
    ];
    let ids: Vec<SessionId> = sessions.iter().map(|s| s.id).collect();
    let state = service(project.path(), sessions);
    for (id, dir) in [(ids[0], &one), (ids[1], &two)] {
        stream_script(dir, &answering_turn(0));
        state
            .start_session_gated(
                id,
                LaunchMode::Fresh,
                micold_core::occupancy::Additional::Yes,
            )
            .expect("starts");
        started_times(dir, 1).await;
    }
    let mut client = connect(&state).await;
    assert_ok(
        &send_text(&mut client, 1, ids[0], "for one").await,
        "the message",
    );
    delivered_is(&one, ["for one"]).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !two.join("stdin-record").exists()
            || std::fs::read_to_string(two.join("stdin-record"))
                .unwrap()
                .is_empty(),
        "the other session read nothing"
    );
    for id in ids {
        state.stop_session(id);
    }
}

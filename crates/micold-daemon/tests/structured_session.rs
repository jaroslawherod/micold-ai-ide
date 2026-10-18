//! Feature 938, milestone M2 (T014): a structured Claude Code session spawned over pipes streams the
//! stand-in's stdout to viewing clients as typed events, with a terminal session's launch.
//!
//! Every case runs the stand-in of `support/structured.rs` (R12), first on `PATH` for the whole test
//! process, scripted per project through `.fake-stream`.
#![cfg(unix)]

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult, WireLifecycle};
use micold_core::session::{AiCli, KindRefusal, SessionId, SessionKind, SessionLocation};
use micold_core::structured::StructuredEvent;
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
use conn::{connect, Client};
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
    let all = events_until(&mut client, id, complete(expected.len())).await;
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

/// The `OperationOk`/`OperationError` answering `req`.
async fn answer(client: &mut Client, req: u64) -> DaemonMsg {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(
                    m @ (DaemonMsg::OperationError { req: r, .. }
                    | DaemonMsg::OperationOk { req: r, .. }),
                ) if r == req => return m,
                _ => continue,
            }
        }
    })
    .await
    .expect("the request is answered")
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

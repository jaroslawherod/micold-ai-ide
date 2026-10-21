//! #942 milestone M2 (T029–T031): a structured session's permission prompts at the daemon.
//!
//! The stand-in `claude` of `support/structured.rs` makes real prompt-tool calls to the daemon's
//! tool server with its own credential (`prompt`, `prompt-async`, research R13), and test clients
//! connected over the wire see each request and decide it. Prompts are switched on for every test
//! (R14).
// unix-only: the stand-in CLI of support/structured.rs is put on PATH as a POSIX executable.
#![cfg(unix)]

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, PermissionRequest, PromptChoice, PromptId, QuestionAnswer, Resolution,
};
use micold_core::session::{PermissionMode, Session, SessionId, SessionLocation};
use micold_core::structured::permission::{DecisionRefusal, RequestKind};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use conn::{connect, Client};
use structured::{
    fake_stream_cli, prompt_answer_count, prompt_answers, prompt_call_file, prompts_on,
    stream_script, structured_session,
};

/// A daemon with a bound tool server and prompts on, holding one structured session per project.
struct Daemon {
    state: Arc<DaemonState>,
    projects: Vec<tempfile::TempDir>,
    ids: Vec<SessionId>,
    _store: tempfile::TempDir,
    _bindings: tempfile::TempDir,
}

impl Daemon {
    /// `n` projects, each with one structured session at its root, none started.
    async fn new(n: usize) -> Self {
        fake_stream_cli();
        mcp::log();
        let projects: Vec<tempfile::TempDir> =
            (0..n).map(|_| tempfile::tempdir().unwrap()).collect();
        let mut ids = Vec::new();
        let listed = projects
            .iter()
            .map(|project| {
                let mut session = structured_session(SessionLocation::Default);
                session.lifecycle = micold_core::session::SessionLifecycle::Idle;
                ids.push(session.id);
                (project.path().to_path_buf(), true, vec![session])
            })
            .collect();
        let store = tempfile::tempdir().unwrap();
        let state = mcp::state_over(listed, store.path());
        let bindings = tempfile::tempdir().unwrap();
        mcp::serve_tool_server(&state, bindings.path().to_path_buf()).await;
        prompts_on(&state);
        Self {
            state,
            projects,
            ids,
            _store: store,
            _bindings: bindings,
        }
    }

    fn dir(&self, n: usize) -> &Path {
        self.projects[n].path()
    }

    /// Start session `n` with `script`.
    async fn start(&self, n: usize, script: &str) {
        stream_script(self.dir(n), script);
        let (state, id) = (Arc::clone(&self.state), self.ids[n]);
        tokio::task::spawn_blocking(move || state.start_session(id, LaunchMode::Fresh))
            .await
            .unwrap()
            .expect("the structured session starts");
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        for id in &self.ids {
            self.state.stop_session(*id);
        }
    }
}

/// The directive making a prompt-tool call for `tool` with `input`, its file written in `dir`.
fn prompt(dir: &Path, name: &str, tool: &str, input: Value) -> String {
    let file = prompt_call_file(dir, name, tool, input, &format!("toolu_{name}"));
    format!("prompt {}\n", file.display())
}

/// As [`prompt`], the call made on its own thread so the script goes on.
fn prompt_async(dir: &Path, name: &str, tool: &str, input: Value) -> String {
    format!(
        "prompt-async{}",
        &prompt(dir, name, tool, input)["prompt".len()..]
    )
}

/// A connected client, keeping the messages it read past while waiting for an answer, so a
/// resolution broadcast before the decision's own answer is still seen.
struct Window {
    client: Client,
    skipped: std::collections::VecDeque<DaemonMsg>,
}

async fn window(state: &Arc<DaemonState>) -> Window {
    Window {
        client: connect(state).await,
        skipped: Default::default(),
    }
}

impl Window {
    /// The next control message, a skipped one first, or `None` once `within` passed.
    async fn next_within(&mut self, within: Duration) -> Option<DaemonMsg> {
        if let Some(msg) = self.skipped.pop_front() {
            return Some(msg);
        }
        tokio::time::timeout(within, async {
            loop {
                match self
                    .client
                    .next()
                    .await
                    .expect("stream open")
                    .expect("a frame")
                {
                    Frame::Control(msg) => return msg,
                    Frame::Grid(_) => {}
                }
            }
        })
        .await
        .ok()
    }

    async fn send(&mut self, msg: ClientMsg) {
        self.client
            .send(Frame::Control(msg))
            .await
            .expect("the message is sent");
    }

    /// The `OperationOk`/`OperationError` answering `req`, keeping what came before it.
    async fn answer(&mut self, req: u64) -> DaemonMsg {
        let mut before = Vec::new();
        let found = loop {
            match self.next_within(Duration::from_secs(15)).await {
                Some(
                    msg @ (DaemonMsg::OperationOk { req: r, .. }
                    | DaemonMsg::OperationError { req: r, .. }),
                ) if r == req => break msg,
                Some(msg) => before.push(msg),
                None => panic!("request {req} is never answered"),
            }
        };
        self.skipped.extend(before);
        found
    }
}

/// The next message of `window` satisfying `pick`, within 15 s.
async fn next_of<T>(window: &mut Window, pick: impl Fn(DaemonMsg) -> Option<T>) -> T {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        let msg = window.next_within(left).await.expect("the message arrives");
        if let Some(found) = pick(msg) {
            return found;
        }
    }
}

async fn requested(window: &mut Window) -> PermissionRequest {
    next_of(window, |msg| match msg {
        DaemonMsg::PermissionRequested { request } => Some(request),
        _ => None,
    })
    .await
}

async fn resolved(window: &mut Window) -> (SessionId, PromptId, Option<String>, Resolution) {
    next_of(window, |msg| match msg {
        DaemonMsg::PermissionResolved {
            session,
            id,
            tool_use_id,
            resolution,
        } => Some((session, id, tool_use_id, resolution)),
        _ => None,
    })
    .await
}

/// Every permission message `window` receives within `within`.
async fn permission_messages_within(window: &mut Window, within: Duration) -> Vec<DaemonMsg> {
    let deadline = tokio::time::Instant::now() + within;
    let mut got = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        match window.next_within(left).await {
            Some(
                msg
                @ (DaemonMsg::PermissionRequested { .. } | DaemonMsg::PermissionResolved { .. }),
            ) => got.push(msg),
            Some(_) => {}
            None => return got,
        }
    }
}

async fn decide(
    window: &mut Window,
    req: u64,
    session: SessionId,
    id: PromptId,
    choice: PromptChoice,
) -> DaemonMsg {
    window
        .send(ClientMsg::PermissionDecision {
            req,
            session,
            id,
            choice,
        })
        .await;
    window.answer(req).await
}

fn assert_ok(answer: &DaemonMsg) {
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "accepted: {answer:?}"
    );
}

fn assert_refused(answer: &DaemonMsg, why: DecisionRefusal) {
    match answer {
        DaemonMsg::OperationError { message, .. } => assert_eq!(message, &why.to_string()),
        other => panic!("expected {why:?}, got {other:?}"),
    }
}

fn allow(input: Value) -> Value {
    json!({"behavior": "allow", "updatedInput": input})
}

fn deny(message: &str) -> Value {
    json!({"behavior": "deny", "message": message})
}

fn bash(command: &str) -> Value {
    json!({"command": command})
}

/// T029 (US1 s1, s5, s6; FR-002, FR-003, FR-005, SC-001): a call becomes a request every client
/// is shown, with its tool, input, tool use id and kind; nothing answers it across 2 s and a
/// client's reconnect, a client connecting meanwhile is shown it once, and the first decision
/// reaches the CLI and closes it everywhere.
#[tokio::test(flavor = "multi_thread")]
async fn a_call_is_shown_to_every_client_and_held_until_one_decides() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let mut b = window(&daemon.state).await;
    let script = prompt(daemon.dir(0), "p1", "Bash", bash("npm test"));
    daemon.start(0, &script).await;

    let request = requested(&mut a).await;
    assert_eq!(requested(&mut b).await, request, "both clients see it");
    assert_eq!(request.session, daemon.ids[0]);
    assert_eq!(request.tool_name, "Bash");
    assert_eq!(request.tool_use_id.as_deref(), Some("toolu_p1"));
    assert_eq!(request.kind, RequestKind::Tool);
    assert_eq!(
        serde_json::from_str::<Value>(&request.input_preview).unwrap(),
        bash("npm test")
    );
    assert_eq!(request.input_bytes, request.input_preview.len() as u64);
    assert_eq!(request.always_allow.as_deref(), Some("Bash: \"npm test\""));

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(
        prompt_answer_count(daemon.dir(0)),
        0,
        "no answer while pending"
    );
    drop(b);
    let mut c = window(&daemon.state).await;
    assert_eq!(
        requested(&mut c).await,
        request,
        "a late client is shown it"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(prompt_answer_count(daemon.dir(0)), 0, "still no answer");

    assert_ok(&decide(&mut a, 1, request.session, request.id, PromptChoice::Allow).await);
    assert_eq!(prompt_answers(daemon.dir(0), 1), [allow(bash("npm test"))]);
    for client in [&mut a, &mut c] {
        assert_eq!(
            resolved(client).await,
            (
                request.session,
                request.id,
                Some("toolu_p1".into()),
                Resolution::Allowed
            )
        );
    }
    let later = permission_messages_within(&mut c, Duration::from_millis(300)).await;
    assert!(later.is_empty(), "nothing shown twice: {later:?}");
}

/// T029 (US1 s2–s4; FR-010): Allow, Deny with and without a message, and Always allow each reach
/// the CLI as their answer and every client sees the resolution; after Always allow an identical
/// call is answered allow without a request.
#[tokio::test(flavor = "multi_thread")]
async fn each_decision_reaches_the_cli_and_always_allow_skips_an_identical_call() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let mut b = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let script = [
        prompt(dir, "p1", "Bash", bash("ls")),
        prompt(dir, "p2", "Bash", bash("rm x")),
        prompt(dir, "p3", "Bash", bash("npm i")),
        prompt(dir, "p4", "Bash", bash("npm test")),
        prompt(dir, "p5", "Bash", bash("npm test")),
    ]
    .concat();
    daemon.start(0, &script).await;

    let cases = [
        (PromptChoice::Allow, allow(bash("ls")), Resolution::Allowed),
        (
            PromptChoice::Deny { message: None },
            deny("The user denied this request."),
            Resolution::Denied,
        ),
        (
            PromptChoice::Deny {
                message: Some("use yarn".into()),
            },
            deny("use yarn"),
            Resolution::Denied,
        ),
        (
            PromptChoice::AlwaysAllow,
            json!({"behavior": "allow", "updatedInput": bash("npm test"),
                "updatedPermissions": [{"type": "addRules",
                    "rules": [{"toolName": "Bash", "ruleContent": "npm test"}],
                    "behavior": "allow", "destination": "session"}]}),
            Resolution::AllowedAlways,
        ),
    ];
    for (n, (choice, expected, resolution)) in cases.into_iter().enumerate() {
        let request = requested(&mut a).await;
        assert_eq!(requested(&mut b).await.id, request.id);
        assert_ok(&decide(&mut a, n as u64, request.session, request.id, choice).await);
        assert_eq!(prompt_answers(dir, n + 1)[n], expected, "case {n}");
        assert_eq!(resolved(&mut a).await.3, resolution);
        assert_eq!(resolved(&mut b).await.3, resolution);
    }
    assert_eq!(
        prompt_answers(dir, 5)[4],
        allow(bash("npm test")),
        "the identical call is answered by the rule"
    );
    let shown = permission_messages_within(&mut b, Duration::from_millis(300)).await;
    assert!(shown.is_empty(), "the rule asks nobody: {shown:?}");
}

/// T029 (FR-005): clients connecting while requests open, one after another, each get every
/// request exactly once, whether it was opened before, during or after their handshake.
#[tokio::test(flavor = "multi_thread")]
async fn a_client_connecting_while_requests_open_gets_each_exactly_once() {
    let daemon = Daemon::new(1).await;
    let dir = daemon.dir(0);
    let mut script = String::new();
    for n in 0..5 {
        script += "pause 15\n";
        script += &prompt_async(dir, &format!("p{n}"), "Bash", bash(&format!("echo {n}")));
    }
    let mut first = window(&daemon.state).await;
    daemon.start(0, &script).await;
    let mut late = Vec::new();
    for _ in 0..10 {
        late.push(window(&daemon.state).await);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let mut all = HashSet::new();
    while all.len() < 5 {
        all.insert(requested(&mut first).await.id);
    }
    for client in &mut late {
        let mut seen = Vec::new();
        while seen.len() < 5 {
            seen.push(requested(client).await.id);
        }
        let more = permission_messages_within(client, Duration::from_millis(200)).await;
        assert!(more.is_empty(), "nothing twice: {more:?}");
        assert_eq!(seen.iter().copied().collect::<HashSet<_>>(), all);
    }
    for (n, id) in all.iter().enumerate() {
        assert_ok(
            &decide(
                &mut first,
                n as u64,
                daemon.ids[0],
                *id,
                PromptChoice::Allow,
            )
            .await,
        );
    }
    prompt_answers(dir, 5);
}

/// T029 (edge case *Bypass mode*): a request raised while Claude reports the Bypass mode is shown
/// and held like any other; the daemon never answers it itself.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_in_bypass_mode_is_shown_and_held() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let script = format!(
        "{}\n{}",
        json!({"type": "system", "subtype": "status", "permissionMode": "bypassPermissions"}),
        prompt(dir, "p1", "Bash", bash("make"))
    );
    daemon.start(0, &script).await;
    let request = requested(&mut a).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(prompt_answer_count(dir), 0, "held");
    assert_ok(&decide(&mut a, 1, request.session, request.id, PromptChoice::Allow).await);
    assert_eq!(prompt_answers(dir, 1), [allow(bash("make"))]);
}

/// T029 (spec edge case *a very large input*, R10): a Write over 64 KiB is shown cut, and
/// `PermissionInput` returns its whole input while it is pending, and is refused after.
#[tokio::test(flavor = "multi_thread")]
async fn the_whole_input_of_a_large_write_is_fetched_on_request() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let input = json!({"file_path": "/w/big.txt", "content": "é".repeat(40 * 1024)});
    let script = prompt(dir, "p1", "Write", input.clone());
    daemon.start(0, &script).await;
    let request = requested(&mut a).await;
    assert!(request.input_preview.len() <= 64 * 1024);
    let whole = serde_json::to_string(&input).unwrap();
    assert_eq!(request.input_bytes, whole.len() as u64);
    assert!(request.input_bytes > request.input_preview.len() as u64);

    let fetch = |req| ClientMsg::PermissionInput {
        req,
        session: request.session,
        id: request.id,
    };
    a.send(fetch(7)).await;
    let text = next_of(&mut a, |msg| match msg {
        DaemonMsg::PermissionInputText { req: 7, text } => Some(text),
        DaemonMsg::OperationError {
            req: 7, message, ..
        } => panic!("refused: {message}"),
        _ => None,
    })
    .await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), input);

    assert_ok(&decide(&mut a, 8, request.session, request.id, PromptChoice::Allow).await);
    a.send(fetch(9)).await;
    assert_refused(&a.answer(9).await, DecisionRefusal::NotPending);
}

/// T030 (US1 s5, SC-002): two clients deciding one request at once, 50 times: exactly one decision
/// reaches the CLI, both see the same one resolution, and the other client is refused.
#[tokio::test(flavor = "multi_thread")]
async fn of_two_concurrent_decisions_exactly_one_wins() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let mut b = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let directive = prompt(dir, "p", "Bash", bash("deploy"));
    daemon.start(0, &directive.repeat(50)).await;
    for round in 0..50u64 {
        let request = requested(&mut a).await;
        assert_eq!(requested(&mut b).await.id, request.id);
        let (session, id) = (request.session, request.id);
        let deny_b = PromptChoice::Deny {
            message: Some("b says no".into()),
        };
        let (from_a, from_b) = tokio::join!(
            decide(&mut a, 2 * round, session, id, PromptChoice::Allow),
            decide(&mut b, 2 * round + 1, session, id, deny_b),
        );
        let a_won = matches!(from_a, DaemonMsg::OperationOk { .. });
        let b_won = matches!(from_b, DaemonMsg::OperationOk { .. });
        assert!(a_won ^ b_won, "round {round}: {from_a:?} / {from_b:?}");
        let loser = if a_won { &from_b } else { &from_a };
        assert_refused(loser, DecisionRefusal::NotPending);
        let (expected, resolution) = if a_won {
            (allow(bash("deploy")), Resolution::Allowed)
        } else {
            (deny("b says no"), Resolution::Denied)
        };
        assert_eq!(
            prompt_answers(dir, round as usize + 1)[round as usize],
            expected
        );
        assert_eq!(resolved(&mut a).await.3, resolution, "round {round}");
        assert_eq!(resolved(&mut b).await.3, resolution, "round {round}");
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(prompt_answer_count(dir), 50, "one answer per call");
}

fn question(text: &str, labels: [&str; 2], multi: bool) -> Value {
    json!({"question": text, "header": text.split(' ').next_back().unwrap(),
        "multiSelect": multi,
        "options": labels.map(|label| json!({"label": label, "description": ""}))})
}

/// T030 (FR-012): a decision for an unknown id, for another session's request, of the wrong kind,
/// an incomplete answer, and one for an answered request are each refused and change nothing.
#[tokio::test(flavor = "multi_thread")]
async fn decisions_for_requests_not_pending_or_not_fitting_are_refused() {
    let daemon = Daemon::new(2).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let questions = json!({"questions": [question("Which color?", ["red", "blue"], false)]});
    let script = [
        prompt(dir, "p1", "Bash", bash("ls")),
        prompt(dir, "p2", "AskUserQuestion", questions),
    ]
    .concat();
    daemon.start(0, &script).await;
    let (own, other) = (daemon.ids[0], daemon.ids[1]);
    let request = requested(&mut a).await;
    let id = request.id;

    let unknown = decide(&mut a, 1, own, PromptId(id.0 + 1000), PromptChoice::Allow).await;
    assert_refused(&unknown, DecisionRefusal::NotPending);
    let foreign = decide(&mut a, 2, other, id, PromptChoice::Allow).await;
    assert_refused(&foreign, DecisionRefusal::OtherSession);
    let plan = PromptChoice::ApprovePlan { accept_edits: true };
    assert_refused(
        &decide(&mut a, 3, own, id, plan).await,
        DecisionRefusal::WrongKind,
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(prompt_answer_count(dir), 0, "nothing reached the CLI");

    assert_ok(&decide(&mut a, 4, own, id, PromptChoice::Allow).await);
    assert_eq!(prompt_answers(dir, 1), [allow(bash("ls"))]);
    let again = decide(&mut a, 5, own, id, PromptChoice::Deny { message: None }).await;
    assert_refused(&again, DecisionRefusal::NotPending);

    let asked = requested(&mut a).await;
    let none = PromptChoice::Answer {
        answers: vec![QuestionAnswer::default()],
    };
    assert_refused(
        &decide(&mut a, 6, own, asked.id, none).await,
        DecisionRefusal::Incomplete,
    );
    let two = PromptChoice::Answer {
        answers: vec![QuestionAnswer {
            labels: vec!["red".into(), "blue".into()],
            other: None,
        }],
    };
    assert_refused(
        &decide(&mut a, 7, own, asked.id, two).await,
        DecisionRefusal::Incomplete,
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(prompt_answer_count(dir), 1, "nothing more reached the CLI");
    let decline = PromptChoice::DeclineQuestion;
    assert_ok(&decide(&mut a, 8, own, asked.id, decline).await);
    prompt_answers(dir, 2);
}

/// T030 (FR-004): a malformed call is answered deny at once, never becomes a request, and is
/// logged naming the tool but not the input.
#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_call_is_denied_at_once_and_logged_without_its_input() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let not_an_object = prompt(dir, "p1", "Bash", json!("cat secret-input-942"));
    let no_tool = dir.join("p2");
    std::fs::write(
        &no_tool,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "permission_prompt", "arguments": {"input": {}}}})
        .to_string(),
    )
    .unwrap();
    let script = format!("{not_an_object}prompt {}\n", no_tool.display());
    daemon.start(0, &script).await;
    let answers = prompt_answers(dir, 2);
    for answer in &answers {
        assert_eq!(answer["behavior"], "deny", "{answer}");
        assert!(answer["message"].as_str().is_some_and(|m| !m.is_empty()));
    }
    let shown = permission_messages_within(&mut a, Duration::from_millis(300)).await;
    assert!(shown.is_empty(), "no request: {shown:?}");
    let lines = mcp::log_lines_for(daemon.ids[0]);
    let malformed: Vec<&String> = lines.iter().filter(|l| l.contains("malformed")).collect();
    assert_eq!(malformed.len(), 2, "{lines:#?}");
    assert!(malformed[0].contains("Bash"), "{}", malformed[0]);
    assert!(
        !lines.iter().any(|l| l.contains("secret-input-942")),
        "{lines:#?}"
    );
}

/// T030 (FR-001, FR-018): a terminal session's credential is offered no prompt tool, and its call
/// of it fails as an unknown tool; a structured one's is offered it.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_sessions_credential_cannot_list_or_call_the_prompt_tool() {
    fake_stream_cli();
    mcp::log();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let bindings = tempfile::tempdir().unwrap();
    let terminal: Session = history::ai_session();
    let structured = structured_session(SessionLocation::Default);
    let (t, s) = (terminal.id, structured.id);
    let state = mcp::state_over(
        vec![(
            project.path().to_path_buf(),
            true,
            vec![terminal, structured],
        )],
        store.path(),
    );
    let addr = mcp::serve_tool_server(&state, bindings.path().to_path_buf()).await;
    prompts_on(&state);
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).to_string();
    let lists_prompt = |body: String| {
        let response: Value = serde_json::from_str(&body).unwrap();
        response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "permission_prompt")
    };
    let terminal_bearer = mcp::credential(&state, t);
    let (_, body) = mcp::post_mcp(addr, Some(&terminal_bearer), &list).await;
    assert!(!lists_prompt(body), "not offered to a terminal session");
    let error = mcp::call_err(
        addr,
        &terminal_bearer,
        "permission_prompt",
        json!({"tool_name": "Bash", "input": {"command": "ls"}}),
    )
    .await;
    assert_eq!(error["message"], "unknown tool \"permission_prompt\"");

    let structured_bearer = mcp::credential(&state, s);
    let (_, body) = mcp::post_mcp(addr, Some(&structured_bearer), &list).await;
    assert!(lists_prompt(body), "offered to a structured session");
    state.set_permission_prompts(false);
    let (_, body) = mcp::post_mcp(addr, Some(&structured_bearer), &list).await;
    assert!(!lists_prompt(body), "not offered while prompts are off");
}

/// T030 (edge case *Several requests at once*): three requests of one session pending at once are
/// decided in another order than they arrived, each answer reaching its own call.
#[tokio::test(flavor = "multi_thread")]
async fn several_pending_requests_are_decided_in_any_order() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let script = ["a", "b", "c"]
        .map(|name| prompt_async(dir, name, "Bash", bash(name)))
        .concat();
    daemon.start(0, &script).await;
    let mut by_command = std::collections::HashMap::new();
    for _ in 0..3 {
        let request = requested(&mut a).await;
        let input: Value = serde_json::from_str(&request.input_preview).unwrap();
        by_command.insert(input["command"].as_str().unwrap().to_string(), request.id);
    }
    for (n, name) in ["c", "a", "b"].into_iter().enumerate() {
        let choice = PromptChoice::Deny {
            message: Some(format!("not {name}")),
        };
        assert_ok(&decide(&mut a, n as u64, daemon.ids[0], by_command[name], choice).await);
        assert_eq!(prompt_answers(dir, n + 1)[n], deny(&format!("not {name}")));
    }
}

/// T030: two structured sessions with requests pending at once stay independent: each decision
/// reaches its own session's CLI, and naming the other session is refused.
#[tokio::test(flavor = "multi_thread")]
async fn two_sessions_with_pending_requests_stay_independent() {
    let daemon = Daemon::new(2).await;
    let mut a = window(&daemon.state).await;
    for n in 0..2 {
        let script = prompt(daemon.dir(n), "p1", "Bash", bash(&format!("job {n}")));
        daemon.start(n, &script).await;
    }
    let mut ids = std::collections::HashMap::new();
    for _ in 0..2 {
        let request = requested(&mut a).await;
        ids.insert(request.session, request.id);
    }
    let (s0, s1) = (daemon.ids[0], daemon.ids[1]);
    let crossed = decide(&mut a, 1, s0, ids[&s1], PromptChoice::Allow).await;
    assert_refused(&crossed, DecisionRefusal::OtherSession);
    assert_ok(
        &decide(
            &mut a,
            2,
            s1,
            ids[&s1],
            PromptChoice::Deny { message: None },
        )
        .await,
    );
    assert_eq!(
        prompt_answers(daemon.dir(1), 1),
        [deny("The user denied this request.")]
    );
    assert_eq!(
        prompt_answer_count(daemon.dir(0)),
        0,
        "the other still waits"
    );
    assert_ok(&decide(&mut a, 3, s0, ids[&s0], PromptChoice::Allow).await);
    assert_eq!(prompt_answers(daemon.dir(0), 1), [allow(bash("job 0"))]);
}

/// T031 (US3 s1, s3; FR-002, FR-010): a plan approval carries the whole plan; Keep planning with
/// a note reaches the CLI as deny with the note; approving reaches it as allow.
#[tokio::test(flavor = "multi_thread")]
async fn a_plan_approval_carries_the_plan_and_keep_planning_returns_the_note() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let plan = format!(
        "# Plan\n\n1. Edit the parser\n2. {}",
        "Test it. ".repeat(10_000)
    );
    let input = json!({"plan": plan});
    let script = [
        prompt(dir, "p1", "ExitPlanMode", input.clone()),
        prompt(dir, "p2", "ExitPlanMode", input.clone()),
    ]
    .concat();
    daemon.start(0, &script).await;

    let request = requested(&mut a).await;
    assert_eq!(
        request.kind,
        RequestKind::PlanApproval { plan: plan.clone() }
    );
    assert_eq!(request.always_allow, None, "no rule for a plan");
    let keep = PromptChoice::KeepPlanning {
        note: Some("also update the docs".into()),
    };
    assert_ok(&decide(&mut a, 1, request.session, request.id, keep).await);
    assert_eq!(prompt_answers(dir, 1), [deny("also update the docs")]);
    assert_eq!(resolved(&mut a).await.3, Resolution::KeptPlanning);

    let request = requested(&mut a).await;
    let approve = PromptChoice::ApprovePlan { accept_edits: true };
    assert_ok(&decide(&mut a, 2, request.session, request.id, approve).await);
    assert_eq!(prompt_answers(dir, 2)[1], allow(input));
    assert_eq!(
        resolved(&mut a).await.3,
        Resolution::PlanApproved {
            mode: PermissionMode::AcceptEdits
        }
    );
}

/// T031 (US4 s1–s3): a question request with two questions, one multi-select, is answered and
/// the CLI gets the `answers` map; declining reaches it as deny.
#[tokio::test(flavor = "multi_thread")]
async fn a_question_request_is_answered_or_declined() {
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    let dir = daemon.dir(0);
    let input = json!({"questions": [
        question("Which color?", ["red", "blue"], false),
        question("Which fruits?", ["apple", "pear"], true),
    ]});
    let script = [
        prompt(dir, "p1", "AskUserQuestion", input.clone()),
        prompt(dir, "p2", "AskUserQuestion", input.clone()),
    ]
    .concat();
    daemon.start(0, &script).await;

    let request = requested(&mut a).await;
    let RequestKind::Question { questions } = &request.kind else {
        panic!("a question request: {:?}", request.kind);
    };
    assert_eq!(questions.len(), 2);
    assert!(!questions[0].multi_select && questions[1].multi_select);
    let answers = PromptChoice::Answer {
        answers: vec![
            QuestionAnswer {
                labels: vec!["red".into()],
                other: None,
            },
            QuestionAnswer {
                labels: vec!["apple".into(), "pear".into()],
                other: None,
            },
        ],
    };
    assert_ok(&decide(&mut a, 1, request.session, request.id, answers).await);
    let mut expected = input.clone();
    expected["answers"] = json!({"Which color?": "red", "Which fruits?": "apple, pear"});
    assert_eq!(prompt_answers(dir, 1), [allow(expected)]);
    assert_eq!(resolved(&mut a).await.3, Resolution::Answered);

    let request = requested(&mut a).await;
    let decline = PromptChoice::DeclineQuestion;
    assert_ok(&decide(&mut a, 2, request.session, request.id, decline).await);
    assert_eq!(
        prompt_answers(dir, 2)[1],
        deny("The user declined to answer.")
    );
    assert_eq!(resolved(&mut a).await.3, Resolution::Declined);
}

/// T032 (R4): an agent that hangs up while its request is pending withdraws it from every client,
/// and a decision after that is refused.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_whose_agent_hangs_up_is_withdrawn() {
    use tokio::io::AsyncWriteExt;
    let daemon = Daemon::new(1).await;
    let mut a = window(&daemon.state).await;
    daemon.start(0, "").await;
    let id = daemon.ids[0];
    let server = daemon.state.tool_server().expect("bound");
    let (addr, bearer) = (server.addr(), server.credential_for(id));
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "permission_prompt",
            "arguments": {"tool_name": "Bash", "input": bash("sleep 1"), "tool_use_id": "toolu_h"}}})
    .to_string();
    let mut agent = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nAuthorization: Bearer {bearer}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    agent.write_all(request.as_bytes()).await.unwrap();
    let shown = requested(&mut a).await;
    drop(agent);
    assert_eq!(
        resolved(&mut a).await,
        (
            id,
            shown.id,
            Some("toolu_h".into()),
            Resolution::Withdrawn {
                reason: micold_core::protocol::messages::WithdrawReason::AgentHungUp
            }
        )
    );
    let late = decide(&mut a, 1, id, shown.id, PromptChoice::Allow).await;
    assert_refused(&late, DecisionRefusal::NotPending);
}

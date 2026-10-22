//! Feature 943, milestone M3 (T028, T029): every structured spawn asks the CLI for its commands
//! with one `initialize` request, kept as a `CommandsReported` note in the event history; commands
//! reach the CLI as typed; `/model` and the terminal-only commands are refused and never reach it.
//!
//! Every case runs the stand-in of `support/structured.rs` (#938 R12, #943 R12).
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::SinkExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::{AiCli, SessionId, SessionLocation};
use micold_core::structured::commands::{parse_initialize, TERMINAL_ONLY};
use micold_core::structured::{ReportedCommand, StructuredEvent};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use conn::{answer, connect, Client};
use history::service;
use structured::{
    events_of, events_until, fake_stream_cli, fixture, fixture_events, is_initialize, reply,
    stdin_lines, stream_script, structured_session, viewer,
};

/// A service with one structured session at `project`'s root, started with `script`; queued
/// messages are delivered as turns end.
fn started(project: &Path, script: &str) -> (Arc<DaemonState>, SessionId) {
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project, vec![session]);
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    stream_script(project, script);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    (state, id)
}

fn replay_each(name: &str) -> String {
    format!("on-message {}\n", fixture(name).display())
}

/// The commands `command-initialize.jsonl`'s recorded answer reports.
fn recorded_commands() -> Vec<ReportedCommand> {
    let outcome = fixture_events("command-initialize.jsonl")
        .into_iter()
        .find_map(|event| match event {
            StructuredEvent::ControlResponse { outcome, .. } => Some(outcome),
            _ => None,
        })
        .expect("the recorded answer");
    let (commands, _) = parse_initialize(&outcome);
    assert!(!commands.is_empty(), "the recording reports commands");
    commands
}

/// The events `id`'s history holds now.
fn history(state: &DaemonState, id: SessionId) -> Vec<StructuredEvent> {
    state
        .event_stream(id)
        .map(|stream| {
            let (_, events, _) = stream.saved_snapshot();
            events.into_iter().map(|e| e.event).collect()
        })
        .unwrap_or_default()
}

fn notes(events: &[StructuredEvent]) -> Vec<&StructuredEvent> {
    events
        .iter()
        .filter(|e| matches!(e, StructuredEvent::CommandsReported { .. }))
        .collect()
}

/// Wait, within 10 s, until `id`'s history holds a `CommandsReported` note.
async fn reported(state: &DaemonState, id: SessionId) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while notes(&history(state, id)).is_empty() {
        assert!(std::time::Instant::now() < deadline, "no note in time");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The text of each user message the stand-in in `dir` read.
fn delivered(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .iter()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            value["message"]["content"][0]["text"]
                .as_str()
                .expect("a text block")
                .to_owned()
        })
        .collect()
}

/// Wait, within 10 s, until the stand-in in `dir` has read exactly `expected`.
async fn delivered_is(dir: &Path, expected: &[&str]) {
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

async fn send_text(client: &mut Client, req: u64, id: SessionId, text: &str) -> DaemonMsg {
    client
        .send(Frame::Control(ClientMsg::SessionUserMessage {
            req,
            session: id,
            text: text.into(),
            images: vec![],
        }))
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

/// T028 (R1, FR-002): the first stdin line of a structured spawn is one `initialize` request, and
/// a first prompt that is a command follows it unchanged as one user message.
#[tokio::test(flavor = "multi_thread")]
async fn initialize_is_the_first_stdin_line_and_a_command_first_prompt_follows_unchanged() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project.path(), vec![session]);
    stream_script(
        project.path(),
        &format!("initialize\n{}", replay_each("command-skill.jsonl")),
    );
    let (_, typed) = micold_daemon::ops::start_with_prompt(
        &state,
        id,
        project.path(),
        AiCli::ClaudeCode,
        LaunchMode::Fresh,
        micold_core::occupancy::Additional::Yes,
        Some(micold_daemon::ops::FirstPrompt {
            text: "/greet Ada",
            asked: tokio::time::Instant::now(),
            require_bracketed: false,
        }),
    )
    .await;
    assert!(typed.is_ok(), "the prompt was delivered: {typed:?}");
    delivered_is(project.path(), &["/greet Ada"]).await;
    let lines: Vec<String> = std::fs::read_to_string(project.path().join("stdin-record"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert!(is_initialize(&lines[0]), "first: {}", lines[0]);
    assert!(lines[1].contains("\"type\":\"user\""), "then: {}", lines[1]);
    state.stop_session(id);
}

/// T028 (R1, FR-009): the answer is one `CommandsReported` note with the reported commands, not the
/// raw answer; a viewer attached before it and one that connects later both get it.
#[tokio::test(flavor = "multi_thread")]
async fn the_answer_is_one_note_seen_by_a_viewer_and_by_a_later_one() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = started(project.path(), "pause 300\ninitialize\n");
    let mut early = viewer(&state, project.path(), id).await;
    let has_note = |got: &[structured::Received]| {
        got.iter()
            .any(|r| matches!(r.event.event, StructuredEvent::CommandsReported { .. }))
    };
    let got = events_of(&events_until(&mut early, id, has_note).await);
    let expected = recorded_commands();
    match notes(&got).as_slice() {
        [StructuredEvent::CommandsReported { commands, models }] => {
            assert_eq!(commands, &expected);
            assert!(!models.is_empty(), "the recording reports models");
        }
        other => panic!("one note: {other:#?}"),
    }
    let mut later = viewer(&state, project.path(), id).await;
    let replayed = events_of(&events_until(&mut later, id, has_note).await);
    assert_eq!(notes(&replayed).len(), 1, "the handshake replays the note");
    let held = history(&state, id);
    assert_eq!(notes(&held).len(), 1, "{held:#?}");
    assert!(
        !held
            .iter()
            .any(|e| matches!(e, StructuredEvent::ControlResponse { .. })),
        "the raw answer is not kept beside the note: {held:#?}"
    );
    state.stop_session(id);
}

/// T028 (R1): an `initialize` the CLI never answers, or refuses, adds no note, and the session
/// keeps running and takes messages.
#[tokio::test(flavor = "multi_thread")]
async fn a_silent_or_refused_initialize_adds_no_note_and_messages_still_run() {
    for first in ["", "initialize error unsupported not here\n"] {
        let project = tempfile::tempdir().unwrap();
        let script = format!("{first}{}", replay_each("text-turn.jsonl"));
        let (state, id) = started(project.path(), &script);
        let mut client = connect(&state).await;
        assert_ok(&send_text(&mut client, 1, id, "hello").await, first);
        delivered_is(project.path(), &["hello"]).await;
        let mut seen = viewer(&state, project.path(), id).await;
        let got = events_until(&mut seen, id, |got| {
            got.iter()
                .any(|r| matches!(r.event.event, StructuredEvent::Result { .. }))
        })
        .await;
        let events = events_of(&got);
        assert!(notes(&events).is_empty(), "{first:?}: {events:#?}");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, StructuredEvent::ControlResponse { .. })),
            "{first:?}: the answer to the initialize is not an event"
        );
        state.stop_session(id);
    }
}

/// T028 (FR-001, FR-002, SC-001 at the daemon): commands the app does not handle reach the CLI as
/// typed, one user message each, known or not; a `/compact` gets the CLI's compaction.
#[tokio::test(flavor = "multi_thread")]
async fn commands_reach_the_cli_unchanged() {
    let project = tempfile::tempdir().unwrap();
    let script = format!(
        "initialize\n{}on-message {}\n",
        reply("/compact", "command-compact.jsonl"),
        fixture("text-turn.jsonl").display()
    );
    let (state, id) = started(project.path(), &script);
    reported(&state, id).await;
    let mut client = connect(&state).await;
    let texts = ["/hello world", "/speckit-plan #1", "/nosuchcmd", "/compact"];
    for (req, text) in (1..).zip(texts) {
        assert_ok(&send_text(&mut client, req, id, text).await, text);
    }
    delivered_is(project.path(), &texts).await;
    let mut seen = viewer(&state, project.path(), id).await;
    let got = events_until(&mut seen, id, |got| {
        got.iter()
            .any(|r| matches!(r.event.event, StructuredEvent::CompactBoundary { .. }))
    })
    .await;
    let sent: Vec<String> = events_of(&got)
        .into_iter()
        .filter_map(|e| match e {
            StructuredEvent::UserSent { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(sent, texts, "each noted once, as typed");
    state.stop_session(id);
}

/// T028 (Principle II): two structured sessions each ask, and each keeps its own note.
#[tokio::test(flavor = "multi_thread")]
async fn two_sessions_each_get_their_own_note() {
    fake_stream_cli();
    let project = tempfile::tempdir().unwrap();
    let sessions = vec![
        structured_session(SessionLocation::Default),
        structured_session(SessionLocation::Default),
    ];
    let ids: Vec<SessionId> = sessions.iter().map(|s| s.id).collect();
    let state = service(project.path(), sessions);
    stream_script(project.path(), "initialize\n");
    for id in &ids {
        state
            .start_session_gated(
                *id,
                LaunchMode::Fresh,
                micold_core::occupancy::Additional::Yes,
            )
            .expect("starts");
    }
    for id in &ids {
        reported(&state, *id).await;
        assert_eq!(notes(&history(&state, *id)).len(), 1);
    }
    let initialize_requests = std::fs::read_to_string(project.path().join("stdin-record"))
        .unwrap()
        .lines()
        .filter(|line| is_initialize(line))
        .count();
    assert_eq!(initialize_requests, 2, "one request per spawn");
    for id in ids {
        state.stop_session(id);
    }
}

/// T029 (FR-010, FR-018, SC-006): `/model`, `/model <name>`, every terminal-only command and the
/// `/settings` alias are refused with the reason, whether the CLI's list is known or not, while
/// idle or busy; none reaches the CLI, is queued or is noted.
#[tokio::test(flavor = "multi_thread")]
async fn model_and_terminal_only_commands_are_refused_and_never_reach_the_cli() {
    let mut refused: Vec<(String, String)> = vec![
        ("/model".into(), "/model is handled by the app".into()),
        ("/model haiku".into(), "/model is handled by the app".into()),
        ("/settings".into(), "/settings needs the terminal".into()),
    ];
    for entry in TERMINAL_ONLY {
        refused.push((
            format!("/{} now", entry.name),
            format!("/{} needs the terminal", entry.name),
        ));
    }
    for first in ["initialize\n", ""] {
        let project = tempfile::tempdir().unwrap();
        let script = format!(
            "{first}on-message-gated {}\n",
            fixture("text-turn.jsonl").display()
        );
        let (state, id) = started(project.path(), &script);
        if !first.is_empty() {
            reported(&state, id).await;
        }
        let mut client = connect(&state).await;
        let mut req = 0;
        for busy in [false, true] {
            if busy {
                req += 1;
                assert_ok(&send_text(&mut client, req, id, "hello").await, "hello");
                delivered_is(project.path(), &["hello"]).await;
            }
            for (text, reason) in &refused {
                req += 1;
                match send_text(&mut client, req, id, text).await {
                    DaemonMsg::OperationError { message, .. } => assert!(
                        message.contains(reason.as_str()),
                        "{text}: {message} lacks {reason}"
                    ),
                    other => panic!("{text} (busy: {busy}): not refused: {other:?}"),
                }
            }
        }
        std::fs::write(project.path().join("turn-1"), "").unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(delivered(project.path()), ["hello"], "{first:?}");
        let sent = history(&state, id)
            .into_iter()
            .filter(|e| matches!(e, StructuredEvent::UserSent { .. }))
            .count();
        assert_eq!(sent, 1, "{first:?}: only hello was noted");
        state.stop_session(id);
    }
}

//! #1732 FR-002, FR-006, FR-007: `read_session` goes through the link directory to the owning
//! daemon's `ReadStructuredEvents` (M1) and returns the content; a terminal session is refused as
//! `terminal_not_supported`, an unknown one `not_found`.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use micold_core::session::{AiCli, Session, SessionLabel, SessionLocation, TerminalMode};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;
use serde_json::{json, Value};

use crate::history::catalog;
use crate::structured::{fake_stream_cli, stream_script, structured_session};
use crate::ws_support::Api;

/// The id of the one project the daemon lists.
async fn project_of(ws: &mut crate::ws_support::Ws) -> String {
    let reply = ws.ask(json!({"id":90,"type":"subscribe"})).await;
    reply["snapshot"]["projects"][0]["id"]
        .as_str()
        .expect("a project")
        .to_owned()
}

fn read(id: u64, project: &str, session: &str) -> Value {
    json!({"id":id,"type":"read_session","daemon":"d1","project":project,"session":session})
}

#[tokio::test(flavor = "multi_thread")]
async fn a_structured_session_is_read_and_a_terminal_one_refused() {
    let project = tempfile::tempdir().unwrap();
    fake_stream_cli();
    let structured = structured_session(SessionLocation::Default);
    let terminal = Session::restored(
        micold_core::session::SessionId::new(),
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    let (structured_id, terminal_id) = (structured.id, terminal.id);
    let api = Api::start().await;
    let daemon = api
        .daemon("d1", catalog(project.path(), vec![structured, terminal]))
        .await;
    stream_script(project.path(), "");
    daemon
        .state
        .start_session(structured_id, LaunchMode::Fresh)
        .expect("starts");
    let stream = daemon.state.event_stream(structured_id).expect("a stream");
    for n in 0..3 {
        stream.push(StructuredEvent::Unreadable {
            text: format!("line {n}"),
        });
    }

    let mut ws = api.client().await;
    let project_id = project_of(&mut ws).await;

    let content = ws
        .ask(read(1, &project_id, &structured_id.to_string()))
        .await;
    assert_eq!(content["type"], "session_content", "{content}");
    let texts: Vec<&str> = content["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["text"].as_str())
        .filter(|t| t.starts_with("line "))
        .collect();
    assert_eq!(texts, ["line 0", "line 1", "line 2"], "{content}");

    let refused = ws.ask(read(2, &project_id, &terminal_id.to_string())).await;
    assert_eq!(refused["code"], "terminal_not_supported", "{refused}");

    let unknown = ws
        .ask(read(3, &project_id, "00000000-0000-0000-0000-000000000000"))
        .await;
    assert_eq!(unknown["code"], "not_found", "{unknown}");
    let no_project = ws.ask(read(4, "nope", &structured_id.to_string())).await;
    assert_eq!(no_project["code"], "not_found", "{no_project}");
}

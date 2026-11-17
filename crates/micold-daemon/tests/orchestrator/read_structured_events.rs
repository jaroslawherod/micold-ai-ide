//! #1732 M1 (T003): `ClientMsg::ReadStructuredEvents` reads a structured session's history without
//! attaching, locking or viewing, so reading never displaces a desktop window (FR-002, FR-014b,
//! research R9). Runs against a real daemon state over an in-memory connection.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason, StructuredReadRefusal};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;

use super::conn::{connect, connect_and_attach, Client};
use super::history::service;
use super::structured::{fake_stream_cli, stream_script, structured_session};

fn line(n: usize) -> StructuredEvent {
    StructuredEvent::Unreadable {
        text: format!("line {n}"),
    }
}

async fn ask(client: &mut Client, project: &Path, session: SessionId) {
    client
        .send(Frame::Control(ClientMsg::ReadStructuredEvents {
            project: project.to_path_buf(),
            session,
        }))
        .await
        .expect("the read is sent");
}

/// The next answer to a read: a `StructuredEvents` or a refusal, within 15 s. Anything else the
/// connection carries (a catalog push) is skipped; a `Refused` fails the test.
async fn answer(client: &mut Client) -> DaemonMsg {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match client.next().await.expect("open").expect("a frame") {
                Frame::Control(
                    m @ (DaemonMsg::StructuredEvents { .. }
                    | DaemonMsg::StructuredEventsRefused { .. }),
                ) => return m,
                Frame::Control(DaemonMsg::Refused { reason }) => {
                    panic!("the read was refused as a whole: {reason:?}")
                }
                _ => {}
            }
        }
    })
    .await
    .expect("the read is answered")
}

#[tokio::test(flavor = "multi_thread")]
async fn it_returns_the_history_while_a_desktop_holds_the_project() {
    let project = tempfile::tempdir().unwrap();
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project.path(), vec![session]);
    stream_script(project.path(), "");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let stream = state.event_stream(id).expect("a stream");
    for n in 0..3 {
        stream.push(line(n));
    }

    let mut desktop = connect_and_attach(&state, project.path()).await;
    let mut reader = connect(&state).await;
    ask(&mut reader, project.path(), id).await;

    let DaemonMsg::StructuredEvents {
        session,
        omitted,
        events,
        live,
    } = answer(&mut reader).await
    else {
        panic!("expected the history")
    };
    assert_eq!(session, id);
    assert_eq!(omitted, 0);
    assert!(!live, "a read is the non-live history only");
    assert_eq!(
        events.iter().map(|e| e.event.clone()).collect::<Vec<_>>(),
        (0..3).map(line).collect::<Vec<_>>()
    );

    // The read is one-shot: a later event is not pushed to the reader.
    stream.push(line(3));
    let more = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            if let Some(Ok(Frame::Control(DaemonMsg::StructuredEvents { .. }))) =
                reader.next().await
            {
                return;
            }
        }
    })
    .await;
    assert!(more.is_err(), "a read leaves no subscription behind");

    // The desktop still holds the project: nothing displaced it.
    let displaced = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            match desktop.next().await {
                Some(Ok(Frame::Control(
                    DaemonMsg::Displaced { .. }
                    | DaemonMsg::Refused {
                        reason: RefusalReason::ProjectBusy { .. },
                    },
                ))) => return,
                Some(_) => {}
                None => return,
            }
        }
    })
    .await;
    assert!(displaced.is_err(), "the desktop was kicked out by a read");
    state.stop_session(id);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_session_answers_terminal_session() {
    let project = tempfile::tempdir().unwrap();
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    let state = service(project.path(), vec![session]);
    let mut reader = connect(&state).await;
    ask(&mut reader, project.path(), id).await;
    assert!(matches!(
        answer(&mut reader).await,
        DaemonMsg::StructuredEventsRefused {
            session,
            reason: StructuredReadRefusal::TerminalSession,
        } if session == id
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_session_answers_not_found() {
    let project = tempfile::tempdir().unwrap();
    let state = service(project.path(), vec![]);
    let unknown = SessionId::new();
    let mut reader = connect(&state).await;
    ask(&mut reader, project.path(), unknown).await;
    assert!(matches!(
        answer(&mut reader).await,
        DaemonMsg::StructuredEventsRefused {
            session,
            reason: StructuredReadRefusal::NotFound,
        } if session == unknown
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_that_never_ran_has_an_empty_history() {
    let project = tempfile::tempdir().unwrap();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project.path(), vec![session]);
    let mut reader = connect(&state).await;
    ask(&mut reader, project.path(), id).await;
    let DaemonMsg::StructuredEvents { events, live, .. } = answer(&mut reader).await else {
        panic!("expected the history")
    };
    assert!(events.is_empty() && !live);
}

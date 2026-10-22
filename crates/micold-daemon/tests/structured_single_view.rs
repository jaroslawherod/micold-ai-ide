//! BUG-1256: a structured session shown in a single-pane layout streams its events. A window with
//! one pane names its session with `SetViewedSession`, not `SetViewedTerminals`, and the daemon
//! used to stream only a PTY for it, so the transcript stayed "No output yet".
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, SessionProcess};
use micold_core::session::{SessionId, SessionLocation, ShellInstanceId};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use history::service;
use structured::{
    events_of, events_until, fake_stream_cli, fixture, fixture_events, stream_script,
    structured_session,
};

const FIXTURE: &str = "text-turn.jsonl";

/// A service with one structured session at `project`'s root that replays [`FIXTURE`] when started.
fn service_with_session(project: &Path) -> (Arc<DaemonState>, SessionId) {
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project, vec![session]);
    stream_script(project, &format!("replay {}\n", fixture(FIXTURE).display()));
    (state, id)
}

/// A client of `state` viewing `id` as a single-pane window does.
async fn single_viewer(state: &Arc<DaemonState>, project: &Path, id: SessionId) -> conn::Client {
    let mut client = conn::connect_as(state, "test-client").await;
    send(
        &mut client,
        ClientMsg::SetViewedSession {
            project: project.to_path_buf(),
            session: Some(id),
        },
    )
    .await;
    client
}

async fn send(client: &mut conn::Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.expect("sent");
}

/// Whether `client` receives a `StructuredEvents` of `id` within `window`.
async fn events_within(client: &mut conn::Client, id: SessionId, window: Duration) -> bool {
    let end = tokio::time::Instant::now() + window;
    while let Ok(Some(Ok(frame))) = tokio::time::timeout_at(end, client.next()).await {
        if matches!(frame, Frame::Control(DaemonMsg::StructuredEvents { session, .. }) if session == id)
        {
            return true;
        }
    }
    false
}

/// A running structured session viewed with `SetViewedSession` streams its events.
#[tokio::test(flavor = "multi_thread")]
async fn a_single_view_of_a_running_structured_session_streams_its_events() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = service_with_session(project.path());
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let expected = fixture_events(FIXTURE);
    let mut client = single_viewer(&state, project.path(), id).await;
    let got = events_until(&mut client, id, |got| got.len() >= expected.len()).await;
    assert_eq!(events_of(&got), expected);
    state.stop_session(id);
}

/// The view arrives before the session has started, as a window's `SessionStart` +
/// `SetViewedSession` do: the events stream once the start finishes.
#[tokio::test(flavor = "multi_thread")]
async fn a_single_view_sent_before_the_start_streams_once_it_starts() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = service_with_session(project.path());
    let mut client = single_viewer(&state, project.path(), id).await;
    send(
        &mut client,
        ClientMsg::SessionStart {
            session: id,
            allow_additional_session: false,
        },
    )
    .await;
    let expected = fixture_events(FIXTURE);
    let got = events_until(&mut client, id, |got| got.len() >= expected.len()).await;
    assert_eq!(events_of(&got), expected);
    state.stop_session(id);
}

/// Attaching a shell instance shows the shell instead; attaching the primary again, or closing the
/// attached shell, streams the events again.
#[tokio::test(flavor = "multi_thread")]
async fn reattaching_the_primary_of_a_single_view_streams_its_events_again() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = service_with_session(project.path());
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let instance = ShellInstanceId(1);
    state.open_shell(id, instance).expect("shell opens");
    let expected = fixture_events(FIXTURE);
    let mut client = single_viewer(&state, project.path(), id).await;
    events_until(&mut client, id, |got| got.len() >= expected.len()).await;

    let attach = |process| ClientMsg::SessionAttachProcess {
        session: id,
        process,
    };
    send(&mut client, attach(SessionProcess::Shell(instance))).await;
    assert!(
        !events_within(&mut client, id, Duration::from_millis(500)).await,
        "the shell is shown, not the events"
    );
    send(&mut client, attach(SessionProcess::Primary)).await;
    let got = events_until(&mut client, id, |got| got.len() >= expected.len()).await;
    assert_eq!(
        events_of(&got),
        expected,
        "a fresh history after the attach"
    );

    send(&mut client, attach(SessionProcess::Shell(instance))).await;
    send(
        &mut client,
        ClientMsg::SessionCloseShell {
            session: id,
            instance,
        },
    )
    .await;
    assert!(
        events_within(&mut client, id, Duration::from_secs(10)).await,
        "closing the attached shell shows the events again"
    );
    state.stop_session(id);
}

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

/// Whether `client` receives a frame that `wanted` holds for within `window`.
async fn receives(
    client: &mut conn::Client,
    window: Duration,
    wanted: impl Fn(&Frame<DaemonMsg>) -> bool,
) -> bool {
    let end = tokio::time::Instant::now() + window;
    while let Ok(Some(Ok(frame))) = tokio::time::timeout_at(end, client.next()).await {
        if wanted(&frame) {
            return true;
        }
    }
    false
}

/// Whether `frame` carries `id`'s events.
fn is_events(id: SessionId) -> impl Fn(&Frame<DaemonMsg>) -> bool {
    move |frame| matches!(frame, Frame::Control(DaemonMsg::StructuredEvents { session, .. }) if *session == id)
}

/// Whether `frame` is a screen of `id`'s `process`.
fn is_screen_of(id: SessionId, process: SessionProcess) -> impl Fn(&Frame<DaemonMsg>) -> bool {
    move |frame| matches!(frame, Frame::Grid(g) if g.session == id && g.process == process)
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
    let shell = SessionProcess::Shell(instance);
    send(&mut client, attach(shell)).await;
    assert!(
        receives(
            &mut client,
            Duration::from_secs(10),
            is_screen_of(id, shell)
        )
        .await,
        "the shell is shown"
    );
    // The event stream stopped with the shell attached: attaching the primary starts a new one,
    // which begins with the history again.
    send(&mut client, attach(SessionProcess::Primary)).await;
    let got = events_until(&mut client, id, |got| got.len() >= expected.len()).await;
    assert_eq!(
        events_of(&got),
        expected,
        "a fresh history after the attach"
    );

    send(&mut client, attach(shell)).await;
    send(
        &mut client,
        ClientMsg::SessionCloseShell {
            session: id,
            instance,
        },
    )
    .await;
    assert!(
        receives(&mut client, Duration::from_secs(10), is_events(id)).await,
        "closing the attached shell shows the events again"
    );
    state.stop_session(id);
}

/// Restarting the attached shell instance of a structured session attaches the fresh instance: the
/// view shows it and input without a `process` reaches it.
#[tokio::test(flavor = "multi_thread")]
async fn restarting_the_attached_shell_of_a_structured_session_attaches_the_new_one() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = service_with_session(project.path());
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let instance = ShellInstanceId(1);
    let shell = SessionProcess::Shell(instance);
    state.open_shell(id, instance).expect("shell opens");
    let mut client = single_viewer(&state, project.path(), id).await;
    send(
        &mut client,
        ClientMsg::SessionAttachProcess {
            session: id,
            process: shell,
        },
    )
    .await;
    assert!(
        receives(
            &mut client,
            Duration::from_secs(10),
            is_screen_of(id, shell)
        )
        .await
    );
    send(
        &mut client,
        ClientMsg::SessionRestartShell {
            session: id,
            instance,
        },
    )
    .await;
    let marker = project.path().join("restarted-shell-served");
    send(
        &mut client,
        ClientMsg::SessionInput {
            session: id,
            serial: 0,
            bytes: format!(": > '{}'\n", marker.display()).into_bytes(),
            process: None,
        },
    )
    .await;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "the restarted shell is not attached: it never ran the typed command"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        receives(
            &mut client,
            Duration::from_secs(10),
            is_screen_of(id, shell)
        )
        .await,
        "the restarted shell is shown"
    );
    state.stop_session(id);
}

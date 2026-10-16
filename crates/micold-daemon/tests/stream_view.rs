//! Phase 4 (US2) — the drive loop over the wire: a viewing client receives grid frames for a live
//! session and drives it with input, all through `serve_connection` (FR-014/FR-016/FR-019/FR-020).
//!
//! This closes the loop the unit tests prove in halves: `client_input` stamps a `SessionInput`,
//! `drive_loop` shows the daemon writes it to the PTY, `reattach_snapshot` frames a `Term`. Here the
//! bytes travel the real socket: view a session → get a full snapshot → type "hello" → get a delta
//! that shows it echoed back.

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;

#[path = "support/conn.rs"]
mod conn;
use conn::{connect, Client};

/// Register a live session of `cmd` with `scrollback` lines of history, 80x24, under a new id.
fn spawn_session(state: &DaemonState, mut cmd: CommandBuilder, scrollback: usize) -> SessionId {
    let sid = SessionId::new();
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(sid, cmd, scrollback, Some((80, 24))).expect("spawn session");
    state.register_session(session);
    sid
}

/// Ask the daemon to stream `sid` (under `project`) to `client`.
async fn view_session(client: &mut Client, project: PathBuf, sid: SessionId) {
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project,
            session: Some(sid),
        }))
        .await
        .unwrap();
}

/// Say goodbye, then stop the test-owned process so nothing leaks.
async fn finish(mut client: Client, state: &DaemonState, sid: SessionId) {
    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    if let Some(pty) = state.live_session(sid) {
        let _ = pty.kill();
    }
}

fn frame_has_text(frame: &GridFrame, needle: &str) -> bool {
    frame.lines.iter().any(|l| l.text.contains(needle))
}

/// A process that echoes whatever is typed into its PTY: a deterministic sink for proving typed
/// bytes come back as rendered output.
#[cfg(unix)]
fn echo_sink() -> CommandBuilder {
    CommandBuilder::new("cat")
}

/// Windows has no `cat`; an interactive `cmd` echoes typed input through the console's line editor.
#[cfg(windows)]
fn echo_sink() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.arg("/q");
    cmd
}

/// Print `scrollback_line_1` to `scrollback_line_100`, then idle for about a minute.
#[cfg(unix)]
fn hundred_lines_then_idle() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("i=1; while [ $i -le 100 ]; do echo scrollback_line_$i; i=$((i+1)); done; sleep 60");
    cmd
}

/// `cmd` has no `sleep`; `ping -n 61` to loopback waits about a minute.
#[cfg(windows)]
fn hundred_lines_then_idle() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.args([
        "/c",
        "(for /l %i in (1,1,100) do @echo scrollback_line_%i)& ping -n 61 127.0.0.1 >nul",
    ]);
    cmd
}

/// Read frames until one is a `Grid` frame satisfying `pred`, or the deadline passes. Returns
/// whether such a frame arrived. Control frames along the way are ignored.
async fn wait_for_grid<C>(
    client: &mut Framed<tokio::io::DuplexStream, C>,
    mut pred: impl FnMut(&GridFrame) -> bool,
) -> bool
where
    C: tokio_util::codec::Decoder<Item = Frame<DaemonMsg>> + Unpin,
    C::Error: std::fmt::Debug,
{
    let overall = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < overall {
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if pred(&frame) => return true,
            Ok(Some(Ok(_))) => continue, // another grid/control frame — keep looking
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => return false, // connection closed
            Err(_) => continue,       // read timeout — try again until the overall deadline
        }
    }
    false
}

#[tokio::test]
async fn a_viewing_client_receives_frames_and_can_drive_the_session() {
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));

    // A live echoing session: whatever input is written to its PTY comes back as output.
    let sid = spawn_session(&state, echo_sink(), 1_000);

    let mut client = connect(&state).await;

    // View the session: the daemon streams a full snapshot first (the current — here blank — screen).
    view_session(&mut client, PathBuf::from("/repo/demo"), sid).await;
    assert!(
        wait_for_grid(&mut client, |f| f.full).await,
        "viewing a session must yield a full snapshot frame first"
    );

    // Drive it: type "hello". The stamp mirrors what the client stamper produces (first serial 0).
    client
        .send(Frame::Control(ClientMsg::SessionInput {
            process: None,
            session: sid,
            serial: 0,
            bytes: b"hello\n".to_vec(),
        }))
        .await
        .unwrap();

    // The echo comes back as a streamed grid delta — the loop is closed end to end.
    assert!(
        wait_for_grid(&mut client, |f| frame_has_text(f, "hello")).await,
        "typed input must drive the session and stream back the echoed output"
    );

    finish(client, &state, sid).await;
}

#[tokio::test]
async fn session_resize_reframes_at_the_new_size() {
    // Regression (code review): the daemon must honour ClientMsg::SessionResize; before the fix it
    // fell into `_ => {}` and every session stayed at the 100×30 spawn seed.
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));
    let sid = spawn_session(&state, echo_sink(), 1_000);

    let mut client = connect(&state).await;
    view_session(&mut client, PathBuf::from("/repo/demo"), sid).await;
    assert!(
        wait_for_grid(&mut client, |f| f.cols == 80).await,
        "first snapshot is at the 80-col spawn size"
    );

    client
        .send(Frame::Control(ClientMsg::SessionResize {
            process: None,
            session: sid,
            cols: 120,
            rows: 40,
        }))
        .await
        .unwrap();
    assert!(
        wait_for_grid(&mut client, |f| f.cols == 120 && f.rows == 40).await,
        "after SessionResize the stream must re-frame at the new size"
    );

    finish(client, &state, sid).await;
}

/// Read frames until a full-snapshot `Grid` frame arrives, returning it (or `None` on timeout).
async fn read_full_snapshot<C>(client: &mut Framed<tokio::io::DuplexStream, C>) -> Option<GridFrame>
where
    C: tokio_util::codec::Decoder<Item = Frame<DaemonMsg>> + Unpin,
    C::Error: std::fmt::Debug,
{
    let overall = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < overall {
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.full => return Some(frame),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => return None,
            Err(_) => continue,
        }
    }
    None
}

#[tokio::test]
async fn a_client_can_fetch_scrollback_history_over_the_wire() {
    // A session that emits ~100 lines then idles — far more than the 24-row screen, so most of it
    // is scrollback the daemon retains but does not stream.
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));
    let sid = spawn_session(&state, hundred_lines_then_idle(), 10_000);

    let mut client = connect(&state).await;
    view_session(&mut client, PathBuf::from("/repo/demo"), sid).await;

    // Wait until the last emitted line reaches the screen — that proves all 100 lines have been
    // processed into the Term (and thus into history), robust to load-dependent timing.
    assert!(
        wait_for_grid(&mut client, |f| frame_has_text(f, "scrollback_line_100")).await,
        "the session's output must be fully processed"
    );
    // Re-view for a fresh full snapshot to learn the watermark.
    view_session(&mut client, PathBuf::from("/repo/demo"), sid).await;
    let snap = read_full_snapshot(&mut client)
        .await
        .expect("full snapshot");
    // History exists below the viewport (lines scrolled off the 24-row screen).
    assert!(
        snap.viewport_top.0 > snap.oldest_available.0,
        "there should be retained scrollback below the viewport"
    );

    // Request the earliest handful of scrollback lines — content NOT in the streamed viewport.
    use micold_core::protocol::grid::LineId;
    let from = snap.oldest_available;
    let to = LineId(snap.oldest_available.0 + 5);
    client
        .send(Frame::Control(ClientMsg::ScrollbackRequest {
            session: sid,
            req: 1,
            ranges: vec![from..to],
        }))
        .await
        .unwrap();

    // The response carries historical lines with their own palette.
    let mut got = None;
    let overall = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < overall {
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::ScrollbackResponse { req, lines, .. })))) => {
                assert_eq!(req, 1);
                got = Some(lines);
                break;
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    let lines = got.expect("a ScrollbackResponse arrived");
    assert!(!lines.is_empty(), "the daemon served retained history");
    assert!(
        lines.iter().any(|l| l.text.contains("scrollback_line_")),
        "the served lines are the scrolled-off content"
    );

    finish(client, &state, sid).await;
}

/// A catalog holding one Regular (shell) session at `project`, so `SessionStart` can spawn it.
fn catalog_with_shell(
    project: &std::path::Path,
    store: &std::path::Path,
    id: SessionId,
) -> Catalog {
    use micold_core::project::{Availability, Project};
    use micold_core::session::{AiCli, Session, SessionLabel, SessionLocation, TerminalMode};
    use micold_core::store::ProjectStore;
    use micold_core::workspace::Workspace;
    use std::collections::BTreeMap;

    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.join("projects.json");
    micold_core::store::JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(micold_core::store::JsonFileStore::at(projects_path)),
        Box::new(micold_core::settings::JsonFileSettingsStore::at(
            store.join("settings.json"),
        )),
    )
}

#[tokio::test]
async fn a_client_can_start_view_and_drive_a_session_from_cold_over_the_wire() {
    // The full cold-start loop: nothing is pre-registered. The client asks the daemon to Start a
    // durable session, then views and drives it — all through serve_connection.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let sid = SessionId::new();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_shell(
        project.path(),
        store.path(),
        sid,
    )));
    let mut client = connect(&state).await;

    // Bring the session to life, then view it — the daemon spawns the shell and streams a snapshot.
    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: sid,
            allow_additional_session: false,
        }))
        .await
        .unwrap();
    view_session(&mut client, project.path().to_path_buf(), sid).await;
    assert!(
        wait_for_grid(&mut client, |f| f.full).await,
        "starting + viewing a session must yield a full snapshot"
    );

    // Drive the freshly-started shell: the tty echoes typed input back as streamed output.
    client
        .send(Frame::Control(ClientMsg::SessionInput {
            process: None,
            session: sid,
            serial: 0,
            bytes: b"wire_marker\n".to_vec(),
        }))
        .await
        .unwrap();
    assert!(
        wait_for_grid(&mut client, |f| frame_has_text(f, "wire_marker")).await,
        "a cold-started session must be drivable end to end"
    );

    finish(client, &state, sid).await;
}

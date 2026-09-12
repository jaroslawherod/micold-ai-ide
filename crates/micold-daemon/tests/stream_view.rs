//! Phase 4 (US2) — the drive loop over the wire: a viewing client receives grid frames for a live
//! session and drives it with input, all through `serve_connection` (FR-014/FR-016/FR-019/FR-020).
//!
//! This closes the loop the unit tests prove in halves: `client_input` stamps a `SessionInput`,
//! `drive_loop` shows the daemon writes it to the PTY, `reattach_snapshot` frames a `Term`. Here the
//! bytes travel the real socket: view a session → get a full snapshot → type "hello" → get a delta
//! that shows it echoed back.

#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;

fn frame_has_text(frame: &GridFrame, needle: &str) -> bool {
    frame.lines.iter().any(|l| l.text.contains(needle))
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
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));

    // A live `cat` session: it echoes whatever input is written to its PTY — a deterministic sink
    // for proving typed bytes come back as rendered output.
    let sid = SessionId::new();
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(sid, cmd, 1_000, Some((80, 24))).expect("spawn cat session");
    state.register_session(session);

    let server = tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());

    // Handshake.
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }

    // View the session: the daemon streams a full snapshot first (the current — here blank — screen).
    let project = PathBuf::from("/repo/demo");
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project,
            session: Some(sid),
        }))
        .await
        .unwrap();
    assert!(
        wait_for_grid(&mut client, |f| f.full).await,
        "viewing a session must yield a full snapshot frame first"
    );

    // Drive it: type "hello". The stamp mirrors what the client stamper produces (first serial 0).
    client
        .send(Frame::Control(ClientMsg::SessionInput {
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

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    let _ = server.await;

    // Test-owned process: stop it so nothing leaks.
    if let Some(pty) = state.live_session(sid) {
        let _ = pty.kill();
    }
}

#[tokio::test]
async fn session_resize_reframes_at_the_new_size() {
    // Regression (code review): the daemon must honour ClientMsg::SessionResize; before the fix it
    // fell into `_ => {}` and every session stayed at the 100×30 spawn seed.
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));
    let sid = SessionId::new();
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(sid, cmd, 1_000, Some((80, 24))).expect("spawn");
    state.register_session(session);

    let server = tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: PathBuf::from("/repo/demo"),
            session: Some(sid),
        }))
        .await
        .unwrap();
    assert!(
        wait_for_grid(&mut client, |f| f.cols == 80).await,
        "first snapshot is at the 80-col spawn size"
    );

    client
        .send(Frame::Control(ClientMsg::SessionResize {
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

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    let _ = server.await;
    if let Some(pty) = state.live_session(sid) {
        let _ = pty.kill();
    }
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
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));
    let sid = SessionId::new();
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("i=1; while [ $i -le 100 ]; do echo scrollback_line_$i; i=$((i+1)); done; sleep 60");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(sid, cmd, 10_000, Some((80, 24))).expect("spawn");
    state.register_session(session);

    let server = tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: std::path::PathBuf::from("/repo/demo"),
            session: Some(sid),
        }))
        .await
        .unwrap();

    // Wait until the last emitted line reaches the screen — that proves all 100 lines have been
    // processed into the Term (and thus into history), robust to load-dependent timing.
    assert!(
        wait_for_grid(&mut client, |f| frame_has_text(f, "scrollback_line_100")).await,
        "the session's output must be fully processed"
    );
    // Re-view for a fresh full snapshot to learn the watermark.
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: std::path::PathBuf::from("/repo/demo"),
            session: Some(sid),
        }))
        .await
        .unwrap();
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

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    let _ = server.await;
    if let Some(pty) = state.live_session(sid) {
        let _ = pty.kill();
    }
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
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_shell(
        project.path(),
        store.path(),
        sid,
    )));
    let server = tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());

    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }

    // Bring the session to life, then view it — the daemon spawns the shell and streams a snapshot.
    client
        .send(Frame::Control(ClientMsg::SessionStart { session: sid }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: project.path().to_path_buf(),
            session: Some(sid),
        }))
        .await
        .unwrap();
    assert!(
        wait_for_grid(&mut client, |f| f.full).await,
        "starting + viewing a session must yield a full snapshot"
    );

    // Drive the freshly-started shell: the tty echoes typed input back as streamed output.
    client
        .send(Frame::Control(ClientMsg::SessionInput {
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

    client
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .unwrap();
    let _ = server.await;
    if let Some(pty) = state.live_session(sid) {
        let _ = pty.kill();
    }
}

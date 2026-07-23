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
use micold_core::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};
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

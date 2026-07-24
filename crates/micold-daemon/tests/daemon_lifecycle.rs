//! T023/T024 — the daemon lifecycle rule and the catalog push projection.
//!
//! - The daemon stays up with one live session and no clients, and may exit only at zero/zero
//!   (FR-002, data-model G4).
//! - A catalog/settings mutation reaches a *second* connected client with no user action (FR-011).
//! - Attach is exclusive, and a forced takeover displaces the holder without terminating it
//!   (FR-023/024) — the routing added in T022.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};
use micold_daemon::catalog::Catalog;
use micold_daemon::lifecycle::may_exit;
use micold_daemon::state::DaemonState;
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;

type Client = Framed<DuplexStream, ClientCodec>;

fn new_state() -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::ephemeral()))
}

/// Connect a client through the real `serve_connection` path and complete the handshake.
async fn connect(state: &Arc<DaemonState>, build: &str) -> Client {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: build.into(),
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Poll until `cond` holds (the server task deregisters asynchronously).
async fn wait_until(mut cond: impl FnMut() -> bool) {
    for _ in 0..400 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("condition never became true");
}

#[test]
fn may_exit_requires_both_counts_at_zero() {
    assert!(may_exit(0, 0));
    assert!(
        !may_exit(1, 0),
        "a live session alone must keep the daemon up"
    );
    assert!(!may_exit(0, 1), "a connected client alone must keep it up");
}

#[tokio::test]
async fn daemon_stays_up_with_a_live_session_after_the_last_client_leaves() {
    let state = new_state();
    let client = connect(&state, "client-a").await;
    wait_until(|| state.client_count() == 1).await;

    // A session's process is alive (the supervisor will drive this for real in T031).
    state.lifecycle().session_started();
    assert!(!state.lifecycle().may_exit());

    // The client goes away — closing the UI must change nothing about the session (FR-007).
    drop(client);
    wait_until(|| state.client_count() == 0).await;

    assert_eq!(state.lifecycle().counts(), (1, 0));
    assert!(
        !state.lifecycle().may_exit(),
        "the daemon must NOT exit while a session is alive, even with zero clients"
    );

    // Only once the session ends too is exit permitted.
    state.lifecycle().session_ended();
    assert!(state.lifecycle().may_exit());
}

#[tokio::test]
async fn a_settings_mutation_reaches_a_second_connected_client() {
    let state = new_state();
    let mut a = connect(&state, "client-a").await;
    let mut b = connect(&state, "client-b").await;
    wait_until(|| state.client_count() == 2).await;

    // Client A changes a service-owned setting.
    a.send(Frame::Control(ClientMsg::SettingsSet {
        req: 1,
        scrollback_lines: Some(5_000),
    }))
    .await
    .unwrap();

    // Client B is told, without doing anything itself (FR-011).
    match b.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::SettingsChanged { settings }) => {
            assert_eq!(settings.scrollback_lines, 5_000);
        }
        other => panic!("expected SettingsChanged on the second client, got {other:?}"),
    }
}

#[tokio::test]
async fn attach_is_exclusive_and_a_forced_takeover_displaces_the_holder() {
    let state = new_state();
    let mut a = connect(&state, "client-a").await;
    let mut b = connect(&state, "client-b").await;
    wait_until(|| state.client_count() == 2).await;

    let project = PathBuf::from("/repo/alpha");

    // A attaches first and holds the project.
    a.send(Frame::Control(ClientMsg::Attach {
        project: project.clone(),
        force: false,
    }))
    .await
    .unwrap();
    match a.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Attached { project: p, .. }) => assert_eq!(p, project),
        other => panic!("expected Attached, got {other:?}"),
    }
    assert!(state.is_attached(&project));

    // B is refused with an actionable takeover offer, not queued (FR-023).
    b.send(Frame::Control(ClientMsg::Attach {
        project: project.clone(),
        force: false,
    }))
    .await
    .unwrap();
    match b.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Refused {
            reason:
                RefusalReason::ProjectBusy {
                    project: p, holder, ..
                },
        }) => {
            assert_eq!(p, project);
            assert_eq!(
                holder, "client-a",
                "the refusal must name the current holder"
            );
        }
        other => panic!("expected Refused::ProjectBusy, got {other:?}"),
    }

    // B forces the takeover: B gets Attached, A is told it was Displaced (but is NOT terminated).
    b.send(Frame::Control(ClientMsg::Attach {
        project: project.clone(),
        force: true,
    }))
    .await
    .unwrap();
    match b.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Attached { project: p, .. }) => assert_eq!(p, project),
        other => panic!("expected Attached after force, got {other:?}"),
    }
    // A also has a targeted `CatalogChanged` queued from its own successful attach (the daemon sends
    // the attaching client the refreshed catalog + worktrees, T053) — skip past it to the Displaced.
    loop {
        match a.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { .. }) => continue,
            Frame::Control(DaemonMsg::Displaced { project: p, by }) => {
                assert_eq!(p, project);
                assert_eq!(by, "client-b");
                break;
            }
            other => panic!("expected Displaced on the previous holder, got {other:?}"),
        }
    }

    // The displaced client is still connected — displacement never terminates it (FR-024/T4).
    assert_eq!(state.client_count(), 2);
}

#[tokio::test]
async fn a_disconnect_releases_the_attachment() {
    let state = new_state();
    let mut a = connect(&state, "client-a").await;
    let project = PathBuf::from("/repo/gamma");

    a.send(Frame::Control(ClientMsg::Attach {
        project: project.clone(),
        force: false,
    }))
    .await
    .unwrap();
    match a.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Attached { .. }) => {}
        other => panic!("expected Attached, got {other:?}"),
    }
    assert!(state.is_attached(&project));

    // The connection owns the attachment, so EOF is the release signal (data-model T2) — this is
    // what lets a crashed holder free the project without restarting the daemon.
    drop(a);
    wait_until(|| !state.is_attached(&project)).await;
}

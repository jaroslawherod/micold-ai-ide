//! Phase 7 (US5) — one viewer per project, with deliberate takeover (T064–T066).
//!
//! These drive `server::serve_connection` over in-memory duplexes with the real `ClientCodec`, one
//! per simulated window, all sharing a single `DaemonState`. That is exactly the multi-window shape
//! FR-023/024/025 govern: a second attach on a held project is refused with an actionable takeover
//! offer; a confirmed `force` takeover displaces the prior holder *without terminating it*; a holder
//! that simply disconnects frees the project; and two windows on two different projects never
//! interfere.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// Handshake a fresh client against `state`, consuming its `Welcome`. Returns the framed stream; the
/// server task is detached but kept alive as long as the returned stream (its socket) is held.
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
}

/// The next control message, skipping the unsolicited `CatalogChanged` pushes an attach triggers
/// (worktree discovery, FR-018) and any keepalive `Pong` — neither is what an exclusivity assertion
/// is about.
async fn next_control(client: &mut Client) -> DaemonMsg {
    loop {
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { .. })
            | Frame::Control(DaemonMsg::Pong { .. }) => continue,
            Frame::Control(msg) => return msg,
            Frame::Grid(_) => continue,
        }
    }
}

/// Send `Attach` and return the daemon's decision (an `Attached` or a `Refused`).
async fn attach(client: &mut Client, project: &str, force: bool) -> DaemonMsg {
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.into(),
            force,
        }))
        .await
        .unwrap();
    next_control(client).await
}

fn state() -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::ephemeral()))
}

#[tokio::test]
async fn a_second_attach_is_refused_then_force_displaces_the_holder() {
    let state = state();
    let mut a = connect(&state, "window-A").await;
    let mut b = connect(&state, "window-B").await;

    // A takes the project.
    match attach(&mut a, "/proj", false).await {
        DaemonMsg::Attached { project, .. } => assert_eq!(project, std::path::Path::new("/proj")),
        other => panic!("A expected Attached, got {other:?}"),
    }

    // B is refused with an actionable, holder-naming offer — not silently, not by force (FR-023).
    match attach(&mut b, "/proj", false).await {
        DaemonMsg::Refused {
            reason: RefusalReason::ProjectBusy {
                project, holder, ..
            },
        } => {
            assert_eq!(project, std::path::Path::new("/proj"));
            assert_eq!(holder, "window-A"); // the refusal names the current holder
        }
        other => panic!("B expected ProjectBusy, got {other:?}"),
    }

    // The user confirms takeover: B forces, B is Attached, and A is Displaced *by B* (FR-023/024).
    match attach(&mut b, "/proj", true).await {
        DaemonMsg::Attached { project, .. } => assert_eq!(project, std::path::Path::new("/proj")),
        other => panic!("B expected Attached after force, got {other:?}"),
    }
    match next_control(&mut a).await {
        DaemonMsg::Displaced { project, by } => {
            assert_eq!(project, std::path::Path::new("/proj"));
            assert_eq!(by, "window-B");
        }
        other => panic!("A expected Displaced, got {other:?}"),
    }
}

#[tokio::test]
async fn a_displaced_client_is_not_terminated() {
    // FR-024: the displaced window MUST NOT exit — its connection stays open and responsive. We prove
    // liveness by a post-displacement Ping/Pong round-trip.
    let state = state();
    let mut a = connect(&state, "window-A").await;
    let mut b = connect(&state, "window-B").await;

    attach(&mut a, "/proj", false).await;
    attach(&mut b, "/proj", true).await;

    match next_control(&mut a).await {
        DaemonMsg::Displaced { .. } => {}
        other => panic!("A expected Displaced, got {other:?}"),
    }

    // The connection is still alive: the daemon answers A's keepalive.
    a.send(Frame::Control(ClientMsg::Ping { nonce: 7 }))
        .await
        .unwrap();
    // Read raw so the Pong is asserted directly (next_control skips Pong by design).
    loop {
        match a.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::Pong { nonce }) => {
                assert_eq!(nonce, 7);
                break;
            }
            Frame::Control(DaemonMsg::CatalogChanged { .. }) | Frame::Grid(_) => continue,
            other => panic!("expected Pong, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_crashed_holder_frees_the_project_without_a_restart() {
    // FR-025 (Edge: holder dies): a holder that just disconnects — no clean Detach, the crash case —
    // frees the project so the next attach succeeds by default, no `force`, no daemon restart.
    let state = state();
    let mut a = connect(&state, "window-A").await;
    match attach(&mut a, "/proj", false).await {
        DaemonMsg::Attached { .. } => {}
        other => panic!("A expected Attached, got {other:?}"),
    }

    // A "crashes": drop its socket. EOF is the release signal (the connection owns the attachment).
    drop(a);

    // The same shared daemon frees the project once the disconnect is observed. A fresh window then
    // attaches by default. Retry briefly — the peer's deregister races the new attach.
    let mut b = connect(&state, "window-B").await;
    let mut last = None;
    for _ in 0..50 {
        match attach(&mut b, "/proj", false).await {
            DaemonMsg::Attached { .. } => return,
            other => {
                last = Some(other);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    }
    panic!("B never attached after the holder crashed; last was {last:?}");
}

#[tokio::test]
async fn two_clients_on_two_projects_do_not_interfere() {
    // FR-024 scenario 4: exclusivity is per-project, so two windows on two different projects both
    // attach cleanly and neither refuses the other.
    let state = state();
    let mut a = connect(&state, "window-A").await;
    let mut b = connect(&state, "window-B").await;

    match attach(&mut a, "/proj-1", false).await {
        DaemonMsg::Attached { project, .. } => assert_eq!(project, std::path::Path::new("/proj-1")),
        other => panic!("A expected Attached, got {other:?}"),
    }
    match attach(&mut b, "/proj-2", false).await {
        DaemonMsg::Attached { project, .. } => assert_eq!(project, std::path::Path::new("/proj-2")),
        other => panic!("B expected Attached, got {other:?}"),
    }
}

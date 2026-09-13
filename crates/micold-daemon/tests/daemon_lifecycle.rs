//! T023/T024 — the daemon lifecycle rule and the catalog push projection.
//!
//! - The presence count is what decides whether the service is idle, it is fed by the real accept
//!   loop, and **a live session is not an input to it** (feature 028 FR-006a, T007).
//! - A catalog/settings mutation reaches a *second* connected client with no user action (FR-011).
//! - Attach is exclusive, and a forced takeover displaces the holder without terminating it
//!   (FR-023/024) — the routing added in T022.
//!
//! What the *count* does under each way a connection can end — refused, closed cleanly, dropped —
//! lives in `presence_counting.rs` (T029/T030). This file asserts the rule read against it.
//!
//! # What feature 028 changed here
//!
//! This file used to assert `may_exit(live_sessions, connected_clients)` — "the daemon never exits
//! while a session is alive" — and that predicate had no call site anywhere, so the daemon had in
//! fact never exited on its own (research R1). Feature 028 makes the rule real and narrows it to
//! connections alone: a machine left with an agent running and no window open must not keep a
//! service alive forever. The work is protected on the way *down* instead — every live session is
//! marked `InterruptedResumable` and persisted before anything is killed (data-model G5), the same
//! durable situation as any other service restart.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::idle::{IdleWindow, Presence};
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

/// The rule the daemon now actually runs, over the presence the real accept loop feeds.
///
/// One client attaches and leaves; the window arms at the moment it left, and the rule is unexpired
/// before it and expired after. Driven through `serve_connection` rather than by calling `Presence`
/// directly, because the thing worth asserting is that the count the rule reads is the one the
/// transport produces — research R1's "no call sites" finding is exactly what this closes.
#[tokio::test]
async fn the_window_arms_when_the_last_client_leaves_and_expires_after_it() {
    let state = new_state();
    let client = connect(&state, "client-a").await;
    wait_until(|| state.client_count() == 1).await;

    let rule = IdleWindow::default();
    let busy = state.presence();
    assert_eq!(busy.connected(), 1);
    assert_eq!(
        busy.alone_since(),
        None,
        "the window must not be armed while a client is connected"
    );
    assert!(
        !rule.expired(&busy, micold_core::clock::now()),
        "a connected client must hold the service up"
    );

    // The client goes away — closing the UI must change no session's fate (FR-006, §2.4).
    drop(client);
    wait_until(|| state.client_count() == 0).await;

    let alone = state.presence();
    assert_eq!(alone.connected(), 0);
    let armed = alone
        .alone_since()
        .expect("the window must arm when the last client leaves");
    assert!(
        !rule.expired(&alone, armed),
        "not idle the instant it empties"
    );
    assert!(
        rule.expired(&alone, micold_core::clock::Uptime::from_nanos(u64::MAX)),
        "thirty minutes with nobody connected must expire the window"
    );
}

/// FR-006a, the clarified rule: a live session does **not** hold the service up.
///
/// There is no session count to assert against any more — the predicate takes one argument, and
/// that is the point. What this pins is that the shape cannot quietly grow one back: `Presence`
/// exposes a connection count and an armed deadline, and nothing else.
#[tokio::test]
async fn a_live_session_does_not_hold_the_service_up() {
    let state = new_state();
    let client = connect(&state, "client-a").await;
    wait_until(|| state.client_count() == 1).await;
    drop(client);
    wait_until(|| state.client_count() == 0).await;

    // Whatever sessions this daemon is running, this is the entire state the rule reads.
    let presence: Presence = state.presence();
    assert!(
        IdleWindow::default().expired(&presence, micold_core::clock::Uptime::from_nanos(u64::MAX)),
        "the window must expire on an empty presence regardless of what is still running"
    );
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
        env_include_enabled: None,
        env_include_script_path: None,
        env_include_timeout_secs: None,
        default_ai_cli: None,
        pi_activity_component: None,
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
                holder.build, "client-a",
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
                assert_eq!(by.build, "client-b");
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

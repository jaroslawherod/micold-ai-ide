//! T029/T030 [US2] — the presence count is the honest one (lifecycle contract §2.5, §2.6).
//!
//! The idle rule is only as good as the count it reads, and every way this count could lie is a
//! daemon that either stops on a live user or never stops at all. So each transition is driven
//! through the real `serve_connection` path rather than by calling [`Presence`] directly — the unit
//! tests in `micold_daemon::idle` already pin the type's own algebra, and what is left to prove is
//! that the transport calls it, once, on every way a connection can end.
//!
//! Research R1 is why this file exists: the predicate this feature replaces had *no call sites*, so
//! the daemon had never exited on its own and nothing would have noticed if the counting had been
//! wrong the whole time.
//!
//! The three endings a connection has:
//!
//! - **Refused** (§2.5) — never counted, because the refusal returns before `register`.
//! - **Clean close** — `Goodbye`, then the socket goes.
//! - **Unclean drop** (§2.6) — the peer vanishes with no warning, which is what a crash or a
//!   `SIGKILL`ed window looks like from here. Counted gone on EOF, with no keepalive (research R6).

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;

type Client = Framed<DuplexStream, ClientCodec>;

fn new_state() -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::ephemeral()))
}

/// A `Hello` this daemon will accept, except for whatever the caller overrides.
fn hello(build: &str, protocol_version: u32) -> ClientMsg {
    ClientMsg::Hello {
        protocol_version,
        schema_hash: SCHEMA_HASH,
        client_build: build.into(),
        client_instance: ClientInstance::current(),
        client_package_version: PACKAGE_VERSION.into(),
        // Feature 027: the host-process placement presents no token, and a fingerprint mismatch is
        // not a refusal there.
        auth_token: None,
        client_fingerprint: BUILD_FINGERPRINT.into(),
        require_fingerprint_match: false,
    }
}

/// Connect through the real `serve_connection` and complete the handshake.
async fn connect(state: &Arc<DaemonState>, build: &str) -> Client {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(hello(build, PROTOCOL_VERSION)))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Poll until `cond` holds, returning how long it took. Panics past `timeout`.
///
/// Returning the elapsed time rather than just waiting is what lets §2.6's "within 60 seconds" be
/// asserted as a measurement instead of assumed from the fact that the test finished.
async fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> Duration {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if cond() {
            return started.elapsed();
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("condition never became true within {timeout:?}");
}

/// §2.5, the half that keeps a stale build from pinning a daemon open.
///
/// A refused client is the one that arrives *repeatedly*: a client from a previous release
/// reconnecting on a loop produces a refusal every time, and counting any of them would hold the
/// service up forever on behalf of something that cannot talk to it.
#[tokio::test]
async fn a_refused_handshake_never_increments_the_count() {
    let state = new_state();
    let before = state.presence();

    for attempt in 0..5 {
        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let served = tokio::spawn(micold_daemon::server::serve_connection(
            Arc::clone(&state),
            server_io,
        ));
        let mut client = Framed::new(client_io, ClientCodec::new());
        client
            .send(Frame::Control(hello(
                "a-stale-build",
                // Any version but this daemon's is a contract mismatch.
                PROTOCOL_VERSION.wrapping_add(1),
            )))
            .await
            .unwrap();
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::Refused { .. }) => {}
            other => panic!("attempt {attempt}: expected Refused, got {other:?}"),
        }
        let _ = served.await;

        let presence = state.presence();
        assert_eq!(
            presence.connected(),
            0,
            "attempt {attempt}: a refused client was counted"
        );
        assert_eq!(
            presence.alone_since(),
            before.alone_since(),
            "attempt {attempt}: a refusal moved the idle deadline; a reconnect loop would then \
             keep the service alive forever"
        );
    }
}

/// A completed handshake counts once — not once per frame, and not once per attach.
///
/// Two clients rather than one, because "increments once" and "increments" are the same assertion
/// at a count of one: only the second connection distinguishes a counter from a boolean.
#[tokio::test]
async fn a_completed_handshake_increments_the_count_once() {
    let state = new_state();

    let mut a = connect(&state, "client-a").await;
    wait_until(Duration::from_secs(2), || state.presence().connected() == 1).await;
    assert_eq!(
        state.presence().alone_since(),
        None,
        "the idle deadline must be disarmed while a client is connected"
    );

    let _b = connect(&state, "client-b").await;
    wait_until(Duration::from_secs(2), || state.presence().connected() == 2).await;

    // Traffic on an already-counted connection must not count again.
    a.send(Frame::Control(ClientMsg::Ping { nonce: 1 }))
        .await
        .unwrap();
    match a.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Pong { .. }) => {}
        other => panic!("expected Pong, got {other:?}"),
    }
    assert_eq!(
        state.presence().connected(),
        2,
        "a message on an existing connection changed the count"
    );
}

/// A clean close decrements, and the last one out arms the window.
#[tokio::test]
async fn a_clean_close_decrements_the_count() {
    let state = new_state();
    let mut a = connect(&state, "client-a").await;
    let b = connect(&state, "client-b").await;
    wait_until(Duration::from_secs(2), || state.presence().connected() == 2).await;

    // A says goodbye and goes. One left, so the window stays disarmed.
    a.send(Frame::Control(ClientMsg::Goodbye)).await.unwrap();
    drop(a);
    wait_until(Duration::from_secs(2), || state.presence().connected() == 1).await;
    assert_eq!(
        state.presence().alone_since(),
        None,
        "the window armed while a client was still connected"
    );

    drop(b);
    wait_until(Duration::from_secs(2), || state.presence().connected() == 0).await;
    assert!(
        state.presence().alone_since().is_some(),
        "the window must arm when the last client leaves"
    );
}

/// §2.6: a connection dropped with no clean close is counted gone within 60 seconds.
///
/// No `Goodbye`, no shutdown — the socket simply ends, which is what the daemon sees when a window
/// is `SIGKILL`ed, when the machine's session ends, or when a client panics. The bound is asserted
/// against a measured elapsed time rather than inferred from the test completing, because a
/// mechanism that took five minutes would still make a test that only waits pass.
///
/// Research R6 chose EOF over a keepalive precisely so this is immediate rather than within a
/// probe interval; the sixty seconds is the contract's ceiling, not the design target.
#[tokio::test]
async fn an_unclean_drop_is_counted_gone_within_sixty_seconds() {
    let state = new_state();
    let client = connect(&state, "client-a").await;
    wait_until(Duration::from_secs(2), || state.presence().connected() == 1).await;

    // The peer vanishes. Nothing is sent, and nothing is closed in an orderly way.
    drop(client);

    let elapsed = wait_until(Duration::from_secs(60), || {
        state.presence().connected() == 0
    })
    .await;
    assert!(
        elapsed < Duration::from_secs(60),
        "an unclean disconnect took {elapsed:?} to be counted gone (§2.6 allows 60s)"
    );
    assert!(
        state.presence().alone_since().is_some(),
        "the window must arm after an unclean disconnect too — a crashed client is not company"
    );
}

/// T032: there is exactly one count, so the observability accessor cannot disagree with the rule.
///
/// `client_count()` used to report the length of the client table, a second thing that happened to
/// track the same transitions. Two counters that agree today are two counters that can drift, and
/// the drift would be invisible: the daemon would stop with a window open, or stay up with none.
#[tokio::test]
async fn the_reported_client_count_is_the_presence_count() {
    let state = new_state();
    assert_eq!(state.client_count(), state.presence().connected());

    let a = connect(&state, "client-a").await;
    let b = connect(&state, "client-b").await;
    wait_until(Duration::from_secs(2), || state.presence().connected() == 2).await;
    assert_eq!(state.client_count(), 2);
    assert_eq!(state.client_count(), state.presence().connected());

    drop(a);
    wait_until(Duration::from_secs(2), || state.presence().connected() == 1).await;
    assert_eq!(state.client_count(), state.presence().connected());

    drop(b);
    wait_until(Duration::from_secs(2), || state.presence().connected() == 0).await;
    assert_eq!(state.client_count(), state.presence().connected());
}

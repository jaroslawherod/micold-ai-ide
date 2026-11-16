//! #1726 US3/US4 (FR-007–FR-009, SC-004, SC-005): the app's one orchestrator connection, headless.
//! A real `micold-daemon orchestrator` process on a temp runtime dir, started by the connection
//! itself, and a real daemon (in this process, no socket of its own) linked to it as `local`.

#[path = "orchestrator_world/mod.rs"]
mod orchestrator_world;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_client::orchestrator_conn::OrchestratorConn;
use micold_core::connect::{handshake, Connected, DaemonConnection};
use micold_core::protocol::routing::WireDaemonId;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{announcement, spawn_with, LinkHandle};
use micold_daemon::state::DaemonState;
use orchestrator_world::{daemon_binary, link_as, World, WITHIN};

/// A real daemon linked to the world's orchestrator as `local`.
fn link_local(world: &World) -> (Arc<DaemonState>, LinkHandle) {
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Host\"\nkind = \"host\"\n\
         [orchestrator_link]\nenabled = true\nsocket = \"{}\"\n",
        world.socket().display()
    );
    let config =
        Arc::new(micold_core::daemon_config::parse(&text, "daemon.toml").expect("a valid config"));
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let link = spawn_with(&config, announcement(&config), Some(Arc::clone(&state)))
        .expect("the link is enabled");
    (state, link)
}

/// Open a route to `local` and handshake, looking again until the daemon has linked.
async fn welcome(conn: &OrchestratorConn) -> DaemonConnection {
    let deadline = Instant::now() + WITHIN;
    loop {
        let route = conn
            .open(&WireDaemonId::local())
            .await
            .expect("the orchestrator connection opens");
        match handshake(route, "test-app").await {
            Ok(Connected::Ready(framed, _)) => return *framed,
            Ok(Connected::Refused(reason)) => panic!("refused: {reason:?}"),
            Err(e) => assert!(Instant::now() < deadline, "no Welcome from local: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_app_starts_the_orchestrator_and_reaches_the_daemon_over_one_connection() {
    let world = World::new(daemon_binary());
    let conn = OrchestratorConn::new(world.starter.clone());
    let started = Instant::now();
    // No orchestrator runs: opening a route starts one (US3.1, SC-004).
    let _first = conn
        .open(&WireDaemonId::local())
        .await
        .expect("the orchestrator is started");
    assert!(started.elapsed() < Duration::from_secs(5), "SC-004");
    assert_eq!(world.running(), 1);
    let (_state, _link) = link_local(&world);
    // Two actors' routes: both reach the daemon, over the one connection (FR-007). The daemon has
    // no socket of its own, so the orchestrator is the only way to it.
    let _a = welcome(&conn).await;
    let _b = welcome(&conn).await;
    assert_eq!(conn.connections_opened(), 1);
    assert_eq!(world.running(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn two_concurrent_starts_leave_one_orchestrator() {
    let world = World::new(daemon_binary());
    let (a, b) = (
        OrchestratorConn::new(world.starter.clone()),
        OrchestratorConn::new(world.starter.clone()),
    );
    let local = WireDaemonId::local();
    let (ra, rb) = tokio::join!(a.open(&local), b.open(&local));
    ra.expect("the first app connects");
    rb.expect("the second app connects");
    // US3.3: whatever both started, the singleton leaves one, and both are connected to it.
    world.settles_on(1).await;
    assert!(a.is_connected().await && b.is_connected().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_orchestrator_that_cannot_start_is_named() {
    let world = World::new(PathBuf::from("/nonexistent/micold-daemon"));
    let conn = OrchestratorConn::new(world.starter.clone());
    let err = conn
        .open(&WireDaemonId::local())
        .await
        .expect_err("no binary, no orchestrator");
    // US3.4: the message names the cause; nothing fell back to a daemon socket.
    let text = err.to_string();
    assert!(text.contains("/nonexistent/micold-daemon"), "{text}");
    assert_eq!(conn.connections_opened(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn after_the_orchestrator_is_killed_the_app_starts_it_again_and_re_handshakes() {
    use futures_util::StreamExt;

    let world = World::new(daemon_binary());
    let conn = OrchestratorConn::new(world.starter.clone());
    let _ = conn.open(&WireDaemonId::local()).await.expect("started");
    let (state, _link) = link_local(&world);
    let mut route = welcome(&conn).await;

    // US4.1: the orchestrator stops; the route ends and the daemon keeps its state.
    world.kill_all();
    let ended = tokio::time::timeout(WITHIN, async {
        while let Some(Ok(_)) = route.next().await {}
    })
    .await;
    assert!(ended.is_ok(), "the route ends when the orchestrator goes");

    // US4.2 / FR-009: the next open starts the orchestrator again, the daemon links again, and a
    // fresh handshake (the re-read) reaches the same daemon.
    let _again = welcome(&conn).await;
    assert_eq!(conn.connections_opened(), 2, "one new connection");
    assert!(world.started() >= 2, "started again");
    assert_eq!(world.running(), 1);
    drop(state);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_route_to_a_provisioned_daemon_waits_for_its_link_and_reaches_it() {
    let world = World::new(daemon_binary());
    let conn = Arc::new(OrchestratorConn::new(world.starter.clone()));
    let id = WireDaemonId::new("box-1").unwrap();
    // FR-007, M4: the route is asked for before the daemon has linked (its bridge was just closed
    // and its link redials); it succeeds once the daemon links and handshakes.
    let routed = {
        let (conn, id) = (Arc::clone(&conn), id.clone());
        tokio::spawn(async move {
            conn.routed_to(&id, Duration::from_secs(10), "test-app")
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (_state, _link) = link_as(&world, "box-1");
    match routed.await.unwrap().expect("routed") {
        Connected::Ready(..) => {}
        Connected::Refused(reason) => panic!("refused: {reason:?}"),
    }
    assert_eq!(
        conn.connections_opened(),
        1,
        "one connection, none to a daemon"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_that_never_links_fails_the_route_naming_its_wire_id() {
    let world = World::new(daemon_binary());
    let conn = OrchestratorConn::new(world.starter.clone());
    let id = WireDaemonId::new("box-2").unwrap();
    let Err(err) = conn
        .routed_to(&id, Duration::from_secs(1), "test-app")
        .await
    else {
        panic!("nothing links, yet the route was made")
    };
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(err.to_string().contains("box-2"), "{err}");
    assert_eq!(conn.connections_opened(), 1);
}

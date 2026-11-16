//! #1726 US3/US4 (FR-007–FR-009, SC-004, SC-005): the app's one orchestrator connection, headless.
//! A real `micold-daemon orchestrator` process on a temp runtime dir, started by the connection
//! itself, and a real daemon (in this process, no socket of its own) linked to it as `local`.

#[path = "daemon_bin/mod.rs"]
mod daemon_bin;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use micold_client::orchestrator_conn::{OrchestratorConn, Starter};
use micold_core::connect::{handshake, Connected, DaemonConnection};
use micold_core::endpoint::Endpoint;
use micold_core::protocol::routing::WireDaemonId;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{announcement, spawn_with, LinkHandle};
use micold_daemon::state::DaemonState;

const WITHIN: Duration = Duration::from_secs(20);

fn daemon_binary() -> PathBuf {
    daemon_bin::built_daemon().unwrap_or_else(|| {
        panic!(
            "build micold-daemon first; none above {:?}",
            std::env::current_exe()
        )
    })
}

/// A temp runtime and data dir, and every orchestrator the starter started.
struct World {
    _root: tempfile::TempDir,
    children: Arc<Mutex<Vec<Child>>>,
    starter: Starter,
}

impl World {
    fn new(binary: PathBuf) -> Self {
        let root = tempfile::tempdir().expect("temp dir");
        let runtime = root.path().join("run");
        std::fs::create_dir(&runtime).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let dir = runtime.join("micold");
        let endpoint = Endpoint {
            socket_path: dir.join("orchestrator.sock"),
            lock_path: dir.join("orchestrator.lock"),
        };
        let children: Arc<Mutex<Vec<Child>>> = Arc::default();
        let (spawned, home) = (Arc::clone(&children), root.path().to_path_buf());
        let starter = Starter {
            endpoint,
            spawn: Arc::new(move || {
                let child = Command::new(&binary)
                    .arg("orchestrator")
                    .env("XDG_RUNTIME_DIR", home.join("run"))
                    .env("XDG_DATA_HOME", home.join("data"))
                    .env("HOME", &home)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| {
                        std::io::Error::new(
                            e.kind(),
                            format!("cannot start {}: {e}", binary.display()),
                        )
                    })?;
                spawned.lock().unwrap().push(child);
                Ok(())
            }),
        };
        Self {
            _root: root,
            children,
            starter,
        }
    }

    fn socket(&self) -> &Path {
        &self.starter.endpoint.socket_path
    }

    /// How many of the started orchestrators still run.
    fn running(&self) -> usize {
        self.children
            .lock()
            .unwrap()
            .iter_mut()
            .map(|c| matches!(c.try_wait(), Ok(None)))
            .filter(|running| *running)
            .count()
    }

    fn started(&self) -> usize {
        self.children.lock().unwrap().len()
    }

    /// Wait until exactly `n` orchestrators run.
    async fn settles_on(&self, n: usize) {
        let deadline = Instant::now() + WITHIN;
        while self.running() != n {
            assert!(
                Instant::now() < deadline,
                "{} orchestrators run, not {n}",
                self.running()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn kill_all(&self) {
        for child in self.children.lock().unwrap().iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.kill_all();
    }
}

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

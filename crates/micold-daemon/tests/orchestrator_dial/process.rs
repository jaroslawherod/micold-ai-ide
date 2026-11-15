//! The wiring `server::run` does (#1725, T014), visible only from outside: the real
//! `micold-daemon --config` process, a real orchestrator in this process.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::connect::{connect, Connected};
use micold_core::endpoint::Endpoint;

use crate::support::{record_when, Orchestrator, WITHIN};
use crate::FakeOrchestrator;

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Kills the daemon even when an assertion fails.
struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A host daemon process on `dir` with `body` appended to a minimal `daemon.toml`, and the
/// endpoint it serves clients on.
fn start(dir: &Path, body: &str) -> (Daemon, Endpoint) {
    let file = dir.join("daemon.toml");
    let text = format!(
        "schema = 1\n[daemon]\nname = \"Process Host\"\nid = \"proc\"\nkind = \"host\"\n\
         data_dir = '{}'\n{body}",
        dir.join("data").display()
    );
    std::fs::write(&file, text).unwrap();
    std::fs::create_dir_all(dir.join("run")).unwrap();
    let child = Command::new(DAEMON_BIN)
        .arg("--config")
        .arg(&file)
        .env("HOME", dir)
        .env("XDG_DATA_HOME", dir.join("xdg-data"))
        .env("XDG_RUNTIME_DIR", dir.join("run"))
        .env("MICOLD_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let run = dir.join("run").join("micold");
    let endpoint = Endpoint {
        socket_path: run.join("daemon.sock"),
        lock_path: run.join("daemon.lock"),
    };
    (Daemon(child), endpoint)
}

/// Wait until the daemon accepts a client: it is serving.
async fn serving(endpoint: &Endpoint) {
    let started = Instant::now();
    loop {
        if let Ok(Some(Connected::Ready(..))) = connect(endpoint, "dial-test").await {
            return;
        }
        assert!(started.elapsed() < WITHIN, "the daemon never served");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_process_with_the_link_enabled_is_listed_online() {
    let orchestrator = Orchestrator::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (_daemon, endpoint) = start(
        dir.path(),
        &format!(
            "[orchestrator_link]\nenabled = true\nsocket = '{}'\n",
            orchestrator.socket.display()
        ),
    );

    let record = record_when(&orchestrator, "proc", |r| r["health"] == "online").await;
    assert_eq!(record["name"], "Process Host");
    serving(&endpoint).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_process_with_no_link_configured_never_dials() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    // The table is absent; `[orchestrator_link]` with `enabled = false` is the same.
    let (_daemon, endpoint) = start(
        dir.path(),
        &format!(
            "[orchestrator_link]\nenabled = false\nsocket = '{}'\n",
            socket.display()
        ),
    );
    serving(&endpoint).await;
    let dialled = tokio::time::timeout(Duration::from_secs(2), fake.accept()).await;
    assert!(dialled.is_err(), "a daemon with no link configured dialled");
}

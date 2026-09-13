//! The daemon's pid record and the version-agnostic stop built on it (feature 030, FR-023).
//!
//! A client whose version does not match the daemon cannot handshake, so "Restart service" cannot
//! send it a control message. What it can do is read the pid the daemon recorded at `lock_path` and
//! stop that process. These cases run the real `micold-daemon` binary on every platform, because
//! the record is exactly the part the Windows endpoint stub never wrote (contract E4).
//!
//! On Unix the endpoint is isolated into a tempdir through `XDG_RUNTIME_DIR` (Linux) and `HOME`
//! (macOS). The Windows endpoint takes no environment input by design, so there these cases use the
//! signed-in user's real endpoint; they are meant for the CI leg, not a desktop with the app open.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::endpoint::Endpoint;

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// How long a cold daemon gets to bind and write its pid record.
const STARTUP_BUDGET: Duration = Duration::from_secs(20);

/// How long a stopped daemon's endpoint may keep accepting (contract E4.2).
const STOP_BUDGET: Duration = Duration::from_secs(5);

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// The cases share one endpoint (on Windows there is only one) and the process environment the
/// resolver reads, so they run one at a time.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A daemon this test started. Dropping it kills that process and no other.
struct SpawnedDaemon {
    child: Child,
    endpoint: Endpoint,
    _home: tempfile::TempDir,
}

impl Drop for SpawnedDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start the real daemon against an endpoint private to this test (on Unix).
fn spawn_daemon() -> SpawnedDaemon {
    let home = tempfile::tempdir().expect("temp home");
    point_the_resolver_at(home.path());
    let endpoint = micold_core::endpoint::resolve().expect("resolve the test endpoint");

    let mut command = Command::new(DAEMON_BIN);
    command
        .env("MICOLD_LOG", "warn")
        .env("HOME", home.path())
        .env("XDG_RUNTIME_DIR", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_CACHE_HOME", home.path().join("cache"))
        // Neither the container placement nor socket activation: the endpoint path under test.
        .env_remove(micold_daemon::server::LISTEN_ADDR_ENV)
        .env_remove("LISTEN_FDS")
        .env_remove("LISTEN_PID")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command.spawn().expect("spawn micold-daemon");
    SpawnedDaemon {
        child,
        endpoint,
        _home: home,
    }
}

/// Make this process resolve the same endpoint the spawned daemon will.
fn point_the_resolver_at(home: &Path) {
    std::env::set_var("XDG_RUNTIME_DIR", home);
    std::env::set_var("HOME", home);
}

/// Wait until `lock_path` holds something, and return it.
async fn recorded_pid_text(daemon: &SpawnedDaemon) -> String {
    let deadline = Instant::now() + STARTUP_BUDGET;
    loop {
        if let Ok(text) = std::fs::read_to_string(&daemon.endpoint.lock_path) {
            if !text.is_empty() {
                return text;
            }
        }
        assert!(
            Instant::now() < deadline,
            "the daemon wrote no pid to {} within {STARTUP_BUDGET:?}",
            daemon.endpoint.lock_path.display()
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[tokio::test]
async fn pid_record_lifecycle() {
    let _serial = SERIAL.lock().await;
    let daemon = spawn_daemon();

    let recorded = recorded_pid_text(&daemon).await;

    assert_eq!(
        recorded,
        format!("{}\n", daemon.child.id()),
        "lock_path must hold the running daemon's pid followed by a newline"
    );
}

#[tokio::test]
async fn stop_running_daemon_ends_endpoint() {
    let _serial = SERIAL.lock().await;
    let daemon = spawn_daemon();
    recorded_pid_text(&daemon).await;

    let stopped = micold_core::spawn::stop_running_daemon(&daemon.endpoint);

    assert!(
        matches!(stopped, Ok(true)),
        "a live daemon with a pid record must be stopped; got {stopped:?}"
    );
    let deadline = Instant::now() + STOP_BUDGET;
    while endpoint_accepts(&daemon.endpoint).await {
        assert!(
            Instant::now() < deadline,
            "the endpoint still accepts connections {STOP_BUDGET:?} after the stop"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// `true` iff a client can reach the daemon at `endpoint` right now.
async fn endpoint_accepts(endpoint: &Endpoint) -> bool {
    matches!(
        micold_core::connect::connect(endpoint, "daemon-stop-test").await,
        Ok(Some(_))
    )
}

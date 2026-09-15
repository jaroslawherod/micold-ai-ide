//! A daemon that cannot start says why in its log file (feature 030, FR-005, E6.4).
//!
//! The installed daemon is a GUI-subsystem program: it has no console, so the `fatal:` line `main`
//! prints to stderr goes nowhere. The log file is the one place a user (or a bug report) can find
//! the reason, so the fatal error has to reach it too. The case runs the real binary detached, the
//! way the client spawns it, which is what selects the file sink.
//!
//! On Linux and macOS the log path is isolated through `XDG_DATA_HOME` and `HOME`. The Windows data
//! dir takes no environment input, so there the case writes to the signed-in user's real daemon log;
//! it is meant for the CI leg.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Not a socket address, so the sandbox listener cannot bind and startup fails.
const UNBINDABLE_ADDR: &str = "not-an-addr";

/// How long a failing daemon gets to exit.
const EXIT_BUDGET: Duration = Duration::from_secs(20);

const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[test]
fn fatal_startup_error_reaches_the_log_file() {
    let home = tempfile::tempdir().expect("temp home");
    let data = home.path().join("data");
    // Resolve the log path the way the daemon will, from the same environment.
    std::env::set_var("HOME", home.path());
    std::env::set_var("XDG_DATA_HOME", &data);
    let log_path = micold_daemon::logging::default_log_path().expect("a data dir for the log");

    let mut child = Command::new(DAEMON_BIN)
        .env("MICOLD_LOG", "info")
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", &data)
        .env("XDG_RUNTIME_DIR", home.path().join("run"))
        .env(micold_daemon::server::LISTEN_ADDR_ENV, UNBINDABLE_ADDR)
        // Neither journald nor a sandbox token: the detached file-sink placement.
        .env_remove("JOURNAL_STREAM")
        .env_remove(micold_core::protocol::auth::TOKEN_PATH_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn micold-daemon");

    let deadline = Instant::now() + EXIT_BUDGET;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll the daemon") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("a daemon that cannot bind {UNBINDABLE_ADDR:?} was still running after {EXIT_BUDGET:?}");
        }
        std::thread::sleep(POLL_INTERVAL);
    };

    assert!(
        !status.success(),
        "a failed startup must exit non-zero, got {status:?}"
    );
    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        log.contains("fatal:"),
        "the log at {} must record the fatal startup error; it holds:\n{log}",
        log_path.display()
    );
}

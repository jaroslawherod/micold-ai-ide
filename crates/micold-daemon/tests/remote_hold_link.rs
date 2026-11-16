//! `micold-daemon remote hold-link` (#1726, FR-007): the remote command of the link holder, the
//! `ssh` that keeps the orchestrator link's `-R` up once the bridge is closed. It prints `ready`
//! (ssh runs it only after the forward is bound), touches no daemon and ends on stdin EOF.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

#[test]
fn hold_link_prints_ready_touches_no_daemon_and_exits_on_stdin_eof() {
    let home = tempfile::tempdir().unwrap();
    let runtime = tempfile::tempdir().unwrap();
    let mut child = Command::new(DAEMON_BIN)
        .args(["remote", "hold-link"])
        .env("HOME", home.path())
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert_eq!(line.trim(), "ready");

    // Still holding while stdin is open.
    std::thread::sleep(Duration::from_millis(200));
    assert!(child.try_wait().unwrap().is_none(), "holds until stdin EOF");

    drop(child.stdin.take());
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "ends on EOF");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "{status:?}");

    // A bridge would have resolved the daemon's endpoint and started a daemon writing its data
    // directory; the holder opens no daemon connection and so leaves both untouched.
    for dir in [home.path(), runtime.path()] {
        let entries: Vec<_> = std::fs::read_dir(dir).unwrap().collect();
        assert!(entries.is_empty(), "{dir:?} touched: {entries:?}");
    }
}

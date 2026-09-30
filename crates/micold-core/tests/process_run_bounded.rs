//! `process::run_bounded` (feature 034, research R6): the bounded runner shared by
//! environment-include and the GitHub CLI.
//!
//! The child is this test binary itself, re-run with `CHILD_MODE` set so that only
//! [`child_helper`] acts. That keeps every case identical on Linux, macOS and Windows without a
//! shell (a cold `powershell.exe` is what made `env_include_resolve.rs` flaky on Windows runners).
//! libtest prints a short header of its own to the child's stdout, so assertions count the bytes
//! the child wrote rather than comparing the whole stream.

use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

use micold_core::process::{run_bounded, RunOutcome};

/// Selects what [`child_helper`] does when this binary runs as the child.
const CHILD_MODE: &str = "MICOLD_RUN_BOUNDED_CHILD";
/// Where a beating child writes its heartbeat for as long as it lives.
const CHILD_MARKER: &str = "MICOLD_RUN_BOUNDED_MARKER";
/// How often a beating child writes its heartbeat.
const BEAT: Duration = Duration::from_millis(50);
/// How long a heartbeat must stay unchanged before the child counts as dead: ten beats.
const QUIET: Duration = Duration::from_millis(500);

/// 1 MiB: well over every OS's pipe buffer (64 KiB on Linux, less on macOS and Windows).
const LARGE_STDOUT: usize = 1024 * 1024;
/// 64 KiB on stderr at the same time, so both pipes must be drained, not just one.
const LARGE_STDERR: usize = 64 * 1024;

/// Not a test of its own: the body of the child process. Does nothing unless `CHILD_MODE` is set.
#[test]
fn child_helper() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        return;
    };
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    match mode.as_str() {
        "beat" => {
            // Outlives any bound these tests set, writing a new count every beat while alive.
            let marker = std::env::var(CHILD_MARKER).expect("marker path");
            for beat in 0..400u32 {
                std::fs::write(&marker, beat.to_string()).expect("write heartbeat");
                std::thread::sleep(BEAT);
            }
            std::process::exit(0);
        }
        "brief" => {
            std::thread::sleep(Duration::from_millis(300));
            std::process::exit(0);
        }
        "large" => {
            out.write_all(&vec![b'x'; LARGE_STDOUT]).unwrap();
            out.flush().unwrap();
            err.write_all(&vec![b'y'; LARGE_STDERR]).unwrap();
            err.flush().unwrap();
            std::process::exit(0);
        }
        "fail" => {
            out.write_all(b"partial-output").unwrap();
            out.flush().unwrap();
            err.write_all(b"something went wrong").unwrap();
            err.flush().unwrap();
            std::process::exit(3);
        }
        other => panic!("unknown child mode {other}"),
    }
}

fn child(mode: &str) -> Command {
    let mut cmd = Command::new(std::env::current_exe().expect("test binary path"));
    cmd.args(["child_helper", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_MODE, mode);
    cmd
}

fn count(bytes: &[u8], of: u8) -> usize {
    bytes.iter().filter(|b| **b == of).count()
}

#[test]
fn a_child_past_the_bound_is_killed_and_reported() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let bound = Duration::from_millis(500);
    let mut cmd = child("beat");
    cmd.env(CHILD_MARKER, &marker);

    let started = Instant::now();
    let outcome = run_bounded(cmd, bound);
    let took = started.elapsed();

    assert!(
        matches!(outcome, RunOutcome::TimedOut { .. }),
        "a child still running at the bound is reported as timed out, got {outcome:?}"
    );
    assert!(
        took < bound + Duration::from_secs(1),
        "the runner returns within bound + 1 s (FR-007), took {took:?}"
    );
    assert!(
        marker.exists(),
        "precondition: the child was beating before the bound"
    );
    // Dead means the heartbeat stays unchanged for `QUIET`; a live child changes it every `BEAT`.
    // Polled against a deadline, so a dead child passes in about `QUIET` and a live one fails.
    let heartbeat = || std::fs::read_to_string(&marker).unwrap_or_default();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last = heartbeat();
    let mut unchanged_since = Instant::now();
    while unchanged_since.elapsed() < QUIET {
        assert!(
            Instant::now() < deadline,
            "a timed-out child is killed, not left running: its heartbeat is still at {last:?}"
        );
        std::thread::sleep(BEAT);
        let now = heartbeat();
        if now != last {
            last = now;
            unchanged_since = Instant::now();
        }
    }
}

#[test]
fn a_child_inside_the_bound_exits_normally() {
    let outcome = run_bounded(child("brief"), Duration::from_secs(5));
    assert!(
        matches!(outcome, RunOutcome::Exited { code: 0, .. }),
        "a child that finishes inside the bound is reported as exited, got {outcome:?}"
    );
}

#[test]
fn large_output_is_drained_while_waiting() {
    let started = Instant::now();
    let outcome = run_bounded(child("large"), Duration::from_secs(5));
    let took = started.elapsed();

    match outcome {
        RunOutcome::Exited {
            code,
            stdout,
            stderr,
        } => {
            assert_eq!(code, 0, "the writer exits cleanly");
            assert_eq!(
                count(&stdout, b'x'),
                LARGE_STDOUT,
                "every stdout byte arrives: the pipe is read while the child runs"
            );
            assert_eq!(
                count(stderr.as_bytes(), b'y'),
                LARGE_STDERR,
                "every stderr byte arrives too"
            );
        }
        other => panic!(
            "output larger than a pipe buffer must not stall the child into a timeout, got \
             {other:?} after {took:?}"
        ),
    }
}

#[test]
fn a_failing_child_reports_status_and_output() {
    let outcome = run_bounded(child("fail"), Duration::from_secs(5));
    match outcome {
        RunOutcome::Exited {
            code,
            stdout,
            stderr,
        } => {
            assert_eq!(code, 3, "the child's own exit status is reported");
            assert!(
                String::from_utf8_lossy(&stdout).contains("partial-output"),
                "stdout of a failing child is kept"
            );
            assert!(
                stderr.contains("something went wrong"),
                "stderr of a failing child is kept"
            );
        }
        other => panic!("a non-zero exit is still an exit, got {other:?}"),
    }
}

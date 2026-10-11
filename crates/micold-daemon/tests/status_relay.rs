//! The status-line relay, `micold-daemon status-line <relay-file>` (feature 490, contract S2;
//! FR-012, FR-014, FR-017, FR-020): it posts only `rate_limits` to the daemon and keeps the user's
//! own status line, silently and without delaying it.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use micold_core::plan_usage::user_status_line::RelayConfig;
use micold_daemon::status_relay::{relay, user_shell, MAX_INPUT, RELAY_ENV};
use serde_json::json;

const TOKEN: &str = "tok-490";

/// One request as the fake receiver saw it.
struct Seen {
    head: String,
    body: Vec<u8>,
}

/// A loopback listener that records one request and answers 200, or never answers.
struct Receiver {
    port: u16,
    seen: mpsc::Receiver<Seen>,
}

fn receiver(answer: bool) -> Receiver {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, seen) = mpsc::channel();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        if !answer {
            // Hold the connection open and say nothing.
            std::thread::sleep(Duration::from_secs(10));
            return;
        }
        let _ = tx.send(read_request(&mut stream));
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    });
    Receiver { port, seen }
}

fn read_request(stream: &mut TcpStream) -> Seen {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        let n = stream.read(&mut chunk).unwrap();
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(n > 0, "the request ended before its head");
    };
    let head = String::from_utf8(buf[..end].to_vec()).unwrap();
    let length: usize = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = buf[end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0, "the request ended before its body");
        body.extend_from_slice(&chunk[..n]);
    }
    Seen { head, body }
}

fn relay_file(dir: &Path, port: u16, user_command: Option<&str>) -> PathBuf {
    let path = dir.join("session.status.json");
    let config = RelayConfig {
        url: format!("http://127.0.0.1:{port}/status/0b6e4a62-6d0b-4a8e-9a43-5d2c9f0b1a77"),
        token: TOKEN.to_owned(),
        user_command: user_command.map(str::to_owned),
    };
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    path
}

fn input() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "session_id": "s",
        "transcript_path": "/home/me/.claude/projects/x.jsonl",
        "cwd": "/home/me/project",
        "rate_limits": {"five_hour": {"used_percentage": 42.0, "resets_at": 1_791_644_400}}
    }))
    .unwrap()
}

/// Run the relay with `stdin`, returning (exit status, stdout).
fn run(path: &Path, stdin: &[u8], nested: bool) -> (i32, Vec<u8>) {
    let mut out = Vec::new();
    let status = relay(path, &mut &stdin[..], &mut out, nested, user_shell);
    (status, out)
}

#[test]
fn the_body_is_exactly_the_rate_limits_object_with_the_bearer_token() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let (status, out) = run(&relay_file(dir.path(), rx.port, None), &input(), false);
    assert_eq!(status, 0);
    assert!(out.is_empty());
    let seen = rx.seen.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(seen.head.starts_with("POST /status/0b6e4a62-6d0b-4a8e-9a43-5d2c9f0b1a77 HTTP/1.1\r\n"));
    assert!(
        seen.head
            .lines()
            .any(|l| l.eq_ignore_ascii_case(&format!("authorization: Bearer {TOKEN}"))),
        "{}",
        seen.head
    );
    let body: serde_json::Value = serde_json::from_slice(&seen.body).unwrap();
    assert_eq!(
        body,
        json!({"five_hour": {"used_percentage": 42.0, "resets_at": 1_791_644_400}})
    );
}

#[test]
fn an_input_without_rate_limits_posts_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let (status, out) = run(&relay_file(dir.path(), rx.port, None), br#"{"cwd": "/x"}"#, false);
    assert_eq!((status, out.is_empty()), (0, true));
    assert!(rx.seen.recv_timeout(Duration::from_millis(700)).is_err());
}

#[cfg(unix)]
#[test]
fn the_user_command_gets_the_whole_input_and_its_output_is_copied_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let command = r#"printf '%s|' "$MICOLD_STATUS_RELAY"; cat; printf '\nsecond line\n'"#;
    let (status, out) = run(&relay_file(dir.path(), rx.port, Some(command)), &input(), false);
    assert_eq!(status, 0);
    let mut expected = b"1|".to_vec();
    expected.extend_from_slice(&input());
    expected.extend_from_slice(b"\nsecond line\n");
    assert_eq!(out, expected);
}

#[cfg(unix)]
#[test]
fn the_exit_status_is_the_user_commands() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let (status, out) = run(
        &relay_file(dir.path(), rx.port, Some("echo out; exit 3")),
        &input(),
        false,
    );
    assert_eq!((status, out), (3, b"out\n".to_vec()));
}

#[cfg(unix)]
#[test]
fn a_receiver_that_never_answers_does_not_hold_the_users_line() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(false);
    let started = Instant::now();
    let (status, out) = run(
        &relay_file(dir.path(), rx.port, Some("echo line")),
        &input(),
        false,
    );
    assert_eq!((status, out), (0, b"line\n".to_vec()));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "the POST gives up at 500 ms: {:?}",
        started.elapsed()
    );
}

#[test]
fn a_refused_post_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    // Bind and drop a listener, so the port refuses connections.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (status, out) = run(&relay_file(dir.path(), port, None), &input(), false);
    assert_eq!((status, out.is_empty()), (0, true));
}

#[test]
fn a_missing_or_malformed_relay_file_runs_nothing_and_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    let (status, out) = run(&dir.path().join("absent.json"), &input(), false);
    assert_eq!((status, out.is_empty()), (0, true));
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();
    let (status, out) = run(&bad, &input(), false);
    assert_eq!((status, out.is_empty()), (0, true));
}

#[cfg(unix)]
#[test]
fn a_failing_user_command_is_not_reported_by_the_relay() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let (status, out) = run(
        &relay_file(dir.path(), rx.port, Some("/nonexistent/line.sh 2>/dev/null")),
        &input(),
        false,
    );
    assert_eq!(status, 127);
    assert!(out.is_empty());
}

#[cfg(unix)]
#[test]
fn input_over_1_mib_posts_nothing_but_still_runs_the_user_command() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let mut big = input();
    big.extend(std::iter::repeat_n(b' ', MAX_INPUT));
    let (status, out) = run(
        &relay_file(dir.path(), rx.port, Some("wc -c | tr -d ' '")),
        &big,
        false,
    );
    assert_eq!(status, 0);
    assert_eq!(String::from_utf8(out).unwrap().trim(), big.len().to_string());
    assert!(rx.seen.recv_timeout(Duration::from_millis(700)).is_err());
}

#[cfg(unix)]
#[test]
fn a_nested_relay_posts_nothing_and_runs_no_user_command() {
    let dir = tempfile::tempdir().unwrap();
    let rx = receiver(true);
    let (status, out) = run(
        &relay_file(dir.path(), rx.port, Some("echo recursed")),
        &input(),
        true,
    );
    assert_eq!((status, out.is_empty()), (0, true));
    assert!(rx.seen.recv_timeout(Duration::from_millis(700)).is_err());
}

#[test]
fn the_binary_with_a_missing_relay_file_is_silent_and_starts_no_daemon() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
        .arg("status-line")
        .arg(dir.path().join("absent.json"))
        .env_remove(RELAY_ENV)
        // A daemon that did start would put its endpoint and log under these.
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path())
        .env("XDG_STATE_HOME", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input()).unwrap();
    let started = Instant::now();
    let output = child.wait_with_output().unwrap();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "{:?}", output.stdout);
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(left.is_empty(), "the relay created {left:?}");
}

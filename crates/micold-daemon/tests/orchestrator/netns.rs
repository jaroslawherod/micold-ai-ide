//! FR-008, runtime half: with outbound connections blocked, the orchestrator still links and keeps a
//! heartbeat, and it makes no `connect(2)` call except to its own socket.
//!
//! Two things are proven, because the first alone is not enough: a network namespace blocks IP
//! sockets only, so a dial to a filesystem socket, which is what the tests and a local daemon use,
//! would still succeed and could pass unseen.
//!
//! 1. The `link_and_list` and `heartbeat` tests pass inside `unshare -rn` (a new user and network
//!    namespace with no interface up), so nothing they need goes over IP.
//! 2. That run is traced with `strace -f -e trace=connect`: every `connect` is to
//!    `orchestrator.sock` (the fake daemons dialling in, and the orchestrator's own start-up probe of
//!    that socket) and to nothing else, so the orchestrator never dialled a daemon at any address.
//!
//! Scope: Linux only (a namespace and `strace` have no counterpart on macOS or Windows; the source
//! scan in `no_outbound.rs` covers every platform). Each part skips, saying why, where the tool or
//! unprivileged user namespaces are unavailable (a locked-down container or CI runner).

#![cfg(target_os = "linux")]

use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Output};
use std::time::Duration;

use crate::support::children_quiet_blocking;

/// Set in the re-executed binary so it runs the tests instead of spawning itself again.
const INSIDE: &str = "MICOLD_ORCHESTRATOR_NETNS";
/// A loopback port the outer test listens on; inside the namespace it must be unreachable.
const LOOPBACK_PORT: &str = "MICOLD_ORCHESTRATOR_NETNS_PORT";

/// One test from each module, which must be reported `ok` in the re-executed run: renaming a module
/// would make its filter match nothing and the run would pass empty.
const MUST_RUN: [&str; 3] = [
    "link_and_list::a_linked_daemon_is_listed_online_with_every_field ... ok",
    "heartbeat::a_silent_daemon_lists_offline_and_its_next_frame_brings_it_back ... ok",
    "netns::outbound_connections_are_blocked_in_the_namespace ... ok",
];

fn can_run(tool: &[&str]) -> bool {
    Command::new(tool[0])
        .args(&tool[1..])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// This binary re-run under `unshare -rn`, optionally behind `prefix`, on the three tests.
fn rerun(prefix: &[&str], port: u16) -> Output {
    let exe = std::env::current_exe().unwrap();
    let mut command = Command::new("unshare");
    command.arg("-rn").args(prefix).arg(exe).args([
        "link_and_list::",
        "heartbeat::",
        "netns::outbound_connections_are_blocked_in_the_namespace",
        "--test-threads=1",
    ]);
    command
        .env(INSIDE, "1")
        .env(LOOPBACK_PORT, port.to_string())
        .output()
        .expect("run the tests under unshare")
}

fn assert_ran(output: &Output) {
    let text = text(output);
    assert!(
        output.status.success(),
        "tests failed with outbound blocked:\n{text}"
    );
    for name in MUST_RUN {
        assert!(text.contains(name), "`{name}` did not run:\n{text}");
    }
}

#[test]
fn link_and_heartbeat_pass_with_outbound_connections_blocked() {
    if std::env::var_os(INSIDE).is_some() {
        return; // the re-executed run: the tests are run directly
    }
    // The child-count tests need this process to have no children, so they and this test take turns.
    let _quiet = children_quiet_blocking();
    if !can_run(&["unshare", "-rn", "true"]) {
        eprintln!(
            "skipped: `unshare -rn` is unavailable here (no unprivileged user namespaces), so \
             outbound connections cannot be blocked. FR-008 stays covered by no_outbound.rs."
        );
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    assert_ran(&rerun(&[], port));
}

#[test]
fn the_orchestrator_connects_to_nothing_but_its_own_socket() {
    if std::env::var_os(INSIDE).is_some() {
        return;
    }
    let _quiet = children_quiet_blocking();
    if !can_run(&[
        "unshare",
        "-rn",
        "strace",
        "-qq",
        "-e",
        "trace=connect",
        "true",
    ]) {
        eprintln!(
            "skipped: `strace` under `unshare -rn` is unavailable here, so the connect calls \
             cannot be traced. The pass-in-a-namespace test and no_outbound.rs still run."
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let trace = dir.path().join("connect.trace");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let trace_arg = trace.to_str().unwrap().to_owned();
    let output = rerun(
        &[
            "strace",
            "-f",
            "-qq",
            "-e",
            "trace=connect",
            "-o",
            &trace_arg,
        ],
        port,
    );
    assert_ran(&output);

    let traced = std::fs::read_to_string(&trace).unwrap();
    let connects: Vec<&str> = traced.lines().filter(|l| l.contains("connect(")).collect();
    assert!(
        connects.iter().any(|l| l.contains("orchestrator.sock")),
        "the trace saw no link at all, so it proves nothing:\n{traced}"
    );
    // The namespace probe below is this test binary's own deliberate TCP connect: not the orchestrator.
    let strangers: Vec<_> = connects
        .iter()
        .filter(|l| !l.contains("orchestrator.sock") && !l.contains("sin_port=htons("))
        .collect();
    let tcp: Vec<_> = connects
        .iter()
        .filter(|l| l.contains("sin_port=htons("))
        .collect();
    assert!(
        strangers.is_empty(),
        "connect to a stranger:\n{strangers:#?}"
    );
    // Only the two probe connects (loopback and TEST-NET-1) are IP; the orchestrator made none.
    assert!(tcp.len() <= 2, "IP connects beyond the probe's:\n{tcp:#?}");
}

/// Run only inside the namespace: proves the block is real, so the passing tests mean something.
/// Outside it, `192.0.2.1` is merely unreachable and loopback connects, so both would differ.
#[test]
fn outbound_connections_are_blocked_in_the_namespace() {
    if std::env::var_os(INSIDE).is_none() {
        return; // outside the namespace there is nothing to prove
    }
    let timeout = Duration::from_secs(2);
    let attempt = TcpStream::connect_timeout(&"192.0.2.1:9".parse().unwrap(), timeout);
    assert_eq!(
        attempt.unwrap_err().kind(),
        ErrorKind::NetworkUnreachable,
        "no interface is up inside the namespace"
    );
    // A listener the outer process holds on loopback, which a connect outside the namespace reaches.
    let port: u16 = std::env::var(LOOPBACK_PORT).unwrap().parse().unwrap();
    let loopback = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), timeout);
    assert!(
        loopback.is_err(),
        "a loopback listener outside the namespace was reachable from inside it"
    );
}

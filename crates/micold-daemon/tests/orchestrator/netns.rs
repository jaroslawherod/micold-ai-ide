//! FR-008, runtime half: with outbound connections blocked, the orchestrator still links and keeps a
//! heartbeat, because daemons dial it over a local socket and it dials nothing.
//!
//! Scope: the proof is Linux only (a network namespace has no counterpart on macOS or Windows; the
//! source scan in `no_outbound.rs` covers every platform). The test re-executes this binary under
//! `unshare -rn`: a new user and network namespace with no interfaces up, so any TCP or UDP connect
//! fails at once, while a filesystem socket, which these tests use, still works. It skips, saying why,
//! where unprivileged user namespaces are unavailable (a locked-down container or CI runner).

#![cfg(target_os = "linux")]

use std::process::Command;

use crate::support::children_quiet_blocking;

/// Set in the re-executed binary so it runs the tests instead of spawning itself again.
const INSIDE: &str = "MICOLD_ORCHESTRATOR_NETNS";

#[test]
fn link_and_heartbeat_pass_with_outbound_connections_blocked() {
    if std::env::var_os(INSIDE).is_some() {
        return; // the re-executed run: the tests below are run directly
    }
    // The child-count tests need this process to have no children, so they and this test take turns.
    let _quiet = children_quiet_blocking();
    if !Command::new("unshare")
        .args(["-rn", "true"])
        .status()
        .is_ok_and(|s| s.success())
    {
        eprintln!(
            "skipped: `unshare -rn` is unavailable here (no unprivileged user namespaces), so \
             outbound connections cannot be blocked. FR-008 stays covered by no_outbound.rs."
        );
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let output = Command::new("unshare")
        .args(["-rn"])
        .arg(exe)
        .args([
            "link_and_list::",
            "heartbeat::",
            "netns::outbound_connections_are_blocked_in_the_namespace",
            "--test-threads=1",
        ])
        .env(INSIDE, "1")
        .output()
        .expect("run the tests under unshare");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "tests failed with outbound blocked:\n{text}"
    );
    assert!(
        text.contains("outbound_connections_are_blocked_in_the_namespace ... ok"),
        "the namespace probe did not run:\n{text}"
    );
}

/// Run only inside the namespace: proves the block is real, so the passing tests mean something.
#[test]
fn outbound_connections_are_blocked_in_the_namespace() {
    if std::env::var_os(INSIDE).is_none() {
        return; // outside the namespace there is nothing to prove
    }
    let attempt = std::net::TcpStream::connect_timeout(
        &"192.0.2.1:9".parse().unwrap(),
        std::time::Duration::from_secs(2),
    );
    assert!(
        attempt.is_err(),
        "an outbound TCP connect succeeded inside the namespace"
    );
}

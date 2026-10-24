//! §B.5–B.6 of the quickstart — the idle rule, measured from *outside* the container (feature 028,
//! US4, FR-019/FR-020/FR-022, lifecycle contract §5.17/§5.18), as feature 806 reshaped it.
//!
//! `crates/micold-daemon/tests/idle_stop.rs` already proves the rule itself: nobody connected for
//! the window, the daemon writes its reason, hands the sessions back resumable and exits. None of
//! that is what this file is for.
//!
//! What changed with 806: the container's process is `sshd`, not the daemon, so a daemon that
//! stops itself no longer ends the container — it ends the *daemon*, and the next connection
//! starts it again through the bridge. §5.17 is therefore measured as that: the daemon process is
//! gone, the container is still up and was not restarted by the runtime, and a client that
//! connects afterwards gets a daemon. The idle window reaches the daemon the entrypoint starts
//! (`~/.micold/remote/last`, run with the container's environment), so the probes restart the
//! container once provisioned, which is also what a runtime restart under the survival opt-in
//! does.
//!
//! §5.18 is the single approved exception: with the opt-in the container is created with the idle
//! rule switched off, and the daemon the entrypoint starts must still be there long after the
//! window that would have stopped the other probe's.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::{Duration, Instant};

use sandbox_real_support::{
    cli_out, daemon_running, provision_then_restart, seed, start_sandbox, wait_for_accept,
    SandboxSpec,
};

const IDLE_CONTAINER: &str = "micold-idle-probe";
const IDLE_NETWORK: &str = "micold-idle-probe-net";
const IDLE_PORT: u16 = 17741;

const KEPT_CONTAINER: &str = "micold-idle-kept-probe";
const KEPT_NETWORK: &str = "micold-idle-kept-probe-net";
const KEPT_PORT: u16 = 17742;

/// The window these probes run under.
///
/// Long enough that the entrypoint's daemon comes up comfortably before it expires, short enough
/// that the suite stays a suite.
const WINDOW: &str = "5s";

/// `{{.State.Status}}` and `{{.RestartCount}}` as one reading, because the two failures this file
/// separates are `running`/0 (nothing restarted it) and `running`/1+ (the runtime did).
fn status_of(container: &str) -> (String, String) {
    let raw = cli_out(&[
        "inspect",
        "-f",
        "{{.State.Status}} {{.RestartCount}}",
        container,
    ]);
    let mut parts = raw.split_whitespace();
    (
        parts.next().unwrap_or_default().to_string(),
        parts.next().unwrap_or_default().to_string(),
    )
}

/// T047, FR-019/FR-020, lifecycle contract §5.17.
///
/// The daemon ends when nobody is connected for the window, and stays ended: the container is up
/// and the runtime has not restarted anything. `RestartCount` is half the assertion rather than a
/// detail — a sandbox created with a restart policy would otherwise hide a container that died and
/// came back. A client that arrives afterwards gets a daemon again.
#[tokio::test]
async fn sandbox_real_idle_daemon_stops_and_the_next_connection_starts_it() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "idle");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // Through `extra` rather than through `create`: the argv builder is pure by obligation C-1, so
    // it has no business reading this process's environment, and a test's own timing belongs in
    // the test's own arguments.
    let window = format!("{}={WINDOW}", micold_core::spawn::IDLE_STOP_ENV);
    let _sandbox = start_sandbox(&SandboxSpec {
        container: IDLE_CONTAINER,
        network: IDLE_NETWORK,
        port: IDLE_PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &["-e".to_string(), window],
    });
    provision_then_restart(IDLE_CONTAINER, IDLE_PORT).await;

    // Generous against the window, because what is being measured is *that* it stops, not when:
    // the rule ticks at a fraction of the window, and the ordered shutdown persists every session
    // before anything is killed.
    let deadline = Instant::now() + Duration::from_secs(60);
    while daemon_running(IDLE_CONTAINER) {
        assert!(
            Instant::now() < deadline,
            "the daemon was still running 60s after a {WINDOW} idle window"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let (status, restarts) = status_of(IDLE_CONTAINER);
    assert_eq!(
        (status.as_str(), restarts.as_str()),
        ("running", "0"),
        "the container stays up (sshd is its process) and the runtime restarted nothing: its \
         restart count covers restart-policy restarts, not the `docker restart` this test issued"
    );

    // Not stuck: the next client gets a daemon.
    let (_conn, _catalog) = wait_for_accept(IDLE_PORT).await;
}

/// T048, FR-022, lifecycle contract §5.18 — the one approved difference between the placements.
///
/// The opt-in is the user saying, in so many words, keep this running. A service asked to keep
/// running is not one to stop for being unused, so the sandbox is created with the idle rule
/// switched off and there is nothing for the window to expire.
#[tokio::test]
async fn sandbox_real_idle_is_off_for_the_keep_running_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "kept");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // No window through `extra`: `argv::idle_stop_value` puts `off` in the argv for this spec, and
    // the harness reads it from there. Passing a window here as well would test which of the two
    // `-e` flags the runtime happens to prefer, which is not a promise this product makes.
    let _sandbox = start_sandbox(&SandboxSpec {
        container: KEPT_CONTAINER,
        network: KEPT_NETWORK,
        port: KEPT_PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: true,
        extra: &[],
    });

    // The opt-in reached the runtime as an environment entry, not merely as an argv the test built.
    let env = cli_out(&["inspect", "-f", "{{.Config.Env}}", KEPT_CONTAINER]);
    let expected = format!(
        "{}={}",
        micold_core::spawn::IDLE_STOP_ENV,
        micold_core::spawn::IDLE_STOP_OFF
    );
    assert!(
        env.contains(&expected),
        "the keep-it-running opt-in must reach the container as {expected}; a container created \
         without it obeys the idle rule and stops half an hour after the user walked away from \
         the thing they asked to keep running. Got: {env}"
    );

    provision_then_restart(KEPT_CONTAINER, KEPT_PORT).await;

    // Several times the window that would have stopped the other probe. There is no event to wait
    // for here — the assertion is that nothing happens — so the only honest shape is to wait past
    // the point where something would have.
    tokio::time::sleep(Duration::from_secs(20)).await;

    assert!(
        daemon_running(KEPT_CONTAINER),
        "the daemon stopped itself despite the opt-in; §5.18 is the only place the two placements \
         may differ, and this is the difference"
    );
    let (status, _) = status_of(KEPT_CONTAINER);
    assert_eq!(status, "running");
}

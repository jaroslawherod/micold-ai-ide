//! A container daemon's start script (#1533, M5; US1.2, US4.1, FR-011): delivered under
//! `/run/micold/config`, run as the daemon's own user, never root, with `HOME` the container home,
//! both when the daemon is started over SSH and when the entrypoint starts it at container boot;
//! a refusal reaches the client with the script's output.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_startup -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::{Duration, Instant};

use sandbox_real_support::{
    cli_out, daemon_running, deliver_start_script, provision_outcome, provision_then_restart, seed,
    start_sandbox, wait_for_accept, SandboxSpec,
};

const OK_CONTAINER: &str = "micold-startup-probe";
const OK_NETWORK: &str = "micold-startup-probe-net";
const OK_PORT: u16 = 17751;

const REFUSED_CONTAINER: &str = "micold-startup-refused-probe";
const REFUSED_NETWORK: &str = "micold-startup-refused-probe-net";
const REFUSED_PORT: u16 = 17752;

fn exec(container: &str, script: &str) -> String {
    cli_out(&["exec", container, "sh", "-c", script])
}

/// The script runs once per start: over SSH when the client first connects, and again at boot when
/// the entrypoint starts the daemon after a restart. Each time as the container's own user, with
/// `HOME` the container home.
#[tokio::test]
async fn sandbox_real_startup_script_runs_as_the_daemons_user_over_ssh_and_at_boot() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "startup");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    deliver_start_script(
        dir.path(),
        "echo \"$(id -u) $HOME\" >> \"$HOME/startup-runs\"\nmkdir -p \"$HOME/.local/bin\"\n",
    );

    let _sandbox = start_sandbox(&SandboxSpec {
        container: OK_CONTAINER,
        network: OK_NETWORK,
        port: OK_PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &[],
    });
    let (_conn, _catalog) = wait_for_accept(OK_PORT).await;
    let uid = exec(OK_CONTAINER, "id -u");
    assert_ne!(uid, "0", "a sandbox never runs the script as root");
    let expected = format!("{uid} {home}");
    let runs = exec(OK_CONTAINER, "cat \"$HOME/startup-runs\"");
    assert_eq!(
        runs.lines().collect::<Vec<_>>(),
        [expected.as_str()],
        "over SSH"
    );

    provision_then_restart(OK_CONTAINER, OK_PORT).await;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let runs = exec(OK_CONTAINER, "cat \"$HOME/startup-runs\"");
        if runs.lines().count() >= 2 {
            assert!(runs.lines().all(|l| l == expected), "{runs}");
            break;
        }
        assert!(Instant::now() < deadline, "no second run at boot: {runs}");
        assert!(daemon_running(OK_CONTAINER), "the daemon is not running");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// A script the daemon refuses on: the client's connection fails with the exit status and the
/// script's output, not a timeout (US4.1).
#[tokio::test]
async fn sandbox_real_startup_refusal_reaches_the_client_with_its_output() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "startup-refused");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    deliver_start_script(dir.path(), "echo oops\nexit 1\n");

    let _sandbox = start_sandbox(&SandboxSpec {
        container: REFUSED_CONTAINER,
        network: REFUSED_NETWORK,
        port: REFUSED_PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &[],
    });
    let (_, outcome, _) =
        provision_outcome(REFUSED_PORT, sandbox_real_support::container_bounds()).await;
    let failure = outcome
        .err()
        .expect("a refused start is a failed connection");
    let text = format!(
        "{}\n{}",
        failure.output,
        failure.reason(&sandbox_real_support::reason_context(REFUSED_PORT))
    );
    assert!(text.contains("oops"), "{text}");
}

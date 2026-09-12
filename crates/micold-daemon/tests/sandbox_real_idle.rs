//! §B.5–B.6 of the quickstart — the idle rule, measured from *outside* the container (feature 028,
//! US4, FR-019/FR-020/FR-022, lifecycle contract §5.17/§5.18).
//!
//! `crates/micold-daemon/tests/idle_stop.rs` already proves the rule itself: nobody connected for
//! the window, the daemon writes its reason, hands the sessions back resumable and exits. None of
//! that is what this file is for, and repeating it here against a container would only make the
//! same assertion slower.
//!
//! What a host-placement test cannot say is whether the *container* ends. A daemon inside a
//! sandbox can obey the rule perfectly and leave a container sitting there running nothing — or
//! worse, leave one the runtime dutifully restarts, so the idle stop becomes an idle restart loop
//! and the machine keeps a service alive for nobody by a route the rule never sees. §5.17 is a
//! claim about the container's state, so it is measured as one: `exited`, and `RestartCount`
//! unchanged.
//!
//! §5.18 is the single approved exception, and it needs measuring for the opposite reason. It is
//! the only place where the two placements deliberately differ, which makes it the only place a
//! difference can hide: a sandbox that stopped itself despite the opt-in would look, from the
//! outside, exactly like a sandbox whose restart policy had failed.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::{Duration, Instant};

use micold_core::protocol::auth::Token;

use sandbox_real_support::{
    cli_out, credentials, seed, start_sandbox, wait_for_accept, SandboxSpec,
};

const IDLE_CONTAINER: &str = "micold-idle-probe";
const IDLE_NETWORK: &str = "micold-idle-probe-net";
const IDLE_PORT: u16 = 17741;

const KEPT_CONTAINER: &str = "micold-idle-kept-probe";
const KEPT_NETWORK: &str = "micold-idle-kept-probe-net";
const KEPT_PORT: u16 = 17742;

/// The window these probes run under.
///
/// Long enough that connecting, handshaking and disconnecting all happen comfortably inside a
/// *non*-idle daemon — a window shorter than the handshake would stop the daemon before the client
/// arrived, and the test would pass for the wrong reason. Short enough that the suite stays a suite.
const WINDOW: &str = "5s";

/// `{{.State.Status}}` and `{{.RestartCount}}` as one reading, because the two failures this file
/// separates are `running`/0 (the rule never fired) and `running`/1+ (it fired and the runtime
/// undid it), and taking them at different instants makes the second look like the first.
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

/// Wait for `container` to reach `exited`, or say what it was doing instead.
fn wait_for_exit(container: &str, within: Duration) -> String {
    let deadline = Instant::now() + within;
    loop {
        let (status, restarts) = status_of(container);
        if status == "exited" {
            return restarts;
        }
        assert!(
            Instant::now() < deadline,
            "the sandbox was still `{status}` {within:?} after a {WINDOW} idle window \
             (RestartCount {restarts}); the daemon inside it may have stopped, but the container \
             did not — §5.17 is a claim about the container"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// T047, FR-019/FR-020, lifecycle contract §5.17.
///
/// The container ends when the daemon inside it does, and stays ended. `RestartCount` is half the
/// assertion rather than a detail: a sandbox created with a restart policy would come straight back
/// up, turning the idle stop into a restart loop that keeps the machine busy on behalf of nobody.
/// Off is the default here precisely because that is the configuration the rule has to hold under.
#[tokio::test]
async fn sandbox_real_an_idle_sandbox_stops_and_stays_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "idle");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

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
        token_path: &token_path,
        home: &home,
        survive_logout: false,
        extra: &["-e".to_string(), window],
    });

    // Connect and leave. Without this the container might be exiting for a reason that has nothing
    // to do with the idle rule — a daemon that never came up at all also ends up `exited`.
    {
        let (_conn, _catalog) = wait_for_accept(IDLE_PORT, &credentials(&token)).await;
    }

    // Generous against the window, because what is being measured is *that* it stops, not when:
    // the rule ticks at a fraction of the window, and the ordered shutdown persists every session
    // before anything is killed.
    let restarts = wait_for_exit(IDLE_CONTAINER, Duration::from_secs(60));
    assert_eq!(
        restarts, "0",
        "the sandbox came back after stopping itself; an idle stop the runtime undoes is an idle \
         restart loop, which keeps a service running for nobody by a route the rule cannot see"
    );
}

/// T048, FR-022, lifecycle contract §5.18 — the one approved difference between the placements.
///
/// The opt-in is the user saying, in so many words, keep this running. A service asked to keep
/// running is not one to stop for being unused, so the sandbox is created with the idle rule
/// switched off and there is nothing for the window to expire.
#[tokio::test]
async fn sandbox_real_the_keep_running_opt_in_is_not_idle_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "kept");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

    // No window through `extra`: `argv::idle_stop_value` puts `off` in the argv for this spec, and
    // the harness reads it from there. Passing a window here as well would test which of the two
    // `-e` flags the runtime happens to prefer, which is not a promise this product makes.
    let _sandbox = start_sandbox(&SandboxSpec {
        container: KEPT_CONTAINER,
        network: KEPT_NETWORK,
        port: KEPT_PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
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

    {
        let (_conn, _catalog) = wait_for_accept(KEPT_PORT, &credentials(&token)).await;
    }

    // Several times the window that would have stopped the other probe. There is no event to wait
    // for here — the assertion is that nothing happens — so the only honest shape is to wait past
    // the point where something would have.
    tokio::time::sleep(Duration::from_secs(20)).await;

    let (status, restarts) = status_of(KEPT_CONTAINER);
    assert_eq!(
        status, "running",
        "the sandbox stopped itself despite the opt-in (RestartCount {restarts}); §5.18 is the \
         only place the two placements may differ, and this is the difference"
    );
    assert_eq!(
        restarts, "0",
        "the sandbox is running because the runtime restarted it, not because it never stopped — \
         which is the restart policy covering for the idle rule, and hides exactly the defect \
         this test exists to catch"
    );
}

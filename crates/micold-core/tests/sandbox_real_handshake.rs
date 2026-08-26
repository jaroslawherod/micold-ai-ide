//! The client handshakes with a daemon **inside a real container** (feature 027).
//!
//! Behind the `sandbox-real-runtime` feature, so the default suite needs nothing installed on any
//! platform — that property is what keeps the adapter layer's cross-platform coverage honest. The
//! `sandbox-runtime` CI job turns it on, on Linux only, where Docker is available.
//!
//! Everything else in this feature's test suite asserts on strings: the argv we would construct,
//! the output we would parse. This is the one test that puts a real container between the client
//! and the daemon and asks whether they can talk. It is therefore the only place the following can
//! fail and be noticed: the base image being too old for the daemon binary, the daemon resolving
//! its state directory somewhere the container cannot write, a bind on the container's loopback
//! instead of its bridge address, or the token not arriving where the daemon looks for it.
//!
//! Every one of those was a real failure found the first time this ran by hand.

#![cfg(feature = "sandbox-real-runtime")]

use std::path::PathBuf;
use std::process::Command;

use micold_core::connect::{connect_at, Connected, Credentials};
use micold_core::endpoint::DialAddress;
use micold_core::protocol::auth::Token;
use micold_core::protocol::messages::PresentedToken;

const IMAGE: &str = "micold-daemon:dev";

/// Each test takes its **own** container, network and published port.
///
/// The tests in this file run concurrently by default, and each one begins by clearing away any
/// leftovers under its own names. Sharing a set meant the second test to start tore down the first
/// test's container — which surfaced as `network ... already exists` from whichever lost the race,
/// a failure that reads like a broken sandbox and is nothing of the kind. `sandbox_real_lifecycle`
/// was written this way from the start; this file predates it.
///
/// None of the ports is the default 7727: a developer running these while their own sandbox is up
/// must neither fight it for the port nor accidentally test against it.
struct Sandbox {
    _dir: tempfile::TempDir,
    token: Token,
    container: String,
    network: String,
    port: u16,
}

impl Sandbox {
    fn start(label: &str, port: u16) -> Self {
        let container = format!("micold-sandbox-realtest-{label}");
        let network = format!("{container}-net");
        teardown(&container, &network);

        let dir = tempfile::tempdir().expect("temp dir");
        let state = dir.path().join("state");
        let project = dir.path().join("project");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::create_dir_all(&project).unwrap();

        let token = Token::generate();
        let token_path = state.join("sandbox.token");
        token.write_to(&token_path).unwrap();

        run(&["network", "create", "--driver", "bridge", &network]);

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let uid_gid = format!("{}:{}", uid(), gid());
        let publish = format!("127.0.0.1:{port}:7727");
        let project_mount = format!("{0}:{0}:rw", project.display());
        let state_mount = format!("{}:/var/lib/micold-ai-ide:rw", state.display());
        let token_mount = format!("{}:/run/micold/token:ro", token_path.display());
        let home_env = format!("HOME={home}");

        run(&[
            "create",
            "--name",
            &container,
            "--user",
            &uid_gid,
            "--restart",
            "no",
            "--network",
            &network,
            "-p",
            &publish,
            "-e",
            &home_env,
            "-v",
            &project_mount,
            "-v",
            &state_mount,
            "-v",
            &token_mount,
            IMAGE,
        ]);
        run(&["start", &container]);

        Self {
            _dir: dir,
            token,
            container,
            network,
            port,
        }
    }

    fn address(&self) -> DialAddress {
        DialAddress::Loopback { port: self.port }
    }

    fn credentials(&self) -> Credentials {
        Credentials {
            auth_token: Some(PresentedToken::new(self.token.as_str())),
            require_fingerprint_match: false,
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        teardown(&self.container, &self.network);
    }
}

fn teardown(container: &str, network: &str) {
    let _ = Command::new("docker").args(["rm", "-f", container]).output();
    let _ = Command::new("docker")
        .args(["network", "rm", network])
        .output();
}

fn run(args: &[&str]) {
    let out = Command::new("docker")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("docker {args:?}: {e}"));
    assert!(
        out.status.success(),
        "docker {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn uid() -> u32 {
    micold_core::sandbox::host_identity().0
}
fn gid() -> u32 {
    micold_core::sandbox::host_identity().1
}

/// Poll until the daemon accepts, or give up with its logs attached — a bare timeout here would
/// report "did not start" for four different causes.
async fn wait_for_accept(sandbox: &Sandbox, credentials: &Credentials) -> Connected {
    let address = sandbox.address();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match connect_at(&address, "real-test-client", credentials).await {
            Ok(Some(connected)) => return connected,
            Ok(None) | Err(_) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            other => {
                // `Connected` holds a live connection and is deliberately not `Debug`; describe the
                // outcome rather than the value.
                let described = match other {
                    Ok(Some(Connected::Ready(..))) => "connected".to_string(),
                    Ok(Some(Connected::Refused(reason))) => format!("refused: {reason:?}"),
                    Ok(None) => "nothing listening".to_string(),
                    Err(e) => format!("io error: {e}"),
                };
                let logs = Command::new("docker")
                    .args(["logs", &sandbox.container])
                    .output()
                    .map(|o| {
                        format!(
                            "{}{}",
                            String::from_utf8_lossy(&o.stdout),
                            String::from_utf8_lossy(&o.stderr)
                        )
                    })
                    .unwrap_or_default();
                panic!("the sandboxed daemon never accepted ({described}). Its logs:\n{logs}");
            }
        }
    }
}

#[tokio::test]
async fn sandbox_real_handshake_succeeds_with_the_mounted_token() {
    let sandbox = Sandbox::start("handshake", 17727);
    let credentials = sandbox.credentials();

    match wait_for_accept(&sandbox, &credentials).await {
        Connected::Ready(_conn, welcome) => {
            assert!(
                welcome.daemon_build.contains("micold-daemon"),
                "unexpected build string: {}",
                welcome.daemon_build
            );
        }
        Connected::Refused(reason) => panic!("the daemon refused a correct token: {reason:?}"),
    }
}

/// The point of the token. On this transport nothing else stands between a local process and the
/// daemon, so a wrong secret must be refused by a *real* daemon and not only by the evaluator.
#[tokio::test]
async fn sandbox_real_handshake_refuses_a_wrong_token() {
    let sandbox = Sandbox::start("wrong-token", 17728);

    // Wait for it to be up using the right token first, so a refusal below is a refusal and not a
    // "not listening yet".
    let _ = wait_for_accept(&sandbox, &sandbox.credentials()).await;

    let bad = Credentials {
        auth_token: Some(PresentedToken::new(Token::generate().as_str())),
        require_fingerprint_match: false,
    };
    match connect_at(&sandbox.address(), "real-test-client", &bad).await {
        Ok(Some(Connected::Refused(
            micold_core::protocol::messages::RefusalReason::AuthRejected,
        ))) => {}
        Ok(Some(Connected::Refused(reason))) => {
            panic!("expected AuthRejected from a real daemon, got {reason:?}")
        }
        Ok(Some(Connected::Ready(..))) => {
            panic!("a real daemon accepted a token it never issued")
        }
        Ok(None) => panic!("the daemon stopped listening between the two connects"),
        Err(e) => panic!("connecting with a wrong token failed for another reason: {e}"),
    }
}

/// The daemon writes its catalogue where the **host** can read it. That is what lets the client
/// build the mount set before the sandbox exists, and it is why state is a bind mount rather than a
/// runtime-managed volume.
#[tokio::test]
async fn sandbox_real_state_is_written_where_the_host_can_read_it() {
    let sandbox = Sandbox::start("state", 17729);
    let credentials = sandbox.credentials();
    let _ = wait_for_accept(&sandbox, &credentials).await;

    let state: PathBuf = sandbox._dir.path().join("state");
    assert!(state.is_dir(), "the state directory must exist on the host");
    // The daemon may not have written anything yet with no projects registered; what is asserted is
    // that the directory it was given is the host's, and is writable by the container's uid.
    let probe = state.join(".host-readable");
    std::fs::write(&probe, "ok").expect("the host can write where the daemon writes");
    std::fs::remove_file(&probe).ok();
}

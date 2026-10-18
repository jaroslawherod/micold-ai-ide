//! §B.7 of the quickstart, as feature 806 reshaped it: the container's daemon is **this build**.
//!
//! Before 806 a `:dev` image carried its own daemon and the client refused one built from a
//! different tree (FR-024d, research R8). The basic image carries no daemon: the client copies its
//! own into the container (FR-003), and a daemon of another build is replaced by the next
//! provisioning (`ensure_build` copies when the recorded build differs) rather than refused
//! (research R10). So the property worth a real container is that the daemon that answers is the
//! client's own build, whichever image the container started from — and that a second start with
//! the container running neither copies nor reinstalls, so the answer is still the same build.
//!
//! What this catches, and nothing else does: a provisioning that connects to a daemon left by an
//! earlier run, or an image whose entrypoint starts a daemon of its own that wins the endpoint.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::protocol::version::PACKAGE_VERSION;

use sandbox_real_support::{provision_and_connect, seed, start_sandbox, SandboxSpec};

const CONTAINER: &str = "micold-fingerprint-probe";
const NETWORK: &str = "micold-fingerprint-probe-net";
const PORT: u16 = 17734;

#[tokio::test]
async fn sandbox_real_the_container_daemon_is_the_clients_own_build() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "fingerprint");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &[],
    });

    let (first, welcome) = provision_and_connect(PORT).await;
    assert!(
        welcome.daemon_build.contains(PACKAGE_VERSION),
        "the container's daemon reports {:?}, not this build ({PACKAGE_VERSION})",
        welcome.daemon_build
    );
    drop(first);

    // The second start finds the build and the CLIs in place and bridges to the same daemon.
    let (_second, again) = provision_and_connect(PORT).await;
    assert_eq!(again.daemon_build, welcome.daemon_build);
}

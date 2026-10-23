//! The client reaches a **real container** over SSH (feature 806).
//!
//! Behind the `sandbox-real-runtime` feature, so the default suite needs nothing installed on any
//! platform. The `sandbox-runtime` CI job turns it on, on Linux only, where Docker is available.
//!
//! Everything else in the suite asserts on strings: the argv we would construct, the output we
//! would parse. These tests put a real container between the client and its sshd and ask whether
//! the keys the client makes are the keys the image accepts: the image's sshd reading the mounted
//! host key and `authorized_keys`, a pinned host key matching, a key the container never authorised
//! being refused, and the state directory being the host's.
//!
//! The daemon-level handshake over the bridge is the daemon crate's `sandbox_real_fingerprint`.

#![cfg(feature = "sandbox-real-runtime")]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use micold_core::sandbox::argv;
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::runtime::ContainerRuntime;
use micold_core::sandbox::{
    CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SANDBOX_HOME_DIR,
};

mod sandbox_real_support;
use sandbox_real_support::{image, make_keys, program, runtime_kind, wait_for_sshd};

/// Each test takes its **own** container, network and published port.
///
/// The tests in this file run concurrently by default, and each one begins by clearing away any
/// leftovers under its own names; sharing a set meant the second test to start tore down the first
/// test's container. None of the ports is a default one: a developer running these while their own
/// sandbox is up must neither fight it for the port nor accidentally test against it.
struct Sandbox {
    _dir: tempfile::TempDir,
    keys: micold_core::provision::keys::DaemonKeys,
    container: String,
    network: String,
    port: u16,
}

impl Sandbox {
    /// Creates the container with the application's own argv (`argv::create`, `argv::network_create`)
    /// against the probed capabilities of the runtime `MICOLD_TEST_RUNTIME` selects, so these tests
    /// run the mounts a user's sandbox gets — labelled on an SELinux-enforcing host, as they are for
    /// the user, and never with labelling turned off (BUG-1296, U62).
    fn start(label: &str, port: u16) -> Self {
        let container = format!("micold-sandbox-realtest-{label}");
        let network = format!("{container}-net");
        teardown(&container, &network);

        let dir = tempfile::tempdir().expect("temp dir");
        let state = dir.path().join("state");
        let project = dir.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        // Before the runtime sees them: it would create a missing bind source owned by root.
        std::fs::create_dir_all(state.join(SANDBOX_HOME_DIR)).unwrap();
        let (keys, secret) = make_keys(&state, port);

        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
        let profile = SandboxProfile {
            image: ImageChoice::Own(OwnImage::Reference(image())),
            ..SandboxProfile::default()
        };
        let (uid, gid) = micold_core::sandbox::host_identity();
        let spec = SandboxSpec {
            name: container.clone(),
            mounts: MountSet::build(
                std::slice::from_ref(&project),
                &profile,
                &CredentialLayout::default(),
                &state,
                &home,
                secret,
            ),
            profile,
            uid,
            gid,
            control_port: port,
            published_ports: Vec::new(),
            network_name: network.clone(),
            time_zone: None,
            home,
        };
        let caps = CliRuntime::new(runtime_kind(), SystemRunner)
            .probe()
            .unwrap_or_else(|e| panic!("probe {}: {}", runtime_kind(), e.reason()));
        let network_args = argv::network_create(&spec, &caps)
            .unwrap_or_else(|e| panic!("{}: {}", e.reason(), e.remedy()));
        run(&network_args);
        run(&argv::create(&spec, &caps));
        run(&["start", &container]);
        wait_for_sshd(port, Duration::from_secs(60), "the container's sshd");

        Self {
            _dir: dir,
            keys,
            container,
            network,
            port,
        }
    }

    /// The container's login user: the entrypoint gives the host uid a passwd entry.
    fn user(&self) -> String {
        let out = Command::new(program())
            .args(["exec", &self.container, "id", "-un"])
            .output()
            .expect("exec id");
        assert!(out.status.success(), "id -un failed in the container");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// `ssh <user>@127.0.0.1 true` with `key`, host key pinned to this container's; the exit code.
    fn ssh_true(&self, key: &Path) -> Option<i32> {
        Command::new("ssh")
            .args(["-F", "none", "-p", &self.port.to_string(), "-i"])
            .arg(key)
            .args(["-o", "IdentitiesOnly=yes", "-o"])
            .arg(format!(
                "UserKnownHostsFile={}",
                self.keys.known_hosts().display()
            ))
            .args(["-o", "StrictHostKeyChecking=yes", "-o", "BatchMode=yes"])
            .arg(format!("{}@127.0.0.1", self.user()))
            .arg("true")
            .status()
            .expect("run ssh")
            .code()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        teardown(&self.container, &self.network);
    }
}

fn teardown(container: &str, network: &str) {
    let _ = Command::new(program())
        .args(["rm", "-f", container])
        .output();
    let _ = Command::new(program())
        .args(["network", "rm", network])
        .output();
}

fn run<S: AsRef<OsStr> + std::fmt::Debug>(args: &[S]) {
    let program = program();
    let out = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("{program} {args:?}: {e}"));
    assert!(
        out.status.success(),
        "{program} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The client's own key is the one the container authorises, and the pinned host key matches what
/// the image's sshd presents: the mounted key files reach the paths `sshd_config` reads.
#[test]
fn sandbox_real_ssh_accepts_the_clients_key() {
    let sandbox = Sandbox::start("handshake", 17727);
    assert_eq!(
        sandbox.ssh_true(&sandbox.keys.client_key()),
        Some(0),
        "ssh with the client's key was not accepted by the container"
    );
}

/// The point of `authorized_keys`: a key the container never authorised is refused by a *real*
/// sshd, not only by the argv that mounts the right file. ssh exits 255 on an authentication
/// failure.
#[test]
fn sandbox_real_ssh_refuses_a_key_the_container_does_not_authorise() {
    let sandbox = Sandbox::start("wrong-key", 17728);
    let other = sandbox._dir.path().join("other_ed25519");
    let made = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&other)
        .status()
        .expect("ssh-keygen");
    assert!(made.success(), "could not generate a second key");
    // The right key first, so a refusal below is a refusal and not "sshd not up yet".
    assert_eq!(sandbox.ssh_true(&sandbox.keys.client_key()), Some(0));
    assert_eq!(
        sandbox.ssh_true(&other),
        Some(255),
        "a real sshd accepted a key it never authorised"
    );
}

/// The container writes its state where the **host** can read it. That is what lets the client
/// build the mount set before the sandbox exists, and it is why state is a bind mount rather than a
/// runtime-managed volume.
#[test]
fn sandbox_real_state_is_written_where_the_host_can_read_it() {
    let sandbox = Sandbox::start("state", 17729);

    let state: PathBuf = sandbox._dir.path().join("state");
    assert!(state.is_dir(), "the state directory must exist on the host");
    // A file written by the container's own user at the image's state path is visible to the host:
    // the bind mount is the host's directory, not a runtime-managed volume.
    let out = Command::new(program())
        .args([
            "exec",
            &sandbox.container,
            "sh",
            "-c",
            "echo ok > /var/lib/micold-ai-ide/.host-readable",
        ])
        .output()
        .expect("exec");
    assert!(
        out.status.success(),
        "the container cannot write its state directory: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let seen = std::fs::read_to_string(state.join(".host-readable"))
        .expect("the host reads what the container wrote");
    assert_eq!(seen.trim(), "ok");
}

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

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod sandbox_real_support;
use sandbox_real_support::{image, make_keys, wait_for_sshd};

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
    fn start(label: &str, port: u16) -> Self {
        let container = format!("micold-sandbox-realtest-{label}");
        let network = format!("{container}-net");
        teardown(&container, &network);

        let dir = tempfile::tempdir().expect("temp dir");
        let state = dir.path().join("state");
        let project = dir.path().join("project");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        let (keys, _) = make_keys(&state, port);

        run(&["network", "create", "--driver", "bridge", &network]);

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let uid_gid = format!("{}:{}", uid(), gid());
        let publish = format!(
            "127.0.0.1:{port}:{}",
            micold_core::sandbox::argv::CONTAINER_SSH_PORT
        );
        let project_mount = format!("{0}:{0}:rw", project.display());
        let state_mount = format!("{}:/var/lib/micold-ai-ide:rw", state.display());
        let ssh_dir = micold_core::sandbox::argv::CONTAINER_SSH_DIR;
        let key_mounts = [
            ("host_ed25519", "host_ed25519_key"),
            ("host_ed25519.pub", "host_ed25519_key.pub"),
            ("authorized_keys", "authorized_keys"),
        ]
        .map(|(file, as_name)| format!("{}:{ssh_dir}/{as_name}:ro", keys.dir.join(file).display()));
        let home_env = format!("HOME={home}");
        let micold_home_env = format!("MICOLD_HOME={home}");

        let mut args: Vec<String> = [
            "create",
            "--name",
            &container,
            "--user",
            &uid_gid,
            // As `argv::create` does for Docker: group 0 lets the entrypoint append the uid's
            // passwd entry, without which `id -un` and sshd cannot resolve it.
            "--group-add",
            "0",
            "--restart",
            "no",
            "--network",
            &network,
            "-p",
            &publish,
            "-e",
            &home_env,
            "-e",
            &micold_home_env,
            "-v",
            &project_mount,
            "-v",
            &state_mount,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for mount in key_mounts {
            args.push("-v".to_string());
            args.push(mount);
        }
        args.push(image());
        run(&args.iter().map(String::as_str).collect::<Vec<_>>());
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
        let out = Command::new("docker")
            .args(["exec", &self.container, "id", "-un"])
            .output()
            .expect("docker exec id");
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
    let _ = Command::new("docker")
        .args(["rm", "-f", container])
        .output();
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
    let out = Command::new("docker")
        .args([
            "exec",
            &sandbox.container,
            "sh",
            "-c",
            "echo ok > /var/lib/micold-ai-ide/.host-readable",
        ])
        .output()
        .expect("docker exec");
    assert!(
        out.status.success(),
        "the container cannot write its state directory: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let seen = std::fs::read_to_string(state.join(".host-readable"))
        .expect("the host reads what the container wrote");
    assert_eq!(seen.trim(), "ok");
}

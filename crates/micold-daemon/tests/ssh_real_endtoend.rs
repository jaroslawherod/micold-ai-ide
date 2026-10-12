//! The remote daemon over a real `ssh` (feature 489, T022, part 1): a stand-in host reached through
//! a rootless `sshd`; the client's own build is copied, started and connected.
//!
//! Off by default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason
//! when `sshd` is not installed.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use micold_core::connect::Connected;
use micold_core::ssh::argv::current_build_id;
use micold_core::ssh::provision::{connect, ProvisionOptions};
use micold_core::ssh::{ClientPlatform, SshTarget, SystemSsh};

/// The daemon pid a test started on the stand-in host, killed when the test ends.
struct RemoteDaemon(Option<u32>);

impl Drop for RemoteDaemon {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        }
    }
}

/// Inodes of the sockets a process holds open.
fn socket_inodes(pid: u32) -> Vec<String> {
    std::fs::read_dir(format!("/proc/{pid}/fd"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| std::fs::read_link(e.path()).ok())
        .filter_map(|l| {
            l.to_string_lossy()
                .strip_prefix("socket:[")
                .and_then(|s| s.strip_suffix(']'))
                .map(str::to_string)
        })
        .collect()
}

/// Inodes of every TCP socket in the listening state, either family.
fn listening_inodes() -> Vec<String> {
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|text| {
            text.lines()
                .skip(1)
                .filter_map(|l| {
                    let f: Vec<&str> = l.split_whitespace().collect();
                    (f.get(3) == Some(&"0A")).then(|| f.get(9).map(|s| s.to_string()))?
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[tokio::test]
async fn ssh_real_add_start_copies_the_build_and_connects() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_add_start_copies_the_build_and_connects")
    else {
        return;
    };
    // The remote helper starts its daemon on the pinned endpoint; a daemon already there would be
    // reached instead of the one this test provisions.
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!("SKIP ssh_real_add_start_copies_the_build_and_connects: a daemon already listens on {}", endpoint.socket_path.display());
        return;
    }

    let runner = SystemSsh {
        own_daemon: PathBuf::from(env!("CARGO_BIN_EXE_micold-daemon")),
        bundled_dir: None,
        ssh_path: Some(host.ssh_wrapper.clone()),
    };
    let target = SshTarget::new(ssh_real_support::sshd::ALIAS, None, None, None).expect("target");
    let build = current_build_id();
    let opts = ProvisionOptions {
        platform: ClientPlatform::current(),
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: 30,
        build_id: build.clone(),
        client_build: "micold-ai-ide/ssh-real".to_string(),
    };
    let seen = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = seen.clone();
    let progress: micold_core::ssh::ProvisionProgress =
        Arc::new(move |pct| sink.lock().unwrap().push(pct));

    let connected = connect(&runner, &target, &opts, Some(progress)).await;
    let pid = micold_core::spawn::running_daemon_pid(&endpoint);
    let _cleanup = RemoteDaemon(pid);
    let connected = match connected {
        Ok(c) => c,
        Err(e) => panic!("connect failed: {e:?}"),
    };
    let Connected::Ready(_connection, welcome) = connected else {
        panic!("the daemon refused the handshake");
    };

    assert!(
        !welcome.daemon_build.is_empty(),
        "the daemon reports its build"
    );
    let copied = host
        .remote_home
        .join(".micold/remote")
        .join(&build)
        .join("micold-daemon");
    assert!(copied.is_file(), "the build was copied to {}", copied.display());
    assert_eq!(
        seen.lock().unwrap().last(),
        Some(&100),
        "the copy reported progress up to 100%"
    );

    // No new listener: the daemon the bridge started binds a Unix socket only.
    let pid = pid.expect("the bridge started a daemon");
    let listening = listening_inodes();
    let held = socket_inodes(pid);
    assert!(
        !held.iter().any(|i| listening.contains(i)),
        "the remote daemon holds a listening TCP socket"
    );
}

//! SSH failures over a real `ssh` (feature 489, T060, FR-009, FR-010, SC-003): an unknown and a
//! changed host key, refused authentication, a missing key and an unreachable host each fail with
//! the named class within the connection timeout, never as a hang, and the client never records a
//! host key on the user's behalf.
//!
//! Off by default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason
//! when `sshd` is not installed.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use micold_core::ssh::SshFailure;
use ssh_real_support::remote::Dialer;
use ssh_real_support::sshd::{Sshd, ALIAS};

/// How long a failure may take: the connection timeout plus the slack of a loaded CI machine.
const TIMEOUT_SECS: u32 = 5;
const SLACK: Duration = Duration::from_secs(10);

/// An `ssh` wrapper like the harness's own, over `config`.
fn wrapper(host: &Sshd, name: &str, config: &str) -> PathBuf {
    let config_path = host.dir.join(format!("{name}_config"));
    fs::write(&config_path, config).expect("write the ssh config");
    let script = host.dir.join(format!("{name}-wrapper"));
    let real = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.join("ssh"))
        .find(|p| p.is_file())
        .expect("ssh on PATH");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nexec \"{}\" -F \"{}\" \"$@\"\n",
            real.display(),
            config_path.display()
        ),
    )
    .expect("write the wrapper");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod");
    script
}

/// The config of the harness's own wrapper with `known_hosts` and `identity` replaced.
fn config(host: &Sshd, known_hosts: &Path, identity: &Path, hostname: &str, port: u16) -> String {
    let _ = host;
    format!(
        "Host {ALIAS}\n  HostName {hostname}\n  Port {port}\n  IdentityFile {}\n  \
         IdentitiesOnly yes\n  IdentityAgent none\n  UserKnownHostsFile {}\n  \
         GlobalKnownHostsFile /dev/null\n",
        identity.display(),
        known_hosts.display()
    )
}

/// Dial and return the failure, its duration and the reason shown, failing the test on success.
async fn fails(dialer: &Dialer) -> (SshFailure, Duration) {
    let started = Instant::now();
    let outcome = tokio::time::timeout(
        Duration::from_secs(u64::from(TIMEOUT_SECS)) + SLACK,
        dialer.connect(None),
    )
    .await
    .expect("the attempt must end within the connection timeout, never hang");
    let took = started.elapsed();
    match outcome {
        Ok(_) => panic!("the dial succeeded where a failure was expected"),
        Err(failure) => (failure, took),
    }
}

#[tokio::test]
async fn ssh_real_an_unknown_host_key_is_refused_and_no_key_is_recorded() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_unknown_host_key") else {
        return;
    };
    let known = host.dir.join("empty_known_hosts");
    fs::write(&known, "").unwrap();
    let cfg = config(
        &host,
        &known,
        &host.dir.join("client_key"),
        "127.0.0.1",
        host.port,
    );
    let dialer = Dialer::with_ssh(wrapper(&host, "unknown", &cfg), ALIAS, TIMEOUT_SECS);
    let (failure, took) = fails(&dialer).await;
    assert_eq!(failure, SshFailure::HostKeyUnknown, "{failure:?}");
    assert!(took < Duration::from_secs(u64::from(TIMEOUT_SECS)) + SLACK);
    assert_eq!(
        fs::read_to_string(&known).unwrap(),
        "",
        "the client must not accept or record a host key on the user's behalf"
    );
}

#[tokio::test]
async fn ssh_real_a_changed_host_key_is_refused_and_the_record_is_untouched() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_changed_host_key") else {
        return;
    };
    // Another key under the same host entry: what a man in the middle, or a rebuilt host, shows.
    let other = host.dir.join("other_host_key");
    let status = std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&other)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!("SKIP ssh_real_changed_host_key: ssh-keygen is not available");
        return;
    }
    let pub_text = fs::read_to_string(host.dir.join("other_host_key.pub")).unwrap();
    let mut parts = pub_text.split_whitespace();
    let (kind, key) = (parts.next().unwrap(), parts.next().unwrap());
    let known = host.dir.join("changed_known_hosts");
    let recorded = format!("[127.0.0.1]:{} {kind} {key}\n", host.port);
    fs::write(&known, &recorded).unwrap();
    let cfg = config(
        &host,
        &known,
        &host.dir.join("client_key"),
        "127.0.0.1",
        host.port,
    );
    let dialer = Dialer::with_ssh(wrapper(&host, "changed", &cfg), ALIAS, TIMEOUT_SECS);
    let (failure, _) = fails(&dialer).await;
    assert_eq!(failure, SshFailure::HostKeyChanged, "{failure:?}");
    assert_eq!(
        fs::read_to_string(&known).unwrap(),
        recorded,
        "a changed key must leave the user's record as it was"
    );
}

#[tokio::test]
async fn ssh_real_refused_authentication_and_a_missing_key_are_named() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_auth_refused") else {
        return;
    };
    let known = host.dir.join("known_hosts");
    // A key the server does not authorise, then a key file that does not exist.
    let stranger = host.dir.join("stranger_key");
    let made = std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&stranger)
        .status();
    if !matches!(made, Ok(s) if s.success()) {
        eprintln!("SKIP ssh_real_auth_refused: ssh-keygen is not available");
        return;
    }
    for (name, identity) in [
        ("stranger", stranger.clone()),
        ("missing", host.dir.join("no_such_key")),
    ] {
        let cfg = config(&host, &known, &identity, "127.0.0.1", host.port);
        let dialer = Dialer::with_ssh(wrapper(&host, name, &cfg), ALIAS, TIMEOUT_SECS);
        let (failure, _) = fails(&dialer).await;
        assert_eq!(failure, SshFailure::AuthRefused, "{name}: {failure:?}");
    }
}

#[tokio::test]
async fn ssh_real_an_unreachable_host_fails_within_the_connection_timeout() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_unreachable") else {
        return;
    };
    // 192.0.2.1 is TEST-NET-1: nothing answers, or the network says no route at once.
    let cfg = config(
        &host,
        &host.dir.join("known_hosts"),
        &host.dir.join("client_key"),
        "192.0.2.1",
        22,
    );
    let dialer = Dialer::with_ssh(wrapper(&host, "unreachable", &cfg), ALIAS, TIMEOUT_SECS);
    let (failure, took) = fails(&dialer).await;
    assert!(
        matches!(failure, SshFailure::Timeout | SshFailure::HostUnreachable),
        "{failure:?}"
    );
    assert!(
        took < Duration::from_secs(u64::from(TIMEOUT_SECS)) + SLACK,
        "took {took:?}, more than the connection timeout"
    );
}

#[tokio::test]
async fn ssh_real_a_closed_port_is_a_refused_connection_not_a_hang() {
    let Some(host) = ssh_real_support::host_or_skip("ssh_real_refused") else {
        return;
    };
    let closed = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let cfg = config(
        &host,
        &host.dir.join("known_hosts"),
        &host.dir.join("client_key"),
        "127.0.0.1",
        closed,
    );
    let dialer = Dialer::with_ssh(wrapper(&host, "closed", &cfg), ALIAS, TIMEOUT_SECS);
    let (failure, _) = fails(&dialer).await;
    assert_eq!(failure, SshFailure::ConnectionRefused, "{failure:?}");
}

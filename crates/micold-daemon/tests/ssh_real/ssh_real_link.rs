//! #1725 US2 (T031): container, SSH-host and pod daemons link to the orchestrator through the
//! reverse forward the app's SSH connection opens, against the real `ssh` and a rootless `sshd`
//! (feature 489's harness). Off by default (`ssh-real-runtime`); run with `--test-threads=1`.
//!
//! The stand-in remote runs as this user on this machine, so every remote daemon resolves the same
//! pinned runtime directory (`/tmp/micold-<uid>`): the kinds behind the forward are linked one after
//! another, each under its own id, while a host daemon stays linked directly. The container and the
//! pod are reached as the app reaches them (a pinned reach, `-F none`, the app's own key); the SSH
//! host through the user's configuration (the harness's `ssh -F` wrapper), so its `ssh -G` check
//! runs. One loopback `sshd` stands for each (spec 686 R10).
//!
//! US3 (T038): killing the app's `ssh` shows that daemon `offline` within 10 s; reopening the
//! connection shows it `online` within 16 s, the same daemon process, one record.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::connect::Connected;
use micold_core::endpoint::{orchestrator_beside, resolve_pinned};
use micold_core::provision::{self, CancelToken, ProvisionTarget};
use micold_core::ssh::argv::{PinnedReach, Reach};
use micold_core::ssh::provision::{LinkRequest, ProvisionOptions};
use micold_core::ssh::{ClientPlatform, SystemSsh};

use crate::orchestrator_support::{record_when, Orchestrator};
use crate::ssh_real_support::remote::{Dialer, RemoteDaemon};
use crate::ssh_real_support::Sshd;

/// The basic image's two lines (packaging/basic/sshd_config, research R1).
const LINK_SSHD: &str = "AllowStreamLocalForwarding remote\nStreamLocalBindUnlink yes\n";

fn pinned_dir() -> PathBuf {
    resolve_pinned()
        .expect("pinned endpoint")
        .socket_path
        .parent()
        .expect("a directory")
        .to_path_buf()
}

/// `None` (printing why) when a daemon already serves the pinned endpoint: it would be reached
/// instead of the one this test starts.
fn pinned_free(test: &str) -> Option<()> {
    let endpoint = resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!(
            "SKIP {test}: a daemon already listens on {}",
            endpoint.socket_path.display()
        );
        return None;
    }
    Some(())
}

/// The configuration the remote daemon of `kind` starts with (`~/.micold/remote/daemon.toml` of
/// the stand-in home): its own id, the link on, the socket left to the pinned default.
fn write_remote_config(host: &Sshd, kind: &str, id: &str) {
    let dir = host.remote_home.join(".micold/remote");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("daemon.toml"),
        format!(
            "schema = 1\n[daemon]\nname = \"Linked {kind}\"\nid = \"{id}\"\nkind = \"{kind}\"\n\
             data_dir = '{}'\n[orchestrator_link]\nenabled = true\n",
            host.remote_home.join(format!("data-{id}")).display()
        ),
    )
    .unwrap();
}

fn link_opts(orchestrator: &Orchestrator) -> Option<LinkRequest> {
    Some(LinkRequest {
        local_socket: orchestrator.socket.clone(),
    })
}

fn user_name() -> String {
    let out = Command::new("id").arg("-un").output().expect("id -un");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// How the app reaches a container or a pod: loopback, its own key and `known_hosts`, `-F none`.
fn pinned_reach(host: &Sshd) -> Reach {
    Reach::Pinned(PinnedReach {
        host: "127.0.0.1".into(),
        port: host.port,
        user: user_name(),
        key: host.dir.join("client_key"),
        known_hosts: host.dir.join("known_hosts"),
    })
}

/// Start and connect the remote daemon through a pinned reach with the link's forward.
async fn open_pinned(
    host: &Sshd,
    link: Option<LinkRequest>,
) -> Result<Connected, provision::ProvisionFailure> {
    let runner = SystemSsh {
        own_daemon: PathBuf::from(env!("CARGO_BIN_EXE_micold-daemon")),
        bundled_dir: None,
        ssh_path: None,
        cancel: None,
    };
    let opts = ProvisionOptions {
        platform: ClientPlatform::current(),
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: 30,
        build_id: micold_core::ssh::argv::current_build_id(),
        client_build: "micold-ai-ide/ssh-real-link".to_string(),
        link,
    };
    let target = ProvisionTarget {
        reach: pinned_reach(host),
        owned: provision::Ownership::UserHost,
        bounds: provision::StepBounds::user_host(Duration::from_secs(30)),
        env: Vec::new(),
        config: None,
    };
    let sink: provision::EventSink = std::sync::Arc::new(|_| {});
    provision::run(&runner, &target, &opts, &CancelToken::new(), &sink)
        .await
        .map(|p| p.connected)
}

/// Ends the remote daemon on the pinned endpoint when a test ends, failed or not, so the next test
/// does not find it there.
struct RemoteCleanup;

impl Drop for RemoteCleanup {
    fn drop(&mut self) {
        if let Ok(endpoint) = resolve_pinned() {
            drop(RemoteDaemon(micold_core::spawn::running_daemon_pid(
                &endpoint,
            )));
        }
    }
}

/// Kill the remote daemon on the pinned endpoint and wait until it no longer serves.
fn stop_remote_daemon() {
    let endpoint = resolve_pinned().expect("pinned endpoint");
    drop(RemoteDaemon(micold_core::spawn::running_daemon_pid(
        &endpoint,
    )));
    let deadline = Instant::now() + Duration::from_secs(10);
    while micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        assert!(Instant::now() < deadline, "the remote daemon did not stop");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A host daemon process dialling the orchestrator directly (US1), killed on drop.
struct HostDaemon(Child, #[allow(dead_code)] tempfile::TempDir);

impl Drop for HostDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn host_daemon(orchestrator: &Orchestrator) -> HostDaemon {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(
        &file,
        format!(
            "schema = 1\n[daemon]\nname = \"Linked host\"\nid = \"link-host\"\nkind = \"host\"\n\
             data_dir = '{}'\n[orchestrator_link]\nenabled = true\nsocket = '{}'\n",
            dir.path().join("data").display(),
            orchestrator.socket.display()
        ),
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("run")).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
        .arg("--config")
        .arg(&file)
        .env("HOME", dir.path())
        .env("XDG_RUNTIME_DIR", dir.path().join("run"))
        .env("MICOLD_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    HostDaemon(child, dir)
}

/// The socket inodes every `sshd` process (and OpenSSH 9.8's `sshd-session`) holds open.
fn sshd_socket_inodes() -> std::collections::BTreeSet<String> {
    let mut inodes = std::collections::BTreeSet::new();
    for proc_dir in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let comm = std::fs::read_to_string(proc_dir.path().join("comm")).unwrap_or_default();
        if !comm.trim().starts_with("sshd") {
            continue;
        }
        for fd in std::fs::read_dir(proc_dir.path().join("fd"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let target = std::fs::read_link(fd.path()).unwrap_or_default();
            let target = target.to_string_lossy();
            if let Some(inode) = target
                .strip_prefix("socket:[")
                .and_then(|t| t.strip_suffix(']'))
            {
                inodes.insert(inode.to_string());
            }
        }
    }
    inodes
}

/// What `sshd` listens on: Unix socket paths (`/proc/net/unix`, flag `00010000`, the accepting
/// state) and TCP addresses (`/proc/net/tcp*`, state `0A`), each held by an `sshd` process.
fn sshd_listeners() -> (Vec<String>, Vec<String>) {
    let inodes = sshd_socket_inodes();
    let unix = std::fs::read_to_string("/proc/net/unix").unwrap_or_default();
    let mut paths: Vec<String> = unix
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() >= 8 && f[3] == "00010000" && inodes.contains(f[6])).then(|| f[7].to_string())
        })
        .collect();
    paths.sort();
    let mut tcp = Vec::new();
    for file in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        tcp.extend(text.lines().skip(1).filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() > 9 && f[3] == "0A" && inodes.contains(f[9])).then(|| f[1].to_string())
        }));
    }
    tcp.sort();
    (paths, tcp)
}

fn is_ready(connected: &Connected) -> bool {
    matches!(connected, Connected::Ready(..))
}

/// Scenarios 1 and 4, SC-001: a host daemon and a container, an SSH-host and a pod daemon behind the
/// forward are each listed `online` once, under their own id, with their own kind.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_four_kinds_are_each_online_once_under_their_own_id() {
    let test = "ssh_real_link_four_kinds_are_each_online_once_under_their_own_id";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    let _host_daemon = host_daemon(&orchestrator);
    record_when(&orchestrator, "link-host", |r| r["health"] == "online").await;

    for (kind, id) in [
        ("container", "link-container"),
        ("ssh", "link-ssh"),
        ("kubernetes", "link-pod"),
    ] {
        write_remote_config(&host, kind, id);
        let connected = if kind == "ssh" {
            let mut dialer = Dialer::new(&host);
            dialer.opts.link = link_opts(&orchestrator);
            dialer.connect(None).await.map_err(|e| format!("{e:?}"))
        } else {
            open_pinned(&host, link_opts(&orchestrator))
                .await
                .map_err(|e| format!("{e:?}"))
        };
        let connected = connected.unwrap_or_else(|e| panic!("{kind}: the open failed: {e}"));
        assert!(is_ready(&connected), "{kind}: the handshake was refused");
        let record = record_when(&orchestrator, id, |r| r["health"] == "online").await;
        assert_eq!(record["kind"], kind, "{record}");
        drop(connected);
        stop_remote_daemon();
    }

    let daemons = orchestrator.daemons().await;
    let mut ids: Vec<&str> = daemons.iter().filter_map(|d| d["id"].as_str()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        ["link-container", "link-host", "link-pod", "link-ssh"],
        "each once, under its own id"
    );
    let host_record = daemons.iter().find(|d| d["id"] == "link-host").unwrap();
    assert_eq!(
        host_record["health"], "online",
        "the host link is untouched"
    );
}

/// Scenario 2, SC-006, FR-005: while the forward is open, the remote side offers the orchestrator
/// link only: the one new listener is `orchestrator.sock` in the owner-only runtime directory, no TCP
/// port is opened, and the socket reaches the orchestrator.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_the_forward_offers_the_orchestrator_socket_only() {
    let test = "ssh_real_link_the_forward_offers_the_orchestrator_socket_only";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    write_remote_config(&host, "container", "link-only");
    let (unix_before, tcp_before) = sshd_listeners();
    assert!(
        unix_before.is_empty(),
        "sshd listens on no socket yet: {unix_before:?}"
    );

    let connected = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the open succeeds");
    record_when(&orchestrator, "link-only", |r| r["health"] == "online").await;

    let dir = pinned_dir();
    let remote = orchestrator_beside(&resolve_pinned().unwrap()).socket_path;
    let (unix_after, tcp_after) = sshd_listeners();
    assert_eq!(
        unix_after,
        [remote.display().to_string()],
        "the remote offers the orchestrator socket only"
    );
    assert_eq!(tcp_before, tcp_after, "the forward opened no TCP port");
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::symlink_metadata(&dir)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700, "reachable only by the daemon's own user");

    // A probe through the forward reaches the orchestrator: it answers its own `list` query.
    let through = Orchestrator::external(orchestrator.dir.clone(), remote).await;
    let ids: Vec<String> = through
        .daemons()
        .await
        .iter()
        .filter_map(|d| d["id"].as_str().map(str::to_string))
        .collect();
    assert_eq!(ids, ["link-only"]);
    drop(connected);
    stop_remote_daemon();
}

/// Scenario 3: a remote endpoint the forward cannot take (a directory `prepare-link` cannot remove)
/// fails the open with a message, and no bridge runs without the forward. ssh's own bind failure is
/// the `AllowStreamLocalForwarding no` test's.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_an_endpoint_prepare_link_cannot_clear_fails_the_open_with_a_message() {
    let test = "ssh_real_link_an_endpoint_prepare_link_cannot_clear_fails_the_open_with_a_message";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    write_remote_config(&host, "container", "link-taken");
    // `prepare-link` removes any socket there; a directory in its place cannot be removed or bound.
    let remote = orchestrator_beside(&resolve_pinned().unwrap()).socket_path;
    let _ = std::fs::remove_file(&remote);
    std::fs::create_dir(&remote).unwrap();
    std::fs::write(remote.join("keep"), b"").unwrap();

    let failure = open_pinned(&host, link_opts(&orchestrator)).await;
    std::fs::remove_dir_all(&remote).unwrap();
    let failure = match failure {
        Ok(_) => panic!("the open must fail while the endpoint is taken"),
        Err(f) => f,
    };
    let reason = failure.reason(&micold_core::ssh::ReasonContext {
        host: "127.0.0.1".into(),
        port: host.port,
        timeout_secs: 30,
        platform: ClientPlatform::current(),
    });
    assert!(
        reason.contains("orchestrator link could not be opened"),
        "{reason}"
    );
    assert!(
        matches!(
            failure.cause,
            provision::FailureCause::LinkForward(
                micold_core::ssh::argv::LinkRefusal::Prepare { .. }
            )
        ),
        "the remote step refused it: {failure:?}"
    );
    assert!(
        micold_core::spawn::running_daemon_pid(&resolve_pinned().unwrap()).is_none(),
        "no bridge started a daemon without the forward"
    );
}

/// FR-005: a server with `AllowStreamLocalForwarding no` fails the open with ssh's own text.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_a_server_refusing_socket_forwards_fails_with_ssh_text() {
    let test = "ssh_real_link_a_server_refusing_socket_forwards_fails_with_ssh_text";
    let Some(host) = Sshd::start_with("AllowStreamLocalForwarding no\n")
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    write_remote_config(&host, "container", "link-refused");

    let failure = match open_pinned(&host, link_opts(&orchestrator)).await {
        Ok(_) => panic!("the open must fail without the forward"),
        Err(f) => f,
    };
    let text = format!("{failure:?}");
    assert!(
        text.contains("remote port forwarding failed"),
        "ssh's own text: {text}"
    );
    assert!(
        matches!(
            failure.cause,
            provision::FailureCause::LinkForward(
                micold_core::ssh::argv::LinkRefusal::NotBound { .. }
            )
        ),
        "{failure:?}"
    );
    stop_remote_daemon();
}

/// SC-002's bound on `offline`: the heartbeat deadline plus one check interval.
const OFFLINE_WITHIN: Duration = Duration::from_secs(10);

/// SC-002's bound on `online` after a reopen: one capped reconnect delay plus a handshake.
const ONLINE_WITHIN: Duration = Duration::from_secs(16);

/// What the test's own observation adds to those bounds: the clock follower's step, the 50 ms poll of
/// the registry and the `list` query itself.
const POLL_SLACK: Duration = Duration::from_millis(500);

/// Moves the harness orchestrator's test clock on with real time, in 20 ms steps, so its heartbeat
/// deadline runs out as a real orchestrator's does; stops when dropped.
struct RealTime(tokio::task::JoinHandle<()>);

impl RealTime {
    fn follow(orchestrator: &Orchestrator) -> Self {
        let clock = std::sync::Arc::clone(&orchestrator.clock);
        Self(tokio::spawn(async move {
            let mut last = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let now = Instant::now();
                clock.advance(now - last);
                last = now;
            }
        }))
    }
}

impl Drop for RealTime {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The pids of the app's `ssh` processes carrying the link's `-R` to `local_socket`.
fn link_ssh_pids(local_socket: &std::path::Path) -> Vec<u32> {
    let suffix = format!(":{}", local_socket.display());
    let mut pids = Vec::new();
    for proc_dir in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = proc_dir.file_name().to_str().and_then(|n| n.parse().ok()) else {
            continue;
        };
        let comm = std::fs::read_to_string(proc_dir.path().join("comm")).unwrap_or_default();
        if comm.trim() != "ssh" {
            continue;
        }
        let cmdline = std::fs::read(proc_dir.path().join("cmdline")).unwrap_or_default();
        let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
        let forwards = args
            .windows(2)
            .any(|w| w[0] == b"-R" && String::from_utf8_lossy(w[1]).ends_with(&suffix));
        if forwards {
            pids.push(pid);
        }
    }
    pids
}

/// Kill the app's one `ssh` carrying the link with `signal`, as a dropped connection does.
fn kill_link_ssh(orchestrator: &Orchestrator, signal: &str) {
    let pids = link_ssh_pids(&orchestrator.socket);
    assert_eq!(pids.len(), 1, "one ssh carries the link: {pids:?}");
    let status = Command::new("kill")
        .args([signal, &pids[0].to_string()])
        .status()
        .unwrap();
    assert!(status.success(), "kill {signal} {}", pids[0]);
}

fn remote_daemon_pid() -> u32 {
    micold_core::spawn::running_daemon_pid(&resolve_pinned().unwrap())
        .expect("the remote daemon runs")
}

/// The remote daemon's link object of `status.json`.
fn remote_link() -> micold_core::ssh::status::LinkStatusReport {
    micold_daemon::runtime_status::read(&resolve_pinned().unwrap())
        .and_then(|s| s.link)
        .expect("the remote daemon reports its link")
}

/// The remote daemon's log lines that name the orchestrator link (its `$HOME` is the stand-in's).
fn link_log_lines(host: &Sshd) -> Vec<String> {
    fn walk(dir: &std::path::Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.file_name().is_some_and(|n| n == "micold-daemon.log") {
                found.push(path);
            }
        }
    }
    let mut logs = Vec::new();
    walk(&host.remote_home, &mut logs);
    logs.iter()
        .flat_map(|p| {
            std::fs::read_to_string(p)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|l| l.contains("orchestrator"))
        .collect()
}

/// Wait until `id`'s record is `health`, returning how long it took from `since`.
async fn health_within(
    orchestrator: &Orchestrator,
    id: &str,
    health: &str,
    since: Instant,
) -> Duration {
    record_when(orchestrator, id, |r| r["health"] == health).await;
    since.elapsed()
}

/// Scenarios 1-3, SC-002, FR-009: killing the connection shows that daemon `offline` within 10 s
/// and leaves the directly linked host daemon `online`; while the forward is down the dials are
/// connect failures (a stale or absent `orchestrator.sock`), not refusals, backing off to the cap
/// without a log line each; reopening shows it `online` within 16 s, the same process, one record.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_a_dropped_connection_shows_offline_and_a_reopen_online_with_the_same_daemon()
{
    let test =
        "ssh_real_link_a_dropped_connection_shows_offline_and_a_reopen_online_with_the_same_daemon";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    let _real_time = RealTime::follow(&orchestrator);
    let _host_daemon = host_daemon(&orchestrator);
    record_when(&orchestrator, "link-host", |r| r["health"] == "online").await;
    write_remote_config(&host, "container", "link-drop");
    let first = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the open succeeds");
    record_when(&orchestrator, "link-drop", |r| r["health"] == "online").await;
    let pid = remote_daemon_pid();

    let dropped = Instant::now();
    kill_link_ssh(&orchestrator, "-TERM");
    let waited = health_within(&orchestrator, "link-drop", "offline", dropped).await;
    assert!(
        waited <= OFFLINE_WITHIN + POLL_SLACK,
        "offline {waited:?} after the drop"
    );
    let host_record = orchestrator.daemons().await;
    let host_record = host_record.iter().find(|d| d["id"] == "link-host").unwrap();
    assert_eq!(
        host_record["health"], "online",
        "the other daemon is unchanged"
    );
    drop(first);

    // The forward is down: whatever the dead session left (`orchestrator.sock` stale or gone), a
    // dial fails to connect. Past 1 + 2 + 4 + 8 s the delay is at its 15 s cap.
    let remote_socket = orchestrator_beside(&resolve_pinned().unwrap()).socket_path;
    if std::fs::symlink_metadata(&remote_socket).is_err() {
        drop(std::os::unix::net::UnixListener::bind(&remote_socket).unwrap());
    }
    let lost = within_link(|l| l.state == "not-linked").await;
    let lines_at_loss = link_log_lines(&host).len();
    let backoff_until = Instant::now() + Duration::from_secs(16);
    while Instant::now() < backoff_until {
        let link = remote_link();
        assert_eq!(
            link.state, "not-linked",
            "a connect failure, never a refusal: {link:?}"
        );
        assert_eq!(
            link.since, lost.since,
            "a retry ending the same way changes nothing"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let lines = link_log_lines(&host);
    assert_eq!(
        lines.len(),
        lines_at_loss,
        "no log line per retry: {lines:#?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("orchestrator link lost"))
            .count(),
        1,
        "the loss is logged once: {lines:#?}"
    );
    assert!(!lines.iter().any(|l| l.contains("refused")), "{lines:#?}");

    let reopened = Instant::now();
    let second = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the reopen succeeds");
    let waited = health_within(&orchestrator, "link-drop", "online", reopened).await;
    assert!(
        waited <= ONLINE_WITHIN,
        "online {waited:?} after the reopen"
    );
    assert_eq!(remote_daemon_pid(), pid, "the daemon was not restarted");
    let daemons = orchestrator.daemons().await;
    let mut ids: Vec<&str> = daemons.iter().filter_map(|d| d["id"].as_str()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        ["link-drop", "link-host"],
        "the record is updated, not duplicated"
    );
    drop(second);
    stop_remote_daemon();
}

/// Spec edge case: the app's `ssh` is killed with `SIGKILL` and the connection reopened at once,
/// while the remote end of the old forward may still be held. `prepare-link` clears the socket,
/// the new forward binds, and the same daemon is `online` again within 16 s.
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_a_killed_connection_reopened_at_once_comes_back_online() {
    let test = "ssh_real_link_a_killed_connection_reopened_at_once_comes_back_online";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    let _real_time = RealTime::follow(&orchestrator);
    write_remote_config(&host, "container", "link-kill");
    let first = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the open succeeds");
    record_when(&orchestrator, "link-kill", |r| r["health"] == "online").await;
    let pid = remote_daemon_pid();
    let first_link = within_link(|l| l.state == "linked").await;
    // `since` counts whole seconds: past this one, a new link's `since` is later than the first's.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    kill_link_ssh(&orchestrator, "-KILL");
    let reopened = Instant::now();
    let second = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the reopen succeeds over the old forward's socket");
    drop(first);
    // The orchestrator keeps the record `online` until its deadline, so the return is timed on the
    // daemon's side: a new link, entered after the first.
    within_link(|l| l.state == "linked" && l.since > first_link.since).await;
    let waited = reopened.elapsed();
    assert!(
        waited <= ONLINE_WITHIN + POLL_SLACK,
        "linked again {waited:?} after the reopen"
    );
    // Past the heartbeat deadline plus a check, a link still riding the killed session would have
    // ended on both sides: still linked and online now, it runs through the new forward.
    tokio::time::sleep_until((reopened + OFFLINE_WITHIN + POLL_SLACK).into()).await;
    let link = remote_link();
    assert_eq!(link.state, "linked", "{link:?}");
    record_when(&orchestrator, "link-kill", |r| r["health"] == "online").await;
    assert_eq!(remote_daemon_pid(), pid, "the daemon was not restarted");
    assert_eq!(
        link_ssh_pids(&orchestrator.socket).len(),
        1,
        "only the new forward"
    );
    assert_eq!(orchestrator.daemons().await.len(), 1, "one record");
    drop(second);
    stop_remote_daemon();
}

/// The remote daemon's link object once `ready` holds for it.
async fn within_link(
    ready: impl Fn(&micold_core::ssh::status::LinkStatusReport) -> bool,
) -> micold_core::ssh::status::LinkStatusReport {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(link) = micold_daemon::runtime_status::read(&resolve_pinned().unwrap())
            .and_then(|s| s.link)
            .filter(|l| ready(l))
        {
            return link;
        }
        assert!(Instant::now() < deadline, "the remote link never settled");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// #1726 M4 (T033, FR-007): after provisioning the bridge is closed and the link holder keeps the
/// daemon's forward; the daemon stays linked under its wire id with no bridge, and ending the holder
/// takes it `offline`. The route itself is opened by id through the orchestrator
/// (`micold-client` `orchestrator_provisioned`).
#[tokio::test(flavor = "multi_thread")]
async fn ssh_real_link_the_holder_keeps_a_provisioned_daemon_linked_without_its_bridge() {
    let test = "ssh_real_link_the_holder_keeps_a_provisioned_daemon_linked_without_its_bridge";
    let Some(host) = Sshd::start_with(LINK_SSHD)
        .map_err(|e| eprintln!("SKIP {test}: {e}"))
        .ok()
    else {
        return;
    };
    let Some(()) = pinned_free(test) else { return };
    let _cleanup = RemoteCleanup;
    let orchestrator = Orchestrator::start().await;
    // Offline is judged on the orchestrator's clock: without real time it never passes.
    let _real_time = RealTime::follow(&orchestrator);
    write_remote_config(&host, "ssh", "link-routed");
    let connected = open_pinned(&host, link_opts(&orchestrator))
        .await
        .expect("the open succeeds");
    record_when(&orchestrator, "link-routed", |r| r["health"] == "online").await;

    // Swap order: the bridge goes first, then the holder takes the forward over.
    drop(connected);
    // The bridge's forward is gone: the record is offline before the holder brings it back, so
    // the `online` below is the holder's link and not a stale record.
    health_within(&orchestrator, "link-routed", "offline", Instant::now()).await;
    let runner = SystemSsh {
        own_daemon: PathBuf::from(env!("CARGO_BIN_EXE_micold-daemon")),
        bundled_dir: None,
        ssh_path: None,
        cancel: None,
    };
    let opts = ProvisionOptions {
        platform: ClientPlatform::current(),
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: 30,
        build_id: micold_core::ssh::argv::current_build_id(),
        client_build: "micold-ai-ide/ssh-real-link".to_string(),
        link: link_opts(&orchestrator),
    };
    let target = ProvisionTarget {
        reach: pinned_reach(&host),
        owned: provision::Ownership::UserHost,
        bounds: provision::StepBounds::user_host(Duration::from_secs(30)),
        env: Vec::new(),
        config: None,
    };
    let request = opts.link.clone().expect("the link is asked for");
    let holder = provision::hold_link(&runner, &target, &opts, &request, &CancelToken::new())
        .await
        .expect("the holder opens");
    record_when(&orchestrator, "link-routed", |r| r["health"] == "online").await;

    let dropped = Instant::now();
    drop(holder);
    let waited = health_within(&orchestrator, "link-routed", "offline", dropped).await;
    assert!(
        waited <= OFFLINE_WITHIN + POLL_SLACK,
        "offline {waited:?} after the holder ended"
    );
    stop_remote_daemon();
}

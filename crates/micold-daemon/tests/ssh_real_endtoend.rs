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

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{Connected, DaemonConnection, Welcome};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult};
use micold_core::ssh::argv::current_build_id;
use micold_core::ssh::provision::{connect, ProvisionOptions};
use micold_core::ssh::{ClientPlatform, SshTarget, SystemSsh};

/// The daemon pid a test started on the stand-in host, killed when the test ends.
struct RemoteDaemon(Option<u32>);

impl Drop for RemoteDaemon {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();
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

/// Inodes of every TCP socket listening on a non-loopback address, either family. The daemon's own
/// loopback listeners (hooks, MCP) are not reachable from the network and are not what FR-005 bars.
fn listening_inodes() -> Vec<String> {
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|text| {
            text.lines()
                .skip(1)
                .filter_map(|l| {
                    let f: Vec<&str> = l.split_whitespace().collect();
                    let loopback = f.get(1).is_some_and(|a| {
                        a.starts_with("0100007F:")
                            || a.starts_with("00000000000000000000000001000000:")
                    });
                    (f.get(3) == Some(&"0A") && !loopback)
                        .then(|| f.get(9).map(|s| s.to_string()))?
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A daemon provisioned on the stand-in host and connected to; the remote daemon is killed on drop.
struct Started {
    connection: Box<DaemonConnection>,
    welcome: Welcome,
    build: String,
    progress: Arc<Mutex<Vec<u8>>>,
    pid: Option<u32>,
    _cleanup: RemoteDaemon,
}

/// Provision and connect, or `None` (printing why) when the stand-in host or the endpoint is not
/// available. The remote helper starts its daemon on the pinned endpoint; a daemon already there
/// would be reached instead of the one this test provisions.
async fn start(host: &ssh_real_support::sshd::Sshd, test: &str) -> Option<Started> {
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!(
            "SKIP {test}: a daemon already listens on {}",
            endpoint.socket_path.display()
        );
        return None;
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
    let progress = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = progress.clone();
    let report: micold_core::ssh::ProvisionProgress =
        Arc::new(move |pct| sink.lock().unwrap().push(pct));

    let connected = connect(&runner, &target, &opts, Some(report)).await;
    let pid = micold_core::spawn::running_daemon_pid(&endpoint);
    let cleanup = RemoteDaemon(pid);
    let (connection, welcome) = match connected {
        Ok(Connected::Ready(connection, welcome)) => (connection, welcome),
        Ok(Connected::Refused(_)) => panic!("the daemon refused the handshake"),
        Err(e) => panic!("connect failed: {e:?}"),
    };
    Some(Started {
        connection,
        welcome,
        build,
        progress,
        pid,
        _cleanup: cleanup,
    })
}

#[tokio::test]
async fn ssh_real_add_start_copies_the_build_and_connects() {
    let Some(host) =
        ssh_real_support::host_or_skip("ssh_real_add_start_copies_the_build_and_connects")
    else {
        return;
    };
    let Some(started) = start(&host, "ssh_real_add_start_copies_the_build_and_connects").await
    else {
        return;
    };

    assert!(
        !started.welcome.daemon_build.is_empty(),
        "the daemon reports its build"
    );
    let copied = host
        .remote_home
        .join(".micold/remote")
        .join(&started.build)
        .join("micold-daemon");
    assert!(
        copied.is_file(),
        "the build was copied to {}",
        copied.display()
    );
    assert_eq!(
        started.progress.lock().unwrap().last(),
        Some(&100),
        "the copy reported progress up to 100%"
    );

    // No network listener: the daemon the bridge started is reached over its Unix socket only.
    let pid = started.pid.expect("the bridge started a daemon");
    let listening = listening_inodes();
    let held = socket_inodes(pid);
    assert!(
        !held.iter().any(|i| listening.contains(i)),
        "the remote daemon holds a non-loopback listening TCP socket"
    );
}

/// Send one correlated request and wait for its answer, collecting the text of every grid frame
/// that arrives meanwhile.
async fn request(
    connection: &mut DaemonConnection,
    msg: ClientMsg,
    req: u64,
    seen: &mut String,
) -> OperationResult {
    connection
        .send(Frame::Control(msg))
        .await
        .expect("send the request");
    let deadline = std::time::Duration::from_secs(30);
    loop {
        let frame = tokio::time::timeout(deadline, connection.next())
            .await
            .unwrap_or_else(|_| panic!("no answer to request {req}"))
            .expect("the connection stays open")
            .expect("a decodable frame");
        match frame {
            Frame::Control(DaemonMsg::OperationOk { req: r, result }) if r == req => return result,
            Frame::Control(DaemonMsg::OperationError { req: r, .. }) if r == req => {
                panic!("request {req} was refused")
            }
            Frame::Grid(grid) => {
                for line in &grid.lines {
                    seen.push_str(&line.text);
                    seen.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// Feature 489, T032, FR-012/FR-013: over the real `ssh`, a repository on the host is added as a
/// project, a worktree is created in it, a session with a stand-in AI CLI runs there, and its
/// input and a shell command's output come back.
#[tokio::test]
async fn ssh_real_repository_worktree_session_and_terminal_work_on_the_host() {
    use micold_core::protocol::messages::SessionProcess;
    use micold_core::session::{AiCli, ShellInstanceId};
    use micold_core::worktree::CreateMode;

    let name = "ssh_real_repository_worktree_session_and_terminal_work_on_the_host";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    // The stand-in host's sessions find this `claude` first (`SetEnv PATH` of the harness).
    let bin = host.remote_home.join("bin");
    std::fs::create_dir_all(&bin).expect("a bin directory on the host");
    let claude = bin.join("claude");
    std::fs::write(&claude, "#!/bin/sh\necho STAND-IN-AI-READY\nexec cat\n").expect("stand-in");
    std::fs::set_permissions(&claude, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("executable");
    let repo = host.remote_home.join("repo");
    std::fs::create_dir_all(&repo).expect("repo directory");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    ] {
        let status = std::process::Command::new("git")
            .args(&args)
            .current_dir(&repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}");
    }

    let Some(mut started) = start(&host, name).await else {
        return;
    };
    let mut seen = String::new();
    let conn = &mut *started.connection;

    let result = request(
        conn,
        ClientMsg::ProjectAdd {
            req: 1,
            path: repo.clone(),
        },
        1,
        &mut seen,
    )
    .await;
    assert_eq!(result, OperationResult::Ack, "the repository is added");

    let result = request(
        conn,
        ClientMsg::WorktreeCreate {
            req: 2,
            project: repo.clone(),
            branch: "feature-x".into(),
            dir_name: "feature-x".into(),
            mode: CreateMode::NewBranch,
            session: None,
        },
        2,
        &mut seen,
    )
    .await;
    assert!(
        matches!(result, OperationResult::WorktreeCreated { .. }),
        "{result:?}"
    );

    let OperationResult::SessionCreated { session } = request(
        conn,
        ClientMsg::SessionCreate {
            req: 3,
            project: repo.clone(),
            worktree_dir: "feature-x".into(),
            provider: AiCli::ClaudeCode,
            allow_additional_session: false,
        },
        3,
        &mut seen,
    )
    .await
    else {
        panic!("a session is created")
    };
    conn.send(Frame::Control(ClientMsg::SessionStart {
        session,
        allow_additional_session: false,
    }))
    .await
    .expect("start the session");
    conn.send(Frame::Control(ClientMsg::SessionInput {
        session,
        serial: 1,
        bytes: b"hello from the client\r".to_vec(),
        process: Some(SessionProcess::Primary),
    }))
    .await
    .expect("send input");
    // A terminal: an additional shell on the same session, a command typed into it.
    let shell = ShellInstanceId(1);
    conn.send(Frame::Control(ClientMsg::SessionOpenShell {
        session,
        instance: shell,
    }))
    .await
    .expect("open a shell");
    // Grids stream only for the terminals a client views.
    conn.send(Frame::Control(ClientMsg::SetViewedTerminals {
        project: repo.clone(),
        terminals: vec![
            micold_core::protocol::messages::TerminalRef {
                session,
                process: SessionProcess::Primary,
            },
            micold_core::protocol::messages::TerminalRef {
                session,
                process: SessionProcess::Shell(shell),
            },
        ],
    }))
    .await
    .expect("view the terminals");
    conn.send(Frame::Control(ClientMsg::SessionInput {
        session,
        serial: 2,
        bytes: b"echo TERMINAL-$((40+2))\r".to_vec(),
        process: Some(SessionProcess::Shell(shell)),
    }))
    .await
    .expect("send the command");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::time::Instant::now() < deadline
        && !(seen.contains("STAND-IN-AI-READY") && seen.contains("TERMINAL-42"))
    {
        if let Ok(Some(Ok(Frame::Grid(grid)))) =
            tokio::time::timeout(std::time::Duration::from_millis(500), conn.next()).await
        {
            for line in &grid.lines {
                seen.push_str(&line.text);
                seen.push('\n');
            }
        }
    }
    assert!(seen.contains("STAND-IN-AI-READY"), "AI output: {seen}");
    assert!(seen.contains("TERMINAL-42"), "terminal output: {seen}");
}

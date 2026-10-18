//! The remote daemon over a real `ssh` (feature 489, T022, part 1): a stand-in host reached through
//! a rootless `sshd`; the client's own build is copied, started and connected.
//!
//! Off by default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason
//! when `sshd` is not installed.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, OperationResult};
use ssh_real_support::remote::{request, start};

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

#[tokio::test]
async fn ssh_real_repository_worktree_session_and_terminal_work_on_the_host() {
    use micold_core::protocol::messages::SessionProcess;
    use micold_core::session::{AiCli, ShellInstanceId};
    use micold_core::worktree::CreateMode;

    let name = "ssh_real_repository_worktree_session_and_terminal_work_on_the_host";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let repo = ssh_real_support::remote::stand_in_repo(
        &host,
        "#!/bin/sh\necho STAND-IN-AI-READY\nexec cat\n",
    );

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
            kind: Default::default(),
            allow_additional_session: true,
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
        allow_additional_session: true,
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

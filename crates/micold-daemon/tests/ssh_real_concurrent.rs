//! Several clients on one remote daemon (feature 489, T053, FR-021, FR-013): two bridges to one
//! daemon see the same sessions, input reaches only the session it was sent to, and one client
//! leaving leaves the other connected. A cut remote link never reaches a local session.
//!
//! Off by default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason
//! when `sshd` is not installed.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{Connected, DaemonConnection};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, OperationResult, SessionProcess, TerminalRef,
};
use micold_core::session::{AiCli, SessionId};
use micold_core::worktree::CreateMode;
use ssh_real_support::remote::{request, start};

/// A session reading lines from its terminal and echoing them with the worktree's marker.
const ECHO: &str = "#!/bin/sh\nwhile read line; do echo \"GOT-$line\"; done\n";

async fn view(conn: &mut DaemonConnection, repo: &std::path::Path, session: SessionId) {
    conn.send(Frame::Control(ClientMsg::SetViewedTerminals {
        project: repo.to_path_buf(),
        terminals: vec![TerminalRef {
            session,
            process: SessionProcess::Primary,
        }],
    }))
    .await
    .expect("view the terminal");
}

/// Read grid frames for `span` and return their text.
async fn drain(conn: &mut DaemonConnection, span: Duration) -> String {
    let mut seen = String::new();
    let end = Instant::now() + span;
    while Instant::now() < end {
        if let Ok(Some(Ok(Frame::Grid(grid)))) =
            tokio::time::timeout(Duration::from_millis(300), conn.next()).await
        {
            for line in &grid.lines {
                seen.push_str(&line.text);
                seen.push('\n');
            }
        }
    }
    seen
}

async fn create_session(
    conn: &mut DaemonConnection,
    repo: &std::path::Path,
    dir: &str,
    req: u64,
    seen: &mut String,
) -> SessionId {
    let created = request(
        conn,
        ClientMsg::WorktreeCreate {
            req,
            project: repo.to_path_buf(),
            branch: dir.into(),
            dir_name: dir.into(),
            mode: CreateMode::NewBranch,
            session: None,
        },
        req,
        seen,
    )
    .await;
    assert!(matches!(created, OperationResult::WorktreeCreated { .. }));
    let OperationResult::SessionCreated { session } = request(
        conn,
        ClientMsg::SessionCreate {
            req: req + 1,
            project: repo.to_path_buf(),
            worktree_dir: dir.into(),
            provider: AiCli::ClaudeCode,
            kind: Default::default(),
            permission_mode: Default::default(),
            allow_additional_session: true,
        },
        req + 1,
        seen,
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
    session
}

#[tokio::test]
async fn ssh_real_two_clients_share_one_daemon_and_one_leaving_leaves_the_other() {
    let name = "ssh_real_two_clients_share_one_daemon_and_one_leaving_leaves_the_other";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let repo = ssh_real_support::remote::stand_in_repo(&host, ECHO);
    let Some(mut first) = start(&host, name).await else {
        return;
    };
    let mut seen = String::new();
    let added = request(
        &mut *first.connection,
        ClientMsg::ProjectAdd {
            req: 1,
            path: repo.clone(),
        },
        1,
        &mut seen,
    )
    .await;
    assert_eq!(added, OperationResult::Ack);
    let one = create_session(&mut first.connection, &repo, "one", 2, &mut seen).await;
    let two = create_session(&mut first.connection, &repo, "two", 4, &mut seen).await;

    // A second bridge to the same daemon: the same process, the same sessions.
    let Ok(Connected::Ready(mut second, welcome)) = first.dialer.connect(None).await else {
        panic!("a second client connects to the running daemon")
    };
    assert_eq!(
        micold_core::spawn::running_daemon_pid(
            &micold_core::endpoint::resolve_pinned().expect("pinned endpoint")
        ),
        first.pid,
        "one daemon serves both clients"
    );
    let known: Vec<SessionId> = welcome
        .catalog
        .projects
        .iter()
        .flat_map(|p| p.sessions.iter().map(|s| s.id))
        .collect();
    assert!(
        known.contains(&one) && known.contains(&two),
        "the second client sees both sessions: {known:?}"
    );

    // Input from the second client reaches session one only.
    view(&mut first.connection, &repo, one).await;
    view(&mut second, &repo, one).await;
    second
        .send(Frame::Control(ClientMsg::SessionInput {
            session: one,
            serial: 1,
            bytes: b"hello\r".to_vec(),
            process: Some(SessionProcess::Primary),
        }))
        .await
        .expect("send input");
    let on_first = drain(&mut first.connection, Duration::from_secs(3)).await;
    assert!(on_first.contains("GOT-hello"), "{on_first}");
    view(&mut first.connection, &repo, two).await;
    // Frames of session one still in flight are not session two's.
    drain(&mut first.connection, Duration::from_secs(1)).await;
    let other = drain(&mut first.connection, Duration::from_secs(2)).await;
    assert!(
        !other.contains("GOT-hello"),
        "session two got nothing: {other}"
    );

    // The second client leaves; the first stays connected, answers, and still reaches session one.
    drop(second);
    tokio::time::sleep(Duration::from_millis(500)).await;
    first
        .connection
        .send(Frame::Control(ClientMsg::Ping { nonce: 7 }))
        .await
        .expect("the first client is still connected");
    let pong = async {
        loop {
            if let Some(Ok(Frame::Control(DaemonMsg::Pong { nonce: 7 }))) =
                first.connection.next().await
            {
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), pong)
        .await
        .expect("the first client still gets answers after the other left");
    first
        .connection
        .send(Frame::Control(ClientMsg::SessionInput {
            session: one,
            serial: 2,
            bytes: b"again\r".to_vec(),
            process: Some(SessionProcess::Primary),
        }))
        .await
        .expect("send input");
    view(&mut first.connection, &repo, one).await;
    let text = drain(&mut first.connection, Duration::from_secs(3)).await;
    assert!(text.contains("GOT-again"), "{text}");
}

/// A daemon of its own on a private runtime directory, standing in for the user's local daemon.
struct LocalDaemon {
    child: std::process::Child,
}

impl Drop for LocalDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn ssh_real_a_local_session_runs_on_while_the_remote_link_is_cut() {
    let name = "ssh_real_a_local_session_runs_on_while_the_remote_link_is_cut";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let tick = "#!/bin/sh\ni=1\nwhile true; do echo TICK-$i; i=$((i+1)); sleep 1; done\n";
    ssh_real_support::remote::stand_in_repo(&host, tick);
    let Some(mut remote) = start(&host, name).await else {
        return;
    };

    // The local daemon: its own runtime directory, home and project, the same stand-in `claude`.
    let base = host.dir.join("local");
    let (run, home, local_repo) = (base.join("run"), base.join("home"), base.join("repo"));
    for dir in [&run, &home] {
        std::fs::create_dir_all(dir).expect("local directory");
    }
    std::fs::set_permissions(&run, std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .expect("private runtime directory");
    ssh_real_support::remote::git_repo_at(&local_repo);
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_micold-daemon"))
        .env("XDG_RUNTIME_DIR", &run)
        .env("HOME", &home)
        .env(
            "PATH",
            format!(
                "{}/bin:/usr/local/bin:/usr/bin:/bin",
                host.remote_home.display()
            ),
        )
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start the local daemon");
    let _local_daemon = LocalDaemon { child };
    let endpoint = micold_core::endpoint::Endpoint {
        socket_path: run.join("micold").join("daemon.sock"),
        lock_path: run.join("micold").join("daemon.lock"),
    };
    let mut local = {
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            if let Ok(Some(Connected::Ready(conn, _))) =
                micold_core::connect::connect(&endpoint, "ssh-real-local").await
            {
                break conn;
            }
            assert!(Instant::now() < end, "the local daemon never answered");
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };
    let mut ignored = String::new();
    let added = request(
        &mut *local,
        ClientMsg::ProjectAdd {
            req: 1,
            path: local_repo.clone(),
        },
        1,
        &mut ignored,
    )
    .await;
    assert_eq!(added, OperationResult::Ack);
    let session = create_session(&mut local, &local_repo, "loc", 2, &mut ignored).await;
    view(&mut local, &local_repo, session).await;
    let before = drain(&mut local, Duration::from_secs(3)).await;
    assert!(
        before.contains("TICK-"),
        "the local session ticks: {before}"
    );

    // The remote link is cut (the remote daemon holds no session here); the local session keeps ticking.
    host.proxy.cut();
    let _ = tokio::time::timeout(Duration::from_secs(10), remote.connection.next()).await;
    let during = drain(&mut local, Duration::from_secs(4)).await;
    let newest = |t: &str| {
        t.lines()
            .filter_map(|l| l.trim().strip_prefix("TICK-")?.trim().parse::<u32>().ok())
            .max()
    };
    assert!(
        newest(&during) > newest(&before),
        "the local session kept running through the cut: {during}"
    );
    local
        .send(Frame::Control(ClientMsg::Ping { nonce: 1 }))
        .await
        .expect("the local connection stays open");
    host.proxy.restore();
}

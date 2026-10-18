//! Reconnect after a network drop over a real `ssh` (feature 489, T044, FR-007 to FR-010, SC-002):
//! the link is cut at a fault proxy between the client's `ssh` and the stand-in `sshd`; the remote
//! daemon and its session keep running, and the client's dial loop (the core `connect` and
//! `Backoff`) restores the connection within 30 s to the same daemon with the output intact.
//!
//! Off by default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason
//! when `sshd` is not installed.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{Connected, DaemonConnection};
use micold_core::daemons::{DaemonEvent, DaemonId, DaemonState, DaemonStates, ATTEMPTS_STOPPED};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, OperationResult, SessionProcess, TerminalRef};
use micold_core::session::{AiCli, SessionId};
use micold_core::ssh::reconnect::Backoff;
use micold_core::worktree::CreateMode;
use ssh_real_support::remote::{request, start, Dialer};

/// The numbers `n` of every `TICK-n` line in `text`.
fn ticks(text: &str) -> BTreeSet<u32> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("TICK-")?.trim().parse().ok())
        .collect()
}

/// Read grid frames for `span`, appending their text to `seen`.
async fn drain(conn: &mut DaemonConnection, seen: &mut String, span: Duration) {
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
}

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

/// Dial with the client's backoff until the host answers; `cut_until` is the attempt number after
/// whose failure the network comes back. Returns the connection and the attempts it took.
async fn redial(
    dialer: &Dialer,
    proxy: &ssh_real_support::proxy::Proxy,
    states: &mut DaemonStates,
    id: DaemonId,
    restore_after_failures: u32,
) -> (Box<DaemonConnection>, u32) {
    let mut backoff = Backoff::new();
    let mut failures = 0;
    loop {
        match dialer.connect(None).await {
            Ok(Connected::Ready(connection, _)) => {
                states.apply(id, DaemonEvent::Connected);
                return (connection, failures + 1);
            }
            Ok(Connected::Refused(_)) => panic!("the daemon refused the handshake"),
            Err(e) => {
                failures += 1;
                states.apply(id, DaemonEvent::DialFailed(format!("{e:?}")));
                assert!(
                    matches!(states.state(id), Some(DaemonState::Reconnecting { .. })),
                    "a failed attempt keeps the daemon reconnecting"
                );
                if failures >= restore_after_failures {
                    proxy.restore();
                }
                tokio::time::sleep(backoff.next_delay()).await;
            }
        }
    }
}

#[tokio::test]
async fn ssh_real_cut_link_reconnects_to_the_same_daemon_with_output_intact() {
    let name = "ssh_real_cut_link_reconnects_to_the_same_daemon_with_output_intact";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let repo = ssh_real_support::remote::stand_in_repo(
        &host,
        "#!/bin/sh\ni=1\nwhile true; do echo TICK-$i; i=$((i+1)); sleep 1; done\n",
    );
    let Some(mut started) = start(&host, name).await else {
        return;
    };
    let id = DaemonId(1);
    let mut states = DaemonStates::default();
    states.insert(id, DaemonState::Starting);
    states.apply(id, DaemonEvent::Connected);

    let mut seen = String::new();
    {
        let conn = &mut *started.connection;
        let ok = request(
            conn,
            ClientMsg::ProjectAdd {
                req: 1,
                path: repo.clone(),
            },
            1,
            &mut seen,
        )
        .await;
        assert_eq!(ok, OperationResult::Ack);
        let created = request(
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
        assert!(matches!(created, OperationResult::WorktreeCreated { .. }));
    }
    let OperationResult::SessionCreated { session } = request(
        &mut *started.connection,
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
    let conn = &mut *started.connection;
    conn.send(Frame::Control(ClientMsg::SessionStart {
        session,
        allow_additional_session: true,
    }))
    .await
    .expect("start the session");
    view(conn, &repo, session).await;
    drain(conn, &mut seen, Duration::from_secs(4)).await;
    let before = ticks(&seen);
    assert!(
        before.len() >= 2,
        "the session ticks before the cut: {seen}"
    );
    let last_before = *before.iter().max().unwrap();

    // The network drops: ssh sees the connection die, sshd and the daemon do not.
    let pid_before = started.pid.expect("the bridge started a daemon");
    host.proxy.cut();
    let cut_at = Instant::now();
    states.apply(id, DaemonEvent::Lost("connection closed".into()));
    assert!(matches!(
        states.state(id),
        Some(DaemonState::Reconnecting { .. })
    ));
    drop(started.connection);

    // The client's loop: backoff 1, 2, ... until the network is back (restored after 2 failures).
    let (mut conn, attempts) = redial(&started.dialer, &host.proxy, &mut states, id, 2).await;
    let took = cut_at.elapsed();
    assert!(attempts >= 3, "the cut was felt: {attempts} attempts");
    assert!(took < Duration::from_secs(30), "SC-002: back in {took:?}");
    assert_eq!(states.state(id), Some(&DaemonState::Connected));

    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    assert_eq!(
        micold_core::spawn::running_daemon_pid(&endpoint),
        Some(pid_before),
        "the same daemon process serves the new connection"
    );

    // A fresh connection finds the session, running on, its output unbroken.
    let mut after = String::new();
    view(&mut conn, &repo, session).await;
    drain(&mut conn, &mut after, Duration::from_secs(4)).await;
    let now = ticks(&after);
    let last_after = *now.iter().max().expect("the session shows output again");
    assert!(
        last_after > last_before,
        "the session kept running through the cut: {last_before} -> {last_after}"
    );
    assert!(
        (last_before..=last_after).all(|n| now.contains(&n)),
        "output is contiguous across the cut: {now:?}"
    );

    // Stop trying: the daemon is left unreachable and not dialled; Reconnect dials at once.
    host.proxy.cut();
    states.apply(id, DaemonEvent::Lost("connection closed".into()));
    drop(conn);
    states.apply(id, DaemonEvent::GiveUp);
    assert_eq!(
        states.state(id),
        Some(&DaemonState::Unreachable {
            reason: ATTEMPTS_STOPPED.into()
        })
    );
    assert!(!states.should_dial(id), "no attempt after Stop trying");
    let epoch = states.epoch(id);
    host.proxy.restore();
    states.apply(id, DaemonEvent::Reconnect);
    assert_eq!(states.epoch(id), epoch + 1);
    assert!(states.should_dial(id));
    let Ok(Connected::Ready(_conn, _)) = started.dialer.connect(None).await else {
        panic!("Reconnect reaches the daemon again")
    };
    states.apply(id, DaemonEvent::Connected);
    assert_eq!(states.state(id), Some(&DaemonState::Connected));
}

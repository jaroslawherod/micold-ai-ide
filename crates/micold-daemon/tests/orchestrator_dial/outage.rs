//! User Story 4 (#1725, FR-008, SC-003): the orchestrator stops or restarts and nothing a daemon
//! hosts notices. Its sessions keep producing output, its clients stay connected, it logs the loss
//! once, and it is `online` again by itself within one capped reconnect delay of the return. A
//! restart while linked is a re-dial accepted as the restored record, never `AlreadyLinked`.
//!
//! The clock is paused, so the 30 s outage and the reconnect delays cost nothing; the session is a
//! real PTY writing in real time, and every wait is bounded in real time by `within`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::orchestrator::LinkStatus;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::{spawn, LinkHandle};
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

use crate::conn::{connect_as, Client};
use crate::support::{record_when, within, Orchestrator};
use crate::{link_to, Captured};

/// SC-003's outage.
const OUTAGE: Duration = Duration::from_secs(30);

/// SC-003's bound on the return: the capped reconnect delay (15 s), plus a second for the dial.
const BACK_ONLINE_WITHIN: Duration = Duration::from_secs(16);

async fn status_is(handle: &LinkHandle, want: LinkStatus) {
    within("the status settles", async {
        while handle.status() != want {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

/// A shell that prints `tick_1`, `tick_2`, … every 50 ms, forever.
fn counting() -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("i=0; while :; do i=$((i+1)); echo tick_$i; sleep 0.05; done");
    cmd.cwd(std::env::temp_dir());
    cmd
}

/// The highest `tick_<n>` on a grid frame's lines.
fn highest_tick(frame: &micold_core::protocol::grid::GridFrame) -> Option<u64> {
    frame
        .lines
        .iter()
        .filter_map(|line| line.text.trim().strip_prefix("tick_")?.parse().ok())
        .max()
}

/// Read grid frames until one shows a tick above `above`; its tick. Panics if the client's
/// connection closes or nothing new arrives in real time.
async fn tick_above(client: &mut Client, above: u64) -> u64 {
    within("the session's output reaches the client", async {
        loop {
            match client.next().await {
                Some(Ok(Frame::Grid(frame))) => {
                    if let Some(tick) = highest_tick(&frame).filter(|&t| t > above) {
                        return tick;
                    }
                }
                Some(Ok(_)) => {}
                other => panic!("the client's connection ended: {other:?}"),
            }
        }
    })
    .await
}

/// Scenario 1, SC-003: with a session producing output and a client viewing it, a 30 s outage
/// ends no session, closes no client and is logged once.
#[tokio::test(start_paused = true)]
async fn an_orchestrator_outage_changes_nothing_for_a_session_or_its_client() {
    let captured = Captured::for_this_thread();

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let sid = SessionId::new();
    let session = PtySession::spawn(sid, counting(), 1_000, Some((80, 24))).expect("spawn");
    let pty = state.register_session(session);
    let mut client = connect_as(&state, "outage-client").await;
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: PathBuf::from("/repo/demo"),
            session: Some(sid),
        }))
        .await
        .expect("viewed");
    let before = tick_above(&mut client, 0).await;

    let orchestrator = Orchestrator::start().await;
    let handle = spawn(&link_to(&orchestrator.socket)).expect("enabled spawns a task");
    status_is(&handle, LinkStatus::Linked).await;

    let _dir = orchestrator.stop().await;
    tokio::time::sleep(OUTAGE).await;

    assert_eq!(handle.status(), LinkStatus::NotLinked);
    assert!(pty.is_alive(), "the session ended during the outage");
    assert!(
        state.primary_pty(sid).is_some(),
        "the session left the daemon"
    );
    assert_eq!(state.client_count(), 1, "the client was disconnected");
    tick_above(&mut client, before).await;
    let log = captured.text();
    assert_eq!(
        log.matches("orchestrator link lost").count(),
        1,
        "the loss is logged once: {log}"
    );
    pty.kill().expect("the session is stopped");
}

/// Scenario 2, SC-003: after 30 s down, the daemon is `online` within 16 s of the return, with no
/// restart.
#[tokio::test(start_paused = true)]
async fn a_daemon_is_online_within_16_s_of_the_orchestrator_returning() {
    let orchestrator = Orchestrator::start().await;
    let handle = spawn(&link_to(&orchestrator.socket)).expect("enabled spawns a task");
    record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;

    let dir = orchestrator.stop().await;
    tokio::time::sleep(OUTAGE).await;
    let returned = tokio::time::Instant::now();
    let orchestrator = Orchestrator::start_in(dir).await;

    status_is(&handle, LinkStatus::Linked).await;
    let waited = returned.elapsed();
    assert!(
        waited <= BACK_ONLINE_WITHIN,
        "online {waited:?} after the return"
    );
    record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;
    assert!(!handle.is_finished(), "the link task ended");
}

/// Scenario 3: an orchestrator restart while linked is a re-dial accepted as the restored record:
/// never refused as already linked, never a second record.
#[tokio::test(start_paused = true)]
async fn a_restart_while_linked_is_accepted_as_the_restored_record() {
    let orchestrator = Orchestrator::start().await;
    let handle = spawn(&link_to(&orchestrator.socket)).expect("enabled spawns a task");
    let mut statuses = handle.watch();
    let seen = tokio::spawn(async move {
        let mut seen = Vec::new();
        while statuses.changed().await.is_ok() {
            seen.push(statuses.borrow_and_update().clone());
        }
        seen
    });
    let first = record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;
    status_is(&handle, LinkStatus::Linked).await;

    let orchestrator = orchestrator.restart().await;
    // The restored record is listed before the daemon is back; the re-dial brings it online.
    within("the daemon re-dials", async {
        while handle.status() == LinkStatus::Linked {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    status_is(&handle, LinkStatus::Linked).await;
    let again = record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;

    assert_eq!(orchestrator.daemons().await.len(), 1, "a duplicate record");
    for field in [
        "id",
        "kind",
        "name",
        "protocol_version",
        "tenant",
        "capabilities",
    ] {
        assert_eq!(again[field], first[field], "{field}");
    }
    handle.abort();
    let seen = seen.await.expect("the watcher ends with the task");
    assert!(
        !seen
            .iter()
            .any(|s| matches!(s, LinkStatus::Refused(_) | LinkStatus::HelloRefused(_))),
        "{seen:?}"
    );
}

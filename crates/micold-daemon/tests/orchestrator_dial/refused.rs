//! User Story 5 (#1725, FR-007, FR-012, SC-004): a refused daemon retries politely. Whatever the
//! orchestrator refuses (`VersionMismatch`, `AlreadyLinked`, `Hello`), the next attempt waits the
//! capped delay with its jitter, never less; an older daemon is listed `needs-update` meanwhile;
//! a link that was up and then lost is re-dialled after one second; the log has one line per
//! status change and none per retry.
//!
//! The clock is paused, so a 60 s run costs nothing; every wait is bounded in real time by
//! `within`.

use std::time::Duration;

use micold_core::orchestrator::dial::MAX_JITTER;
use micold_core::orchestrator::{LinkStatus, LinkVerdict, RefuseReason};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_core::ssh::reconnect::MAX_DELAY;
use micold_daemon::orchestrator_link::{announcement, spawn, spawn_as};
use tokio::time::Instant;

use crate::support::{announcement as peer_announcement, record_when, FakeDaemon, Orchestrator};
use crate::{link_to, status_is, Captured, FakeOrchestrator, Incoming};

/// SC-004's run.
const RUN: Duration = Duration::from_secs(60);

/// Slack for the dial itself on a paused clock.
const SLACK: Duration = Duration::from_millis(50);

fn refusal(reason: RefuseReason) -> LinkVerdict {
    LinkVerdict::Refused {
        reason,
        orchestrator_version: PROTOCOL_VERSION,
        daemon_version: PROTOCOL_VERSION,
        message: format!("refused for {reason:?}"),
    }
}

/// Refuse one attempt the `turn`th way: the announcement for a version, the announcement for an
/// id already linked, or `Hello` after `Accepted`. The status the daemon then reports.
async fn refuse(incoming: Incoming, turn: usize) -> LinkStatus {
    let mut incoming = incoming;
    match turn % 3 {
        0 => {
            incoming
                .verdict(&refusal(RefuseReason::VersionMismatch))
                .await;
            LinkStatus::Refused(RefuseReason::VersionMismatch)
        }
        1 => {
            incoming
                .verdict(&refusal(RefuseReason::AlreadyLinked))
                .await;
            LinkStatus::Refused(RefuseReason::AlreadyLinked)
        }
        _ => {
            let mut wire = incoming.accepted().await;
            assert!(matches!(wire.next().await, ClientMsg::Hello { .. }));
            let reason = RefusalReason::BuildMismatch {
                client_build: "dialer".into(),
                daemon_build: "orchestrator".into(),
            };
            wire.send(DaemonMsg::Refused {
                reason: reason.clone(),
            })
            .await;
            LinkStatus::HelloRefused(format!("{reason:?}"))
        }
    }
}

/// Scenarios 1-2, SC-004: refused every time, each way twice in a row, the daemon tries at most
/// once per capped delay over 60 s, and logs each change of refusal once, never the repeat.
#[tokio::test(start_paused = true)]
async fn a_daemon_refused_every_time_tries_at_most_once_per_capped_delay() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let captured = Captured::for_this_thread();
    let handle = spawn(&link_to(&socket)).unwrap();

    let started = Instant::now();
    let mut attempts = Vec::new();
    for turn in 0.. {
        let incoming = fake.accept().await;
        if started.elapsed() > RUN {
            break;
        }
        attempts.push(Instant::now());
        let want = refuse(incoming, turn / 2).await;
        status_is(&handle, want).await;
    }

    let longest = MAX_DELAY.mul_f64(1.0 + MAX_JITTER) + SLACK;
    for pair in attempts.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= MAX_DELAY && gap <= longest,
            "{gap:?} between attempts"
        );
    }
    let most = 1 + (RUN.as_secs() / MAX_DELAY.as_secs()) as usize;
    assert!(
        (4..=most).contains(&attempts.len()),
        "{} attempts in {RUN:?}",
        attempts.len()
    );
    let refused = captured.text().matches("orchestrator link refused").count();
    assert_eq!(
        refused,
        attempts.len().div_ceil(2),
        "one line per change:\n{}",
        captured.text()
    );
    assert!(!handle.is_finished());
}

/// Scenario 1, against the real orchestrator: a daemon of another protocol version is listed
/// `needs-update` while it retries at the capped delay, and logs the refusal once.
#[tokio::test(start_paused = true)]
async fn an_older_daemon_is_listed_needs_update_while_it_retries_at_the_capped_delay() {
    let orchestrator = Orchestrator::start().await;
    let captured = Captured::for_this_thread();
    let config = link_to(&orchestrator.socket);
    let mut older = announcement(&config);
    older.protocol_version = PROTOCOL_VERSION - 1;
    let handle = spawn_as(&config, older).expect("enabled spawns a task");

    status_is(&handle, LinkStatus::Refused(RefuseReason::VersionMismatch)).await;
    let record = record_when(&orchestrator, "hostbox", |r| r["health"] == "needs-update").await;
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION - 1);
    assert_eq!(handle.state().attempts, 1);
    assert!(
        handle
            .state()
            .detail
            .contains(&(PROTOCOL_VERSION - 1).to_string()),
        "{:?}",
        handle.state()
    );

    tokio::time::sleep(RUN).await;
    let attempts = handle.state().attempts;
    let fewest = 1 + (RUN.as_secs_f64() / MAX_DELAY.mul_f64(1.0 + MAX_JITTER).as_secs_f64()) as u64;
    let most = 1 + RUN.as_secs() / MAX_DELAY.as_secs();
    assert!(
        (fewest..=most).contains(&attempts),
        "{attempts} attempts in {RUN:?}"
    );
    assert_eq!(
        handle.status(),
        LinkStatus::Refused(RefuseReason::VersionMismatch)
    );
    let record = record_when(&orchestrator, "hostbox", |r| r["health"] == "needs-update").await;
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION - 1);
    let log = captured.text();
    assert_eq!(
        log.matches("orchestrator link refused").count(),
        1,
        "no line per retry:\n{log}"
    );
}

/// Scenario 2, against the real orchestrator: another link holds the id; the daemon is refused
/// `AlreadyLinked` and the first link stays online, alone.
#[tokio::test]
async fn a_refused_duplicate_never_disturbs_the_first_link() {
    let orchestrator = Orchestrator::start().await;
    let _first = FakeDaemon::link(&orchestrator, &peer_announcement("hostbox")).await;
    let handle = spawn(&link_to(&orchestrator.socket)).unwrap();

    status_is(&handle, LinkStatus::Refused(RefuseReason::AlreadyLinked)).await;
    let record = record_when(&orchestrator, "hostbox", |r| r["health"] == "online").await;
    assert_eq!(record["kind"], "container", "still the first link's record");
    assert_eq!(orchestrator.daemons().await.len(), 1);
}

/// Scenario 3: once the failure backoff has grown, a link that comes up and is then lost is
/// re-dialled after one second (plus its jitter), not after the grown delay.
#[tokio::test(start_paused = true)]
async fn the_first_redial_after_a_lost_link_comes_after_one_second() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let handle = spawn(&link_to(&socket)).unwrap();
    // Nothing listens: 1, 2, 4, 8, 15 s, so the backoff is at its cap.
    tokio::time::sleep(Duration::from_secs(40)).await;

    let fake = FakeOrchestrator::bind(&socket);
    let wire = fake.accept().await.link().await;
    status_is(&handle, LinkStatus::Linked).await;
    drop(wire);
    let lost = Instant::now();
    let _again = fake.accept().await;
    let gap = lost.elapsed();
    let one = Duration::from_secs(1);
    assert!(
        gap >= one && gap <= one.mul_f64(1.0 + MAX_JITTER) + SLACK,
        "re-dialled {gap:?} after the loss"
    );
}

/// FR-012: a refused daemon whose orchestrator goes away logs that change once too.
#[tokio::test(start_paused = true)]
async fn a_refusal_followed_by_an_absent_orchestrator_is_logged_once() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let captured = Captured::for_this_thread();
    let handle = spawn(&link_to(&socket)).unwrap();
    let want = refuse(fake.accept().await, 1).await;
    status_is(&handle, want).await;
    drop(fake);
    let _ = std::fs::remove_file(&socket);

    status_is(&handle, LinkStatus::NotLinked).await;
    tokio::time::sleep(RUN).await;
    let log = captured.text();
    assert_eq!(
        log.matches("orchestrator link not linked").count(),
        1,
        "{log}"
    );
    assert_eq!(handle.status(), LinkStatus::NotLinked);
}

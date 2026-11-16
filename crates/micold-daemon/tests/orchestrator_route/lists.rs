//! FR-001, FR-002, SC-001, US1: one list request through the orchestrator returns every linked
//! daemon's rows, kept apart by daemon, and names a daemon that is stopped (`offline`) or hung
//! (`timeout`) within 2.5 s.

use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::protocol::messages::{DaemonMsg, ListRows};
use micold_core::protocol::routing::{ListKind, MissingDaemon, MissingReason};
use micold_daemon::state::DaemonState;

use crate::support::{announcement, record_when, within, FakeDaemon, Orchestrator};
use crate::{shell_catalog, Client, Daemon};

/// Daemon `id` with one project holding one shell session; the temp dirs live as long as it.
async fn with_project(
    orchestrator: &Orchestrator,
    id: &str,
) -> (Daemon, tempfile::TempDir, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (mut catalog, _) = shell_catalog(project.path(), store.path());
    // A daemon started with its config stamps its rows with its own id.
    catalog.set_daemon_id(id.parse().unwrap());
    let daemon = Daemon::link_state(orchestrator, id, Arc::new(DaemonState::new(catalog)));
    daemon.listed_online(orchestrator).await;
    (daemon, project, store)
}

fn listed(msg: DaemonMsg) -> (ListRows, Vec<MissingDaemon>, bool) {
    match msg {
        DaemonMsg::OrchListed {
            rows,
            missing,
            no_daemons,
            ..
        } => (rows, missing, no_daemons),
        other => panic!("expected OrchListed, got {other:?}"),
    }
}

fn daemons_of(rows: &ListRows) -> Vec<String> {
    match rows {
        ListRows::Projects(p) => p.iter().map(|p| p.daemon.to_string()).collect(),
        ListRows::Sessions(s) => s.iter().map(|s| s.daemon.to_string()).collect(),
        ListRows::Worktrees(w) => w.iter().map(|w| w.daemon.to_string()).collect(),
    }
}

#[tokio::test]
async fn a_list_holds_every_daemons_rows_and_names_a_stopped_one_offline() {
    let orchestrator = Orchestrator::start().await;
    let (alpha, _p1, _s1) = with_project(&orchestrator, "alpha").await;
    let (_beta, _p2, _s2) = with_project(&orchestrator, "beta").await;
    let mut client = Client::connect(&orchestrator).await;

    let (rows, missing, no_daemons) = listed(client.list(1, 1, ListKind::Projects).await);
    assert_eq!(daemons_of(&rows), ["alpha", "beta"], "{rows:?}");
    assert!(missing.is_empty(), "{missing:?}");
    assert!(!no_daemons);
    let (rows, missing, _) = listed(client.list(1, 2, ListKind::Sessions).await);
    assert_eq!(daemons_of(&rows), ["alpha", "beta"], "{rows:?}");
    assert!(missing.is_empty(), "{missing:?}");

    // Once the orchestrator sees the link close, alpha is `offline` without being asked.
    alpha.drop_link();
    let (rows, missing) = within("alpha is offline in the list", async {
        for req in 3.. {
            let (rows, missing, _) = listed(client.list(1, req, ListKind::Projects).await);
            if missing.iter().any(|m| m.reason == MissingReason::Offline) {
                return (rows, missing);
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        unreachable!()
    })
    .await;
    assert_eq!(daemons_of(&rows), ["beta"]);
    assert_eq!(
        missing,
        [MissingDaemon {
            daemon: "alpha".parse().unwrap(),
            reason: MissingReason::Offline,
        }]
    );
}

#[tokio::test]
async fn a_hung_daemon_is_named_timeout_within_the_deadline() {
    let orchestrator = Orchestrator::start().await;
    let (_beta, _p, _s) = with_project(&orchestrator, "beta").await;
    // Linked, never answers what is routed to it.
    let _hung = FakeDaemon::link(&orchestrator, &announcement("hung")).await;
    record_when(&orchestrator, "hung", |r| r["health"] == "online").await;
    let mut client = Client::connect(&orchestrator).await;

    let started = Instant::now();
    let (rows, missing, _) = listed(client.list(1, 1, ListKind::Projects).await);
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(2500),
        "answered after {took:?}"
    );
    assert_eq!(daemons_of(&rows), ["beta"]);
    assert_eq!(
        missing,
        [MissingDaemon {
            daemon: "hung".parse().unwrap(),
            reason: MissingReason::Timeout,
        }]
    );
}

#[tokio::test]
async fn no_daemon_linked_is_an_empty_list_not_an_error() {
    let orchestrator = Orchestrator::start().await;
    let mut client = Client::connect(&orchestrator).await;
    let (rows, missing, no_daemons) = listed(client.list(1, 1, ListKind::Worktrees).await);
    assert!(rows.is_empty() && missing.is_empty() && no_daemons);
}

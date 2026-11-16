//! User Story 1, scenario 2 (FR-001): the orchestrator hosts no session. Since #1726 it relays a
//! client's session requests to the owning daemon (`tests/orchestrator_route`) and answers none
//! itself.

use micold_core::protocol::messages::ClientMsg;

use crate::support::{announcement, child_count, children_quiet, FakeDaemon, Orchestrator};

#[tokio::test]
async fn no_child_process_is_spawned() {
    let _quiet = children_quiet().await;
    let orchestrator = Orchestrator::start().await;
    let mut link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    link.send(ClientMsg::Goodbye).await;
    drop(link);
    // The orchestrator builds no `DaemonState`, so nothing owns a PTY or a supervisor: its process
    // has no child after serving the link.
    assert_eq!(child_count(), 0);
    drop(orchestrator);
}

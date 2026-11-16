//! FR-010, US4: the orchestrator stops; the session keeps running on its daemon, the daemon's link
//! redials the restarted orchestrator, and a client then sees the daemon's own state.

use std::sync::Arc;
use std::time::Duration;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_daemon::state::DaemonState;

use crate::support::{within, Orchestrator};
use crate::{shell_catalog, Client, Daemon};

#[tokio::test]
async fn a_session_survives_the_orchestrator_and_the_link_redials() {
    let orchestrator = Orchestrator::start().await;
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, session) = shell_catalog(project.path(), store.path());
    let state = Arc::new(DaemonState::new(catalog));
    let daemon = Daemon::link_state(&orchestrator, "alpha", Arc::clone(&state));
    daemon.listed_online(&orchestrator).await;

    // Start the session through the orchestrator.
    let mut client = Client::connect(&orchestrator).await;
    client.open("alpha", 1, "c").await;
    client
        .send(
            "alpha",
            1,
            &ClientMsg::SessionStart {
                session,
                allow_additional_session: false,
            },
        )
        .await;
    within("the session is live", async {
        while state.live_session(session).is_none() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;

    // The orchestrator goes away and comes back.
    let orchestrator = orchestrator.restart().await;
    assert!(
        state.live_session(session).is_some(),
        "the session was lost"
    );
    // The daemon's link redials by itself.
    daemon.listed_online(&orchestrator).await;
    assert!(state.live_session(session).is_some());

    // A new client sees the daemon's own state through the new orchestrator.
    let mut again = Client::connect(&orchestrator).await;
    again.open("alpha", 1, "c2").await;
    again.send("alpha", 1, &ClientMsg::Ping { nonce: 1 }).await;
    match again.next_msg("alpha", 1).await {
        DaemonMsg::Pong { nonce: 1 } => {}
        other => panic!("expected Pong, got {other:?}"),
    }
    for pty in state.remove_session(session) {
        let _ = pty.kill();
    }
}

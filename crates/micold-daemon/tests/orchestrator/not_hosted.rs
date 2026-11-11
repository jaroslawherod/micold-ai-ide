//! User Story 1, scenario 2 (FR-001): the orchestrator hosts no session.

use std::path::PathBuf;

use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::session::AiCli;

use crate::support::{announcement, FakeDaemon, Orchestrator};

#[tokio::test]
async fn a_session_start_is_refused_not_hosted() {
    let orchestrator = Orchestrator::start().await;
    let mut link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    link.send(ClientMsg::SessionCreate {
        req: 7,
        project: PathBuf::from("/p"),
        worktree_dir: "w".into(),
        provider: AiCli::default(),
        allow_additional_session: false,
        kind: Default::default(),
        permission_mode: Default::default(),
    })
    .await;
    match link.next().await {
        DaemonMsg::OperationError {
            req, kind, message, ..
        } => {
            assert_eq!(req, 7);
            assert_eq!(kind, ErrorKind::Refused);
            assert_eq!(message, "not hosted by the orchestrator");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn a_session_list_is_empty() {
    let orchestrator = Orchestrator::start().await;
    let mut link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    link.send(ClientMsg::Attach {
        project: PathBuf::from("/p"),
        force: false,
    })
    .await;
    match link.next().await {
        DaemonMsg::Attached { sessions, .. } => assert!(sessions.is_empty()),
        other => panic!("expected an empty list, got {other:?}"),
    }
}

#[tokio::test]
async fn no_child_process_is_spawned() {
    let orchestrator = Orchestrator::start().await;
    let mut link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    link.send(ClientMsg::Goodbye).await;
    drop(link);
    // The orchestrator builds no `DaemonState`, so nothing owns a PTY or a supervisor: its process
    // has no child after serving the link.
    assert_eq!(child_count(), 0);
    drop(orchestrator);
}

#[cfg(target_os = "linux")]
fn child_count() -> usize {
    let me = std::process::id();
    std::fs::read_to_string(format!("/proc/{me}/task/{me}/children"))
        .map(|children| children.split_whitespace().count())
        .unwrap_or(0)
}

#[cfg(not(target_os = "linux"))]
fn child_count() -> usize {
    0
}

//! Bug 1423: each daemon keeps its own catalog, so a worktree create on a daemon that was never
//! told the project is refused, and one after `ProjectAdd` succeeds. The refusal says why.

#[path = "support/conn.rs"]
mod conn;
#[path = "support/mcp.rs"]
mod mcp;

use std::sync::Arc;

use futures_util::SinkExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, OperationResult};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

fn create(req: u64, project: &std::path::Path) -> ClientMsg {
    ClientMsg::WorktreeCreate {
        req,
        project: project.to_path_buf(),
        branch: "feat-x".into(),
        dir_name: "feat-x".into(),
        mode: micold_core::worktree::CreateMode::NewBranch,
        session: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_daemon_that_was_told_the_project_creates_the_worktree() {
    let project = tempfile::tempdir().unwrap();
    mcp::init_repo(project.path());
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = conn::connect(&state).await;

    client
        .send(Frame::Control(create(1, project.path())))
        .await
        .unwrap();
    match conn::answer(&mut client, 1).await {
        DaemonMsg::OperationError { kind, message, .. } => {
            assert_eq!(kind, ErrorKind::NotFound);
            assert!(
                message.contains("never added here"),
                "the refusal must say why, got {message:?}"
            );
        }
        other => panic!("an unregistered project must be refused, got {other:?}"),
    }

    client
        .send(Frame::Control(ClientMsg::ProjectAdd {
            req: 2,
            path: project.path().to_path_buf(),
        }))
        .await
        .unwrap();
    assert!(matches!(
        conn::answer(&mut client, 2).await,
        DaemonMsg::OperationOk { .. }
    ));
    assert!(state.project_repo(project.path()).is_some());

    client
        .send(Frame::Control(create(3, project.path())))
        .await
        .unwrap();
    match conn::answer(&mut client, 3).await {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { dir_name, .. },
            ..
        } => assert_eq!(dir_name, "feat-x"),
        other => panic!("a registered project must create the worktree, got {other:?}"),
    }
}

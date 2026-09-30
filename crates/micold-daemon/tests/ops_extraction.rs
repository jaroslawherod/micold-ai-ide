//! Feature 034 (research R9, FR-009; U142–U144): the worktree operations the sidebar sends as
//! protocol messages are callable without a client, with the same result.
//!
//! Each test runs the operation twice over two identical projects: once as the protocol message a
//! window sends, once through `ops` directly, and compares what each left behind — the catalog, the
//! provenance record and the directory on disk. `mutation_semantics.rs`, `mutation_atomicity.rs`
//! and `worktree_provenance_rpc.rs` pin the protocol side itself (U141).

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::Path;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use mcp_support::*;
use micold_core::naming::DerivedNames;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::worktree::CreateMode;
use micold_daemon::ops;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// One project with a store, as a window or the tool server would find it.
struct Side {
    state: Arc<DaemonState>,
    project: tempfile::TempDir,
    _store: tempfile::TempDir,
}

impl Side {
    fn new(worktrees: &[&str]) -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        for name in worktrees {
            add_worktree(project.path(), name);
        }
        let state = state_over(
            vec![(project.path().to_path_buf(), true, Vec::new())],
            store.path(),
        );
        state.refresh_worktrees(project.path());
        Self {
            state,
            project,
            _store: store,
        }
    }

    fn repo(&self) -> &Path {
        self.project.path()
    }

    /// What the operation left behind, in a form two projects can be compared by.
    fn outcome(&self) -> (Vec<(String, Option<String>, String)>, Vec<String>, Vec<String>) {
        self.state.refresh_worktrees(self.repo());
        let snapshot = self.state.catalog_snapshot();
        let project = snapshot
            .projects
            .iter()
            .find(|p| p.path == self.repo())
            .expect("the project is in the catalog");
        let worktrees = project
            .worktrees
            .iter()
            .map(|w| (w.dir_name.clone(), w.branch.clone(), w.display_name.clone()))
            .collect();
        let (records, _) = self.state.provenance(self.repo());
        let mut on_disk: Vec<String> = std::fs::read_dir(self.repo().join(".claude/worktrees"))
            .map(|dir| {
                dir.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        on_disk.sort();
        (worktrees, records.into_iter().collect(), on_disk)
    }
}

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
}

/// Send `msg` and wait for the reply to request `req`.
async fn request(state: &Arc<DaemonState>, req: u64, msg: ClientMsg) -> DaemonMsg {
    let mut client = connect(state).await;
    client.send(Frame::Control(msg)).await.unwrap();
    let wait = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(m @ DaemonMsg::OperationOk { req: r, .. })
                | Frame::Control(m @ DaemonMsg::OperationError { req: r, .. })
                    if r == req =>
                {
                    return m
                }
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), wait)
        .await
        .expect("the protocol reply arrives")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_without_a_client_matches_the_protocol_create() {
    let window = Side::new(&[]);
    let agent = Side::new(&[]);
    let reply = request(
        &window.state,
        1,
        ClientMsg::WorktreeCreate {
            req: 1,
            project: window.repo().to_path_buf(),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::NewBranch,
        },
    )
    .await;
    assert!(
        matches!(reply, DaemonMsg::OperationOk { .. }),
        "{reply:?}"
    );

    let created = ops::create_worktree(
        &agent.state,
        agent.repo().to_path_buf(),
        DerivedNames {
            dir_name: "feat-x".into(),
            branch: "feat/x".into(),
        },
        CreateMode::NewBranch,
        None,
    )
    .await;
    assert!(created.is_ok(), "{created:?}");
    assert_eq!(
        agent.outcome(),
        window.outcome(),
        "the same worktree, provenance record and directory as the dialog's create"
    );
    assert!(agent.outcome().1.contains(&"feat-x".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_without_a_client_matches_the_protocol_delete() {
    let window = Side::new(&["a", "b"]);
    let agent = Side::new(&["a", "b"]);
    let reply = request(
        &window.state,
        2,
        ClientMsg::WorktreeDelete {
            req: 2,
            project: window.repo().to_path_buf(),
            dir_name: "a".into(),
            stop_sessions: false,
            delete_branch: true,
        },
    )
    .await;
    assert!(
        matches!(reply, DaemonMsg::OperationOk { .. }),
        "{reply:?}"
    );

    let deleted = ops::delete_worktree(
        &agent.state,
        agent.repo().to_path_buf(),
        "a".into(),
        false,
        true,
    )
    .await;
    assert!(deleted.is_ok(), "{deleted:?}");
    assert_eq!(agent.outcome(), window.outcome());
    assert_eq!(
        agent.outcome().2,
        ["b"],
        "the deleted worktree's directory is gone"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rename_without_a_client_matches_the_protocol_rename() {
    let window = Side::new(&["a"]);
    let agent = Side::new(&["a"]);
    let reply = request(
        &window.state,
        3,
        ClientMsg::WorktreeRename {
            req: 3,
            project: window.repo().to_path_buf(),
            dir_name: "a".into(),
            display_name: "Alpha".into(),
        },
    )
    .await;
    assert!(
        matches!(reply, DaemonMsg::OperationOk { .. }),
        "{reply:?}"
    );

    let renamed = ops::rename_worktree(&agent.state, agent.repo(), "a", "Alpha");
    assert!(renamed.is_ok(), "{renamed:?}");
    assert_eq!(agent.outcome(), window.outcome());
    assert_eq!(agent.outcome().0[0].2, "Alpha");
}

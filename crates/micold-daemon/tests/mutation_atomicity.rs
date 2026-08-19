//! Phase 5 (US3) — mutating requests are **atomic** on the daemon, so a client that loses the reply
//! to a disconnect settles by reading authoritative state on reconnect, never on a guessed outcome
//! (T051; FR-031/035).
//!
//! The client-side half — resolving an in-flight `req` to an explicit *unknown* on disconnect — lives
//! in the client's `DaemonDisconnected` handler. What makes that safe is the property proven here: the
//! daemon applies each mutation to its durable catalog **before** it replies, so whether or not the
//! reply is delivered, the next connection's welcome snapshot reflects the true, completed outcome.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::CreateMode;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

fn init_git_repo(dir: &Path) {
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@t.test"],
        &["config", "user.name", "T"],
        &["commit", "-q", "--allow-empty", "-m", "root"],
    ] {
        assert!(Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap()
            .status
            .success());
    }
}

fn catalog_with_project(project_dir: &Path, store_dir: &Path) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &std::sync::Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
}

/// The first message must be the Welcome; return its catalog.
async fn welcome_catalog(client: &mut Client) -> CatalogSnapshot {
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { catalog, .. }) => catalog,
        other => panic!("expected Welcome, got {other:?}"),
    }
}

fn worktrees(snapshot: &CatalogSnapshot, project: &Path) -> Vec<String> {
    snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.worktrees.iter().map(|w| w.dir_name.clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_mutation_whose_reply_is_lost_is_visible_to_the_next_connection() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));

    // Client A submits a worktree create, then vanishes WITHOUT reading the OperationOk — modelling a
    // disconnect that loses the reply. (The client would resolve this `req` to "unknown".)
    {
        let mut a = connect(&state).await;
        let _ = welcome_catalog(&mut a).await;
        a.send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 1,
            project: project.path().to_path_buf(),
            branch: "feature/lost".into(),
            dir_name: "lost".into(),
            mode: CreateMode::NewBranch,
        }))
        .await
        .unwrap();
        // Drain frames until the worktree directory actually exists on disk — i.e. the daemon has run
        // the create — WITHOUT treating the OperationOk as the settling signal. Then drop `a`.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !project.path().join(".claude/worktrees/lost").exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "create did not complete"
            );
            tokio::select! {
                _ = a.next() => {}
                _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {}
            }
        }
    }

    // Client B connects fresh. The mutation was applied atomically before the (lost) reply, so B's
    // authoritative welcome snapshot reflects it — the client settles on truth, not a guess.
    let mut b = connect(&state).await;
    b.send(Frame::Control(ClientMsg::Attach {
        project: project.path().to_path_buf(),
        force: false,
    }))
    .await
    .unwrap();
    let _welcome = welcome_catalog(&mut b).await;
    // After attach the daemon refreshes worktrees from git and pushes a CatalogChanged; read until it
    // shows the created worktree (bounded).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut seen = false;
    while std::time::Instant::now() < deadline {
        match b.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { catalog })
                if worktrees(&catalog, project.path()).contains(&"lost".to_string()) =>
            {
                seen = true;
                break;
            }
            _ => {}
        }
    }
    assert!(
        seen,
        "the reconnecting client reads the completed worktree from authoritative state"
    );
}

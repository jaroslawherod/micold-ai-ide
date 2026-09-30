//! `ClientMsg::RemoteList` (feature 034, contracts/remote-list-rpc.md §2): the daemon answers a
//! project's own remote URLs, read-only, and refuses a non-repository exactly as it refuses
//! `BranchList`.
//!
//! Drives `server::serve_connection` over an in-memory duplex with a real `ClientCodec`, as
//! `mutation_semantics.rs` does, so the whole route — `spawn_blocking` git, error mapping — is
//! under test.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::git::GitRemote;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, OperationResult};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs")
        .status
        .success();
    assert!(ok, "git {args:?} failed");
}

/// A catalog holding one project at `project_dir`, a git repository or not.
fn catalog_with_project(project_dir: &Path, store_dir: &Path, is_git_repo: bool) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            is_git_repo,
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
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Send `msg` and return the reply correlated to `req`, skipping anything else.
async fn ask(client: &mut Client, req: u64, msg: ClientMsg) -> DaemonMsg {
    client.send(Frame::Control(msg)).await.unwrap();
    let wait = async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(
                    m @ (DaemonMsg::OperationOk { req: r, .. }
                    | DaemonMsg::OperationError { req: r, .. }),
                ) if r == req => return m,
                _ => continue,
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), wait)
        .await
        .expect("the reply arrives")
}

#[tokio::test]
async fn a_repository_answers_its_remotes() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    git(project.path(), &["init", "-q"]);
    git(
        project.path(),
        &["remote", "add", "origin", "git@github.com:o/r.git"],
    );
    git(
        project.path(),
        &["remote", "add", "upstream", "https://gitlab.com/u/r.git"],
    );
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        true,
    )));
    let mut client = connect(&state).await;

    let reply = ask(
        &mut client,
        7,
        ClientMsg::RemoteList {
            req: 7,
            project: project.path().to_path_buf(),
        },
    )
    .await;
    match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::RemoteList { remotes },
            ..
        } => assert_eq!(
            remotes,
            vec![
                GitRemote {
                    name: "origin".into(),
                    url: "git@github.com:o/r.git".into(),
                },
                GitRemote {
                    name: "upstream".into(),
                    url: "https://gitlab.com/u/r.git".into(),
                },
            ],
            "both remotes, in config order, URLs as written"
        ),
        other => panic!("expected a RemoteList result, got {other:?}"),
    }
}

#[tokio::test]
async fn a_non_repository_is_rejected() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        false,
    )));
    let mut client = connect(&state).await;

    let remotes = ask(
        &mut client,
        1,
        ClientMsg::RemoteList {
            req: 1,
            project: project.path().to_path_buf(),
        },
    )
    .await;
    let branches = ask(
        &mut client,
        2,
        ClientMsg::BranchList {
            req: 2,
            project: project.path().to_path_buf(),
        },
    )
    .await;
    match (remotes, branches) {
        (
            DaemonMsg::OperationError {
                kind,
                message,
                detail,
                ..
            },
            DaemonMsg::OperationError {
                kind: branch_kind,
                message: branch_message,
                detail: branch_detail,
                ..
            },
        ) => {
            assert_eq!(
                (kind, message.as_str()),
                (ErrorKind::Refused, "project is not a git repository"),
                "a non-repository is refused, and the refusal says why"
            );
            assert_eq!(
                (kind, message, detail),
                (branch_kind, branch_message, branch_detail),
                "a non-repository is refused exactly as BranchList refuses it"
            );
        }
        other => panic!("both requests must be refused, got {other:?}"),
    }
}

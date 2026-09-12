//! Feature 029 — the user-initiated re-read, driven over a real codec
//! (contracts/worktree-refresh.md).
//!
//! `ClientMsg::WorktreeRefresh` mutates nothing, so what there is to prove is not a side effect but
//! an **order**: the refreshed listing goes out on `CatalogChanged` first, and the `Ack` that ends
//! the client's busy state follows it. A client that receives the ack has, by then, already
//! received the listing, which is what lets the completion notice say so honestly. The reverse
//! order would let the button return to idle while the stale list was still on screen — a one-frame
//! lie, and the kind that gets reported as a bug.
//!
//! So `expect_control`'s skip-until-match is deliberately *not* used for the reply pair here. The
//! frames are read one at a time, in order, because the order is the assertion.
//!
//! Drives `server::serve_connection` over an in-memory duplex exactly as `mutation_semantics` does,
//! so the whole `route()` path is under test — including the `spawn_blocking` git hop — rather than
//! a hand-rolled stand-in.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// Init a real git repo with one commit (so `HEAD` exists for `git worktree add`).
fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git runs")
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "t@t.test"]);
    run(&["config", "user.name", "T"]);
    run(&["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// Create a worktree the way something *other than this application* would: a plain `git worktree
/// add` in the repository, with the daemon neither asked nor told.
fn git_worktree_add(repo: &Path, dir_name: &str, branch: &str) {
    let target = repo.join(".claude/worktrees").join(dir_name);
    std::fs::create_dir_all(target.parent().unwrap()).expect("worktrees root");
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "add", "-b", branch])
        .arg(&target)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git worktree add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A catalog holding one git-repo project rooted at `project_dir`, persisted to `store_dir`.
fn catalog_with_project(project_dir: &Path, store_dir: &Path) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            true, // is_git_repo
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

/// Handshake a fresh client against `state`, draining the `Welcome`.
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

/// Handshake, then attach `project`, draining the `Attached` + `CatalogChanged` the attach
/// produces so later reads see only what the refresh caused.
async fn connect_and_attach(state: &std::sync::Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    expect_control(&mut client, |m| matches!(m, DaemonMsg::Attached { .. })).await;
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::CatalogChanged { .. })
    })
    .await;
    client
}

/// Read control frames until one matches `pred` (grid frames are skipped).
async fn expect_control(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

/// The very next control frame, whatever it is. Grid frames are skipped because they belong to a
/// different plane; control frames are not, because their order is what these tests assert.
async fn next_control(client: &mut Client) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) => return m,
            Frame::Grid(_) => continue,
        }
    }
}

/// The worktree directory names the daemon reports for `project` in a snapshot.
fn worktrees_for(snapshot: &CatalogSnapshot, project: &Path) -> Vec<String> {
    snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.worktrees.iter().map(|w| w.dir_name.clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn refresh_finds_a_worktree_created_behind_the_daemons_back_then_acks() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    // The attach's own refresh has already run and found nothing. This is the situation the whole
    // feature exists for: the repository changes, and nothing tells the application.
    assert!(
        worktrees_for(&state.catalog_snapshot(), project.path()).is_empty(),
        "precondition: the daemon knows of no worktrees yet"
    );
    git_worktree_add(project.path(), "outside", "feat/outside");
    assert!(
        worktrees_for(&state.catalog_snapshot(), project.path()).is_empty(),
        "precondition: nothing watches the repository, so the cache is still empty"
    );

    client
        .send(Frame::Control(ClientMsg::WorktreeRefresh {
            req: 1,
            project: project.path().to_path_buf(),
        }))
        .await
        .unwrap();

    // First the listing...
    match next_control(&mut client).await {
        DaemonMsg::CatalogChanged { catalog, .. } => assert!(
            worktrees_for(&catalog, project.path()).contains(&"outside".to_string()),
            "the refreshed listing must carry the worktree git already knows about"
        ),
        other => panic!("expected CatalogChanged before the ack, got {other:?}"),
    }
    // ...and only then the ack that ends the client's busy state (contracts §4).
    match next_control(&mut client).await {
        DaemonMsg::OperationOk { req, result } => {
            assert_eq!(req, 1);
            assert_eq!(result, OperationResult::Ack);
        }
        other => panic!("expected OperationOk after the listing, got {other:?}"),
    }
}

#[tokio::test]
async fn refresh_that_finds_nothing_new_still_acks() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::WorktreeRefresh {
            req: 2,
            project: project.path().to_path_buf(),
        }))
        .await
        .unwrap();

    // The common case — nothing changed — is still a completed operation, not a silent one. A
    // control that reports nothing when nothing changed is indistinguishable from a broken one
    // (029 US2), and the ack is what the client's completion notice is built on.
    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 2, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationOk { result, .. } => assert_eq!(result, OperationResult::Ack),
        other => unreachable!("filtered above: {other:?}"),
    }
}

#[tokio::test]
async fn refresh_of_a_project_that_is_no_longer_a_repository_empties_it_rather_than_failing() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;
    git_worktree_add(project.path(), "gone-soon", "feat/gone-soon");

    // Prime the cache so there is something to clear.
    client
        .send(Frame::Control(ClientMsg::WorktreeRefresh {
            req: 3,
            project: project.path().to_path_buf(),
        }))
        .await
        .unwrap();
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 3, .. })
    })
    .await;
    assert!(
        worktrees_for(&state.catalog_snapshot(), project.path()).contains(&"gone-soon".to_string()),
        "the first refresh must have found it, or the second one proves nothing"
    );

    // Now the repository stops being one. `refresh_worktrees` cannot fail — it clears the entry,
    // which is the correct answer to "what worktrees does this non-repository have" (contracts §3).
    // So the reply is still an ack, and the *listing* is what carries the bad news.
    std::fs::remove_dir_all(project.path().join(".git")).expect("remove .git");
    client
        .send(Frame::Control(ClientMsg::WorktreeRefresh {
            req: 4,
            project: project.path().to_path_buf(),
        }))
        .await
        .unwrap();
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 4, .. })
    })
    .await;
}

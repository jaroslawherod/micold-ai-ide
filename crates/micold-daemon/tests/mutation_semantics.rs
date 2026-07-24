//! Phase 5 (US3) — the mutating worktree RPCs run **through the daemon** with specific, actionable
//! failures and no side effects on failure (T050, T052; FR-034, W2).
//!
//! Drives `server::serve_connection` over an in-memory duplex with a real `ClientCodec`, exactly as
//! `handshake_flow` does, so the whole `route()` path — spawn_blocking git, error mapping, catalog
//! reconcile, broadcast — is under test, not a hand-rolled stand-in.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, ErrorKind, OperationResult,
};
use micold_core::protocol::version::{PROTOCOL_VERSION, SCHEMA_HASH};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use uuid::Uuid;

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

/// Pre-create a branch (for the duplicate-branch failure).
fn create_branch(repo: &Path, branch: &str) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["branch", branch])
        .output()
        .expect("git runs")
        .status
        .success();
    assert!(ok, "git branch {branch} failed");
}

/// A catalog holding one git-repo project rooted at `project_dir`, persisted to `store_dir`.
fn catalog_with_project(project_dir: &Path, store_dir: &Path, sessions: Vec<Session>) -> Catalog {
    let mut map = BTreeMap::new();
    if !sessions.is_empty() {
        map.insert(project_dir.to_path_buf(), sessions);
    }
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            true, // is_git_repo
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions: map,
        worktree_names: BTreeMap::new(),
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
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Handshake, then attach `project`, draining the `Attached` + `CatalogChanged` the attach produces
/// so later `next()`s see only the RPC reply.
async fn connect_and_attach(state: &std::sync::Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    // Attach → `Attached`, then `refresh_worktrees_and_broadcast` → `CatalogChanged`.
    expect_control(&mut client, |m| matches!(m, DaemonMsg::Attached { .. })).await;
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::CatalogChanged { .. })
    })
    .await;
    client
}

/// Read control frames until one matches `pred`, returning it (grid frames are skipped).
async fn expect_control(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

/// The worktrees the daemon reports for `project` in a `CatalogChanged`/welcome snapshot.
fn worktrees_for(snapshot: &CatalogSnapshot, project: &Path) -> Vec<String> {
    snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.worktrees.iter().map(|w| w.dir_name.clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn worktree_create_duplicate_branch_is_rejected_without_side_effects() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    create_branch(project.path(), "feature/dup");

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 1,
            project: project.path().to_path_buf(),
            branch: "feature/dup".into(),
            dir_name: "dup".into(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 1, .. })
    })
    .await;
    match reply {
        // A duplicate branch is caught pre-flight → the specific, actionable AlreadyExists (not a
        // generic GitFailed): no git mutation was attempted, so there is nothing to roll back.
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::AlreadyExists),
        other => panic!("expected OperationError, got {other:?}"),
    }

    // No leftover directory and no catalog entry (FR-034): the failed create left no trace.
    let target = project.path().join(".claude/worktrees/dup");
    assert!(
        !target.exists(),
        "no leftover worktree directory on failure"
    );
    let (snapshot, _) = state.welcome_payload();
    assert!(
        !worktrees_for(&snapshot, project.path()).contains(&"dup".to_string()),
        "no worktree recorded on failure"
    );
}

#[tokio::test]
async fn worktree_create_git_failure_reports_stderr_and_leaves_no_dir() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    // `bad..branch` passes the duplicate-branch pre-flight (it does not exist) but git rejects it as
    // an invalid ref during `worktree add` → a genuine git failure whose stderr must ride along.
    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 7,
            project: project.path().to_path_buf(),
            branch: "bad..branch".into(),
            dir_name: "bad".into(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 7, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationError { kind, detail, .. } => {
            assert_eq!(kind, ErrorKind::GitFailed);
            let detail = detail.expect("git stderr preserved");
            assert!(!detail.trim().is_empty(), "stderr detail is non-empty");
        }
        other => panic!("expected OperationError, got {other:?}"),
    }
    assert!(
        !project.path().join(".claude/worktrees/bad").exists(),
        "the failed create rolled back its directory"
    );
}

#[tokio::test]
async fn worktree_create_succeeds_and_propagates() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 2,
            project: project.path().to_path_buf(),
            branch: "feature/new".into(),
            dir_name: "new".into(),
        }))
        .await
        .unwrap();

    // The op resolves to exactly one OperationOk, and the change propagates as a CatalogChanged
    // naming the new worktree (a second window would observe it the same way).
    let ok = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 2, .. })
    })
    .await;
    assert!(matches!(
        ok,
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { .. },
            ..
        }
    ));
    assert!(
        project.path().join(".claude/worktrees/new").exists(),
        "the worktree directory was created"
    );
    let (snapshot, _) = state.welcome_payload();
    assert!(
        worktrees_for(&snapshot, project.path()).contains(&"new".to_string()),
        "the new worktree is discoverable in the catalog snapshot"
    );
}

#[tokio::test]
async fn worktree_delete_with_live_session_and_no_stop_is_refused() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    // A durable Regular (shell) session bound to the "wt1" worktree.
    let sid = SessionId::from_uuid(Uuid::from_u128(0x77));
    let session = Session::restored(
        sid,
        SessionLocation::Worktree("wt1".into()),
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![session],
    )));

    // Create the worktree on disk so the shell has a cwd, then bring the session to life.
    let ok = Command::new("git")
        .arg("-C")
        .arg(project.path())
        .args([
            "worktree",
            "add",
            "-b",
            "wt1",
            ".claude/worktrees/wt1",
            "HEAD",
        ])
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git worktree add for the fixture failed");
    state
        .start_session(sid, micold_core::terminal::LaunchMode::Resume)
        .expect("start the worktree's shell session");
    assert!(state.live_session(sid).is_some(), "session is live");

    let mut client = connect_and_attach(&state, project.path()).await;
    client
        .send(Frame::Control(ClientMsg::WorktreeDelete {
            req: 3,
            project: project.path().to_path_buf(),
            dir_name: "wt1".into(),
            stop_sessions: false,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 3, .. })
    })
    .await;
    match reply {
        // W2: the delete fails specifically rather than orphaning the live process.
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::Busy),
        other => panic!("expected OperationError::Busy, got {other:?}"),
    }
    // Untouched: the worktree survives on disk and the session is still live (not archived/killed).
    assert!(
        project.path().join(".claude/worktrees/wt1").exists(),
        "worktree not removed on a refused delete"
    );
    assert!(
        state.live_session(sid).is_some(),
        "session left running on a refused delete"
    );
}

#[tokio::test]
async fn worktree_delete_with_stop_sessions_archives_and_removes() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let sid = SessionId::from_uuid(Uuid::from_u128(0x78));
    let session = Session::restored(
        sid,
        SessionLocation::Worktree("wt2".into()),
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![session],
    )));
    let ok = Command::new("git")
        .arg("-C")
        .arg(project.path())
        .args([
            "worktree",
            "add",
            "-b",
            "wt2",
            ".claude/worktrees/wt2",
            "HEAD",
        ])
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git worktree add for the fixture failed");
    state
        .start_session(sid, micold_core::terminal::LaunchMode::Resume)
        .expect("start the worktree's shell session");

    let mut client = connect_and_attach(&state, project.path()).await;
    client
        .send(Frame::Control(ClientMsg::WorktreeDelete {
            req: 4,
            project: project.path().to_path_buf(),
            dir_name: "wt2".into(),
            stop_sessions: true,
        }))
        .await
        .unwrap();

    let ok = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 4, .. })
    })
    .await;
    assert!(matches!(
        ok,
        DaemonMsg::OperationOk {
            result: OperationResult::Ack,
            ..
        }
    ));
    assert!(
        !project.path().join(".claude/worktrees/wt2").exists(),
        "the worktree directory was removed"
    );
    assert!(
        state.live_session(sid).is_none(),
        "the worktree's session was stopped"
    );
    // Archived, not resurrectable: the durable session is filtered out of the snapshot.
    let (snapshot, _) = state.welcome_payload();
    let has_session = snapshot
        .projects
        .iter()
        .find(|p| p.path == project.path())
        .map(|p| p.sessions.iter().any(|s| s.id == sid))
        .unwrap_or(false);
    assert!(
        !has_session,
        "the deleted worktree's session is archived out"
    );
}

/// An empty (no-projects) catalog persisted to `store_dir`, for the ProjectAdd path.
fn empty_catalog(store_dir: &Path) -> Catalog {
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace::empty())
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

/// Whether the snapshot knows `project` at all.
fn has_project(snapshot: &CatalogSnapshot, project: &Path) -> bool {
    snapshot.projects.iter().any(|p| p.path == project)
}

#[tokio::test]
async fn project_add_makes_it_discoverable() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(empty_catalog(store.path())));
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::ProjectAdd {
            req: 10,
            path: project.path().to_path_buf(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 10, .. })
    })
    .await;
    assert!(matches!(
        reply,
        DaemonMsg::OperationOk {
            result: OperationResult::Ack,
            ..
        }
    ));
    let (snapshot, _) = state.welcome_payload();
    assert!(
        has_project(&snapshot, project.path()),
        "the added project is in the catalog"
    );
}

#[tokio::test]
async fn project_rename_rejects_blank_name() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::ProjectRename {
            req: 11,
            path: project.path().to_path_buf(),
            display_name: "   ".into(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 11, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::InvalidInput),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[tokio::test]
async fn session_delete_archives_and_stops() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();

    let sid = SessionId::from_uuid(Uuid::from_u128(0x5D));
    let session = Session::restored(
        sid,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![session],
    )));
    state
        .start_session(sid, micold_core::terminal::LaunchMode::Resume)
        .expect("start the shell session");
    assert!(state.live_session(sid).is_some());

    let mut client = connect_and_attach(&state, project.path()).await;
    client
        .send(Frame::Control(ClientMsg::SessionDelete {
            req: 12,
            session: sid,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 12, .. })
    })
    .await;
    assert!(matches!(
        reply,
        DaemonMsg::OperationOk {
            result: OperationResult::Ack,
            ..
        }
    ));
    assert!(state.live_session(sid).is_none(), "the session was stopped");
    let (snapshot, _) = state.welcome_payload();
    let still_listed = snapshot
        .projects
        .iter()
        .find(|p| p.path == project.path())
        .map(|p| p.sessions.iter().any(|s| s.id == sid))
        .unwrap_or(false);
    assert!(
        !still_listed,
        "the deleted session is archived out of the snapshot"
    );
}

#[tokio::test]
async fn session_delete_unknown_id_is_not_found() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::SessionDelete {
            req: 13,
            session: SessionId::from_uuid(Uuid::from_u128(0xDEAD)),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 13, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn project_remove_forgets_and_stops_sessions() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();

    let sid = SessionId::from_uuid(Uuid::from_u128(0x5E));
    let session = Session::restored(
        sid,
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![session],
    )));
    state
        .start_session(sid, micold_core::terminal::LaunchMode::Resume)
        .expect("start the shell session");

    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::ProjectRemove {
            req: 14,
            path: project.path().to_path_buf(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 14, .. })
    })
    .await;
    assert!(matches!(
        reply,
        DaemonMsg::OperationOk {
            result: OperationResult::Ack,
            ..
        }
    ));
    assert!(
        state.live_session(sid).is_none(),
        "the project's session was stopped"
    );
    let (snapshot, _) = state.welcome_payload();
    assert!(
        !has_project(&snapshot, project.path()),
        "the project was forgotten"
    );
}

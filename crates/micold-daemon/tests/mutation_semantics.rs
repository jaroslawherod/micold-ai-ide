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
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::{CreateMode, CreateStage};
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

/// Whether `branch` currently exists in `repo` (feature 013, FR-011/FR-012 regression check).
fn branch_exists(repo: &Path, branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .expect("git runs")
        .success()
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
            client_package_version: PACKAGE_VERSION.into(),
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
async fn worktree_create_on_a_taken_branch_with_new_branch_mode_is_refused_without_side_effects() {
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
            mode: CreateMode::NewBranch,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 1, .. })
    })
    .await;
    match reply {
        // Feature 016 changed what this means. An existing branch is no longer an error in itself
        // — it is a decision the client resolves first (reuse / overwrite / cancel). Asking for
        // `NewBranch` on a name that is already taken is therefore a *stale answer*: pre-flight
        // re-runs daemon-side and refuses before any git mutation (FR-009), so there is still
        // nothing to roll back.
        DaemonMsg::OperationError { kind, .. } => assert_eq!(kind, ErrorKind::Refused),
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
            mode: CreateMode::NewBranch,
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
            mode: CreateMode::NewBranch,
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

/// T058 (BUG-002, FR-023c case 2) — a delete whose directory cannot be fully removed is a
/// **partial success**, not a failure.
///
/// `git worktree remove --force` has already deregistered the worktree by the time the directory
/// removal runs, so failing the whole operation there stranded the worktree's sessions (the archive
/// is gated on success) and left the surviving directory to come back as an unregistered orphan.
///
/// The unremovable case is built without privilege by clearing write permission on a subdirectory:
/// unlinking an entry requires write on the *containing* directory, so this reproduces the `EACCES`
/// a root-owned file produces.
#[cfg(unix)]
#[tokio::test]
async fn worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports() {
    use std::os::unix::fs::PermissionsExt;

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let sid = SessionId::from_uuid(Uuid::from_u128(0x79));
    let session = Session::restored(
        sid,
        SessionLocation::Worktree("wt3".into()),
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
            "wt3",
            ".claude/worktrees/wt3",
            "HEAD",
        ])
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git worktree add for the fixture failed");

    // Build output the daemon cannot unlink, exactly as a container running as root leaves behind.
    let blocked = project.path().join(".claude/worktrees/wt3/build");
    std::fs::create_dir_all(&blocked).unwrap();
    std::fs::write(blocked.join("artifact.jar"), b"binary").unwrap();
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o555)).unwrap();

    let mut client = connect_and_attach(&state, project.path()).await;
    client
        .send(Frame::Control(ClientMsg::WorktreeDelete {
            req: 9,
            project: project.path().to_path_buf(),
            dir_name: "wt3".into(),
            stop_sessions: true,
            delete_branch: false,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(
            m,
            DaemonMsg::OperationOk { req: 9, .. } | DaemonMsg::OperationError { req: 9, .. }
        )
    })
    .await;

    // Restore before asserting so the temp dir can always be cleaned up.
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755)).unwrap();

    let leftovers = match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeDeleted { leftovers, .. },
            ..
        } => leftovers,
        other => panic!("a blocked directory is partial success, not a failed delete: {other:?}"),
    };
    assert!(
        !leftovers.is_empty(),
        "the surviving paths must be reported (FR-023d)"
    );

    // git released the worktree even though its directory did not fully go.
    let registered = String::from_utf8(
        Command::new("git")
            .arg("-C")
            .arg(project.path())
            .args(["worktree", "list", "--porcelain"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(
        !registered.contains("worktrees/wt3"),
        "git no longer registers the worktree, so the delete did happen"
    );

    // The whole point of FR-023c: the session is archived anyway, so it cannot be resurrected and
    // the row does not come back.
    assert!(
        state.live_session(sid).is_none(),
        "the worktree's session was stopped despite the leftover directory"
    );
    let (snapshot, _) = state.welcome_payload();
    let has_session = snapshot
        .projects
        .iter()
        .find(|p| p.path == project.path())
        .map(|p| p.sessions.iter().any(|s| s.id == sid))
        .unwrap_or(false);
    assert!(
        !has_session,
        "the session was archived, not left behind for a retry that can never succeed"
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
            delete_branch: true,
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
            delete_branch: true,
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
            result: OperationResult::WorktreeDeleted {
                branch_delete_failed: false,
                ..
            },
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
    // Convergence fix (retrofit session, 2026-07-27): `delete_branch: true` (the default, per
    // FR-012) must actually delete the branch — the wire previously had no field for this at
    // all, and the daemon hardcoded `None` (keep), so the user's choice had zero effect.
    assert!(
        !branch_exists(project.path(), "wt2"),
        "delete_branch: true must actually delete the branch (FR-011/FR-012/FR-014)"
    );
}

#[tokio::test]
async fn worktree_delete_with_delete_branch_false_keeps_the_branch() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        Vec::new(),
    )));
    let ok = Command::new("git")
        .arg("-C")
        .arg(project.path())
        .args([
            "worktree",
            "add",
            "-b",
            "wt3",
            ".claude/worktrees/wt3",
            "HEAD",
        ])
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git worktree add for the fixture failed");

    let mut client = connect_and_attach(&state, project.path()).await;
    client
        .send(Frame::Control(ClientMsg::WorktreeDelete {
            req: 7,
            project: project.path().to_path_buf(),
            dir_name: "wt3".into(),
            stop_sessions: true,
            delete_branch: false,
        }))
        .await
        .unwrap();

    let ok = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 7, .. })
    })
    .await;
    assert!(matches!(
        ok,
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeDeleted {
                branch_delete_failed: false,
                ..
            },
            ..
        }
    ));
    assert!(
        !project.path().join(".claude/worktrees/wt3").exists(),
        "the worktree directory was removed"
    );
    assert!(
        branch_exists(project.path(), "wt3"),
        "delete_branch: false must keep the branch (FR-013)"
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

#[tokio::test]
async fn attach_prunes_empty_sessions_but_keeps_live_ones() {
    use micold_core::provider::{AiCliProvider, ClaudeProvider};

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();

    // Two Default (shell) sessions: `live` will be started (excluded from pruning); `empty` stays
    // idle with no recorded conversation → a prune candidate.
    let live = SessionId::from_uuid(Uuid::from_u128(0xA11E));
    let empty = SessionId::from_uuid(Uuid::from_u128(0xE111));
    let mk = |id| {
        Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("S".into()),
            TerminalMode::Regular,
        )
    };
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![mk(live), mk(empty)],
    )));
    state
        .start_session(live, micold_core::terminal::LaunchMode::Resume)
        .expect("start the live session");

    // Attaching brings an observer → pruning runs (FR-007a).
    let _client = connect_and_attach(&state, project.path()).await;

    let (snapshot, _) = state.welcome_payload();
    let listed: Vec<SessionId> = snapshot
        .projects
        .iter()
        .find(|p| p.path == project.path())
        .map(|p| p.sessions.iter().map(|s| s.id).collect())
        .unwrap_or_default();

    assert!(listed.contains(&live), "a live session is never pruned");
    // The empty session is pruned only when the provider config dir is resolvable (it always is on
    // the CI/dev Linux hosts); guard so the test never flakes if a home dir can't be found.
    if ClaudeProvider.config_dir().is_some() {
        assert!(
            !listed.contains(&empty),
            "an idle, no-conversation session is pruned once observed"
        );
    }
}

// =======================================================================================
// Feature 016 — the existing-branch flow end-to-end through the daemon.
//
// These are the tests `FakeGit` cannot give: real git, real RPCs, real wire types.
// =======================================================================================

/// FR-001/FR-004: reusing an existing branch creates the worktree ON it, history intact.
#[tokio::test]
async fn worktree_create_can_reuse_an_existing_branch() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    create_branch(project.path(), "feature/reuse");

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
            branch: "feature/reuse".into(),
            dir_name: "reuse".into(),
            mode: CreateMode::ReuseLocal,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 1, .. })
    })
    .await;
    assert!(matches!(reply, DaemonMsg::OperationOk { .. }));

    // On the branch it was told to reuse — not a new one.
    let head = Command::new("git")
        .arg("-C")
        .arg(project.path().join(".claude/worktrees/reuse"))
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("git runs");
    assert_eq!(
        String::from_utf8_lossy(&head.stdout).trim(),
        "feature/reuse"
    );
}

/// FR-021: a branch checked out elsewhere is refused by name, not with a bare git failure.
#[tokio::test]
async fn worktree_create_on_a_branch_held_by_the_project_checkout_is_refused() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    // The repository's own current branch.
    let current = Command::new("git")
        .arg("-C")
        .arg(project.path())
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .expect("git runs");
    let current = String::from_utf8_lossy(&current.stdout).trim().to_string();

    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 1,
            project: project.path().to_path_buf(),
            branch: current.clone(),
            dir_name: "held".into(),
            mode: CreateMode::ReuseLocal,
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 1, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationError { kind, message, .. } => {
            assert_eq!(kind, ErrorKind::Busy);
            assert!(
                message.contains(&current) && message.contains("project"),
                "the message must name the branch and its holder, got: {message}"
            );
        }
        other => panic!("expected OperationError, got {other:?}"),
    }
}

/// FR-001: pre-flight classifies over the wire and mutates nothing.
#[tokio::test]
async fn branch_preflight_reports_an_existing_branch_without_touching_anything() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    create_branch(project.path(), "feature/exists");

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::BranchPreflight {
            req: 1,
            project: project.path().to_path_buf(),
            branch: "feature/exists".into(),
            dir_name: "exists".into(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 1, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::BranchPreflight { situation },
            ..
        } => assert_eq!(
            situation,
            micold_core::worktree::BranchSituation::LocalAvailable {
                branch: "feature/exists".into()
            }
        ),
        other => panic!("expected a BranchPreflight result, got {other:?}"),
    }

    // Read-only: no worktree directory appeared.
    assert!(!project.path().join(".claude/worktrees/exists").exists());
}

/// FR-011/FR-012: the picker's list arrives over the wire, with the held branch marked.
#[tokio::test]
async fn branch_list_returns_candidates_with_block_reasons() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    create_branch(project.path(), "feature/free");

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
        vec![],
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::BranchList {
            req: 1,
            project: project.path().to_path_buf(),
        }))
        .await
        .unwrap();

    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 1, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::BranchList { candidates },
            ..
        } => {
            let free = candidates
                .iter()
                .find(|c| c.name == "feature/free")
                .expect("the free branch is listed");
            assert!(free.is_available());
            // The project's own checkout is listed too — visible, but marked unavailable.
            assert!(
                candidates.iter().any(|c| !c.is_available()),
                "the branch held by the project checkout must be listed as unavailable: {candidates:?}"
            );
        }
        other => panic!("expected a BranchList result, got {other:?}"),
    }
}

/// FR-024 — the daemon streams the stage as the create advances, so the client can name the step
/// being performed. The *wording* is the client's; what must arrive here is the stage itself, in
/// order, before the terminal reply.
#[tokio::test]
async fn worktree_create_streams_its_stages_before_the_terminal_reply() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    create_branch(project.path(), "feature/staged");

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
            branch: "feature/staged".into(),
            dir_name: "staged".into(),
            mode: CreateMode::ReuseLocal,
        }))
        .await
        .unwrap();

    // Collect the progress pushes that precede the terminal reply.
    let mut stages = Vec::new();
    loop {
        let msg = expect_control(&mut client, |m| {
            matches!(
                m,
                DaemonMsg::OperationProgress { req: 1, .. } | DaemonMsg::OperationOk { req: 1, .. }
            )
        })
        .await;
        match msg {
            DaemonMsg::OperationProgress { stage, .. } => stages.push(stage),
            DaemonMsg::OperationOk { .. } => break,
            other => panic!("unexpected {other:?}"),
        }
    }

    assert_eq!(
        stages,
        vec![CreateStage::PreflightCheck, CreateStage::CreatingWorktree],
        "the stages a successful create passes through, each reported once"
    );
    // And the client can word them for the mode it asked for — the point of FR-024.
    assert_eq!(
        stages[1].label(&CreateMode::ReuseLocal),
        "Checking out existing branch"
    );
}

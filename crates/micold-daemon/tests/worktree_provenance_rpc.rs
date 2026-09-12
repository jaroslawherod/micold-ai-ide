//! T027/T032 (US2) — the daemon records what it creates, and migrates once (feature 029,
//! FR-002/FR-003/FR-006c/FR-006d/FR-010).
//!
//! Drives `server::serve_connection` over an in-memory duplex against **real git repositories**,
//! the same shape as `mutation_semantics.rs`, so the whole path is under test: the `WorktreeCreate`
//! arm, the record write, the broadcast that follows it, and the durable file the next launch would
//! read.
//!
//! Two properties are asserted throughout, and they are the whole feature:
//!
//! 1. **Every create records, whatever mode produced the branch.** The record is about who asked
//!    for the directory, not about how its branch came to exist — so all four `CreateMode`s record
//!    identically, and a rolled-back attempt records nothing because there is no directory to own.
//! 2. **The record precedes the broadcast.** The first snapshot a client sees after its create
//!    already carries `user_created: true`, so a worktree the user just made never appears as
//!    something the app would hide.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult, WorktreeSnapshot,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::CreateMode;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use uuid::Uuid;

// --- git fixtures -----------------------------------------------------------

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A real repository with one commit, so `HEAD` exists for `git worktree add`.
fn init_git_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "t@t.test"]);
    git(dir, &["config", "user.name", "T"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// Make a worktree on disk the way a *different* tool would — by hand, behind this app's back.
/// Used for the migration cases, which are about state that predates the feature.
fn worktree_by_hand(repo: &Path, dir_name: &str, branch: &str) {
    let target = repo.join(".claude/worktrees").join(dir_name);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            target.to_str().unwrap(),
        ],
    );
}

// --- daemon fixtures --------------------------------------------------------

/// A catalog holding one git-repo project, persisted under `store_dir`, seeded with `workspace`'s
/// per-project history. The store path is returned so a test can read back what was persisted.
fn catalog_at(store_dir: &Path, workspace: &Workspace) -> Catalog {
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

fn workspace_for(project_dir: &Path) -> Workspace {
    Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        ..Default::default()
    }
}

/// Re-read the durable store from disk — what the *next launch* would see, which is the only
/// evidence that a record survived the process that wrote it.
fn reload(store_dir: &Path) -> Workspace {
    JsonFileStore::at(store_dir.join("projects.json"))
        .load()
        .workspace
}

fn recorded(store_dir: &Path, project: &Path) -> Vec<String> {
    reload(store_dir)
        .user_created_worktrees(project)
        .iter()
        .cloned()
        .collect()
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

async fn expect_control(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

fn worktrees_in(snapshot: &CatalogSnapshot, project: &Path) -> Vec<WorktreeSnapshot> {
    snapshot
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.worktrees.clone())
        .unwrap_or_default()
}

/// Ask the daemon to create one worktree and wait for the operation to settle. Returns the last
/// `CatalogChanged` snapshot seen *before* the `OperationOk` — the first thing a real client would
/// render for this create.
async fn create(
    client: &mut Client,
    project: &Path,
    req: u64,
    branch: &str,
    dir_name: &str,
    mode: CreateMode,
) -> Option<CatalogSnapshot> {
    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req,
            project: project.to_path_buf(),
            branch: branch.to_string(),
            dir_name: dir_name.to_string(),
            mode,
        }))
        .await
        .unwrap();
    let mut last_snapshot = None;
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { catalog: snapshot }) => {
                last_snapshot = Some(snapshot)
            }
            Frame::Control(DaemonMsg::OperationOk { result, .. }) => {
                assert!(
                    matches!(result, OperationResult::WorktreeCreated { .. }),
                    "expected a create result, got {result:?}"
                );
                return last_snapshot;
            }
            Frame::Control(DaemonMsg::OperationError {
                kind,
                message,
                detail,
                ..
            }) => panic!("create failed: {kind:?} {message} {detail:?}"),
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

// ---------------------------------------------------------------------------
// T027 — every successful create records, in every mode
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_new_branch_create_is_recorded_durably() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/login",
        "feat-login",
        CreateMode::NewBranch,
    )
    .await;

    assert_eq!(recorded(store.path(), project.path()), vec!["feat-login"]);
}

#[tokio::test]
async fn reusing_an_existing_local_branch_is_recorded() {
    // The branch predates the app; the *directory* does not. Provenance is about the directory.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    git(project.path(), &["branch", "feat/existing"]);

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/existing",
        "feat-existing",
        CreateMode::ReuseLocal,
    )
    .await;

    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["feat-existing"]
    );
}

#[tokio::test]
async fn overwriting_a_branch_is_recorded() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    git(project.path(), &["branch", "feat/stale"]);

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/stale",
        "feat-stale",
        CreateMode::Overwrite,
    )
    .await;

    assert_eq!(recorded(store.path(), project.path()), vec!["feat-stale"]);
}

#[tokio::test]
async fn tracking_a_remote_branch_is_recorded() {
    let upstream = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(upstream.path());
    git(upstream.path(), &["branch", "feat/remote-only"]);
    init_git_repo(project.path());
    git(
        project.path(),
        &["remote", "add", "origin", upstream.path().to_str().unwrap()],
    );
    git(project.path(), &["fetch", "-q", "origin"]);

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/remote-only",
        "feat-remote-only",
        CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
    )
    .await;

    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["feat-remote-only"]
    );
}

#[tokio::test]
async fn a_refused_create_records_nothing() {
    // `NewBranch` on a name already taken is refused before any git mutation (016 FR-009). No
    // directory was made, so there is nothing whose provenance could be claimed — and recording
    // one anyway would grandfather a directory that a *later*, different tool might create there.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    git(project.path(), &["branch", "feat/dup"]);

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req: 1,
            project: project.path().to_path_buf(),
            branch: "feat/dup".into(),
            dir_name: "dup".into(),
            mode: CreateMode::NewBranch,
        }))
        .await
        .unwrap();
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationError { req: 1, .. })
    })
    .await;

    assert!(
        recorded(store.path(), project.path()).is_empty(),
        "a create that made nothing recorded nothing"
    );
}

#[tokio::test]
async fn the_first_snapshot_after_a_create_already_owns_the_worktree() {
    // FR-010's ordering, asserted where it can actually fail: the record is written before
    // `refresh_worktrees_and_broadcast`, so the snapshot the client renders for its own create
    // never says `user_created: false`. Were the order reversed, the row would arrive classifiable
    // as the app's own and blink out of a list the user is looking straight at.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    let snapshot = create(
        &mut client,
        project.path(),
        1,
        "feat/login",
        "feat-login",
        CreateMode::NewBranch,
    )
    .await
    .expect("the create broadcasts a catalog");

    let row = worktrees_in(&snapshot, project.path())
        .into_iter()
        .find(|w| w.dir_name == "feat-login")
        .expect("the new worktree is in the snapshot");
    assert!(
        row.user_created,
        "the create's own snapshot must already carry the record"
    );
}

// ---------------------------------------------------------------------------
// T032 — the migration runs once, and only once
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_migration_grandfathers_a_worktree_the_user_had_worked_in() {
    // The upgrade case. `feat-login` exists on disk and the user renamed it before this feature
    // shipped; nothing recorded it, because nothing was recording yet. Attaching refreshes, the
    // backfill runs, and the worktree stays in the user's list.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "feat-login", "feat/login");
    worktree_by_hand(project.path(), "tidy-the-parser", "tidy/parser");

    let mut workspace = workspace_for(project.path());
    workspace.worktree_names.insert(
        project.path().to_path_buf(),
        BTreeMap::from([("feat-login".to_string(), "Login work".to_string())]),
    );

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(store.path(), &workspace)));
    let _client = connect_and_attach(&state, project.path()).await;

    // Only the one with evidence. `tidy-the-parser` is exactly what an assistant session worktree
    // looks like: a real directory under the managed root with an ordinary name and no history.
    assert_eq!(recorded(store.path(), project.path()), vec!["feat-login"]);
    assert!(
        reload(store.path())
            .provenance_migrated
            .contains(project.path()),
        "the marker is persisted in the same write as the records"
    );
}

#[tokio::test]
async fn the_migration_marks_a_project_with_no_evidence_as_done() {
    // "Ran and found nothing" is a different state from "never ran", and only the first stops a
    // later session from grandfathering a worktree (FR-006d). The marker must be written even
    // though there was nothing to write beside it.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "tidy-the-parser", "tidy/parser");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let _client = connect_and_attach(&state, project.path()).await;

    assert!(recorded(store.path(), project.path()).is_empty());
    assert!(reload(store.path())
        .provenance_migrated
        .contains(project.path()));
}

#[tokio::test]
async fn evidence_appearing_after_the_migration_grandfathers_nothing() {
    // FR-006c/FR-006d together. The project migrates on the first refresh with no evidence, then
    // a session is started in the revealed `tidy-the-parser` — which 014 permits, and which under
    // a *standing* evidence rule would silently promote it to the user's for good. A second
    // refresh must leave it exactly as it was.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "tidy-the-parser", "tidy/parser");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let _client = connect_and_attach(&state, project.path()).await;
    assert!(recorded(store.path(), project.path()).is_empty());

    // The session the user starts in the revealed worktree, exactly as the store would hold it.
    state
        .create_session(project.path(), "tidy-the-parser", AiCli::ClaudeCode)
        .expect("session created");

    // Another refresh — every path that lists worktrees goes through this.
    state.refresh_worktrees(project.path());

    assert!(
        recorded(store.path(), project.path()).is_empty(),
        "a session started after the migration is not evidence"
    );
}

#[tokio::test]
async fn the_migration_does_not_overwrite_a_record_written_by_a_create() {
    // FR-006b at the daemon level: a worktree this app made in the same run is already recorded,
    // and the backfill neither duplicates nor disturbs it.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let mut workspace = workspace_for(project.path());
    workspace.sessions.insert(
        project.path().to_path_buf(),
        vec![Session::restored(
            SessionId::from_uuid(Uuid::from_u128(0x29)),
            SessionLocation::Worktree("feat-login".into()),
            SessionLabel::Named("Login".into()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        )],
    );

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(store.path(), &workspace)));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/login",
        "feat-login",
        CreateMode::NewBranch,
    )
    .await;

    assert_eq!(recorded(store.path(), project.path()), vec!["feat-login"]);
}

// ---------------------------------------------------------------------------
// T053 (US4) — the record dies with the worktree
// ---------------------------------------------------------------------------

/// Ask the daemon to delete one worktree, returning the operation reply.
async fn delete(client: &mut Client, project: &Path, req: u64, dir_name: &str) -> DaemonMsg {
    client
        .send(Frame::Control(ClientMsg::WorktreeDelete {
            req,
            project: project.to_path_buf(),
            dir_name: dir_name.to_string(),
            stop_sessions: true,
            delete_branch: true,
        }))
        .await
        .unwrap();
    expect_control(client, |m| {
        matches!(
            m,
            DaemonMsg::OperationOk { req: r, .. } | DaemonMsg::OperationError { req: r, .. }
                if *r == req
        )
    })
    .await
}

#[tokio::test]
async fn deleting_a_worktree_forgets_its_record() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/login",
        "feat-login",
        CreateMode::NewBranch,
    )
    .await;
    assert_eq!(recorded(store.path(), project.path()), vec!["feat-login"]);

    match delete(&mut client, project.path(), 2, "feat-login").await {
        DaemonMsg::OperationOk { .. } => {}
        other => panic!("expected the delete to succeed, got {other:?}"),
    }

    assert!(
        recorded(store.path(), project.path()).is_empty(),
        "the record must not outlive the worktree"
    );
}

#[tokio::test]
async fn a_directory_name_reused_after_a_delete_is_not_the_users() {
    // FR-019, the reason the record is forgotten at all. Directory names are reusable, so a record
    // that outlived its worktree would hand whatever appears next at that path an ownership nobody
    // granted it — including an assistant's session worktree that happened to pick the same name.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/login",
        "feat-login",
        CreateMode::NewBranch,
    )
    .await;
    delete(&mut client, project.path(), 2, "feat-login").await;

    // Something else now makes a worktree at the very same directory name, from outside the app.
    worktree_by_hand(project.path(), "feat-login", "feat/login-again");
    state.refresh_worktrees(project.path());

    let (snapshot, _) = state.welcome_payload();
    let row = worktrees_in(&snapshot, project.path())
        .into_iter()
        .find(|w| w.dir_name == "feat-login")
        .expect("the new directory is discovered");
    assert!(
        !row.user_created,
        "a reused name inherits nothing from the worktree that had it"
    );
}

#[tokio::test]
async fn a_failed_delete_leaves_the_record_alone() {
    // The delete is refused by git and the worktree survives on disk, so nothing may change in the
    // records — forgetting on a failed delete would hide a worktree the user still has, which is
    // exactly the harm this feature exists to prevent.
    //
    // The refusal is a **locked** worktree, not an absent one. Asking to remove a directory that
    // was never there succeeds: `GitCli::worktree_remove` decides by outcome rather than by exit
    // status (BUG-001), and a path git does not list afterwards is a path the caller got what it
    // asked for. A lock is the cheap genuine refusal — `git worktree remove --force` declines it
    // and it is still registered when git is asked again.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    create(
        &mut client,
        project.path(),
        1,
        "feat/locked",
        "feat-locked",
        CreateMode::NewBranch,
    )
    .await;
    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["feat-locked"],
        "precondition: the create recorded it"
    );
    let target = project.path().join(".claude/worktrees/feat-locked");
    git(
        project.path(),
        &["worktree", "lock", target.to_str().unwrap()],
    );

    match delete(&mut client, project.path(), 2, "feat-locked").await {
        DaemonMsg::OperationError { .. } => {}
        other => panic!("expected the delete to fail, got {other:?}"),
    }

    assert!(target.is_dir(), "precondition: the lock kept it on disk");
    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["feat-locked"],
        "a delete that removed nothing forgets nothing"
    );
}

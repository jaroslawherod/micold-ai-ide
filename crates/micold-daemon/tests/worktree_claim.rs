//! T061/T062 [US5] — the claim RPC (feature 029, FR-020–FR-024).
//!
//! Drives `server::serve_connection` over an in-memory duplex, the same shape as
//! `worktree_provenance_rpc.rs`, against a **real git repository** — because the load-bearing claim
//! of this message is a negative one: that it touches nothing. A fake git would prove nothing about
//! a handler that is supposed to stay away from git entirely, so the repository is real and the
//! test looks at it before and after.
//!
//! What is asserted:
//!
//! 1. **It persists and broadcasts.** The record survives into the file the next launch reads, and
//!    the push that follows is what drops the `agent` chip in a second open window.
//! 2. **It is honoured for a reserved-convention name** (FR-023). The naming veto binds the
//!    automatic backfill, which guesses; it never binds the user, who is stating a fact.
//! 3. **It does nothing on disk** (FR-003). Directory contents and branch, unchanged.
//! 4. **A persistence failure is reported as `IoFailed`** — the one error this message has.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, ErrorKind, OperationResult, WorktreeSnapshot,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{FakeProjectStore, JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

// --- git fixtures -----------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn init_git_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "t@t.test"]);
    git(dir, &["config", "user.name", "T"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// A worktree made behind this app's back — which is the only kind that can be claimed.
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

/// Everything about a worktree that a git or filesystem operation would disturb: the sorted names
/// of its directory entries, the branch checked out in it, and that branch's commit.
fn on_disk(repo: &Path, dir_name: &str, branch: &str) -> (Vec<String>, String, String) {
    let dir = repo.join(".claude/worktrees").join(dir_name);
    let mut entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("worktree directory exists")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    let head = git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit = git(repo, &["rev-parse", branch]);
    (entries, head, commit)
}

// --- daemon fixtures --------------------------------------------------------

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

fn recorded(store_dir: &Path, project: &Path) -> Vec<String> {
    JsonFileStore::at(store_dir.join("projects.json"))
        .load()
        .workspace
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

/// Attach as a real client does, so the connection is registered for catalog pushes.
///
/// Attaching also runs the one-time FR-006 backfill for the project. That is deliberate: every
/// worktree here is made by hand with no session and no rename behind it, so the backfill has no
/// evidence to act on and records nothing — which is precisely the state a claim exists for.
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

/// Send one claim and return everything the daemon said in reply: the last catalog push seen before
/// the operation settled, and the settling message itself.
async fn claim(
    client: &mut Client,
    project: &Path,
    req: u64,
    dir_name: &str,
) -> (Option<CatalogSnapshot>, DaemonMsg) {
    client
        .send(Frame::Control(ClientMsg::WorktreeClaim {
            req,
            project: project.to_path_buf(),
            dir_name: dir_name.to_string(),
        }))
        .await
        .unwrap();
    let mut last_snapshot = None;
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { catalog: snapshot }) => {
                last_snapshot = Some(snapshot)
            }
            Frame::Control(
                m @ (DaemonMsg::OperationOk { .. } | DaemonMsg::OperationError { .. }),
            ) => return (last_snapshot, m),
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

// ---------------------------------------------------------------------------
// T061 — acks, persists, broadcasts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_claim_acks_persists_and_broadcasts() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "hand-made", "hand/made");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    let (snapshot, settled) = claim(&mut client, project.path(), 1, "hand-made").await;

    assert!(
        matches!(
            settled,
            DaemonMsg::OperationOk {
                req: 1,
                result: OperationResult::Ack
            }
        ),
        "a claim acks — there is nothing to report back but that it was written, got {settled:?}"
    );
    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["hand-made".to_string()],
        "and the record is in the file the next launch reads"
    );
    let snapshot = snapshot.expect("a claim broadcasts, so a second window agrees");
    assert!(
        worktrees_in(&snapshot, project.path())
            .iter()
            .any(|w| w.dir_name == "hand-made" && w.user_created),
        "the pushed snapshot carries the flag: the broadcast is what drops the `agent` chip in \
         another open window, the same mechanism that already carries a rename between windows"
    );
}

#[tokio::test]
async fn a_reserved_convention_name_is_claimed_without_veto() {
    // FR-023. `matches_reserved_convention` is the migration's veto, and the migration guesses.
    // The user is not guessing, and a worktree they made and happened to name `agent-<hex>` is
    // exactly the case a veto here would make permanently unfixable — which is the opposite of
    // what the claim exists for.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    let dir = "agent-a885b42dc521fbda1";
    worktree_by_hand(project.path(), dir, "worktree-agent-a885b42dc521fbda1");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    let (_, settled) = claim(&mut client, project.path(), 2, dir).await;

    assert!(
        matches!(settled, DaemonMsg::OperationOk { .. }),
        "{settled:?}"
    );
    assert_eq!(
        recorded(store.path(), project.path()),
        vec![dir.to_string()]
    );
}

#[tokio::test]
async fn claiming_the_same_worktree_twice_is_idempotent() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "hand-made", "hand/made");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    let (_, first) = claim(&mut client, project.path(), 3, "hand-made").await;
    let (_, second) = claim(&mut client, project.path(), 4, "hand-made").await;

    assert!(matches!(first, DaemonMsg::OperationOk { .. }), "{first:?}");
    assert!(
        matches!(second, DaemonMsg::OperationOk { .. }),
        "claiming what is already claimed acks rather than erroring (FR-022), got {second:?}"
    );
    assert_eq!(
        recorded(store.path(), project.path()),
        vec!["hand-made".to_string()],
        "and the set is a set"
    );
}

#[tokio::test]
async fn a_claim_that_cannot_be_persisted_reports_io_failed() {
    // The one error this message has: there is no user-supplied string to validate, so nothing can
    // be invalid. A claim the user made and the app forgot would be worse than a refusal — the row
    // would be listed until the next restart and hidden after it — so the failure is reported.
    let project = tempfile::tempdir().unwrap();
    init_git_repo(project.path());

    let settings_dir = tempfile::tempdir().unwrap();
    let store = FakeProjectStore::loaded(workspace_for(project.path()))
        .failing_save(std::io::ErrorKind::PermissionDenied);
    let catalog = Catalog::load(
        Box::new(store),
        Box::new(JsonFileSettingsStore::at(
            settings_dir.path().join("settings.json"),
        )),
    );
    // Not attached: attaching would refresh worktrees and try the same failing save on the
    // migration marker, which is a different write from the one under test.
    let state = std::sync::Arc::new(DaemonState::new(catalog));
    let mut client = connect(&state).await;

    let (_, settled) = claim(&mut client, project.path(), 5, "hand-made").await;

    match settled {
        DaemonMsg::OperationError { req, kind, .. } => {
            assert_eq!(req, 5);
            assert_eq!(kind, ErrorKind::IoFailed);
        }
        other => panic!("expected an IoFailed error, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// T062 — nothing on disk (FR-003, US5 scenario 4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_claim_performs_no_git_or_filesystem_operation() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    worktree_by_hand(project.path(), "hand-made", "hand/made");
    // A file only this test put there: if anything about the claim went near the directory —
    // a re-add, a prune, a checkout — this is what would not survive it.
    std::fs::write(
        project.path().join(".claude/worktrees/hand-made/NOTES.txt"),
        "work in progress",
    )
    .unwrap();

    let before = on_disk(project.path(), "hand-made", "hand/made");

    let state = std::sync::Arc::new(DaemonState::new(catalog_at(
        store.path(),
        &workspace_for(project.path()),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;
    let (_, settled) = claim(&mut client, project.path(), 6, "hand-made").await;
    assert!(
        matches!(settled, DaemonMsg::OperationOk { .. }),
        "{settled:?}"
    );

    assert_eq!(
        on_disk(project.path(), "hand-made", "hand/made"),
        before,
        "a claim is a statement about authorship, not an operation: the directory's contents, the \
         branch checked out in it and that branch's commit are all exactly as they were (FR-003)"
    );
    assert!(
        project
            .path()
            .join(".claude/worktrees/hand-made/NOTES.txt")
            .exists(),
        "including uncommitted work nobody asked the app to manage"
    );
}

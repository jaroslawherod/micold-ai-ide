//! Feature 582: attaching a provider's worktrees (T005 catalog level, T008 daemon level).
//!
//! T005 drives `Catalog::attach_worktrees` directly over a real on-disk store, so "one persist" and
//! "survives a restart" are checked through the file the next launch reads.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use micold_core::attach::{AttachOutcome, RefuseReason};
use micold_core::project::{Availability, Project};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::{Worktree, WorktreeStatus};
use micold_daemon::catalog::Catalog;

fn catalog_at(store_dir: &Path, project: &Path) -> Catalog {
    let projects_path = store_dir.join("projects.json");
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        ..Default::default()
    };
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
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

fn live(project: &Path, names: &[&str]) -> Vec<Worktree> {
    names
        .iter()
        .map(|n| Worktree {
            dir_name: (*n).to_string(),
            path: project.join(".claude/worktrees").join(n),
            branch: Some(format!("worktree-{n}")),
            status: WorktreeStatus::Valid,
            included: false,
        })
        .collect()
}

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn a_batch_writes_one_record_per_dir_name() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut catalog = catalog_at(store.path(), &project);
    let live = live(&project, &["a", "b", "c"]);

    let outcomes = catalog
        .attach_worktrees(&project, &live, &names(&["a", "b", "c"]))
        .unwrap();

    assert_eq!(outcomes, vec![AttachOutcome::Attached; 3]);
    assert_eq!(recorded(store.path(), &project), ["a", "b", "c"]);
}

#[test]
fn a_recorded_dir_name_is_already_attached_and_nothing_is_written() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut catalog = catalog_at(store.path(), &project);
    let live = live(&project, &["a"]);
    catalog.claim_worktree(&project, "a").unwrap();
    let before = std::fs::metadata(store.path().join("projects.json"))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let outcomes = catalog
        .attach_worktrees(&project, &live, &names(&["a", "a"]))
        .unwrap();

    assert_eq!(outcomes, vec![AttachOutcome::AlreadyAttached; 2], "FR-004");
    assert_eq!(recorded(store.path(), &project), ["a"], "no duplicate");
    let after = std::fs::metadata(store.path().join("projects.json"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        before, after,
        "an all-already-attached batch does not persist"
    );
}

#[test]
fn a_target_absent_from_the_live_cache_is_refused_and_unrecorded() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut catalog = catalog_at(store.path(), &project);
    let live = live(&project, &["a"]);

    let outcomes = catalog
        .attach_worktrees(&project, &live, &names(&["ghost", "a"]))
        .unwrap();

    assert_eq!(
        outcomes,
        vec![
            AttachOutcome::Refused(RefuseReason::NotAWorktreeOfProject),
            AttachOutcome::Attached
        ],
        "a stale client list cannot attach a non-worktree, and does not stop the rest"
    );
    assert_eq!(recorded(store.path(), &project), ["a"]);
}

#[test]
fn a_missing_worktree_is_refused_as_unavailable() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut catalog = catalog_at(store.path(), &project);
    let mut live = live(&project, &["a"]);
    live[0].status = WorktreeStatus::Missing;

    let outcomes = catalog
        .attach_worktrees(&project, &live, &names(&["a"]))
        .unwrap();

    assert_eq!(
        outcomes,
        vec![AttachOutcome::Refused(RefuseReason::Unavailable)]
    );
    assert!(recorded(store.path(), &project).is_empty());
}

#[test]
fn two_concurrent_attaches_of_one_worktree_leave_exactly_one_record() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let catalog = Arc::new(Mutex::new(catalog_at(store.path(), &project)));
    let live = Arc::new(live(&project, &["a"]));

    let handles: Vec<_> = (0..2)
        .map(|_| {
            let (catalog, live, project) =
                (Arc::clone(&catalog), Arc::clone(&live), project.clone());
            std::thread::spawn(move || {
                catalog
                    .lock()
                    .unwrap()
                    .attach_worktrees(&project, &live, &names(&["a"]))
                    .unwrap()
                    .remove(0)
            })
        })
        .collect();
    let mut outcomes: Vec<AttachOutcome> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    outcomes.sort_by_key(|o| matches!(o, AttachOutcome::AlreadyAttached));

    assert_eq!(
        outcomes,
        vec![AttachOutcome::Attached, AttachOutcome::AlreadyAttached],
        "FR-016: one wins, the other is told it was already attached"
    );
    assert_eq!(recorded(store.path(), &project), ["a"]);
}

// ---------------------------------------------------------------------------
// T008 [US1] — the daemon messages, against a real git repository
// ---------------------------------------------------------------------------

use std::process::Command;

use futures_util::{SinkExt, StreamExt};
use micold_core::attach::{AttachItem, AttachResult, DiscoveryReport};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult, WorktreeSnapshot,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

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

/// A provider worktree: made behind the app's back under `.claude/worktrees/`.
fn provider_worktree(repo: &Path, dir_name: &str) {
    let target = repo.join(".claude/worktrees").join(dir_name);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("worktree-{dir_name}"),
            target.to_str().unwrap(),
        ],
    );
}

/// Everything an attach must not disturb in one worktree: branch, commit, `git status`, and a hash
/// of every file's name and bytes (SC-003).
fn fingerprint(repo: &Path, dir_name: &str) -> String {
    let dir = repo.join(".claude/worktrees").join(dir_name);
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_name() != ".git")
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap_or_default(),
            )
        })
        .collect();
    files.sort();
    format!(
        "{}|{}|{}|{files:?}",
        git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        git(&dir, &["rev-parse", "HEAD"]),
        git(&dir, &["status", "--porcelain"]),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect_and_attach(state: &Arc<DaemonState>, project: &Path) -> Client {
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
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    expect(&mut client, |m| matches!(m, DaemonMsg::Attached { .. })).await;
    expect(&mut client, |m| {
        matches!(m, DaemonMsg::CatalogChanged { .. })
    })
    .await;
    client
}

async fn expect(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

async fn discover(client: &mut Client, project: &Path, req: u64) -> DiscoveryReport {
    client
        .send(Frame::Control(ClientMsg::AttachDiscover {
            req,
            project: project.to_path_buf(),
        }))
        .await
        .unwrap();
    match expect(client, |m| matches!(m, DaemonMsg::AttachReport { .. })).await {
        DaemonMsg::AttachReport { report, .. } => report,
        _ => unreachable!(),
    }
}

/// Send an apply; return the last catalog push before it settled and the per-target results.
async fn apply(
    client: &mut Client,
    project: &Path,
    req: u64,
    dirs: &[&str],
) -> (Option<CatalogSnapshot>, Vec<AttachResult>) {
    client
        .send(Frame::Control(ClientMsg::AttachApply {
            req,
            project: project.to_path_buf(),
            targets: dirs
                .iter()
                .map(|d| AttachItem::Worktree {
                    dir_name: (*d).to_string(),
                })
                .collect(),
        }))
        .await
        .unwrap();
    let mut last = None;
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { catalog }) => last = Some(catalog),
            Frame::Control(DaemonMsg::OperationOk {
                result: OperationResult::AttachApplied { results },
                ..
            }) => return (last, results),
            Frame::Control(m @ DaemonMsg::OperationError { .. }) => panic!("{m:?}"),
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

fn fresh_state(store: &Path, project: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(catalog_at(store, project)))
}

#[tokio::test]
async fn attached_worktrees_survive_a_restart_and_show_with_the_filter_off() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    for n in ["one", "two", "three"] {
        provider_worktree(project.path(), n);
    }
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;

    let report = discover(&mut client, project.path(), 1).await;
    assert_eq!(
        report.worktrees.len(),
        3,
        "scenario 1: all three are offered"
    );

    let (snapshot, results) = apply(&mut client, project.path(), 2, &["one", "two"]).await;
    assert!(results.iter().all(|r| r.outcome == AttachOutcome::Attached));
    let snapshot = snapshot.expect("an attach broadcasts the catalog");
    let shown: Vec<_> = worktrees_in(&snapshot, project.path())
        .into_iter()
        .filter(|w| w.user_created)
        .map(|w| w.dir_name)
        .collect();
    assert_eq!(
        shown,
        ["one", "two"],
        "FR-002: attached rows are the user's, so shown with the filter off"
    );

    drop(client);
    drop(state);
    let restarted = Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    )));
    let mut client = connect_and_attach(&restarted, project.path()).await;
    let report = discover(&mut client, project.path(), 3).await;
    let left: Vec<_> = report
        .worktrees
        .iter()
        .map(|w| w.dir_name.as_str())
        .collect();
    assert_eq!(
        left,
        ["three"],
        "the attachment survived the restart; only one is still offered"
    );
}

#[tokio::test]
async fn attaching_changes_nothing_in_the_worktree() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "work");
    let dir = project.path().join(".claude/worktrees/work");
    std::fs::write(dir.join("scratch.txt"), "uncommitted").unwrap();
    git(&dir, &["add", "scratch.txt"]);
    std::fs::write(dir.join("untracked.txt"), "untracked").unwrap();
    let before = fingerprint(project.path(), "work");

    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;
    let (_, results) = apply(&mut client, project.path(), 1, &["work"]).await;

    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert_eq!(
        before,
        fingerprint(project.path(), "work"),
        "FR-003, SC-003: branch, commit, status and every file are byte-for-byte unchanged"
    );
}

#[tokio::test]
async fn attaching_again_reports_already_attached() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "work");
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;

    apply(&mut client, project.path(), 1, &["work"]).await;
    let (_, again) = apply(&mut client, project.path(), 2, &["work"]).await;

    assert_eq!(again[0].outcome, AttachOutcome::AlreadyAttached, "FR-004");
    assert_eq!(recorded(store.path(), project.path()), ["work"]);
}

#[tokio::test]
async fn sixteen_worktrees_attach_in_one_action_within_ten_seconds() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    let dirs: Vec<String> = (0..16).map(|i| format!("wt-{i:02}")).collect();
    for d in &dirs {
        provider_worktree(project.path(), d);
    }
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;

    let started = std::time::Instant::now();
    let report = discover(&mut client, project.path(), 1).await;
    let all: Vec<&str> = report
        .worktrees
        .iter()
        .map(|w| w.dir_name.as_str())
        .collect();
    let (snapshot, results) = apply(&mut client, project.path(), 2, &all).await;
    let took = started.elapsed();

    assert_eq!(results.len(), 16);
    assert!(results.iter().all(|r| r.outcome == AttachOutcome::Attached));
    let shown = worktrees_in(&snapshot.unwrap(), project.path())
        .iter()
        .filter(|w| w.user_created)
        .count();
    assert_eq!(shown, 16, "SC-001: all sixteen are in the sidebar");
    assert!(
        took < std::time::Duration::from_secs(10),
        "SC-001: took {took:?}"
    );
}

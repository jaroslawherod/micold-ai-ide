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
fn a_worktree_outside_the_managed_directory_is_refused_and_unrecorded() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut catalog = catalog_at(store.path(), &project);
    let mut live = live(&project, &["a", "b"]);
    live[0].included = true;

    let outcomes = catalog
        .attach_worktrees(&project, &live, &names(&["a", "b"]))
        .unwrap();

    assert_eq!(
        outcomes,
        vec![
            AttachOutcome::Refused(RefuseReason::NotAWorktreeOfProject),
            AttachOutcome::Attached
        ],
        "an included (outside-the-directory) worktree is not attachable"
    );
    assert_eq!(recorded(store.path(), &project), ["b"], "no record for `a`");
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

#[test]
fn an_unreadable_project_refuses_every_target_and_records_nothing() {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let mut seeded = Workspace {
        projects: vec![Project::new(project.clone(), true, Availability::Available)],
        active: Some(project.clone()),
        ..Default::default()
    };
    seeded.record_user_created(&project, "old");
    let projects_path = store.path().join("projects.json");
    let file_store = JsonFileStore::at(projects_path.clone());
    file_store.save(&seeded).unwrap();
    std::fs::write(file_store.project_state_path(&project), "not json").unwrap();
    let mut catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    );

    let outcomes = catalog
        .attach_worktrees(&project, &live(&project, &["a"]), &names(&["a"]))
        .unwrap();

    assert_eq!(
        outcomes,
        vec![AttachOutcome::Refused(RefuseReason::IoFailed)]
    );
}

/// A store that loads from disk but whose `save` fails while `fail` is set.
struct FlakyStore {
    inner: JsonFileStore,
    fail: Arc<std::sync::atomic::AtomicBool>,
}

impl ProjectStore for FlakyStore {
    fn load(&self) -> micold_core::store::LoadOutcome {
        self.inner.load()
    }
    fn save(&self, workspace: &Workspace) -> std::io::Result<()> {
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(std::io::Error::other("disk full"));
        }
        self.inner.save(workspace)
    }
}

#[test]
fn a_failed_persist_rolls_the_records_back_so_a_retry_attaches() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let projects_path = store.path().join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            ..Default::default()
        })
        .unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let mut catalog = Catalog::load(
        Box::new(FlakyStore {
            inner: JsonFileStore::at(projects_path),
            fail: fail.clone(),
        }),
        Box::new(JsonFileSettingsStore::at(
            store.path().join("settings.json"),
        )),
    );
    let live = live(&project, &["a"]);

    assert!(catalog
        .attach_worktrees(&project, &live, &names(&["a"]))
        .is_err());
    fail.store(false, Ordering::SeqCst);
    let retry = catalog
        .attach_worktrees(&project, &live, &names(&["a"]))
        .unwrap();

    assert_eq!(retry, vec![AttachOutcome::Attached]);
    assert_eq!(recorded(store.path(), &project), ["a"]);
}

// ---------------------------------------------------------------------------
// T019 [US2] — sessions: discovering, attaching idle, resuming (feature 582, M2)
// ---------------------------------------------------------------------------

use micold_core::attach::{ResumableStatus, SkipReason};
use micold_core::session::{AiCli, Session, SessionId, SessionLifecycle, SessionLocation};
use micold_core::terminal::LaunchMode;
use uuid::Uuid;

/// One scratch home for every provider store in this test binary. The providers read their base
/// directory from the environment, so it is set once; each test seeds under its own project's
/// path, so tests never see each other's sessions.
fn provider_home() -> &'static Path {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    HOME.get_or_init(|| {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("CLAUDE_CONFIG_DIR", home.path().join(".claude"));
        std::env::set_var("COPILOT_HOME", home.path().join(".copilot"));
        std::env::set_var("PI_CODING_AGENT_DIR", home.path().join(".pi"));
        home
    })
    .path()
}

/// A Claude transcript for a session that ran in `cwd`.
fn seed_claude(cwd: &Path) -> Uuid {
    let id = Uuid::new_v4();
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = provider_home().join(".claude/projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{id}.jsonl")),
        format!(
            "{{\"type\":\"user\",\"cwd\":{},\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n",
            serde_json::to_string(&cwd.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    id
}

async fn apply_items(
    client: &mut Client,
    project: &Path,
    req: u64,
    targets: Vec<AttachItem>,
) -> (Option<CatalogSnapshot>, Vec<AttachResult>) {
    client
        .send(Frame::Control(ClientMsg::AttachApply {
            req,
            project: project.to_path_buf(),
            targets,
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

fn session_ids(state: &DaemonState, project: &Path) -> Vec<Uuid> {
    state
        .catalog_snapshot()
        .projects
        .iter()
        .find(|p| p.path == project)
        .map(|p| p.sessions.iter().map(|s| s.id.0).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn discovery_lists_this_projects_sessions_and_attaching_one_adds_an_idle_entry() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;
    // Written after the attach, so feature 026's adoption pass has not already taken it.
    let mine = seed_claude(project.path());

    let report = discover(&mut client, project.path(), 1).await;
    assert!(
        report.sessions.iter().any(|s| s.id == mine),
        "FR-006: the project's stored session is listed"
    );

    let (_, results) = apply_items(
        &mut client,
        project.path(),
        2,
        vec![AttachItem::Session { id: mine }],
    )
    .await;

    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert!(session_ids(&state, project.path()).contains(&mine));
    assert!(
        state.live_session(SessionId::from_uuid(mine)).is_none(),
        "FR-008: attaching never starts the provider"
    );
    let after = discover(&mut client, project.path(), 3).await;
    assert!(
        after.sessions.iter().all(|s| s.id != mine),
        "scenario 3: a catalog session is not listed again"
    );
}

#[tokio::test]
async fn resuming_a_session_in_an_unattached_worktree_attaches_the_worktree_first() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "alpha");
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;
    // Written after the attach, so feature 026's adoption pass has not already taken it.
    let id = seed_claude(&project.path().join(".claude/worktrees/alpha"));

    let report = discover(&mut client, project.path(), 1).await;
    let listed = report.sessions.iter().find(|s| s.id == id).expect("listed");
    assert_eq!(listed.status, ResumableStatus::NeedsWorktreeAttach);

    let (_, results) = apply_items(
        &mut client,
        project.path(),
        2,
        vec![AttachItem::Session { id }],
    )
    .await;

    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert_eq!(
        recorded(store.path(), project.path()),
        ["alpha"],
        "the session's worktree is attached before the session"
    );
    let snapshot = state.catalog_snapshot();
    let project_snapshot = snapshot
        .projects
        .iter()
        .find(|p| p.path == project.path())
        .unwrap();
    assert!(project_snapshot.sessions.iter().any(|s| s.id.0 == id));
}

#[tokio::test]
async fn a_session_of_a_deleted_worktree_is_refused_not_resumed_elsewhere() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    let id = seed_claude(&project.path().join(".claude/worktrees/vanished"));
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;

    let (_, results) = apply_items(
        &mut client,
        project.path(),
        1,
        vec![AttachItem::Session { id }],
    )
    .await;

    assert_eq!(
        results[0].outcome,
        AttachOutcome::Refused(RefuseReason::Unavailable)
    );
    assert!(!session_ids(&state, project.path()).contains(&id));
}

/// A catalog holding one session of `project` in `lifecycle`, and a fresh copy of that session
/// (the one a second resume would offer).
fn catalog_with_session_in(
    store: &Path,
    project: &Path,
    lifecycle: SessionLifecycle,
) -> (Catalog, Session) {
    let mut catalog = catalog_at(store, project);
    let mut session = Session::restored(
        SessionId::from_uuid(Uuid::new_v4()),
        SessionLocation::Default,
        micold_core::session::SessionLabel::Pending,
        micold_core::session::TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    session.lifecycle = lifecycle;
    catalog.attach_session(project, session.clone()).unwrap();
    (catalog, session)
}

fn second_resume_in(lifecycle: SessionLifecycle) -> (AttachOutcome, usize) {
    let store = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/p");
    let (mut catalog, session) = catalog_with_session_in(store.path(), &project, lifecycle);
    let outcome = catalog.attach_session(&project, session).unwrap();
    (outcome, catalog.known_session_ids(&project).len())
}

#[test]
fn a_second_resume_of_a_starting_session_is_refused_as_already_running() {
    let (outcome, entries) = second_resume_in(SessionLifecycle::Starting);
    assert_eq!(
        outcome,
        AttachOutcome::Refused(RefuseReason::AlreadyRunning)
    );
    assert_eq!(entries, 1, "one entry");
}

#[test]
fn a_second_resume_of_a_running_session_is_refused_as_already_running() {
    let (outcome, entries) = second_resume_in(SessionLifecycle::Running);
    assert_eq!(
        outcome,
        AttachOutcome::Refused(RefuseReason::AlreadyRunning)
    );
    assert_eq!(entries, 1, "one entry");
}

#[test]
fn a_second_resume_of_a_restarting_session_is_refused_as_already_running() {
    let (outcome, entries) = second_resume_in(SessionLifecycle::Restarting { attempts: 1 });
    assert_eq!(
        outcome,
        AttachOutcome::Refused(RefuseReason::AlreadyRunning)
    );
    assert_eq!(entries, 1, "one entry");
}

#[test]
fn a_second_resume_of_an_idle_session_is_one_entry_already_attached() {
    let (outcome, entries) = second_resume_in(SessionLifecycle::Idle);
    assert_eq!(outcome, AttachOutcome::AlreadyAttached);
    assert_eq!(entries, 1, "one entry");
}

#[test]
fn a_second_resume_of_a_failed_session_is_one_entry_already_attached() {
    let (outcome, entries) = second_resume_in(SessionLifecycle::Failed {
        reason: "x".into(),
        attempts: 3,
    });
    assert_eq!(outcome, AttachOutcome::AlreadyAttached);
    assert_eq!(entries, 1, "one entry");
}

#[test]
fn the_already_running_refusal_tells_the_user_why() {
    assert!(
        RefuseReason::AlreadyRunning
            .text()
            .contains("already running"),
        "FR-016: the user is told why"
    );
}

#[tokio::test]
async fn a_sandboxed_daemon_reports_the_store_is_not_readable() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    seed_claude(project.path());
    let state = fresh_state(store.path(), project.path());
    let token = store.path().join("token");
    std::fs::write(&token, "0123456789abcdef0123456789abcdef").unwrap();
    // The container placement is the one that supplies an authentication token.
    state.set_auth_token(&token).unwrap();
    let report = state.attach_discover(project.path());
    assert!(report.sessions.is_empty());
    assert!(report
        .notes
        .iter()
        .any(|n| n.reason == SkipReason::SandboxStoreNotReadable));
}

#[test]
fn the_resume_launch_carries_the_right_argument_and_session_id_for_each_provider() {
    let id = Uuid::parse_str("11111111-2222-4222-8222-333333333333").unwrap();
    let args = |cli: AiCli| cli.provider().launch_args(id, LaunchMode::Resume);
    assert_eq!(
        args(AiCli::ClaudeCode),
        ["--resume".to_string(), id.to_string()]
    );
    assert!(args(AiCli::Copilot).contains(&format!("--resume={id}")));
    let pi = args(AiCli::Pi);
    let at = pi
        .iter()
        .position(|a| a == "--session-id")
        .expect("pi flag");
    assert_eq!(pi[at + 1], id.to_string());
}

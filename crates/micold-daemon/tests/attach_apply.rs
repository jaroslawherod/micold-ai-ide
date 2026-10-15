//! Feature 582: attaching a provider's sessions (daemon level, feature 552 removed worktree attaching).
//!
//! Worktrees are never attached; only stored provider sessions are discovered and resumed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use micold_core::attach::{AttachItem, AttachOutcome, AttachResult, DiscoveryReport, RefuseReason};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
use conn::{connect_and_attach, Client};

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
fn fresh_state(store: &Path, project: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(catalog_at(store, project)))
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
        vec![AttachItem::Session {
            id: mine,
            replace: false,
            allow_additional_session: false,
        }],
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
async fn a_session_in_an_existing_worktree_is_resumable_and_attaches() {
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
    assert_eq!(listed.status, ResumableStatus::Resumable);

    let (_, results) = apply_items(
        &mut client,
        project.path(),
        2,
        vec![AttachItem::Session {
            id,
            replace: false,
            allow_additional_session: false,
        }],
    )
    .await;

    assert_eq!(results[0].outcome, AttachOutcome::Attached);
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
        vec![AttachItem::Session {
            id,
            replace: false,
            allow_additional_session: false,
        }],
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

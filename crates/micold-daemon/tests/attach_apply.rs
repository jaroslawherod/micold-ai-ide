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

// ---------------------------------------------------------------------------
// T024 [US3] — resume into an occupied location (feature 776, FR-007, FR-008b, FR-008c, FR-012)
// ---------------------------------------------------------------------------

use micold_core::session::{SessionLabel, TerminalMode};

fn item(id: Uuid, replace: bool, allow_additional_session: bool) -> AttachItem {
    AttachItem::Session {
        id,
        replace,
        allow_additional_session,
    }
}

fn occupant(dir: &str, name: &str) -> Session {
    Session::restored(
        SessionId::from_uuid(Uuid::new_v4()),
        SessionLocation::Worktree(dir.into()),
        SessionLabel::Named(name.into()),
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

/// A project with a git worktree `alpha` that already has `occupant`, as a state and a client.
async fn occupied_alpha() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Arc<DaemonState>,
    Client,
    Session,
) {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "alpha");
    let mut catalog = catalog_at(store.path(), project.path());
    let old = occupant("alpha", "Old");
    catalog.attach_session(project.path(), old.clone()).unwrap();
    let state = Arc::new(DaemonState::new(catalog));
    let client = connect_and_attach(&state, project.path()).await;
    (project, store, state, client, old)
}

fn alpha_cwd(project: &Path) -> PathBuf {
    project.join(".claude/worktrees/alpha")
}

/// `(id, main)` of the project's listed sessions.
fn listed(state: &DaemonState, project: &Path) -> Vec<(Uuid, bool)> {
    state
        .sessions_for(project)
        .into_iter()
        .map(|s| (s.id.0, s.main))
        .collect()
}

#[tokio::test]
async fn both_replace_and_keep_both_on_one_item_is_refused_and_changes_nothing() {
    let (project, _store, state, mut client, old) = occupied_alpha().await;
    let new = seed_claude(&alpha_cwd(project.path()));
    let (_, results) =
        apply_items(&mut client, project.path(), 1, vec![item(new, true, true)]).await;
    assert!(matches!(results[0].outcome, AttachOutcome::Refused(_)));
    assert_eq!(listed(&state, project.path()), [(old.id.0, true)]);
}

#[tokio::test]
async fn resuming_into_an_occupied_worktree_is_refused_naming_the_occupant() {
    let (project, _store, state, mut client, old) = occupied_alpha().await;
    let new = seed_claude(&alpha_cwd(project.path()));
    let (_, results) = apply_items(
        &mut client,
        project.path(),
        1,
        vec![item(new, false, false)],
    )
    .await;
    assert_eq!(
        results[0].outcome,
        AttachOutcome::Refused(RefuseReason::LocationHasSession)
    );
    let existing = results[0].existing.as_ref().expect("names the occupant");
    assert_eq!((existing.id, existing.name.as_str()), (old.id, "Old"));
    assert_eq!(listed(&state, project.path()), [(old.id.0, true)]);
}

#[tokio::test]
async fn replace_lists_only_the_resumed_session_and_it_inherits_main() {
    let (project, _store, state, mut client, old) = occupied_alpha().await;
    let new = seed_claude(&alpha_cwd(project.path()));
    let (_, results) =
        apply_items(&mut client, project.path(), 1, vec![item(new, true, false)]).await;
    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert_eq!(listed(&state, project.path()), [(new, true)]);
    assert!(
        state.live_session(old.id).is_none(),
        "nothing of the replaced session runs"
    );
}

#[tokio::test]
async fn keep_both_adds_a_helper_beside_the_occupant_and_stops_nothing() {
    let (project, _store, state, mut client, old) = occupied_alpha().await;
    let new = seed_claude(&alpha_cwd(project.path()));
    let (_, results) =
        apply_items(&mut client, project.path(), 1, vec![item(new, false, true)]).await;
    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert_eq!(
        listed(&state, project.path()),
        [(old.id.0, true), (new, false)],
        "the occupant stays main, the resumed one is a helper"
    );
}

#[tokio::test]
async fn keep_both_on_a_free_location_is_ignored() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "alpha");
    let state = fresh_state(store.path(), project.path());
    let mut client = connect_and_attach(&state, project.path()).await;
    let new = seed_claude(&alpha_cwd(project.path()));
    let (_, results) =
        apply_items(&mut client, project.path(), 1, vec![item(new, false, true)]).await;
    assert_eq!(results[0].outcome, AttachOutcome::Attached);
    assert_eq!(listed(&state, project.path()), [(new, true)]);
}

/// A write that fails leaves the replaced session listed and the new one absent (research R7).
#[test]
fn a_failed_write_on_replace_leaves_the_old_session_listed() {
    let project = PathBuf::from("/p");
    let old = occupant("alpha", "Old");
    let mut workspace = Workspace {
        projects: vec![Project::new(project.clone(), true, Availability::Available)],
        ..Default::default()
    };
    workspace
        .sessions
        .insert(project.clone(), vec![old.clone()]);
    let store = micold_core::store::FakeProjectStore::loaded(workspace)
        .failing_save(std::io::ErrorKind::PermissionDenied);
    let store_dir = tempfile::tempdir().unwrap();
    let mut catalog = Catalog::load(
        Box::new(store),
        Box::new(JsonFileSettingsStore::at(
            store_dir.path().join("settings.json"),
        )),
    );
    let new = occupant("alpha", "New");
    assert!(catalog.replace_session(&project, old.id, new).is_err());
    let ids: Vec<_> = catalog
        .sessions_for(&project)
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(
        ids,
        [old.id],
        "the old session is still listed, un-archived"
    );
}

#[tokio::test]
async fn a_batch_keeps_the_most_recent_session_per_location_and_a_single_resume_is_never_capped() {
    provider_home();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    provider_worktree(project.path(), "alpha");
    provider_worktree(project.path(), "beta");
    let mut catalog = catalog_at(store.path(), project.path());
    let beta_owner = occupant("beta", "Beta");
    catalog
        .attach_session(project.path(), beta_owner.clone())
        .unwrap();
    let state = Arc::new(DaemonState::new(catalog));
    let mut client = connect_and_attach(&state, project.path()).await;
    let alpha = alpha_cwd(project.path());
    let older = seed_claude(&alpha);
    let newer = seed_claude(&alpha);
    let beta_new = seed_claude(&project.path().join(".claude/worktrees/beta"));
    let touch = |cwd: &Path, id: Uuid, secs: u64| {
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let file = std::fs::File::options()
            .write(true)
            .open(
                provider_home()
                    .join(".claude/projects")
                    .join(encoded)
                    .join(format!("{id}.jsonl")),
            )
            .unwrap();
        file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    };
    touch(&alpha, older, 1_000);
    touch(&alpha, newer, 2_000);

    let (_, results) = apply_items(
        &mut client,
        project.path(),
        1,
        vec![
            item(older, false, false),
            item(newer, false, false),
            item(beta_new, false, false),
        ],
    )
    .await;
    assert_eq!(
        results[0].outcome,
        AttachOutcome::Refused(RefuseReason::LocationHasSession)
    );
    assert_eq!(
        results[1].outcome,
        AttachOutcome::Attached,
        "the most recent"
    );
    assert_eq!(
        results[2].outcome,
        AttachOutcome::Refused(RefuseReason::LocationHasSession),
        "an occupied location catalogues none"
    );
    let ids: Vec<Uuid> = listed(&state, project.path())
        .into_iter()
        .map(|(i, _)| i)
        .collect();
    assert!(ids.contains(&newer) && !ids.contains(&older) && !ids.contains(&beta_new));
    let again = discover(&mut client, project.path(), 2).await;
    assert!(
        again.sessions.iter().any(|s| s.id == older),
        "the capped session stays resumable"
    );

    // A single explicit resume is the user's pick: free location, older session, not capped.
    let store2 = tempfile::tempdir().unwrap();
    let project2 = tempfile::tempdir().unwrap();
    init_git_repo(project2.path());
    provider_worktree(project2.path(), "alpha");
    let state2 = fresh_state(store2.path(), project2.path());
    let mut client2 = connect_and_attach(&state2, project2.path()).await;
    let a = alpha_cwd(project2.path());
    let first = seed_claude(&a);
    let _second = seed_claude(&a);
    let (_, results) = apply_items(
        &mut client2,
        project2.path(),
        1,
        vec![item(first, false, false)],
    )
    .await;
    assert_eq!(results[0].outcome, AttachOutcome::Attached);
}

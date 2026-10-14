//! Feature 776, US1: a location holds at most one session, and the daemon is what enforces it.
//!
//! Every test drives the real `SessionCreate` request over a connection, so the refusal is the one
//! a client sees, and reads the catalog back to prove a refused request changed nothing.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, OperationResult, SessionDefinition,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{FakeProjectStore, JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/mcp.rs"]
mod mcp;
use conn::{connect, Client};
use mcp::{add_worktree, init_repo, sid};

const BOUND: Duration = Duration::from_secs(20);

/// A project at `project` with `sessions`, and a catalog over it. `running` is marked running.
fn state_with(
    project: &Path,
    store_dir: &Path,
    sessions: Vec<Session>,
    running: &[SessionId],
) -> Arc<DaemonState> {
    let mut map = BTreeMap::new();
    map.insert(project.to_path_buf(), sessions);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: map,
        ..Default::default()
    };
    let mut catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    );
    for id in running {
        catalog.mark_session_running(*id);
    }
    Arc::new(DaemonState::new(catalog))
}

fn shell(id: SessionId, worktree: Option<&str>) -> Session {
    let location = match worktree {
        Some(dir) => SessionLocation::Worktree(dir.to_string()),
        None => SessionLocation::Default,
    };
    Session::restored(
        id,
        location,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    )
}

fn create(req: u64, project: &Path, dir: &str) -> ClientMsg {
    create_with(req, project, dir, false)
}

fn create_with(req: u64, project: &Path, dir: &str, allow_additional_session: bool) -> ClientMsg {
    ClientMsg::SessionCreate {
        req,
        project: project.to_path_buf(),
        worktree_dir: dir.to_string(),
        provider: AiCli::ClaudeCode,
        allow_additional_session,
    }
}

/// The first `OperationError`/`SessionCreated` answering `req`.
async fn answer(client: &mut Client, req: u64) -> DaemonMsg {
    tokio::time::timeout(BOUND, async {
        loop {
            match client.next().await.expect("stream open").unwrap() {
                Frame::Control(
                    m @ (DaemonMsg::OperationError { req: r, .. }
                    | DaemonMsg::OperationOk { req: r, .. }),
                ) if r == req => return m,
                _ => continue,
            }
        }
    })
    .await
    .expect("the request is answered")
}

fn count(state: &DaemonState, project: &Path) -> usize {
    state.sessions_for(project).len()
}

fn kill_all(state: &DaemonState, project: &Path) {
    for s in state.sessions_for(project) {
        for pty in state.remove_session(s.id) {
            let _ = pty.kill();
        }
    }
}

async fn refused_in_worktree_with(running: bool) {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let running_ids: Vec<_> = running.then(|| sid(1)).into_iter().collect();
    let state = state_with(
        project.path(),
        store.path(),
        vec![shell(sid(1), Some("wt"))],
        &running_ids,
    );
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create(1, project.path(), "wt")))
        .await
        .unwrap();
    match answer(&mut client, 1).await {
        DaemonMsg::OperationError {
            kind,
            message,
            detail,
            ..
        } => {
            assert_eq!(kind, ErrorKind::Refused);
            assert!(
                message.contains("already has a main session"),
                "names the occupant: {message}"
            );
            assert_eq!(
                detail.as_deref(),
                Some(sid(1).0.to_string().as_str()),
                "carries the existing session's id"
            );
        }
        other => panic!("a second session is refused: {other:?}"),
    }
    assert_eq!(count(&state, project.path()), 1, "the catalog is unchanged");
}

#[tokio::test]
async fn a_second_session_in_a_worktree_is_refused_when_the_first_is_idle() {
    refused_in_worktree_with(false).await;
}

#[tokio::test]
async fn a_second_session_in_a_worktree_is_refused_when_the_first_is_running() {
    refused_in_worktree_with(true).await;
}

#[tokio::test]
async fn a_second_session_in_the_project_root_is_refused() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    let state = state_with(project.path(), store.path(), vec![shell(sid(1), None)], &[]);
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create(1, project.path(), "")))
        .await
        .unwrap();
    assert!(matches!(
        answer(&mut client, 1).await,
        DaemonMsg::OperationError {
            kind: ErrorKind::Refused,
            ..
        }
    ));
    assert_eq!(count(&state, project.path()), 1);
}

#[tokio::test]
async fn a_worktree_with_only_an_archived_session_accepts_one() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let mut old = shell(sid(1), Some("wt"));
    old.archive();
    let state = state_with(project.path(), store.path(), vec![old], &[]);
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create(1, project.path(), "wt")))
        .await
        .unwrap();
    assert!(
        matches!(answer(&mut client, 1).await, DaemonMsg::OperationOk { .. }),
        "an archived session does not occupy the worktree"
    );
    assert_eq!(
        count(&state, project.path()),
        1,
        "the new session; archived ones are not listed"
    );
    kill_all(&state, project.path());
}

/// FR-009: requests racing for one empty worktree create exactly one session.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hundred_concurrent_creates_for_an_empty_worktree_make_exactly_one() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(project.path(), store.path(), vec![], &[]);

    let mut tasks = Vec::new();
    for c in 0..10u64 {
        let state = Arc::clone(&state);
        let path = project.path().to_path_buf();
        tasks.push(tokio::spawn(async move {
            let mut client = connect(&state).await;
            for i in 0..10u64 {
                client
                    .send(Frame::Control(create(c * 10 + i, &path, "wt")))
                    .await
                    .unwrap();
            }
            let mut refused = 0;
            let mut other = 0;
            for _ in 0..10 {
                match tokio::time::timeout(BOUND, async {
                    loop {
                        match client.next().await.expect("stream open").unwrap() {
                            Frame::Control(
                                m @ (DaemonMsg::OperationError { .. }
                                | DaemonMsg::OperationOk { .. }),
                            ) => return m,
                            _ => continue,
                        }
                    }
                })
                .await
                {
                    Ok(DaemonMsg::OperationError {
                        kind: ErrorKind::Refused,
                        ..
                    }) => refused += 1,
                    Ok(_) => other += 1,
                    Err(_) => break,
                }
            }
            (refused, other)
        }));
    }
    let (mut refused, mut other) = (0, 0);
    for t in tasks {
        let (r, o) = t.await.unwrap();
        refused += r;
        other += o;
    }
    assert_eq!(refused, 99, "every request but one is refused");
    assert!(other <= 1, "at most one request was accepted: {other}");
    assert_eq!(count(&state, project.path()), 1, "one session exists");
    kill_all(&state, project.path());
}

/// The first session's record as JSON, to prove a request beside it left it byte-identical.
fn record_of(state: &DaemonState, project: &Path, id: SessionId) -> String {
    let session = state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .expect("the session is listed");
    format!("{session:?}")
}

/// FR-015, scenario 5: the explicit request adds a second session and leaves the first untouched.
async fn soft_limit_adds_beside(dir: &str, worktree: Option<&str>) {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    if let Some(wt) = worktree {
        add_worktree(project.path(), wt);
    }
    let state = state_with(
        project.path(),
        store.path(),
        vec![shell(sid(1), worktree)],
        &[],
    );
    let before = record_of(&state, project.path(), sid(1));
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create_with(1, project.path(), dir, true)))
        .await
        .unwrap();
    assert!(
        matches!(answer(&mut client, 1).await, DaemonMsg::OperationOk { .. }),
        "the explicit request is honoured"
    );
    assert_eq!(count(&state, project.path()), 2, "a second session exists");
    assert_eq!(
        record_of(&state, project.path(), sid(1)),
        before,
        "the first session's record is byte-identical"
    );
    kill_all(&state, project.path());
}

#[tokio::test]
async fn soft_limit_flag_adds_a_second_session_in_an_occupied_worktree() {
    soft_limit_adds_beside("wt", Some("wt")).await;
}

#[tokio::test]
async fn soft_limit_flag_adds_a_second_session_in_the_project_root() {
    soft_limit_adds_beside("", None).await;
}

#[tokio::test]
async fn soft_limit_flag_on_a_free_worktree_creates_its_first_session() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(project.path(), store.path(), vec![], &[]);
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create_with(1, project.path(), "wt", true)))
        .await
        .unwrap();
    assert!(matches!(
        answer(&mut client, 1).await,
        DaemonMsg::OperationOk { .. }
    ));
    assert_eq!(count(&state, project.path()), 1);
    kill_all(&state, project.path());
}

#[tokio::test]
async fn soft_limit_without_the_flag_is_still_refused_and_names_the_way_forward() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(
        project.path(),
        store.path(),
        vec![shell(sid(1), Some("wt"))],
        &[],
    );
    let mut client = connect(&state).await;
    client
        .send(Frame::Control(create_with(1, project.path(), "wt", false)))
        .await
        .unwrap();
    match answer(&mut client, 1).await {
        DaemonMsg::OperationError {
            kind: ErrorKind::Refused,
            message,
            ..
        } => assert!(
            message.contains("allow_additional_session: true"),
            "FR-003: {message}"
        ),
        other => panic!("refused without the flag: {other:?}"),
    }
    assert_eq!(count(&state, project.path()), 1);
}

/// Requests that carry the flag each create their own session (FR-005).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soft_limit_concurrent_flagged_creates_each_make_a_session() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(project.path(), store.path(), vec![], &[]);
    let mut client = connect(&state).await;
    // One at a time: `answer` drops frames for other requests, so overlapping sends could lose one.
    for i in 0..5u64 {
        client
            .send(Frame::Control(create_with(i, project.path(), "wt", true)))
            .await
            .unwrap();
        assert!(matches!(
            answer(&mut client, i).await,
            DaemonMsg::OperationOk { .. }
        ));
    }
    assert_eq!(count(&state, project.path()), 5);
    kill_all(&state, project.path());
}

// --- US2: `WorktreeCreate` with a definition (FR-006) ---------------------------------------------

/// `PATH` is process-wide: the tests below set it to a directory they control, one at a time.
static PATH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Set `PATH` to `bin` (and the system's) for a test; restored on drop.
struct PathGuard(Option<std::ffi::OsString>);

impl PathGuard {
    fn set(bin: &Path) -> Self {
        let saved = std::env::var_os("PATH");
        let paths = [bin.to_path_buf(), "/usr/bin".into(), "/bin".into()];
        std::env::set_var("PATH", std::env::join_paths(paths).unwrap());
        Self(saved)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// A `claude` stand-in that reads its input until closed.
#[cfg(unix)]
fn stand_in_claude(bin: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = bin.join("claude");
    std::fs::write(&path, "#!/bin/sh\nexec cat > /dev/null\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn worktree_create(
    req: u64,
    project: &Path,
    dir: &str,
    session: Option<SessionDefinition>,
) -> ClientMsg {
    ClientMsg::WorktreeCreate {
        req,
        project: project.to_path_buf(),
        branch: dir.to_string(),
        dir_name: dir.to_string(),
        mode: micold_core::worktree::CreateMode::NewBranch,
        session,
    }
}

fn worktree_dirs(project: &Path) -> Vec<String> {
    std::fs::read_dir(project.join(".claude/worktrees"))
        .map(|dir| {
            dir.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// US2 s1/s5 at the wire: a definition makes the worktree and its session together and the reply
/// names the session; none leaves the worktree without one, as before.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn worktree_create_with_a_definition_creates_the_session_with_it() {
    let _guard = PATH_LOCK.lock().await;
    let bin = tempfile::tempdir().unwrap();
    stand_in_claude(bin.path());
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    let state = state_with(project.path(), store.path(), vec![], &[]);
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(worktree_create(
            1,
            project.path(),
            "plain",
            None,
        )))
        .await
        .unwrap();
    match answer(&mut client, 1).await {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { dir_name, session },
            ..
        } => {
            assert_eq!(dir_name, "plain");
            assert_eq!(session, None, "no definition, no session");
        }
        other => panic!("created: {other:?}"),
    }
    assert_eq!(count(&state, project.path()), 0);

    let definition = SessionDefinition::new(AiCli::ClaudeCode, None);
    client
        .send(Frame::Control(worktree_create(
            2,
            project.path(),
            "with-session",
            Some(definition),
        )))
        .await
        .unwrap();
    let id = match answer(&mut client, 2).await {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { session, .. },
            ..
        } => session.expect("the reply names the session"),
        other => panic!("created: {other:?}"),
    };
    let sessions = state.sessions_for(project.path());
    assert_eq!(sessions.len(), 1, "one session");
    assert_eq!(sessions[0].id, id);
    assert_eq!(sessions[0].worktree_dir.as_deref(), Some("with-session"));
    kill_all(&state, project.path());
}

/// US2 s4 at the wire: a CLI that is not available leaves no worktree and no session.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn worktree_create_with_a_missing_cli_creates_nothing() {
    let _guard = PATH_LOCK.lock().await;
    let bin = tempfile::tempdir().unwrap();
    let _path = PathGuard::set(bin.path());
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    let state = state_with(project.path(), store.path(), vec![], &[]);
    let mut client = connect(&state).await;
    let before = worktree_dirs(project.path());

    let definition = SessionDefinition::new(AiCli::Pi, Some("go".into()));
    client
        .send(Frame::Control(worktree_create(
            1,
            project.path(),
            "nothing",
            Some(definition),
        )))
        .await
        .unwrap();
    assert!(
        matches!(
            answer(&mut client, 1).await,
            DaemonMsg::OperationError {
                kind: ErrorKind::Refused,
                ..
            }
        ),
        "a missing CLI is refused"
    );
    assert_eq!(count(&state, project.path()), 0, "no session");
    assert_eq!(worktree_dirs(project.path()), before, "no worktree");
}

// --- US7: the main session (FR-020 to FR-022, SC-008) ---------------------------------------------

/// The `main` marker of each of `project`'s sessions, in list order, with their ids.
fn mains(state: &DaemonState, project: &Path) -> Vec<(SessionId, bool)> {
    state
        .sessions_for(project)
        .into_iter()
        .map(|s| (s.id, s.main))
        .collect()
}

/// Wait until `ready` holds: `SessionSetMain` and the catalog update that answers it are
/// asynchronous.
async fn until(state: &DaemonState, project: &Path, ready: impl Fn(&[(SessionId, bool)]) -> bool) {
    tokio::time::timeout(BOUND, async {
        while !ready(&mains(state, project)) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the catalog reaches the expected state");
}

async fn send(client: &mut Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.unwrap();
}

/// Create a session in `wt` and return its id.
async fn create_in_wt(
    state: &DaemonState,
    client: &mut Client,
    project: &Path,
    req: u64,
    flag: bool,
) -> SessionId {
    let before: Vec<SessionId> = mains(state, project).into_iter().map(|(i, _)| i).collect();
    send(client, create_with(req, project, "wt", flag)).await;
    assert!(matches!(
        answer(client, req).await,
        DaemonMsg::OperationOk { .. }
    ));
    mains(state, project)
        .into_iter()
        .map(|(i, _)| i)
        .find(|i| !before.contains(i))
        .expect("a session was added")
}

async fn delete(client: &mut Client, req: u64, session: SessionId) {
    send(client, ClientMsg::SessionDelete { req, session }).await;
    assert!(matches!(
        answer(client, req).await,
        DaemonMsg::OperationOk { .. }
    ));
}

fn named(id: SessionId, name: &str, main: bool) -> Session {
    let mut s = shell(id, Some("wt"));
    s.label = SessionLabel::Named(name.into());
    s.main = main;
    s
}

#[tokio::test]
async fn main_session_the_session_created_on_a_free_worktree_is_main() {
    for flag in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        add_worktree(project.path(), "wt");
        let state = state_with(project.path(), store.path(), vec![], &[]);
        let mut client = connect(&state).await;
        let id = create_in_wt(&state, &mut client, project.path(), 1, flag).await;
        assert_eq!(
            mains(&state, project.path()),
            vec![(id, true)],
            "flag {flag}"
        );
        kill_all(&state, project.path());
    }
}

#[tokio::test]
async fn main_session_a_flagged_create_on_an_occupied_worktree_is_a_helper() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(project.path(), store.path(), vec![], &[]);
    let mut client = connect(&state).await;
    let first = create_in_wt(&state, &mut client, project.path(), 1, false).await;
    let second = create_in_wt(&state, &mut client, project.path(), 2, true).await;
    assert_eq!(
        mains(&state, project.path()),
        vec![(first, true), (second, false)]
    );
    kill_all(&state, project.path());
}

#[tokio::test]
async fn main_session_set_main_moves_the_role_in_one_write_and_noops_otherwise() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(
        project.path(),
        store.path(),
        vec![
            named(sid(1), "one", true),
            named(sid(2), "two", false),
            named(sid(3), "three", false),
        ],
        &[],
    );
    let mut client = connect(&state).await;
    send(&mut client, ClientMsg::SessionSetMain { id: sid(2) }).await;
    until(&state, project.path(), |m| {
        m == [(sid(1), false), (sid(2), true), (sid(3), false)]
    })
    .await;
    // Already main, unknown and archived ids change nothing.
    let settled = mains(&state, project.path());
    send(&mut client, ClientMsg::SessionSetMain { id: sid(2) }).await;
    send(&mut client, ClientMsg::SessionSetMain { id: sid(99) }).await;
    delete(&mut client, 5, sid(3)).await;
    send(&mut client, ClientMsg::SessionSetMain { id: sid(3) }).await;
    // A round trip after the fire-and-forget messages orders them before the check.
    send(
        &mut client,
        ClientMsg::SessionDelete {
            req: 6,
            session: sid(99),
        },
    )
    .await;
    assert!(matches!(
        answer(&mut client, 6).await,
        DaemonMsg::OperationError { .. }
    ));
    assert_eq!(mains(&state, project.path()).len(), 2);
    assert_eq!(
        mains(&state, project.path()),
        settled
            .into_iter()
            .filter(|(i, _)| *i != sid(3))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn main_session_set_main_survives_a_daemon_restart() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let file = store.path().join("projects.json");
    let mut map = BTreeMap::new();
    map.insert(
        project.path().to_path_buf(),
        vec![named(sid(1), "one", true), named(sid(2), "two", false)],
    );
    let workspace = Workspace {
        projects: vec![Project::new(
            project.path().to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.path().to_path_buf()),
        sessions: map,
        ..Default::default()
    };
    JsonFileStore::at(file.clone()).save(&workspace).unwrap();
    let load = || {
        let catalog = Catalog::load(
            Box::new(JsonFileStore::at(file.clone())),
            Box::new(JsonFileSettingsStore::at(
                store.path().join("settings.json"),
            )),
        );
        Arc::new(DaemonState::new(catalog))
    };
    let state = load();
    let mut client = connect(&state).await;
    send(&mut client, ClientMsg::SessionSetMain { id: sid(2) }).await;
    until(&state, project.path(), |m| {
        m == [(sid(1), false), (sid(2), true)]
    })
    .await;
    drop(client);
    drop(state);
    let restarted = load();
    assert_eq!(
        mains(&restarted, project.path()),
        vec![(sid(1), false), (sid(2), true)]
    );
}

#[tokio::test]
async fn main_session_archiving_the_main_promotes_the_first_remaining() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(
        project.path(),
        store.path(),
        vec![
            named(sid(1), "one", false),
            named(sid(2), "two", true),
            named(sid(3), "three", false),
        ],
        &[],
    );
    let mut client = connect(&state).await;
    // A helper's delete changes nothing else.
    delete(&mut client, 1, sid(3)).await;
    assert_eq!(
        mains(&state, project.path()),
        vec![(sid(1), false), (sid(2), true)]
    );
    // The main's delete promotes the first remaining, in the same write.
    delete(&mut client, 2, sid(2)).await;
    assert_eq!(mains(&state, project.path()), vec![(sid(1), true)]);
    // The only session's delete frees the location; the next create is its main.
    delete(&mut client, 3, sid(1)).await;
    assert!(mains(&state, project.path()).is_empty());
    let id = create_in_wt(&state, &mut client, project.path(), 4, false).await;
    assert_eq!(mains(&state, project.path()), vec![(id, true)]);
    kill_all(&state, project.path());
}

#[tokio::test]
async fn main_session_a_catalog_without_the_marker_derives_its_main_and_rewrites_nothing() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let mut gone = named(sid(1), "archived", false);
    gone.archived = true;
    let state = state_with(
        project.path(),
        store.path(),
        vec![
            gone,
            named(sid(2), "first", false),
            named(sid(3), "second", false),
        ],
        &[],
    );
    // No record gained the marker on load.
    assert_eq!(
        mains(&state, project.path()),
        vec![(sid(2), false), (sid(3), false)]
    );
    // The refusal names the derived main, the first un-archived session.
    let mut client = connect(&state).await;
    send(&mut client, create_with(1, project.path(), "wt", false)).await;
    match answer(&mut client, 1).await {
        DaemonMsg::OperationError { message, .. } => {
            assert!(message.contains("main session: \"first\""), "{message}");
        }
        other => panic!("expected a refusal: {other:?}"),
    }
}

#[tokio::test]
async fn main_session_the_refusal_names_the_marked_main_not_the_first() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    add_worktree(project.path(), "wt");
    let state = state_with(
        project.path(),
        store.path(),
        vec![named(sid(1), "helper", false), named(sid(2), "chief", true)],
        &[],
    );
    let mut client = connect(&state).await;
    send(&mut client, create_with(1, project.path(), "wt", false)).await;
    match answer(&mut client, 1).await {
        DaemonMsg::OperationError { message, .. } => {
            assert!(message.contains("main session: \"chief\""), "{message}");
        }
        other => panic!("expected a refusal: {other:?}"),
    }
}

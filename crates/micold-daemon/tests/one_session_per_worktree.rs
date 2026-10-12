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
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::FakeProjectStore;
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
    ClientMsg::SessionCreate {
        req,
        project: project.to_path_buf(),
        worktree_dir: dir.to_string(),
        provider: AiCli::ClaudeCode,
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
                message.contains("already has a session"),
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
        matches!(
            answer(&mut client, 1).await,
            DaemonMsg::OperationOk { .. }
        ),
        "an archived session does not occupy the worktree"
    );
    assert_eq!(count(&state, project.path()), 1, "the new session; archived ones are not listed");
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

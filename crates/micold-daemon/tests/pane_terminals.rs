//! Feature 484 (terminal panes), daemon half: several terminals of one client are streamed at
//! once, each at its own size, and input and resizes address one `(session, process)`.
//!
//! (Unix-only; see the gate below.)
// unix-only: the stand-in processes are `cat`, which Windows lacks.
#![cfg(unix)]

#[path = "support/conn.rs"]
mod conn;

use conn::{connect_as, Client};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{ClientMsg, SessionProcess, TerminalRef};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, ShellInstanceId, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

fn text(pty: &PtySession) -> String {
    let term = pty.term().lock();
    let grid = term.grid();
    let mut out = String::new();
    for line in 0..grid.screen_lines() {
        for col in 0..grid.columns() {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

fn size(pty: &PtySession) -> (usize, usize) {
    let term = pty.term().lock();
    (term.grid().columns(), term.grid().screen_lines())
}

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

fn cat(id: SessionId, size: (u16, u16)) -> PtySession {
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 1_000, Some(size)).expect("spawn cat")
}

/// A catalog holding `sessions` at the project root (a real dir, so a shell instance can `cwd`).
fn catalog_with(project: &Path, store: &Path, ids: &[SessionId]) -> Catalog {
    let sessions_vec = ids
        .iter()
        .map(|id| {
            Session::restored(
                *id,
                SessionLocation::Default,
                SessionLabel::Named("S".into()),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )
        })
        .collect();
    let mut sessions = BTreeMap::new();
    sessions.insert(project.to_path_buf(), sessions_vec);
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

fn primary(session: SessionId) -> TerminalRef {
    TerminalRef {
        session,
        process: SessionProcess::Primary,
    }
}

fn shell(session: SessionId, n: u32) -> TerminalRef {
    TerminalRef {
        session,
        process: SessionProcess::Shell(ShellInstanceId(n)),
    }
}

/// Grid frames that arrive within `window`, whatever they are.
async fn frames_within(client: &mut Client, window: Duration) -> Vec<GridFrame> {
    let end = tokio::time::Instant::now() + window;
    let mut out = Vec::new();
    while let Ok(next) = tokio::time::timeout_at(end, client.next()).await {
        match next {
            Some(Ok(Frame::Grid(f))) => out.push(f),
            Some(Ok(_)) => {}
            _ => break,
        }
    }
    out
}

async fn view(client: &mut Client, terminals: Vec<TerminalRef>) {
    client
        .send(Frame::Control(ClientMsg::SetViewedTerminals {
            project: PathBuf::from("/repo/demo"),
            terminals,
        }))
        .await
        .unwrap();
}

#[test]
fn input_and_resize_with_a_process_act_on_that_pty_only() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let sid = SessionId::new();
    let state = DaemonState::new(catalog_with(project.path(), store.path(), &[sid]));
    let primary_pty = state.register_session(cat(sid, (80, 24)));
    state
        .open_shell(sid, ShellInstanceId(1))
        .expect("open shell");
    let (shell_pty, _) = state
        .attach_process(sid, SessionProcess::Shell(ShellInstanceId(1)))
        .expect("attach");
    state.attach_process(sid, SessionProcess::Primary).unwrap();

    // Some(process) wins over whatever is attached (Primary here).
    state.session_input_to(
        sid,
        0,
        b"to_shell\n",
        Some(SessionProcess::Shell(ShellInstanceId(1))),
    );
    assert!(wait_until(Duration::from_secs(5), || text(&shell_pty)
        .contains("to_shell")));
    assert!(!text(&primary_pty).contains("to_shell"));

    // None is today's behaviour: the attached process.
    state.session_input_to(sid, 1, b"to_primary\n", None);
    assert!(wait_until(Duration::from_secs(5), || text(&primary_pty)
        .contains("to_primary")));
    assert!(!text(&shell_pty).contains("to_primary"));

    // Two panes of one session hold distinct sizes (T018's daemon half).
    state.resize_terminal(sid, Some(SessionProcess::Primary), 100, 30);
    state.resize_terminal(sid, Some(SessionProcess::Shell(ShellInstanceId(1))), 60, 12);
    assert!(wait_until(Duration::from_secs(5), || {
        size(&primary_pty) == (100, 30) && size(&shell_pty) == (60, 12)
    }));

    // A terminal the session does not have is ignored; the others keep their size.
    state.resize_terminal(sid, Some(SessionProcess::Shell(ShellInstanceId(9))), 10, 10);
    state.session_input_to(
        sid,
        2,
        b"x",
        Some(SessionProcess::Shell(ShellInstanceId(9))),
    );
    assert_eq!(size(&primary_pty), (100, 30));
    assert_eq!(size(&shell_pty), (60, 12));

    // None keeps the old reach: every process of the session.
    state.resize_terminal(sid, None, 90, 20);
    assert!(wait_until(Duration::from_secs(5), || {
        size(&primary_pty) == (90, 20) && size(&shell_pty) == (90, 20)
    }));
    for p in state.remove_session(sid) {
        let _ = p.kill();
    }
}

#[tokio::test]
async fn set_viewed_terminals_streams_exactly_the_named_terminals_and_replaces_the_set() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = (SessionId::new(), SessionId::new());
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &[a, b],
    )));
    let pa = state.register_session(cat(a, (80, 24)));
    let _pb = state.register_session(cat(b, (80, 24)));
    state.open_shell(a, ShellInstanceId(1)).unwrap();
    let mut client = connect_as(&state, "test-client").await;

    // Two processes of one session, and another session.
    view(&mut client, vec![primary(a), shell(a, 1), primary(b)]).await;
    let first = frames_within(&mut client, Duration::from_millis(800)).await;
    for want in [
        (a, SessionProcess::Primary),
        (a, SessionProcess::Shell(ShellInstanceId(1))),
        (b, SessionProcess::Primary),
    ] {
        assert!(
            first
                .iter()
                .any(|f| f.full && (f.session, f.process) == want),
            "no full frame for {want:?}"
        );
    }

    // Replacing the set stops the rest: output of a dropped terminal never arrives.
    view(&mut client, vec![primary(b)]).await;
    let _ = frames_within(&mut client, Duration::from_millis(300)).await;
    pa.write_input(b"after_drop\n").unwrap();
    state.session_input_to(b, 0, b"kept\n", None);
    let later = frames_within(&mut client, Duration::from_millis(800)).await;
    assert!(
        later.iter().all(|f| f.session == b),
        "a dropped terminal still streams"
    );
    assert!(later
        .iter()
        .any(|f| f.lines.iter().any(|l| l.text.contains("kept"))));

    // Unknown terminals are ignored, not an error.
    view(&mut client, vec![primary(SessionId::new()), shell(b, 7)]).await;
    assert!(frames_within(&mut client, Duration::from_millis(300))
        .await
        .is_empty());
    for s in [a, b] {
        for p in state.remove_session(s) {
            let _ = p.kill();
        }
    }
}

/// SC-004 / FR-001: at most 6 terminals stream to one client; a 7th named is dropped, not an error.
#[tokio::test]
async fn at_most_six_terminals_stream_to_one_client() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let ids: Vec<SessionId> = (0..7).map(|_| SessionId::new()).collect();
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &ids,
    )));
    for id in &ids {
        let _ = state.register_session(cat(*id, (80, 24)));
    }
    let mut client = connect_as(&state, "test-client").await;
    view(&mut client, ids.iter().map(|i| primary(*i)).collect()).await;
    let frames = frames_within(&mut client, Duration::from_millis(800)).await;
    for (n, id) in ids.iter().enumerate() {
        let streamed = frames.iter().any(|f| f.full && f.session == *id);
        assert_eq!(streamed, n < 6, "terminal {n} streamed = {streamed}");
    }
    for s in ids {
        for p in state.remove_session(s) {
            let _ = p.kill();
        }
    }
}

/// A restart replaces the terminal's PTY: the viewing client's stream of the old one would stay
/// silent, so the new one must be streamed once the session announces itself (review A, M1).
#[tokio::test]
async fn a_restarted_terminal_streams_again_to_the_client_viewing_it() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let a = SessionId::new();
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &[a],
    )));
    let _old = state.register_session(cat(a, (80, 24)));
    let mut client = connect_as(&state, "test-client").await;
    view(&mut client, vec![primary(a)]).await;
    let first = frames_within(&mut client, Duration::from_millis(600)).await;
    assert!(first.iter().any(|f| f.full && f.session == a));

    // The restart: the old PTY goes, a new one takes its place, and the start is announced.
    for p in state.remove_session(a) {
        let _ = p.kill();
    }
    let new = state.register_session(cat(a, (80, 24)));
    state.announce_session_started(a);
    let _ = frames_within(&mut client, Duration::from_millis(600)).await;
    new.write_input(b"after_restart\n").unwrap();
    let later = frames_within(&mut client, Duration::from_millis(1000)).await;
    assert!(
        later
            .iter()
            .any(|f| f.lines.iter().any(|l| l.text.contains("after_restart"))),
        "the new PTY's output never reached the client"
    );
    for p in state.remove_session(a) {
        let _ = p.kill();
    }
}

#[tokio::test]
async fn a_frame_reaches_only_clients_whose_set_contains_it() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = (SessionId::new(), SessionId::new());
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &[a, b],
    )));
    let _pa = state.register_session(cat(a, (80, 24)));
    let _pb = state.register_session(cat(b, (80, 24)));
    let (mut one, mut two) = (
        connect_as(&state, "test-client").await,
        connect_as(&state, "test-client").await,
    );
    view(&mut one, vec![primary(a)]).await;
    view(&mut two, vec![primary(b)]).await;
    state.session_input_to(a, 0, b"only_a\n", None);
    state.session_input_to(b, 0, b"only_b\n", None);
    let (f1, f2) = (
        frames_within(&mut one, Duration::from_millis(800)).await,
        frames_within(&mut two, Duration::from_millis(800)).await,
    );
    assert!(f1.iter().all(|f| f.session == a) && !f1.is_empty());
    assert!(f2.iter().all(|f| f.session == b) && !f2.is_empty());
    let seen = |fs: &[GridFrame], needle: &str| {
        fs.iter()
            .any(|f| f.lines.iter().any(|l| l.text.contains(needle)))
    };
    assert!(seen(&f1, "only_a") && !seen(&f1, "only_b"));
    assert!(seen(&f2, "only_b") && !seen(&f2, "only_a"));
    for s in [a, b] {
        for p in state.remove_session(s) {
            let _ = p.kill();
        }
    }
}

#[tokio::test]
async fn the_first_terminal_is_remembered_as_the_projects_foreground_session() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = (SessionId::new(), SessionId::new());
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &[a, b],
    )));
    let _pa = state.register_session(cat(a, (80, 24)));
    let _pb = state.register_session(cat(b, (80, 24)));
    let mut client = connect_as(&state, "test-client").await;
    client
        .send(Frame::Control(ClientMsg::SetViewedTerminals {
            project: project.path().to_path_buf(),
            terminals: vec![primary(b), primary(a)],
        }))
        .await
        .unwrap();
    let _ = frames_within(&mut client, Duration::from_millis(400)).await;
    // Remembered where `SetViewedSession` always put it: the first terminal's session.
    let saved = JsonFileStore::at(store.path().join("projects.json"))
        .load()
        .workspace;
    assert_eq!(
        saved.foreground_by_project.get(project.path()),
        Some(&b),
        "the first terminal is the foreground session"
    );
    for s in [a, b] {
        for p in state.remove_session(s) {
            let _ = p.kill();
        }
    }
}

/// Constitution II isolation gate (T006a): two sessions in different worktrees, both shown at once
/// to one client; neither's output reaches the other's frames, and ending one leaves the other
/// streaming.
#[tokio::test]
async fn sessions_of_different_worktrees_stay_isolated_and_one_ending_leaves_the_other() {
    let (project, store) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (wt_a, wt_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = (SessionId::new(), SessionId::new());
    let state = Arc::new(DaemonState::new(catalog_with(
        project.path(),
        store.path(),
        &[a, b],
    )));
    let spawn_in = |id: SessionId, dir: &Path| {
        let mut cmd = CommandBuilder::new("cat");
        cmd.cwd(dir);
        PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn")
    };
    let _pa = state.register_session(spawn_in(a, wt_a.path()));
    let _pb = state.register_session(spawn_in(b, wt_b.path()));
    let mut client = connect_as(&state, "test-client").await;
    view(&mut client, vec![primary(a), primary(b)]).await;
    let _ = frames_within(&mut client, Duration::from_millis(500)).await;

    state.session_input_to(a, 0, b"secret_of_a\n", None);
    let frames = frames_within(&mut client, Duration::from_millis(800)).await;
    let has = |fs: &[GridFrame], s: SessionId, needle: &str| {
        fs.iter()
            .any(|f| f.session == s && f.lines.iter().any(|l| l.text.contains(needle)))
    };
    assert!(has(&frames, a, "secret_of_a"));
    assert!(
        !has(&frames, b, "secret_of_a"),
        "A's output reached B's pane"
    );

    for p in state.remove_session(a) {
        let _ = p.kill();
    }
    state.session_input_to(b, 0, b"still_b\n", None);
    let later = frames_within(&mut client, Duration::from_millis(800)).await;
    assert!(
        has(&later, b, "still_b"),
        "B stopped streaming when A ended"
    );
    for p in state.remove_session(b) {
        let _ = p.kill();
    }
}

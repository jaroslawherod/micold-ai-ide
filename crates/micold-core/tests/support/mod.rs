//! T001 — shared test scaffolding for multi-project state (feature 008).
//!
//! Builds a `Workspace` with two-or-more projects, each holding sessions in caller-chosen
//! lifecycles, with no filesystem access. Included via `mod support;` from the feature-008
//! integration tests. Compiles under `cargo test --no-default-features` (pure core only).

#![allow(dead_code)]

use micold_core::fs_scan::FakeFolderScanner;
use micold_core::project::canonicalize_best_effort;
use micold_core::session::{
    RestartDecision, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::workspace::Workspace;
use std::path::{Path, PathBuf};

/// The shared in-memory scanner, primed the way this scaffolding's callers expect: every folder
/// is a git repository and every folder is present. Replaces a hand-written `FakeScanner`
/// (feature 021 T048).
pub fn fake_scanner() -> FakeFolderScanner {
    FakeFolderScanner::new().git_repos(true)
}

/// A session that is currently `Running`.
pub fn running_session(worktree_dir: &str) -> Session {
    let mut s = Session::start_new(SessionLocation::Worktree(worktree_dir.to_string()));
    s.mark_running();
    s
}

/// A `SessionLocation::Default` session that is currently `Running` (feature 010).
pub fn running_default_session() -> Session {
    let mut s = Session::start_new(SessionLocation::Default);
    s.mark_running();
    s
}

/// A persisted session restored as `Idle`.
pub fn idle_session(worktree_dir: &str) -> Session {
    Session::restored(
        SessionId::new(),
        SessionLocation::Worktree(worktree_dir.to_string()),
        SessionLabel::Pending,
        TerminalMode::AiCli,
    )
}

/// A session driven to `Failed` via repeated unexpected exits (crash-loop guard).
pub fn failed_session(worktree_dir: &str) -> Session {
    let mut s = Session::start_new(SessionLocation::Worktree(worktree_dir.to_string()));
    s.mark_running();
    loop {
        if s.on_unexpected_exit() == RestartDecision::GiveUp {
            break;
        }
    }
    s
}

/// Build a `Workspace` with the given `(path, sessions)` projects — all `Available`, **no**
/// active project (caller sets `active` + `active_session` explicitly). Sessions are keyed by
/// the same canonicalized path the app uses.
pub fn workspace_with(projects: Vec<(&str, Vec<Session>)>) -> Workspace {
    let mut ws = Workspace::empty();
    let scanner = fake_scanner();
    for (path, sessions) in projects {
        ws.open_or_activate(PathBuf::from(path), &scanner);
        let key = canonicalize_best_effort(Path::new(path));
        ws.sessions.insert(key, sessions);
    }
    ws.active = None;
    ws
}

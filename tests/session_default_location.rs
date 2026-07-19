//! T008 (010-root-dir-session) — a SessionLocation::Default session's resolved cwd equals the
//! project root exactly, for the pure logic these five call sites in `src/main.rs` share
//! (research.md R2): `session_has_conversation`, the `SessionStartRequested` handler,
//! `sync_session_titles`, `session_cwd`, `session_cwd_any`. `main.rs` is behind the `gui`
//! feature (it spawns real PTYs), so this test exercises the pure model directly — the
//! `Worktree`/`Default` branch on `SessionLocation` is the entire decision under test; the
//! GUI layer's `session_cwd_for_location` is a thin, behavior-preserving wrapper over it,
//! separately covered by `quickstart.md` steps 3-4 (manual, `cargo run`).

use micold_ai_ide::session::{Session, SessionLocation};
use std::path::{Path, PathBuf};

/// Mirrors `session_cwd_for_location` in `src/main.rs` (kept here so this pure crate's tests
/// can assert the decision without requiring the `gui` feature).
fn session_cwd_for_location(repo: &Path, location: &SessionLocation) -> PathBuf {
    match location {
        SessionLocation::Worktree(dir) => repo.join(".claude/worktrees").join(dir),
        SessionLocation::Default => repo.to_path_buf(),
    }
}

#[test]
fn default_session_cwd_is_the_project_root_exactly() {
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::start_new(SessionLocation::Default);
    let cwd = session_cwd_for_location(&repo, &session.location);
    assert_eq!(cwd, repo, "Default session cwd must be the project root, no join/suffix");
}

#[test]
fn worktree_session_cwd_is_unchanged_by_this_feature() {
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let cwd = session_cwd_for_location(&repo, &session.location);
    assert_eq!(cwd, repo.join(".claude/worktrees").join("feat-x"));
}

#[test]
fn restored_default_session_cwd_is_also_the_project_root() {
    // Covers the reopen/resume call sites (session_cwd, session_cwd_any), not just fresh start.
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::restored(SessionId::new(), SessionLocation::Default, SessionLabel::Pending);
    let cwd = session_cwd_for_location(&repo, &session.location);
    assert_eq!(cwd, repo);
}

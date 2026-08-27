//! T008 (010-root-dir-session) — a SessionLocation::Default session's resolved cwd equals the
//! project root exactly, for the pure logic these five call sites in `src/main.rs` share
//! (research.md R2): `session_has_conversation`, the `SessionStartRequested` handler,
//! `sync_session_titles`, `session_cwd_and_mode` (feature 010's mode-aware replacement for the
//! original `session_cwd`), `session_cwd_any`. All five delegate to
//! `SessionLocation::cwd` (`src/session.rs`), the single authoritative implementation of the
//! `Worktree`/`Default` decision — exercised directly here rather than a hand-copied mirror, so
//! this test can't silently drift from the real logic. `main.rs`'s thin wrapper over it is
//! separately covered by `quickstart.md` steps 3-4 (manual, `cargo run`).

use micold_core::session::{AiCli, Session, SessionLocation};
use std::path::PathBuf;

#[test]
fn default_session_cwd_is_the_project_root_exactly() {
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let cwd = session.location.cwd(&repo);
    assert_eq!(
        cwd, repo,
        "Default session cwd must be the project root, no join/suffix"
    );
}

#[test]
fn worktree_session_cwd_is_unchanged_by_this_feature() {
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    let cwd = session.location.cwd(&repo);
    assert_eq!(cwd, repo.join(".claude/worktrees").join("feat-x"));
}

#[test]
fn restored_default_session_cwd_is_also_the_project_root() {
    // Covers the reopen/resume call sites (session_cwd, session_cwd_any), not just fresh start.
    use micold_core::session::{SessionId, SessionLabel, TerminalMode};
    let repo = PathBuf::from("/home/dev/proj");
    let session = Session::restored(
        SessionId::new(),
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    let cwd = session.location.cwd(&repo);
    assert_eq!(cwd, repo);
}

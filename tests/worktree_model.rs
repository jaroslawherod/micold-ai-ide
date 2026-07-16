//! T009 — Worktree model + WorktreeStatus (FR-018a).

use micold_ai_ide::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn worktree(status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status,
    }
}

#[test]
fn session_start_allowed_only_when_valid() {
    assert!(worktree(WorktreeStatus::Valid).can_start_session());
    assert!(!worktree(WorktreeStatus::Missing).can_start_session());
    assert!(!worktree(WorktreeStatus::Invalid).can_start_session());
}

#[test]
fn fields_are_preserved() {
    let w = worktree(WorktreeStatus::Valid);
    assert_eq!(w.dir_name, "feat-x");
    assert_eq!(w.branch.as_deref(), Some("feat/x"));
    assert_eq!(w.path, PathBuf::from("/repo/.claude/worktrees/feat-x"));
}

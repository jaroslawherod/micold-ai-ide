//! T009 — Worktree model + WorktreeStatus (FR-018a).

use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn worktree(status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status,
        included: false,
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

// --- Feature 029: one source for the word a status is shown as (FR-006, contract §4) ---

#[test]
fn an_unhealthy_status_has_a_word_of_its_own() {
    assert_eq!(WorktreeStatus::Missing.label(), Some("missing"));
    assert_eq!(WorktreeStatus::Invalid.label(), Some("invalid"));
}

#[test]
fn a_healthy_status_has_no_word_at_all() {
    // The assertion that matters. The sidebar's status chip used to answer this with `""`, which
    // is a word — an empty one — and left every caller to remember that it had to test for it.
    // `None` makes "normal reads as normal" a fact of the type rather than a convention, so a
    // caller that forgets cannot render a blank chip or a blank tooltip line (§4.3).
    assert_eq!(WorktreeStatus::Valid.label(), None);
}

//! T027 — rollback plan ordering + clean unwind on failure (FR-006b).

use micold_ai_ide::git::{FakeGit, Git};
use micold_ai_ide::naming::DerivedNames;
use micold_ai_ide::worktree::{create_worktree, rollback_plan, CleanupStep, CreateError};
use std::path::PathBuf;

#[test]
fn rollback_plan_order_removes_registration_before_branch() {
    let plan = rollback_plan();
    assert_eq!(
        plan,
        [
            CleanupStep::WorktreeRemove,
            CleanupStep::WorktreePrune,
            CleanupStep::BranchDelete,
            CleanupStep::RemoveDir,
        ]
    );
    // Registration removal must precede branch deletion (git refuses to delete a checked-out
    // branch).
    let remove = plan
        .iter()
        .position(|s| *s == CleanupStep::WorktreeRemove)
        .unwrap();
    let delete = plan
        .iter()
        .position(|s| *s == CleanupStep::BranchDelete)
        .unwrap();
    assert!(remove < delete);
}

#[test]
fn failed_create_rolls_back_leaving_no_orphan_branch_or_worktree() {
    let git = FakeGit::new().with_repo("/repo").failing_next_add();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");

    let err = create_worktree(&git, &repo, &target, &names, false).unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    // The orphan branch git created before the simulated failure must be cleaned up.
    assert!(!git.branch_exists(&repo, "feat/x").unwrap());
    assert!(git.worktrees(&repo).is_empty());
}

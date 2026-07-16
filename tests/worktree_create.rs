//! T026 — create orchestration happy path + duplicate detection (FR-006, FR-009).

use micold_ai_ide::git::{FakeGit, Git};
use micold_ai_ide::naming::DerivedNames;
use micold_ai_ide::worktree::{create_worktree, CreateError, WorktreeStatus};
use std::path::PathBuf;

fn names() -> DerivedNames {
    DerivedNames {
        dir_name: "feat-abc-123-login".to_string(),
        branch: "feat/abc-123-login".to_string(),
    }
}

fn target() -> PathBuf {
    PathBuf::from("/repo/.claude/worktrees/feat-abc-123-login")
}

#[test]
fn happy_path_creates_branch_and_worktree() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");

    let wt = create_worktree(&git, &repo, &target(), &names(), false).unwrap();

    assert_eq!(wt.status, WorktreeStatus::Valid);
    assert_eq!(wt.dir_name, "feat-abc-123-login");
    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    assert!(git.branch_exists(&repo, "feat/abc-123-login").unwrap());
    assert_eq!(git.worktrees(&repo).len(), 1);
}

#[test]
fn duplicate_branch_is_rejected_without_mutation() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/abc-123-login");
    let repo = PathBuf::from("/repo");

    let err = create_worktree(&git, &repo, &target(), &names(), false).unwrap_err();
    assert_eq!(err, CreateError::DuplicateBranch);
    assert!(git.worktrees(&repo).is_empty());
}

#[test]
fn duplicate_target_dir_is_rejected() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    // target_exists = true simulates the fs check.
    let err = create_worktree(&git, &repo, &target(), &names(), true).unwrap_err();
    assert_eq!(err, CreateError::DuplicateDir);
}

#[test]
fn duplicate_registered_worktree_is_rejected() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    // Pre-register the same path via a first successful create.
    create_worktree(&git, &repo, &target(), &names(), false).unwrap();
    // A second create with a different branch but the same path → DuplicateDir.
    let other = DerivedNames {
        dir_name: "feat-abc-123-login".to_string(),
        branch: "feat/other".to_string(),
    };
    let err = create_worktree(&git, &repo, &target(), &other, false).unwrap_err();
    assert_eq!(err, CreateError::DuplicateDir);
}

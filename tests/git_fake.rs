//! T005 — the `Git` boundary exercised via the in-memory `FakeGit`.

use micold_ai_ide::git::{FakeGit, Git};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from("/repo")
}

#[test]
fn is_repo_root_reflects_registration() {
    let git = FakeGit::new().with_repo(repo());
    assert!(git.is_repo_root(&repo()));
    assert!(!git.is_repo_root(Path::new("/not-a-repo")));
}

#[test]
fn branch_exists_reports_registered_branches() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x");
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert!(!git.branch_exists(&repo(), "feat/y").unwrap());
}

#[test]
fn add_registers_branch_and_worktree() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    git.worktree_add_new_branch(&repo(), "feat/x", &path)
        .unwrap();

    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    let porcelain = git.worktree_list_porcelain(&repo()).unwrap();
    assert!(porcelain.contains("worktree /repo/.claude/worktrees/feat-x"));
    assert!(porcelain.contains("branch refs/heads/feat/x"));
}

#[test]
fn remove_and_branch_delete_are_idempotent() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    git.worktree_add_new_branch(&repo(), "feat/x", &path)
        .unwrap();

    git.worktree_remove(&repo(), &path, true).unwrap();
    git.branch_delete(&repo(), "feat/x").unwrap();
    // Second call must not error (idempotent for rollback).
    git.worktree_remove(&repo(), &path, true).unwrap();
    git.branch_delete(&repo(), "feat/x").unwrap();

    assert!(git.worktrees(&repo()).is_empty());
    assert!(!git.branch_exists(&repo(), "feat/x").unwrap());
}

#[test]
fn failing_next_add_leaves_branch_but_no_worktree() {
    let git = FakeGit::new().with_repo(repo()).failing_next_add();
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    assert!(git
        .worktree_add_new_branch(&repo(), "feat/x", &path)
        .is_err());
    // git creates the branch first, then the checkout fails → orphan branch, no worktree.
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert!(git.worktrees(&repo()).is_empty());
}

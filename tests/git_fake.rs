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

// --- Feature 013 US2: branch_delete becomes outcome-based, not always-Ok ---

#[test]
fn branch_delete_succeeds_when_the_branch_is_actually_gone() {
    let git = FakeGit::new().with_repo(repo()).with_branch(repo(), "feat/x");
    assert!(git.branch_delete(&repo(), "feat/x").is_ok());
    assert!(!git.branch_exists(&repo(), "feat/x").unwrap());
}

#[test]
fn failing_next_branch_delete_reports_a_genuine_refusal() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x")
        .failing_next_branch_delete();
    assert!(git.branch_delete(&repo(), "feat/x").is_err());
    // Must not have silently succeeded — the branch really is still there.
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
}

#[test]
fn failing_next_branch_delete_only_fails_once() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x")
        .failing_next_branch_delete();
    assert!(git.branch_delete(&repo(), "feat/x").is_err());
    // Primed to fail only once — a retry succeeds (mirrors failing_next_add's shape).
    assert!(git.branch_delete(&repo(), "feat/x").is_ok());
    assert!(!git.branch_exists(&repo(), "feat/x").unwrap());
}

#[test]
fn has_submodules_reflects_priming() {
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let git = FakeGit::new().with_repo(repo()).with_submodules(&path);
    assert!(git.has_submodules(&path));
    assert!(!git.has_submodules(Path::new("/repo/.claude/worktrees/feat-y")));
}

#[test]
fn submodule_update_records_call_and_defaults_to_success() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    git.submodule_update_init_recursive(&path, &mut |_| {})
        .unwrap();
    assert_eq!(git.submodule_update_calls(), vec![path]);
}

#[test]
fn failing_next_submodule_update_errors_once() {
    let git = FakeGit::new()
        .with_repo(repo())
        .failing_next_submodule_update();
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    assert!(git
        .submodule_update_init_recursive(&path, &mut |_| {})
        .is_err());
    // Primed to fail only once — a retry would succeed (mirrors failing_next_add's shape).
    assert!(git
        .submodule_update_init_recursive(&path, &mut |_| {})
        .is_ok());
}

#[test]
fn submodule_update_reports_primed_progress_lines_in_order() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_submodule_progress_lines(vec![
            "Cloning into 'vendor/sub'...".to_string(),
            "done.".to_string(),
        ]);
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let mut received = Vec::new();
    git.submodule_update_init_recursive(&path, &mut |line| received.push(line))
        .unwrap();
    assert_eq!(
        received,
        vec![
            "Cloning into 'vendor/sub'...".to_string(),
            "done.".to_string(),
        ]
    );
}

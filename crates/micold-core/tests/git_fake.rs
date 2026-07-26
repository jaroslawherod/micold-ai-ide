//! T005 — the `Git` boundary exercised via the in-memory `FakeGit`.

use micold_core::git::{FakeGit, Git};
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
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x");
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

// =======================================================================================
// Feature 016 — ref listing and the existing-branch worktree-add variants
// (contracts/git-trait-branches.md).
// =======================================================================================

use micold_core::worktree::{parse_branch_refs, BranchOrigin};

#[test]
fn list_branch_refs_renders_local_and_remote_refs_in_gits_format() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "main")
        .with_remote_branch(repo(), "origin", "feat/reporting");

    let out = git.list_branch_refs(&repo()).unwrap();
    assert!(out.contains("refs/heads/main"));
    assert!(out.contains("refs/remotes/origin/feat/reporting"));
    // The fake emits the symbolic alias real git emits, so the parser's filtering is exercised.
    assert!(out.contains("refs/remotes/origin/HEAD"));
}

#[test]
fn parse_branch_refs_maps_each_line_shape() {
    let out = "refs/heads/main\n\
               refs/heads/feat/login\n\
               refs/remotes/origin/feat/reporting\n";
    let candidates = parse_branch_refs(out);

    assert_eq!(candidates.len(), 3);
    assert!(candidates
        .iter()
        .any(|c| c.name == "main" && c.origin == BranchOrigin::Local));
    assert!(candidates
        .iter()
        .any(|c| c.name == "feat/login" && c.origin == BranchOrigin::Local));
    assert!(candidates.iter().any(|c| c.name == "feat/reporting"
        && c.origin
            == BranchOrigin::Remote {
                remote: "origin".to_string()
            }));
    // The parser never invents availability — that is the caller's annotation step.
    assert!(candidates.iter().all(|c| c.blocked_by.is_none()));
}

#[test]
fn parse_branch_refs_drops_head_aliases_and_junk() {
    let out = "refs/remotes/origin/HEAD\n\
               refs/heads/main\n\
               \n\
               refs/tags/v1.0\n\
               garbage\n";
    let candidates = parse_branch_refs(out);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "main");
}

#[test]
fn parse_branch_refs_splits_remote_names_on_the_first_component_only() {
    let candidates = parse_branch_refs("refs/remotes/origin/feat/a/b\n");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "feat/a/b");
    assert_eq!(
        candidates[0].origin,
        BranchOrigin::Remote {
            remote: "origin".to_string()
        }
    );
}

#[test]
fn parse_branch_refs_collapses_a_local_and_remote_duplicate_to_local() {
    // FR-019: reuse and overwrite act on the local branch.
    let candidates = parse_branch_refs(
        "refs/heads/feat/login\nrefs/remotes/origin/feat/login\nrefs/remotes/upstream/feat/login\n",
    );
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "feat/login");
    assert_eq!(candidates[0].origin, BranchOrigin::Local);
}

#[test]
fn parse_branch_refs_keeps_the_same_name_on_distinct_remotes_as_distinct_candidates() {
    let candidates =
        parse_branch_refs("refs/remotes/origin/feat/x\nrefs/remotes/upstream/feat/x\n");
    assert_eq!(candidates.len(), 2);
}

// --- worktree_add_existing_branch (US1) -----------------------------------------------

#[test]
fn add_existing_branch_registers_the_worktree_without_touching_the_branch_set() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x");
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    git.worktree_add_existing_branch(&repo(), "feat/x", &path)
        .unwrap();

    assert_eq!(git.branches(&repo()), vec!["feat/x".to_string()]);
    assert_eq!(git.worktrees(&repo()), vec![(path, "feat/x".to_string())]);
}

#[test]
fn add_existing_branch_rejects_an_unknown_branch() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    assert!(git
        .worktree_add_existing_branch(&repo(), "feat/x", &path)
        .is_err());
}

#[test]
fn add_existing_branch_rejects_a_branch_already_bound_to_another_worktree() {
    let held = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x")
        .with_worktree(repo(), &held, "feat/x");

    let other = PathBuf::from("/repo/.claude/worktrees/feat-x-2");
    assert!(git
        .worktree_add_existing_branch(&repo(), "feat/x", &other)
        .is_err());
}

#[test]
fn add_existing_branch_honors_the_primed_failure_once() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x")
        .failing_next_add();
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    assert!(git
        .worktree_add_existing_branch(&repo(), "feat/x", &path)
        .is_err());
    // The branch survives a failed reuse even at the boundary level.
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert!(git
        .worktree_add_existing_branch(&repo(), "feat/x", &path)
        .is_ok());
}

// --- worktree_add_reset_branch (US3) --------------------------------------------------

#[test]
fn add_reset_branch_creates_the_branch_when_absent() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    git.worktree_add_reset_branch(&repo(), "feat/x", &path)
        .unwrap();

    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert_eq!(git.worktrees(&repo()), vec![(path, "feat/x".to_string())]);
}

#[test]
fn add_reset_branch_keeps_the_branch_present_on_a_primed_failure() {
    // Mirrors git: `-B` resets the branch before the checkout can fail.
    let git = FakeGit::new()
        .with_repo(repo())
        .with_branch(repo(), "feat/x")
        .failing_next_add();
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    assert!(git
        .worktree_add_reset_branch(&repo(), "feat/x", &path)
        .is_err());
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert!(git.worktrees(&repo()).is_empty());
}

// --- worktree_add_tracking_branch (US4) -----------------------------------------------

#[test]
fn add_tracking_branch_creates_a_local_branch_at_the_remote_and_records_the_upstream() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_remote_branch(repo(), "origin", "feat/x");
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    git.worktree_add_tracking_branch(&repo(), "feat/x", "origin", &path)
        .unwrap();

    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
    assert_eq!(
        git.upstream(&repo(), "feat/x").as_deref(),
        Some("origin/feat/x")
    );
    assert_eq!(git.worktrees(&repo()), vec![(path, "feat/x".to_string())]);
    // The remote ref is untouched (FR-017, FR-020).
    assert_eq!(
        git.remote_branches(&repo()),
        vec!["origin/feat/x".to_string()]
    );
}

#[test]
fn add_tracking_branch_requires_the_remote_ref_to_exist() {
    let git = FakeGit::new().with_repo(repo());
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");
    assert!(git
        .worktree_add_tracking_branch(&repo(), "feat/x", "origin", &path)
        .is_err());
}

#[test]
fn add_tracking_branch_leaves_the_local_branch_behind_on_a_primed_failure() {
    let git = FakeGit::new()
        .with_repo(repo())
        .with_remote_branch(repo(), "origin", "feat/x")
        .failing_next_add();
    let path = PathBuf::from("/repo/.claude/worktrees/feat-x");

    assert!(git
        .worktree_add_tracking_branch(&repo(), "feat/x", "origin", &path)
        .is_err());
    // Created before the checkout failed — rollback is what cleans it up.
    assert!(git.branch_exists(&repo(), "feat/x").unwrap());
}

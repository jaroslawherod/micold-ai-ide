//! T028 [US2] — the existing-branch candidate list: what it contains, how it is ordered, how
//! unavailable branches are marked, and how each row reads (feature 016, FR-010–FR-013;
//! contract `branch-picker.md` §2).

use micold_core::git::FakeGit;
use micold_core::worktree::{branch_candidates, BlockReason, BranchOrigin, WorktreeOwner};
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from("/repo")
}

/// A repository with: two free local branches, one local branch held by a worktree, the
/// project's own checkout on `main`, and two remote-only branches (one of them on a second
/// remote).
fn mixed() -> FakeGit {
    FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "main")
        .with_branch("/repo", "feat/login")
        .with_branch("/repo", "feat/held")
        .with_branch("/repo", "chore/deps")
        .with_remote_branch("/repo", "origin", "feat/reporting")
        .with_remote_branch("/repo", "upstream", "feat/vendor")
        .with_worktree("/repo", "/repo", "main")
        .with_worktree("/repo", "/repo/.claude/worktrees/feat-held", "feat/held")
}

/// The two holders BUG-001 found — a worktree outside `.claude/worktrees/`, and an agent-owned one
/// inside it — kept in their own fixture rather than folded into [`mixed`]. Widening the shared
/// fixture would have rewritten every count and ordering expectation built on it, and those
/// expectations are the suite's record of what the listing already does (FR-027).
fn hidden_holders() -> FakeGit {
    FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "fix/paw")
        .with_branch("/repo", "feat/agent-held")
        .with_worktree("/repo", "/repo/.git-paw/worktrees/fix-paw", "fix/paw")
        .with_worktree(
            "/repo",
            "/repo/.claude/worktrees/agent-deadbeefdeadbeef",
            "feat/agent-held",
        )
}

#[test]
fn the_list_contains_local_and_remote_branches_with_their_origin() {
    let candidates = branch_candidates(&mixed(), &repo()).unwrap();
    let names: Vec<&str> = candidates.iter().map(|c| c.name.as_str()).collect();

    assert_eq!(names.len(), 6);
    assert!(names.contains(&"feat/login"));
    assert!(names.contains(&"feat/reporting"));

    let reporting = candidates
        .iter()
        .find(|c| c.name == "feat/reporting")
        .unwrap();
    assert_eq!(
        reporting.origin,
        BranchOrigin::Remote {
            remote: "origin".to_string()
        }
    );
    let vendor = candidates.iter().find(|c| c.name == "feat/vendor").unwrap();
    assert_eq!(
        vendor.origin,
        BranchOrigin::Remote {
            remote: "upstream".to_string()
        }
    );
}

#[test]
fn branches_held_elsewhere_are_marked_unavailable_with_the_right_reason() {
    let candidates = branch_candidates(&mixed(), &repo()).unwrap();

    // FR-021's two cases are distinguished, so the UI can phrase them differently.
    let main = candidates.iter().find(|c| c.name == "main").unwrap();
    assert_eq!(main.blocked_by, Some(BlockReason::CheckedOutInProjectRoot));
    assert!(!main.is_available());

    let held = candidates.iter().find(|c| c.name == "feat/held").unwrap();
    assert_eq!(
        held.blocked_by,
        Some(BlockReason::CheckedOutAt {
            path: PathBuf::from("/repo/.claude/worktrees/feat-held"),
            owner: WorktreeOwner::User,
        })
    );

    // FR-012: they are marked, NOT omitted.
    assert!(candidates.iter().any(|c| c.name == "main"));
    assert!(candidates.iter().any(|c| c.name == "feat/held"));

    let free = candidates.iter().find(|c| c.name == "feat/login").unwrap();
    assert!(free.is_available());
}

/// BUG-001: the two holders the sidebar cannot show are marked with reasons of their own, so the
/// UI never describes them as a worktree the user could go and find (FR-021a, FR-021b).
#[test]
fn holders_the_sidebar_cannot_show_are_marked_with_their_own_reasons() {
    let candidates = branch_candidates(&hidden_holders(), &repo()).unwrap();
    let reason = |name: &str| {
        candidates
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .blocked_by
            .clone()
    };

    // Not one of the app's worktrees, so not described as one.
    assert_eq!(
        reason("fix/paw"),
        Some(BlockReason::CheckedOutOutsideApp {
            path: PathBuf::from("/repo/.git-paw/worktrees/fix-paw"),
        })
    );

    // The app's own, but hidden by default — so it keeps `CheckedOutAt` and carries the owner.
    assert_eq!(
        reason("feat/agent-held"),
        Some(BlockReason::CheckedOutAt {
            path: PathBuf::from("/repo/.claude/worktrees/agent-deadbeefdeadbeef"),
            owner: WorktreeOwner::Agent,
        })
    );

    // FR-012 holds for these too: marked, not omitted.
    assert!(candidates.iter().any(|c| c.name == "fix/paw"));
    assert!(candidates.iter().any(|c| c.name == "feat/agent-held"));
}

#[test]
fn ordering_is_available_first_then_local_then_by_remote_then_by_name() {
    let candidates = branch_candidates(&mixed(), &repo()).unwrap();
    let rendered: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();

    assert_eq!(
        rendered,
        vec![
            // available locals, alphabetical
            "chore/deps",
            "feat/login",
            // available remotes, by remote then name
            "feat/reporting", // origin
            "feat/vendor",    // upstream
            // blocked last
            "feat/held",
            "main",
        ]
    );
}

#[test]
fn row_labels_read_as_the_contract_specifies() {
    let candidates = branch_candidates(&mixed(), &repo()).unwrap();
    let label = |name: &str| {
        candidates
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .to_string()
    };

    assert_eq!(label("feat/login"), "feat/login");
    assert_eq!(label("feat/reporting"), "feat/reporting · origin");
    assert_eq!(label("feat/held"), "feat/held · in use by feat-held");
    assert_eq!(label("main"), "main · in use by the project checkout");
}

/// BUG-001: a row is one line, so it names the KIND of holder. The location that makes an
/// unmanaged holder findable goes in the inline explanation, which has room for it. What a row
/// must never print is a folder name for something the sidebar cannot show
/// (contract `branch-picker.md` §2).
#[test]
fn rows_for_holders_the_sidebar_cannot_show_name_the_kind_not_a_folder() {
    let candidates = branch_candidates(&hidden_holders(), &repo()).unwrap();
    let label = |name: &str| {
        candidates
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .to_string()
    };

    assert_eq!(label("fix/paw"), "fix/paw · in use outside this app");
    assert_eq!(
        label("feat/agent-held"),
        "feat/agent-held · in use by a hidden agent worktree"
    );
    assert!(
        !label("fix/paw").contains("fix-paw"),
        "a row must not print a folder name the sidebar cannot show"
    );
}

#[test]
fn a_repository_with_no_branches_yields_an_empty_list() {
    // FR-013's "there are none" case — the caller says so explicitly rather than showing an
    // empty control.
    let git = FakeGit::new().with_repo("/repo");
    assert!(branch_candidates(&git, &repo()).unwrap().is_empty());
}

#[test]
fn a_repository_whose_every_branch_is_checked_out_yields_no_available_candidate() {
    // FR-013's "none available" case: the rows still exist, so their reasons stay visible.
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "main")
        .with_worktree("/repo", "/repo", "main");

    let candidates = branch_candidates(&git, &repo()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(!candidates.iter().any(|c| c.is_available()));
}

#[test]
fn a_local_branch_hides_the_remote_one_of_the_same_name() {
    // FR-019 at the picker level: one row, and it is the local branch.
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/x")
        .with_remote_branch("/repo", "origin", "feat/x");

    let candidates = branch_candidates(&git, &repo()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].origin, BranchOrigin::Local);
}

#[test]
fn listing_never_mutates_the_repository() {
    let git = mixed();
    let before = (
        git.branches(&repo()),
        git.worktrees(&repo()),
        git.remote_branches(&repo()),
    );
    let _ = branch_candidates(&git, &repo()).unwrap();
    let after = (
        git.branches(&repo()),
        git.worktrees(&repo()),
        git.remote_branches(&repo()),
    );
    assert_eq!(before, after);
}

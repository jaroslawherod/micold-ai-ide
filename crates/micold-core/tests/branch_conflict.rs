//! T004/T021/T047/T053 — branch-conflict pre-flight classification and re-verification
//! (feature 016, contracts/branch-conflict.md §1, §4).
//!
//! `preflight` is the single classifier both the prompt and `create_worktree` use, so these tests
//! pin the five situations, their precedence, and the guarantee that classifying mutates nothing.

use micold_core::git::FakeGit;
use micold_core::worktree::{preflight, BlockReason, BranchSituation, CreateError, CreateMode};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from("/repo")
}

fn target() -> PathBuf {
    PathBuf::from("/repo/.claude/worktrees/feat-login")
}

/// Snapshot of everything `preflight` must never touch.
fn snapshot(git: &FakeGit, repo: &Path) -> (Vec<String>, Vec<(PathBuf, String)>) {
    (git.branches(repo), git.worktrees(repo))
}

// ---------------------------------------------------------------------------------------
// Classification (contracts/branch-conflict.md §1)
// ---------------------------------------------------------------------------------------

#[test]
fn an_unused_name_is_free() {
    let git = FakeGit::new().with_repo("/repo");
    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(s, BranchSituation::Free);
}

#[test]
fn an_existing_local_branch_is_available_not_an_error() {
    // The heart of the feature: what used to be `CreateError::DuplicateBranch` is now a
    // situation the user can act on (FR-001).
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login");
    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::LocalAvailable {
            branch: "feat/login".to_string()
        }
    );
}

#[test]
fn a_branch_only_on_a_remote_is_remote_only() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_remote_branch("/repo", "origin", "feat/login");
    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::RemoteOnly {
            branch: "feat/login".to_string(),
            remotes: vec!["origin".to_string()]
        }
    );
}

#[test]
fn a_branch_checked_out_in_another_worktree_is_blocked_and_names_it() {
    let holder = PathBuf::from("/repo/.claude/worktrees/feat-login-old");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", &holder, "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutAt { path: holder },
        }
    );
}

#[test]
fn a_branch_checked_out_in_the_project_root_is_blocked_as_the_project_checkout() {
    // FR-021's second case. The repository's own checkout appears in `worktree list --porcelain`
    // as the record whose path IS the repo root — which is exactly why `preflight` must read the
    // raw records rather than `reconcile()` (research R1).
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "main")
        .with_worktree("/repo", "/repo", "main");

    let s = preflight(&git, &repo(), &target(), "main", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::Blocked {
            branch: "main".to_string(),
            reason: BlockReason::CheckedOutInProjectRoot,
        }
    );
}

#[test]
fn an_existing_target_directory_is_a_directory_clash() {
    let git = FakeGit::new().with_repo("/repo");
    let s = preflight(&git, &repo(), &target(), "feat/login", true).unwrap();
    assert_eq!(s, BranchSituation::DirectoryTaken { dir: target() });
}

// ---------------------------------------------------------------------------------------
// Precedence: directory > blocked > local > remote > free
// ---------------------------------------------------------------------------------------

#[test]
fn a_directory_clash_outranks_every_branch_situation() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", "/repo", "feat/login");

    // Branch is both existing AND checked out, but the directory is what blocks first: no branch
    // choice could resolve it (FR-022).
    let s = preflight(&git, &repo(), &target(), "feat/login", true).unwrap();
    assert_eq!(s, BranchSituation::DirectoryTaken { dir: target() });
}

#[test]
fn a_checked_out_branch_outranks_a_merely_existing_one() {
    let holder = PathBuf::from("/repo/.claude/worktrees/other");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", &holder, "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert!(matches!(s, BranchSituation::Blocked { .. }));
}

#[test]
fn a_local_branch_outranks_a_remote_one_of_the_same_name() {
    // FR-019: reuse and overwrite act on the local branch.
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_remote_branch("/repo", "origin", "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::LocalAvailable {
            branch: "feat/login".to_string()
        }
    );
}

/// Spec Edge Cases: the app must NOT silently choose one remote — every remote carrying the
/// name is reported, so the user picks explicitly.
#[test]
fn the_same_name_on_several_remotes_reports_all_of_them() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_remote_branch("/repo", "upstream", "feat/login")
        .with_remote_branch("/repo", "origin", "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false).unwrap();
    assert_eq!(
        s,
        BranchSituation::RemoteOnly {
            branch: "feat/login".to_string(),
            remotes: vec!["origin".to_string(), "upstream".to_string()]
        }
    );
}

// ---------------------------------------------------------------------------------------
// No mutation (SC-007)
// ---------------------------------------------------------------------------------------

#[test]
fn preflight_never_mutates_the_repository() {
    let holder = PathBuf::from("/repo/.claude/worktrees/other");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_branch("/repo", "main")
        .with_remote_branch("/repo", "origin", "feat/remote-only")
        .with_worktree("/repo", &holder, "feat/login");

    let before = snapshot(&git, &repo());
    for (branch, dir_taken) in [
        ("feat/login", false),
        ("main", false),
        ("feat/remote-only", false),
        ("feat/nothing", false),
        ("feat/nothing", true),
    ] {
        let _ = preflight(&git, &repo(), &target(), branch, dir_taken).unwrap();
    }
    assert_eq!(snapshot(&git, &repo()), before);
}

// ---------------------------------------------------------------------------------------
// Re-verification compatibility matrix (contracts/branch-conflict.md §4)
// ---------------------------------------------------------------------------------------

/// Every (mode, situation) pair the contract declares compatible.
fn compatible(mode: &CreateMode, situation: &BranchSituation) -> bool {
    match (mode, situation) {
        (CreateMode::NewBranch, BranchSituation::Free) => true,
        // The deliberate "start fresh at HEAD" answer to a remote-only branch (FR-018).
        (CreateMode::NewBranch, BranchSituation::RemoteOnly { .. }) => true,
        (CreateMode::ReuseLocal, BranchSituation::LocalAvailable { .. }) => true,
        (CreateMode::Overwrite, BranchSituation::LocalAvailable { .. }) => true,
        (CreateMode::TrackRemote { remote }, BranchSituation::RemoteOnly { remotes, .. }) => {
            remotes.contains(remote)
        }
        _ => false,
    }
}

/// Build a `FakeGit` that will make `preflight` report `situation` for `feat/login`.
fn fake_for(situation: &BranchSituation) -> (FakeGit, bool) {
    match situation {
        BranchSituation::Free => (FakeGit::new().with_repo("/repo"), false),
        BranchSituation::LocalAvailable { .. } => (
            FakeGit::new()
                .with_repo("/repo")
                .with_branch("/repo", "feat/login"),
            false,
        ),
        BranchSituation::RemoteOnly { remotes, .. } => {
            let mut git = FakeGit::new().with_repo("/repo");
            for remote in remotes {
                git = git.with_remote_branch("/repo", remote, "feat/login");
            }
            (git, false)
        }
        BranchSituation::Blocked { .. } => (
            FakeGit::new()
                .with_repo("/repo")
                .with_branch("/repo", "feat/login")
                .with_worktree("/repo", "/repo", "feat/login"),
            false,
        ),
        BranchSituation::DirectoryTaken { .. } => (FakeGit::new().with_repo("/repo"), true),
    }
}

#[test]
fn every_mode_situation_pair_matches_the_contract_and_incompatible_ones_never_mutate() {
    let modes = [
        CreateMode::NewBranch,
        CreateMode::ReuseLocal,
        CreateMode::Overwrite,
        CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
    ];
    let situations = [
        BranchSituation::Free,
        BranchSituation::LocalAvailable {
            branch: "feat/login".to_string(),
        },
        BranchSituation::RemoteOnly {
            branch: "feat/login".to_string(),
            remotes: vec!["origin".to_string()],
        },
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutInProjectRoot,
        },
        BranchSituation::DirectoryTaken {
            dir: PathBuf::from("/repo/.claude/worktrees/feat-login"),
        },
    ];

    for mode in &modes {
        for situation in &situations {
            let (git, dir_taken) = fake_for(situation);
            let names = micold_core::naming::DerivedNames {
                dir_name: "feat-login".to_string(),
                branch: "feat/login".to_string(),
            };
            let before = snapshot(&git, &repo());

            let result = micold_core::worktree::create_worktree(
                &git,
                &repo(),
                &target(),
                &names,
                dir_taken,
                mode,
                &mut |_| {},
            );

            if compatible(mode, situation) {
                assert!(
                    result.is_ok(),
                    "{mode:?} + {situation:?} should be compatible but failed: {result:?}"
                );
            } else {
                assert!(
                    result.is_err(),
                    "{mode:?} + {situation:?} should be rejected but succeeded"
                );
                // Incompatible pairs abort BEFORE any mutation (FR-009).
                assert_eq!(
                    snapshot(&git, &repo()),
                    before,
                    "{mode:?} + {situation:?} mutated the repository before rejecting"
                );
            }
        }
    }
}

#[test]
fn a_situation_that_changed_since_the_prompt_is_reported_as_such() {
    // The user answered "reuse", but the branch vanished in the meantime.
    let git = FakeGit::new().with_repo("/repo");
    let names = micold_core::naming::DerivedNames {
        dir_name: "feat-login".to_string(),
        branch: "feat/login".to_string(),
    };

    let err = micold_core::worktree::create_worktree(
        &git,
        &repo(),
        &target(),
        &names,
        false,
        &CreateMode::ReuseLocal,
        &mut |_| {},
    )
    .unwrap_err();

    assert_eq!(err, CreateError::SituationChanged);
    assert!(git.worktrees(&repo()).is_empty());
}

#[test]
fn a_blocked_branch_reports_who_holds_it_rather_than_a_raw_git_failure() {
    // US5: the message must carry the holder, not just "it failed".
    let holder = PathBuf::from("/repo/.claude/worktrees/feat-login-old");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", &holder, "feat/login");
    let names = micold_core::naming::DerivedNames {
        dir_name: "feat-login".to_string(),
        branch: "feat/login".to_string(),
    };

    let err = micold_core::worktree::create_worktree(
        &git,
        &repo(),
        &target(),
        &names,
        false,
        &CreateMode::ReuseLocal,
        &mut |_| {},
    )
    .unwrap_err();

    assert_eq!(
        err,
        CreateError::BranchInUse {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutAt { path: holder },
        }
    );
}

/// Every remote that really carries the ref is an acceptable answer; one that doesn't is not.
#[test]
fn tracking_is_accepted_for_any_remote_that_carries_the_branch() {
    let situation = BranchSituation::RemoteOnly {
        branch: "feat/login".to_string(),
        remotes: vec!["origin".to_string(), "upstream".to_string()],
    };
    for remote in ["origin", "upstream"] {
        assert!(CreateMode::TrackRemote {
            remote: remote.to_string()
        }
        .is_compatible_with(&situation));
    }
    assert!(!CreateMode::TrackRemote {
        remote: "fork".to_string()
    }
    .is_compatible_with(&situation));
}

/// Picking `feat/login · upstream` must create a branch tracking UPSTREAM, not whichever remote
/// happens to sort first (spec Edge Cases). This is the regression guard for the silent-choice
/// bug: before `RemoteOnly` carried every remote, this created a branch tracking `origin`.
#[test]
fn tracking_the_second_remote_creates_a_branch_tracking_that_remote() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_remote_branch("/repo", "origin", "feat/login")
        .with_remote_branch("/repo", "upstream", "feat/login");
    let names = micold_core::naming::DerivedNames {
        dir_name: "feat-login".to_string(),
        branch: "feat/login".to_string(),
    };

    micold_core::worktree::create_worktree(
        &git,
        &repo(),
        &target(),
        &names,
        false,
        &CreateMode::TrackRemote {
            remote: "upstream".to_string(),
        },
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(
        git.upstream(&repo(), "feat/login").as_deref(),
        Some("upstream/feat/login")
    );
}

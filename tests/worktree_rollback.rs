//! T027 — rollback plan ordering + clean unwind on failure (FR-006b).
//! Feature 013 US3 extends this with the stage-tagged progress channel: a rolled-back create
//! must report `CreateStage::RollingBack` as its final stage (FR-009).

use micold_ai_ide::git::{FakeGit, Git};
use micold_ai_ide::naming::DerivedNames;
use micold_ai_ide::worktree::{
    create_worktree, rollback_plan, CleanupStep, CreateError, CreateMode, CreateProgressEvent,
    CreateStage,
};
use std::path::PathBuf;

/// Collapse consecutive same-stage events into the ordered sequence of *distinct* stages
/// reached (mirrors the identical helper in `tests/worktree_create.rs`).
fn stage_sequence(events: &[CreateProgressEvent]) -> Vec<CreateStage> {
    let mut stages: Vec<CreateStage> = Vec::new();
    for e in events {
        if stages.last() != Some(&e.stage) {
            stages.push(e.stage);
        }
    }
    stages
}

#[test]
fn rollback_plan_order_removes_registration_before_branch() {
    let plan = rollback_plan(&CreateMode::NewBranch);
    assert_eq!(
        plan,
        vec![
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

    let mut events: Vec<CreateProgressEvent> = Vec::new();
    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::NewBranch,
        &mut |e| events.push(e),
    )
    .unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    // The orphan branch git created before the simulated failure must be cleaned up.
    assert!(!git.branch_exists(&repo, "feat/x").unwrap());
    assert!(git.worktrees(&repo).is_empty());
    // The failure and the rollback both surface to the caller (feature 010 follow-up), now
    // stage-tagged (feature 013, US3).
    assert!(events
        .iter()
        .any(|e| e.line.contains("worktree add failed")));
    assert!(events.iter().any(|e| e.line.contains("Rolling back")));
    // The failed stage is identifiable (FR-009): the sequence ends at RollingBack, never
    // silently reverting or continuing past it.
    assert_eq!(
        stage_sequence(&events),
        vec![
            CreateStage::PreflightCheck,
            CreateStage::CreatingWorktree,
            CreateStage::RollingBack,
        ]
    );
}

#[test]
fn failed_submodule_fetch_rolls_back_the_whole_worktree() {
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_submodules(&target)
        .failing_next_submodule_update();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };

    let mut events: Vec<CreateProgressEvent> = Vec::new();
    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::NewBranch,
        &mut |e| events.push(e),
    )
    .unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    // Same full rollback as a worktree-add failure (spec FR-005): no worktree, no branch.
    assert!(!git.branch_exists(&repo, "feat/x").unwrap());
    assert!(git.worktrees(&repo).is_empty());
    // The failed stage is identifiable (FR-009): reached SettingUpSubmodules before failing,
    // ends at RollingBack.
    assert_eq!(
        stage_sequence(&events),
        vec![
            CreateStage::PreflightCheck,
            CreateStage::CreatingWorktree,
            CreateStage::SettingUpSubmodules,
            CreateStage::RollingBack,
        ]
    );
}

/// FR-006 regression test (US3): nothing between the failing `submodule_update_init_recursive`
/// call and the `CreateError` handed to the caller re-classifies or drops the underlying git
/// error text — it survives verbatim, the same way a `worktree_add` failure's text already did.
/// This already passes once T005 lands (no new production code for US3 — see tasks.md).
#[test]
fn submodule_failure_message_is_preserved_verbatim_for_the_user() {
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_submodules(&target)
        .failing_next_submodule_update();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };

    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap_err();
    let CreateError::RolledBack(message) = err else {
        panic!("expected RolledBack, got {err:?}");
    };
    assert!(
        message.contains("simulated submodule update failure"),
        "expected the underlying git error text to survive verbatim, got: {message}"
    );
}

// =======================================================================================
// Feature 016 — mode-aware rollback (contracts/branch-conflict.md §5, FR-008).
//
// The single most important guard in feature 016: today's plan deletes the branch
// unconditionally, which under reuse would destroy the user's pre-existing commits as a
// consequence of a failure they did not cause.
// =======================================================================================

#[test]
fn only_reuse_declines_to_claim_it_created_the_branch() {
    assert!(CreateMode::NewBranch.creates_branch());
    assert!(!CreateMode::ReuseLocal.creates_branch());
    assert!(CreateMode::Overwrite.creates_branch());
    assert!(CreateMode::TrackRemote {
        remote: "origin".to_string()
    }
    .creates_branch());
}

#[test]
fn rollback_plan_omits_branch_deletion_only_for_reuse() {
    let full = vec![
        CleanupStep::WorktreeRemove,
        CleanupStep::WorktreePrune,
        CleanupStep::BranchDelete,
        CleanupStep::RemoveDir,
    ];
    let without_delete = vec![
        CleanupStep::WorktreeRemove,
        CleanupStep::WorktreePrune,
        CleanupStep::RemoveDir,
    ];

    assert_eq!(rollback_plan(&CreateMode::NewBranch), full);
    assert_eq!(rollback_plan(&CreateMode::Overwrite), full);
    assert_eq!(
        rollback_plan(&CreateMode::TrackRemote {
            remote: "origin".to_string()
        }),
        full
    );
    assert_eq!(rollback_plan(&CreateMode::ReuseLocal), without_delete);

    // Ordering is preserved in every mode: registration removal precedes branch deletion, and
    // the directory removal is always last.
    for mode in [
        CreateMode::NewBranch,
        CreateMode::ReuseLocal,
        CreateMode::Overwrite,
        CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
    ] {
        let plan = rollback_plan(&mode);
        assert_eq!(plan.first(), Some(&CleanupStep::WorktreeRemove));
        assert_eq!(plan.last(), Some(&CleanupStep::RemoveDir));
    }
}

/// T019 — SC-003's regression guard. A failed *reuse* must leave the pre-existing branch, and
/// its commits, exactly where they were.
#[test]
fn a_failed_reuse_preserves_the_pre_existing_branch() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/x")
        .failing_next_add();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");

    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::ReuseLocal,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    // THE assertion this whole feature hangs on (FR-008).
    assert!(
        git.branch_exists(&repo, "feat/x").unwrap(),
        "reuse rollback deleted a branch it did not create"
    );
    assert!(git.worktrees(&repo).is_empty());
}

#[test]
fn a_failed_overwrite_deletes_the_branch_it_created() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/x")
        .failing_next_add();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");

    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::Overwrite,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    // The branch present at failure is the *new* one `-B` reset into place; the old tip is
    // already gone, so removing it leaves no junk behind.
    assert!(!git.branch_exists(&repo, "feat/x").unwrap());
    assert!(git.worktrees(&repo).is_empty());
}

#[test]
fn a_failed_remote_track_deletes_the_local_branch_but_not_the_remote_one() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_remote_branch("/repo", "origin", "feat/x")
        .failing_next_add();
    let repo = PathBuf::from("/repo");
    let names = DerivedNames {
        dir_name: "feat-x".to_string(),
        branch: "feat/x".to_string(),
    };
    let target = PathBuf::from("/repo/.claude/worktrees/feat-x");

    let err = create_worktree(
        &git,
        &repo,
        &target,
        &names,
        false,
        &CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, CreateError::RolledBack(_)));

    assert!(!git.branch_exists(&repo, "feat/x").unwrap());
    assert!(
        git.remote_branches(&repo)
            .contains(&"origin/feat/x".to_string()),
        "rollback must never touch a remote ref"
    );
}

//! T026 — create orchestration happy path + duplicate detection (FR-006, FR-009).
//! Feature 013 US3 extends this with the stage-tagged progress channel (`CreateStage`/
//! `CreateProgressEvent`).

use micold_core::git::{FakeGit, Git};
use micold_core::naming::DerivedNames;
use micold_core::worktree::{
    create_worktree, CreateError, CreateMode, CreateProgressEvent, CreateStage, WorktreeStatus,
};
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

/// Collapse consecutive same-stage events into the ordered sequence of *distinct* stages
/// reached, so assertions don't depend on how many raw lines a stage happens to emit.
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
fn happy_path_creates_branch_and_worktree() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(wt.status, WorktreeStatus::Valid);
    assert_eq!(wt.dir_name, "feat-abc-123-login");
    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    assert!(git.branch_exists(&repo, "feat/abc-123-login").unwrap());
    assert_eq!(git.worktrees(&repo).len(), 1);
}

/// Feature 016 replaced `CreateError::DuplicateBranch` with a decision (FR-001). A `NewBranch`
/// create against a name that is already taken is now a *stale answer*, not a duplicate error —
/// the caller is expected to have resolved the situation first.
#[test]
fn a_new_branch_create_against_a_taken_name_is_rejected_without_mutation() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/abc-123-login");
    let repo = PathBuf::from("/repo");

    let err = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap_err();
    assert_eq!(err, CreateError::SituationChanged);
    assert!(git.worktrees(&repo).is_empty());
}

#[test]
fn duplicate_target_dir_is_rejected() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    // target_exists = true simulates the fs check.
    let err = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        true,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap_err();
    assert_eq!(err, CreateError::DuplicateDir);
}

#[test]
fn submodules_are_fetched_when_present() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_submodules(target())
        .with_submodule_progress_lines(vec!["Cloning into 'vendor/sub'...".to_string()]);
    let repo = PathBuf::from("/repo");
    let mut events: Vec<CreateProgressEvent> = Vec::new();

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |e| events.push(e),
    )
    .unwrap();

    assert_eq!(wt.status, WorktreeStatus::Valid);
    assert_eq!(git.submodule_update_calls(), vec![target()]);
    // The executed commands and the submodule fetch's live output both reach the caller
    // (feature 010 follow-up — progress visibility), now stage-tagged (feature 013, US3).
    assert!(events
        .iter()
        .any(|e| e.line.contains("git worktree add") && e.line.contains("feat/abc-123-login")));
    assert!(events
        .iter()
        .any(|e| e.line.contains("git submodule update --init --recursive")));
    assert!(events
        .iter()
        .any(|e| e.line.contains("Cloning into 'vendor/sub'...")));

    // The submodule stage is only ever reached after preflight + branch/worktree creation
    // (FR-007), and is present at all only because this repo actually has submodules (FR-008).
    assert_eq!(
        stage_sequence(&events),
        vec![
            CreateStage::PreflightCheck,
            CreateStage::CreatingWorktree,
            CreateStage::SettingUpSubmodules,
        ]
    );
}

#[test]
fn submodule_fetch_is_skipped_when_absent() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");

    create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap();

    assert!(git.submodule_update_calls().is_empty());
}

/// FR-008: a repo with no submodules never reaches (and never displays) the submodule stage.
#[test]
fn plain_repo_stage_sequence_has_no_submodule_stage() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    let mut events: Vec<CreateProgressEvent> = Vec::new();

    create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |e| events.push(e),
    )
    .unwrap();

    assert_eq!(
        stage_sequence(&events),
        vec![CreateStage::PreflightCheck, CreateStage::CreatingWorktree]
    );
}

/// FR-007: every stage has a distinct, non-empty plain-language label for the progress display.
#[test]
fn every_create_stage_has_a_distinct_plain_language_label() {
    let stages = [
        CreateStage::PreflightCheck,
        CreateStage::CreatingWorktree,
        CreateStage::SettingUpSubmodules,
        CreateStage::RollingBack,
    ];
    let labels: Vec<&str> = stages
        .iter()
        .map(|s| s.label(&CreateMode::NewBranch))
        .collect();
    assert!(labels.iter().all(|l| !l.is_empty()));
    let unique: std::collections::BTreeSet<&str> = labels.iter().copied().collect();
    assert_eq!(
        unique.len(),
        labels.len(),
        "labels must be distinct: {labels:?}"
    );
}

#[test]
fn duplicate_registered_worktree_is_rejected() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    // Pre-register the same path via a first successful create.
    create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap();
    // A second create with a different branch but the same path → DuplicateDir.
    let other = DerivedNames {
        dir_name: "feat-abc-123-login".to_string(),
        branch: "feat/other".to_string(),
    };
    let err = create_worktree(
        &git,
        &repo,
        &target(),
        &other,
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap_err();
    assert_eq!(err, CreateError::DuplicateDir);
}

// =======================================================================================
// Feature 016 — creation on an existing branch (US1), overwrite (US3), remote track (US4).
// =======================================================================================

/// T018/US1 — reuse binds the existing branch and leaves its tip alone.
#[test]
fn reuse_checks_out_the_existing_branch_without_recreating_it() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/abc-123-login");
    let repo = PathBuf::from("/repo");

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::ReuseLocal,
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(wt.status, WorktreeStatus::Valid);
    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    // Exactly one branch, still the original — reuse never creates a second one.
    assert_eq!(git.branches(&repo), vec!["feat/abc-123-login".to_string()]);
    assert_eq!(git.worktrees(&repo).len(), 1);
    // And it went through the reuse command, not `-b`.
    assert_eq!(git.add_existing_calls(&repo), vec!["feat/abc-123-login"]);
}

/// FR-024 — the reuse path reports what it is doing, not "creating branch".
#[test]
fn reuse_progress_names_the_checkout_step() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/abc-123-login");
    let repo = PathBuf::from("/repo");
    let mut events: Vec<CreateProgressEvent> = Vec::new();

    create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::ReuseLocal,
        &mut |e| events.push(e),
    )
    .unwrap();

    assert_eq!(
        stage_sequence(&events),
        vec![CreateStage::PreflightCheck, CreateStage::CreatingWorktree]
    );
    assert!(
        events
            .iter()
            .any(|e| e.line.contains("git worktree add") && !e.line.contains("-b")),
        "reuse must not report a `-b` branch-creating command: {events:?}"
    );
    assert_ne!(
        CreateStage::CreatingWorktree.label(&CreateMode::ReuseLocal),
        CreateStage::CreatingWorktree.label(&CreateMode::NewBranch)
    );
}

/// T039/US3 — overwrite recreates the branch at HEAD.
#[test]
fn overwrite_replaces_the_branch_and_creates_the_worktree() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/abc-123-login");
    let repo = PathBuf::from("/repo");

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::Overwrite,
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    assert!(git.branch_exists(&repo, "feat/abc-123-login").unwrap());
    assert_eq!(git.worktrees(&repo).len(), 1);
    assert_eq!(git.add_reset_calls(&repo), vec!["feat/abc-123-login"]);
}

/// T046/US4 — continuing from a remote branch creates a local tracking branch.
#[test]
fn tracking_a_remote_branch_creates_a_local_branch_that_tracks_it() {
    let git = FakeGit::new().with_repo("/repo").with_remote_branch(
        "/repo",
        "origin",
        "feat/abc-123-login",
    );
    let repo = PathBuf::from("/repo");

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    assert!(git.branch_exists(&repo, "feat/abc-123-login").unwrap());
    assert_eq!(
        git.upstream(&repo, "feat/abc-123-login").as_deref(),
        Some("origin/feat/abc-123-login")
    );
    // FR-020: the remote ref is read, never written.
    assert_eq!(
        git.remote_branches(&repo),
        vec!["origin/feat/abc-123-login".to_string()]
    );
}

/// FR-018 — answering "start fresh at HEAD" to a remote-only branch creates an ordinary new
/// local branch, and pointedly does NOT track the remote one.
#[test]
fn starting_fresh_over_a_remote_only_name_creates_an_untracked_branch() {
    let git = FakeGit::new().with_repo("/repo").with_remote_branch(
        "/repo",
        "origin",
        "feat/abc-123-login",
    );
    let repo = PathBuf::from("/repo");

    create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |_| {},
    )
    .unwrap();

    assert!(git.branch_exists(&repo, "feat/abc-123-login").unwrap());
    assert_eq!(git.upstream(&repo, "feat/abc-123-login"), None);
}

/// SC-008 — the conflict-free path is untouched by this feature: same stages, same commands.
#[test]
fn a_free_name_still_creates_exactly_as_before() {
    let git = FakeGit::new().with_repo("/repo");
    let repo = PathBuf::from("/repo");
    let mut events: Vec<CreateProgressEvent> = Vec::new();

    let wt = create_worktree(
        &git,
        &repo,
        &target(),
        &names(),
        false,
        &CreateMode::NewBranch,
        &mut |e| events.push(e),
    )
    .unwrap();

    assert_eq!(wt.dir_name, "feat-abc-123-login");
    assert_eq!(wt.branch.as_deref(), Some("feat/abc-123-login"));
    assert_eq!(
        stage_sequence(&events),
        vec![CreateStage::PreflightCheck, CreateStage::CreatingWorktree]
    );
    assert!(events
        .iter()
        .any(|e| e.line.contains("git worktree add -b feat/abc-123-login")));
    // No reuse/overwrite/track command was involved.
    assert!(git.add_existing_calls(&repo).is_empty());
    assert!(git.add_reset_calls(&repo).is_empty());
}

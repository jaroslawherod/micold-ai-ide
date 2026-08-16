//! T004/T021/T047/T053 — branch-conflict pre-flight classification and re-verification
//! (feature 016, contracts/branch-conflict.md §1, §4).
//!
//! `preflight` is the single classifier both the prompt and `create_worktree` use, so these tests
//! pin the five situations, their precedence, and the guarantee that classifying mutates nothing.

use micold_core::git::FakeGit;
use micold_core::worktree::{
    preflight, BlockReason, BranchSituation, CreateError, CreateMode, WorktreeOwner,
};
use std::path::{Path, PathBuf};

/// The app-managed holder shape, which is the common case in these tests.
fn held_at(path: impl Into<PathBuf>) -> BlockReason {
    BlockReason::CheckedOutAt {
        path: path.into(),
        owner: WorktreeOwner::User,
    }
}

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
    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
    assert_eq!(s, BranchSituation::Free);
}

#[test]
fn an_existing_local_branch_is_available_not_an_error() {
    // The heart of the feature: what used to be `CreateError::DuplicateBranch` is now a
    // situation the user can act on (FR-001).
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login");
    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
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
    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
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

    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
    assert_eq!(
        s,
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: held_at(holder),
        }
    );
}

/// BUG-001 / FR-021a. `worktree list --porcelain` reports every worktree git knows about, not only
/// the ones under `.claude/worktrees/` — another tool's worktree tree, a folder anywhere else on
/// disk. Blocking on those is right (git refuses the second checkout regardless), but they are not
/// the app's, `reconcile()` drops them, and the sidebar never shows them. Describing one as
/// `CheckedOutAt` is what sent the user hunting for a worktree that is not in the list.
#[test]
fn a_branch_held_by_a_worktree_the_app_does_not_manage_is_blocked_as_outside_the_app() {
    for holder in [
        // Another tool's worktree directory, inside the repository.
        "/repo/.git-paw/worktrees/feat-login",
        // A plain sibling directory, outside the repository altogether.
        "/elsewhere/feat-login",
        // Nested one level too deep to be a `.claude/worktrees/` child — `reconcile()` tests the
        // immediate parent, and so must this.
        "/repo/.claude/worktrees/nested/feat-login",
    ] {
        let holder = PathBuf::from(holder);
        let git = FakeGit::new()
            .with_repo("/repo")
            .with_branch("/repo", "feat/login")
            .with_worktree("/repo", &holder, "feat/login");

        let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
        assert_eq!(
            s,
            BranchSituation::Blocked {
                branch: "feat/login".to_string(),
                reason: BlockReason::CheckedOutOutsideApp {
                    path: holder.clone()
                },
            },
            "{} is not one of the app's worktrees",
            holder.display()
        );
    }
}

/// BUG-001 / FR-021b. An agent-owned holder IS one of the app's worktrees, so it stays
/// `CheckedOutAt` — but the sidebar hides it by default, so the reason carries the owner and the
/// message can say how to reveal it.
#[test]
fn a_branch_held_by_an_agent_worktree_is_blocked_and_says_who_owns_it() {
    let holder = PathBuf::from("/repo/.claude/worktrees/agent-deadbeefdeadbeef");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", &holder, "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
    assert_eq!(
        s,
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutAt {
                path: holder,
                owner: WorktreeOwner::Agent,
            },
        }
    );
}

/// The branch name is the other half of feature 014's rule (FR-005/FR-007): a worktree whose
/// directory was renamed is still agent-owned if its branch says so.
#[test]
fn an_agent_holder_is_recognised_by_its_branch_when_its_directory_was_renamed() {
    let holder = PathBuf::from("/repo/.claude/worktrees/renamed-by-hand");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "worktree-agent-deadbeefdeadbeef")
        .with_worktree("/repo", &holder, "worktree-agent-deadbeefdeadbeef");

    let s = preflight(
        &git,
        &repo(),
        &target(),
        "worktree-agent-deadbeefdeadbeef",
        false,
        &[],
    )
    .unwrap();
    assert_eq!(
        s,
        BranchSituation::Blocked {
            branch: "worktree-agent-deadbeefdeadbeef".to_string(),
            reason: BlockReason::CheckedOutAt {
                path: holder,
                owner: WorktreeOwner::Agent,
            },
        }
    );
}

/// The classification split must be `reconcile()`'s, so "described as one of your worktrees" and
/// "appears in your worktree list" are the same set. Asserted against `discover()` itself rather
/// than against a restatement of the rule, which could drift (BUG-001, contract §1 rule 2).
#[test]
fn only_holders_the_sidebar_would_list_are_described_as_the_apps_own() {
    for holder in [
        "/repo/.claude/worktrees/feat-login-old",
        "/repo/.git-paw/worktrees/feat-login",
        "/elsewhere/feat-login",
    ] {
        let holder = PathBuf::from(holder);
        let git = FakeGit::new()
            .with_repo("/repo")
            .with_branch("/repo", "feat/login")
            .with_worktree("/repo", &holder, "feat/login");

        let listed = micold_core::worktree::discover(&git, &repo(), &[])
            .iter()
            .any(|w| w.path == holder);
        let described_as_ours = matches!(
            preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap(),
            BranchSituation::Blocked {
                reason: BlockReason::CheckedOutAt { .. },
                ..
            }
        );
        assert_eq!(
            described_as_ours,
            listed,
            "{} is described as the app's own but {} in the worktree list",
            holder.display(),
            if listed { "is" } else { "is not" }
        );
    }
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

    let s = preflight(&git, &repo(), &target(), "main", false, &[]).unwrap();
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
    let s = preflight(&git, &repo(), &target(), "feat/login", true, &[]).unwrap();
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
    let s = preflight(&git, &repo(), &target(), "feat/login", true, &[]).unwrap();
    assert_eq!(s, BranchSituation::DirectoryTaken { dir: target() });
}

#[test]
fn a_checked_out_branch_outranks_a_merely_existing_one() {
    let holder = PathBuf::from("/repo/.claude/worktrees/other");
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", &holder, "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
    assert!(matches!(s, BranchSituation::Blocked { .. }));
}

#[test]
fn a_local_branch_outranks_a_remote_one_of_the_same_name() {
    // FR-019: reuse and overwrite act on the local branch.
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_remote_branch("/repo", "origin", "feat/login");

    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
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

    let s = preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap();
    assert_eq!(
        s,
        BranchSituation::RemoteOnly {
            branch: "feat/login".to_string(),
            remotes: vec!["origin".to_string(), "upstream".to_string()]
        }
    );
}

// ---------------------------------------------------------------------------------------
// How each holder is explained (SC-006, FR-021/FR-021a/FR-021b)
// ---------------------------------------------------------------------------------------

/// The app's own worktree is named by its folder, because that IS the sidebar row to go to.
#[test]
fn a_listed_holder_is_explained_by_its_folder_name() {
    assert_eq!(
        held_at("/repo/.claude/worktrees/feat-login-old").explain("feat/login"),
        "'feat/login' is already checked out in the worktree 'feat-login-old'."
    );
}

#[test]
fn the_project_checkout_is_explained_without_a_path() {
    let sentence = BlockReason::CheckedOutInProjectRoot.explain("main");
    assert_eq!(
        sentence,
        "'main' is currently checked out in the project itself."
    );
    assert!(!sentence.contains('/'), "no path belongs here: {sentence}");
}

/// BUG-001 / SC-006. The regression guard: an unmanaged holder must never be described by its
/// folder name alone, because that reads exactly like a sidebar row and there is no such row.
#[test]
fn an_unmanaged_holder_is_explained_by_its_full_path_and_said_to_be_outside_the_app() {
    let sentence = BlockReason::CheckedOutOutsideApp {
        path: PathBuf::from("/repo/.git-paw/worktrees/fix-olx"),
    }
    .explain("fix/olx");

    assert!(
        sentence.contains("/repo/.git-paw/worktrees/fix-olx"),
        "the full path is the only thing that leads the user to it: {sentence}"
    );
    assert!(
        sentence.contains("outside this app"),
        "must say the holder is not one of ours: {sentence}"
    );
}

/// BUG-001 / FR-021b. A hidden holder IS one of the app's, so it keeps its folder name — but the
/// sentence has to account for the row not being on screen.
#[test]
fn a_hidden_agent_holder_is_named_and_says_how_to_reveal_it() {
    let sentence = BlockReason::CheckedOutAt {
        path: PathBuf::from("/repo/.claude/worktrees/agent-deadbeefdeadbeef"),
        owner: WorktreeOwner::Agent,
    }
    .explain("feat/login");

    assert!(
        sentence.contains("agent-deadbeefdeadbeef"),
        "the holder is the app's, so name it: {sentence}"
    );
    assert!(
        sentence.contains("Show agent worktrees"),
        "must say how to bring it into view: {sentence}"
    );
}

/// SC-006 as a property over every holder: whatever the shape, the explanation names the branch
/// and gives the user something to go on beyond "it failed".
#[test]
fn every_holder_explanation_names_the_branch_and_locates_the_holder() {
    let reasons = [
        BlockReason::CheckedOutInProjectRoot,
        held_at("/repo/.claude/worktrees/feat-login-old"),
        BlockReason::CheckedOutAt {
            path: PathBuf::from("/repo/.claude/worktrees/agent-deadbeefdeadbeef"),
            owner: WorktreeOwner::Agent,
        },
        BlockReason::CheckedOutOutsideApp {
            path: PathBuf::from("/elsewhere/feat-login"),
        },
    ];
    for reason in reasons {
        let sentence = reason.explain("feat/login");
        assert!(
            sentence.contains("feat/login"),
            "{reason:?} does not name the branch: {sentence}"
        );
        assert!(
            sentence.ends_with('.'),
            "{reason:?} is not a sentence: {sentence}"
        );
    }
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
        let _ = preflight(&git, &repo(), &target(), branch, dir_taken, &[]).unwrap();
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
                &[],
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
        &[],
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
        &[],
        &mut |_| {},
    )
    .unwrap_err();

    assert_eq!(
        err,
        CreateError::BranchInUse {
            branch: "feat/login".to_string(),
            reason: held_at(holder),
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
        &[],
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(
        git.upstream(&repo(), "feat/login").as_deref(),
        Some("upstream/feat/login")
    );
}

// ---------------------------------------------------------------------------------------------
// 016 BUG-002 (T076): including a holder changes how it is described, because it changes what the
// list shows — and both answers come from one predicate, never from two kept in step (FR-032).
// ---------------------------------------------------------------------------------------------

/// The holder's own worktree, outside anything this app created.
fn outsider() -> PathBuf {
    PathBuf::from("/elsewhere/worktrees/feat-login")
}

fn held_by_the_outsider() -> FakeGit {
    FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "feat/login")
        .with_worktree("/repo", outsider(), "feat/login")
}

/// Before inclusion: the holder is somewhere the user has to be given directions to (BUG-001).
#[test]
fn an_unincluded_holder_is_still_described_as_outside_the_app() {
    let git = held_by_the_outsider();

    assert_eq!(
        preflight(&git, &repo(), &target(), "feat/login", false, &[]).unwrap(),
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutOutsideApp { path: outsider() },
        },
    );
}

/// After it: the same holder, the same block, described as one of the user's own — because it now
/// is one. Nothing about the branch changed; git still refuses the second checkout.
#[test]
fn an_included_holder_is_described_as_one_of_the_apps_own() {
    let git = held_by_the_outsider();
    let included = [outsider()];

    assert_eq!(
        preflight(&git, &repo(), &target(), "feat/login", false, &included).unwrap(),
        BranchSituation::Blocked {
            branch: "feat/login".to_string(),
            reason: BlockReason::CheckedOutAt {
                path: outsider(),
                owner: WorktreeOwner::User,
            },
        },
        "including a worktree puts it in the list, so the explanation must stop sending the user \
         outside the app to find it (FR-032). Reuse and overwrite stay unavailable either way",
    );
}

/// The same guarantee BUG-001 established, now asked with inclusion in play: "described as ours"
/// and "shown in the list" must still be the same set, whichever way inclusion goes. Asked of
/// `discover()` rather than of a restatement of the rule, so a second rule cannot appear and drift.
#[test]
fn description_and_listing_agree_with_or_without_inclusion() {
    for included in [Vec::new(), vec![outsider()]] {
        let git = held_by_the_outsider();

        let listed = micold_core::worktree::discover(&git, &repo(), &included)
            .iter()
            .any(|w| w.path == outsider());
        let described_as_ours = matches!(
            preflight(&git, &repo(), &target(), "feat/login", false, &included).unwrap(),
            BranchSituation::Blocked {
                reason: BlockReason::CheckedOutAt { .. },
                ..
            }
        );

        assert_eq!(
            described_as_ours, listed,
            "with included = {included:?}, the holder is described as the app's own ({described_as_ours}) \
             but listed = {listed}. These are one predicate or they are a defect (BUG-001, FR-032)",
        );
    }
}

/// Inclusion changes the wording and nothing else: the branch is still refused, and still offers
/// neither reuse nor overwrite.
#[test]
fn including_a_holder_does_not_free_its_branch() {
    let git = held_by_the_outsider();

    let situation =
        preflight(&git, &repo(), &target(), "feat/login", false, &[outsider()]).unwrap();

    assert!(
        matches!(situation, BranchSituation::Blocked { .. }),
        "git refuses a second checkout wherever the branch is held, and inclusion is not a git \
         operation — it changed where the user can *find* the holder, not who holds it. Got \
         {situation:?}"
    );
}

/// The project's own checkout is not a worktree to include, and saying it is must not change how
/// it is described.
#[test]
fn the_project_checkout_is_unaffected_by_inclusion() {
    let git = FakeGit::new()
        .with_repo("/repo")
        .with_branch("/repo", "main")
        .with_worktree("/repo", "/repo", "main");

    assert_eq!(
        preflight(&git, &repo(), &target(), "main", false, &[repo()]).unwrap(),
        BranchSituation::Blocked {
            branch: "main".to_string(),
            reason: BlockReason::CheckedOutInProjectRoot,
        },
        "the project checkout is the project. It is named as such whatever the included set says",
    );
}

//! What an entry's changes are compared with (FR-003, FR-004, research R7): the base of a
//! worktree, the uncommitted-only scope of the Default entry, the two toggles and the git range
//! they select.

use serde::{Deserialize, Serialize};

/// What an entry's Changes view can compare. The Default entry (the project root) is
/// uncommitted-only by type: it has no committed range to represent (FR-003, US1 s9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewScope {
    /// A worktree, compared with the point where its branch left the default branch.
    Worktree {
        /// That point, or why there is none.
        base: Base,
    },
    /// The project root: uncommitted changes only.
    RootUncommitted,
}

/// A worktree's base, resolved on every read and never persisted (R7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Base {
    /// The merge-base of `HEAD` and the default branch.
    MergeBase {
        /// The default branch as git names it (`origin/main`, `main` or `master`).
        branch: String,
        /// The merge-base commit (full id).
        commit: String,
    },
    /// No base: committed changes cannot be listed, for this reason.
    Unavailable(BaseUnavailable),
}

/// Why a worktree has no base (Edge Case "No base found").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseUnavailable {
    /// Neither `origin/HEAD`, `main` nor `master` exists.
    NoDefaultBranch,
    /// The branch shares no history with the default branch.
    NoCommonHistory,
}

/// Which kinds of change the view lists. Both on each time the view opens (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toggles {
    /// Changes committed on the branch since the base.
    pub committed: bool,
    /// Staged, unstaged and untracked-not-ignored changes (FR-005).
    pub uncommitted: bool,
}

impl Default for Toggles {
    fn default() -> Self {
        Self {
            committed: false,
            uncommitted: false,
        }
    }
}

/// The git range a list or diff is read over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffRange {
    /// `<base>` → `HEAD`: committed changes only.
    BaseToHead,
    /// `HEAD` → the working tree: uncommitted changes only.
    HeadToWorktree,
    /// `<base>` → the working tree: both, a file changed in both listed once (FR-004).
    BaseToWorktree,
}

impl DiffRange {
    /// The range the view reads for `scope` under `toggles`; `None` when nothing that can be
    /// listed is switched on. The Default entry reads only `toggles.uncommitted`, and a worktree
    /// without a base lists its uncommitted changes only, since committed ones have nothing to be
    /// compared with.
    pub fn for_view(_scope: &ReviewScope, _toggles: Toggles) -> Option<DiffRange> {
        None
    }
}

/// The default branch from what git reports (R7): `origin/HEAD`'s target (the output of
/// `git symbolic-ref refs/remotes/origin/HEAD`, e.g. `refs/remotes/origin/main`), else local
/// `main`, else local `master`, else `None`.
pub fn default_branch_from(
    _origin_head: Option<&str>,
    _has_main: bool,
    _has_master: bool,
) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worktree() -> ReviewScope {
        ReviewScope::Worktree {
            base: Base::MergeBase {
                branch: "main".into(),
                commit: "1a2b3c4d".into(),
            },
        }
    }

    fn toggles(committed: bool, uncommitted: bool) -> Toggles {
        Toggles {
            committed,
            uncommitted,
        }
    }

    #[test]
    fn the_default_branch_is_origin_head_then_main_then_master() {
        assert_eq!(
            default_branch_from(Some("refs/remotes/origin/trunk\n"), true, true).as_deref(),
            Some("origin/trunk"),
            "origin/HEAD's target wins over local branches"
        );
        assert_eq!(
            default_branch_from(None, true, true).as_deref(),
            Some("main"),
            "without origin/HEAD, local main"
        );
        assert_eq!(
            default_branch_from(Some(""), false, true).as_deref(),
            Some("master"),
            "an empty origin/HEAD answer counts as none; then master"
        );
        assert_eq!(default_branch_from(None, false, false), None);
    }

    #[test]
    fn the_toggles_are_both_on_by_default() {
        assert_eq!(Toggles::default(), toggles(true, true));
    }

    #[test]
    fn a_worktree_with_a_base_reads_the_range_its_toggles_select() {
        let scope = worktree();
        assert_eq!(
            DiffRange::for_view(&scope, toggles(true, false)),
            Some(DiffRange::BaseToHead),
            "committed only"
        );
        assert_eq!(
            DiffRange::for_view(&scope, toggles(false, true)),
            Some(DiffRange::HeadToWorktree),
            "uncommitted only"
        );
        assert_eq!(
            DiffRange::for_view(&scope, toggles(true, true)),
            Some(DiffRange::BaseToWorktree),
            "both: one diff from the base to the working tree"
        );
        assert_eq!(DiffRange::for_view(&scope, toggles(false, false)), None);
    }

    #[test]
    fn a_worktree_without_a_base_lists_only_uncommitted_changes() {
        for reason in [BaseUnavailable::NoDefaultBranch, BaseUnavailable::NoCommonHistory] {
            let scope = ReviewScope::Worktree {
                base: Base::Unavailable(reason),
            };
            assert_eq!(
                DiffRange::for_view(&scope, toggles(true, true)),
                Some(DiffRange::HeadToWorktree),
                "committed changes have nothing to be compared with ({reason:?})"
            );
            assert_eq!(
                DiffRange::for_view(&scope, toggles(false, true)),
                Some(DiffRange::HeadToWorktree)
            );
            assert_eq!(
                DiffRange::for_view(&scope, toggles(true, false)),
                None,
                "committed only, without a base, lists nothing (the view shows the reason)"
            );
        }
    }

    #[test]
    fn the_default_entry_never_reads_a_committed_range() {
        let root = ReviewScope::RootUncommitted;
        for (committed, uncommitted) in [(true, true), (false, true), (true, false), (false, false)]
        {
            let range = DiffRange::for_view(&root, toggles(committed, uncommitted));
            assert_ne!(range, Some(DiffRange::BaseToHead));
            assert_ne!(range, Some(DiffRange::BaseToWorktree));
            assert_eq!(
                range,
                uncommitted.then_some(DiffRange::HeadToWorktree),
                "the root reads only its uncommitted toggle"
            );
        }
    }
}

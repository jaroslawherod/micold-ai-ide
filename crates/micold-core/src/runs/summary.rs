//! A run's change counts for Compare (feature 483, research R7, R8): the sums over feature 482's
//! change list, so Compare and the Changes view count with one reader (FR-010).

use std::io;
use std::path::Path;

use super::RunSummary;
use crate::git::Git;
use crate::review::base::{Base, ReviewScope, Toggles};
use crate::review::changes::{ChangeList, ChangedFile, Origin};

/// The files, added lines and removed lines of `list`, and whether any row holds uncommitted
/// changes. A binary row counts as a file and contributes no lines (its counts are zero).
pub fn totals(list: &ChangeList) -> RunSummary {
    let sum = |line: fn(&ChangedFile) -> u32| {
        list.files
            .iter()
            .fold(0u32, |sum, file| sum.saturating_add(line(file)))
    };
    RunSummary {
        files: u32::try_from(list.files.len()).unwrap_or(u32::MAX),
        added: sum(|file| file.added),
        removed: sum(|file| file.removed),
        uncommitted: has_uncommitted(list),
    }
}

/// Whether any row of `list` holds uncommitted changes: a staged, unstaged or untracked-not-ignored
/// file, alone or beside committed changes (R8).
pub fn has_uncommitted(list: &ChangeList) -> bool {
    list.files
        .iter()
        .any(|file| matches!(file.origin, Origin::Uncommitted | Origin::Both))
}

/// Read the summary of the run whose worktree is `dir`, started from local branch `base_branch`
/// (R7). When `base_branch` is the default branch the Changes view compares with, the Changes
/// view's own base is used, so the two agree to the line (FR-010, SC-004); any other base branch
/// is counted against the merge-base with that branch. One read with both kinds of change on, so
/// the counts and the `uncommitted` tag come from the same answer.
pub fn read(git: &dyn Git, dir: &Path, base_branch: &str) -> io::Result<RunSummary> {
    let view_base = git.review_base(dir);
    let base = match &view_base {
        Base::MergeBase { branch, .. } if short_name(branch) == base_branch => view_base,
        _ => git.review_base_against(dir, base_branch),
    };
    let list = git.change_list(dir, ReviewScope::Worktree { base }, Toggles::default())?;
    Ok(totals(&list))
}

/// A branch as the user names it: `origin/main` is `main`.
fn short_name(branch: &str) -> &str {
    branch.strip_prefix("origin/").unwrap_or(branch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::base::{Base, ReviewScope};
    use crate::review::changes::{ChangeKind, ChangedFile, Content};
    use crate::review::RelPath;

    fn row(path: &str, added: u32, removed: u32, origin: Origin, content: Content) -> ChangedFile {
        ChangedFile {
            path: RelPath::from_git(path),
            kind: ChangeKind::Modified,
            content,
            added,
            removed,
            large: false,
            origin,
        }
    }

    fn list(files: Vec<ChangedFile>) -> ChangeList {
        ChangeList {
            files,
            scope: ReviewScope::Worktree {
                base: Base::Unavailable(crate::review::base::BaseUnavailable::NoDefaultBranch),
            },
        }
    }

    fn text(path: &str, added: u32, removed: u32, origin: Origin) -> ChangedFile {
        row(path, added, removed, origin, Content::Text)
    }

    #[test]
    fn committed_only_rows_sum_and_are_not_uncommitted() {
        let list = list(vec![
            text("a", 10, 2, Origin::Committed),
            text("b", 5, 0, Origin::Committed),
        ]);
        let got = totals(&list);
        assert_eq!((got.files, got.added, got.removed), (2, 15, 2));
        assert!(!got.uncommitted);
    }

    #[test]
    fn uncommitted_only_rows_sum_and_are_uncommitted() {
        let list = list(vec![text("a", 3, 4, Origin::Uncommitted)]);
        let got = totals(&list);
        assert_eq!((got.files, got.added, got.removed), (1, 3, 4));
        assert!(got.uncommitted);
    }

    #[test]
    fn committed_and_uncommitted_rows_sum_together() {
        let list = list(vec![
            text("a", 100, 20, Origin::Committed),
            text("b", 20, 10, Origin::Both),
            text("c", 0, 0, Origin::Uncommitted),
            text("d", 0, 0, Origin::Committed),
        ]);
        let got = totals(&list);
        assert_eq!((got.files, got.added, got.removed), (4, 120, 30));
        assert!(got.uncommitted);
    }

    #[test]
    fn a_binary_row_is_a_file_with_no_lines() {
        let list = list(vec![
            row("img.png", 0, 0, Origin::Committed, Content::Binary),
            text("a", 1, 1, Origin::Committed),
        ]);
        let got = totals(&list);
        assert_eq!((got.files, got.added, got.removed), (2, 1, 1));
    }

    #[test]
    fn an_empty_list_is_zero_files_and_no_lines() {
        let got = totals(&list(Vec::new()));
        assert_eq!(got, RunSummary::default());
        assert_eq!((got.files, got.added, got.removed), (0, 0, 0));
    }

    #[test]
    fn has_uncommitted_is_true_for_untracked_staged_and_unstaged_rows() {
        let untracked = {
            let mut file = text("new.txt", 3, 0, Origin::Uncommitted);
            file.kind = ChangeKind::Untracked;
            file
        };
        // A staged or unstaged edit of a tracked file is `Uncommitted` (or `Both` after a commit).
        for file in [
            untracked,
            text("staged.rs", 1, 1, Origin::Uncommitted),
            text("unstaged.rs", 1, 1, Origin::Both),
        ] {
            assert!(has_uncommitted(&list(vec![file])));
        }
    }

    #[test]
    fn has_uncommitted_is_false_for_committed_only_rows_and_for_nothing() {
        assert!(!has_uncommitted(&list(vec![
            text("a", 1, 1, Origin::Committed),
            text("b", 2, 2, Origin::Committed),
        ])));
        assert!(!has_uncommitted(&list(Vec::new())));
    }
}

//! The pure half of a pick (feature 483, contracts/integration.md): reading git's merge pre-check,
//! choosing how the base branch moves, classifying git's failures and wording the merge commit.
//! The I/O half is the `Git` trait's `merge_tree_write_tree`, `commit_tree_merge`, `update_ref_cas`,
//! `merge_in_checkout` and `merge_abort`.

use std::path::{Path, PathBuf};

use super::PickRefusal;
use crate::review::RelPath;

/// How the base branch moves (I3, I4, I5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The base tip is an ancestor of the run's tip: the branch moves to `run_tip`.
    FastForward {
        /// The run's tip, the base branch's new tip.
        run_tip: String,
    },
    /// A merge commit of both tips, written with `commit-tree` and moved into place with a
    /// compare-and-swap.
    MergeCommit,
    /// The base branch is checked out in `path`: git merges there itself.
    MergeInCheckout {
        /// The worktree that has the base branch checked out.
        path: PathBuf,
    },
}

/// Choose how the base branch moves (I3 to I5). A checked-out base branch is always merged in its
/// checkout, so the files there follow the branch; otherwise the branch ref moves by itself.
pub fn plan(run_tip: &str, is_ancestor: bool, checkout: Option<&Path>) -> Plan {
    match checkout {
        Some(path) => Plan::MergeInCheckout {
            path: path.to_path_buf(),
        },
        None if is_ancestor => Plan::FastForward {
            run_tip: run_tip.to_owned(),
        },
        None => Plan::MergeCommit,
    }
}

/// The conflicted paths in the output of `git merge-tree --write-tree -z --name-only`, each once,
/// in git's order (I2). The output is the tree id, then the conflicted paths, then an empty entry,
/// then git's messages (which are not read).
pub fn parse_merge_tree_conflicts(stdout: &str) -> Vec<RelPath> {
    let mut files: Vec<RelPath> = Vec::new();
    for entry in stdout.split('\0').skip(1).take_while(|e| !e.is_empty()) {
        let path = RelPath::from_git(entry);
        if !files.contains(&path) {
            files.push(path);
        }
    }
    files
}

/// What git's stderr says about a failed pre-check (I2): `GitTooOld` when it names `--write-tree`
/// (git before 2.38), otherwise git's own words.
pub fn classify_stderr(stderr: &str) -> PickRefusal {
    if stderr.contains("write-tree") {
        PickRefusal::GitTooOld
    } else {
        PickRefusal::Git(stderr.trim().to_owned())
    }
}

/// The merge commit's message (I4): the only text a pick writes into the repository.
pub fn merge_message(run: u8, group_name: &str, base: &str) -> String {
    format!("Merge run {run} of {group_name} into {base}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(files: &[&str]) -> Vec<RelPath> {
        files.iter().map(|f| RelPath::from_git(f)).collect()
    }

    #[test]
    fn conflicted_paths_are_read_once_each_up_to_the_empty_entry() {
        // Captured from git 2.43: a modify/delete conflict, a content conflict, a UTF-8 name.
        let out = "00c82d1fef8d834aa97e7f631b2166ef36b02f24\0del.txt\0f 1.txt\0é.txt\0\0\
                   1\0del.txt\0CONFLICT (modify/delete)\0CONFLICT (modify/delete): del.txt \
                   deleted in x and modified in main.\n\0\
                   1\0f 1.txt\0Auto-merging\0Auto-merging f 1.txt\n\0";
        assert_eq!(
            parse_merge_tree_conflicts(out),
            paths(&["del.txt", "f 1.txt", "é.txt"]),
            "each conflicted path is read once, up to the empty entry, names with spaces and UTF-8 intact"
        );
    }

    #[test]
    fn a_repeated_path_is_listed_once() {
        let out = "abc\0a.rs\0a.rs\0b.rs\0\0";
        assert_eq!(
            parse_merge_tree_conflicts(out),
            paths(&["a.rs", "b.rs"]),
            "a path repeated in the output is listed once"
        );
    }

    #[test]
    fn a_clean_merge_has_no_conflicts() {
        assert!(
            parse_merge_tree_conflicts("abc\0").is_empty(),
            "a tree id alone means a clean merge"
        );
        assert!(
            parse_merge_tree_conflicts("abc\0\0messages\0").is_empty(),
            "informational messages after the empty entry are not conflicts"
        );
        assert!(
            parse_merge_tree_conflicts("").is_empty(),
            "empty output has no conflicts"
        );
    }

    #[test]
    fn the_plan_fast_forwards_when_the_base_tip_is_an_ancestor() {
        assert_eq!(
            plan("run", true, None),
            Plan::FastForward {
                run_tip: "run".into()
            },
            "a base tip that is an ancestor of the run is fast-forwarded to the run tip"
        );
    }

    #[test]
    fn the_plan_writes_a_merge_commit_when_the_base_moved_on() {
        assert_eq!(
            plan("run", false, None),
            Plan::MergeCommit,
            "a base that moved on needs a merge commit"
        );
    }

    #[test]
    fn a_checked_out_base_branch_is_merged_in_its_checkout_either_way() {
        let path = Path::new("/repo");
        for ancestor in [true, false] {
            assert_eq!(
                plan("run", ancestor, Some(path)),
                Plan::MergeInCheckout { path: path.into() },
                "a base branch checked out somewhere is merged in that checkout, ancestor or not"
            );
        }
    }

    #[test]
    fn stderr_naming_write_tree_means_git_is_too_old() {
        assert_eq!(
            classify_stderr("error: unknown option `write-tree'\nusage: git merge-tree"),
            PickRefusal::GitTooOld,
            "stderr naming write-tree means git is too old for merge-tree --write-tree"
        );
        assert_eq!(
            classify_stderr("usage: git merge-tree [--write-tree] <b1> <b2>"),
            PickRefusal::GitTooOld,
            "the usage text mentioning write-tree also means git is too old"
        );
    }

    #[test]
    fn any_other_stderr_is_git_s_own_message() {
        assert_eq!(
            classify_stderr("fatal: not a valid object name\n"),
            PickRefusal::Git("fatal: not a valid object name".into()),
            "any other stderr is git's own message, trimmed"
        );
    }

    #[test]
    fn the_merge_message_names_the_run_the_group_and_the_base() {
        assert_eq!(
            merge_message(2, "login page", "main"),
            "Merge run 2 of login page into main",
            "the merge message names the run, the group and the base"
        );
    }
}

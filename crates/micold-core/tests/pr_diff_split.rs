//! A pull request's diff split per file (feature 775, FR-004): path, kind, binary, counts and
//! `large` from GitHub's one unified diff, each file's `patch` readable by `parse_unified`.

use std::path::PathBuf;

use micold_core::review::changes::{ChangeKind, Content, Origin};
use micold_core::review::diff::{parse_unified, FileDiff};
use micold_core::review::limits;
use micold_core::review::pr_diff::split_pr_diff;

fn split(file: &str) -> micold_core::review::pr_diff::PrDiff {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(file);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    split_pr_diff(&raw)
}

#[test]
fn a_rename_keeps_the_new_path_and_the_old_one() {
    let diff = split("pr_diff_rename.txt");
    assert_eq!(diff.files.len(), 1);
    let file = &diff.files[0].file;
    assert_eq!(file.path.as_str(), "src/new_name.rs");
    assert!(
        matches!(&file.kind, ChangeKind::Renamed { from } if from.as_str() == "src/old_name.rs")
    );
    assert_eq!((file.added, file.removed), (1, 1));
    assert_eq!(file.origin, Origin::Committed);
}

#[test]
fn an_added_and_a_deleted_file_are_told_apart() {
    let added = split("pr_diff_add.txt");
    assert_eq!(added.files[0].file.kind, ChangeKind::Added);
    assert_eq!(added.files[0].file.path.as_str(), "docs/new.md");
    assert_eq!(
        (added.files[0].file.added, added.files[0].file.removed),
        (2, 0)
    );
    let deleted = split("pr_diff_delete.txt");
    assert_eq!(deleted.files[0].file.kind, ChangeKind::Deleted);
    assert_eq!(deleted.files[0].file.path.as_str(), "gone.txt");
    assert_eq!(
        (deleted.files[0].file.added, deleted.files[0].file.removed),
        (0, 3)
    );
}

#[test]
fn a_binary_file_is_binary_with_no_counts() {
    let diff = split("pr_diff_binary.txt");
    let file = &diff.files[0].file;
    assert_eq!(file.path.as_str(), "logo.png");
    assert_eq!(file.content, Content::Binary);
    assert_eq!((file.added, file.removed), (0, 0));
}

#[test]
fn a_quoted_path_is_unquoted() {
    let diff = split("pr_diff_quoted.txt");
    assert_eq!(diff.files[0].file.path.as_str(), "späce \"q\".txt");
    assert_eq!(diff.files[0].file.kind, ChangeKind::Modified);
}

#[test]
fn only_a_file_over_the_changed_lines_limit_is_large() {
    let diff = split("pr_diff_large.txt");
    assert_eq!(diff.files.len(), 2);
    assert_eq!(diff.files[0].file.path.as_str(), "big.txt");
    assert!(u64::from(diff.files[0].file.added) > u64::from(limits::MAX_CHANGED_LINES));
    assert!(diff.files[0].file.large);
    assert!(!diff.files[1].file.large);
}

#[test]
fn a_pull_request_with_no_change_has_no_file() {
    assert!(split("pr_diff_empty.txt").files.is_empty());
}

#[test]
fn a_patch_is_what_parse_unified_reads() {
    let diff = split("pr_diff_rename.txt");
    let FileDiff::Text(hunks) = parse_unified(&diff.files[0].patch) else {
        panic!("a text diff");
    };
    assert_eq!(hunks.len(), 1);
}

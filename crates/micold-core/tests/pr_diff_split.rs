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

fn added_lines(count: u32) -> String {
    let mut raw = format!(
        "diff --git a/n.txt b/n.txt\nnew file mode 100644\nindex 0000000..1111111\n--- /dev/null\n+++ b/n.txt\n@@ -0,0 +1,{count} @@\n"
    );
    for i in 0..count {
        raw.push_str(&format!("+line {i}\n"));
    }
    raw
}

#[test]
fn exactly_the_limit_is_not_large_and_one_more_is() {
    let limit = limits::MAX_CHANGED_LINES;
    assert_eq!(limit, 5000);
    let at = split_pr_diff(&added_lines(limit));
    assert_eq!(at.files[0].file.added, limit);
    assert!(!at.files[0].file.large);
    let below = split_pr_diff(&added_lines(limit - 1));
    assert!(!below.files[0].file.large);
    let over = split_pr_diff(&added_lines(limit + 1));
    assert!(over.files[0].file.large);
}

#[test]
fn a_mode_change_with_no_hunk_is_mode_only() {
    let diff = split("pr_diff_mode.txt");
    assert_eq!(diff.files.len(), 1);
    let file = &diff.files[0].file;
    assert_eq!(file.path.as_str(), "scripts/run.sh");
    assert_eq!(file.kind, ChangeKind::ModeOnly);
    assert_eq!((file.added, file.removed), (0, 0));
}

#[test]
fn a_git_binary_patch_is_binary_with_no_counts() {
    let raw = "diff --git a/img.bin b/img.bin\nindex 1111111..2222222 100644\nGIT binary patch\nliteral 4\nLcmZQz00000\n\nliteral 0\nHcmV?d00001\n\n";
    let diff = split_pr_diff(raw);
    assert_eq!(diff.files.len(), 1);
    let file = &diff.files[0].file;
    assert_eq!(file.path.as_str(), "img.bin");
    assert_eq!(file.content, Content::Binary);
    assert_eq!((file.added, file.removed), (0, 0));
}

#[test]
fn a_patch_is_its_section_byte_for_byte() {
    let first = "diff --git a/a.txt b/a.txt\nindex 1..2 100644\n--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n-x\n+y\n \n";
    let second = "diff --git a/b.txt b/b.txt\nindex 1..2 100644\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-p\n+q\n  \n";
    let diff = split_pr_diff(&format!("{first}{second}"));
    assert_eq!(diff.files.len(), 2);
    assert_eq!(diff.files[0].patch, first.as_bytes());
    assert_eq!(diff.files[1].patch, second.as_bytes());
}

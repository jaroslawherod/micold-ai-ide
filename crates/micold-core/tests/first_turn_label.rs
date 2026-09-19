//! A session's label from the first thing the user typed in it (feature 032 — contract
//! `specs/032-untitled-session-labels/contracts/first-turn-label.md`, C2–C5).
//!
//! Three layers, tested separately so a failure says which one broke:
//!
//! - **the bounded prefix** (C2): how much of a record file is read, and which lines of it count;
//! - **shaping** (C5): one line, at most 80 user-perceived characters, cut with `…`;
//! - **the provider's turn rule** (C3 for `claude`): which record is the first *typed* turn, as
//!   opposed to text the CLI or the application inserted.

use std::path::{Path, PathBuf};

use micold_core::first_turn::{read_prefix, shape_label, LABEL_BUDGET_BYTES};

/// The longest label, in extended grapheme clusters (FR-003).
const MAX_GRAPHEMES: usize = 80;

// ---------------------------------------------------------------------------------------
// C5 — shaping
// ---------------------------------------------------------------------------------------

#[test]
fn whitespace_and_line_break_runs_collapse_to_one_space_and_ends_are_trimmed() {
    assert_eq!(
        shape_label("  Fix the\n\n  flaky \t login\r\ntest  ").as_deref(),
        Some("Fix the flaky login test"),
        "a row is one line: every whitespace run, line breaks included, is one space (C5.1)"
    );
}

#[test]
fn whitespace_only_text_yields_no_label() {
    assert_eq!(
        shape_label(" \n\t \u{3000} "),
        None,
        "an empty turn is no label source, so the next turn is looked at instead (C5.2)"
    );
}

#[test]
fn exactly_eighty_graphemes_pass_unchanged() {
    let text = "a".repeat(MAX_GRAPHEMES);
    assert_eq!(
        shape_label(&text).as_deref(),
        Some(text.as_str()),
        "the bound is inclusive: 80 characters fit (C5.3)"
    );
}

#[test]
fn eighty_one_graphemes_yield_the_first_seventy_nine_and_an_ellipsis() {
    let text = format!("{}XY", "a".repeat(MAX_GRAPHEMES - 1));
    let expected = format!("{}…", "a".repeat(MAX_GRAPHEMES - 1));
    assert_eq!(
        shape_label(&text).as_deref(),
        Some(expected.as_str()),
        "one over the bound is cut to 79 plus `…`, 80 in total, so the reader sees it was cut (C5.4)"
    );
}

#[test]
fn the_cut_never_splits_a_grapheme_cluster() {
    // Each of these is ONE user-perceived character made of several code points. Placed as the 79th
    // character of an 81-character line, a cut by `char` would keep only part of it.
    let family = "👨\u{200d}👩\u{200d}👧"; // ZWJ sequence
    let accented = "e\u{0301}"; // base + combining acute
    let cjk = "漢";
    for cluster in [family, accented, cjk] {
        let text = format!("{}{cluster}Z!", "a".repeat(MAX_GRAPHEMES - 2));
        let expected = format!("{}{cluster}…", "a".repeat(MAX_GRAPHEMES - 2));
        assert_eq!(
            shape_label(&text).as_deref(),
            Some(expected.as_str()),
            "{cluster:?} is kept whole at the cut (C5.5)"
        );
    }
}

// ---------------------------------------------------------------------------------------
// C2 — the bounded prefix
// ---------------------------------------------------------------------------------------

fn write_file(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn the_prefix_is_at_most_one_mebibyte() {
    assert_eq!(
        LABEL_BUDGET_BYTES,
        1024 * 1024,
        "the FR-014 bound (research R7)"
    );
    let dir = tempfile::tempdir().unwrap();
    // Short complete lines up to one byte past the bound.
    let line = b"0123456789abcde\n"; // 16 bytes
    let mut contents = line.repeat((LABEL_BUDGET_BYTES as usize) / line.len());
    contents.push(b'x');
    assert_eq!(contents.len() as u64, LABEL_BUDGET_BYTES + 1);
    let path = write_file(dir.path(), "big.jsonl", &contents);

    let prefix = read_prefix(&path).expect("a readable file has a prefix");
    assert_eq!(
        prefix.len() as u64,
        LABEL_BUDGET_BYTES,
        "a file one byte over the bound is read up to the bound and no further (C2.1)"
    );
}

#[test]
fn a_trailing_line_without_a_newline_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "half.jsonl", b"{\"complete\":1}\n{\"being_writ");
    assert_eq!(
        read_prefix(&path).as_deref(),
        Some(&b"{\"complete\":1}\n"[..]),
        "a line still being written is not read — only lines ending in `\\n` count (C2.2)"
    );
}

#[test]
fn a_missing_file_has_no_prefix() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        read_prefix(&dir.path().join("never-written.jsonl")),
        None,
        "a missing record is `None`, never an error (FR-011)"
    );
}

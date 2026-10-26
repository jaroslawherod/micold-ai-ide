//! The user guide's review shortcut table is the review key table (#1276 FR-004, T004).
//!
//! The table sits between two marker comments in `docs/user-guide/reviewing-changes.md`. Each row
//! is one row of [`REVIEW_KEYS`], in its order: the key as the guide writes it (modifiers in the
//! order `Ctrl+Alt+Shift+<key>`, and ``(`Cmd+…` on macOS)`` after a chord on the platform's
//! primary modifier) and the `?` sheet's text verbatim. A key added, dropped, moved or reworded
//! without the guide following fails here. Contract:
//! `specs/1276-review-ui-docs-sync/contracts/guide-shortcut-table.md`.

mod support;

use micold_core::review::keys::{KeyName, ReviewKey, REVIEW_KEYS};
use support::marked_table::{guide_page, marked_section, markers, rows};

const PAGE: &str = "docs/user-guide/reviewing-changes.md";
const MARKER: &str = "review-shortcuts";
const MACOS: &str = " on macOS)";

/// The chord `row` binds with `primary` written as `primary` (`Ctrl` or `Cmd`).
fn chord(row: &ReviewKey, primary: &str) -> String {
    let key = match row.key {
        KeyName::Char(c) => c.to_string(),
        KeyName::Enter => "Enter".to_owned(),
    };
    if !row.mods.primary {
        return key;
    }
    let shift = if row.mods.shift { "Shift+" } else { "" };
    format!("{primary}+{shift}{key}")
}

/// The key cell the guide writes for `row`.
fn key_cell(row: &ReviewKey) -> String {
    let cell = format!("`{}`", chord(row, "Ctrl"));
    if row.mods.primary {
        format!("{cell} (`{}`{MACOS}", chord(row, "Cmd"))
    } else {
        cell
    }
}

fn table() -> Vec<(String, String)> {
    rows(&marked_section(PAGE, MARKER))
}

fn listed(rows: &[(String, String)]) -> String {
    rows.iter()
        .map(|(k, d)| format!("  | {k} | {d} |"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_markers_are_there() {
    let text = guide_page(PAGE);
    let (start, end) = markers(MARKER);
    let at_start = text.find(&start);
    let at_end = text.rfind(&end);
    assert!(at_start.is_some(), "`{PAGE}` has the marker `{start}`");
    assert!(at_end.is_some(), "`{PAGE}` has the marker `{end}`");
    assert!(at_start < at_end, "`{PAGE}`: `{end}` comes after `{start}`");
}

#[test]
fn the_guides_table_is_the_review_key_table() {
    let found = table();
    let expected: Vec<(String, String)> = REVIEW_KEYS
        .iter()
        .map(|row| (key_cell(row), row.sheet.to_owned()))
        .collect();
    assert!(
        found == expected,
        "the guide's review shortcuts are not the review key table, in its order.\n\
         The guide has:\n{}\nREVIEW_KEYS has:\n{}",
        listed(&found),
        listed(&expected)
    );
}

#[test]
fn each_primary_chord_names_its_macos_form() {
    let found = table();
    for (row, (cell, _)) in REVIEW_KEYS.iter().zip(&found) {
        let suffixed = cell.ends_with(MACOS) && cell.contains("(`Cmd+");
        assert_eq!(
            suffixed, row.mods.primary,
            "`{cell}`: a chord on the primary modifier names its `Cmd` form on macOS, \
             and no other key does"
        );
    }
    assert_eq!(found.len(), REVIEW_KEYS.len(), "one row per review key");
}

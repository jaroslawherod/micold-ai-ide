//! The user guide's terminal-only table is the application's list (#943 FR-012, T061).
//!
//! The table sits between two marker comments in `docs/user-guide/worktrees-and-sessions.md`.
//! Each row names one command of [`TERMINAL_ONLY`] (with its other names, if any) and says why it
//! needs the terminal in the words the app shows; the line under the table names
//! [`CHECKED_AGAINST`]. A command added to or dropped from the list, a reason reworded, or the
//! version moved without the guide following fails here.

mod support;

use micold_core::structured::commands::{CHECKED_AGAINST, TERMINAL_ONLY};
use support::marked_table::{marked_section, rows};

const PAGE: &str = "docs/user-guide/worktrees-and-sessions.md";
const MARKER: &str = "terminal-only-commands";

/// The command cell the guide writes for `name` and its `aliases`.
fn command_cell(name: &str, aliases: &[&str]) -> String {
    if aliases.is_empty() {
        format!("`/{name}`")
    } else {
        let also: Vec<String> = aliases.iter().map(|a| format!("`/{a}`")).collect();
        format!("`/{name}` (also {})", also.join(", "))
    }
}

#[test]
fn the_guides_table_is_the_terminal_only_list_with_its_reasons() {
    let found = rows(&marked_section(PAGE, MARKER));
    let expected: Vec<(String, String)> = TERMINAL_ONLY
        .iter()
        .map(|t| (command_cell(t.name, t.aliases), t.why.to_string()))
        .collect();
    assert_eq!(found, expected, "the guide's rows, in the list's order");
}

#[test]
fn the_guide_names_the_version_the_list_was_checked_against() {
    let line = format!("Checked against Claude Code {CHECKED_AGAINST}.");
    assert!(
        marked_section(PAGE, MARKER)
            .lines()
            .any(|l| l.trim() == line),
        "the guide says `{line}` between the markers"
    );
}

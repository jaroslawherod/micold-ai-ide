//! The showcase's view holds no decision logic (feature 020, T054 — Principle I).
//!
//! Constitution 1.5.0 extended Principle I's GUI-wiring exception to cover a development-only binary's
//! own render glue, so `showcase/gallery.rs` and `showcase/main.rs` may be validated by the recorded
//! `quickstart.md` procedure instead of by an automated test. That amendment widens what a
//! NON-NEGOTIABLE principle exempts, and it has a precondition: *no decision logic, branching, or
//! business rule of its own.*
//!
//! A precondition nobody checks is a precondition nobody keeps. So this reads the two glue files and
//! fails when one branches on the showcase's state — at which point the fix is to move the decision
//! into `showcase/state.rs`, where it is driven by `tests/showcase_state.rs` like every other
//! transition.
//!
//! # What is permitted, and why
//!
//! Iterating the catalogue, unwrapping an `Option`, and matching on a **catalogue** value
//! (`Section`, `Layout`) are all fine: they are the shape of "render what the list says", not a
//! decision about behaviour. What is not fine is asking the *state* a question — `match
//! showcase.scheme`, `if showcase.open.is_some()` — because the answer is a rule, and a rule belongs
//! where it can be tested.
//!
//! The first draft of `gallery.rs` failed this check on its own scheme-control label. That is the
//! check working: the label is a decision about state, and it now lives in the reducer with a test.

use std::fs;
use std::path::{Path, PathBuf};

/// The two files the GUI-wiring exception covers, relative to `src/`.
const GLUE: &[&str] = &["showcase/gallery.rs", "showcase/main.rs"];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The glue files, as `(path relative to src/, source)`.
fn glue_sources() -> Vec<(String, String)> {
    GLUE.iter()
        .map(|rel| {
            let path = src_dir().join(rel);
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            ((*rel).to_string(), source)
        })
        .collect()
}

/// Strips `//` line comments and `/* */` blocks, so this file's own subject matter — which both glue
/// files discuss in their module docs — cannot read as a violation.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// A branch, as it appears in source.
const BRANCHES: &[&str] = &["match ", "if ", "unwrap_or_else(", "then_some("];

/// Ways a line can be asking the showcase's state a question.
const STATE: &[&str] = &[
    "showcase.",
    "showcase .",
    ".scheme",
    ".replays(",
    ".running(",
    ".shown(",
    ".open",
];

/// Lines that branch on state, as `(file, line, text)`.
///
/// A line has to do both to count. `if filled == per_row` branches on a local counter and is layout
/// arithmetic; `showcase.grid()` reads state without deciding anything. Only the conjunction is a
/// decision, and only a decision has to be tested somewhere else.
fn decisions(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, src) in sources {
        for (i, line) in code_only(src).lines().enumerate() {
            let branches = BRANCHES.iter().any(|b| line.contains(b));
            let reads_state = STATE.iter().any(|s| line.contains(s));
            if branches && reads_state {
                out.push(format!("{path}:{}  {}", i + 1, line.trim()));
            }
        }
    }
    out
}

#[test]
fn the_glue_decides_nothing() {
    let found = decisions(&glue_sources());
    assert!(
        found.is_empty(),
        "the GUI-wiring exception (Principle I) covers glue with *no decision logic of its own*. \
         These lines branch on the showcase's state, so they are rules, and a rule belongs in \
         `showcase/state.rs` where `tests/showcase_state.rs` drives it:\n{}",
        found.join("\n")
    );
}

/// A scan that reads nothing passes trivially. If either glue file is renamed or moved, this fails
/// rather than certifying an exemption over an empty set.
#[test]
fn the_scan_actually_reads_both_glue_files() {
    let sources = glue_sources();
    assert_eq!(sources.len(), GLUE.len());
    for (path, src) in &sources {
        assert!(
            src.len() > 200,
            "{path} is suspiciously short — if the glue moved, GLUE must move with it, or the \
             exception goes unchecked"
        );
    }
}

/// The synthetic Red: the rule really does fire, and names the line.
#[test]
fn a_view_that_branches_on_the_scheme_fails() {
    let planted = vec![(
        "showcase/gallery.rs".to_string(),
        "fn header() {\n    let label = match showcase.scheme {\n        Light => \"a\",\n    };\n}\n"
            .to_string(),
    )];
    let found = decisions(&planted);
    assert_eq!(found.len(), 1, "expected one decision, got {found:?}");
    assert!(
        found[0].contains("gallery.rs:2"),
        "the failure must name the line: {}",
        found[0]
    );
}

#[test]
fn a_view_that_tests_whether_something_is_open_fails() {
    let planted = vec![(
        "showcase/gallery.rs".to_string(),
        "if showcase.open.is_some() { overlay() }\n".to_string(),
    )];
    assert_eq!(decisions(&planted).len(), 1);
}

/// Layout arithmetic and catalogue matching are not decisions about state, and a rule that flagged
/// them would push the gallery's layout into the reducer, where it does not belong.
#[test]
fn iterating_the_catalogue_and_counting_rows_are_not_decisions() {
    let planted = vec![(
        "showcase/gallery.rs".to_string(),
        "if filled == per_row {\n    stack = stack.push(line);\n}\n\
         let per_row = match layout { Layout::Inline => 3, Layout::FullWidth => 1 };\n\
         if entry.section == section {\n    render(entry);\n}\n"
            .to_string(),
    )];
    assert!(
        decisions(&planted).is_empty(),
        "layout arithmetic and catalogue matching must stay permitted: {:?}",
        decisions(&planted)
    );
}

/// The comment stripper is load-bearing: both glue files' module docs describe this rule in prose,
/// and without stripping, the documentation would fail the check it documents.
#[test]
fn prose_about_branching_on_state_is_not_branching_on_state() {
    let planted = vec![(
        "showcase/main.rs".to_string(),
        "//! Nothing here does `match showcase.scheme`.\n/* nor if showcase.open */\nlet x = 1;\n"
            .to_string(),
    )];
    assert!(
        decisions(&planted).is_empty(),
        "comments survived stripping"
    );
}

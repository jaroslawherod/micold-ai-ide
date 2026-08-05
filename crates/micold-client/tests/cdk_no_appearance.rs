//! The behaviour layer carries no appearance (feature 017, T012 — FR-007).
//!
//! `ui/cdk/` is this codebase's equivalent of Angular's CDK: it decides *how a floating surface
//! behaves* — where it sits, what captures input, when it closes, what order it stacks in — and
//! nothing about how it looks. `ui/material/` supplies the looks by handing the cdk already-
//! resolved values.
//!
//! The split only survives if it is enforced. Left to convention, the first surface that needs "a
//! slightly darker scrim" reaches for a colour role from inside the cdk, and from then on changing
//! the scrim means editing the behaviour layer. So this reads the source and fails on the reach.
//!
//! It scans text rather than types deliberately: the point is that these *names* must not appear,
//! which is a property of the source, not of what it compiles to.

use std::fs;
use std::path::{Path, PathBuf};

fn cdk_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/cdk")
}

/// Every `.rs` file under `ui/cdk/`, recursively, as `(display path, source)`.
fn cdk_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&cdk_dir(), &mut out);
    out.sort();
    out
}

/// Strips `//` line comments and `/* */` blocks, so prose *about* the rule ("carries no colour
/// role") cannot trip the rule. Without this the module doc explaining the split would fail it.
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

/// The vocabulary of appearance. Each entry is `(needle, what it would mean)`.
///
/// `Roles` and `tokens` cover colour roles; the scale modules cover shape, type and spacing. A cdk
/// module that names any of them has started deciding how something looks.
const APPEARANCE: &[(&str, &str)] = &[
    ("Roles", "a colour role set"),
    ("tokens::", "a design token"),
    ("micold_core::tokens", "the token module"),
    ("typography::", "a type role"),
    ("type_scale", "a type role"),
    ("shape::", "a shape size"),
    ("spacing::", "a spacing step"),
    ("elevation", "an elevation level"),
    ("ui::style", "the styling layer"),
    ("crate::ui::style", "the styling layer"),
];

#[test]
fn the_cdk_layer_names_nothing_about_appearance() {
    let mut violations = Vec::new();
    for (path, src) in cdk_sources() {
        let code = code_only(&src);
        for (line_no, line) in code.lines().enumerate() {
            for (needle, meaning) in APPEARANCE {
                if line.contains(needle) {
                    violations.push(format!(
                        "{path}:{} names `{needle}` ({meaning}): {}",
                        line_no + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "the behaviour layer must carry no appearance (FR-007). \
         Appearance belongs in `ui/material/`, which hands the cdk already-resolved values:\n{}",
        violations.join("\n")
    );
}

/// A scan that scans nothing passes trivially. If `ui/cdk/` is ever emptied or moved, this fails
/// rather than reporting a clean bill of health for an empty set.
#[test]
fn the_scan_actually_has_something_to_scan() {
    let sources = cdk_sources();
    assert!(
        !sources.is_empty(),
        "no sources found under {} — the check above would pass vacuously",
        cdk_dir().display()
    );
    assert!(
        sources.iter().any(|(p, _)| p.ends_with("overlay.rs")),
        "expected the overlay primitive under ui/cdk/, found: {:?}",
        sources.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
}

/// The comment stripper is load-bearing — if it silently stopped working, prose would trip the
/// rule and the failure would look like a real violation.
#[test]
fn prose_about_appearance_does_not_count_as_appearance() {
    let src = "//! This module holds no Roles.\n/* not even tokens:: here */\nlet x = 1;\n";
    let code = code_only(src);
    assert!(!code.contains("Roles"), "line comment survived stripping");
    assert!(
        !code.contains("tokens::"),
        "block comment survived stripping"
    );
    assert!(code.contains("let x = 1;"), "stripper ate real code");
}

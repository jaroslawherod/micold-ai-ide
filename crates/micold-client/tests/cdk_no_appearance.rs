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

#[path = "support/source_scan.rs"]
mod source_scan;

use source_scan::{read_rs_under, strip_comments};

use std::path::{Path, PathBuf};

fn cdk_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/cdk")
}

/// Every `.rs` file under `ui/cdk/`, recursively, as `(display path, source)`.
fn cdk_sources() -> Vec<(String, String)> {
    read_rs_under(&[cdk_dir()], Path::new(env!("CARGO_MANIFEST_DIR")))
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
        let code = strip_comments(&src);
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
    let code = strip_comments(src);
    assert!(!code.contains("Roles"), "line comment survived stripping");
    assert!(
        !code.contains("tokens::"),
        "block comment survived stripping"
    );
    assert!(code.contains("let x = 1;"), "stripper ate real code");
}

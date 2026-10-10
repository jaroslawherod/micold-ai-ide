//! The type-ahead knows nothing about branches (feature 021, FR-019, FR-021a).
//!
//! FR-018 asks for a shared primitive rather than a picker-local widget, and FR-019 spells out what
//! that has to mean in practice: the component takes a list of items and cannot name what they are.
//! The whole value of the requirement is in the future — the next picker adopts the component
//! instead of rebuilding it (SC-007) — which is exactly the kind of value that erodes silently. One
//! `if candidate.is_available()` inside the component, added in a hurry by someone who only had a
//! branch list in front of them, and the next author finds a "shared" component they cannot reuse.
//!
//! Every other rule about these two layers is held by a gate that reads the source:
//! `cdk_no_appearance.rs`, `material_boundary.rs`, `material_builder_api.rs`,
//! `component_api_opacity.rs`. This one was the odd one out, held only by review. Now it is not.
//!
//! Text scanning, like its siblings, and for the same reason: the property is about what these
//! *source files* are allowed to name.

#[path = "support/source_scan.rs"]
mod source_scan;

use source_scan::strip_comments;

use std::fs;
use std::path::{Path, PathBuf};

/// The two halves of the component.
fn component_sources() -> Vec<(String, String)> {
    ["src/ui/cdk/picker.rs", "src/ui/material/typeahead.rs"]
        .iter()
        .map(|rel| {
            let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            let src = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            ((*rel).to_string(), src)
        })
        .collect()
}

/// Vocabulary that belongs to the feature module, never to the component.
const DOMAIN_WORDS: &[&str] = &["branch", "Branch", "worktree", "Worktree", "git", "Git"];

#[test]
fn the_component_cannot_name_a_branch_a_worktree_or_git() {
    let mut violations = Vec::new();
    for (path, src) in component_sources() {
        for (n, line) in strip_comments(&src).lines().enumerate() {
            for word in DOMAIN_WORDS {
                if line.contains(word) {
                    violations.push(format!("  {path}:{} names `{word}`", n + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the shared type-ahead named domain vocabulary:\n{}\n\nFR-019: the component takes a list \
         of rows and knows nothing about what they are. Whatever this needed belongs in the \
         feature module that builds the rows — `ui/worktree_form.rs` — which is free to know all \
         about branches.",
        violations.join("\n")
    );
}

/// A scan over files that do not exist passes trivially, which is the failure mode this whole file
/// would otherwise have.
#[test]
fn the_scan_actually_reads_both_halves() {
    let sources = component_sources();
    assert_eq!(sources.len(), 2);
    for (path, src) in sources {
        assert!(
            src.contains("//!"),
            "{path} has no module doc — is it the file this gate thinks it is?"
        );
    }
}

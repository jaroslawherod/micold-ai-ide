//! Nothing under test reads project prose (feature 023, FR-024/FR-025).
//!
//! CI skips the entire build when a change touches only declared documentation. That is sound for
//! exactly one reason: no test or build step reads those files, so skipping the suite cannot hide a
//! failure. The constitution's documentation-only exemption says so in writing — and a precondition
//! nobody checks is a precondition nobody keeps, which is why it is checked here instead.
//!
//! This follows the precedent 1.5.0 set with `showcase_glue.rs`: when a NON-NEGOTIABLE principle's
//! exemption has a checkable precondition, the check ships with the exemption rather than resting
//! on review.
//!
//! # What counts as a violation
//!
//! Any string literal in a scanned source file that resolves — against the repository root, or
//! against the file's own directory, the way `include_str!` does — to a path that exists and
//! carries `micold-docs`. Existence is what makes this precise: fixture strings that merely *look*
//! like documentation paths (`"docs/abc-204-token-refresh"`, a branch name in the typeahead
//! corpus) resolve to nothing and are ignored.
//!
//! A handful of literals do resolve to real documentation paths while reading nothing. Those are
//! allowlisted below with a written reason, and a stale entry fails too — a list that can rot is a
//! list that stops meaning anything.
//!
//! # Scope
//!
//! Rust sources under `crates/`, which is what `cargo build` and `cargo test` compile. A shell
//! script that read a documentation file into a test would slip past this; if that ever becomes a
//! real pattern here, widen the scan rather than trusting the gap.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, iter};

/// Literals that resolve to a real documentation path but read nothing.
///
/// `(file relative to the repository root, exact literal, why it is not a read)`
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "crates/micold-core/tests/typeahead_corpus.rs",
        "docs/user-guide",
        "A fixture branch name in the typeahead corpus. Branch names look like paths, and this one \
         collides with a real directory; the test never touches the filesystem.",
    ),
    (
        "crates/micold-core/tests/submodule_failure_detail.rs",
        "README.md",
        "A filename created inside a temporary git repository the test builds, not this \
         repository's README.",
    ),
];

/// This file, which is exempt from its own scan.
///
/// The allowlist has to quote the paths it exempts, so scanning this file finds every entry in
/// ALLOWED and reports it as a violation — the gate tripping over its own bookkeeping. The cost of
/// the exemption is that a genuine documentation read *inside this file* would go unnoticed; that
/// is acceptable, because this file is the gate, and its reads exist to run the gate.
const SELF: &str = "crates/micold-core/tests/documentation_is_not_read.rs";

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

/// Every `.rs` file under `crates/`, skipping build output.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "target" || name == "target-shared" || name.starts_with('.') {
                    continue;
                }
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("crates"), &mut out);
    out.sort();
    out
}

/// String literals on a line, ignoring comment-only lines.
///
/// Deliberately simple: a comment mentioning a documentation path is a reference, not a read, and
/// the existence check downstream is what carries the precision.
fn literals(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut lit = String::new();
        let mut closed = false;
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    // Keep the escape's payload; a literal path never needs one, and swallowing the
                    // pair keeps `\"` from ending the string early.
                    let _ = chars.next();
                    lit.push('?');
                }
                '"' => {
                    closed = true;
                    break;
                }
                _ => lit.push(c),
            }
        }
        if closed && !lit.is_empty() {
            out.push(lit);
        }
    }
    out
}

/// Where a literal points, if it points at something real inside the repository.
fn resolve(root: &Path, source: &Path, literal: &str) -> Option<String> {
    if literal.len() > 200 || literal.contains('\n') {
        return None;
    }
    let here = source.parent()?;
    for candidate in [root.join(literal), here.join(literal)] {
        let Ok(real) = candidate.canonicalize() else {
            continue;
        };
        let Ok(rel) = real.strip_prefix(root) else {
            continue; // outside the repository entirely
        };
        // check-attr speaks forward slashes on every platform.
        return Some(rel.to_string_lossy().replace('\\', "/"));
    }
    None
}

/// Ask the one matcher, in one call, which of these paths are declared documentation.
fn documentation_paths(root: &Path, paths: &BTreeSet<String>) -> BTreeSet<String> {
    if paths.is_empty() {
        return BTreeSet::new();
    }
    let mut child = Command::new("git")
        .args(["check-attr", "--stdin", "-z", "micold-docs"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git check-attr");
    {
        let stdin = child.stdin.as_mut().expect("check-attr stdin");
        for p in paths {
            stdin.write_all(p.as_bytes()).expect("write path");
            stdin.write_all(&[0]).expect("write NUL");
        }
    }
    let out = child.wait_with_output().expect("run git check-attr");
    assert!(
        out.status.success(),
        "git check-attr failed: {:?}",
        out.status
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut fields = text.split('\0');
    let mut docs = BTreeSet::new();
    while let (Some(path), Some(_attr), Some(value)) = (fields.next(), fields.next(), fields.next())
    {
        if value == "set" {
            docs.insert(path.to_string());
        }
    }
    docs
}

/// `(source file relative to root, literal, resolved documentation path)`
fn violations() -> Vec<(String, String, String)> {
    let root = repo_root();
    let mut found: Vec<(String, String, String)> = Vec::new();
    let mut candidates = BTreeSet::new();

    for source in rust_sources(&root) {
        let Ok(text) = fs::read_to_string(&source) else {
            continue;
        };
        let rel_source = source
            .strip_prefix(&root)
            .unwrap_or(&source)
            .to_string_lossy()
            .replace('\\', "/");
        if rel_source == SELF {
            continue;
        }
        for literal in text.lines().flat_map(literals) {
            if let Some(target) = resolve(&root, &source, &literal) {
                candidates.insert(target.clone());
                found.push((rel_source.clone(), literal, target));
            }
        }
    }

    let docs = documentation_paths(&root, &candidates);
    found.retain(|(_, _, target)| docs.contains(target));
    found
}

fn is_allowed(file: &str, literal: &str) -> bool {
    ALLOWED.iter().any(|(f, l, _)| *f == file && *l == literal)
}

#[test]
fn no_test_or_build_step_reads_declared_documentation() {
    let offenders: Vec<_> = violations()
        .into_iter()
        .filter(|(file, literal, _)| !is_allowed(file, literal))
        .collect();

    assert!(
        offenders.is_empty(),
        "these literals resolve to declared documentation:\n{}\n\n\
         CI skips the whole build when a change touches only those paths, so a test that reads one \
         makes the skip unsound: the change that breaks the test is the change CI declines to run. \
         Either stop reading the file, remove the path from `.gitattributes`, or — if the literal \
         reads nothing — add it to ALLOWED with a reason.",
        offenders
            .iter()
            .map(|(file, literal, target)| format!("  {file}: \"{literal}\" -> {target}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_allowlist_entry_still_matches() {
    let found = violations();
    let stale: Vec<_> = ALLOWED
        .iter()
        .filter(|(file, literal, _)| {
            !found
                .iter()
                .any(|(f, l, _)| f == file && l.as_str() == *literal)
        })
        .collect();

    assert!(
        stale.is_empty(),
        "these ALLOWED entries no longer match anything: {stale:?}\n\
         A list that can rot stops meaning anything — delete them."
    );
}

#[test]
fn the_scan_can_see() {
    // Guards against the scan silently finding nothing because a path or a walk broke: without
    // this, an empty result reads identically to a clean tree.
    let root = repo_root();
    let sources = rust_sources(&root);
    assert!(
        sources.len() > 50,
        "scanned only {} Rust sources under crates/ — the walk is broken, not the tree",
        sources.len()
    );
    let saw_literals = sources
        .iter()
        .filter_map(|p| fs::read_to_string(p).ok())
        .flat_map(|t| t.lines().flat_map(literals).collect::<Vec<_>>())
        .chain(iter::once(String::new()))
        .count();
    assert!(saw_literals > 100, "extracted almost no string literals");
}

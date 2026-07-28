//! Two launches of the showcase render the same page (feature 020, T007 — FR-022, SC-010).
//!
//! SC-010 asks that two consecutive launches show the same components, the same sample data and the
//! same order, with nothing varying by time, randomness or host. Without screenshots, the only honest
//! way to assert that is to forbid the inputs that could vary — so this reads the gallery's source and
//! fails when it names one.
//!
//! It is a *source* scan rather than a behavioural one deliberately. Determinism is a property of
//! what the gallery is allowed to read, and a behavioural test would have to run the gallery twice
//! and compare something — which, for a page whose output is pixels, is the image diffing this
//! feature's spec puts out of scope.
//!
//! The rule takes its input, so its failure behaviour is proved rather than assumed: a vacuity guard
//! alone would not have been a Red here, because the module skeleton already existed for the scan to
//! find.

use std::fs;
use std::path::{Path, PathBuf};

fn showcase_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/showcase")
}

/// Every `.rs` file under `src/showcase/`, recursively, as `(path relative to src/, source)`.
fn showcase_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&showcase_dir(), &mut out);
    out.sort();
    out
}

/// Strips `//` line comments and `/* */` blocks, so prose *about* the rule cannot trip it — this
/// file's subject matter appears in the gallery's own module docs.
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

/// The vocabulary of a page that could differ between two launches, and what each would mean.
const NONDETERMINISM: &[(&str, &str)] = &[
    ("Instant::now", "the clock"),
    ("SystemTime", "the clock"),
    ("Utc::now", "the clock"),
    ("rand", "a random source"),
    ("new_v4", "a random identifier"),
    ("env::var", "the environment"),
    ("current_dir", "the process's working directory"),
    ("home_dir", "the host's home directory"),
    ("read_to_string", "the filesystem"),
    ("File::open", "the filesystem"),
];

/// Lines of real code naming a source of variation, as `(file, line, text, meaning)`.
fn offenders(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, src) in sources {
        for (i, line) in code_only(src).lines().enumerate() {
            for (needle, meaning) in NONDETERMINISM {
                if line.contains(needle) {
                    out.push(format!(
                        "{path}:{} reads {meaning} (`{needle}`): {}",
                        i + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn the_gallery_reads_nothing_that_could_differ_between_launches() {
    let found = offenders(&showcase_sources());
    assert!(
        found.is_empty(),
        "the gallery's content must be fixed: the same components, the same sample data and the same \
         ordering on every launch (FR-022, SC-010). Sample content belongs in `samples.rs` as a \
         constant, not read from the world:\n{}",
        found.join("\n")
    );
}

/// A scan that scans nothing passes trivially. If `src/showcase/` moves, this fails rather than
/// reporting a clean bill of health for an empty set.
#[test]
fn the_scan_actually_finds_the_gallery() {
    let sources = showcase_sources();
    assert!(
        !sources.is_empty(),
        "no sources found under {} — the check above would pass vacuously",
        showcase_dir().display()
    );
    for expected in ["showcase/catalogue.rs", "showcase/samples.rs"] {
        assert!(
            sources.iter().any(|(p, _)| p == expected),
            "expected {expected} among the gallery's sources, found: {:?}",
            sources.iter().map(|(p, _)| p).collect::<Vec<_>>()
        );
    }
}

/// The synthetic Red: the rule really does fail, and names the line.
#[test]
fn a_gallery_that_reads_the_clock_fails() {
    let planted = vec![(
        "showcase/samples.rs".to_string(),
        "pub fn label() -> String {\n    format!(\"{:?}\", Instant::now())\n}\n".to_string(),
    )];
    let found = offenders(&planted);
    assert_eq!(found.len(), 1, "expected one offender, got {found:?}");
    assert!(
        found[0].contains("samples.rs:2") && found[0].contains("the clock"),
        "the failure must name the file, the line and what it read: {}",
        found[0]
    );
}

#[test]
fn a_gallery_that_invents_an_identifier_fails() {
    let planted = vec![(
        "showcase/catalogue.rs".to_string(),
        "let id = uuid::Uuid::new_v4();\n".to_string(),
    )];
    assert_eq!(offenders(&planted).len(), 1);
}

/// The comment stripper is load-bearing: without it, this file's own subject matter written as prose
/// in the gallery's module docs would read as a violation, and the failure would look real.
#[test]
fn prose_about_the_clock_does_not_count_as_reading_it() {
    let planted = vec![(
        "showcase/mod.rs".to_string(),
        "//! Nothing here calls Instant::now.\n/* nor rand */\nlet x = 1;\n".to_string(),
    )];
    assert!(
        offenders(&planted).is_empty(),
        "comments survived stripping"
    );
}

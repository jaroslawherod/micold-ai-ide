//! The showcase touches none of the application's state (feature 020, T017 — FR-017, FR-020).
//!
//! US1's independent test asks a person to launch the showcase on a machine with no configuration and
//! no git repository, then inspect the running processes and confirm no session daemon was started.
//! That is a good check and it is also the weak half: it proves nothing about the launch nobody
//! thought to inspect, and it cannot be run in CI at all.
//!
//! The property underneath it is structural and checkable. The showcase cannot spawn a daemon, read a
//! settings file or touch a worktree if it never *names* the modules that do those things — and an
//! import added in a hurry is exactly how a development tool starts writing to somebody's
//! configuration. So this reads the gallery's source and fails on the reach.
//!
//! Text scanning, not type inspection: the property is about what a source file is allowed to name.
//! A module that cannot name `micold_core::store` cannot persist a project list.

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

/// Strips comments, so the module docs that explain this rule cannot trip it.
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

/// What the showcase must not reach, and what reaching it would mean.
///
/// Each entry is a capability, not a convenience. Together they are the whole of FR-020: the showcase
/// runs without a session daemon, without a git repository and without any saved application state,
/// and creates, reads or modifies none of them.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "micold_core::store",
        "the project store — it would read or write the user's saved workspace",
    ),
    (
        "micold_core::settings",
        "the settings store — it would read or write the user's preferences",
    ),
    (
        "micold_core::endpoint",
        "the daemon endpoint — the first step toward connecting to one",
    ),
    (
        "micold_core::spawn",
        "process spawning — it could start a session daemon (FR-017)",
    ),
    (
        "micold_core::git",
        "git — it would touch a repository the showcase must not require or modify",
    ),
    (
        "micold_core::worktree",
        "worktrees — Principle III's territory, and none of the showcase's business",
    ),
    (
        "micold_core::fs_scan",
        "filesystem scanning — the gallery's content is fixed, not discovered (FR-022)",
    ),
    ("micold_client::daemon", "the daemon connection"),
    ("crate::daemon", "the daemon connection"),
    (
        "dark_light",
        "the host's theme preference — the scheme comes from the showcase's own control (FR-009)",
    ),
];

/// Lines of real code naming something forbidden, as `file:line naming what it means`.
fn reaches(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, src) in sources {
        for (i, line) in code_only(src).lines().enumerate() {
            for (needle, meaning) in FORBIDDEN {
                if line.contains(needle) {
                    out.push(format!(
                        "{path}:{} names `{needle}` ({meaning}): {}",
                        i + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    out
}

/// The headline claim: launching the showcase cannot start a daemon, read state, or touch a repository,
/// because nothing in it can name the code that would.
#[test]
fn the_showcase_reaches_none_of_the_applications_state() {
    let found = reaches(&showcase_sources());
    assert!(
        found.is_empty(),
        "the showcase MUST run without a session daemon, without a git repository and without any \
         saved application state, and MUST NOT create, read or modify any of them (FR-017, FR-020). \
         It is a development tool; a user's configuration is not its to touch:\n{}",
        found.join("\n")
    );
}

/// A scan that scans nothing passes trivially. If `src/showcase/` moves, this fails rather than
/// certifying isolation for an empty set.
#[test]
fn the_scan_actually_finds_the_gallery() {
    let sources = showcase_sources();
    assert!(
        !sources.is_empty(),
        "no sources found under {} — the check above would pass vacuously",
        showcase_dir().display()
    );
    for expected in ["showcase/main.rs", "showcase/gallery.rs"] {
        assert!(
            sources.iter().any(|(p, _)| p == expected),
            "expected {expected}; if the showcase moved, this file's path must move with it"
        );
    }
}

/// The synthetic Red: the rule really does fire, and names the line and the capability.
#[test]
fn a_gallery_that_reads_the_settings_file_fails() {
    let planted = vec![(
        "showcase/main.rs".to_string(),
        "fn boot() {\n    let s = micold_core::settings::JsonFileSettingsStore::default_location();\n}\n"
            .to_string(),
    )];
    let found = reaches(&planted);
    assert_eq!(found.len(), 1, "expected one reach, got {found:?}");
    assert!(
        found[0].contains("main.rs:2") && found[0].contains("settings store"),
        "the failure must name the line and what it would do: {}",
        found[0]
    );
}

#[test]
fn a_gallery_that_could_spawn_a_daemon_fails() {
    let planted = vec![(
        "showcase/main.rs".to_string(),
        "micold_core::spawn::ensure_running(&endpoint);\n".to_string(),
    )];
    let found = reaches(&planted);
    assert_eq!(found.len(), 1, "got {found:?}");
    assert!(found[0].contains("FR-017"), "{}", found[0]);
}

#[test]
fn a_gallery_that_follows_the_host_theme_fails() {
    let planted = vec![(
        "showcase/state.rs".to_string(),
        "let scheme = dark_light::detect();\n".to_string(),
    )];
    assert_eq!(reaches(&planted).len(), 1);
}

/// The comment stripper is load-bearing: `main.rs`'s module doc lists every one of these names in
/// prose, precisely to say it does not use them. Without stripping, the documentation would fail the
/// check it documents.
#[test]
fn prose_naming_what_is_forbidden_is_not_a_reach() {
    let planted = vec![(
        "showcase/main.rs".to_string(),
        "//! Names no micold_core::store, no micold_core::spawn, no dark_light.\n\
         /* nor micold_core::git */\nlet x = 1;\n"
            .to_string(),
    )];
    assert!(reaches(&planted).is_empty(), "comments survived stripping");
}

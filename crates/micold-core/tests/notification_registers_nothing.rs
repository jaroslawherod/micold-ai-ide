//! A desktop notification registers nothing and reads nothing from outside (feature 039).
//!
//! The notification for "a session needs you" is deliberately inert: clicking it only brings the
//! app forward (FR-015a), so there is no toast activator, no URL protocol handler, and no
//! command-line argument the client interprets. And the daemon stays out of the desktop session
//! altogether (FR-007): it never links a notification crate, so it cannot notify from a headless
//! host.
//!
//! Each of those is a promise about what is *absent*, the kind that
//! decays quietly: a protocol handler is a plausible way to "make the click open the session", and
//! it would be a new attack surface nobody decided on. So they are checked properties here.
//!
//! # What is scanned, and what is not
//!
//! Every Rust file of the client (for the command line, every one of them: an argument read in a
//! module is as read as one in `main.rs`) and the daemon's manifest. Comments are skipped, so the files may explain the decision in their own words.
//! Not `tests/`: this file names the very strings it forbids.

use std::fs;
use std::path::{Path, PathBuf};

const MAIN_RS: &str = "crates/micold-client/src/main.rs";
const DAEMON_TOML: &str = "crates/micold-daemon/Cargo.toml";

const CLIENT_SRC: &str = "crates/micold-client/src";

const SCANNED: &[&str] = &[MAIN_RS, DAEMON_TOML];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Numbered, trimmed lines that are not comments according to `is_comment`.
fn code_lines(text: &str, is_comment: fn(&str) -> bool) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !is_comment(l))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// TOML comments start with `#`.
fn toml_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

/// A Rust line comment (`//`, which takes in `///` and `//!`). Block comments are removed first,
/// by [`strip_rust_block_comments`]: a line that starts with `*` may be code, the continuation of
/// a product.
fn rust_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Replaces every `/* … */` span, which may cross lines, with nothing but its newlines so that line
/// numbers stay true.
fn strip_rust_block_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find("*/").map_or(after.len(), |e| e + 2);
        out.extend(after[..end].chars().filter(|c| *c == '\n'));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The code lines of a Rust file: its comments of both kinds left out.
fn rust_code_lines(text: &str) -> Vec<(usize, String)> {
    code_lines(&strip_rust_block_comments(text), rust_comment)
}

/// Every `.rs` file under `rel`, repo-relative, sorted.
fn rust_files_under(rel: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut dirs = vec![repo_root().join(rel)];
    while let Some(dir) = dirs.pop() {
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let relative = path
                    .strip_prefix(repo_root())
                    .expect("under the repository")
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

/// What U48 forbids: the ways a Rust program reads its command line.
const ARGUMENT_READERS: &[&str] = &["env::args", "args_os", "clap", "pico_args"];

/// The `file:line: text` of every non-comment line that contains one of `needles`.
fn offending(
    rel: &str,
    lines: &[(usize, String)],
    needles: &[&str],
    ignore_case: bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for (n, text) in lines {
        let hay = if ignore_case {
            text.to_lowercase()
        } else {
            text.clone()
        };
        for needle in needles {
            let needle_cmp = if ignore_case {
                needle.to_lowercase()
            } else {
                (*needle).to_string()
            };
            if hay.contains(&needle_cmp) {
                found.push(format!("{rel}:{n}: names `{needle}` — {text}"));
            }
        }
    }
    found
}

// U48 (FR-015a, contract N8): the client has no command line to interpret.
#[test]
fn the_client_reads_no_command_line_argument() {
    let files = rust_files_under(CLIENT_SRC);
    assert!(
        files.iter().any(|f| f == MAIN_RS),
        "the walk of {CLIENT_SRC} does not reach {MAIN_RS}: {files:?}"
    );
    let found: Vec<String> = files
        .iter()
        .flat_map(|rel| offending(rel, &rust_code_lines(&read(rel)), ARGUMENT_READERS, false))
        .collect();
    assert!(
        found.is_empty(),
        "the client reads a command-line argument:\n  {}\n\n\
         Contract N8: a notification click carries no instruction, so the client interprets no \
         argument that one could be smuggled in through.",
        found.join("\n  ")
    );
}

// U49 (FR-007): the daemon never notifies the desktop; it has no notification crate.
#[test]
fn the_daemon_depends_on_no_notification_crate() {
    let found = offending(
        DAEMON_TOML,
        &code_lines(&read(DAEMON_TOML), toml_comment),
        &["zbus", "mac-usernotifications", "tauri-winrt-notification"],
        false,
    );
    assert!(
        found.is_empty(),
        "the daemon's manifest names a desktop notification crate:\n  {}\n\n\
         FR-007: the daemon runs on hosts with no desktop session; the client notifies.",
        found.join("\n  ")
    );
}

#[test]
fn the_scan_reads_what_it_claims_to() {
    // A scan pointed at nothing passes forever.
    for rel in SCANNED {
        let path = repo_root().join(rel);
        assert!(
            path.is_file(),
            "{rel} does not exist; the scan covers nothing"
        );
        assert!(
            !read(rel).trim().is_empty(),
            "{rel} is empty; the scan covers nothing"
        );
    }
    // And it can see a violation when there is one.
    // A line that starts with `*` is code when it continues an expression, and a block comment
    // that crosses lines is a comment on every one of them (issue #572).
    let planted = "fn main() {\n    let n = 2\n        * std::env::args().count();\n    /* env::args\n    args_os */\n}";
    assert_eq!(
        offending("x", &rust_code_lines(planted), ARGUMENT_READERS, false),
        vec!["x:3: names `env::args` — * std::env::args().count();".to_string()],
        "the continuation line is read as code, and the block comment is not"
    );
}

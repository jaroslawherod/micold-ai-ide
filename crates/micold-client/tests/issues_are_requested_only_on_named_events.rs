//! GitHub is asked for issues only when the user asks for them (feature 034, FR-003, FR-024).
//!
//! Opening the add-worktree form must contact nothing but the daemon's local `RemoteList`. The
//! issue source runs `gh`, which reaches GitHub, and the spec allows that on exactly two named
//! events in this milestone: choosing the **GitHub issue** source, and pressing **Retry** after a
//! failed load (M3 adds the debounced search). A load on form open, on reconnect or on a timer
//! would pass every behavioural test of the picker and quietly send the repository's name to
//! GitHub for a user who never chose the source — which is what FR-025's opt-in notice promises
//! does not happen.
//!
//! So this counts the callers, in the pattern of `refresh_is_only_on_demand.rs`: every
//! non-comment line under `crates/micold-client/src/` naming a [`MARKERS`] entry must be in
//! [`ALLOWED`], with a reason, and every [`ALLOWED`] entry must still match a line.

use std::fs;
use std::path::{Path, PathBuf};

/// The names that mean "issues are being fetched": the capability that builds the `gh`-backed
/// source, and the one shell function that uses it.
const MARKERS: &[&str] = &[".issue_tooling()", "start_issue_load("];

/// `(file relative to the repository root, the trimmed line, why it is not another trigger)`.
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "crates/micold-client/src/shell/issues.rs",
        "fn start_issue_load(app: &mut App, seq: u64) -> Task<Message> {",
        "The definition of the single load path. A signature names the function; it calls nothing.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "let tooling = app.caps.issue_tooling();",
        "The only use of the capability, inside `start_issue_load`, so every fetch goes through the \
         two callers below.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(chosen) => start_issue_load(app, chosen),",
        "Named event 1: the reducer accepted `SourceChanged(Issue)` and now awaits a fresh seq.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(retried) => start_issue_load(app, retried),",
        "Named event 2: the reducer accepted `IssueRetry` from a failed load.",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

fn client_sources(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("crates/micold-client/src"), &mut out);
    out.sort();
    out
}

/// A line naming a marker in code rather than in a comment.
fn names_a_marker(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') {
        return false;
    }
    MARKERS.iter().any(|m| line.contains(m))
}

fn call_sites() -> Vec<(String, String)> {
    let root = repo_root();
    let mut found = Vec::new();
    for source in client_sources(&root) {
        let Ok(text) = fs::read_to_string(&source) else {
            continue;
        };
        let rel = source
            .strip_prefix(&root)
            .unwrap_or(&source)
            .to_string_lossy()
            .replace('\\', "/");
        for line in text.lines().filter(|l| names_a_marker(l)) {
            found.push((rel.clone(), line.trim().to_string()));
        }
    }
    found
}

#[test]
fn issues_are_fetched_only_from_the_named_events() {
    let offenders: Vec<_> = call_sites()
        .into_iter()
        .filter(|(file, line)| !ALLOWED.iter().any(|(f, l, _)| f == file && l == line))
        .collect();
    assert!(
        offenders.is_empty(),
        "these lines fetch issues and are not accounted for:\n{}\n\nGitHub is contacted only when \
         the user chooses the issue source or retries a failed load (FR-003). A load on form open, \
         reconnect or a timer contradicts FR-025's opt-in notice; anything else (a rename, a test) \
         goes in ALLOWED with its reason.",
        offenders
            .iter()
            .map(|(f, l)| format!("  {f}: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_allowlist_entry_still_matches() {
    let found = call_sites();
    let stale: Vec<_> = ALLOWED
        .iter()
        .filter(|(file, line, _)| !found.iter().any(|(f, l)| f == file && l == line))
        .map(|(file, line, _)| format!("  {file}: {line}"))
        .collect();
    assert!(
        stale.is_empty(),
        "these ALLOWED entries match nothing — the load path moved and the gate did not:\n{}",
        stale.join("\n")
    );
}

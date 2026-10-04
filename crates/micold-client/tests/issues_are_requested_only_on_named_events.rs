//! GitHub is asked for issues only when the user asks for them (feature 034, FR-003, FR-024).
//!
//! Opening the add-worktree form must contact nothing but the daemon's local `RemoteList`. The
//! issue source runs `gh`, which reaches GitHub, and the spec allows that on exactly three named
//! events: choosing the **GitHub issue** source, pressing **Retry** after a failed load or search,
//! and the debounce after a keystroke on a capped list running out (FR-005a). The descriptions of a
//! loaded list are read in a pass that follows the load: it starts when a load was accepted and
//! goes on when a page of it was, so it adds no event of its own (038 FR-024). A load on form open,
//! on reconnect or on a timer
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
/// source, and the three shell functions that use it — the load, the search beyond it and the
/// description pass that follows a load.
const MARKERS: &[&str] = &[
    ".issue_tooling()",
    "start_issue_load(",
    "start_issue_search(",
    "start_issue_descriptions(",
];

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
    (
        "crates/micold-client/src/shell/issues.rs",
        "fn start_issue_search(app: &mut App, seq: u64) -> Task<Message> {",
        "The definition of the single search path; it calls nothing.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "let source = app.caps.issue_tooling().source;",
        "The capability's use inside `start_issue_search`: the source is built from the `gh` the \
         load located, so the search never locates again (R3).",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(due) => start_issue_search(app, due),",
        "Named event 3: the debounce after a keystroke on a capped list ran out and the reducer \
         accepted `IssueSearchDue` for the pending seq (FR-005a, R9).",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(retried) => start_issue_search(app, retried),",
        "Named event 2, for a search: the reducer accepted `IssueRetry` from a failed search.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "fn start_issue_descriptions(app: &mut App, request: DescriptionRequest) -> Task<Message> {",
        "The definition of the single path to `describe_open`; it calls nothing.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "let describing = app.caps.issue_tooling().source;",
        "The capability's use inside `start_issue_descriptions`: the source is built from the `gh` \
         the load located, as the search's is.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(first) => start_issue_descriptions(app, first),",
        "038 U99 (FR-024): the reducer accepted a load (named events 1 and 2) and now awaits the \
         first page of its descriptions. A stale or failed load awaits none.",
    ),
    (
        "crates/micold-client/src/shell/issues.rs",
        "Some(next) => start_issue_descriptions(app, next),",
        "038 U99 (FR-024): the reducer accepted a page of that pass and awaits the one after it. \
         A stale page, the last page, the cap and a failure await none.",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

fn client_sources(root: &Path) -> Vec<PathBuf> {
    // An unreadable directory or file fails the scan rather than shrinking it: a scan that skipped
    // part of the tree would pass without having looked there.
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|e| panic!("cannot read an entry of {}: {e}", dir.display()))
                .path();
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
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", source.display()));
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
         the user chooses the issue source, retries, or types on a capped list (FR-003, FR-005a). A load on form open, \
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

/// 038 U77 (FR-024, SC-006) — nothing under `src/ui/` fetches issues. A row's tooltip is built and
/// opened there, from the description the issue already carries, so a cursor resting on a row
/// cannot cause a request: the code that draws has no way to make one.
#[test]
fn no_view_code_fetches_issues() {
    const UI: &str = "crates/micold-client/src/ui/";
    let sites = call_sites();
    let in_ui: Vec<_> = sites
        .iter()
        .filter(|(file, _)| file.starts_with(UI))
        .map(|(f, l)| format!("  {f}: {l}"))
        .collect();
    assert!(
        in_ui.is_empty(),
        "view code names the issue source; a tooltip, a hover or a redraw must never fetch:\n{}",
        in_ui.join("\n")
    );
    assert!(
        ALLOWED.iter().all(|(file, _, _)| !file.starts_with(UI)),
        "no ALLOWED entry excuses a fetch from view code"
    );
    assert!(
        !sites.is_empty(),
        "the scan finds the shell's call sites, so finding none under `ui/` means something"
    );
    let root = repo_root();
    assert!(
        client_sources(&root).iter().any(|p| p
            .strip_prefix(&root)
            .is_ok_and(|rel| rel.to_string_lossy().replace('\\', "/").starts_with(UI))),
        "the scan reads the files under `ui/`"
    );
}

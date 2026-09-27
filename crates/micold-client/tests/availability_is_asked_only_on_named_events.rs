//! The client asks which AI CLIs a directory has only on the events contract C1 names (feature
//! 033, SC-003, FR-006, FR-004).
//!
//! A per-row answer is cheap to keep and expensive to get: every request runs the service's
//! PATH lookup, and with environment-include on, the user's script too. Nothing the other tests
//! check would notice an extra asker — a request on every hover, a timer, a view that asks while it
//! draws — because they all assert what an answer *does*, never who asked for it. Feature 026's
//! research R11 and SC-006 forbid exactly that per-render lookup; this file is where it is held.
//!
//! So this counts the askers. Contract C1 (`specs/033-directory-aware-start-affordance/contracts/
//! availability-asks.md`) lists them: connect (A1), Settings opened (A2), a row's start list opened
//! (A3), a project opened or switched to (A4), a worktree-list push (A5), agent worktrees revealed
//! or hidden (A5b), a project forgotten (A6), and an environment-include change (A7).
//!
//! # What counts as a violation
//!
//! Any non-comment line under `crates/micold-client/src/` that names one of [`MARKERS`] and is not
//! in [`ALLOWED`], or that is in [`ALLOWED`] but appears a different number of times than the entry
//! says. The count is part of the entry because several sites share one spelling —
//! `sync_cli_availability(app);` is both the connect's sync and the catalog push's — and an entry
//! without a count would silently wave through a third copy.
//!
//! Any marker line under `src/ui/` fails whatever the allowlist says: a view runs on every frame,
//! so nothing drawn may ask (contract C1, "Nothing under `ui/` sends it").
//!
//! A stale entry fails too, as in `refresh_is_only_on_demand.rs`, the precedent for this scan.
//!
//! # Scope
//!
//! `crates/micold-client/src/` minus `src/main_tests.rs`, whose tests drive the askers rather than
//! being askers. Inline `#[cfg(test)]` code is scanned like the rest: several files put a
//! test-gated item above production code, so skipping "from the first `#[cfg(test)]` on" would
//! hide production code (M2 review), and no inline test names a marker today.

use std::fs;
use std::path::{Path, PathBuf};

/// The names that mean "an availability answer is being asked for".
///
/// The wire request, the one function that sends it, and the two that call it for a set of keys.
/// A new asker has to name one of them to reach the service.
const MARKERS: &[&str] = &[
    "ask_cli_availability",
    "sync_cli_availability",
    "refresh_cli_availability",
    // Bare, not `ClientMsg::`-qualified: a glob import would otherwise slip past. The reply is
    // `AiCliAvailability`, which this does not match.
    "AiCliAvailabilityRequest",
];

/// The lines that may name a marker.
///
/// `(file relative to the repository root, the trimmed line, how many times it appears in that
/// file, why each appearance is a contract C1 site or a definition)`
const ALLOWED: &[(&str, &str, usize, &str)] = &[
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "pub fn ask_cli_availability(app: &mut App, key: AvailabilityKey) {",
        1,
        "The definition every asker funnels through.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "d.send(ClientMsg::AiCliAvailabilityRequest { req, cwd });",
        1,
        "The single send, inside `ask_cli_availability`.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "pub fn sync_cli_availability(app: &mut App) {",
        1,
        "The definition of \"sync\": prune to the rows on screen, ask the unasked ones.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "pub fn refresh_cli_availability(app: &mut App) {",
        1,
        "The definition of the env-include refresh: ask home and every row again.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "ask_cli_availability(app, AvailabilityKey::Dir(dir));",
        2,
        "Inside `sync_cli_availability` (the unasked rows) and `refresh_cli_availability` (every \
         row). Both are the bodies of the definitions above, not callers of their own.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "ask_cli_availability(app, AvailabilityKey::Home);",
        2,
        "C1 A1, `on_connected` asking home after the clear; and C1 A7, `refresh_cli_availability` \
         asking home again.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "sync_cli_availability(app);",
        2,
        "C1 A1, `on_connected` asking every row after home; and C1 A5, the `CatalogChanged` arm \
         after `reconcile_catalog(.., true)`.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "refresh_cli_availability(app);",
        1,
        "C1 A7, the `SettingsChanged` arm, only when `env_include_changed` says the echoed \
         settings differ from those the answers were asked under.",
    ),
    (
        "crates/micold-client/src/shell/persist.rs",
        "crate::shell::daemon_sync::ask_cli_availability(",
        1,
        "C1 A2, Settings opened: asks home, for the Settings default select.",
    ),
    (
        "crates/micold-client/src/main.rs",
        "shell::daemon_sync::ask_cli_availability(",
        1,
        "C1 A3, the `StartMenuOpened` arm: a row's list refreshes that row's answer.",
    ),
    (
        "crates/micold-client/src/main.rs",
        "shell::daemon_sync::sync_cli_availability(app);",
        2,
        "C1 A6, the `ForgetConfirmed` arm (prunes the forgotten project's rows); and C1 A5b, the \
         `ShowAgentWorktreesToggled` arm (asks revealed rows, prunes hidden ones).",
    ),
    (
        "crates/micold-client/src/shell/workspace.rs",
        "crate::shell::daemon_sync::sync_cli_availability(app);",
        2,
        "C1 A4, `open_verified_project` and `on_known_project_reopened`, after `set_worktrees`.",
    ),
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-client/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

/// Every `.rs` file except `main_tests.rs` under `crates/micold-client/src/`.
fn client_sources(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|n| n != "main_tests.rs")
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("crates/micold-client/src"), &mut out);
    out.sort();
    out
}

/// A line that names a marker in code rather than in prose.
///
/// Only `//` comments are skipped. This crate writes no `/* */` blocks, and skipping lines that
/// start with `*` would also skip code that starts with a dereference.
fn names_a_marker(line: &str) -> bool {
    if line.trim_start().starts_with("//") {
        return false;
    }
    MARKERS.iter().any(|m| line.contains(m))
}

/// `(file relative to root, the trimmed line)` for every non-test site the scan can see.
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

fn count(found: &[(String, String)], file: &str, line: &str) -> usize {
    found.iter().filter(|(f, l)| f == file && l == line).count()
}

#[test]
fn nothing_but_the_contracts_named_events_asks_for_availability() {
    let found = call_sites();
    let offenders: Vec<_> = found
        .iter()
        .filter(|(file, line)| !ALLOWED.iter().any(|(f, l, _, _)| f == file && l == line))
        .collect();

    assert!(
        offenders.is_empty(),
        "these lines ask which AI CLIs a directory has and are not accounted for:\n{}\n\n\
         Availability is asked only on the events contract C1 names (feature 033, SC-003). A new \
         asker on a timer, a hover, a redraw or any other event contradicts the feature and \
         belongs in a spec change. If it is a new C1 site, add it to ALLOWED with its C1 id.",
        offenders
            .iter()
            .map(|(file, line)| format!("  {file}: {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn each_allowed_line_appears_exactly_as_often_as_its_entry_says() {
    let found = call_sites();
    let wrong: Vec<_> = ALLOWED
        .iter()
        .filter_map(|(file, line, expected, _)| {
            let seen = count(&found, file, line);
            (seen != *expected)
                .then(|| format!("  {file}: {line} — expected {expected}, found {seen}"))
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "these ALLOWED entries no longer match the code:\n{}\n\n\
         A count of 0 is a stale entry: re-point it or delete it. A higher count is a new asker \
         sharing an old spelling: account for it in the entry's reason, with its C1 id.",
        wrong.join("\n")
    );
}

#[test]
fn nothing_under_ui_asks_for_availability() {
    let drawn: Vec<_> = call_sites()
        .into_iter()
        .filter(|(file, _)| file.starts_with("crates/micold-client/src/ui/"))
        .collect();
    assert!(
        drawn.is_empty(),
        "a view runs on every frame, so nothing under ui/ may ask (contract C1):\n{drawn:?}"
    );
    assert!(
        ALLOWED
            .iter()
            .all(|(file, ..)| !file.starts_with("crates/micold-client/src/ui/")),
        "no allowlist entry may exempt a line under ui/"
    );
}

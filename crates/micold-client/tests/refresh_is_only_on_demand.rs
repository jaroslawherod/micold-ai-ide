//! The worktree list is re-read only when someone asks for it (feature 029, FR-012).
//!
//! "On demand" is the whole point of the feature. A refresh that also fired on a timer, on a
//! filesystem event, on reconnect, or on every navigation would still satisfy every other
//! requirement here — the list would refresh, the button would work, the notice would appear — and
//! it would quietly undo the reason the button exists. Nothing in the spec's other checks can see
//! that, because they all assert what happens *after* a refresh starts, never who started it.
//!
//! So this counts the starters. There is exactly one: the sidebar header's refresh control.
//!
//! # What counts as a violation
//!
//! Any non-comment line under `crates/micold-client/src/` that names [`MARKERS`] — the wire request
//! (`ClientMsg::WorktreeRefresh`) or the message that produces it
//! (`WorktreeMsg::RefreshRequested`) — and is not in [`ALLOWED`]. The five lines that exist today
//! are listed there with a reason each: one button, one route, one send, one reducer feed, and one
//! test that reads back what was sent.
//!
//! A stale entry fails too. An allowlist that can rot is a list that stops meaning anything, and
//! the rot here would be silent: the entry that no longer matches anything is also the entry that
//! no longer guards anything.
//!
//! This is a tripwire, not a proof. It cannot tell a subscription from a button press; what it can
//! do is refuse to let either appear without someone writing down which one it is. That is enough,
//! because the failure mode being guarded against is not malice but drift — a `Subscription` here,
//! a reconnect hook there, each reasonable on its own.
//!
//! # Scope
//!
//! `crates/micold-client/src/` only. The daemon end is not scanned: it re-reads what it is asked to
//! re-read, and asking is the client's job. Tests are not scanned either — driving the message is
//! how they test it, and a test that fires a refresh fires nothing at a user.
//!
//! `crates/micold-core/tests/documentation_is_not_read.rs` is the precedent for the scan style,
//! including the rule about stale entries.

use std::fs;
use std::path::{Path, PathBuf};

/// The names that mean "a refresh is being asked for".
///
/// Both, not either: `ClientMsg::WorktreeRefresh` is the request that reaches the daemon, and
/// `WorktreeMsg::RefreshRequested` is the message that reaches the shell function which sends it.
/// A new caller would name one or the other, and which one it names says how deep it reached in.
const MARKERS: &[&str] = &[
    "ClientMsg::WorktreeRefresh",
    "WorktreeMsg::RefreshRequested",
];

/// The lines that may name a marker.
///
/// `(file relative to the repository root, the line with surrounding whitespace trimmed, why it is
/// not a second way to start a refresh)`
const ALLOWED: &[(&str, &str, &str)] = &[
    (
        "crates/micold-client/src/ui/sidebar.rs",
        ".then_some(Message::Worktree(WorktreeMsg::RefreshRequested)),",
        "The refresh control in the sidebar header — the one starter. `then_some` is what makes it \
         a starter and not an automation: the message exists only when `can_refresh_worktrees()` \
         says so, and only a press delivers it.",
    ),
    (
        "crates/micold-client/src/main.rs",
        "Message::Worktree(WorktreeMsg::RefreshRequested) => {",
        "The routing arm. A match pattern reads the message, it does not produce one.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "ClientMsg::WorktreeRefresh { req, project }",
        "The single send, inside `on_worktree_refresh_requested`. Everything above funnels here, \
         which is why one entry can speak for the whole path.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        ".update(Message::Worktree(WorktreeMsg::RefreshRequested));",
        "The same function feeding the pure reducer so the control goes busy. It reacts to the \
         request it is already handling; it does not raise a second one.",
    ),
    (
        "crates/micold-client/src/shell/daemon_sync.rs",
        "let ClientMsg::WorktreeRefresh { req, project: p } = &sent[0] else {",
        "An inline test reading back what the send above put on the wire. A destructuring pattern, \
         and a test besides.",
    ),
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-client/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

/// Every `.rs` file under `crates/micold-client/src/`, skipping build output.
fn client_sources(root: &Path) -> Vec<PathBuf> {
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
    walk(&root.join("crates/micold-client/src"), &mut out);
    out.sort();
    out
}

/// A line that names a marker in code rather than in prose.
///
/// Doc comments here talk about the refresh path at length, and every one of them is a reference,
/// not a caller. The check is deliberately crude — a marker inside a `/* */` block or a string
/// would be counted — because the cost of a false positive is one allowlist entry with a reason,
/// and the cost of a false negative is the requirement this file exists to hold.
fn names_a_marker(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') {
        return false;
    }
    MARKERS.iter().any(|m| line.contains(m))
}

/// `(file relative to root, the trimmed line)` for every call site the scan can see.
///
/// Line numbers are deliberately not part of the identity: they move whenever anything above them
/// moves, and an allowlist that has to be renumbered is an allowlist people stop reading.
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

fn is_allowed(file: &str, line: &str) -> bool {
    ALLOWED.iter().any(|(f, l, _)| *f == file && *l == line)
}

#[test]
fn nothing_but_the_sidebar_control_asks_for_a_refresh() {
    let offenders: Vec<_> = call_sites()
        .into_iter()
        .filter(|(file, line)| !is_allowed(file, line))
        .collect();

    assert!(
        offenders.is_empty(),
        "these lines ask for a worktree refresh and are not accounted for:\n{}\n\n\
         The list is re-read on demand and only on demand (FR-012). If this is a new automatic, \
         periodic, reconnect-driven or filesystem-driven refresh, it contradicts the feature and \
         belongs in a spec change rather than here. If it is not — a rename, a pattern match, a \
         test — add it to ALLOWED with a reason saying which.",
        offenders
            .iter()
            .map(|(file, line)| format!("  {file}: {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_allowlist_entry_still_matches() {
    let found = call_sites();
    let stale: Vec<_> = ALLOWED
        .iter()
        .filter(|(file, line, _)| !found.iter().any(|(f, l)| f == file && l.as_str() == *line))
        .collect();

    assert!(
        stale.is_empty(),
        "these ALLOWED entries match nothing:\n{}\n\n\
         The code moved and the exemption did not. Re-point the entry at the line that replaced it, \
         or delete it — an entry that guards nothing still reads like a guarantee.",
        stale
            .iter()
            .map(|(file, line, _)| format!("  {file}: {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

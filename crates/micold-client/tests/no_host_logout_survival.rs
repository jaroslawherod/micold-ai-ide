//! The application no longer offers to make a host-process service survive logout (feature 028,
//! T016 — FR-005, packaging contract §4.11).
//!
//! The old opt-in worked by registering the daemon with the user's own service manager
//! (`loginctl enable-linger`, then `systemctl --user enable --now micold-daemon.socket`). That
//! registration is precisely what this feature removes, so the promise goes with it: a service
//! running directly on the machine does not survive logout, on any platform. The sandboxed
//! placement keeps the promise through the container runtime's restart policy, which is not a
//! session-scoped registration and is not what this guard is about.
//!
//! A guard rather than a deletion diff, because the removal has several separate surfaces — a core
//! function, a message, and a menu item — and leaving any one behind leaves a path back. The menu
//! item alone is enough: an entry that calls nothing is worse than the feature it lost.
//!
//! # What it does *not* forbid
//!
//! `LogoutSurvivalOutcome` stays. It reported the removed host-process attempt's result, but it now
//! carries the sandbox checkbox's answer (feature 027 FR-014d), which §4.13 keeps. Forbidding the
//! name would delete a working control to satisfy a guard.
//!
//! Scanned: `crates/micold-client/src/` and `crates/micold-core/src/`, **line comments stripped**.
//! The removal is a fact about code, not about prose: a module doc that says "this used to offer
//! `Keep sessions after logout`" is the record of the removal, not a way back to it. Not `tests/`,
//! so this file may quote the names it forbids.

use std::fs;
use std::path::{Path, PathBuf};

/// `(needle, why it may not appear)`.
///
/// The user-facing string is in the list on purpose. The message and the function could both be
/// gone while the menu still rendered the label, and a menu item is the only surface a user can
/// see.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "LogoutSurvivalRequested",
        "the message the removed menu item sent",
    ),
    (
        "Keep sessions after logout",
        "the label of the removed menu item — a label is the part a user can still find",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/micold-client")
        .to_path_buf()
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Whether the text calls or imports the removed host-process `enable`.
///
/// Spelled as a scan rather than a `contains`, because `enable_for` — the placement-aware entry the
/// sandbox still uses — starts with the forbidden name. A plain substring would forbid the thing
/// this feature deliberately keeps.
fn names_host_enable(text: &str) -> bool {
    const PREFIX: &str = "logout_survival::enable";
    text.match_indices(PREFIX)
        .any(|(i, _)| !text[i + PREFIX.len()..].starts_with("_for"))
}

/// The text with whole-line `//` comments dropped.
///
/// Prose that *names* the removed feature is how the removal stays explained — `ui/toolbar.rs`
/// records which two commands the menu lost, and that sentence is worth more than the guard's
/// literal reading of it. What may not survive is a line that still calls or renders the thing.
fn code_only(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One violation, phrased so the failure says what to do about it.
#[derive(Debug, PartialEq)]
struct Violation(String);

/// The rule over one file's text.
fn violations_in(label: &str, text: &str) -> Vec<Violation> {
    let text = &code_only(text);
    let mut out: Vec<Violation> = FORBIDDEN
        .iter()
        .filter(|(needle, _)| text.contains(needle))
        .map(|(needle, why)| {
            Violation(format!(
                "{label} names `{needle}` — {why} (FR-005, packaging contract §4.11)"
            ))
        })
        .collect();
    if names_host_enable(text) {
        out.push(Violation(format!(
            "{label} calls `logout_survival::enable` — the host-process mechanism registered the \
             daemon with the user's service manager, which this feature removes. The sandboxed \
             placement keeps the promise; `enable_for` is what is left (FR-005)"
        )));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The real sources
// ---------------------------------------------------------------------------------------------

#[test]
fn no_client_or_core_source_offers_host_logout_survival() {
    let roots = [
        repo_root().join("crates/micold-client/src"),
        repo_root().join("crates/micold-core/src"),
    ];

    let mut found = Vec::new();
    for root in &roots {
        let sources = rust_sources(root);
        assert!(
            !sources.is_empty(),
            "found no sources under {} — if the layout moved, this guard must move with it, or \
             §4.11 goes unchecked",
            root.display()
        );
        for path in sources {
            let text = fs::read_to_string(&path).expect("read a source file");
            let label = path
                .strip_prefix(repo_root())
                .unwrap_or(&path)
                .display()
                .to_string();
            found.extend(violations_in(&label, &text));
        }
    }

    assert!(
        found.is_empty(),
        "the application still offers to make a host-process service survive logout:\n{}",
        found
            .iter()
            .map(|v| format!("  {}", v.0))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The sandbox's survival is *not* what this removes (packaging contract §4.13). Asserting the
/// placement-aware entry survives keeps the guard from being satisfied by deleting too much.
#[test]
fn the_sandbox_placement_keeps_its_own_survival_path() {
    let module = fs::read_to_string(repo_root().join("crates/micold-core/src/logout_survival.rs"))
        .expect("read logout_survival.rs — the sandbox still needs it (research R8)");
    assert!(
        module.contains("pub fn enable_for"),
        "`enable_for` is gone — §4.13 keeps the sandboxed placement's survive-logout opt-in; only \
         the host-process mechanism is removed"
    );
    assert!(
        module.contains("PendingSandboxRestart"),
        "the sandbox's 'takes effect on next start' outcome is gone with it"
    );
}

// ---------------------------------------------------------------------------------------------
// The synthetic Red: the rule really does fail
// ---------------------------------------------------------------------------------------------

#[test]
fn a_clean_source_passes() {
    assert_eq!(
        violations_in(
            "src/x.rs",
            "let outcome = micold_core::logout_survival::enable_for(&placement);",
        ),
        vec![]
    );
}

#[test]
fn a_source_that_calls_the_host_path_fails() {
    let found = violations_in(
        "src/x.rs",
        "Ok(micold_core::logout_survival::enable(&endpoint))",
    );
    assert_eq!(found.len(), 1, "expected exactly one violation: {found:?}");
    assert!(found[0].0.contains("enable_for"), "{}", found[0].0);
}

#[test]
fn a_menu_that_still_offers_it_fails() {
    let found = violations_in(
        "src/ui/toolbar.rs",
        "MenuItem::new(Icon::AutoMode, \"Keep sessions after logout\", Message::LogoutSurvivalRequested)",
    );
    assert_eq!(
        found.len(),
        2,
        "expected the label and the message: {found:?}"
    );
}

/// The other half of stripping comments: a doc comment that *records* the removal must pass, or
/// the guard punishes the explanation. This is `ui/toolbar.rs`'s real first line.
#[test]
fn a_comment_recording_the_removal_passes() {
    assert_eq!(
        violations_in(
            "src/ui/toolbar.rs",
            "//! It used to carry a theme-mode cycle and a \"Keep sessions after logout\" command.",
        ),
        vec![]
    );
}

/// And the sandbox's reporting message is not the removed surface (§4.13): it now answers the
/// survive-logout checkbox, so naming it must not be a violation.
#[test]
fn the_sandbox_outcome_message_is_not_forbidden() {
    assert_eq!(
        violations_in(
            "src/shell/service_control.rs",
            "Task::done(Message::Connection(ConnectionMsg::LogoutSurvivalOutcome(m)))",
        ),
        vec![]
    );
}

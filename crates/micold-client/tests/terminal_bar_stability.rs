//! Two structural gates that keep terminal focus honest (feature 023, T004/T009).
//!
//! # Why a source-level test at all
//!
//! Both of these guard preconditions rather than behaviour, and neither is reachable from an
//! ordinary test. They follow the shape `tests/showcase_glue.rs` established when constitution
//! 1.5.0 widened Principle I's GUI-wiring exception: *a precondition nobody checks is a
//! precondition nobody keeps.*
//!
//! ## `the_bar_does_not_branch_on_focus`
//!
//! This is the reported bug's actual mechanism, and it fails **silently**. `iced_widget::button`
//! publishes `on_press` on `ButtonReleased`, and only if the `is_pressed` flag stored in its node
//! of the widget tree is set. `Tree::diff_children` zips old and new children *by position*. So a
//! bottom-bar child that is pushed only while the terminal is focused shifts every sibling after it
//! by one index the moment focus changes — and a press that changes focus does exactly that between
//! its own press and release. The pressed control is then diffed against its left neighbour's node,
//! `is_pressed` is gone, and `ButtonReleased` publishes nothing. The click vanishes. The user
//! presses again, it works, and it reads as a slow application rather than a bug.
//!
//! Deleting the click-outside release removed today's trigger. This removes the class: as long as
//! the bar's child list is the same length whatever focus is doing, no sibling can shift under a
//! press. The release affordance was therefore always pushed, with only its `on_press` gated —
//! until BUG-001 retired the control outright (FR-021b), which satisfies the same rule from the
//! other direction: a child that never exists cannot shift its siblings either. See
//! `the_bar_has_no_release_focus_control` below for why the removal had to be unconditional.
//!
//! ## `no_scattered_release_writes`
//!
//! `terminal_released` replaced a `terminal_focused` bool that seven scattered assignments had to
//! keep correct between them — which is how project switch, mode toggle and instance switch each
//! ended up missing a case. Two intent-named helpers are now the only writers. This is what stops
//! the eighth assignment, and it reads the whole crate rather than `app.rs` alone, because the
//! helpers are `pub(crate)` and `features/session.rs` calls one of them.

use std::fs;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Strips `//` line comments and `/* */` blocks, so this file's subject matter — which the sources
/// themselves discuss at length in their doc comments — cannot read as a violation.
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

/// Every `.rs` file under `src/`, as `(path relative to src/, code with comments stripped)`.
fn crate_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(root)
                    .expect("under src/")
                    .to_string_lossy()
                    .into_owned();
                let src = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                out.push((rel, code_only(&src)));
            }
        }
    }
    let root = src_dir();
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    assert!(!out.is_empty(), "found no sources under {}", root.display());
    out
}

// ---- T009: the bottom bar's child list must not depend on focus ----

/// How the pane's bottom bar asks whether the terminal holds the keyboard.
///
/// Reading the predicate is fine and expected — the release affordance is *disabled* when it reads
/// false. What is forbidden is deciding whether a child exists at all.
const FOCUS_QUESTION: &[&str] = &["terminal_focused", "state.terminal_focused()"];

/// Adding or removing a child from a row.
const PUSHES_A_CHILD: &[&str] = &[".push(", "row!", "column!"];

#[test]
fn the_bar_does_not_branch_on_focus() {
    let src = crate_sources()
        .into_iter()
        .find(|(rel, _)| rel == "ui/terminal.rs" || rel == "ui\\terminal.rs")
        .map(|(_, src)| src)
        .expect("ui/terminal.rs must exist");

    // A `push` guarded by the focus question, on the same line or the line the guard opens.
    let lines: Vec<&str> = src.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_guard = line.contains("if ") && FOCUS_QUESTION.iter().any(|q| line.contains(q));
        if !is_guard {
            continue;
        }
        // Look at the guard's line and the two after it: `if state.terminal_focused() {` followed
        // by a `bar = bar.push(..)` is the exact shape that swallowed the press.
        let window = lines[i..lines.len().min(i + 3)].join(" ");
        if PUSHES_A_CHILD.iter().any(|p| window.contains(p)) {
            found.push(format!("ui/terminal.rs:{}  {}", i + 1, line.trim()));
        }
    }

    assert!(
        found.is_empty(),
        "the terminal's bottom bar must not add or remove a child as a function of focus \
         (feature 023 FR-002/FR-008a). A focus-conditional child shifts every sibling after it, \
         and iced's positional tree diff then drops the pressed sibling's `is_pressed` state — the \
         press is silently swallowed and the user has to press twice. Push the affordance \
         unconditionally and gate its `on_press` instead:\n{}",
        found.join("\n")
    );
}

/// Feature 026 FR-003/FR-008a: the bar must not add or remove a child as a function of **how many
/// instances are open** either.
///
/// The same rule as `the_bar_does_not_branch_on_focus`, applied to the thing this feature changes.
/// The strip used to be pushed only once a session had more than one instance — an
/// `if let Some(switcher)` around a `push`, which is precisely the shape that shifts every sibling
/// after it. Opening a second instance therefore renumbered the "+" and the mode toggle under
/// whatever press was in flight.
///
/// FR-003 makes the strip unconditional, which removes today's trigger. This removes the class:
/// as long as the bar's child list does not vary with the session's shape, no sibling can shift
/// under a press. It reads the source rather than the behaviour for the reason the module doc
/// gives — the failure is silent, and no behavioural test can see a press that was never published.
#[test]
fn the_bar_does_not_branch_on_how_many_instances_are_open() {
    let src = crate_sources()
        .into_iter()
        .find(|(rel, _)| rel == "ui/terminal.rs" || rel == "ui\\terminal.rs")
        .map(|(_, src)| src)
        .expect("ui/terminal.rs must exist");

    /// The ways the source can ask "how many instances does this session have".
    const INSTANCE_COUNT_QUESTION: &[&str] = &[
        "shells.len()",
        "shells.is_empty()",
        "instance_switcher_row(",
        "tab_strip_row(",
    ];

    let lines: Vec<&str> = src.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_guard = (line.contains("if ") || line.contains("if let"))
            && INSTANCE_COUNT_QUESTION.iter().any(|q| line.contains(q));
        if !is_guard {
            continue;
        }
        let window = lines[i..lines.len().min(i + 3)].join(" ");
        if PUSHES_A_CHILD.iter().any(|p| window.contains(p)) {
            found.push(format!("ui/terminal.rs:{}  {}", i + 1, line.trim()));
        }
    }

    assert!(
        found.is_empty(),
        "the terminal's bottom bar must not add or remove a child as a function of the session's \
         instance count (feature 026 FR-003, feature 023 FR-008a). A conditional child shifts every \
         sibling after it, and iced's positional tree diff then drops the pressed sibling's \
         `is_pressed` — the press is silently swallowed and the user has to press twice. The strip \
         is always drawn now; push it unconditionally:\n{}",
        found.join("\n")
    );
}

// ---- BUG-001/T037: the bar carries no release-focus control ----

/// Feature 023 FR-021b: the bottom bar must not carry a release-focus affordance.
///
/// It dates from feature 006, when focus was acquired only by an explicit click and lost by
/// clicking outside — an always-visible way out was a real safety valve then. Feature 023 replaced
/// both halves of that model: navigation acquires the keyboard automatically and the click-outside
/// release is gone. What was left was a permanently-visible button, disabled in every state where
/// the terminal does not hold the keyboard, doing a job the reserved `Ctrl+Shift+E` / `Cmd+Shift+E`
/// chord already does — and that chord is what carries 006 FR-011's "never trapped" guarantee.
///
/// Note what this gate does *not* relax. `the_bar_does_not_branch_on_focus` above still applies:
/// the removal has to be unconditional. A child that never exists cannot shift its siblings, so
/// deleting the control satisfies the structural precondition the same way pushing it
/// unconditionally did — but making it *conditional* on focus would reintroduce the swallowed
/// press. Both gates pass only for the unconditional removal.
#[test]
fn the_bar_has_no_release_focus_control() {
    let src = crate_sources()
        .into_iter()
        .find(|(rel, _)| rel == "ui/terminal.rs" || rel == "ui\\terminal.rs")
        .map(|(_, src)| src)
        .expect("ui/terminal.rs must exist");

    let found: Vec<String> = src
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("Icon::ReleaseFocus"))
        .map(|(i, line)| format!("ui/terminal.rs:{}  {}", i + 1, line.trim()))
        .collect();

    assert!(
        found.is_empty(),
        "the terminal's bottom bar must not carry a release-focus affordance (feature 023 \
         FR-021b, BUG-001). The reserved Ctrl+Shift+E / Cmd+Shift+E chord is the only explicit \
         release, and navigation clears a release on its own. Remove the control \
         **unconditionally** — gating it on focus instead would satisfy this gate while violating \
         `the_bar_does_not_branch_on_focus`:\n{}",
        found.join("\n")
    );
}

// ---- T004: `terminal_released` has exactly two writers ----

/// The helpers permitted to assign it, and the field's own declaration.
const PERMITTED: &[&str] = &[
    "pub(crate) fn focus_terminal",
    "pub(crate) fn release_terminal",
    "pub terminal_released",
];

#[test]
fn no_scattered_release_writes() {
    let mut found = Vec::new();
    for (rel, src) in crate_sources() {
        let lines: Vec<&str> = src.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let assigns =
                line.contains("terminal_released =") || line.contains("terminal_released:");
            if !assigns {
                continue;
            }
            // Permitted if this line, or one of the three above it, is a helper's signature or the
            // field's declaration. Three is enough for `fn f(&mut self) {` + a doc-free body.
            let start = i.saturating_sub(3);
            let window = lines[start..=i].join(" ");
            if PERMITTED.iter().any(|p| window.contains(p)) {
                continue;
            }
            found.push(format!("{rel}:{}  {}", i + 1, line.trim()));
        }
    }

    assert!(
        found.is_empty(),
        "`terminal_released` may be written only by `focus_terminal()` and `release_terminal()` \
         (feature 023). Seven scattered assignments to the `terminal_focused` bool this replaced \
         are what made project switch, mode toggle and instance switch each miss a case; these \
         lines are the eighth. Call the helper that names the intent instead:\n{}",
        found.join("\n")
    );
}

/// `012` BUG-004 / FR-010 — the wiring half, which a unit test cannot reach.
///
/// `restart_message` decides what the bar's restart control must act on, and `ui/terminal.rs`'s own
/// unit tests prove that decision correct. Neither notices if the button stops asking: the defect
/// was `.on_press(Message::Session(SessionMsg::TerminalRestartRequested))` written straight into
/// the bar, restarting the session while the bar described a shell instance — and the session's AI CLI primary is still
/// alive in Regular mode, so pressing it did nothing at all.
///
/// This is the same shape as `the_bar_does_not_branch_on_focus` above: a precondition of the bar
/// that no ordinary test can observe, so it is read out of the source instead.
#[test]
fn the_bars_restart_control_asks_which_process_it_is_restarting() {
    let src = fs::read_to_string(src_dir().join("ui").join("terminal.rs")).expect("terminal.rs");
    let code = code_only(&src);

    assert!(
        code.contains("on_press(restart_message("),
        "the bar's restart control must take its message from `restart_message`, which splits on \
         the attached process the way `attached_process_restartable` already does"
    );
    assert!(
        !code.contains("on_press(Message::Session(SessionMsg::TerminalRestartRequested))"),
        "a bare session-level restart in the bar is BUG-004: in Regular mode the session's primary \
         is still running, so the request is a no-op and the control does nothing"
    );
}

/// Feature 026-multi-provider-sessions FR-016a (T058a): the bar's pinned AI tab names the session's
/// CLI by its **command** name, and the label follows the session rather than a constant.
///
/// `ui/terminal.rs`'s own unit test proves `session_provider` answers with the session's CLI. What
/// it cannot see is whether the tab still *asks* — a label rewritten to `display_name()`, or to a
/// literal `"claude"`, leaves that test green and puts "GitHub Copilot" (or the wrong CLI outright)
/// on a Copilot session's bar. That is the same silent shape as the restart control above, so it is
/// read out of the source in the same way.
///
/// The register matters as much as the source. FR-016's clarification splits the two deliberately:
/// rows and the bar are labels in a width budget and take `command()`, while menus and failure
/// messages are sentences and take `display_name()`. `tests/features_sidebar.rs` and
/// `features_settings.rs` hold the ends of that split; this holds the terminal bar's.
#[test]
fn the_pinned_ai_tab_names_the_sessions_cli() {
    let src = fs::read_to_string(src_dir().join("ui").join("terminal.rs")).expect("terminal.rs");
    let code = code_only(&src);
    let tab = code
        .split_once("fn pinned_ai_tab")
        .expect(
            "ui/terminal.rs no longer builds a pinned AI tab — FR-016a hangs the CLI's name \
                 on it, so it cannot simply be gone",
        )
        .1;

    assert!(
        tab.contains("session_provider(state, id).provider().command()"),
        "the pinned AI tab must label itself from the *session's* provider, by `command()` — a \
         constant or a call that drops the session names one CLI on every session's bar (FR-016a)"
    );
    assert!(
        !tab.contains("display_name()"),
        "`display_name()` is the menus-and-sentences register (\"GitHub Copilot\"); a tab in a \
         width budget takes `command()`. The two must not converge — see FR-016's clarification"
    );
}

//! One overlay implementation, and a closed list of exceptions (feature 017, T056 — FR-008, SC-003).
//!
//! SC-003 claims exactly one overlay implementation exists, down from five. The claim needs
//! stating precisely, because the literal reading is not true and the true reading is the
//! interesting one:
//!
//! - **Hand-rolled implementations: zero.** Four surfaces — the modal, the overflow menu, the
//!   context menu and the project-switcher popover — each carried their own positioning, backdrop
//!   and dismissal code. All four now sit on `cdk::overlay`. That is the five-to-one story.
//! - **Delegations to the rendering stack's own overlay system: two.** A `pick_list` (the select
//!   dropdown) and a `tooltip` implement `Widget::overlay()` themselves, so the stack positions
//!   them from the trigger's on-screen bounds. Neither was ever a hand-rolled implementation to
//!   remove, and both need to work inside a content-sized dialog, where a window-level surface has
//!   nothing to anchor against.
//!
//! So the honest shape is: one primitive for window-level surfaces, plus a **closed list** of
//! widget-attached delegations. This file closes the list.
//!
//! Why bother, when the arrangement is already correct? Because five implementations did not
//! arrive at once. They accreted one at a time, each individually reasonable, and nothing noticed
//! until the divergence was large enough to need a feature to fix. A closed list means the sixth
//! has to be argued for in a diff.
//!
//! Held the same way as `component_api_opacity`'s ratchet: this fails both when an unsanctioned
//! delegation appears **and** when a sanctioned one disappears without being struck off, so the
//! list cannot quietly drift out of step with the code it describes.

use std::fs;
use std::path::{Path, PathBuf};

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui")
}

/// Rendering-stack widgets that carry their own overlay, i.e. that implement `Widget::overlay()`
/// and float something above the rest of the interface without going through `cdk::overlay`.
const WIDGET_ATTACHED: &[&str] = &["pick_list", "combo_box", "tooltip"];

/// The closed list: `(file relative to src/, widget, why)`.
///
/// Adding an entry here is the deliberate act this test exists to force. Removing a delegation
/// without striking its entry off fails too — a stale exception reads as sanctioned precedent for
/// the next one.
const SANCTIONED: &[(&str, &str, &str)] = &[
    (
        "ui/material/select.rs",
        "pick_list",
        "the dropdown must anchor to its trigger inside a content-sized dialog, where a \
         window-level surface has nothing to anchor against — a hand-rolled version of this is \
         what revealed the list inline",
    ),
    (
        "ui/material/mod.rs",
        "tooltip",
        "a tooltip follows its trigger and has no backdrop, dismissal or stacking order to own; \
         routing it through the window-level primitive would give it three concerns it does not \
         have",
    ),
];

/// Every `.rs` file under `src/ui/`, recursively, as `(path relative to src/, source)`.
fn ui_sources() -> Vec<(String, String)> {
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
    walk(&ui_dir(), &mut out);
    out.sort();
    out
}

/// Strips `//` line comments and `/* */` blocks. Load-bearing here: this file's subject is
/// discussed at length in `select.rs`'s own module doc, which must not count as a use.
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

/// Whether `line` *calls* `widget`, as opposed to mentioning it.
///
/// Word-boundary matching is not optional: `row_tooltip(` and `mode_tooltip(` are this codebase's
/// own helpers and both end in `tooltip(`. A naive `contains` would report the sidebar and the
/// terminal as unsanctioned overlay implementations.
fn calls(line: &str, widget: &str) -> bool {
    let needle = format!("{widget}(");
    let mut from = 0;
    while let Some(at) = line[from..].find(&needle) {
        let start = from + at;
        let preceded_by_ident = start > 0
            && line[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !preceded_by_ident {
            return true;
        }
        from = start + needle.len();
    }
    false
}

/// Every construction of a widget-attached overlay, as `(file, widget)`.
fn delegations() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (path, src) in ui_sources() {
        let code = code_only(&src);
        for widget in WIDGET_ATTACHED {
            if code.lines().any(|l| calls(l, widget)) {
                found.push((path.clone(), (*widget).to_string()));
            }
        }
    }
    found
}

/// No unsanctioned widget-attached overlay.
#[test]
fn every_widget_attached_overlay_is_on_the_list() {
    let strays: Vec<_> = delegations()
        .into_iter()
        .filter(|(path, widget)| !SANCTIONED.iter().any(|(p, w, _)| p == path && w == widget))
        .map(|(path, widget)| format!("  {path} constructs `{widget}`"))
        .collect();

    assert!(
        strays.is_empty(),
        "an unsanctioned widget-attached overlay appeared:\n{}\n\nFR-008 puts every floating \
         surface on one primitive (`cdk::overlay`). A widget that floats its own content bypasses \
         its positioning, backdrop, dismissal and stacking order — which is how five \
         implementations diverged in the first place. Either build it on `cdk::overlay`, or add \
         it to SANCTIONED with the reason it cannot be.",
        strays.join("\n")
    );
}

/// …and no sanctioned entry outlives the code it describes.
///
/// The half that keeps the list honest. A stale exception is worse than no list: it reads as
/// precedent, and the next delegation gets waved through by comparison.
#[test]
fn no_sanctioned_exception_is_stale() {
    let found = delegations();
    let stale: Vec<_> = SANCTIONED
        .iter()
        .filter(|(path, widget, _)| !found.iter().any(|(p, w)| p == path && w == widget))
        .map(|(path, widget, _)| format!("  {path} no longer constructs `{widget}`"))
        .collect();

    assert!(
        stale.is_empty(),
        "a sanctioned exception no longer exists:\n{}\n\nStrike it off. An exception list that \
         outlives its exceptions grants permission nobody asked for.",
        stale.join("\n")
    );
}

/// Nobody hand-rolls `Widget::overlay()` outside the cdk.
///
/// Distinct from the check above: that one catches *using* a stack widget that floats something,
/// this one catches *writing* a new floating mechanism. The five implementations this feature
/// deleted were of the second kind.
#[test]
fn no_module_outside_the_cdk_implements_its_own_overlay() {
    let offenders: Vec<_> = ui_sources()
        .into_iter()
        .filter(|(path, _)| !path.starts_with("ui/cdk/"))
        .filter(|(_, src)| code_only(src).contains("fn overlay("))
        .map(|(path, _)| format!("  {path}"))
        .collect();

    assert!(
        offenders.is_empty(),
        "a module outside `ui/cdk/` implements its own overlay:\n{}\n\nThat is the shape of the \
         five implementations this feature consolidated (FR-008). The primitive owns positioning, \
         backdrop, dismissal and stacking order so that changing any of them is one edit.",
        offenders.join("\n")
    );
}

/// A scan that scans nothing passes trivially, and both checks above are scans.
#[test]
fn the_scan_actually_finds_the_rendering_layer() {
    let sources = ui_sources();
    assert!(
        sources.len() > 20,
        "found only {} sources under {} — the checks above would be near-vacuous",
        sources.len(),
        ui_dir().display()
    );
    assert_eq!(
        delegations().len(),
        SANCTIONED.len(),
        "expected exactly {} widget-attached delegations, found {:?}",
        SANCTIONED.len(),
        delegations()
    );
}

/// The word-boundary rule is the difference between this file working and this file libelling two
/// innocent modules, so it is tested rather than trusted.
#[test]
fn a_helper_ending_in_the_widget_name_is_not_a_use_of_it() {
    assert!(calls("tooltip(t.content, tip, t.position)", "tooltip"));
    assert!(calls("    .push(tooltip(x))", "tooltip"));
    assert!(
        !calls("item.row_tooltip(label)", "tooltip"),
        "`row_tooltip` is this crate's own builder method, not iced's tooltip widget"
    );
    assert!(
        !calls("mode_tooltip(mode)", "tooltip"),
        "`mode_tooltip` is a local helper in the terminal module"
    );
    assert!(!calls("use iced::widget::pick_list;", "pick_list"));
}

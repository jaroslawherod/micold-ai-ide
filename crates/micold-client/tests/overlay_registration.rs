//! Every popover has an answer at every dismissal path (feature 021, T026 — FR-010, contract R2).
//!
//! ## Why this guards the popovers and not the `Overlay` enum
//!
//! The contract states R2's problem as *"forgetting one of eight edit sites produces an overlay
//! that opens but will not close, discovered by hand"*. For the nine `Overlay` variants that is
//! **not true today**, and it is worth being precise about why, because the reason is the whole
//! argument for this file's shape.
//!
//! Those sites are `match` statements over a closed enum, so the compiler already enforces
//! coverage. Removing an arm was tried three ways while writing this test — from the view match,
//! from the snapshot mapping, and by renaming a site out of existence — and every one of them
//! failed to *compile*, before any test ran. A guard asserting what `rustc` already asserts is a
//! guard that can never fire.
//!
//! The seven lightweight popovers have no such protection. They are loose `bool` and `Option`
//! fields on `State`, and each dismissal path clears whichever subset its author remembered:
//!
//! - `open_overlay` clears **four** of them.
//! - `dismiss_on_scroll_beneath` clears **six** — a different six.
//!
//! Nothing checks either list. Add an eighth popover and it silently belongs to neither, which is
//! exactly the "opens but will not close, discovered by hand" failure, sitting in the half of the
//! problem the enum was never covering.
//!
//! Note what this file does **not** claim: that the current subsets are right. Whether a worktree
//! context menu should survive a modal opening over it is a behaviour question, and FR-027 puts
//! behaviour changes out of scope for this feature. What is asserted is that each combination is a
//! decision someone made and wrote down, rather than an omission nobody noticed.
//!
//! ## What happens to this file in Tier 2
//!
//! T031 moves these seven onto the registry, at which point "did this path remember this popover?"
//! stops being a question and `DismissalRules` answers it uniformly. The lists below empty out and
//! the subject becomes registration itself — the obligation is unchanged, so the file stays.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn app_rs() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// What a dismissal path does with a popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition {
    /// The path clears it.
    Cleared,
    /// The path deliberately leaves it alone, for the stated reason.
    Kept(&'static str),
}
use Disposition::{Cleared, Kept};

/// The seven lightweight popovers, and what each dismissal path does with each.
///
/// `(field, on open_overlay, on dismiss_on_scroll_beneath)`.
#[allow(clippy::type_complexity)]
const POPOVERS: &[(&str, Disposition, Disposition)] = &[
    ("help_menu_open", Cleared, Cleared),
    ("project_switcher_open", Cleared, Cleared),
    ("sidebar_filter_open", Cleared, Cleared),
    ("project_menu_open", Cleared, Cleared),
    (
        "worktree_menu_open",
        Kept(
            "not cleared when a modal opens — the paths that open a modal from this menu clear it \
              themselves (the reducer does so at four separate sites), so open_overlay never sees \
              it set. That is the fragile arrangement Tier 2 removes, not a rule worth keeping",
        ),
        Cleared,
    ),
    (
        "session_menu_open",
        Kept(
            "same shape as worktree_menu_open: cleared by the two reducer arms that open a modal \
              from it rather than centrally",
        ),
        Cleared,
    ),
    (
        "terminal_context_menu",
        Kept(
            "the only popover cleared by neither path. It is anchored inside the terminal pane \
              rather than to a window-level trigger, and is cleared by its own reducer arm",
        ),
        Kept(
            "scroll dismissal is about the surface the pointer scrolled over; this menu's own \
              scroll handling lives with the pane",
        ),
    ),
];

/// The body of a function in `app.rs`, from its signature to the first line that closes it at
/// four-space indentation.
fn body_of(fn_signature: &str) -> String {
    let src = app_rs();
    let at = src.find(fn_signature).unwrap_or_else(|| {
        panic!(
            "`{fn_signature}` is gone from app.rs. If Tier 2 replaced it with generic dispatch, \
             this file's subject moved with it: rewrite the lists here against the registry rather \
             than deleting the check, because R2 still has to hold."
        )
    });
    let rest = &src[at..];
    let end = rest.find("\n    }\n").map(|e| e + 6).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// Popover-shaped fields actually declared on `State`, so the list above cannot go stale.
fn declared_popovers() -> BTreeSet<String> {
    let src = app_rs();
    let at = src.find("pub struct State {").expect("State has moved");
    let rest = &src[at..];
    let end = rest.find("\n}").expect("unterminated struct");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_prefix("pub ")?.split(':').next()?;
            (name.ends_with("_open") || name.contains("_menu")).then(|| name.to_string())
        })
        .collect()
}

#[test]
fn the_list_of_popovers_matches_the_ones_that_exist() {
    let declared = declared_popovers();
    let listed: BTreeSet<String> = POPOVERS.iter().map(|(f, ..)| f.to_string()).collect();

    let unlisted: Vec<_> = declared.difference(&listed).collect();
    let phantom: Vec<_> = listed.difference(&declared).collect();

    assert!(
        unlisted.is_empty(),
        "a new popover exists that no dismissal path has been asked about: {unlisted:?}\n\nAdd it \
         to POPOVERS with what each path should do. That is the whole point — a popover nobody \
         decided about is one that opens and will not close (FR-010)."
    );
    assert!(
        phantom.is_empty(),
        "POPOVERS names fields State no longer has: {phantom:?}\n\nIf Tier 2 migrated them onto \
         the registry, strike them off here in the same commit."
    );
}

#[test]
fn opening_a_modal_makes_the_same_decision_about_every_popover_as_last_time() {
    let body = body_of("pub fn open_overlay(&mut self, overlay: Overlay)");
    let mut wrong = Vec::new();

    for (field, on_open, _) in POPOVERS {
        let clears = body.contains(field);
        match (on_open, clears) {
            (Cleared, false) => wrong.push(format!(
                "{field} is listed as cleared when a modal opens, but open_overlay no longer \
                 touches it — a popover left floating over a scrim"
            )),
            (Kept(why), true) => wrong.push(format!(
                "{field} is now cleared by open_overlay, but is listed as kept because: {why}\n     \
                 If that changed on purpose, update the list; the behaviour and its reason have to \
                 travel together"
            )),
            _ => {}
        }
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n  - "));
}

#[test]
fn scrolling_beneath_makes_the_same_decision_about_every_popover_as_last_time() {
    let body = body_of("fn dismiss_on_scroll_beneath(&mut self)");
    let mut wrong = Vec::new();

    for (field, _, on_scroll) in POPOVERS {
        let clears = body.contains(field);
        match (on_scroll, clears) {
            (Cleared, false) => wrong.push(format!(
                "{field} is listed as cleared on a scroll beneath, but the path no longer \
                 touches it"
            )),
            (Kept(why), true) => wrong.push(format!(
                "{field} is now cleared on a scroll beneath, but is listed as kept because: {why}"
            )),
            _ => {}
        }
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n  - "));
}

#[test]
fn the_two_dismissal_paths_disagree_and_that_is_recorded_rather_than_accidental() {
    let cleared_on_open = POPOVERS.iter().filter(|(_, o, _)| *o == Cleared).count();
    let cleared_on_scroll = POPOVERS.iter().filter(|(.., s)| *s == Cleared).count();

    assert_ne!(
        cleared_on_open, cleared_on_scroll,
        "the two paths used to clear different subsets (4 and 6 of 7), which is the duplication \
         Tier 2 removes. If they agree now, either the registry landed — in which case this file's \
         subject is registration, not these lists — or someone unified them by hand, which is a \
         behaviour change FR-027 does not authorise"
    );
    assert_eq!(
        (cleared_on_open, cleared_on_scroll),
        (4, 6),
        "the split moved. Every combination in POPOVERS is meant to be a recorded decision, so a \
         change to the totals means a decision changed without its reason being updated"
    );
}

#[test]
fn the_guard_is_actually_looking_at_something() {
    assert!(
        !declared_popovers().is_empty(),
        "no popover-shaped fields found on State — the parser has stopped matching its shape, and \
         a guard iterating an empty list passes vacuously"
    );
    assert!(
        body_of("pub fn open_overlay(&mut self, overlay: Overlay)").len() > 40,
        "open_overlay's body came back empty, so every 'does it clear this?' check would answer no"
    );
}

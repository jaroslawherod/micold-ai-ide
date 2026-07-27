//! Unified floating-surface dismissal (feature 017, T005 — FR-009, FR-017).
//!
//! The application has five independent overlay implementations, and their dismissal behavior has
//! drifted apart. Feature 017 consolidates them onto one primitive; unifying dismissal is its
//! single sanctioned behavior change (FR-024).
//!
//! Deciding *whether* a trigger dismisses a surface is branching logic, so per Constitution
//! Principle I it lives here — pure, in the render-free core — rather than inside a widget where
//! it would be unreachable from tests. The widget owns *presentation*; this owns the *decision*.

use micold_core::overlay::{dismisses, Surface, Trigger};

/// Non-modal surfaces — menus, context menus, popovers, the select dropdown — are transient. Any
/// of the three triggers closes them.
#[test]
fn a_non_modal_surface_dismisses_on_every_trigger() {
    for trigger in Trigger::ALL {
        assert!(
            dismisses(Surface::NonModal, *trigger),
            "a non-modal surface must dismiss on {trigger:?}"
        );
    }
}

/// A dialog is deliberately harder to dismiss: it holds the user's attention on purpose, so
/// scrolling underneath it must not close it.
#[test]
fn a_dialog_dismisses_on_escape_and_scrim_click_but_not_on_scroll() {
    assert!(dismisses(Surface::Dialog, Trigger::Escape));
    assert!(dismisses(Surface::Dialog, Trigger::OutsideClick));
    assert!(
        !dismisses(Surface::Dialog, Trigger::ScrollBeneath),
        "scrolling behind a dialog must not dismiss it — the dialog is the focus, not the content"
    );
}

/// Reserved for dialogs where losing input would destroy work. Nothing dismisses these implicitly;
/// the user must take an explicit action inside them.
#[test]
fn a_non_dismissible_dialog_ignores_every_trigger() {
    for trigger in Trigger::ALL {
        assert!(
            !dismisses(Surface::NonDismissibleDialog, *trigger),
            "a non-dismissible dialog must ignore {trigger:?} — losing its input would destroy work"
        );
    }
}

/// The rule must be **total**. An undefined combination is how five implementations drifted apart
/// in the first place: each one answered the gaps differently.
#[test]
fn the_rule_is_total_across_every_surface_and_trigger() {
    let mut combinations = 0;
    for surface in Surface::ALL {
        for trigger in Trigger::ALL {
            // Calling it at all proves totality: a panic or a missing arm fails here.
            let _ = dismisses(*surface, *trigger);
            combinations += 1;
        }
    }
    assert_eq!(
        combinations,
        Surface::ALL.len() * Trigger::ALL.len(),
        "every surface/trigger combination must be defined"
    );
    assert!(
        combinations >= 9,
        "expected at least 3 surfaces x 3 triggers"
    );
}

/// The behavior change this feature sanctions is *unification*, which is only meaningful if the
/// surfaces genuinely differ from one another. If every surface behaved identically there would be
/// nothing to consolidate and no reason to accept a behavior change.
#[test]
fn the_surface_kinds_are_actually_distinguishable() {
    assert_ne!(
        dismisses(Surface::NonModal, Trigger::ScrollBeneath),
        dismisses(Surface::Dialog, Trigger::ScrollBeneath),
        "non-modal and dialog must differ on scroll, or the distinction is pointless"
    );
    assert_ne!(
        dismisses(Surface::Dialog, Trigger::Escape),
        dismisses(Surface::NonDismissibleDialog, Trigger::Escape),
        "the non-dismissible variant must actually resist Escape"
    );
}

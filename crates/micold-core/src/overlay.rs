//! Floating-surface dismissal rules (feature 017, FR-009, FR-017).
//!
//! The application grew five independent overlay implementations — the modal, the overflow menu,
//! the context menu, the project-switcher popover and the select dropdown. Each answered "when
//! does this close?" for itself, so the answers drifted apart. Feature 017 consolidates them onto
//! one primitive, and unifying dismissal is its single sanctioned behavior change.
//!
//! The *decision* lives here rather than in the widget because it is branching logic, which
//! Constitution Principle I requires to be testable and which the GUI-wiring exception explicitly
//! does not cover. The widget owns positioning, backdrop and presentation; this owns the rule.
//!
//! Render-free by construction: this crate declares no rendering dependency, so nothing here can
//! come to depend on one.

/// What kind of floating surface is open.
///
/// The distinction is about *intent*, not appearance: a dialog deliberately holds the user's
/// attention, a non-modal surface is transient, and a non-dismissible dialog is protecting work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Menus, context menus, popovers and the select dropdown. Transient: the user is glancing at
    /// it, and almost any other interaction means they are done with it.
    NonModal,
    /// A modal dialog. Dismissible, but only by acting on the dialog itself or its scrim —
    /// scrolling the content behind it is not a decision to close it.
    Dialog,
    /// A dialog holding input that would be destroyed by an accidental dismissal. Reserved, and
    /// deliberately rare: the user must take an explicit action inside it.
    NonDismissibleDialog,
}

impl Surface {
    /// Every surface kind, so callers (and the totality test) can enumerate exhaustively.
    pub const ALL: &'static [Surface] = &[
        Surface::NonModal,
        Surface::Dialog,
        Surface::NonDismissibleDialog,
    ];
}

/// Something the user did that *might* close the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// A press outside the surface's own bounds — on the scrim for a dialog, anywhere else for a
    /// non-modal surface.
    OutsideClick,
    /// The Escape key.
    Escape,
    /// The content beneath the surface scrolled. A menu anchored to a row is meaningless once that
    /// row has moved; a dialog is not anchored to anything, so this does not apply to it.
    ScrollBeneath,
}

impl Trigger {
    /// Every trigger, so the rule can be proven total.
    pub const ALL: &'static [Trigger] = &[
        Trigger::OutsideClick,
        Trigger::Escape,
        Trigger::ScrollBeneath,
    ];
}

/// Whether `trigger` dismisses `surface`.
///
/// Total by construction — every combination is answered, which is precisely what the five
/// hand-rolled implementations failed to do consistently.
pub fn dismisses(surface: Surface, trigger: Trigger) -> bool {
    match surface {
        // Transient: any of the three means the user has moved on.
        Surface::NonModal => true,
        // Anchored to nothing, so scrolling behind it is not a dismissal; the other two are
        // deliberate acts aimed at the dialog.
        Surface::Dialog => matches!(trigger, Trigger::OutsideClick | Trigger::Escape),
        // Protecting unsaved work: nothing implicit closes it.
        Surface::NonDismissibleDialog => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the one asymmetry in the rule, so a later "simplification" that collapses `Dialog`
    /// into `NonModal` fails here rather than silently changing how dialogs behave.
    #[test]
    fn scroll_separates_a_dialog_from_a_non_modal_surface() {
        assert!(dismisses(Surface::NonModal, Trigger::ScrollBeneath));
        assert!(!dismisses(Surface::Dialog, Trigger::ScrollBeneath));
    }

    #[test]
    fn nothing_dismisses_a_protected_dialog() {
        assert!(Trigger::ALL
            .iter()
            .all(|t| !dismisses(Surface::NonDismissibleDialog, *t)));
    }
}

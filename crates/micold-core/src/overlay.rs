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

/// Which band of the z-order a floating surface belongs to (FR-010).
///
/// Stacking used to be whatever fell out of composition order: each surface wrapped the previous
/// one, so "which is on top" was a property of the order the view function happened to build them
/// in. Declaring the band explicitly makes the order a property of *what the surface is*, so
/// reordering the composition cannot reorder the screen.
///
/// Variants are declared bottom-to-top and the derived `Ord` is that order — adding a variant in
/// the middle moves it in the z-order, which is the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// A panel attached to a trigger that stays put: the overflow menu, the project switcher.
    Popover,
    /// A panel anchored at the cursor. Opens *over* a popover, because right-clicking a row in an
    /// open switcher must not put the row's own menu behind the panel it came from.
    ContextMenu,
    /// A modal dialog. Always above every non-modal surface — a dialog beneath a menu would be
    /// unreachable, since the dialog captures input the menu is then drawn on top of.
    Dialog,
    /// A snackbar. Above everything, including a dialog and its scrim (design-tokens §7.8).
    ///
    /// Above a *dialog* specifically, which is unusual and deliberate: a notification raised while
    /// a dialog is open is usually **about** what the dialog just tried to do, so putting it behind
    /// the scrim would hide the answer to the question the user is looking at. It is bottom-aligned
    /// and times out, so it does not permanently obstruct the dialog's action row.
    Snackbar,
}

impl Layer {
    /// Every band, bottom-to-top, so callers (and the totality test) can enumerate exhaustively.
    pub const ALL: &'static [Layer] = &[
        Layer::Popover,
        Layer::ContextMenu,
        Layer::Dialog,
        Layer::Snackbar,
    ];

    /// The surface kind this band implies, so a caller cannot pair a dialog band with non-modal
    /// dismissal by accident. `Dialog` is the dismissible variant; a caller that needs
    /// [`Surface::NonDismissibleDialog`] states so explicitly.
    pub fn surface(self) -> Surface {
        match self {
            Layer::Popover | Layer::ContextMenu => Surface::NonModal,
            Layer::Dialog => Surface::Dialog,
            // A snackbar captures nothing and is not dismissed by the triggers a popover is: the
            // ground moving under it, or Escape, says nothing about whether the user has read it.
            // It clears on its own timeout or on its own button, which is why it is non-modal
            // *and* why the dismissal rule below never sees it — nothing pushes it as dismissible.
            Layer::Snackbar => Surface::NonModal,
        }
    }
}

/// Orders surfaces bottom-to-top by band, keeping the caller's order **within** a band.
///
/// Stable on purpose: two surfaces in the same band (two context menus, say) have no intrinsic
/// order to appeal to, so preserving the order they were registered in is the only answer that is
/// both deterministic and not arbitrary. Across bands the result is independent of registration
/// order, which is the invariant FR-010 asks for.
pub fn stack_order<T: Copy>(surfaces: &[(Layer, T)]) -> Vec<(Layer, T)> {
    let mut ordered = surfaces.to_vec();
    ordered.sort_by_key(|(layer, _)| *layer);
    ordered
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

    /// The point of the band: the same two surfaces come out the same way round however they were
    /// registered. This is what composition order used to decide.
    #[test]
    fn a_dialog_outranks_a_menu_whichever_was_registered_first() {
        let menu_first = stack_order(&[(Layer::Popover, "menu"), (Layer::Dialog, "dialog")]);
        let dialog_first = stack_order(&[(Layer::Dialog, "dialog"), (Layer::Popover, "menu")]);
        assert_eq!(menu_first, dialog_first);
        assert_eq!(menu_first.last().unwrap().1, "dialog");
    }
}

//! Deterministic overlay stacking (feature 017, T011 — FR-010, SC-003).
//!
//! Before this feature, "which floating surface is on top" was decided by the order `ui::view`
//! happened to wrap them in: each overlay took the previous element as its `base`, so the last one
//! composed won. That made the z-order an accident of how the view function was written, and it is
//! why the five implementations could disagree about it without anyone noticing.
//!
//! The single overlay primitive owns the order instead. These tests pin the property that matters:
//! given two open surfaces, the result is the same whichever order they were registered in.

use iced::Element;
use micold_client::ui::cdk::overlay::{Anchor, Overlay, Surface};
use micold_core::overlay::Layer;

/// The app's message type is irrelevant to stacking; a unit message keeps these tests about order.
type Msg = ();

fn panel<'a>() -> Element<'a, Msg> {
    iced::widget::Space::new().into()
}

fn surface<'a>(layer: Layer) -> Surface<'a, Msg> {
    Surface::new(layer, panel(), Anchor::Center)
}

/// The headline invariant. A dialog is above a menu because it is a dialog, not because the view
/// function wrapped it second.
#[test]
fn two_surfaces_stack_the_same_way_whichever_order_they_were_added() {
    let menu_first = Overlay::new(panel())
        .push(surface(Layer::Popover))
        .push(surface(Layer::Dialog))
        .stacking();
    let dialog_first = Overlay::new(panel())
        .push(surface(Layer::Dialog))
        .push(surface(Layer::Popover))
        .stacking();

    assert_eq!(
        menu_first, dialog_first,
        "stacking order must not depend on registration order"
    );
    assert_eq!(
        menu_first.last(),
        Some(&Layer::Dialog),
        "the dialog must end up on top"
    );
}

/// The same, exhaustively: every ordered pair of distinct bands, both ways round.
#[test]
fn every_pair_of_bands_is_registration_order_independent() {
    for &a in Layer::ALL {
        for &b in Layer::ALL {
            if a == b {
                continue;
            }
            let forward = Overlay::new(panel())
                .push(surface(a))
                .push(surface(b))
                .stacking();
            let backward = Overlay::new(panel())
                .push(surface(b))
                .push(surface(a))
                .stacking();
            assert_eq!(forward, backward, "order differed for {a:?} vs {b:?}");
            assert_eq!(
                forward,
                vec![a.min(b), a.max(b)],
                "bands must come out bottom-to-top for {a:?} vs {b:?}"
            );
        }
    }
}

/// A context menu opened from a row inside the project switcher must draw over the switcher, not
/// behind it — the concrete case that motivated giving context menus their own band.
#[test]
fn a_context_menu_draws_over_the_popover_it_was_opened_from() {
    let stacking = Overlay::new(panel())
        .push(surface(Layer::ContextMenu))
        .push(surface(Layer::Popover))
        .stacking();
    assert_eq!(stacking, vec![Layer::Popover, Layer::ContextMenu]);
}

/// Two surfaces in the same band have no intrinsic order, so registration order is the answer —
/// deterministic, and the only one that isn't arbitrary.
#[test]
fn surfaces_in_the_same_band_keep_the_order_they_were_added() {
    let stacking = Overlay::new(panel())
        .push(surface(Layer::ContextMenu))
        .push(surface(Layer::ContextMenu))
        .push(surface(Layer::Popover))
        .stacking();
    assert_eq!(
        stacking,
        vec![Layer::Popover, Layer::ContextMenu, Layer::ContextMenu]
    );
}

/// With nothing open the overlay is inert: it must contribute no layer at all, so a closed overlay
/// cannot cost a stack frame or capture input.
#[test]
fn an_overlay_with_no_surfaces_stacks_nothing() {
    assert!(Overlay::new(panel()).stacking().is_empty());
}

//! The registry answers what the enum answers, for every state it can be in (feature 021, T029 —
//! contract R1, R3; FR-008, FR-012).
//!
//! T029's whole claim is that two representations of "what is floating" now coexist and agree.
//! Agreement is the thing worth testing, because the four commits after this one delete the older
//! representation a site at a time: each of those is safe exactly insofar as the newer one already
//! gives the same answer. So the central test here is an exhaustive equivalence — every `Overlay`
//! variant crossed with the filter panel open and closed, twenty states, registry against
//! `on_escape`. If the two ever diverge, they diverge here rather than in a dialog that will not
//! close.
//!
//! The equivalence covers the states `Overlay` can express, crossed with the one popover Escape
//! reached before T031. T031 registered the other six, and that *did* change what Escape does —
//! recorded below in `escape_now_reaches_every_popover`, not hidden inside the equivalence.

use micold_client::app::{on_escape, Message, Overlay, State};
use micold_client::overlay::registry::{self, Probe};
use micold_core::overlay::{Layer, Trigger};

/// Every `Overlay` variant, `None` included.
///
/// Written out rather than derived: this list going stale is itself a finding, caught by
/// `every_variant_is_in_the_list` below.
const OVERLAYS: &[Overlay] = &[
    Overlay::None,
    Overlay::About,
    Overlay::ProjectSelector,
    Overlay::RenameProject,
    Overlay::AddWorktree,
    Overlay::Settings,
    Overlay::ConfirmWorktreeDelete,
    Overlay::RenameWorktree,
    Overlay::ConfirmSessionRemove,
    Overlay::ConfirmForgetProject,
];

fn state(overlay: Overlay, filter_open: bool) -> State {
    State {
        overlay,
        sidebar_filter_open: filter_open,
        ..Default::default()
    }
}

/// The twenty states the two representations must agree on.
fn every_state() -> impl Iterator<Item = (Overlay, bool, State)> {
    OVERLAYS.iter().flat_map(|overlay| {
        [false, true]
            .into_iter()
            .map(move |filter| (*overlay, filter, state(*overlay, filter)))
    })
}

#[test]
fn the_registry_and_the_enum_answer_escape_identically_in_every_state() {
    for (overlay, filter, state) in every_state() {
        assert_eq!(
            registry::escape(&state),
            on_escape(&state),
            "{overlay:?} with the filter panel {}: generic dispatch disagreed with the enum it is \
             replacing. The next four tasks delete a match site each on the strength of this \
             equality, so a divergence here is a dialog that will not close after one of them",
            if filter { "open" } else { "closed" }
        );
    }
}

#[test]
fn the_registry_and_the_enum_agree_on_which_surface_that_is() {
    // Not just *what closes it* but *which surface it is*. T035 keys the view and the exit
    // animation on identity, so an id typo'd in a feature module would move a dialog's transition
    // rather than break its dismissal — a failure the equivalence above cannot see.
    for overlay in OVERLAYS {
        let state = state(*overlay, false);
        let from_registry = registry::topmost(&state).map(|open| open.id());
        let from_enum = overlay.as_surface().map(|(id, _)| id);

        assert_eq!(
            from_registry, from_enum,
            "{overlay:?}: the registry and the enum name different surfaces"
        );
    }
}

#[test]
fn every_variant_is_in_the_list() {
    // `as_surface` is the enum's own account of itself; a variant added without one is a surface
    // the registry cannot see. The compiler catches the missing arm, and this catches the case
    // where the arm exists but this file's list does not.
    let named = OVERLAYS.len();
    let with_a_surface = OVERLAYS.iter().filter(|o| o.as_surface().is_some()).count();

    assert_eq!(
        named, 10,
        "OVERLAYS has drifted from the enum. Add the new variant here and to the equivalence \
         above, or the twenty states it is meant to prove are no longer twenty"
    );
    assert_eq!(
        with_a_surface, 9,
        "exactly one variant — `None` — names no surface. A second such variant is an overlay \
         that opens and cannot be dismissed"
    );
}

#[test]
fn a_modal_keeps_escape_whatever_floats_above_it() {
    // Contract D1, stated at the registry rather than at `on_escape`: the band decides, and
    // `Dialog` outranks `Popover`. `overlay_dispatch_ordering.rs` holds the same obligation
    // against the public entry points; this holds it against the mechanism that will implement it.
    let both = state(Overlay::About, true);

    let top = registry::topmost(&both).expect("a modal and a popover are open");
    assert_eq!(top.layer(), Layer::Dialog);
    assert_eq!(registry::escape(&both), Some(Message::AboutClosed));

    let popover_alone = state(Overlay::None, true);
    assert_eq!(
        registry::escape(&popover_alone),
        Some(Message::SidebarFilterMenuToggled),
        "with no modal the popover is the topmost surface, and Escape is its own"
    );
}

#[test]
fn a_scroll_beneath_reaches_every_menu_and_no_dialog() {
    // The other dispatch shape. Escape is exclusive; this is not, and that difference is the
    // behaviour `State::dismiss_on_scroll_beneath` has today — it clears the popovers whether or
    // not a modal is over them.
    assert_eq!(
        registry::scroll_beneath(&state(Overlay::About, true)),
        vec![Message::SidebarFilterMenuToggled],
        "a scroll behind an open modal still invalidates the menu anchored beneath it, and does \
         not touch the modal"
    );
    assert!(
        registry::scroll_beneath(&state(Overlay::About, false)).is_empty(),
        "a dialog is anchored to nothing, so scrolling the content behind it closes nothing"
    );
}

#[test]
fn registration_order_does_not_decide_anything() {
    // Contract R3. Testable only by reordering, which is why `probes()` is public.
    let forward: Vec<Probe> = registry::probes().to_vec();
    let reversed: Vec<Probe> = forward.iter().rev().copied().collect();

    for (overlay, filter, state) in every_state() {
        let a = registry::topmost_among(&forward, &state);
        let b = registry::topmost_among(&reversed, &state);
        assert_eq!(
            a, b,
            "{overlay:?} + filter {filter}: reversing the registration list changed which surface \
             is on top. Stacking must be a property of the band, not of the order someone happened \
             to write the register! lines in"
        );
    }
}

#[test]
fn a_surface_is_registered_by_naming_it_once_and_nothing_else() {
    // R1/SC-001 in miniature: the sidebar filter panel is described entirely in
    // `features/sidebar.rs` and appears in exactly one line of `overlay/registry.rs`. Dispatch
    // finds it without any central match having heard of it.
    let open = registry::topmost(&state(Overlay::None, true)).expect("the panel is open");

    assert_eq!(open.id().as_str(), "sidebar_filter");
    assert_eq!(open.layer(), Layer::Popover);
    assert_eq!(
        open.on(Trigger::Escape),
        Some(&Message::SidebarFilterMenuToggled)
    );
}

#[test]
fn escape_now_reaches_every_popover() {
    // **A behaviour change, and the only one in T031's dispatch.** Escape did not close the
    // overflow menu, the switcher, or the three context menus: no widget handles Escape — the
    // `cdk::overlay::Surface` observes an outside click and nothing else — and the keyboard path
    // only ever asked about a modal or the filter panel. The *rule* has said since feature 017
    // that a non-modal surface dismisses on Escape, and `Surface::dismisses_on` exists precisely
    // so "callers that own such a trigger consult the same rule rather than re-deciding it". The
    // subscription was never wired to it. Registering the popovers finishes that wiring.
    //
    // FR-012 preserves the *priority* between simultaneously-open surfaces and the rule that a
    // modal closes popovers; both still hold, and are asserted above. It does not require that a
    // surface Escape never reached keeps not being reached.
    let mut state = State {
        help_menu_open: true,
        ..Default::default()
    };
    assert_eq!(
        registry::escape(&state),
        Some(Message::HelpMenuToggled),
        "Escape closes the overflow menu, which before T031 it left open"
    );

    // And the priority is unchanged: a modal over it still takes Escape for itself.
    state.overlay = Overlay::About;
    assert_eq!(
        registry::escape(&state),
        Some(Message::AboutClosed),
        "a dialog outranks a menu, whichever was opened first (contract D1)"
    );
}

#[test]
fn the_registry_is_actually_looking_at_something() {
    assert!(
        registry::probes().len() >= 2,
        "fewer than two registrations means the band comparison and the ordering test above are \
         both trivially satisfied, and would pass with the mechanism broken"
    );
    assert!(
        registry::topmost(&State::default()).is_none(),
        "the default state has nothing open; a registry that reports a surface there is matching \
         on something other than what it was asked"
    );
}

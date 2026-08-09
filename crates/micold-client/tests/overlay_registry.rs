//! Generic dispatch reaches every dialog, under the right name, with the right cancellation
//! (feature 021, T029/T033/T034 — contract R1, D1, R3; FR-008, FR-012).
//!
//! ## What this file checked before T033, and what it checks now
//!
//! T029's claim was that two representations of "what is floating" coexisted and agreed, so the
//! central test was an exhaustive *equivalence*: every `Overlay` variant crossed with the filter
//! panel open and closed, twenty states, registry against `on_escape`. The four commits that
//! delete the older representation a site at a time are each safe exactly insofar as the newer one
//! already gives the same answer.
//!
//! T033 deleted the older representation's own account of itself — `Overlay::as_surface` — so
//! there is no longer a second answer to compare against. The obligation does not go with it: the
//! nine facts still have to be right, and T034–T036 still delete a site each on the strength of
//! them. So they are stated **here**, in [`expected`], an exhaustive match written independently
//! of the code under test. That is strictly stronger than the equality it replaces, which could
//! only ever catch the two sides *disagreeing*, never both being wrong; and it is exhaustive, so a
//! tenth variant added without an expectation fails to compile rather than going unchecked.
//!
//! The states covered are the ones `Overlay` can express, crossed with the one popover Escape
//! reached before T031. T031 registered the other six, and that *did* change what Escape does —
//! recorded below in `escape_now_reaches_every_popover`, not hidden inside the table.
//!
//! ## The keyboard path (T034)
//!
//! Up to T034 all of that was about what dispatch *would* answer; the live Escape key still went
//! through a nine-arm match in `ui::subscription`. It now emits `Message::EscapePressed` and the
//! reducer asks the registry. A `Subscription` is opaque and cannot be asserted against, so the
//! wiring is covered from both ends instead: `pressing_escape_closes_the_topmost_surface` drives
//! the message the subscription emits, and `the_keyboard_subscription_names_no_surface` reads the
//! function to confirm it still emits only that one.

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

/// What each variant must dispatch as: the surface's name, and the message that cancels it.
///
/// The test's own statement of the nine facts, deliberately *not* read out of the production code
/// it checks. Exhaustive on purpose — a variant added without an expectation is a dialog nobody
/// has said how to close, and this file must fail to build rather than quietly skip it. That is
/// the same compile-time hold T026 verified for the enum's other match sites, kept alive here now
/// that the enum has none of its own.
fn expected(overlay: Overlay) -> Option<(&'static str, Message)> {
    Some(match overlay {
        Overlay::None => return None,
        Overlay::About => ("about", Message::AboutClosed),
        Overlay::ProjectSelector => ("project_selector", Message::ProjectSelectorClosed),
        Overlay::RenameProject => ("rename_project", Message::RenameCancelled),
        Overlay::AddWorktree => ("add_worktree", Message::AddWorktreeCancelled),
        Overlay::Settings => ("settings", Message::SettingsCancelled),
        Overlay::ConfirmWorktreeDelete => {
            ("confirm_worktree_delete", Message::WorktreeDeleteCancelled)
        }
        Overlay::RenameWorktree => ("rename_worktree", Message::WorktreeRenameCancelled),
        Overlay::ConfirmSessionRemove => {
            ("confirm_session_remove", Message::SessionRemoveCancelled)
        }
        Overlay::ConfirmForgetProject => {
            ("confirm_forget_project", Message::ProjectForgetCancelled)
        }
    })
}

fn state(overlay: Overlay, filter_open: bool) -> State {
    State {
        overlay,
        sidebar_filter_open: filter_open,
        ..Default::default()
    }
}

/// The twenty states dispatch must get right.
fn every_state() -> impl Iterator<Item = (Overlay, bool, State)> {
    OVERLAYS.iter().flat_map(|overlay| {
        [false, true]
            .into_iter()
            .map(move |filter| (*overlay, filter, state(*overlay, filter)))
    })
}

#[test]
fn escape_closes_the_open_dialog_in_every_state() {
    // Both entry points, against the table rather than against each other. `on_escape` delegates
    // to the registry as of T033, so comparing the two would now be vacuous — this is the check
    // that outlives the collapse, and it is the one T034–T036 delete a site each on the strength
    // of.
    for (overlay, filter, state) in every_state() {
        let cancel = expected(overlay).map(|(_, cancel)| cancel);
        let panel = filter.then_some(Message::SidebarFilterMenuToggled);
        // A dialog outranks the panel; with no dialog open the panel is the topmost surface.
        let want = cancel.or(panel);

        assert_eq!(
            registry::escape(&state),
            want,
            "{overlay:?} with the filter panel {}: generic dispatch did not produce the \
             cancellation this dialog declares",
            if filter { "open" } else { "closed" }
        );
        assert_eq!(
            on_escape(&state),
            want,
            "{overlay:?} with the filter panel {}: the public Escape entry point disagreed with \
             the registry it now delegates to",
            if filter { "open" } else { "closed" }
        );
    }
}

#[test]
fn each_dialog_registers_under_its_own_identity() {
    // Not just *what closes it* but *which surface it is*. T035 keys the view and the exit
    // animation on identity, so an id typo'd in a feature module would move a dialog's transition
    // rather than break its dismissal — a failure the cancellations above cannot see.
    for overlay in OVERLAYS {
        let state = state(*overlay, false);
        let registered = registry::topmost(&state).map(|open| open.id());
        let want = expected(*overlay).map(|(id, _)| id);

        assert_eq!(
            registered.map(|id| id.as_str()),
            want,
            "{overlay:?}: the registry names a different surface than this dialog is supposed to be"
        );
    }
}

#[test]
fn every_variant_is_in_the_list() {
    // `expected` is exhaustive, so the compiler catches a variant added without an expectation.
    // This catches the other half: the arm exists but this file's iteration list does not mention
    // it, so the twenty states are quietly nineteen.
    let named = OVERLAYS.len();
    let with_a_surface = OVERLAYS.iter().filter(|o| expected(**o).is_some()).count();

    assert_eq!(
        named, 10,
        "OVERLAYS has drifted from the enum. Add the new variant here as well as to `expected`, \
         or the twenty states this file is meant to cover are no longer twenty"
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
fn pressing_escape_closes_the_topmost_surface() {
    // T034: the keyboard path reports *that Escape happened* and the reducer decides. This drives
    // the message the subscription now emits, so it covers the wiring the subscription itself
    // cannot be asked about — `Subscription` is opaque, and the old nine-arm match in it was
    // testable only by reading it.
    fn open(state: &State) -> Vec<&'static str> {
        registry::open_among(registry::probes(), state)
            .iter()
            .map(|s| s.id().as_str())
            .collect()
    }

    for (overlay, filter, mut state) in every_state() {
        let before = open(&state);
        let top = registry::topmost(&state).map(|s| s.id().as_str());
        state.update(Message::EscapePressed);

        // Exactly the topmost surface closed, and nothing else moved either way.
        let want: Vec<&str> = before
            .iter()
            .copied()
            .filter(|id| Some(*id) != top)
            .collect();
        assert_eq!(
            open(&state),
            want,
            "{overlay:?} with the filter panel {}: Escape was supposed to close {top:?} and \
             leave the rest alone",
            if filter { "open" } else { "closed" }
        );
    }

    // A dialog over a popover: Escape takes the dialog and leaves the popover (contract D1), so
    // the two cases above are genuinely different and the loop's allowance is not a hole.
    let mut both = state(Overlay::About, true);
    both.update(Message::EscapePressed);
    assert_eq!(both.overlay, Overlay::None, "the dialog took the Escape");
    assert!(
        both.sidebar_filter_open,
        "and the popover beneath it is untouched — one Escape closes one surface"
    );

    // A popover alone, including one the pre-T031 keyboard path never reached.
    let mut menu = State {
        help_menu_open: true,
        ..Default::default()
    };
    menu.update(Message::EscapePressed);
    assert!(
        !menu.help_menu_open,
        "Escape now reaches the overflow menu, which the subscription's match never named"
    );
}

#[test]
fn pressing_escape_with_nothing_open_changes_nothing() {
    let mut state = State::default();
    let before = state.clone();
    state.update(Message::EscapePressed);
    assert_eq!(
        state, before,
        "the trigger is reported unconditionally; deciding that nothing closes is the reducer's \
         job, and it must be a no-op rather than a state change"
    );
}

#[test]
fn the_keyboard_subscription_names_no_surface() {
    // The collapse itself, held open. The subscription's job is now "hold a listener while Escape
    // has something to close"; the moment a per-surface message or an overlay variant reappears
    // in it, the second copy of the dismissal table is back and the two paths can drift again —
    // which is the failure that produced `on_escape`'s "mirrors the subscription exactly" comment
    // and the hand-written priority guard above the old match.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/mod.rs");
    let src =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let at = src
        .find("pub fn subscription(")
        .expect("`ui::subscription` has moved; point this guard at it");
    let rest = &src[at..];
    let end = rest.find("\n}").expect("unterminated function");
    let body = &rest[..end];

    assert!(
        body.contains("Message::EscapePressed"),
        "the guard is looking at the wrong text: the subscription's body should emit \
         `Message::EscapePressed`"
    );
    assert!(
        !body.contains("Overlay::"),
        "`ui::subscription` matches on an overlay variant again:\n{body}"
    );
    let named: Vec<&str> = body
        .match_indices("Message::")
        .map(|(at, _)| body[at..].split_whitespace().next().unwrap_or_default())
        .filter(|m| !m.starts_with("Message::EscapePressed"))
        .collect();
    assert!(
        named.is_empty(),
        "`ui::subscription` names per-surface messages again: {named:?} — it is supposed to \
         report that Escape happened and let the registry decide what that closes"
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

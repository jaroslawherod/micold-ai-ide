//! Dismissal ordering, as a rule rather than a coincidence (feature 021, T027 — contract D1–D3).
//!
//! Three obligations from `contracts/overlay-registry.md`. All three hold today, and all three are
//! held by code that Tier 2 deletes: D1 is a hand-written `if` placed ahead of the `Overlay` match
//! in `on_escape`, and D2 is a body of four field assignments in `open_overlay`. When the generic
//! dispatch replaces them, nothing in the existing suite would notice these rules quietly changing
//! shape — `overlay_dismissal_delta.rs` pins which *message* Escape produces per overlay, not what
//! happens when two surfaces are open at once.
//!
//! So this file exists to be the thing that notices. It is written before the registry rather than
//! after, and deliberately drives only the public entry points — `on_escape`, `State::update`,
//! `State::open_overlay` — so that the migration must leave it **passing unmodified**. A test that
//! named the current `Overlay` match internals would have to be rewritten alongside the code it is
//! supposed to be checking, which is no check at all.
//!
//! Read a failure here as: the generic dispatch does not preserve an ordering rule the special-case
//! code got right.

use micold_client::app::{on_escape, Message, Overlay, State};

/// Every modal surface, paired with the message its cancellation produces.
///
/// Not `Overlay::None` — that is the absence of a surface, and Tier 2 stops representing it as one.
const MODALS: &[(Overlay, Message)] = &[
    (Overlay::About, Message::AboutClosed),
    (Overlay::ProjectSelector, Message::ProjectSelectorClosed),
    (Overlay::RenameProject, Message::RenameCancelled),
    (Overlay::AddWorktree, Message::AddWorktreeCancelled),
    (Overlay::Settings, Message::SettingsCancelled),
    (
        Overlay::ConfirmWorktreeDelete,
        Message::WorktreeDeleteCancelled,
    ),
    (Overlay::RenameWorktree, Message::WorktreeRenameCancelled),
    (
        Overlay::ConfirmSessionRemove,
        Message::SessionRemoveCancelled,
    ),
    (
        Overlay::ConfirmForgetProject,
        Message::ProjectForgetCancelled,
    ),
];

// ---------------------------------------------------------------------------
// D1 — which surface Escape belongs to when more than one is open.
//
// The contract (`contracts/overlay-registry.md`, D1) says "when a popover and a modal are both
// open, Escape closes the popover first". That is **not** what the code does, and FR-012 says to
// preserve what the code does, so these tests assert the code and the contract has been corrected
// to match. Written out because the difference is easy to read past:
//
//     if state.overlay == Overlay::None && state.sidebar_filter_open { ...popover... }
//     match state.overlay { ...modal... }
//
// The popover branch comes first *textually*, which is what makes it look like popover-priority.
// But it is guarded on no modal being open, so whenever both are open the modal wins.
//
// The comment at that branch says the combination is unreachable because `open_overlay` clears
// popovers. That is only half true: it stops a modal opening *over* a popover, but
// `SidebarFilterMenuToggled` sets the flag with no regard for `overlay`, so a popover can still be
// opened over a modal through the reducer. The state is representable and is handled deliberately;
// it is not a gap.
// ---------------------------------------------------------------------------

/// A modal open *and* a popover open at the same time.
fn modal_and_popover(overlay: Overlay) -> State {
    State {
        overlay,
        sidebar_filter_open: true,
        ..Default::default()
    }
}

#[test]
fn escape_belongs_to_the_popover_when_nothing_modal_is_open() {
    let mut state = State::default();
    state.update(Message::SidebarFilterMenuToggled);
    assert!(state.sidebar_filter_open, "precondition: the panel is open");

    assert_eq!(
        on_escape(&state),
        Some(Message::SidebarFilterMenuToggled),
        "the everyday case: one lightweight surface open, and Escape closes it"
    );
}

#[test]
fn escape_belongs_to_the_modal_when_a_popover_is_open_over_it() {
    for (overlay, modal_cancel) in MODALS {
        let state = modal_and_popover(*overlay);

        assert_eq!(
            on_escape(&state),
            Some(modal_cancel.clone()),
            "with {overlay:?} open beneath a popover, Escape currently reaches the modal. This is \
             the existing priority FR-012 requires preserving; if the generic dispatch reverses \
             it to match the contract's prose, that is a behaviour change and needs to be argued \
             for, not slipped in"
        );
    }
}

#[test]
fn a_popover_alone_and_a_popover_over_a_modal_are_not_the_same_case() {
    // Guards against a dispatch that "simplifies" by treating any open popover as the Escape
    // target -- which would pass the everyday test above while silently changing the tie-break.
    let mut alone = State::default();
    alone.update(Message::SidebarFilterMenuToggled);
    let over_modal = modal_and_popover(Overlay::About);

    assert_ne!(
        on_escape(&alone),
        on_escape(&over_modal),
        "the same popover is open in both states, so a dispatch that answers identically has \
         stopped consulting what is underneath it"
    );
}

// ---------------------------------------------------------------------------
// D2 — opening a modal closes every lightweight popover.
// ---------------------------------------------------------------------------

/// A state with all four popovers that `open_overlay` is responsible for clearing.
fn every_dismissible_popover_open() -> State {
    State {
        help_menu_open: true,
        project_switcher_open: true,
        sidebar_filter_open: true,
        project_menu_open: Some(micold_client::features::project::ProjectMenu {
            path: std::path::PathBuf::from("/p"),
            anchor: (10, 10),
        }),
        ..Default::default()
    }
}

#[test]
fn opening_a_modal_closes_the_popovers_floating_above_it() {
    for (overlay, _) in MODALS {
        let mut state = every_dismissible_popover_open();

        state.open_overlay(*overlay);

        assert_eq!(state.overlay, *overlay, "precondition: the modal opened");
        assert!(
            !state.help_menu_open,
            "the overflow menu survived {overlay:?} opening"
        );
        assert!(
            !state.project_switcher_open,
            "the project switcher survived {overlay:?} opening"
        );
        assert!(
            !state.sidebar_filter_open,
            "the filter panel survived {overlay:?} opening"
        );
        assert!(
            state.project_menu_open.is_none(),
            "the project context menu survived {overlay:?} opening"
        );
    }
}

#[test]
fn opening_a_modal_over_nothing_is_not_a_special_case() {
    let mut state = State::default();

    state.open_overlay(Overlay::About);

    assert_eq!(
        state.overlay,
        Overlay::About,
        "clearing popovers that were never open must not be conditional on any having been"
    );
}

// ---------------------------------------------------------------------------
// D3 — dismissal does not touch state the dismissal does not own.
// ---------------------------------------------------------------------------

#[test]
fn closing_the_filter_panel_leaves_the_active_filters_alone() {
    use micold_client::features::sidebar::TagFilter;
    use micold_core::naming::ConventionalType;

    let mut state = State::default();
    state.update(Message::SidebarFilterMenuToggled);
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    )));
    state.update(Message::SidebarFilterToggled(TagFilter::HasIssue));
    let chosen = state.sidebar_filters.clone();
    assert_eq!(chosen.len(), 2, "precondition: two filters are active");

    state.update(Message::SidebarFilterMenuToggled);

    assert!(!state.sidebar_filter_open, "precondition: the panel closed");
    assert_eq!(
        state.sidebar_filters, chosen,
        "closing the panel is putting the chooser away, not clearing the choice — a sidebar that \
         silently unfiltered itself every time the panel collapsed would be unusable"
    );
}

#[test]
fn dismissing_a_modal_leaves_the_filters_it_never_owned_alone() {
    use micold_client::features::sidebar::TagFilter;

    for (overlay, cancel) in MODALS {
        let mut state = State::default();
        state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
        let chosen = state.sidebar_filters.clone();

        state.open_overlay(*overlay);
        state.update(cancel.clone());

        assert_eq!(
            state.sidebar_filters, chosen,
            "cancelling {overlay:?} reached into the sidebar's filters, which it does not own"
        );
    }
}

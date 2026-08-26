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

use micold_client::features::help;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::worktree::Msg as WorktreeMsg;
use std::path::PathBuf;

use micold_client::app::{on_escape, Message, State};
use micold_client::features::project::RenameDraft;
use micold_client::features::worktree::WorktreeRenameDraft;
use micold_core::selector::Selector;
use micold_core::session::SessionId;

/// Every modal surface: how to open it, and the message its cancellation produces.
///
/// No "nothing open" row — that is the absence of a surface, and Tier 2 stops representing it as
/// one. Until T037 the first column was an `Overlay` variant, which both named the modal and
/// opened it; the enum is gone, so opening one means building the state it draws from.
#[allow(clippy::type_complexity)]
const MODALS: &[(&str, fn(&mut State), Message)] = &[
    (
        "about",
        |s| s.help.about_open = true,
        Message::Help(HelpMsg::AboutClosed),
    ),
    (
        "project_selector",
        |s| s.selector = Some(Selector::open_at(PathBuf::from("/tmp"))),
        Message::Project(ProjectMsg::SelectorClosed),
    ),
    (
        "rename_project",
        |s| {
            s.rename_draft = Some(RenameDraft {
                path: PathBuf::from("/tmp"),
                text: String::new(),
                error: None,
            })
        },
        Message::Project(ProjectMsg::RenameCancelled),
    ),
    (
        "add_worktree",
        |s| s.worktree_form.form = Some(Default::default()),
        Message::WorktreeForm(micold_client::features::worktree_form::Msg::Cancelled),
    ),
    (
        "settings",
        |s| s.settings_draft = Some(Default::default()),
        Message::Settings(SettingsMsg::Cancelled),
    ),
    (
        "confirm_worktree_delete",
        |s| s.worktree_delete_target = Some("wt".to_string()),
        Message::Worktree(WorktreeMsg::DeleteCancelled),
    ),
    (
        "rename_worktree",
        |s| {
            s.worktree_rename_draft = Some(WorktreeRenameDraft {
                dir_name: "wt".to_string(),
                text: String::new(),
                error: None,
            })
        },
        Message::Worktree(WorktreeMsg::RenameCancelled),
    ),
    (
        "confirm_session_remove",
        |s| s.session_remove_target = Some(SessionId::new()),
        Message::Session(SessionMsg::RemoveCancelled),
    ),
    (
        "confirm_forget_project",
        |s| s.forget_target = Some(PathBuf::from("/p")),
        Message::Project(ProjectMsg::ForgetCancelled),
    ),
];

/// Which dialog is open, by name — the question `state.overlay` answered before T037.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

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
fn modal_and_popover(open: fn(&mut State)) -> State {
    let mut state = State {
        sidebar_filter_open: true,
        ..Default::default()
    };
    open(&mut state);
    state
}

#[test]
fn escape_belongs_to_the_popover_when_nothing_modal_is_open() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(state.sidebar_filter_open, "precondition: the panel is open");

    assert_eq!(
        on_escape(&state),
        Some(Message::Sidebar(SidebarMsg::FilterMenuToggled)),
        "the everyday case: one lightweight surface open, and Escape closes it"
    );
}

#[test]
fn escape_belongs_to_the_modal_when_a_popover_is_open_over_it() {
    for (name, open, modal_cancel) in MODALS {
        let state = modal_and_popover(*open);

        assert_eq!(
            on_escape(&state),
            Some(modal_cancel.clone()),
            "with {name} open beneath a popover, Escape currently reaches the modal. This is \
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
    alone.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    let over_modal = modal_and_popover(|s| s.help.about_open = true);

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
        help: help::State {
            help_menu_open: true,
            ..Default::default()
        },
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
    for (name, open, _) in MODALS {
        let mut state = every_dismissible_popover_open();

        state.clear_for_dialog();
        open(&mut state);

        assert_eq!(
            open_dialog(&state),
            Some(*name),
            "precondition: the modal opened"
        );
        assert!(
            !state.help.help_menu_open,
            "the overflow menu survived {name} opening"
        );
        assert!(
            !state.project_switcher_open,
            "the project switcher survived {name} opening"
        );
        assert!(
            !state.sidebar_filter_open,
            "the filter panel survived {name} opening"
        );
        assert!(
            state.project_menu_open.is_none(),
            "the project context menu survived {name} opening"
        );
    }
}

#[test]
fn opening_a_modal_over_nothing_is_not_a_special_case() {
    let mut state = State::default();

    state.clear_for_dialog();
    state.help.about_open = true;

    assert_eq!(
        open_dialog(&state),
        Some("about"),
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
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Feat),
    )));
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::HasIssue,
    )));
    let chosen = state.sidebar_filters.clone();
    assert_eq!(chosen.len(), 2, "precondition: two filters are active");

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));

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

    for (name, open, cancel) in MODALS {
        let mut state = State::default();
        state.update(Message::Sidebar(SidebarMsg::FilterToggled(
            TagFilter::Untyped,
        )));
        let chosen = state.sidebar_filters.clone();

        state.clear_for_dialog();
        open(&mut state);
        state.update(cancel.clone());

        assert_eq!(
            state.sidebar_filters, chosen,
            "cancelling {name} reached into the sidebar's filters, which it does not own"
        );
    }
}

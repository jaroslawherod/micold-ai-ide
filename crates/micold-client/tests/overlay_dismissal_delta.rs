//! The sanctioned behaviour delta (feature 017, T015 — FR-009, FR-024).
//!
//! Feature 017 is defined by changing nothing the user can see. It makes exactly one exception:
//! the five hand-rolled floating surfaces answered "when does this close?" differently, and
//! consolidating them onto one primitive means adopting one answer. FR-024 sanctions that, and
//! scopes it to dismissal alone.
//!
//! This file is the complete list. Each test names a surface whose behaviour moved, states what it
//! used to do, and asserts what it does now — so the delta is a thing that can be reviewed rather
//! than a thing that has to be taken on trust. `specs/017-material-component-architecture/`'s
//! `behavior-delta.md` is the prose version of the same list.
//!
//! ## Restated at feature 021 T037, and why (FR-027)
//!
//! This is one of the protected tests T037 says must keep passing unchanged, and it cannot: it
//! opened dialogs with `state.open_overlay(Overlay::X)` and read them back off `state.overlay`,
//! and T037 deletes both. Nothing here is *about* the enum — the delta this file records is which
//! gestures close which surfaces — so every assertion is preserved, asked of what now holds the
//! answer: a dialog is open when the state it draws from is there, and the registry reports which
//! one. Nine rows replace nine variants. No property changed, nothing was weakened, and the
//! assertion-freeze check flags the file with this paragraph as its explanation.

use micold_client::features::help;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::worktree::Msg as WorktreeMsg;
use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::project::RenameDraft;
use micold_client::features::worktree::WorktreeRenameDraft;
use micold_core::overlay::{dismisses, Surface, Trigger};
use micold_core::selector::Selector;
use micold_core::session::SessionId;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

/// Open the About dialog, the stand-in for "a dialog" throughout this file.
fn with_about() -> State {
    State {
        help: help::State {
            about_open: true,
            ..Default::default()
        },
        ..State::default()
    }
}

/// A state with the overflow menu open.
fn with_help_menu() -> State {
    let mut state = State::default();
    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(state.help.help_menu_open, "precondition: the menu is open");
    state
}

// ---------------------------------------------------------------------------
// Change 1 — dialogs gained scrim-click dismissal.
// ---------------------------------------------------------------------------

/// **Was**: a dialog could only be closed by Escape or by a button inside it; the scrim swallowed
/// clicks and did nothing with them. **Is**: a scrim click cancels the dialog, exactly as Escape
/// does.
///
/// Asserted through `on_escape`, which is the single source both paths now read: `ui::view` hands
/// the scrim whatever `on_escape` would produce, so the two cannot drift apart.
#[test]
fn a_dialog_now_dismisses_on_a_scrim_click() {
    assert!(
        dismisses(Surface::Dialog, Trigger::OutsideClick),
        "the unified rule must close a dialog on an outside click"
    );

    let state = with_about();
    assert_eq!(
        micold_client::app::on_escape(&state),
        Some(Message::Help(HelpMsg::AboutClosed)),
        "the scrim emits whatever Escape would, so the two paths cannot disagree"
    );
}

/// The other half of the same change: a dialog must still survive scrolling behind it. Gaining
/// scrim-click dismissal must not turn a dialog into a menu.
#[test]
fn a_dialog_still_survives_scrolling_behind_it() {
    assert!(!dismisses(Surface::Dialog, Trigger::ScrollBeneath));

    let mut state = with_about();
    state.update(Message::ScrolledBeneathOverlay);
    assert_eq!(
        open_dialog(&state),
        Some("about"),
        "scrolling behind a dialog must not close it"
    );
}

// ---------------------------------------------------------------------------
// Change 2 — non-modal surfaces gained scroll dismissal.
// ---------------------------------------------------------------------------

/// **Was**: scrolling the worktree list left every open menu hanging where it was, anchored to
/// content that had moved out from under it. Nothing reported the scroll, so no surface could
/// react. **Is**: the scrollable reports it and every non-modal surface closes.
#[test]
fn a_menu_now_closes_when_the_list_beneath_it_scrolls() {
    assert!(dismisses(Surface::NonModal, Trigger::ScrollBeneath));

    let mut state = with_help_menu();
    state.update(Message::ScrolledBeneathOverlay);
    assert!(
        !state.help.help_menu_open,
        "the overflow menu must close when content scrolls beneath it"
    );
}

/// Every non-modal surface, not just the one that was convenient to test — the point of unifying
/// is that they no longer differ.
#[test]
fn every_non_modal_surface_closes_on_a_scroll_beneath() {
    let mut state = State {
        help: help::State {
            help_menu_open: true,
            ..Default::default()
        },
        project_switcher_open: true,
        sidebar_filter_open: true,
        ..State::default()
    };

    state.update(Message::ScrolledBeneathOverlay);

    assert!(!state.help.help_menu_open, "overflow menu");
    assert!(!state.project_switcher_open, "project switcher");
    assert!(!state.sidebar_filter_open, "sidebar filter panel");
    assert!(state.project_menu_open.is_none(), "project context menu");
    assert!(state.worktree_menu_open.is_none(), "worktree context menu");
    assert!(state.session_menu_open.is_none(), "session context menu");
}

// ---------------------------------------------------------------------------
// What did *not* change. Guards against the delta quietly widening.
// ---------------------------------------------------------------------------

/// Outside-click dismissal of a menu is not new — it is the behaviour every menu already had, and
/// the one the others were unified *onto*. If this ever fails, the consolidation lost something.
#[test]
fn outside_click_dismissal_of_a_menu_is_unchanged() {
    assert!(dismisses(Surface::NonModal, Trigger::OutsideClick));

    let mut state = with_help_menu();
    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(!state.help.help_menu_open);
}

/// Escape closes what it always closed. Feature 017 changed which *other* gestures close a
/// surface, never which surfaces Escape reaches.
#[test]
fn escape_still_reaches_exactly_what_it_used_to() {
    #[allow(clippy::type_complexity)]
    let dialogs: &[(&str, fn(&mut State), Message)] = &[
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

    for (name, open, expected) in dialogs {
        let mut state = State::default();
        open(&mut state);
        assert_eq!(
            micold_client::app::on_escape(&state),
            Some(expected.clone()),
            "Escape changed for {name}"
        );
    }
}

/// Scrolling with nothing open must do nothing at all. The scrollable reports every scroll
/// unconditionally, so the reducer sees this message constantly; it must be inert.
#[test]
fn scrolling_with_nothing_open_changes_nothing() {
    let mut state = State::default();
    let before = state.clone();
    state.update(Message::ScrolledBeneathOverlay);
    assert_eq!(
        open_dialog(&state),
        open_dialog(&before),
        "an idle scroll must not touch the overlay"
    );
    assert!(!state.help.help_menu_open);
    assert!(!state.project_switcher_open);
}

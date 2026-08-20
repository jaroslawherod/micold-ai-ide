//! Feature 015 — reach "Forget project" by right-clicking a row in the top-bar switcher.
//!
//! Feature 014 already provides forgetting itself (`Workspace::forget`, the confirm dialog, and
//! the Forget control in the known-projects list); this feature only adds the switcher entry
//! point. These tests therefore cover the new pure surface: the context menu's open/close/replace
//! behavior, its press-point anchoring, on-screen clamping, mutual exclusion with the other popovers,
//! and the hand-off into the existing `ProjectForgetRequested` flow. Rendering is build-verified
//! and validated by quickstart.md.

use micold_client::app::{Message, State};
use micold_client::features::project::clamp_menu_anchor;
use std::path::PathBuf;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

// --- Opening, closing, and replacing the menu ---

#[test]
fn right_click_opens_the_menu_and_the_switcher_stays_open_behind_it() {
    // The right-click originates from the open switcher panel.
    let mut st = State {
        project_switcher_open: true,
        ..Default::default()
    };

    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));

    assert_eq!(
        st.project_menu_open.as_ref().map(|m| m.path.clone()),
        Some(PathBuf::from("/a"))
    );
    assert!(
        st.project_switcher_open,
        "the switcher panel stays open behind the row context menu, so the row list stays visible"
    );
}

#[test]
fn toggling_the_same_project_closes_the_menu_and_a_different_one_replaces_it() {
    let mut st = State::default();

    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));
    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));
    assert_eq!(st.project_menu_open, None, "same project toggles off");

    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));
    st.update(Message::ProjectMenuToggled(PathBuf::from("/b"), (100, 100)));
    assert_eq!(
        st.project_menu_open.as_ref().map(|m| m.path.clone()),
        Some(PathBuf::from("/b")),
        "a different project replaces it — only one menu is ever open"
    );

    st.update(Message::ProjectMenuDismissed);
    assert_eq!(st.project_menu_open, None);
}

#[test]
fn opening_any_other_popover_closes_the_project_menu() {
    for opener in [
        Message::HelpMenuToggled,
        Message::ProjectSwitcherToggled,
        Message::SidebarFilterMenuToggled,
        Message::WorktreeMenuToggled("w1".into(), (100, 100)),
    ] {
        let mut st = State::default();
        st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));
        assert!(st.project_menu_open.is_some());

        st.update(opener);
        assert_eq!(
            st.project_menu_open, None,
            "opening another popover closes the project context menu"
        );
    }
}

// --- Press-point anchoring (desktop context-menu behavior) ---

#[test]
fn the_menu_anchors_at_the_press_point() {
    let mut st = State {
        project_switcher_open: true,
        ..Default::default()
    };

    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (412, 233)));

    assert_eq!(
        st.project_menu_open.as_ref().expect("menu open").anchor,
        (412, 233),
        "the panel's top-left corner sits at the click point, so it opens below-right of the pointer"
    );
}

#[test]
fn reopening_on_another_row_re_anchors_at_the_new_press_point() {
    let mut st = State {
        project_switcher_open: true,
        ..Default::default()
    };
    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));
    assert_eq!(st.project_menu_open.as_ref().unwrap().anchor, (100, 100));

    st.update(Message::ProjectMenuToggled(PathBuf::from("/b"), (640, 480)));

    let menu = st.project_menu_open.as_ref().expect("menu open");
    assert_eq!(menu.path, PathBuf::from("/b"));
    assert_eq!(
        menu.anchor,
        (640, 480),
        "re-anchored at the new click point"
    );
}

#[test]
fn window_resize_is_recorded_for_clamping() {
    let mut st = State::default();
    assert_eq!(st.window_size, (0, 0), "unknown until reported");

    st.update(Message::WindowResized {
        width: 1280,
        height: 720,
    });

    assert_eq!(st.window_size, (1280, 720));
}

// --- Clamping: the menu can never open off-screen ---

#[test]
fn the_anchor_is_clamped_so_the_panel_never_leaves_the_window() {
    let menu = (220u16, 44u16);
    let window = (1000u16, 800u16);

    // Comfortably inside: untouched.
    assert_eq!(clamp_menu_anchor((100, 100), menu, window), (100, 100));
    // Past the right edge: slid left just enough to fit exactly.
    assert_eq!(clamp_menu_anchor((900, 100), menu, window), (780, 100));
    // Past the bottom edge: slid up just enough to fit exactly.
    assert_eq!(clamp_menu_anchor((100, 790), menu, window), (100, 756));
    // Bottom-right corner: both axes clamped.
    assert_eq!(clamp_menu_anchor((999, 799), menu, window), (780, 756));
    // Exactly flush against the edge already fits, so it stays put.
    assert_eq!(clamp_menu_anchor((780, 756), menu, window), (780, 756));
}

#[test]
fn clamping_degrades_safely_on_odd_window_sizes() {
    let menu = (220u16, 44u16);

    // Window size not reported yet -> no clamping (better than clamping to a bogus 0x0, which
    // would slam every menu into the top-left corner).
    assert_eq!(clamp_menu_anchor((640, 480), menu, (0, 0)), (640, 480));
    // A window narrower/shorter than the menu clamps to the origin rather than underflowing.
    assert_eq!(clamp_menu_anchor((50, 50), menu, (100, 30)), (0, 0));
}

// --- Hand-off into feature 014's existing forget flow ---

#[test]
fn choosing_forget_closes_the_menu_and_opens_the_existing_confirm_dialog() {
    let mut st = State {
        project_switcher_open: true,
        ..Default::default()
    };
    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));

    // The menu item emits feature 014's message — this feature adds no second forget path.
    st.update(Message::ProjectForgetRequested(PathBuf::from("/a")));

    assert_eq!(st.project_menu_open, None, "the context menu closes");
    assert_eq!(st.forget_target, Some(PathBuf::from("/a")));
    assert_eq!(open_dialog(&st), Some("confirm_forget_project"));
    assert!(
        !st.project_switcher_open,
        "opening the confirm modal closes the switcher (open_overlay)"
    );
}

#[test]
fn dismissing_the_menu_forgets_nothing() {
    let mut st = State {
        project_switcher_open: true,
        ..Default::default()
    };
    st.update(Message::ProjectMenuToggled(PathBuf::from("/a"), (100, 100)));

    st.update(Message::ProjectMenuDismissed);

    assert_eq!(st.project_menu_open, None);
    assert_eq!(st.forget_target, None, "nothing was staged for removal");
    assert_eq!(open_dialog(&st), None, "no confirmation was opened");
}

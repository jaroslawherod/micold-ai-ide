//! Sidebar hide/show + adjustable-width state (feature 005 UI enhancement).

use micold_ai_ide::app::{
    Message, State, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
};

#[test]
fn defaults_visible_with_default_width() {
    let state = State::default();
    assert!(!state.sidebar_hidden);
    assert!(!state.sidebar_dragging);
    assert_eq!(state.sidebar_width_px(), SIDEBAR_DEFAULT_WIDTH);
}

#[test]
fn toggling_hides_and_shows() {
    let mut state = State::default();
    state.update(Message::SidebarToggled);
    assert!(state.sidebar_hidden);
    state.update(Message::SidebarToggled);
    assert!(!state.sidebar_hidden);
}

#[test]
fn drag_updates_width_only_while_dragging() {
    let mut state = State::default();
    // A move with no active drag is ignored.
    state.update(Message::SidebarDragMoved(250));
    assert_eq!(state.sidebar_width_px(), SIDEBAR_DEFAULT_WIDTH);

    state.update(Message::SidebarDragStarted);
    assert!(state.sidebar_dragging);
    state.update(Message::SidebarDragMoved(250));
    assert_eq!(state.sidebar_width_px(), 250);

    state.update(Message::SidebarDragEnded);
    assert!(!state.sidebar_dragging);
}

#[test]
fn drag_width_is_clamped_to_bounds() {
    let mut state = State::default();
    state.update(Message::SidebarDragStarted);

    state.update(Message::SidebarDragMoved(10)); // below min
    assert_eq!(state.sidebar_width_px(), SIDEBAR_MIN_WIDTH);

    state.update(Message::SidebarDragMoved(5000)); // above max
    assert_eq!(state.sidebar_width_px(), SIDEBAR_MAX_WIDTH);
}

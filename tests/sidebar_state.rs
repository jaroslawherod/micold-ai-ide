//! Sidebar hide/show + adjustable-width state (feature 005 UI enhancement).

use micold_ai_ide::app::{
    Message, Overlay, State, TagFilter, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::project::{Availability, Project};
use std::path::PathBuf;

fn state_with_active() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state
}

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

// --- Feature 008 US2: worktree context menu open state ---

#[test]
fn worktree_menu_toggles_replaces_and_dismisses() {
    let mut state = State::default();
    // Toggle open.
    state.update(Message::WorktreeMenuToggled("feat-a".to_string()));
    assert_eq!(state.worktree_menu_open.as_deref(), Some("feat-a"));
    // Toggling the same one closes it.
    state.update(Message::WorktreeMenuToggled("feat-a".to_string()));
    assert_eq!(state.worktree_menu_open, None);
    // Opening a different one while one is open replaces it (only one open at a time).
    state.update(Message::WorktreeMenuToggled("feat-a".to_string()));
    state.update(Message::WorktreeMenuToggled("feat-b".to_string()));
    assert_eq!(state.worktree_menu_open.as_deref(), Some("feat-b"));
    // Dismiss clears.
    state.update(Message::WorktreeMenuDismissed);
    assert_eq!(state.worktree_menu_open, None);
}

// --- Feature 008 US3: worktree rename draft lifecycle ---

#[test]
fn worktree_rename_seeds_edits_and_applies() {
    let mut state = state_with_active();
    state.update(Message::WorktreeRenameStarted(
        "feat-abc-123-login-page".to_string(),
    ));
    assert_eq!(state.overlay, Overlay::RenameWorktree);
    assert!(state.worktree_menu_open.is_none());
    let draft = state.worktree_rename_draft.as_ref().unwrap();
    assert_eq!(draft.dir_name, "feat-abc-123-login-page");
    assert_eq!(draft.text, "Login page"); // seeded from the derived name

    state.update(Message::WorktreeRenameTextChanged("My Login".to_string()));
    assert_eq!(state.worktree_rename_draft.as_ref().unwrap().text, "My Login");

    state.update(Message::WorktreeRenameConfirmed);
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_rename_draft.is_none());
    assert_eq!(
        state.worktree_display_name("feat-abc-123-login-page"),
        "My Login"
    );
}

#[test]
fn worktree_rename_empty_keeps_prior_name_with_error() {
    let mut state = state_with_active();
    state.update(Message::WorktreeRenameStarted("feat-x".to_string()));
    state.update(Message::WorktreeRenameTextChanged("   ".to_string()));
    state.update(Message::WorktreeRenameConfirmed);
    // Stays open with an error; no override applied → still the derived name.
    assert_eq!(state.overlay, Overlay::RenameWorktree);
    assert!(state.worktree_rename_draft.as_ref().unwrap().error.is_some());
    assert_eq!(state.worktree_display_name("feat-x"), "X");
}

#[test]
fn duplicate_worktree_display_names_are_allowed() {
    let mut state = state_with_active();
    for dir in ["feat-a", "feat-b"] {
        state.update(Message::WorktreeRenameStarted(dir.to_string()));
        state.update(Message::WorktreeRenameTextChanged("Same".to_string()));
        state.update(Message::WorktreeRenameConfirmed);
    }
    // Identity stays distinct even though the displayed names collide (spec Edge Cases).
    assert_eq!(state.worktree_display_name("feat-a"), "Same");
    assert_eq!(state.worktree_display_name("feat-b"), "Same");
}

// --- Feature 008: hover-revealed row actions state ---

#[test]
fn worktree_hover_sets_and_clears() {
    let mut state = State::default();
    state.update(Message::WorktreeHovered("feat-a".to_string()));
    assert_eq!(state.hovered_worktree.as_deref(), Some("feat-a"));
    // A stale exit from a different row does not clear the current hover.
    state.update(Message::WorktreeUnhovered("feat-b".to_string()));
    assert_eq!(state.hovered_worktree.as_deref(), Some("feat-a"));
    // Leaving the hovered row clears it.
    state.update(Message::WorktreeUnhovered("feat-a".to_string()));
    assert!(state.hovered_worktree.is_none());
}

// --- Feature 008 US4: sidebar filter set ---

#[test]
fn sidebar_filter_toggles_and_clears() {
    let mut state = State::default();
    let feat = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::SidebarFilterToggled(feat));
    assert!(state.sidebar_filters.contains(&feat));
    // Toggling again removes it.
    state.update(Message::SidebarFilterToggled(feat));
    assert!(!state.sidebar_filters.contains(&feat));
    // Multiple filters accumulate; clear empties them all.
    state.update(Message::SidebarFilterToggled(feat));
    state.update(Message::SidebarFilterToggled(TagFilter::HasIssue));
    assert_eq!(state.sidebar_filters.len(), 2);
    state.update(Message::SidebarFiltersCleared);
    assert!(state.sidebar_filters.is_empty());
}

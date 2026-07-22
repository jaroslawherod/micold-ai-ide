//! Sidebar hide/show + adjustable-width state (feature 005 UI enhancement).

use micold_client::app::{
    on_escape, Message, Overlay, SidebarEntry, State, TagFilter, SIDEBAR_DEFAULT_WIDTH,
    SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
};
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::worktree::{Worktree, WorktreeStatus};
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

// --- Cross-app clipboard copy (worktree "Copy name" context-menu action) ---

#[test]
fn text_copy_requested_is_a_no_op_in_the_pure_reducer() {
    // The binary performs the actual clipboard write; the reducer has no state to update.
    let mut state = State::default();
    state.update(Message::WorktreeMenuToggled("feat-a".to_string()));
    let before = state.clone();
    state.update(Message::TextCopyRequested("Login page".to_string()));
    assert_eq!(state, before);
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
    assert_eq!(
        state.worktree_rename_draft.as_ref().unwrap().text,
        "My Login"
    );

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
    assert!(state
        .worktree_rename_draft
        .as_ref()
        .unwrap()
        .error
        .is_some());
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

// --- Feature 009: sidebar filter panel toggle ---

#[test]
fn sidebar_filter_panel_starts_closed() {
    let state = State::default();
    assert!(!state.sidebar_filter_open);
}

#[test]
fn sidebar_filter_menu_toggle_opens_and_closes_and_excludes_siblings() {
    let mut state = State {
        help_menu_open: true,
        ..Default::default()
    };

    state.update(Message::SidebarFilterMenuToggled);
    assert!(state.sidebar_filter_open);
    // Opening the filter panel closes the sibling popovers (mutual exclusion, symmetric with
    // the existing HelpMenuToggled/ProjectSwitcherToggled pair).
    assert!(!state.help_menu_open);
    assert!(!state.project_switcher_open);

    state.update(Message::SidebarFilterMenuToggled);
    assert!(!state.sidebar_filter_open);
}

#[test]
fn opening_help_menu_or_project_switcher_closes_the_filter_panel() {
    let mut state = State::default();
    state.update(Message::SidebarFilterMenuToggled);
    assert!(state.sidebar_filter_open);

    state.update(Message::HelpMenuToggled);
    assert!(!state.sidebar_filter_open);

    state.update(Message::SidebarFilterMenuToggled);
    assert!(state.sidebar_filter_open);

    state.update(Message::ProjectSwitcherToggled);
    assert!(!state.sidebar_filter_open);
}

#[test]
fn closing_the_filter_panel_never_changes_active_filters() {
    let mut state = State::default();
    let feat = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::SidebarFilterToggled(feat));
    assert!(state.sidebar_filters.contains(&feat));

    state.update(Message::SidebarFilterMenuToggled); // open
    assert!(state.sidebar_filters.contains(&feat));
    state.update(Message::SidebarFilterMenuToggled); // close
    assert!(
        state.sidebar_filters.contains(&feat),
        "toggling panel visibility must not alter the active filter set (FR-007/FR-008)"
    );
}

#[test]
fn escape_dismisses_the_open_filter_panel_when_no_overlay_is_open() {
    let mut state = State::default();
    assert_eq!(on_escape(&state), None);

    state.sidebar_filter_open = true;
    assert_eq!(
        on_escape(&state),
        Some(Message::SidebarFilterMenuToggled),
        "Escape must dismiss the filter panel while it's open"
    );
}

#[test]
fn escape_prefers_an_open_overlay_over_the_filter_panel() {
    // Mirrors the keyboard subscription's guard exactly (`ui::subscription()`): if a modal
    // overlay is somehow open at the same time as the filter panel, the overlay's own Escape
    // handling takes priority, not the filter panel's. In practice `State::open_overlay()`
    // keeps this combination from ever occurring (see the next test), but `on_escape` must not
    // silently disagree with the live subscription if that invariant is ever violated.
    let state = State {
        sidebar_filter_open: true,
        overlay: Overlay::AddWorktree,
        ..Default::default()
    };
    assert_eq!(on_escape(&state), Some(Message::AddWorktreeCancelled));
}

#[test]
fn opening_an_overlay_closes_the_filter_panel() {
    // Regression test: previously, opening a modal overlay (e.g. the Add Worktree form) while
    // the filter accordion was open left `sidebar_filter_open` untouched, so `on_escape` and
    // the live keyboard subscription disagreed about what Escape should dismiss. Every
    // overlay-open now routes through `State::open_overlay`, which resets it unconditionally.
    let mut state = State::default();
    state.update(Message::SidebarFilterMenuToggled);
    assert!(state.sidebar_filter_open);

    state.update(Message::AddWorktreeOpened);
    assert!(
        !state.sidebar_filter_open,
        "opening an overlay must close the filter panel"
    );
    assert_eq!(state.overlay, Overlay::AddWorktree);
}

// T020 (010-root-dir-session, FR-011, research.md R4): the Default entry is exempt from the
// sidebar's tag-filter panel — it stays visible no matter which filters are active.
#[test]
fn default_entry_stays_visible_with_an_active_tag_filter() {
    let mut state = state_with_active();
    state.worktrees = vec![Worktree {
        dir_name: "feat-a".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
        branch: Some("feat/a".to_string()),
        status: WorktreeStatus::Valid,
    }];

    // Sanity: a filter matching nothing still leaves worktree entries empty...
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix, // no `fix` worktree exists — this filter matches nothing
    )));
    assert!(
        !state.available_tag_filters().is_empty(),
        "feat-a offers a filter to toggle"
    );
    let entries = state.sidebar_entries();
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, SidebarEntry::Default(_))),
        "Default entry must remain present even when the active filter matches zero worktrees"
    );
    assert!(
        !entries
            .iter()
            .any(|e| matches!(e, SidebarEntry::Worktree(_))),
        "the worktree portion is still correctly filtered out"
    );
}

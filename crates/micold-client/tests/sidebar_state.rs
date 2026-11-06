//! Sidebar hide/show + adjustable-width state (feature 005 UI enhancement).

use micold_client::app::{
    on_escape, Message, State, MIN_WINDOW_SIZE, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH,
    SIDEBAR_MIN_WIDTH,
};
use micold_client::features::help;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::sidebar;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::sidebar::{SidebarEntry, TagFilter};
use micold_client::features::window::COMPACT_WIDTH;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::worktree_form;
use micold_core::naming::ConventionalType;
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::support;
use support::sidebar_fixture::{deliver, listed, session_at, state_with_active, worktree};

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

#[test]
fn defaults_visible_with_default_width() {
    let state = State::default();
    assert!(!state.sidebar.hidden);
    assert_eq!(state.sidebar_width_px(), SIDEBAR_DEFAULT_WIDTH);
}

#[test]
fn toggling_hides_and_shows() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::Toggled));
    assert!(state.sidebar.hidden);
    state.update(Message::Sidebar(SidebarMsg::Toggled));
    assert!(!state.sidebar.hidden);
}

/// A window one pixel under the compact width, and the smallest the window can be.
const NARROW: (u16, u16) = (COMPACT_WIDTH - 1, 480);
const SMALLEST: (u16, u16) = (MIN_WINDOW_SIZE.width as u16, MIN_WINDOW_SIZE.height as u16);

fn resize(state: &mut State, width: u16, height: u16) {
    state.update(Message::Window(
        micold_client::features::window::Msg::Resized { width, height },
    ));
}

fn toggle(state: &mut State) {
    state.update(Message::Sidebar(SidebarMsg::Toggled));
}

/// Feature 1474 (FR-001 to FR-004): the sidebar gives way below 720px and comes back as the user
/// left it; the stored `hidden` and `width` never change on a resize.
#[test]
fn the_sidebar_collapses_below_720_and_returns_as_left() {
    let mut state = State::default();
    let width = state.sidebar.width;
    resize(&mut state, 1000, 700);
    assert!(!state.sidebar_hidden(), "a wide window shows the sidebar");
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    assert!(
        state.sidebar_hidden(),
        "the smallest window shows the strip"
    );
    resize(&mut state, COMPACT_WIDTH, 480);
    assert!(!state.sidebar_hidden(), "widening to 720 restores it");
    assert!(!state.sidebar.hidden, "the stored state was never written");
    assert_eq!(state.sidebar.width, width, "the width is unchanged");
}

#[test]
fn the_boundary_is_719_compact_720_not() {
    let mut state = State::default();
    resize(&mut state, NARROW.0, NARROW.1);
    assert!(state.sidebar_hidden(), "719 is compact");
    resize(&mut state, COMPACT_WIDTH, 480);
    assert!(!state.sidebar_hidden(), "720 is not compact");
}

#[test]
fn a_hidden_sidebar_stays_hidden_through_a_narrow_window() {
    let mut state = State::default();
    resize(&mut state, 1000, 700);
    toggle(&mut state);
    assert!(state.sidebar_hidden(), "the user hid it");
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    assert!(state.sidebar_hidden(), "still the strip when narrow");
    resize(&mut state, 1000, 700);
    assert!(state.sidebar_hidden(), "still hidden when widened");
    assert!(state.sidebar.hidden, "the stored state is hidden");
}

#[test]
fn starting_narrow_shows_the_strip_without_touching_the_stored_state() {
    let mut state = State::default();
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    assert!(state.sidebar_hidden(), "a narrow start shows the strip");
    assert!(!state.sidebar.hidden, "the stored state is untouched");
}

#[test]
fn an_unreported_size_is_not_compact() {
    let mut state = State::default();
    resize(&mut state, 0, 0);
    assert!(
        !state.sidebar_hidden(),
        "(0, 0) is not reported, not compact"
    );
}

#[test]
fn show_at_compact_width_holds_until_the_window_is_widened() {
    let mut state = State::default();
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    toggle(&mut state);
    assert!(!state.sidebar_hidden(), "the toggle opens the sidebar");
    resize(&mut state, 650, 480);
    assert!(!state.sidebar_hidden(), "it holds while still compact");
    assert!(!state.sidebar.hidden, "the toggle did not write hidden");
    resize(&mut state, COMPACT_WIDTH, 480);
    assert!(!state.sidebar_hidden(), "it stays shown at 720");
    assert!(!state.sidebar.compact_open, "the override is cleared");
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    assert!(state.sidebar_hidden(), "a fresh crossing collapses again");
}

#[test]
fn hide_after_show_at_compact_width_returns_to_the_strip() {
    let mut state = State::default();
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    toggle(&mut state);
    toggle(&mut state);
    assert!(
        state.sidebar_hidden(),
        "the second toggle returns the strip"
    );
    assert!(!state.sidebar.hidden, "the stored state is untouched");
}

#[test]
fn a_toggle_at_720_or_wider_flips_the_stored_state() {
    let mut state = State::default();
    resize(&mut state, 1000, 700);
    toggle(&mut state);
    assert!(state.sidebar.hidden, "the toggle writes hidden");
    assert!(!state.sidebar.compact_open, "no override outside compact");
}

/// Edge case "a resize and a toggle in the same update": a toggle acts on the effective state after
/// the resize that came before it, whichever order they arrive in.
#[test]
fn a_toggle_and_a_resize_act_in_the_order_they_arrive() {
    // Toggle at 1000, then shrink: the hidden sidebar stays hidden.
    let mut state = State::default();
    resize(&mut state, 1000, 700);
    toggle(&mut state);
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    assert!(state.sidebar.hidden, "the toggle wrote hidden while wide");
    assert!(state.sidebar_hidden(), "and it stays hidden when narrow");

    // Shrink, then toggle at 640: the toggle opens the sidebar over the strip, `hidden` untouched.
    let mut state = State::default();
    resize(&mut state, 1000, 700);
    resize(&mut state, SMALLEST.0, SMALLEST.1);
    toggle(&mut state);
    assert!(
        !state.sidebar_hidden(),
        "the toggle after a resize opens it"
    );
    assert!(!state.sidebar.hidden, "without writing hidden");

    // Toggle at 640 (opens), then widen across 720: the override clears, the stored state holds.
    resize(&mut state, COMPACT_WIDTH, 480);
    assert!(!state.sidebar.compact_open, "widening clears the override");
    toggle(&mut state);
    assert!(state.sidebar.hidden, "a toggle at 720 now hides it");
}

/// Each window's `window_size` drives its own `sidebar_hidden()` through `State::update`.
#[test]
fn each_windows_size_drives_its_own_sidebar() {
    let mut narrow = State::default();
    let mut wide = State::default();
    resize(&mut narrow, SMALLEST.0, SMALLEST.1);
    resize(&mut wide, 1000, 700);
    assert_eq!(
        narrow.window.window_size,
        (SMALLEST.0, SMALLEST.1),
        "narrow size"
    );
    assert_eq!(wide.window.window_size, (1000, 700), "wide size");
    assert!(narrow.sidebar_hidden(), "the narrow window shows the strip");
    assert!(!wide.sidebar_hidden(), "the wide window shows the sidebar");
}

/// The drag protocol changed with feature 017 (T041): the resize handle owns the drag itself, so
/// there is no longer a start or an end to report and no `sidebar_dragging` flag to gate on. A
/// width message *is* the drag — the handle only speaks while it is being moved.
#[test]
fn a_reported_width_is_adopted() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::DragMoved(250)));
    assert_eq!(state.sidebar_width_px(), 250);
}

/// Clamping stays here rather than moving into the handle with the drag. How wide the sidebar is
/// allowed to be is a decision about the application's layout, not about the edge being dragged —
/// exactly the logical/presentation split FR-012 draws.
#[test]
fn drag_width_is_clamped_to_bounds() {
    let mut state = State::default();

    state.update(Message::Sidebar(SidebarMsg::DragMoved(10))); // below min
    assert_eq!(state.sidebar_width_px(), SIDEBAR_MIN_WIDTH);

    state.update(Message::Sidebar(SidebarMsg::DragMoved(5000))); // above max
    assert_eq!(state.sidebar_width_px(), SIDEBAR_MAX_WIDTH);
}

// --- Feature 008 US2: worktree context menu open state ---

#[test]
fn worktree_menu_toggles_replaces_and_dismisses() {
    let mut state = State::default();
    let open_dir = |s: &State| s.worktree.menu_open.as_ref().map(|m| m.dir_name.clone());
    // Toggle open, at the point the row was pressed (018 FR-029d).
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    assert_eq!(open_dir(&state).as_deref(), Some("feat-a"));
    assert_eq!(
        state.worktree.menu_open.as_ref().unwrap().anchor,
        (120, 300)
    );
    // Toggling the same one closes it.
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    assert_eq!(state.worktree.menu_open, None);
    // Opening a different one while one is open replaces it (only one open at a time) — and
    // re-anchors at its own press point rather than keeping the first one's (BUG-008).
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-b".to_string(),
        (140, 610),
    )));
    assert_eq!(open_dir(&state).as_deref(), Some("feat-b"));
    assert_eq!(
        state.worktree.menu_open.as_ref().unwrap().anchor,
        (140, 610)
    );
    // Dismiss clears.
    state.update(Message::Worktree(WorktreeMsg::MenuDismissed));
    assert_eq!(state.worktree.menu_open, None);
}

// --- Cross-app clipboard copy (worktree "Copy name" context-menu action) ---

#[test]
fn text_copy_requested_is_a_no_op_in_the_pure_reducer() {
    // The binary performs the actual clipboard write; the reducer has no state to update.
    let mut state = State::default();
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    let before = state.clone();
    state.update(Message::Worktree(WorktreeMsg::TextCopyRequested(
        "Login page".to_string(),
    )));
    assert_eq!(state, before);
}

// --- Feature 008 US3: worktree rename draft lifecycle ---

#[test]
fn worktree_rename_seeds_edits_and_applies() {
    let mut state = state_with_active();
    state.update(Message::Worktree(WorktreeMsg::RenameStarted(
        "feat-abc-123_login-page".to_string(),
    )));
    assert_eq!(open_dialog(&state), Some("rename_worktree"));
    assert!(state.worktree.menu_open.is_none());
    let draft = state.worktree.rename_draft.as_ref().unwrap();
    assert_eq!(draft.dir_name, "feat-abc-123_login-page");
    assert_eq!(draft.text, "Login page"); // seeded from the derived name

    state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
        "My Login".to_string(),
    )));
    assert_eq!(
        state.worktree.rename_draft.as_ref().unwrap().text,
        "My Login"
    );

    state.update(Message::Worktree(WorktreeMsg::RenameConfirmed));
    assert_eq!(open_dialog(&state), None);
    assert!(state.worktree.rename_draft.is_none());
    assert_eq!(
        state.worktree_display_name("feat-abc-123_login-page"),
        "My Login"
    );
}

#[test]
fn worktree_rename_empty_keeps_prior_name_and_marks_the_field_missing() {
    use micold_client::features::window::FieldId;
    use micold_core::required::FieldRequirement::{Missing, Required};
    let mut state = state_with_active();
    state.update(Message::Worktree(WorktreeMsg::RenameStarted(
        "feat-x".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
        "   ".to_string(),
    )));
    let reads = |state: &State| {
        let draft = state.worktree.rename_draft.as_ref().unwrap();
        draft
            .required
            .state_in(&draft.required_fields(), FieldId::RenameWorktreeName)
    };
    assert_eq!(reads(&state), Required, "no error before the confirm");
    state.update(Message::Worktree(WorktreeMsg::RenameConfirmed));
    // Stays open, the name field reads Missing; no override applied → still the derived name.
    assert_eq!(open_dialog(&state), Some("rename_worktree"));
    assert_eq!(reads(&state), Missing);
    assert_eq!(state.worktree_display_name("feat-x"), "X");
    state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
        "Y".to_string(),
    )));
    assert_eq!(reads(&state), Required, "typing clears the error (FR-007)");
}

#[test]
fn project_rename_empty_keeps_the_name_and_marks_the_field_missing() {
    use micold_client::features::window::FieldId;
    use micold_core::required::FieldRequirement::{Missing, Required};
    let mut state = state_with_active();
    state.update(Message::Project(ProjectMsg::RenameStarted(
        std::path::PathBuf::from("/repo"),
    )));
    state.update(Message::Project(ProjectMsg::RenameTextChanged(
        String::new(),
    )));
    let reads = |state: &State| {
        let draft = state.project.rename_draft.as_ref().unwrap();
        draft
            .required
            .state_in(&draft.required_fields(), FieldId::RenameProjectName)
    };
    assert_eq!(reads(&state), Required, "no error before the confirm");
    state.update(Message::Project(ProjectMsg::RenameConfirmed));
    assert_eq!(open_dialog(&state), Some("rename_project"));
    assert_eq!(reads(&state), Missing);
    assert_eq!(state.workspace.projects[0].display_name, "repo");
    state.update(Message::Project(ProjectMsg::RenameTextChanged("x".into())));
    assert_eq!(reads(&state), Required, "typing clears the error (FR-007)");
}

#[test]
fn duplicate_worktree_display_names_are_allowed() {
    let mut state = state_with_active();
    for dir in ["feat-a", "feat-b"] {
        state.update(Message::Worktree(WorktreeMsg::RenameStarted(
            dir.to_string(),
        )));
        state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
            "Same".to_string(),
        )));
        state.update(Message::Worktree(WorktreeMsg::RenameConfirmed));
    }
    // Identity stays distinct even though the displayed names collide (spec Edge Cases).
    assert_eq!(state.worktree_display_name("feat-a"), "Same");
    assert_eq!(state.worktree_display_name("feat-b"), "Same");
}

// --- Feature 008: hover-revealed row actions state ---

#[test]
fn worktree_hover_sets_and_clears() {
    let mut state = State::default();
    state.update(Message::Worktree(WorktreeMsg::Hovered(
        "feat-a".to_string(),
        3,
    )));
    assert_eq!(state.worktree.hovered, Some(("feat-a".to_string(), 3)));
    // A stale exit from a different row does not clear the current hover.
    state.update(Message::Worktree(WorktreeMsg::Unhovered(
        "feat-b".to_string(),
    )));
    assert_eq!(state.worktree.hovered, Some(("feat-a".to_string(), 3)));
    // Leaving the hovered row clears it.
    state.update(Message::Worktree(WorktreeMsg::Unhovered(
        "feat-a".to_string(),
    )));
    assert!(state.worktree.hovered.is_none());
}

// --- Feature 008 US4: sidebar filter set ---

#[test]
fn sidebar_filter_toggles_and_clears() {
    let mut state = State::default();
    let feat = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feat)));
    assert!(state.sidebar.filters.contains(&feat));
    // Toggling again removes it.
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feat)));
    assert!(!state.sidebar.filters.contains(&feat));
    // Multiple filters accumulate; clear empties them all.
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feat)));
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::HasIssue,
    )));
    assert_eq!(state.sidebar.filters.len(), 2);
    state.update(Message::Sidebar(SidebarMsg::FiltersCleared));
    assert!(state.sidebar.filters.is_empty());
}

// --- Feature 1125 (US4.6, FR-015): the chip bar's more menu, where the filter panel was ---

#[test]
fn the_more_menu_starts_closed() {
    let state = State::default();
    assert!(!state.sidebar.more_menu_open);
}

#[test]
fn the_more_menu_toggle_opens_and_closes_and_excludes_siblings() {
    let mut state = State {
        help: help::State {
            help_menu_open: true,
            ..Default::default()
        },
        ..Default::default()
    };

    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(state.sidebar.more_menu_open);
    // Opening the more menu closes the sibling popovers (mutual exclusion, symmetric with
    // the existing help::Msg::MenuToggled/project::Msg::SwitcherToggled pair).
    assert!(!state.help.help_menu_open);
    assert!(!state.project.switcher_open);

    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(!state.sidebar.more_menu_open);
}

#[test]
fn opening_help_menu_or_project_switcher_closes_the_more_menu() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(state.sidebar.more_menu_open);

    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(!state.sidebar.more_menu_open);

    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(state.sidebar.more_menu_open);

    state.update(Message::Project(ProjectMsg::SwitcherToggled));
    assert!(!state.sidebar.more_menu_open);
}

#[test]
fn closing_the_more_menu_never_changes_active_filters() {
    let mut state = State::default();
    let feat = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feat)));
    assert!(state.sidebar.filters.contains(&feat));

    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled)); // open
    assert!(state.sidebar.filters.contains(&feat));
    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled)); // close
    assert!(
        state.sidebar.filters.contains(&feat),
        "toggling the menu must not alter the active filter set (FR-007/FR-008)"
    );
}

#[test]
fn escape_dismisses_the_open_more_menu_when_no_overlay_is_open() {
    let mut state = State::default();
    assert_eq!(on_escape(&state), None);

    state.sidebar.more_menu_open = true;
    assert_eq!(
        on_escape(&state),
        Some(Message::Sidebar(SidebarMsg::MoreMenuToggled)),
        "Escape must dismiss the more menu while it's open"
    );
}

#[test]
fn escape_prefers_an_open_overlay_over_the_more_menu() {
    // Mirrors the keyboard subscription's guard exactly (`ui::subscription()`): if a modal
    // overlay is somehow open at the same time as the more menu, the overlay's own Escape
    // handling takes priority, not the menu's. In practice `State::open_overlay()`
    // keeps this combination from ever occurring (see the next test), but `on_escape` must not
    // silently disagree with the live subscription if that invariant is ever violated.
    let state = State {
        sidebar: sidebar::State {
            more_menu_open: true,
            ..Default::default()
        },

        worktree_form: worktree_form::State {
            form: Some(Default::default()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        on_escape(&state),
        Some(Message::WorktreeForm(
            micold_client::features::worktree_form::Msg::Cancelled
        ))
    );
}

#[test]
fn opening_an_overlay_closes_the_more_menu() {
    // Regression test: previously, opening a modal overlay (e.g. the Add Worktree form) while
    // the filter accordion was open left its flag untouched, so `on_escape` and
    // the live keyboard subscription disagreed about what Escape should dismiss. Every
    // overlay-open now routes through `State::open_overlay`, which resets it unconditionally.
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(state.sidebar.more_menu_open);

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    assert!(
        !state.sidebar.more_menu_open,
        "opening an overlay must close the more menu"
    );
    assert_eq!(open_dialog(&state), Some("add_worktree"));
}

#[test]
fn the_more_menu_hangs_from_the_corner_its_press_reported() {
    let mut state = state_with_active();
    state.update(Message::Sidebar(SidebarMsg::MoreMenuAnchored((240, 120))));
    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(state.sidebar.more_menu_open);
    assert_eq!(state.sidebar.more_anchor, Some((240, 120)));
    state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
    assert!(!state.sidebar.more_menu_open);
    assert_eq!(
        state.sidebar.more_anchor, None,
        "the corner belongs to the press that opened it"
    );
}

#[test]
fn choosing_a_more_menu_item_closes_the_menu() {
    for action in sidebar::MORE_MENU {
        let mut state = state_with_active();
        state.update(Message::Sidebar(SidebarMsg::MoreMenuAnchored((240, 120))));
        state.update(Message::Sidebar(SidebarMsg::MoreMenuToggled));
        state.update(Message::Sidebar(SidebarMsg::MoreMenuChosen(action)));
        assert!(!state.sidebar.more_menu_open, "{action:?}");
        assert_eq!(state.sidebar.more_anchor, None, "{action:?}");
    }
}

/// Each item does what today's header button did: the shell sends the button's own message once
/// the menu has closed (US4.6, FR-015).
#[test]
fn each_more_menu_item_sends_the_header_buttons_message() {
    use micold_client::ui::sidebar::more_action_message;
    assert_eq!(
        sidebar::MORE_MENU.map(more_action_message),
        [
            Message::Worktree(WorktreeMsg::RefreshRequested),
            Message::Attach(micold_client::features::attach::Msg::Opened),
            Message::Sidebar(SidebarMsg::RunInParallelRequested),
        ]
    );
}

/// The items' rule is the header buttons' rule, read from the same state (FR-015).
#[test]
fn the_more_menus_refresh_is_enabled_exactly_when_a_refresh_can_start() {
    let enabled = |state: &State| -> Vec<bool> {
        sidebar::more_menu_items(state.workspace.active.is_some(), state.worktree.refreshing)
            .iter()
            .map(|item| item.enabled)
            .collect()
    };
    let mut state = state_with_active();
    assert!(state.can_refresh_worktrees());
    assert_eq!(enabled(&state), [true, true, true]);
    state.update(Message::Worktree(WorktreeMsg::RefreshRequested));
    assert!(!state.can_refresh_worktrees());
    assert_eq!(enabled(&state), [false, true, true]);
    assert_eq!(enabled(&State::default()), [false, false, false]);
}

/// **All** clears every active filter in one press, as **Clear filters** did (US4.3).
#[test]
fn all_clears_every_filter_in_one_message() {
    let mut state = State::default();
    for filter in [
        TagFilter::Type(ConventionalType::Feat),
        TagFilter::Type(ConventionalType::Fix),
        TagFilter::Untyped,
    ] {
        state.update(Message::Sidebar(SidebarMsg::FilterToggled(filter)));
    }
    assert_eq!(state.sidebar.filters.len(), 3);
    state.update(Message::Sidebar(SidebarMsg::FiltersCleared));
    assert!(state.sidebar.filters.is_empty());
}

// T020 (010-root-dir-session, FR-011, research.md R4): the Default entry is exempt from the
// sidebar's tag-filter panel — it stays visible no matter which filters are active.
#[test]
fn default_entry_stays_visible_with_an_active_tag_filter() {
    let mut state = state_with_active();
    state.worktree.worktrees = vec![Worktree {
        dir_name: "feat-a".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
        branch: Some("feat/a".to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    }];

    // Sanity: a filter matching nothing still leaves worktree entries empty...
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(
            ConventionalType::Fix, // no `fix` worktree exists — this filter matches nothing
        ),
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

// --- Feature 024: re-discovery does not disturb the reveal ------------------------------------
//
// SC-008, asked of the one path both the `WorktreesLoaded` reducer arm and the binary's direct
// re-discovery go through. This is the file that already covers `set_worktrees`'s pruning, so it
// is where "and it does not prune this" belongs.

#[test]
fn re_discovering_worktrees_leaves_the_current_sessions_row_alone() {
    let mut state = state_with_active();
    let path = state.workspace.active.clone().unwrap();
    micold_client::app::drain(
        state.set_worktrees(vec![Worktree {
            dir_name: "feat-a".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
            branch: Some("feat/feat-a".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        }]),
        |o| micold_client::app::interpret(&mut state, o),
    );
    let session = Session::start_new(
        SessionLocation::Worktree("feat-a".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.session.active = Some(id);
    let location = SessionLocation::Worktree("feat-a".to_string());
    assert!(
        state.location_open(&location),
        "precondition: the panel knows the location, so it can open it"
    );

    // A worktree created elsewhere, reported by a fresh discovery: the whole list is replaced.
    micold_client::app::drain(
        state.set_worktrees(vec![
            Worktree {
                dir_name: "feat-a".to_string(),
                path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
                branch: Some("feat/feat-a".to_string()),
                status: WorktreeStatus::Valid,
                included: false,
            },
            Worktree {
                dir_name: "feat-new".to_string(),
                path: PathBuf::from("/repo/.claude/worktrees/feat-new"),
                branch: Some("feat/feat-new".to_string()),
                status: WorktreeStatus::Valid,
                included: false,
            },
        ]),
        |o| micold_client::app::interpret(&mut state, o),
    );

    assert!(
        state.location_open(&location),
        "creating, deleting or re-discovering a worktree replaces the list wholesale; the row \
         holding the current session is derived, so there is nothing for the replacement to prune \
         (SC-008, FR-001b)"
    );
    assert!(
        state.session.reveal_suppressed_for.is_none(),
        "and nothing about the user's own choices is reset by a background discovery either"
    );
}

// --- Feature 029: a refreshed listing is reconciled like any other (FR-004, FR-009, T034) -------
//
// The tests below drive `set_worktrees` + `drain` directly, and that is the point rather than a
// shortcut. FR-004 says a user must not be able to tell from the result which trigger produced a
// listing, and the way this codebase answers that is **structural**: there is one path, not two
// kept in agreement. A refresh's listing arrives on the `CatalogChanged` broadcast like every
// other, `reconcile_catalog` calls `core.set_worktrees(...)` and drains what it returns
// (`catalog_sync.rs`), and `ClientMsg::WorktreeRefresh` adds no second route — the reducer arms
// this feature adds touch `refreshing` and nothing else (`features_worktree.rs` holds that).
//
// So a refresh-specific pair of tests here would be a copy: it would assert about a path that does
// not exist, pass whether or not the shared one behaved, and quietly suggest there were two. These
// assert the shared path, and this comment is what ties them to the requirement.

/// An arranged sidebar: two worktrees, both expanded, a filter applied, scrolled, a session
/// selected. Every field FR-009 names, set to something a reset would visibly undo.
fn arranged() -> State {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);

    state.sidebar.expanded.insert("feat-a".to_string());
    state.sidebar.expanded.insert("feat-b".to_string());
    state.sidebar.default_expanded = true;
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    state.sidebar.scroll_offset = 240;
    state.sidebar.viewport_height = 600;

    let session = Session::start_new(
        SessionLocation::Worktree("feat-a".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    state
        .workspace
        .sessions
        .insert(PathBuf::from("/repo"), vec![session]);
    state.session.active = Some(id);
    state
}

/// FR-009: the common case. Most refreshes find nothing, and a refresh that rearranged the sidebar
/// every time it found nothing would be worse than the switch-away-and-back it replaces.
#[test]
fn an_unchanged_listing_leaves_the_whole_arrangement_alone() {
    let mut state = arranged();
    let before = state.sidebar.clone();
    let selected = state.session.active;

    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);

    assert_eq!(
        state.sidebar, before,
        "a listing identical to the one on screen moved something in the sidebar. Expansion, \
         filters and scroll position are the user's, not the listing's (FR-009)"
    );
    assert_eq!(
        state.session.active, selected,
        "and the session the user is working in is not a function of the worktree listing either"
    );
}

/// FR-004: a worktree that is gone is reconciled by the path that already reconciles one, and the
/// arrangement around it is left alone. Written as one test because "pruned" and "pruned *only*"
/// are the same claim — a reconciliation that cleared the filters too would satisfy either half on
/// its own.
#[test]
fn a_worktree_missing_from_the_listing_is_pruned_and_nothing_else_is() {
    let mut state = arranged();
    let selected = state.session.active;

    deliver(&mut state, vec![listed("feat-a")]);

    assert!(
        !state.sidebar.expanded.contains("feat-b"),
        "a worktree the listing no longer reports must not stay expanded — the row it expands is \
         gone"
    );
    assert!(
        state.sidebar.expanded.contains("feat-a"),
        "and the one that survived must stay expanded; pruning is per row, not a reset"
    );
    assert!(
        state.sidebar.default_expanded,
        "the Default row is not a worktree and no listing can close it"
    );
    assert_eq!(
        state.sidebar.filters,
        BTreeSet::from([TagFilter::Type(ConventionalType::Feat)]),
        "filters are view state, unrelated to which worktrees exist (FR-009)"
    );
    assert_eq!(
        state.sidebar.scroll_offset, 240,
        "and so is scroll position"
    );
    assert_eq!(
        state.session.active, selected,
        "the active session is on `feat-a`, which is still here — a listing that removed a \
         *different* worktree has nothing to say about it"
    );
}

// --- Feature 803: the row a navigation chord reaches, and what clears it (plan D4, U5) ---------

use micold_client::features::sidebar::{current_session_row, row_index, RowRef};

/// Default with one session, `feat-a` open with two sessions, `feat-b` empty.
fn three_locations() -> (State, Vec<micold_core::session::SessionId>) {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("feat-a"), listed("feat-b")]);
    let ids = vec![
        session_at(&mut state, SessionLocation::Default),
        session_at(&mut state, worktree("feat-a")),
        session_at(&mut state, worktree("feat-a")),
    ];
    state.sidebar.expanded.insert("feat-a".to_string());
    (state, ids)
}

#[test]
fn row_index_finds_location_rows_session_sub_rows_and_run_rows() {
    let (mut state, ids) = three_locations();
    let entries = state.sidebar_list();
    // Rows (feature 1718: no group header): 0 Default (one session, no sub-row), 1 feat-a, 2..3
    // its sessions, 4 feat-b.
    assert_eq!(
        row_index(&entries, &RowRef::Location(SessionLocation::Default)),
        Some(0)
    );
    assert_eq!(
        row_index(&entries, &RowRef::Session(ids[0])),
        Some(0),
        "no sub-row: the location's"
    );
    assert_eq!(
        row_index(&entries, &RowRef::Location(worktree("feat-a"))),
        Some(1)
    );
    assert_eq!(
        row_index(&entries, &RowRef::Location(worktree("feat-b"))),
        Some(4)
    );
    let subs: Vec<_> = [ids[1], ids[2]]
        .iter()
        .map(|id| row_index(&entries, &RowRef::Session(*id)).expect("a sub-row"))
        .collect();
    assert_eq!(
        {
            let mut s = subs.clone();
            s.sort_unstable();
            s
        },
        vec![2, 3],
        "the open location's sessions are its sub-rows"
    );
    for id in &ids {
        assert_eq!(
            current_session_row(&entries, Some(*id)),
            row_index(&entries, &RowRef::Session(*id)),
            "current_session_row agrees with row_index"
        );
    }

    // A run group: its row, then its runs' worktree rows.
    deliver(
        &mut state,
        vec![
            listed("feat-a"),
            listed("feat-b"),
            listed("feat-g-1"),
            listed("feat-g-2"),
        ],
    );
    state.update(micold_client::app::Message::Runs(
        micold_client::features::runs::Msg::GroupsChanged(vec![
            support::sidebar_fixture::run_group(1, "g", &["feat-g-1", "feat-g-2"]),
        ]),
    ));
    let entries = state.sidebar_list();
    let group_run = row_index(&entries, &RowRef::Location(worktree("feat-g-2")))
        .expect("an expanded group's run row is drawn");
    let first_run = row_index(&entries, &RowRef::Location(worktree("feat-g-1"))).unwrap();
    assert_eq!(
        group_run,
        first_run + 1,
        "runs are drawn in run order, one row each"
    );
}

#[test]
fn the_reveal_scroll_reaches_the_keyboard_row_when_one_is_set() {
    let mut state = state_with_active();
    let dirs: Vec<String> = (0..30).map(|i| format!("feat-{i:02}")).collect();
    deliver(&mut state, dirs.iter().map(|d| listed(d)).collect());
    let id = session_at(&mut state, SessionLocation::Default);
    state.session.active = Some(id);
    state.sidebar.viewport_height = 200;
    assert_eq!(
        state.reveal_scroll_offset(),
        None,
        "the current session's row (Default, at the top) is already in view"
    );
    state.sidebar.keyboard_row = Some(RowRef::Location(worktree("feat-29")));
    assert!(
        state.reveal_scroll_offset().is_some_and(|y| y > 0),
        "with a keyboard row the reveal scrolls to it, not to the current session's row"
    );
    assert!(
        state.current_session_is_listed(),
        "the target is drawn, so the reveal drains"
    );
    state.sidebar.keyboard_row = Some(RowRef::Location(worktree("gone")));
    assert_eq!(
        state.reveal_scroll_offset(),
        None,
        "a keyboard row the list no longer draws falls back to the current session's row"
    );
}

/// A state with a navigation stop and keyboard row set.
fn with_stop() -> (State, Vec<micold_core::session::SessionId>) {
    let (mut state, ids) = three_locations();
    state.session.active = Some(ids[0]);
    state.sidebar.nav_stop = Some(worktree("feat-b"));
    state.sidebar.keyboard_row = Some(RowRef::Location(worktree("feat-b")));
    (state, ids)
}

#[test]
fn a_current_session_change_clears_the_navigation_stop() {
    let (mut state, ids) = with_stop();
    let outcomes = state.set_current_session(Some(ids[1]));
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(&mut state, o));
    assert_eq!(
        state.sidebar.nav_stop, None,
        "set_current_session leaves the stop"
    );
    assert_eq!(state.sidebar.keyboard_row, None);

    let (mut state, ids) = with_stop();
    state.update(micold_client::app::Message::Session(
        micold_client::features::session::Msg::Selected(ids[1]),
    ));
    assert_eq!(
        state.sidebar.nav_stop, None,
        "a pointer selection leaves the stop"
    );
    assert_eq!(state.sidebar.keyboard_row, None, "and the keyboard row");

    let (mut state, _) = with_stop();
    state.project_entered();
    assert_eq!(
        state.sidebar.nav_stop, None,
        "entering a project leaves the stop"
    );
    assert_eq!(state.sidebar.keyboard_row, None);

    let (mut state, _) = with_stop();
    state.update(micold_client::app::Message::Sidebar(
        SidebarMsg::KeyboardRowLeft,
    ));
    assert_eq!(
        state.sidebar.nav_stop, None,
        "the row losing the keyboard leaves the stop"
    );
    assert_eq!(state.sidebar.keyboard_row, None);
}

// --- Feature 1718: one flat list in the stable order (T002, T006) -----------------------------

mod flat_list {
    use super::support::sidebar_fixture::{
        deliver, five_states, listed, run_group, session_in, state_with_active, worktree,
    };
    use micold_client::app::{Message, State};
    use micold_client::features::overview::location_card_state;
    use micold_client::features::sidebar::{
        location_row, row_heights, row_index, RowRef, SidebarEntry, TagFilter,
    };
    use micold_core::naming::ConventionalType;
    use micold_core::overview::CardState;
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::session::{SessionLifecycle, SessionLocation};
    use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
    use micold_core::store::LoadStatus;
    use micold_core::tokens::density;

    fn location_of(entry: &SidebarEntry) -> SessionLocation {
        match entry {
            SidebarEntry::Default(_) => SessionLocation::Default,
            SidebarEntry::Worktree(node) => node.location(),
            SidebarEntry::Group(group) => {
                SessionLocation::Worktree(format!("group {}", group.name))
            }
        }
    }

    /// The drawn rows' locations, each with the index `row_index` finds it at.
    fn listed_rows(state: &State) -> Vec<(SessionLocation, Option<usize>)> {
        let list = state.sidebar_list();
        list.iter()
            .map(location_of)
            .map(|l| {
                let at = row_index(&list, &RowRef::Location(l.clone()));
                (l, at)
            })
            .collect()
    }

    fn card_of(state: &State, dir: &str) -> CardState {
        location_card_state(state, &worktree(dir), Some(&format!("feat/{dir}"))).0
    }

    /// T002 (US1.1, SC-001, FR-001, FR-003): one location cycles through all five card states and
    /// the drawn list never changes identity or order, and always equals `sidebar_entries()`.
    #[test]
    fn a_location_cycling_through_every_card_state_never_moves() {
        use super::support::sidebar_fixture::pr_read;
        use micold_core::pull_request::{CheckStatus, PrState};
        let mut state = five_states();
        let baseline = listed_rows(&state);
        let mut seen = vec![card_of(&state, "idle")];
        type Step = Box<dyn Fn(&mut State)>;
        let steps: Vec<Step> = vec![
            Box::new(|s| pr_read(s, "feat/idle", PrState::Merged, true)),
            Box::new(|s| {
                s.pr_status.removable.remove("feat/idle");
                pr_read(
                    s,
                    "feat/idle",
                    PrState::Open {
                        checks: CheckStatus::Failing,
                    },
                    false,
                );
            }),
            Box::new(|s| {
                s.pr_status.statuses.remove("feat/idle");
                session_in(
                    s,
                    worktree("idle"),
                    SessionLifecycle::Running,
                    ActivitySignal::Working,
                    false,
                );
            }),
            Box::new(|s| {
                session_in(
                    s,
                    worktree("idle"),
                    SessionLifecycle::Running,
                    ActivitySignal::AwaitingInput,
                    true,
                );
            }),
        ];
        for step in &steps {
            step(&mut state);
            seen.push(card_of(&state, "idle"));
            assert_eq!(listed_rows(&state), baseline, "after {:?}", seen.last());
            let entries: Vec<SessionLocation> =
                state.sidebar_entries().iter().map(location_of).collect();
            let list: Vec<SessionLocation> = state.sidebar_list().iter().map(location_of).collect();
            assert_eq!(list, entries, "the list is the stable order");
        }
        seen.sort_by_key(|c| format!("{c:?}"));
        seen.dedup();
        assert_eq!(seen.len(), 5, "all five card states were visited: {seen:?}");
    }

    /// T002 (US3.1, SC-003): a Done location is listed in the stable order like any other.
    #[test]
    fn a_done_location_is_listed_in_stable_order() {
        let state = five_states();
        assert_eq!(card_of(&state, "done"), CardState::Done);
        let order: Vec<SessionLocation> = state.sidebar_entries().iter().map(location_of).collect();
        let list: Vec<SessionLocation> = state.sidebar_list().iter().map(location_of).collect();
        assert_eq!(list, order);
        assert!(list.contains(&worktree("done")));
        assert_eq!(list.len(), 6, "Default and the five worktrees");
    }

    /// T002 (FR-002): the list holds location entries only, so there is no header row to draw.
    #[test]
    fn the_list_holds_only_location_entries() {
        let state = five_states();
        assert!(state.sidebar_list().iter().all(|e| matches!(
            e,
            SidebarEntry::Default(_) | SidebarEntry::Worktree(_) | SidebarEntry::Group(_)
        )));
    }

    /// T002 (FR-005): a tag filter removes rows without reordering the rest.
    #[test]
    fn a_tag_filter_removes_without_reordering() {
        let mut state = five_states();
        let mut all = state.worktree.worktrees.clone();
        all.push(listed("fix-x"));
        deliver(&mut state, all);
        let before: Vec<SessionLocation> = state.sidebar_list().iter().map(location_of).collect();
        state
            .sidebar
            .filters
            .insert(TagFilter::Type(ConventionalType::Fix));
        let after: Vec<SessionLocation> = state.sidebar_list().iter().map(location_of).collect();
        assert!(after.len() < before.len());
        let kept: Vec<SessionLocation> = before.into_iter().filter(|l| after.contains(l)).collect();
        assert_eq!(after, kept, "the rest keep their relative order");
        assert!(after.contains(&worktree("fix-x")));
    }

    /// T002 (Edge Cases *Run groups*): a run group is one entry, wherever its runs' states go.
    #[test]
    fn a_run_group_is_one_entry_that_stays_put() {
        let mut state = state_with_active();
        deliver(&mut state, vec![listed("g-1"), listed("g-2")]);
        state.update(Message::Runs(
            micold_client::features::runs::Msg::GroupsChanged(vec![run_group(
                1,
                "g",
                &["g-1", "g-2"],
            )]),
        ));
        let before = listed_rows(&state);
        session_in(
            &mut state,
            worktree("g-2"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        assert_eq!(listed_rows(&state), before);
        assert_eq!(
            state
                .sidebar_list()
                .iter()
                .filter(|e| matches!(e, SidebarEntry::Group(_)))
                .count(),
            1
        );
    }

    #[test]
    fn no_project_no_rows() {
        assert!(State::default().sidebar_list().is_empty());
    }

    /// Feature 1125 (Edge Cases): a row whose context menu is open keeps it when its state changes.
    #[test]
    fn a_rows_open_menu_stays_on_it_when_its_state_changes() {
        let mut state = five_states();
        state.update(Message::Worktree(
            micold_client::features::worktree::Msg::MenuToggled("idle".into(), (10, 10)),
        ));
        session_in(
            &mut state,
            worktree("idle"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        assert_eq!(
            state
                .worktree
                .menu_open
                .as_ref()
                .map(|m| m.dir_name.as_str()),
            Some("idle")
        );
    }

    /// T004 (G4): the row metrics have one two-line entry per location row.
    #[test]
    fn row_metrics_have_one_entry_per_location_row() {
        let state = five_states();
        let list = state.sidebar_list();
        let heights = row_heights(&list);
        assert_eq!(heights.len(), list.len());
        let two_line = density::height(density::LIST_ROW_TWO_LINE_BASE, density::DENSE);
        assert!(heights.iter().all(|h| *h == two_line));
        for (i, entry) in list.iter().enumerate() {
            assert_eq!(location_row(&list, &location_of(entry)), Some(i));
        }
        assert_eq!(
            row_index(&list, &RowRef::Location(worktree("done"))),
            location_row(&list, &worktree("done"))
        );
    }

    /// The reveal's offset counts the rows above its row and no header.
    #[test]
    fn the_reveal_offset_counts_rows_only() {
        let mut state = state_with_active();
        let dirs: Vec<String> = (0..30).map(|i| format!("feat-{i:02}")).collect();
        deliver(&mut state, dirs.iter().map(|d| listed(d)).collect());
        state.sidebar.viewport_height = 200;
        let id = session_in(
            &mut state,
            worktree("feat-29"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        state.session.active = Some(id);
        let list = state.sidebar_list();
        let at = location_row(&list, &worktree("feat-28")).expect("drawn");
        let position = state
            .sidebar_entries()
            .iter()
            .position(|e| location_of(e) == worktree("feat-28"));
        assert_eq!(
            Some(at),
            position,
            "the flat index is the stable-order index"
        );
        state.sidebar.keyboard_row = Some(RowRef::Location(worktree("feat-28")));
        assert!(state.reveal_scroll_offset().is_some_and(|y| y > 0));
    }

    /// T006 (FR-008): a settings file written before this change loads unchanged, and a saved one
    /// holds no group choice.
    #[test]
    fn settings_load_unchanged_and_store_no_group_choice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = JsonFileSettingsStore::at(path.clone());
        std::fs::write(&path, r#"{"filter_open": true}"#).unwrap();
        let loaded = store.load();
        assert!(matches!(loaded.status, LoadStatus::Loaded));
        assert!(loaded.settings.filter_open);
        store.save(&loaded.settings).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("group_choices"), "{text}");
    }
}

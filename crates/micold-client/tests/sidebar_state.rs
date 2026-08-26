//! Sidebar hide/show + adjustable-width state (feature 005 UI enhancement).

use micold_client::app::{
    on_escape, Message, State, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
};
use micold_client::features::help;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::sidebar;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::sidebar::{SidebarEntry, TagFilter};
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::worktree_form;
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

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
fn worktree_rename_empty_keeps_prior_name_with_error() {
    let mut state = state_with_active();
    state.update(Message::Worktree(WorktreeMsg::RenameStarted(
        "feat-x".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
        "   ".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::RenameConfirmed));
    // Stays open with an error; no override applied → still the derived name.
    assert_eq!(open_dialog(&state), Some("rename_worktree"));
    assert!(state
        .worktree
        .rename_draft
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
    )));
    assert_eq!(state.worktree.hovered.as_deref(), Some("feat-a"));
    // A stale exit from a different row does not clear the current hover.
    state.update(Message::Worktree(WorktreeMsg::Unhovered(
        "feat-b".to_string(),
    )));
    assert_eq!(state.worktree.hovered.as_deref(), Some("feat-a"));
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

// --- Feature 009: sidebar filter panel toggle ---

#[test]
fn sidebar_filter_panel_starts_closed() {
    let state = State::default();
    assert!(!state.sidebar.filter_open);
}

#[test]
fn sidebar_filter_menu_toggle_opens_and_closes_and_excludes_siblings() {
    let mut state = State {
        help: help::State {
            help_menu_open: true,
            ..Default::default()
        },
        ..Default::default()
    };

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(state.sidebar.filter_open);
    // Opening the filter panel closes the sibling popovers (mutual exclusion, symmetric with
    // the existing help::Msg::MenuToggled/project::Msg::SwitcherToggled pair).
    assert!(!state.help.help_menu_open);
    assert!(!state.project.switcher_open);

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(!state.sidebar.filter_open);
}

#[test]
fn opening_help_menu_or_project_switcher_closes_the_filter_panel() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(state.sidebar.filter_open);

    state.update(Message::Help(HelpMsg::MenuToggled));
    assert!(!state.sidebar.filter_open);

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(state.sidebar.filter_open);

    state.update(Message::Project(ProjectMsg::SwitcherToggled));
    assert!(!state.sidebar.filter_open);
}

#[test]
fn closing_the_filter_panel_never_changes_active_filters() {
    let mut state = State::default();
    let feat = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feat)));
    assert!(state.sidebar.filters.contains(&feat));

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled)); // open
    assert!(state.sidebar.filters.contains(&feat));
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled)); // close
    assert!(
        state.sidebar.filters.contains(&feat),
        "toggling panel visibility must not alter the active filter set (FR-007/FR-008)"
    );
}

#[test]
fn escape_dismisses_the_open_filter_panel_when_no_overlay_is_open() {
    let mut state = State::default();
    assert_eq!(on_escape(&state), None);

    state.sidebar.filter_open = true;
    assert_eq!(
        on_escape(&state),
        Some(Message::Sidebar(SidebarMsg::FilterMenuToggled)),
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
        sidebar: sidebar::State {
            filter_open: true,
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
fn opening_an_overlay_closes_the_filter_panel() {
    // Regression test: previously, opening a modal overlay (e.g. the Add Worktree form) while
    // the filter accordion was open left `filter_open` untouched, so `on_escape` and
    // the live keyboard subscription disagreed about what Escape should dismiss. Every
    // overlay-open now routes through `State::open_overlay`, which resets it unconditionally.
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled));
    assert!(state.sidebar.filter_open);

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    assert!(
        !state.sidebar.filter_open,
        "opening an overlay must close the filter panel"
    );
    assert_eq!(open_dialog(&state), Some("add_worktree"));
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
    let session = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
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
        state.reveal_suppressed_for.is_none(),
        "and nothing about the user's own choices is reset by a background discovery either"
    );
}

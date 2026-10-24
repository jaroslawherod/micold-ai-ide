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
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::PathBuf;

mod support;
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
    // Rows: 0 the Idle header (feature 1125), 1 Default (one session, no sub-row), 2 feat-a, 3..4
    // its sessions, 5 feat-b.
    assert_eq!(
        row_index(&entries, &RowRef::Location(SessionLocation::Default)),
        Some(1)
    );
    assert_eq!(
        row_index(&entries, &RowRef::Session(ids[0])),
        Some(1),
        "no sub-row: the location's"
    );
    assert_eq!(
        row_index(&entries, &RowRef::Location(worktree("feat-a"))),
        Some(2)
    );
    assert_eq!(
        row_index(&entries, &RowRef::Location(worktree("feat-b"))),
        Some(5)
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
        vec![3, 4],
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

// --- Feature 1125: the sidebar grouped by state (T005, T006, T007) ----------------------------

mod state_groups {
    use super::support::sidebar_fixture::{
        deliver, five_states, listed, run_group, session_in, state_with_active, worktree,
    };
    use micold_client::app::{Message, State};
    use micold_client::features::overview::location_card_state;
    use micold_client::features::sidebar::{
        location_row, row_heights, row_index, GroupHeaderRow, ListRow, Msg as SidebarMsg, RowRef,
        SidebarEntry, TagFilter,
    };
    use micold_core::naming::ConventionalType;
    use micold_core::protocol::messages::ActivitySignal;
    use micold_core::session::{SessionLifecycle, SessionLocation};
    use micold_core::sidebar_groups::StateGroup;
    use micold_core::tokens::density;
    use std::path::PathBuf;

    fn location_of(entry: &SidebarEntry) -> SessionLocation {
        match entry {
            SidebarEntry::Default(_) => SessionLocation::Default,
            SidebarEntry::Worktree(node) => node.location(),
            SidebarEntry::Group(group) => {
                SessionLocation::Worktree(format!("group {}", group.name))
            }
        }
    }

    /// Each group and the locations in it, in order.
    fn grouped(state: &State) -> Vec<(StateGroup, Vec<SessionLocation>)> {
        state
            .sidebar_groups()
            .iter()
            .map(|g| (g.group, g.entries.iter().map(location_of).collect()))
            .collect()
    }

    fn header(state: &State, group: StateGroup) -> Option<GroupHeaderRow> {
        state.sidebar_list().into_iter().find_map(|row| match row {
            ListRow::Header(h) if h.group == group => Some(h),
            _ => None,
        })
    }

    fn toggle(state: &mut State, group: StateGroup) {
        state.update(Message::Sidebar(SidebarMsg::GroupToggled(group)));
    }

    /// T005 (US1.1, US1.2, US1.5, SC-001, FR-001, FR-003): groups in order, Needs attention with
    /// Needs action in `sidebar_entries()` order, *Default* first in its group.
    #[test]
    fn the_groups_come_in_state_order_with_todays_order_inside() {
        let state = five_states();
        let order: Vec<SessionLocation> = state.sidebar_entries().iter().map(location_of).collect();
        let groups = grouped(&state);
        assert_eq!(
            groups.iter().map(|(g, _)| *g).collect::<Vec<_>>(),
            StateGroup::ALL.to_vec()
        );
        let in_order = |a: &SessionLocation, b: &SessionLocation| {
            order.iter().position(|l| l == a) < order.iter().position(|l| l == b)
        };
        let needs_action = &groups[0].1;
        assert_eq!(needs_action.len(), 2);
        assert!(
            needs_action.contains(&worktree("act")) && needs_action.contains(&worktree("attn"))
        );
        assert!(in_order(&needs_action[0], &needs_action[1]));
        assert_eq!(groups[1].1, vec![worktree("work")]);
        assert_eq!(
            groups[2].1,
            vec![SessionLocation::Default, worktree("idle")]
        );
        assert_eq!(groups[3].1, vec![worktree("done")]);
        let total: usize = groups.iter().map(|(_, l)| l.len()).sum();
        assert_eq!(total, order.len(), "every entry in exactly one group");
    }

    /// T005 (US1.7): each entry's group is the group of its Overview card's state.
    #[test]
    fn each_entrys_group_is_its_card_states() {
        let state = five_states();
        for group in state.sidebar_groups() {
            for entry in &group.entries {
                let SidebarEntry::Worktree(node) = entry else {
                    continue;
                };
                let (card, _) =
                    location_card_state(&state, &node.location(), node.worktree.branch.as_deref());
                assert_eq!(StateGroup::of(card), group.group, "{:?}", node.location());
            }
        }
    }

    /// T005 (US1.4, FR-002): a group whose only worktree a filter hides is absent.
    #[test]
    fn a_group_a_filter_empties_is_not_shown() {
        let mut state = five_states();
        let mut other = state.worktree.worktrees.clone();
        other.push(listed("fix-x"));
        deliver(&mut state, other);
        state
            .sidebar
            .filters
            .insert(TagFilter::Type(ConventionalType::Fix));
        let groups: Vec<StateGroup> = grouped(&state).into_iter().map(|(g, _)| g).collect();
        assert_eq!(
            groups,
            vec![StateGroup::Idle],
            "Default and fix-x, both Idle"
        );
        assert_eq!(
            state
                .sidebar_list()
                .iter()
                .filter(|r| matches!(r, ListRow::Header(_)))
                .count(),
            1
        );
    }

    /// T005 (Edge Cases *Run groups*): a run group sits in the group of its runs' worst state and
    /// counts as one entry.
    #[test]
    fn a_run_group_is_one_entry_in_its_worst_runs_group() {
        let mut state = state_with_active();
        deliver(&mut state, vec![listed("g-1"), listed("g-2")]);
        session_in(
            &mut state,
            worktree("g-2"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        state.update(Message::Runs(
            micold_client::features::runs::Msg::GroupsChanged(vec![run_group(
                1,
                "g",
                &["g-1", "g-2"],
            )]),
        ));
        let working = state
            .sidebar_groups()
            .into_iter()
            .find(|g| g.group == StateGroup::Working)
            .expect("the run group is Working");
        assert_eq!(working.entries.len(), 1);
        assert!(matches!(working.entries[0], SidebarEntry::Group(_)));
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.count),
            Some(1)
        );
    }

    /// T005 (US1.6, FR-004, SC-007): a session starting to work moves its worktree from Idle to
    /// Working in the same state, and both counts follow.
    #[test]
    fn a_state_change_moves_the_row_and_both_counts() {
        let mut state = five_states();
        assert_eq!(header(&state, StateGroup::Idle).map(|h| h.count), Some(2));
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.count),
            Some(1)
        );
        session_in(
            &mut state,
            worktree("idle"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        assert_eq!(header(&state, StateGroup::Idle).map(|h| h.count), Some(1));
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.count),
            Some(2)
        );
        assert!(grouped(&state)[1].1.contains(&worktree("idle")));
    }

    /// T005 (US4.8, feature 024 FR-012a): the current session's worktree a filter excludes stays
    /// listed, in its state's group.
    #[test]
    fn the_current_sessions_worktree_survives_a_filter_in_its_group() {
        let mut state = five_states();
        let id = state
            .active_sessions()
            .iter()
            .find(|s| s.location == worktree("work"))
            .map(|s| s.id);
        state.session.active = id;
        state
            .sidebar
            .filters
            .insert(TagFilter::Type(ConventionalType::Fix));
        let groups = grouped(&state);
        assert!(groups
            .iter()
            .any(|(g, l)| *g == StateGroup::Working && l == &vec![worktree("work")]));
    }

    #[test]
    fn no_project_no_groups() {
        assert!(State::default().sidebar_groups().is_empty());
        assert!(State::default().sidebar_list().is_empty());
    }

    /// T006 (US1.3, FR-005): a fresh project lists each open group's header and entries, and the
    /// Done header alone.
    #[test]
    fn done_starts_collapsed_and_the_others_open() {
        let state = five_states();
        let list = state.sidebar_list();
        let headers: Vec<GroupHeaderRow> = list
            .iter()
            .filter_map(|r| match r {
                ListRow::Header(h) => Some(*h),
                ListRow::Entry(_) => None,
            })
            .collect();
        assert_eq!(
            headers
                .iter()
                .map(|h| (h.group, h.count, h.open))
                .collect::<Vec<_>>(),
            vec![
                (StateGroup::NeedsAction, 2, true),
                (StateGroup::Working, 1, true),
                (StateGroup::Idle, 2, true),
                (StateGroup::Done, 1, false),
            ]
        );
        assert!(
            matches!(list.last(), Some(ListRow::Header(h)) if h.group == StateGroup::Done),
            "nothing is drawn under the closed Done header"
        );
        assert_eq!(list.len(), 4 + 2 + 1 + 2);
    }

    /// T006 (G5, Edge Cases): Done holding the current session starts open; a press closes it.
    #[test]
    fn done_holding_the_current_session_is_open_and_one_press_closes_it() {
        let mut state = five_states();
        // A session at Done that is not working keeps Done the worktree's state.
        let done_session = session_in(
            &mut state,
            worktree("done"),
            SessionLifecycle::Idle,
            ActivitySignal::AwaitingInput,
            false,
        );
        state.session.active = Some(done_session);
        assert_eq!(header(&state, StateGroup::Done).map(|h| h.open), Some(true));
        toggle(&mut state, StateGroup::Done);
        assert_eq!(
            header(&state, StateGroup::Done).map(|h| h.open),
            Some(false),
            "toggled against the effective state (R4)"
        );
        toggle(&mut state, StateGroup::Done);
        assert_eq!(header(&state, StateGroup::Done).map(|h| h.open), Some(true));
    }

    /// T006 (US5.2): a closed Done opens and an open Working closes.
    #[test]
    fn a_header_press_flips_its_group() {
        let mut state = five_states();
        toggle(&mut state, StateGroup::Done);
        assert_eq!(header(&state, StateGroup::Done).map(|h| h.open), Some(true));
        toggle(&mut state, StateGroup::Working);
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.open),
            Some(false)
        );
        assert!(!state
            .sidebar_list()
            .iter()
            .any(|r| matches!(r, ListRow::Entry(SidebarEntry::Worktree(n)) if n.location() == worktree("work"))));
    }

    /// T006 (US5.3, FR-005): the choice survives rows moving, a re-read, and a project switch and
    /// back, and is per project.
    #[test]
    fn a_choice_holds_for_the_run_per_project() {
        let mut state = five_states();
        toggle(&mut state, StateGroup::Working);
        // A row moves into Working.
        session_in(
            &mut state,
            worktree("idle"),
            SessionLifecycle::Running,
            ActivitySignal::Working,
            false,
        );
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| (h.count, h.open)),
            Some((2, false))
        );
        // A re-read of the worktree list.
        let worktrees = state.worktree.worktrees.clone();
        deliver(&mut state, worktrees);
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.open),
            Some(false)
        );
        // Another project, where nothing was chosen, and back.
        let home = state.workspace.active.clone();
        state.workspace.active = Some(PathBuf::from("/other"));
        assert!(!state
            .sidebar
            .group_choices
            .contains_key(&PathBuf::from("/other")));
        state.workspace.active = home;
        assert_eq!(
            header(&state, StateGroup::Working).map(|h| h.open),
            Some(false)
        );
        assert_eq!(
            state.sidebar.group_choices.values().count(),
            1,
            "one project chose"
        );
    }

    /// T006 (Edge Cases): a row whose context menu is open keeps it when it changes group.
    #[test]
    fn a_rows_open_menu_stays_on_it_when_it_moves() {
        let mut state = five_states();
        state.update(Message::Worktree(
            micold_client::features::worktree::Msg::MenuToggled("idle".into(), (10, 10)),
        ));
        assert_eq!(
            state
                .worktree
                .menu_open
                .as_ref()
                .map(|m| m.dir_name.as_str()),
            Some("idle")
        );
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

    /// T007 (S7): a header is one dense one-line row, and the rows of a closed group are absent.
    #[test]
    fn row_metrics_count_headers_and_skip_closed_groups() {
        let state = five_states();
        let list = state.sidebar_list();
        let heights = row_heights(&list);
        assert_eq!(
            heights.len(),
            list.len(),
            "no location here draws session rows"
        );
        let one_line = density::height(density::LIST_ROW_BASE, density::DENSE);
        for (row, height) in list.iter().zip(&heights) {
            if matches!(row, ListRow::Header(_)) {
                assert_eq!(*height, one_line);
            }
        }
        assert_eq!(location_row(&list, &worktree("act")).map(|_| ()), Some(()));
        assert_eq!(
            location_row(&list, &worktree("done")),
            None,
            "Done is closed"
        );
        assert_eq!(row_index(&list, &RowRef::Location(worktree("done"))), None);
        // The first location row sits under its header.
        let first = location_row(&list, &worktree("attn"))
            .min(location_row(&list, &worktree("act")))
            .expect("drawn");
        assert_eq!(first, 1, "after the Needs action header");
    }

    /// T007 (S7): the reveal's offset counts the headers above its row.
    #[test]
    fn the_reveal_offset_counts_headers() {
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
        // feat-29 is Working; the others and Default are Idle below it.
        let list = state.sidebar_list();
        assert_eq!(location_row(&list, &worktree("feat-29")), Some(1));
        state.sidebar.keyboard_row = Some(RowRef::Location(worktree("feat-28")));
        let at = location_row(&list, &worktree("feat-28")).expect("drawn");
        assert_eq!(
            at,
            2 + 1 + 29,
            "Working header, feat-29, Idle header, Default, feat-00…"
        );
        assert!(state.reveal_scroll_offset().is_some_and(|y| y > 0));
    }
}

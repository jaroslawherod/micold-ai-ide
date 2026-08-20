//! The sidebar's rows and tag filters, exercised in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module and the domain types its API mentions. It builds no
//! `State`, references no other feature's types, and needs no application shell.
//!
//! The sidebar is a harder isolation case than the worktree form: its rows are *projections* of
//! worktrees and sessions, so it necessarily names `micold_core` domain types. That is fine and is
//! the distinction SC-004 draws — depending on the shared domain is not the same as depending on
//! another feature. What must not appear here is another *feature's* state: no project switcher, no
//! settings draft, no session helpers.
//!
//! If a later change makes this file need one of those to compile, the sidebar's boundary has
//! eroded.

use micold_client::features::sidebar::{
    current_session_row, effective_open, filters_from_env_value, matches_filters, row_heights,
    scroll_target, worktree_location_label, DefaultNode, SidebarEntry, TagFilter, WorktreeNode,
    DEFAULT_LOCATION_LABEL, FILTER_ENV_VAR,
};
use micold_core::naming::{ConventionalType, Tag};
use micold_core::session::{Session, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn worktree(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.into(),
        path: Path::new("/p/.claude/worktrees").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

fn filters(of: impl IntoIterator<Item = TagFilter>) -> BTreeSet<TagFilter> {
    of.into_iter().collect()
}

#[test]
fn no_active_filter_admits_everything() {
    let none = BTreeSet::new();

    assert!(
        matches_filters(&[], &none),
        "an unfiltered sidebar hides nothing, not even an untagged row"
    );
    assert!(
        matches_filters(&[Tag::Type(ConventionalType::Fix)], &none),
        "an unfiltered sidebar hides nothing"
    );
}

#[test]
fn active_filters_combine_with_or_rather_than_and() {
    let both = filters([TagFilter::Type(ConventionalType::Feat), TagFilter::HasIssue]);

    assert!(
        matches_filters(&[Tag::Type(ConventionalType::Feat)], &both),
        "satisfying one active filter is enough — requiring all of them would make most \
         two-filter selections show an empty sidebar"
    );
}

#[test]
fn the_untyped_filter_selects_the_rows_the_type_filters_cannot_reach() {
    let untyped = filters([TagFilter::Untyped]);

    assert!(
        matches_filters(&[Tag::Issue("ABC-1".into())], &untyped),
        "a row with an issue but no type is exactly what Untyped is for"
    );
    assert!(
        !matches_filters(&[Tag::Type(ConventionalType::Docs)], &untyped),
        "a typed row is not untyped"
    );
}

#[test]
fn a_worktrees_location_reads_relative_to_the_project_it_belongs_to() {
    let label = worktree_location_label(Path::new("/p"), &worktree("feat-a"));

    assert!(
        !label.starts_with("/p"),
        "the tooltip says where a worktree sits inside the project, so the project's own path is \
         noise: got {label:?}"
    );
}

#[test]
fn a_worktree_outside_the_project_still_gets_a_label() {
    let mut stray = worktree("feat-a");
    stray.path = PathBuf::from("/elsewhere/feat-a");

    assert_eq!(
        worktree_location_label(Path::new("/p"), &stray),
        "/elsewhere/feat-a",
        "an unrelatable path falls back to itself — a tooltip that renders nothing would be worse \
         than one that renders an absolute path"
    );
}

#[test]
fn a_sidebar_row_is_either_a_worktree_or_the_project_root_and_never_both() {
    let rows = [
        SidebarEntry::Default(DefaultNode {
            display_name: "Default",
            expanded: false,
            sessions: Vec::new(),
        }),
        SidebarEntry::Worktree(WorktreeNode {
            worktree: worktree("feat-a"),
            display_name: "a".into(),
            tags: vec![Tag::Type(ConventionalType::Feat)],
            expanded: false,
            sessions: Vec::new(),
            shown_for_current_session: false,
        }),
    ];

    let default_rows = rows
        .iter()
        .filter(|r| matches!(r, SidebarEntry::Default(_)))
        .count();

    assert_eq!(
        default_rows, 1,
        "the project root is one row among worktree rows, distinguishable by variant rather than \
         by inspecting a name"
    );
    assert_eq!(
        DEFAULT_LOCATION_LABEL, "Project root",
        "the project-root row's location never varies, so its label is a constant"
    );
}

// --- Feature 024: which rows the panel shows open -------------------------------------------
//
// `effective_open` is the whole of contract §1.1, reduced to the three booleans that decide it.
// Kept a free function for the same reason `matches_filters` is one: the rule is worth stating
// without a `State` to state it against, and this file is where that is checked (SC-004).

#[test]
fn a_row_the_user_opened_stays_open_whatever_else_is_true() {
    for holds_current in [false, true] {
        for suppressed in [false, true] {
            assert!(
                effective_open(true, holds_current, suppressed),
                "the user's own expansion is not something the app may override \
                 (holds_current={holds_current}, suppressed={suppressed})"
            );
        }
    }
}

#[test]
fn the_location_holding_the_current_session_is_open_without_the_user_opening_it() {
    assert!(
        effective_open(false, true, false),
        "this is the whole feature: the row holding the session you were moved to is listed \
         without you having to find it"
    );
}

#[test]
fn a_user_collapse_closes_the_row_the_app_opened() {
    assert!(
        !effective_open(false, true, true),
        "a row the user closed stays closed — FR-005, and the reason suppression exists at all"
    );
}

#[test]
fn a_location_holding_no_current_session_is_open_only_if_the_user_opened_it() {
    assert!(
        !effective_open(false, false, false),
        "nothing opens a row that neither the user nor the current session asked for"
    );
}

// --- Feature 024: getting the revealed row on screen ------------------------------------------
//
// The sidebar has to answer "is that row visible, and if not where should the list sit" without a
// renderer: iced 0.14 has no scroll-child-into-view operation and reports no child position. Row
// heights are deterministic, so the answer is arithmetic — and arithmetic is checkable here, which
// is the whole reason these are functions over the projection rather than code inside the view.

fn session(location: SessionLocation) -> Session {
    Session::start_new(location)
}

fn default_entry(sessions: Vec<Session>) -> SidebarEntry {
    SidebarEntry::Default(DefaultNode {
        display_name: "Default",
        expanded: !sessions.is_empty(),
        sessions,
    })
}

fn worktree_entry(dir: &str, tags: Vec<Tag>, sessions: Vec<Session>) -> SidebarEntry {
    SidebarEntry::Worktree(WorktreeNode {
        worktree: worktree(dir),
        display_name: dir.to_string(),
        tags,
        expanded: !sessions.is_empty(),
        sessions,
        shown_for_current_session: false,
    })
}

/// The two figures `ui/material/anatomy_size.rs` asserts against the rendered tree, at the sidebar's
/// own density. Shared rather than restated: a metric that agreed with a copy of the numbers
/// instead of with the tokens would drift the moment the density did, and drift silently.
fn one_line() -> f32 {
    micold_core::tokens::density::height(
        micold_core::tokens::density::LIST_ROW_BASE,
        micold_client::features::sidebar::SIDEBAR_DENSITY,
    )
}

fn two_line() -> f32 {
    micold_core::tokens::density::height(
        micold_core::tokens::density::LIST_ROW_TWO_LINE_BASE,
        micold_client::features::sidebar::SIDEBAR_DENSITY,
    )
}

#[test]
fn a_tagged_worktree_row_is_a_two_line_row_and_a_session_row_is_not() {
    let entries = vec![
        default_entry(vec![]),
        worktree_entry("feat-a", vec![Tag::Type(ConventionalType::Feat)], vec![]),
        worktree_entry("plain", vec![], vec![]),
    ];

    assert_eq!(
        row_heights(&entries),
        vec![one_line(), two_line(), one_line()],
        "a row's height follows its line count and the sidebar's density, and nothing else — the \
         same rule `TreeView` renders by. A metric that disagreed with the rendered height would \
         scroll to the wrong place and say nothing about it"
    );
}

#[test]
fn an_open_locations_sessions_are_rows_and_a_closed_ones_are_not() {
    let with_sessions = vec![worktree_entry(
        "feat-a",
        vec![],
        vec![session(SessionLocation::Worktree("feat-a".into()))],
    )];
    assert_eq!(row_heights(&with_sessions).len(), 2);

    let closed = vec![SidebarEntry::Worktree(WorktreeNode {
        worktree: worktree("feat-a"),
        display_name: "feat-a".to_string(),
        tags: vec![],
        expanded: false,
        sessions: vec![session(SessionLocation::Worktree("feat-a".into()))],
        shown_for_current_session: false,
    })];
    assert_eq!(
        row_heights(&closed).len(),
        1,
        "a closed location draws no session rows, so they occupy no height — measuring them anyway \
         would put every row below it out by their total"
    );
}

#[test]
fn an_already_visible_row_is_not_scrolled_to() {
    let heights = vec![40.0, 40.0, 40.0];

    assert_eq!(
        scroll_target(&heights, 1, 200.0, 0.0),
        None,
        "the list does not move under the user when it did not have to (FR-009, SC-007)"
    );
}

#[test]
fn a_row_below_the_fold_is_brought_just_into_view() {
    // Four 40dp rows with 4dp between them: tops at 0, 44, 88, 132.
    let heights = vec![40.0; 4];

    assert_eq!(
        scroll_target(&heights, 3, 100.0, 0.0),
        Some(72.0),
        "the minimal move that brings the row fully in — its bottom (172) less the viewport (100). \
         Not centred: the smallest movement is the one least likely to disturb what the user was \
         reading"
    );
}

#[test]
fn a_row_above_the_fold_is_brought_back_by_scrolling_up() {
    let heights = vec![40.0; 4];

    assert_eq!(
        scroll_target(&heights, 0, 100.0, 132.0),
        Some(0.0),
        "scrolling up stops at the row's top rather than its bottom, which is the same 'minimal' \
         rule seen from the other side"
    );
}

#[test]
fn nothing_is_scrolled_before_the_first_layout() {
    let heights = vec![40.0; 4];

    assert_eq!(
        scroll_target(&heights, 3, 0.0, 0.0),
        None,
        "a viewport of zero height means 'not laid out yet', never 'nothing fits' — scrolling on \
         that reading would jump the list on the frame before it knew its own size (contract §6.3)"
    );
}

#[test]
fn a_target_beyond_the_lists_end_is_clamped_to_it() {
    let heights = vec![40.0; 3];

    // Content is 128 tall (3×40 + 2×4) in a 200 viewport: everything already fits.
    assert_eq!(
        scroll_target(&heights, 2, 200.0, 0.0),
        None,
        "a list shorter than its viewport has nowhere to scroll to, and an unclamped target would \
         ask for an offset the scrollable would refuse"
    );
}

#[test]
fn a_row_that_is_not_there_is_not_scrolled_to() {
    let heights = vec![40.0; 3];

    assert_eq!(
        scroll_target(&heights, 7, 100.0, 0.0),
        None,
        "the index comes from a projection that may not hold the current session's row yet — an \
         out-of-range index is that case, not a bug to panic on (research R7)"
    );
}

#[test]
fn the_current_sessions_row_is_found_by_walking_the_rows_as_drawn() {
    let default_session = session(SessionLocation::Default);
    let worktree_session = session(SessionLocation::Worktree("feat-a".into()));
    let wanted = worktree_session.id;
    let entries = vec![
        default_entry(vec![default_session]),
        worktree_entry("feat-a", vec![], vec![worktree_session]),
    ];

    // Rows as drawn: Default, its session, feat-a, its session.
    assert_eq!(
        current_session_row(&entries, Some(wanted)),
        Some(3),
        "the index is a position in the rendered list, so it has to be counted the way the list is \
         built — locations and the sessions of the open ones, in order"
    );
    assert_eq!(
        current_session_row(&entries, None),
        None,
        "and with no current session there is no row to scroll to (FR-013)"
    );
}

// --- the §B5 test hook (MICOLD_SIDEBAR_FILTER) ------------------------------------------------
//
// Parsing only. What the value is *applied to* is `Message::SidebarFilterToggled`, which the rest
// of this file and `sidebar_state.rs` already cover — the hook deliberately owns no state of its
// own, so that what a visual pass then photographs is the real filter.

/// Absent, empty, or whitespace: the ordinary launch. This is the case every developer who has
/// never heard of the hook is in, so it must not be a failure.
#[test]
fn no_value_means_no_filters() {
    assert_eq!(filters_from_env_value(None), Ok(Vec::new()));
    assert_eq!(filters_from_env_value(Some("")), Ok(Vec::new()));
    assert_eq!(filters_from_env_value(Some("   ")), Ok(Vec::new()));
}

#[test]
fn a_conventional_type_parses_to_its_filter() {
    assert_eq!(
        filters_from_env_value(Some("fix")),
        Ok(vec![TagFilter::Type(ConventionalType::Fix)])
    );
}

/// The two filters that are not types. `untyped` is the one §B5 step 4 leans on.
#[test]
fn issue_and_untyped_are_spelled_out() {
    assert_eq!(
        filters_from_env_value(Some("issue")),
        Ok(vec![TagFilter::HasIssue])
    );
    assert_eq!(
        filters_from_env_value(Some("untyped")),
        Ok(vec![TagFilter::Untyped])
    );
}

/// Several filters, in the order given, with whitespace tolerated — a command line is typed by
/// hand, and `fix, docs` should not be a different value from `fix,docs`.
#[test]
fn a_list_parses_in_order() {
    assert_eq!(
        filters_from_env_value(Some(" fix , docs ,, issue ")),
        Ok(vec![
            TagFilter::Type(ConventionalType::Fix),
            TagFilter::Type(ConventionalType::Docs),
            TagFilter::HasIssue,
        ])
    );
}

/// A typo is refused, not ignored. An unfiltered panel that was *asked* to be filtered is the one
/// outcome that would be recorded as evidence and be wrong.
#[test]
fn an_unknown_token_is_an_error_naming_itself_and_the_variable() {
    let err = filters_from_env_value(Some("feature")).expect_err("expected a refusal");
    assert!(
        err.contains(FILTER_ENV_VAR),
        "{err:?} does not name the variable"
    );
    assert!(
        err.contains("feature"),
        "{err:?} does not name the bad token"
    );
    assert!(
        err.contains("untyped"),
        "{err:?} does not state the grammar"
    );
}

/// One bad token spoils the list: a half-applied filter is a state nobody asked for, and it would
/// photograph as a successful pass of a filter that is not the one requested.
#[test]
fn a_bad_token_among_good_ones_refuses_the_whole_value() {
    assert!(filters_from_env_value(Some("fix,nonsense,docs")).is_err());
}

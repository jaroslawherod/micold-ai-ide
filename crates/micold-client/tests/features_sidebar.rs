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
    effective_open, matches_filters, worktree_location_label, DefaultNode, SidebarEntry, TagFilter,
    WorktreeNode, DEFAULT_LOCATION_LABEL,
};
use micold_core::naming::{ConventionalType, Tag};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn worktree(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.into(),
        path: Path::new("/p/.claude/worktrees").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
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

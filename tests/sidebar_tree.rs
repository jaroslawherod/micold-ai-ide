//! T017 — sidebar tree building + expand/collapse (FR-002/003).

use micold_ai_ide::app::{Message, SidebarEntry, State, TagFilter};
use micold_ai_ide::naming::{ConventionalType, Tag};
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::{Session, SessionLocation};
use micold_ai_ide::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn worktree(dir: &str, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status,
    }
}

fn state_with_active_project() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = vec![
        worktree("feat-a", WorktreeStatus::Valid),
        worktree("feat-b", WorktreeStatus::Valid),
    ];
    // A session on feat-a.
    state.workspace.sessions.insert(
        path,
        vec![Session::start_new(SessionLocation::Worktree(
            "feat-a".to_string(),
        ))],
    );
    state
}

#[test]
fn tree_has_a_node_per_worktree_collapsed_by_default() {
    let state = state_with_active_project();
    let tree = state.worktree_tree();
    assert_eq!(tree.len(), 2);
    assert!(tree.iter().all(|n| !n.expanded));
    assert_eq!(tree[0].worktree.dir_name, "feat-a");
}

#[test]
fn sessions_are_joined_to_their_worktree_by_dir_name() {
    let state = state_with_active_project();
    let tree = state.worktree_tree();
    let feat_a = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    let feat_b = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-b")
        .unwrap();
    assert_eq!(feat_a.sessions.len(), 1);
    assert_eq!(feat_b.sessions.len(), 0);
}

#[test]
fn toggling_expands_then_collapses() {
    let mut state = state_with_active_project();
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    let expanded = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap()
        .expanded;
    assert!(expanded);

    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    let collapsed = !state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap()
        .expanded;
    assert!(collapsed);
}

#[test]
fn reloading_worktrees_drops_stale_expansion_state() {
    let mut state = state_with_active_project();
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    // Reload without feat-a.
    state.update(Message::WorktreesLoaded(vec![worktree(
        "feat-b",
        WorktreeStatus::Valid,
    )]));
    assert!(!state.expanded.contains("feat-a"));
}

// --- Feature 008 US1: display name + tags per worktree ---

fn state_with_named_worktrees(dirs: &[(&str, WorktreeStatus)]) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktrees = dirs.iter().map(|(d, s)| worktree(d, *s)).collect();
    state
}

fn node<'a>(
    state: &'a [micold_ai_ide::app::WorktreeNode],
    dir: &str,
) -> &'a micold_ai_ide::app::WorktreeNode {
    state.iter().find(|n| n.worktree.dir_name == dir).unwrap()
}

#[test]
fn worktree_node_exposes_type_and_issue_tags() {
    let state = state_with_named_worktrees(&[("feat-abc-123-login-page", WorktreeStatus::Valid)]);
    let tree = state.worktree_tree();
    let n = node(&tree, "feat-abc-123-login-page");
    assert_eq!(
        n.tags,
        vec![
            Tag::Type(ConventionalType::Feat),
            Tag::Issue("ABC-123".to_string())
        ]
    );
}

#[test]
fn non_valid_worktree_gets_status_tag() {
    // US5 / FR-011: missing/invalid worktrees carry a status tag (the cue that replaces the
    // removed git icon); valid ones do not.
    let state = state_with_named_worktrees(&[
        ("feat-ok", WorktreeStatus::Valid),
        ("feat-gone", WorktreeStatus::Missing),
        ("orphan-dir", WorktreeStatus::Invalid),
    ]);
    let tree = state.worktree_tree();
    assert!(node(&tree, "feat-ok")
        .tags
        .iter()
        .all(|t| !matches!(t, Tag::Status(_))));
    assert!(node(&tree, "feat-gone")
        .tags
        .contains(&Tag::Status(WorktreeStatus::Missing)));
    assert!(node(&tree, "orphan-dir")
        .tags
        .contains(&Tag::Status(WorktreeStatus::Invalid)));
}

#[test]
fn worktree_node_type_only_and_untyped() {
    let state = state_with_named_worktrees(&[
        ("fix-crash-on-open", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ]);
    let tree = state.worktree_tree();
    assert_eq!(
        node(&tree, "fix-crash-on-open").tags,
        vec![Tag::Type(ConventionalType::Fix)]
    );
    assert!(node(&tree, "my-experiment").tags.is_empty());
}

#[test]
fn worktree_node_display_name_derived_when_no_override() {
    let state = state_with_named_worktrees(&[
        ("feat-abc-123-login-page", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ]);
    let tree = state.worktree_tree();
    assert_eq!(
        node(&tree, "feat-abc-123-login-page").display_name,
        "Login page"
    );
    assert_eq!(node(&tree, "my-experiment").display_name, "My experiment");
}

// --- Feature 010 US2: location tooltip text (FR-010) ---

// T019: the worktree location label is the worktree's path relative to the project root
// (research.md R6 — Path::strip_prefix, since every worktree always lives directly under
// `<project_root>/.claude/worktrees/`).
#[test]
fn worktree_location_label_is_relative_to_project_root() {
    let root = PathBuf::from("/repo");
    let wt = worktree("feat-a", WorktreeStatus::Valid);
    assert_eq!(
        micold_ai_ide::app::worktree_location_label(&root, &wt),
        ".claude/worktrees/feat-a"
    );
}

#[test]
fn default_location_label_is_a_fixed_project_root_string() {
    assert_eq!(micold_ai_ide::app::DEFAULT_LOCATION_LABEL, "Project root");
}

// --- Feature 008 US4: tag filtering ---

fn dirs(tree: &[micold_ai_ide::app::WorktreeNode]) -> Vec<String> {
    tree.iter().map(|n| n.worktree.dir_name.clone()).collect()
}

fn filtered_state() -> State {
    state_with_named_worktrees(&[
        ("feat-abc-123-login", WorktreeStatus::Valid),
        ("fix-crash", WorktreeStatus::Valid),
        ("fix-def-9-thing", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ])
}

#[test]
fn empty_filter_shows_all() {
    let state = filtered_state();
    assert_eq!(state.filtered_worktree_tree().len(), 4);
}

#[test]
fn type_filter_selects_only_that_type() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-crash", "fix-def-9-thing"]
    );
}

#[test]
fn filtered_tree_is_unaffected_by_the_filter_panels_open_state() {
    // Feature 009 FR-007/FR-008: showing/hiding the filter panel is purely a display change and
    // must never affect which worktrees are considered filtered.
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    let expected = dirs(&state.filtered_worktree_tree());

    state.update(Message::SidebarFilterMenuToggled); // open
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
    state.update(Message::SidebarFilterMenuToggled); // close
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
}

#[test]
fn filters_combine_with_or() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    )));
    state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
    // feat + untyped ⇒ the feat worktree and the non-conforming one.
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123-login", "my-experiment"]
    );
}

#[test]
fn has_issue_filter_selects_issue_bearing() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::HasIssue));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123-login", "fix-def-9-thing"]
    );
}

#[test]
fn untyped_filter_selects_non_conforming() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
    assert_eq!(dirs(&state.filtered_worktree_tree()), vec!["my-experiment"]);
}

#[test]
fn available_filters_reflect_present_tags() {
    let filters = filtered_state().available_tag_filters();
    assert!(filters.contains(&TagFilter::Type(ConventionalType::Feat)));
    assert!(filters.contains(&TagFilter::Type(ConventionalType::Fix)));
    assert!(filters.contains(&TagFilter::HasIssue));
    assert!(filters.contains(&TagFilter::Untyped));
    // No `chore` worktree ⇒ no chore filter offered.
    assert!(!filters.contains(&TagFilter::Type(ConventionalType::Chore)));
}

#[test]
fn filter_recomputes_after_delete(/* FR-028 / C1 */) {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    // Remove one fix worktree via the delete reducer path.
    state.update(Message::WorktreeDeleteRequested("fix-crash".to_string()));
    state.update(Message::WorktreeDeleteConfirmed);
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-def-9-thing"]
    );
}

// --- Feature 010 US1/US2: the "Default" (project-root) sidebar entry ---

// T014: the Default entry is always present for an open project, ahead of any worktree
// entries, and absent when no project is open (contracts/sidebar-default-entry.md
// invariant 1, previously untested).

#[test]
fn default_entry_present_and_first_even_with_no_worktrees() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    // No worktrees at all.
    state.worktrees = vec![];

    let entries = state.sidebar_entries();
    assert_eq!(entries.len(), 1, "Default entry alone, no worktrees yet");
    assert!(matches!(entries[0], SidebarEntry::Default(_)));
}

#[test]
fn default_entry_precedes_worktree_entries() {
    let state = state_with_active_project();
    let entries = state.sidebar_entries();
    assert!(matches!(entries[0], SidebarEntry::Default(_)));
    assert_eq!(entries.len(), 1 + 2, "Default + the 2 worktrees");
    assert!(entries[1..]
        .iter()
        .all(|e| matches!(e, SidebarEntry::Worktree(_))));
}

#[test]
fn no_default_entry_when_no_project_is_open() {
    let state = State::default();
    assert!(state.workspace.active.is_none());
    assert!(
        state.sidebar_entries().is_empty(),
        "no project open must yield no sidebar entries at all, not a stray Default"
    );
}

#[test]
fn default_sessions_are_attached_to_the_default_entry_only() {
    let mut state = state_with_active_project(); // has one Worktree("feat-a") session
    let path = state.workspace.active.clone().unwrap();
    state
        .workspace
        .sessions
        .get_mut(&path)
        .unwrap()
        .push(Session::start_new(SessionLocation::Default));

    let entries = state.sidebar_entries();
    let SidebarEntry::Default(default_node) = &entries[0] else {
        panic!("expected the Default entry first");
    };
    assert_eq!(default_node.sessions.len(), 1);
    assert!(default_node
        .sessions
        .iter()
        .all(|s| s.location == SessionLocation::Default));
}

#[test]
fn filter_recomputes_after_rename(/* FR-028 / C1 */) {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    // Renaming changes only the display name; tags (and thus the filter result) are unchanged.
    state.update(Message::WorktreeRenameStarted("fix-crash".to_string()));
    state.update(Message::WorktreeRenameTextChanged("Hotfix".to_string()));
    state.update(Message::WorktreeRenameConfirmed);
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    let renamed = node(&state.filtered_worktree_tree(), "fix-crash")
        .display_name
        .clone();
    assert_eq!(renamed, "Hotfix");
}

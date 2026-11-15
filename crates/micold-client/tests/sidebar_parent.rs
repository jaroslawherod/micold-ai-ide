//! Spec 1857, US4 (T015): the sidebar labels a worktree with its parent and filters by it.

use micold_client::app::{Message, State};
use micold_client::features::sidebar::{row_chips, Msg as SidebarMsg, RowChip, TagFilter};
use micold_core::naming::{ConventionalType, Tag};
use micold_core::worktree::{ParentLabel, Worktree};

use crate::support::sidebar_fixture::{deliver, listed, state_with_active};

fn label(name: &str, removed: bool) -> ParentLabel {
    ParentLabel {
        name: name.into(),
        removed,
    }
}

fn child(dir: &str, parent: Option<ParentLabel>) -> Worktree {
    Worktree {
        parent,
        ..listed(dir)
    }
}

/// `sup` has two children, `feat-abc-1_a` (with an issue) and `fix-c` (a type, no issue);
/// `feat-abc-2_b` is a child of the project root, `gone-kid` of a removed parent `old`, and
/// `sup` and `plain` have no parent.
fn state() -> State {
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![
            child("sup", None),
            child("feat-abc-1_a", Some(label("sup", false))),
            child("fix-c", Some(label("sup", false))),
            child("feat-abc-2_b", Some(label("default", false))),
            child("plain", None),
            child("gone-kid", Some(label("old", true))),
        ],
    );
    state
}

fn shown(state: &State) -> Vec<String> {
    state
        .filtered_worktree_tree()
        .into_iter()
        .map(|n| n.worktree.dir_name)
        .collect()
}

fn toggle(state: &mut State, filter: TagFilter) {
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(filter)));
}

#[test]
fn a_worktree_with_a_parent_has_a_parent_tag_and_one_without_has_none() {
    let tree = state().worktree_tree();
    let tags = |dir: &str| {
        tree.iter()
            .find(|n| n.worktree.dir_name == dir)
            .unwrap()
            .tags
            .clone()
    };
    assert!(tags("fix-c").contains(&Tag::Parent(label("sup", false))));
    assert!(!tags("plain")
        .iter()
        .chain(tags("sup").iter())
        .any(|t| matches!(t, Tag::Parent(_))));
}

#[test]
fn the_row_shows_the_parent_name_and_marks_a_removed_parent() {
    let tree = state().worktree_tree();
    let chips = |dir: &str| {
        row_chips(
            tree.iter().find(|n| n.worktree.dir_name == dir).unwrap(),
            false,
        )
    };
    assert!(chips("fix-c").contains(&RowChip::Parent("sup".into())));
    assert!(chips("gone-kid").contains(&RowChip::Parent("old (removed)".into())));
    assert!(!chips("plain")
        .iter()
        .any(|c| matches!(c, RowChip::Parent(_))));
}

#[test]
fn the_parent_filter_narrows_to_that_parents_direct_children() {
    let mut state = state();
    toggle(&mut state, TagFilter::Parent(label("sup", false)));
    assert_eq!(shown(&state), vec!["feat-abc-1_a", "fix-c"]);
}

#[test]
fn the_default_parent_is_a_valid_filter_and_a_removed_parent_is_its_own() {
    let mut state = state();
    toggle(&mut state, TagFilter::Parent(label("default", false)));
    assert_eq!(shown(&state), vec!["feat-abc-2_b"]);
    toggle(&mut state, TagFilter::Parent(label("default", false)));
    toggle(&mut state, TagFilter::Parent(label("old", true)));
    assert_eq!(shown(&state), vec!["gone-kid"]);
}

#[test]
fn the_parent_filter_composes_with_the_issue_and_type_filters() {
    let mut state = state();
    toggle(&mut state, TagFilter::Parent(label("sup", false)));
    toggle(&mut state, TagFilter::HasIssue);
    assert_eq!(shown(&state), vec!["feat-abc-1_a"]);
    toggle(&mut state, TagFilter::HasIssue);
    toggle(&mut state, TagFilter::Type(ConventionalType::Fix));
    assert_eq!(
        shown(&state),
        vec!["fix-c"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
    );
}

#[test]
fn two_parent_filters_show_either_parents_children() {
    let mut state = state();
    toggle(&mut state, TagFilter::Parent(label("sup", false)));
    toggle(&mut state, TagFilter::Parent(label("default", false)));
    assert_eq!(shown(&state), vec!["feat-abc-1_a", "fix-c", "feat-abc-2_b"]);
}

#[test]
fn only_parents_that_have_children_are_offered() {
    let offered: Vec<TagFilter> = state()
        .available_tag_filters()
        .into_iter()
        .filter(|f| matches!(f, TagFilter::Parent(_)))
        .collect();
    assert_eq!(
        offered,
        vec![
            TagFilter::Parent(label("default", false)),
            TagFilter::Parent(label("old", true)),
            TagFilter::Parent(label("sup", false)),
        ]
    );
}

#[test]
fn renaming_the_parent_updates_the_label_and_keeps_order() {
    let mut state = state();
    let before: Vec<String> = shown(&state);
    deliver(
        &mut state,
        vec![
            child("sup", None),
            child("feat-abc-1_a", Some(label("Supervisor", false))),
            child("fix-c", Some(label("Supervisor", false))),
            child("feat-abc-2_b", Some(label("default", false))),
            child("plain", None),
            child("gone-kid", Some(label("old", true))),
        ],
    );
    assert_eq!(shown(&state), before);
    let tree = state.worktree_tree();
    let node = tree
        .iter()
        .find(|n| n.worktree.dir_name == "fix-c")
        .unwrap();
    assert!(row_chips(node, false).contains(&RowChip::Parent("Supervisor".into())));
}

#[test]
fn the_launch_hook_names_a_live_parent() {
    use micold_client::features::sidebar::filters_from_env_value;
    assert_eq!(
        filters_from_env_value(Some("issue,parent:sup")),
        Ok(vec![
            TagFilter::HasIssue,
            TagFilter::Parent(label("sup", false))
        ])
    );
    assert!(filters_from_env_value(Some("parent:")).is_err());
}

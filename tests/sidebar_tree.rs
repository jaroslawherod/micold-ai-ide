//! T017 — sidebar tree building + expand/collapse (FR-002/003).

use micold_ai_ide::app::{Message, State};
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::Session;
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
    state
        .workspace
        .sessions
        .insert(path, vec![Session::start_new("feat-a")]);
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

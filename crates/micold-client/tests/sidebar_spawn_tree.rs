//! The sidebar nests the sessions an agent started under their origin (feature 551, M2).

mod support;

use micold_client::app::State;
use micold_client::features::sidebar::{SidebarEntry, WorktreeNode};
use micold_core::origin::{OriginStatus, Orphan, TreePlace};
use micold_core::session::{SessionId, SessionLocation};
use support::spawn_tree::{known, session, state_with, wt};

fn node(state: &State) -> WorktreeNode {
    state.worktree_tree().remove(0)
}

fn shape(node: &WorktreeNode) -> Vec<(SessionId, usize, TreePlace)> {
    node.session_rows()
        .iter()
        .map(|r| (r.session.id, r.depth, r.place))
        .collect()
}

#[test]
fn children_nest_under_their_origin_at_any_depth() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let grandchild = session(&wt(), known(&child));
    let second = session(&wt(), known(&user));
    let ids = (user.id, child.id, grandchild.id, second.id);
    let state = state_with(vec![user, child, grandchild, second]);
    assert_eq!(
        shape(&node(&state)),
        vec![
            (ids.0, 0, TreePlace::Root),
            (ids.1, 1, TreePlace::Child),
            (ids.2, 2, TreePlace::Child),
            (ids.3, 1, TreePlace::Child),
        ]
    );
}

#[test]
fn a_session_without_an_origin_is_listed_as_it_was() {
    let mut main = session(&wt(), None);
    main.main = true;
    let helper = session(&wt(), None);
    let ids = (main.id, helper.id);
    let state = state_with(vec![helper, main]);
    let node = node(&state);
    assert_eq!(
        shape(&node),
        vec![(ids.0, 0, TreePlace::Root), (ids.1, 0, TreePlace::Root)]
    );
    let listed: Vec<_> = node.listed_sessions().iter().map(|s| s.id).collect();
    assert_eq!(listed, vec![ids.0, ids.1]);
}

#[test]
fn an_orphan_is_top_level_and_moves_under_its_origin_when_it_appears() {
    let missing = session(&wt(), None);
    let mut main = session(&wt(), None);
    main.main = true;
    let pending = session(&wt(), known(&missing));
    let removed = session(&wt(), Some((SessionId::new(), OriginStatus::Removed)));
    let ids = (main.id, pending.id, removed.id);
    let state = state_with(vec![main.clone(), pending.clone(), removed.clone()]);
    assert_eq!(
        shape(&node(&state)),
        vec![
            (ids.0, 0, TreePlace::Root),
            (ids.1, 0, TreePlace::Orphan(Orphan::Pending)),
            (ids.2, 0, TreePlace::Orphan(Orphan::Removed)),
        ]
    );
    let missing_id = missing.id;
    let state = state_with(vec![main, pending, removed, missing]);
    let rows = shape(&node(&state));
    let at = |id| rows.iter().find(|r| r.0 == id).copied().unwrap();
    assert_eq!(
        at(ids.1).1,
        1,
        "the pending child nests once its origin is listed"
    );
    assert_eq!(at(ids.1).2, TreePlace::Child);
    assert_eq!(at(missing_id).1, 0);
}

#[test]
fn the_default_group_nests_the_same_way() {
    let root = SessionLocation::Default;
    let mut main = session(&root, None);
    main.main = true;
    let child = session(&root, known(&main));
    let (main_id, child_id) = (main.id, child.id);
    let state = state_with(vec![main, child]);
    let SidebarEntry::Default(node) = state.sidebar_entries().remove(0) else {
        panic!("the Default entry leads");
    };
    let rows: Vec<_> = node
        .session_rows()
        .iter()
        .map(|r| (r.session.id, r.depth))
        .collect();
    assert_eq!(rows, vec![(main_id, 0), (child_id, 1)]);
}

#[test]
fn every_session_is_listed_once_and_the_row_count_is_unchanged() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let state = state_with(vec![child, user]);
    let node = node(&state);
    assert_eq!(node.listed_sessions().len(), node.sessions.len());
    assert!(node.has_expander());
}

// Folding (M4, US4, FR-011, FR-017).

use micold_client::features::sidebar::{update, Msg};

fn toggle(state: &mut State, id: SessionId) {
    update(state, Msg::ToggleOrigin(id));
}

fn listed(state: &State) -> Vec<SessionId> {
    node(state).listed_sessions().iter().map(|s| s.id).collect()
}

fn fold_of(state: &State, id: SessionId) -> Option<(bool, usize)> {
    node(state)
        .session_rows()
        .iter()
        .find(|r| r.session.id == id)
        .and_then(|r| r.fold.as_ref().map(|f| (f.folded, f.children)))
}

#[test]
fn an_origin_never_folded_is_expanded_and_toggling_folds_then_expands() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let (u, c) = (user.id, child.id);
    let mut state = state_with(vec![user, child]);
    assert_eq!(listed(&state), vec![u, c]);
    assert_eq!(fold_of(&state, u), Some((false, 1)));
    toggle(&mut state, u);
    assert_eq!(listed(&state), vec![u], "the child is hidden");
    assert_eq!(fold_of(&state, u), Some((true, 1)));
    toggle(&mut state, u);
    assert_eq!(listed(&state), vec![u, c]);
}

#[test]
fn a_folded_origin_counts_direct_children_and_hides_every_descendant() {
    let mut user = session(&wt(), None);
    user.main = true;
    let a = session(&wt(), known(&user));
    let grandchild = session(&wt(), known(&a));
    let b = session(&wt(), known(&user));
    let (u, ga) = (user.id, a.id);
    let mut state = state_with(vec![user, a, grandchild, b]);
    toggle(&mut state, u);
    assert_eq!(listed(&state), vec![u]);
    assert_eq!(fold_of(&state, u), Some((true, 2)), "direct children only");
    // Folding an inner origin hides only its own subtree.
    toggle(&mut state, u);
    toggle(&mut state, ga);
    assert_eq!(listed(&state).len(), 3);
    assert_eq!(fold_of(&state, ga), Some((true, 1)));
}

#[test]
fn a_folded_origin_shows_that_a_hidden_descendant_needs_attention() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let mut grandchild = session(&wt(), known(&child));
    grandchild.unread = true;
    let (u, g) = (user.id, grandchild.id);
    let mut state = state_with(vec![user, child, grandchild]);
    let attention = |state: &State, in_view| {
        let n = node(state);
        let rows = n.session_rows();
        let row = rows.iter().find(|r| r.session.id == u).unwrap();
        row.fold.as_ref().unwrap().needs_attention(in_view)
    };
    assert!(
        !attention(&state, None),
        "expanded: the child shows its own mark"
    );
    toggle(&mut state, u);
    assert!(attention(&state, None), "rolled up over descendants");
    assert!(!attention(&state, Some(g)), "a descendant in view is read");
}

#[test]
fn a_fold_restored_at_boot_survives_the_snapshot_without_pruning() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let u = user.id;
    // Boot: the fold is read from the settings before any session is known.
    let mut state = state_with(vec![]);
    state.sidebar.folded.insert(u);
    state.sidebar.folded.insert(SessionId::new());
    // The first snapshot arrives with the sessions.
    let arrived = state_with(vec![user, child]);
    state.workspace = arrived.workspace;
    assert_eq!(listed(&state), vec![u], "still folded as the user left it");
    assert_eq!(state.sidebar.folded.len(), 2, "no pruning");
}

#[test]
fn a_chord_still_steps_through_the_sessions_a_fold_hides() {
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let (u, c) = (user.id, child.id);
    let mut state = state_with(vec![user, child]);
    toggle(&mut state, u);
    let n = node(&state);
    let all: Vec<_> = n.unfolded_sessions().iter().map(|s| s.id).collect();
    assert_eq!(all, vec![u, c], "the step order keeps the hidden child");
    assert_eq!(n.listed_sessions().len(), 1);
}

/// Feature 1125 (M5 review A F1): the row counts behind hover and scrolling skip the sessions a
/// fold hides, as the list does, so the location under a folded origin tree is where it is drawn.
#[test]
fn the_row_counts_skip_the_sessions_a_fold_hides() {
    use micold_client::features::sidebar::{location_row, row_heights, row_index, RowRef};
    use micold_core::worktree::{Worktree, WorktreeStatus};
    let mut user = session(&wt(), None);
    user.main = true;
    let child = session(&wt(), known(&user));
    let u = user.id;
    let mut state = state_with(vec![user, child]);
    let mut below = state.worktree.worktrees[0].clone();
    below.dir_name = "feat-b".to_string();
    below.path = below.path.with_file_name("feat-b");
    below.branch = Some("feat/b".to_string());
    state.worktree.worktrees.push(Worktree {
        status: WorktreeStatus::Valid,
        ..below
    });
    state
        .sidebar
        .expanded
        .insert(support::spawn_tree::DIR.to_string());
    toggle(&mut state, u);
    assert_eq!(
        listed(&state),
        vec![u],
        "precondition: the child is folded away"
    );

    let list = state.sidebar_list();
    let b = SessionLocation::Worktree("feat-b".to_string());
    let drawn = row_index(&list, &RowRef::Location(b.clone()));
    assert!(drawn.is_some(), "precondition: feat-b is drawn");
    assert_eq!(location_row(&list, &b), drawn, "hover and the list agree");
    let entries = list.len();
    assert_eq!(
        row_heights(&list).len(),
        entries + 1,
        "one session row under the folded origin, none for its hidden child"
    );
}

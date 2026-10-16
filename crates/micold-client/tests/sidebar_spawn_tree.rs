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

//! The sidebar drag in isolation (feature 1786; SC-004 of feature 021): this feature's own state
//! and messages only. Where a drop lands in the layout is in `tests/sidebar_drag.rs`, which reads
//! the sidebar's layout through the root.

use micold_client::app::State;
use micold_client::features::sidebar_drag::{update, Dragged, Msg};
use micold_core::sidebar_layout::LayoutItem;

#[test]
fn with_no_project_a_press_arms_nothing_and_a_release_does_nothing() {
    let mut state = State::default();
    let press = Msg::Pressed(Dragged::Item(LayoutItem::Worktree("a".into())), (0, 0));
    assert!(update(&mut state, press).is_empty());
    assert_eq!(state.sidebar_drag.armed, None);
    assert!(update(&mut state, Msg::PointerMoved((0, 40))).is_empty());
    assert!(!state.sidebar_drag.active);
    assert!(update(&mut state, Msg::Released).is_empty());
    assert!(update(&mut state, Msg::Cancelled).is_empty());
}

//! Dragging a sidebar row between lists, and cancelling the drag (feature 1786, US3 and US5;
//! contracts C3, C4; test list A10 to A15, A20 to A22, A27, U20 to U23).
//!
//! The drag is render-free: a press arms it, window pointer moves activate it, the pointer's
//! height in the list picks the drop place through `hit_test`, and the release commits. What the
//! sidebar draws while dragging, the target's highlight and the insertion marker, is
//! `sidebar_drag.over` and its marker row, asserted here.

use crate::support::sidebar_fixture::{deliver, listed, session_at, state_with_active, worktree};
use micold_client::app::{Message, State};
use micold_client::features::sidebar::{row_heights, SidebarEntry};
use micold_client::features::sidebar_drag::{entry_spans, hit_test, Dragged, DropSpot, Msg};
use micold_core::sidebar_layout::{LayoutItem, SidebarLayout, UserGroupId};
use std::path::PathBuf;

const PROJECT: &str = "/repo";
const WORK: UserGroupId = UserGroupId(0);
const PLAY: UserGroupId = UserGroupId(1);

fn wt(dir: &str) -> LayoutItem {
    LayoutItem::Worktree(dir.into())
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::SidebarDrag(msg));
}

/// `a` and `b` in "Work", an empty "Play", and `c`, `d`, `e` ungrouped, each worktree with a
/// session, the layout saved: Default, Work, a, b, Play, c, d, e.
fn grouped() -> State {
    let mut state = state_with_active();
    let dirs = ["a", "b", "c", "d", "e"];
    deliver(&mut state, dirs.iter().map(|d| listed(d)).collect());
    for dir in dirs {
        session_at(&mut state, worktree(dir));
    }
    let mut layout = SidebarLayout::default();
    layout.create("Work").unwrap();
    layout.create("Play").unwrap();
    layout.groups[0].items = vec![wt("a"), wt("b")];
    layout.ungrouped = vec![wt("c"), wt("d"), wt("e")];
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), layout);
    state
}

fn layout(state: &State) -> SidebarLayout {
    state.sidebar_layout().cloned().unwrap_or_default()
}

fn items_of(state: &State, id: UserGroupId) -> Vec<LayoutItem> {
    layout(state)
        .group(id)
        .map(|group| group.items.clone())
        .unwrap_or_default()
}

fn saved(state: &State) -> bool {
    state
        .sidebar
        .layouts_unsaved
        .contains(&PathBuf::from(PROJECT))
}

/// The index in the drawn list of the first entry `matches` accepts.
fn entry_at(state: &State, matches: impl Fn(&SidebarEntry) -> bool) -> usize {
    state
        .sidebar_list()
        .iter()
        .position(matches)
        .expect("the entry is drawn")
}

fn header(id: UserGroupId) -> impl Fn(&SidebarEntry) -> bool {
    move |entry| matches!(entry, SidebarEntry::UserGroup(h) if h.id == id)
}

fn row(dir: &'static str) -> impl Fn(&SidebarEntry) -> bool {
    move |entry| matches!(entry, SidebarEntry::Worktree(n) if n.worktree.dir_name == dir)
}

/// A height in the list `frac` of the way down entry `index` (0.25 is its upper half, 0.75 its
/// lower).
fn y_in(state: &State, index: usize, frac: f32) -> u32 {
    let entries = state.sidebar_list();
    let span = entry_spans(&row_heights(&entries), &entries)[index];
    (span.top + span.height * frac) as u32
}

/// Report the pointer `frac` of the way down entry `index` of the list.
fn hover(state: &mut State, index: usize, frac: f32) {
    let y = y_in(state, index, frac);
    send(state, Msg::Moved(Some((20, y))));
}

/// The pointer at `(x, y)` in the list's content.
fn at(x: u32, y: u32) -> Msg {
    Msg::Moved(Some((x, y)))
}

/// Press `dragged` and move past the activation distance, so the drag is active.
fn start(state: &mut State, dragged: Dragged) {
    send(state, Msg::Pressed(dragged));
    send(state, at(20, 100));
    send(state, at(20, 110));
    assert!(state.sidebar_drag.active, "the drag is under way");
}

/// Drag `item` to `frac` of the way down the entry `matches` names, and drop it there.
fn drop_on(
    state: &mut State,
    item: LayoutItem,
    matches: impl Fn(&SidebarEntry) -> bool,
    frac: f32,
) {
    start(state, Dragged::Item(item));
    let index = entry_at(state, matches);
    hover(state, index, frac);
    send(state, Msg::Released);
}

// ---- T029: arming, activation, hit test, commit (US3, C3, C4) ---------------------------------

#[test]
fn a_press_arms_and_a_move_under_four_pixels_stays_a_click() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    assert_eq!(state.sidebar_drag.armed, Some(Dragged::Item(wt("c"))));
    send(&mut state, at(20, 100));
    send(&mut state, at(22, 102));
    assert!(!state.sidebar_drag.active, "under 4 px it is still a click");
    let before = layout(&state);
    send(&mut state, Msg::Released);
    assert_eq!(state.sidebar_drag.armed, None, "the release ends the press");
    assert_eq!(layout(&state), before, "a click moves nothing");
    assert!(!saved(&state));
}

/// Review B r1 F3: a pointer that left the list forgets where the drag starts from, so coming
/// back in is no motion; only a move from where it came back activates the drag.
#[test]
fn leaving_the_list_before_the_drag_is_active_forgets_where_it_started() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    send(&mut state, at(20, 100));
    send(&mut state, Msg::Moved(None));
    send(&mut state, at(20, 110));
    assert!(!state.sidebar_drag.active, "re-entering is where it starts");
    send(&mut state, at(20, 112));
    assert!(
        !state.sidebar_drag.active,
        "2 px from there is still a click"
    );
    send(&mut state, at(20, 114));
    assert!(state.sidebar_drag.active, "4 px from there is a drag");
}

#[test]
fn a_move_of_four_pixels_activates_the_drag() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    send(&mut state, at(20, 100));
    send(&mut state, at(20, 103));
    assert!(!state.sidebar_drag.active, "3 px");
    send(&mut state, at(20, 104));
    assert!(state.sidebar_drag.active, "4 px");
}

#[test]
fn the_list_pointer_is_ignored_until_the_drag_is_active() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    let index = entry_at(&state, header(WORK));
    hover(&mut state, index, 0.5);
    assert_eq!(state.sidebar_drag.over, None);
}

#[test]
fn the_hit_test_finds_only_group_headers_group_rows_and_ungrouped_rows() {
    let state = grouped();
    let entries = state.sidebar_list();
    let heights = row_heights(&entries);
    let at = |matches: &dyn Fn(&SidebarEntry) -> bool, frac: f32| {
        let index = entries.iter().position(matches).unwrap();
        let span = entry_spans(&heights, &entries)[index];
        hit_test(span.top + span.height * frac, &heights, &entries)
    };
    let default = |e: &SidebarEntry| matches!(e, SidebarEntry::Default(_));
    assert_eq!(at(&default, 0.5), None, "Default is no target");
    let work = at(&header(WORK), 0.5).expect("a header is a target");
    assert_eq!(
        (work.list, work.before, work.after),
        (Some(WORK), None, None),
        "the header lands at the group's start"
    );
    let play = entries.iter().position(header(PLAY)).unwrap();
    let empty = at(&header(PLAY), 0.5).expect("an empty group stays a target");
    assert_eq!(
        (empty.list, empty.marker, empty.below),
        (Some(PLAY), entry_spans(&heights, &entries)[play].row, true),
        "the marker is under the header, not on the next list's first row"
    );
    let upper = at(&row("a"), 0.25).unwrap();
    assert_eq!((upper.list, upper.before), (Some(WORK), Some(wt("a"))));
    let lower = at(&row("b"), 0.75).unwrap();
    assert_eq!((lower.list, lower.after), (Some(WORK), Some(wt("b"))));
    assert!(lower.below, "under b, not on Play's header");
    let ungrouped = at(&row("c"), 0.25).unwrap();
    assert_eq!((ungrouped.list, ungrouped.before), (None, Some(wt("c"))));
    let spans = entry_spans(&heights, &entries);
    let last = spans.last().unwrap();
    assert_eq!(
        hit_test(last.top + last.height + 40.0, &heights, &entries),
        None,
        "below the last row is no target"
    );
    assert_eq!(hit_test(-1.0, &heights, &entries), None);
}

#[test]
fn the_spans_follow_the_drawn_rows_and_the_gap_between_them() {
    let state = grouped();
    let entries = state.sidebar_list();
    let heights = row_heights(&entries);
    let spans = entry_spans(&heights, &entries);
    assert_eq!(spans.len(), entries.len());
    assert_eq!(spans[0].top, 0.0);
    let gap = spans[1].top - (spans[0].top + spans[0].height);
    assert!(gap > 0.0, "rows are spaced: {gap}");
    let last = spans.last().unwrap();
    assert_eq!(
        last.row + last.rows,
        heights.len(),
        "every drawn row is counted"
    );
}

#[test]
fn over_a_target_the_spot_carries_the_marker_and_over_none_nothing_is_drawn() {
    let mut state = grouped();
    start(&mut state, Dragged::Item(wt("c")));
    let a = entry_at(&state, row("a"));
    hover(&mut state, a, 0.25);
    let spot: DropSpot = state.sidebar_drag.over.clone().expect("highlighted");
    let entries = state.sidebar_list();
    let spans = entry_spans(&row_heights(&entries), &entries);
    assert_eq!(
        (spot.marker, spot.below),
        (spans[a].row, false),
        "the marker sits on a's top edge"
    );
    hover(&mut state, 0, 0.5);
    assert_eq!(
        state.sidebar_drag.over, None,
        "over Default: no highlight, no marker"
    );
    hover(&mut state, a, 0.75);
    assert!(state.sidebar_drag.over.is_some());
    send(&mut state, Msg::Moved(None));
    assert_eq!(state.sidebar_drag.over, None, "the pointer left the list");
}

#[test]
fn dropping_an_ungrouped_row_on_a_group_assigns_it_at_the_marked_place_and_saves() {
    let mut state = grouped();
    drop_on(&mut state, wt("c"), row("b"), 0.25);
    assert_eq!(items_of(&state, WORK), vec![wt("a"), wt("c"), wt("b")]);
    assert!(!layout(&state).ungrouped.contains(&wt("c")));
    assert!(saved(&state));
    assert_eq!(state.sidebar_drag.armed, None, "the drop ends the drag");
    assert_eq!(state.sidebar_drag.over, None);
}

#[test]
fn dropping_on_a_header_puts_the_row_first_in_its_group() {
    let mut state = grouped();
    drop_on(&mut state, wt("c"), header(WORK), 0.5);
    assert_eq!(items_of(&state, WORK), vec![wt("c"), wt("a"), wt("b")]);
}

#[test]
fn dropping_on_another_group_moves_the_row_out_of_the_first() {
    let mut state = grouped();
    drop_on(&mut state, wt("a"), header(PLAY), 0.5);
    assert_eq!(items_of(&state, PLAY), vec![wt("a")]);
    assert_eq!(items_of(&state, WORK), vec![wt("b")]);
}

#[test]
fn dropping_on_the_ungrouped_rows_removes_the_row_from_its_group() {
    let mut state = grouped();
    drop_on(&mut state, wt("a"), row("d"), 0.75);
    assert_eq!(items_of(&state, WORK), vec![wt("b")]);
    assert_eq!(
        layout(&state).ungrouped,
        vec![wt("c"), wt("d"), wt("a"), wt("e")]
    );
}

#[test]
fn a_dropped_worktree_keeps_its_sessions_listed_with_it() {
    let mut state = grouped();
    let sessions = state.workspace.sessions.clone();
    drop_on(&mut state, wt("c"), header(WORK), 0.5);
    assert_eq!(state.workspace.sessions, sessions, "sessions untouched");
    let c = entry_at(&state, row("c"));
    match &state.sidebar_list()[c] {
        SidebarEntry::Worktree(node) => assert_eq!(node.sessions.len(), 1),
        other => panic!("not a worktree row: {other:?}"),
    }
}

#[test]
fn a_drop_on_a_collapsed_group_assigns_the_row_and_expands_the_group() {
    let mut state = grouped();
    let mut held = layout(&state);
    held.groups[0].collapsed = true;
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), held);
    drop_on(&mut state, wt("c"), header(WORK), 0.5);
    let work = layout(&state).group(WORK).cloned().unwrap();
    assert!(work.items.contains(&wt("c")));
    assert!(!work.collapsed, "the group expands to show it");
}

#[test]
fn a_drop_on_the_rows_own_place_changes_nothing() {
    let mut state = grouped();
    let before = layout(&state);
    drop_on(&mut state, wt("a"), row("a"), 0.25);
    drop_on(&mut state, wt("a"), row("a"), 0.75);
    drop_on(&mut state, wt("b"), row("a"), 0.75);
    assert_eq!(layout(&state), before);
    assert!(!saved(&state), "nothing to save");
}

#[test]
fn a_drop_below_the_rows_own_place_in_its_list_lands_after_the_row_it_was_dropped_under() {
    let mut state = grouped();
    drop_on(&mut state, wt("c"), row("d"), 0.75);
    assert_eq!(layout(&state).ungrouped, vec![wt("d"), wt("c"), wt("e")]);
    drop_on(&mut state, wt("e"), row("d"), 0.25);
    assert_eq!(layout(&state).ungrouped, vec![wt("e"), wt("d"), wt("c")]);
}

#[test]
fn default_never_arms() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt(""))));
    assert_eq!(state.sidebar_drag.armed, None);
    send(&mut state, Msg::Pressed(Dragged::Item(wt("gone"))));
    assert_eq!(
        state.sidebar_drag.armed, None,
        "a row the sidebar does not have"
    );
    send(&mut state, Msg::Pressed(Dragged::Group(UserGroupId(9))));
    assert_eq!(
        state.sidebar_drag.armed, None,
        "a group the project does not have"
    );
}

#[test]
fn rows_a_tag_filter_hides_are_no_targets_and_keep_their_stored_place() {
    use micold_client::features::sidebar::TagFilter;
    use micold_core::naming::ConventionalType;
    let mut state = state_with_active();
    deliver(
        &mut state,
        vec![listed("feat-a"), listed("fix-b"), listed("feat-c")],
    );
    let mut held = SidebarLayout::default();
    held.create("Work").unwrap();
    held.groups[0].items = vec![wt("feat-a"), wt("fix-b")];
    held.ungrouped = vec![wt("feat-c")];
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), held);
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    assert!(
        state.sidebar_list().iter().all(|e| !row("fix-b")(e)),
        "fix-b is hidden"
    );
    drop_on(&mut state, wt("feat-c"), row("feat-a"), 0.75);
    assert_eq!(
        items_of(&state, WORK),
        vec![wt("feat-a"), wt("feat-c"), wt("fix-b")],
        "the hidden row keeps its place after the drop"
    );
}

#[test]
fn a_run_group_row_is_dragged_and_dropped_as_one_unit() {
    let mut state = state_with_active();
    deliver(&mut state, vec![listed("g-1"), listed("g-2"), listed("z")]);
    let group = crate::support::sidebar_fixture::run_group(1, "g", &["g-1", "g-2"]);
    let run = LayoutItem::Run(group.id);
    state.update(Message::Runs(
        micold_client::features::runs::Msg::GroupsChanged(vec![group]),
    ));
    let mut held = SidebarLayout::default();
    held.create("Work").unwrap();
    state.sidebar_layout_loaded(PathBuf::from(PROJECT), held);
    let entries = state.sidebar_list();
    let run_at = entries
        .iter()
        .position(|e| matches!(e, SidebarEntry::Group(_)))
        .unwrap();
    let heights = row_heights(&entries);
    let span = entry_spans(&heights, &entries)[run_at];
    let spot = hit_test(span.top + span.height * 0.25, &heights, &entries).unwrap();
    assert_eq!(
        spot.before,
        Some(run.clone()),
        "the run group is one target"
    );
    drop_on(&mut state, run.clone(), header(WORK), 0.5);
    assert_eq!(items_of(&state, WORK), vec![run], "one item, not its runs");
}

#[test]
fn a_group_header_arms_a_group_drag_that_lands_between_headers_only() {
    let mut state = grouped();
    start(&mut state, Dragged::Group(PLAY));
    assert_eq!(state.sidebar_drag.armed, Some(Dragged::Group(PLAY)));
    let c = entry_at(&state, row("c"));
    hover(&mut state, c, 0.25);
    assert_eq!(
        state.sidebar_drag.over, None,
        "a group lands between headers, never in the ungrouped list"
    );
    let work = entry_at(&state, header(WORK));
    hover(&mut state, work, 0.25);
    let spot = state
        .sidebar_drag
        .over
        .clone()
        .expect("above Work's header");
    assert_eq!(spot.slot, Some(0), "before the first group");
    assert!(!spot.below, "the marker on the top edge of Work's header");
    let b = entry_at(&state, row("b"));
    hover(&mut state, b, 0.75);
    let spot = state
        .sidebar_drag
        .over
        .clone()
        .expect("in Work's lower half");
    assert_eq!(spot.slot, Some(1), "after Work, its rows included");
    assert!(spot.below);
}

// ---- T037: reorder by drag (US4, FR-019, C3) --------------------------------------------------

#[test]
fn a_row_dragged_over_a_header_gets_a_list_place_never_a_group_slot() {
    let state = grouped();
    let entries = state.sidebar_list();
    let heights = row_heights(&entries);
    let work = entries.iter().position(header(WORK)).unwrap();
    let span = entry_spans(&heights, &entries)[work];
    let spot = hit_test(span.top + span.height * 0.25, &heights, &entries).unwrap();
    assert_eq!((spot.list, spot.slot), (Some(WORK), None));
}

#[test]
fn dropping_a_group_header_among_the_groups_reorders_them_with_their_rows() {
    let mut state = grouped();
    start(&mut state, Dragged::Group(PLAY));
    let work = entry_at(&state, header(WORK));
    hover(&mut state, work, 0.25);
    send(&mut state, Msg::Released);
    let order: Vec<UserGroupId> = layout(&state).groups.iter().map(|g| g.id).collect();
    assert_eq!(order, vec![PLAY, WORK]);
    assert_eq!(
        items_of(&state, WORK),
        vec![wt("a"), wt("b")],
        "its rows go with it"
    );
    assert!(saved(&state));
    let drawn: Vec<usize> = [header(PLAY), header(WORK)]
        .into_iter()
        .map(|m| entry_at(&state, m))
        .collect();
    assert!(drawn[0] < drawn[1], "the sidebar draws the new order");
    assert!(
        entry_at(&state, row("a")) > drawn[1],
        "Work's rows under Work"
    );
}

#[test]
fn a_group_dropped_on_its_own_place_changes_nothing() {
    let mut state = grouped();
    let before = layout(&state);
    start(&mut state, Dragged::Group(WORK));
    let a = entry_at(&state, row("a"));
    hover(&mut state, a, 0.25);
    assert_eq!(
        state.sidebar_drag.over.as_ref().and_then(|s| s.slot),
        Some(0)
    );
    send(&mut state, Msg::Released);
    assert_eq!(layout(&state), before);
    assert!(!saved(&state));
}

#[test]
fn a_row_dropped_in_its_own_list_takes_the_marked_place_and_keeps_it_on_redraw() {
    let mut state = grouped();
    drop_on(&mut state, wt("e"), row("c"), 0.25);
    assert_eq!(layout(&state).ungrouped, vec![wt("e"), wt("c"), wt("d")]);
    drop_on(&mut state, wt("a"), row("b"), 0.75);
    assert_eq!(items_of(&state, WORK), vec![wt("b"), wt("a")]);
    // The drawn list follows the stored order after both drops.
    assert!(entry_at(&state, row("e")) < entry_at(&state, row("c")));
    assert!(entry_at(&state, row("b")) < entry_at(&state, row("a")));
}

/// Review B r2 (M4) MINOR: a row held after a cancel whose release never reached the window is
/// let go by the next press, whatever that press arms.
#[test]
fn any_press_lets_go_of_a_row_held_after_a_cancel() {
    let mut state = dragging_over_a_target();
    send(&mut state, Msg::Cancelled);
    assert!(state.sidebar_drag.holds(&Dragged::Item(wt("c"))));
    send(&mut state, Msg::Pressed(Dragged::Item(wt(""))));
    assert!(
        !state.sidebar_drag.holds(&Dragged::Item(wt("c"))),
        "a press Default cannot arm still lets go"
    );
    assert!(!state.sidebar_drag.listens());
}

// ---- T031: cancelling (US5, FR-009, FR-018, SC-002) -------------------------------------------

/// An active drag of `c` over Work's first row.
fn dragging_over_a_target() -> State {
    let mut state = grouped();
    start(&mut state, Dragged::Item(wt("c")));
    let a = entry_at(&state, row("a"));
    hover(&mut state, a, 0.25);
    assert!(state.sidebar_drag.over.is_some());
    state
}

#[test]
fn escape_ends_the_drag_with_no_change() {
    let mut state = dragging_over_a_target();
    let entries = state.sidebar_list();
    let before = layout(&state);
    assert_eq!(
        micold_client::app::on_escape(&state),
        Some(Message::SidebarDrag(Msg::Cancelled)),
        "Escape reaches the drag"
    );
    state.update(Message::EscapePressed);
    assert_eq!(state.sidebar_drag.armed, None);
    assert_eq!(state.sidebar_drag.over, None, "highlight and marker vanish");
    send(&mut state, Msg::Released);
    assert_eq!(layout(&state), before);
    assert_eq!(state.sidebar_list(), entries, "the sidebar is as it was");
    assert!(!saved(&state));
}

/// Review B r1 F1: after Escape mid-drag the button is still down, so the dragged row keeps its
/// release held back and the window's release stays heard; the release then lets go of the row
/// with no click and no move.
#[test]
fn escape_mid_drag_holds_the_rows_release_until_the_button_is_up() {
    let mut state = dragging_over_a_target();
    let before = layout(&state);
    state.update(Message::EscapePressed);
    let c = Dragged::Item(wt("c"));
    assert!(
        state.sidebar_drag.holds(&c),
        "the dragged row presses nothing"
    );
    assert!(!state.sidebar_drag.holds(&Dragged::Item(wt("d"))));
    assert!(state.sidebar_drag.listens(), "the release is still heard");
    send(&mut state, at(20, 140));
    assert_eq!(state.sidebar_drag.over, None, "the pointer no longer drags");
    send(&mut state, Msg::Cancelled);
    assert!(
        state.sidebar_drag.holds(&c),
        "a second cancel keeps the hold"
    );
    send(&mut state, Msg::Released);
    assert!(
        !state.sidebar_drag.holds(&c),
        "the release let go of the row"
    );
    assert!(
        !state.sidebar_drag.listens(),
        "and nothing is heard any more"
    );
    assert_eq!(layout(&state), before);
    assert!(!saved(&state));
}

/// A press that never became a drag holds nothing after a focus loss: it was a click.
#[test]
fn a_focus_loss_before_the_drag_is_active_holds_nothing() {
    let mut state = grouped();
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    send(&mut state, Msg::Cancelled);
    assert!(!state.sidebar_drag.holds(&Dragged::Item(wt("c"))));
    assert!(!state.sidebar_drag.listens());
}

#[test]
fn a_release_outside_every_target_changes_nothing() {
    let mut state = dragging_over_a_target();
    let entries = state.sidebar_list();
    send(&mut state, Msg::Moved(None));
    send(&mut state, Msg::Released);
    assert_eq!(state.sidebar_drag.armed, None);
    assert_eq!(state.sidebar_list(), entries);
    assert!(!saved(&state));
}

#[test]
fn the_dragged_worktree_vanishing_ends_the_drag_with_no_change_and_no_error() {
    let mut state = dragging_over_a_target();
    let notices = format!("{:?}", state.notifications);
    deliver(
        &mut state,
        ["a", "b", "d", "e"].iter().map(|d| listed(d)).collect(),
    );
    let before = layout(&state);
    send(&mut state, at(20, 120));
    assert_eq!(state.sidebar_drag.armed, None, "the drag ended");
    send(&mut state, Msg::Released);
    assert_eq!(layout(&state), before);
    assert!(!saved(&state));
    assert_eq!(format!("{:?}", state.notifications), notices, "no error");
}

#[test]
fn a_release_after_the_dragged_worktree_vanished_changes_nothing() {
    let mut state = dragging_over_a_target();
    deliver(
        &mut state,
        ["a", "b", "d", "e"].iter().map(|d| listed(d)).collect(),
    );
    let before = layout(&state);
    send(&mut state, Msg::Released);
    assert_eq!(state.sidebar_drag.armed, None);
    assert_eq!(layout(&state), before);
}

#[test]
fn escape_with_no_drag_keeps_its_meaning() {
    let mut state = grouped();
    assert_eq!(
        micold_client::app::on_escape(&state),
        None,
        "nothing to close"
    );
    send(&mut state, Msg::Pressed(Dragged::Item(wt("c"))));
    assert_eq!(
        micold_client::app::on_escape(&state),
        None,
        "a press that is still a click is not a drag"
    );
}

#[test]
fn losing_the_window_focus_cancels_the_drag() {
    let mut state = dragging_over_a_target();
    let entries = state.sidebar_list();
    send(&mut state, Msg::Cancelled);
    assert_eq!(state.sidebar_drag.armed, None);
    send(&mut state, Msg::Released);
    assert_eq!(state.sidebar_list(), entries);
}

#[test]
fn escape_cancels_the_drag_before_a_surface_open_under_it() {
    use micold_client::features::sidebar_group_actions::Msg as GroupMsg;
    let mut state = dragging_over_a_target();
    state.update(Message::SidebarGroups(GroupMsg::MenuOpened(WORK, (40, 40))));
    assert!(state.sidebar_groups.group_menu.is_some(), "a menu is open");
    state.update(Message::EscapePressed);
    assert_eq!(state.sidebar_drag.armed, None, "the drag ended first");
    assert!(state.sidebar_groups.group_menu.is_some(), "and alone");
    state.update(Message::EscapePressed);
    assert!(
        state.sidebar_groups.group_menu.is_none(),
        "the next press closes the menu"
    );
}

//! Dragging a sidebar row onto a user group, another group or the ungrouped list, and cancelling
//! the drag (feature 1786, US3 and US5; contracts C3, C4; FR-006 to FR-009, FR-018, FR-019).
//!
//! # What this feature owns
//!
//! The drag in progress ([`State`]): what was pressed and where, whether the pointer has moved far
//! enough to make it a drag, and the place it would land now. The layout is the sidebar's: a drop
//! returns the changed layout as
//! [`Outcome::SidebarLayoutChanged`](crate::features::Outcome::SidebarLayoutChanged), as every
//! user-group operation does, and the shell saves it (research R9).
//!
//! # How a drag runs
//!
//! A press on a row arms it ([`Msg::Pressed`]). While armed, the list reports the pointer in its
//! content ([`Msg::Moved`]); the window is never followed (the idle guarantee, 018 FR-029d). The
//! first report is where the drag starts from, and at [`ACTIVATION_PX`] from it the drag is
//! active; below that the release is the click it always was (the row's own press selects). While
//! active, [`hit_test`] over the drawn rows turns each report into the [`DropSpot`]: the target
//! the sidebar highlights and the row edge its insertion marker is on. A dragged group header
//! lands between the groups instead ([`group_hit_test`], FR-019). The release, heard through
//! the window, commits; Escape, the window losing focus, a release over no target, or the dragged
//! row leaving the sidebar ends it with no change. A drag Escape or the focus loss ended still
//! holds the dragged row's release back until the button is up ([`State::held`]), so letting go
//! over that row is no click either; the next press lets go of it whatever it arms, so a release
//! the window never heard leaves no row inert.
//!
//! # Messages
//!
//! `Pressed`, `Moved`, `Released`, `Cancelled`.

use crate::features::sidebar::{layout_item, row_heights, SidebarEntry, ROW_GAP};
use crate::features::Outcome;
use micold_core::sidebar_layout::{DropTarget, LayoutItem, SidebarLayout, UserGroupId};

/// How far, in pixels, the pointer moves from where it was first seen after the press before the
/// press is a drag (C3).
pub const ACTIVATION_PX: u32 = 4;

/// What a drag carries: a row the layout places, or a user group by its header (FR-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dragged {
    /// A worktree row or a run group's row, which moves as one unit with its runs (FR-016).
    Item(LayoutItem),
    /// A user group, by its header. It lands between the groups (US4).
    Group(UserGroupId),
}

/// The drag in progress (data-model.md, *DragState*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// What the press armed; `None` while nothing is pressed.
    pub armed: Option<Dragged>,
    /// Where the list first saw the pointer after the press, in its content's pixels.
    pub from: Option<(u32, u32)>,
    /// Whether the pointer has moved [`ACTIVATION_PX`] from `from`: then it is a drag.
    pub active: bool,
    /// Where a release would land; `None` over no target, and then nothing is highlighted and no
    /// marker is drawn (C3).
    pub over: Option<DropSpot>,
    /// What a cancelled drag was carrying while its button is still down: that row holds its
    /// release back until [`Msg::Released`], so the cancel changes nothing (US5-1, FR-009).
    pub held: Option<Dragged>,
}

impl State {
    /// Whether the row carrying `dragged` holds its release back: it is dragged, or its drag was
    /// cancelled and the button is not up yet.
    pub fn holds(&self, dragged: &Dragged) -> bool {
        let dragging = self.active && self.armed.as_ref() == Some(dragged);
        dragging || self.held.as_ref() == Some(dragged)
    }

    /// Whether the window's release and focus loss are heard: while a press is armed, and after a
    /// cancel until the button is up.
    pub fn listens(&self) -> bool {
        self.armed.is_some() || self.held.is_some()
    }
}

/// A place a dragged row can land, as the drawn list names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropSpot {
    /// The target list: a group, or `None` for the ungrouped rows. The sidebar highlights it.
    pub list: Option<UserGroupId>,
    /// The row lands just above this one.
    pub before: Option<LayoutItem>,
    /// The row lands just below this one. With neither, it lands first in `list`.
    pub after: Option<LayoutItem>,
    /// The drawn row (an index into [`row_heights`]) whose edge the insertion marker is on.
    pub marker: usize,
    /// Whether the marker is on that row's bottom edge rather than its top.
    pub below: bool,
    /// For a dragged group: its new position among the groups ([`DropTarget::GroupSlot`]). Then
    /// no list is highlighted and `list`, `before` and `after` name nothing.
    pub slot: Option<usize>,
}

/// Everything the user does while dragging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// A row was pressed: the drag is armed.
    Pressed(Dragged),
    /// The pointer in the list's content, in whole pixels, `(x, y)`; `None` once it left the list.
    Moved(Option<(u32, u32)>),
    /// The button was released, wherever the pointer is.
    Released,
    /// Escape, or the window lost its focus: the drag ends with no change.
    Cancelled,
}

/// Where one entry of the drawn list sits in the list's content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntrySpan {
    /// Its top edge.
    pub top: f32,
    /// Its height: its rows and the gaps between them, not the gap below it.
    pub height: f32,
    /// Its first drawn row, an index into [`row_heights`].
    pub row: usize,
    /// How many drawn rows it takes: one, and its sessions or runs while shown.
    pub rows: usize,
}

/// Where each of `entries` sits, measured from `heights` (its [`row_heights`]) and the gap
/// [`ROW_GAP`] the list leaves between rows.
pub fn entry_spans(heights: &[f32], entries: &[SidebarEntry]) -> Vec<EntrySpan> {
    let mut spans = Vec::with_capacity(entries.len());
    let (mut top, mut row) = (0.0, 0);
    for entry in entries {
        let rows = row_heights(std::slice::from_ref(entry)).len();
        let Some(drawn) = heights.get(row..row + rows) else {
            break;
        };
        let height = drawn.iter().sum::<f32>() + ROW_GAP * rows.saturating_sub(1) as f32;
        spans.push(EntrySpan {
            top,
            height,
            row,
            rows,
        });
        top += height + ROW_GAP;
        row += rows;
    }
    spans
}

/// The list each of `entries` belongs to: `Some(Some(id))` for a group's header and its rows,
/// `Some(None)` for an ungrouped row, `None` for Default, which is in no list (FR-015). A header's
/// rows are the `count` entries after it while it is expanded (contract C5).
pub fn entry_lists(entries: &[SidebarEntry]) -> Vec<Option<Option<UserGroupId>>> {
    let mut members: Option<(UserGroupId, usize)> = None;
    entries
        .iter()
        .map(|entry| match entry {
            SidebarEntry::Default(_) => None,
            SidebarEntry::UserGroup(header) => {
                members = (!header.collapsed).then_some((header.id, header.count));
                Some(Some(header.id))
            }
            _ => match &mut members {
                Some((id, left)) if *left > 0 => {
                    *left -= 1;
                    Some(Some(*id))
                }
                _ => Some(None),
            },
        })
        .collect()
}

/// The place a row dropped at height `y` of the list lands (contract C3): over a group's header
/// (an empty group's included) it lands first in that group; over a row of a group or of the
/// ungrouped list, above that row in its upper half and below it in its lower half. Anywhere else,
/// Default and the space below the last row included, `None`. A row a tag filter hides is not
/// drawn, so it is never a target; its stored place stays.
pub fn hit_test(y: f32, heights: &[f32], entries: &[SidebarEntry]) -> Option<DropSpot> {
    let spans = entry_spans(heights, entries);
    let at = spans
        .iter()
        .position(|span| y >= span.top && y < span.top + span.height + ROW_GAP)?;
    let span = spans[at];
    let list = entry_lists(entries)[at]?;
    // The marker stays on the target's own rows: under a header, on top of the row the dragged
    // row lands before, or under the last row of the one it lands after.
    let last = span.row + span.rows - 1;
    let Some(item) = layout_item(&entries[at]) else {
        return Some(DropSpot {
            list,
            before: None,
            after: None,
            marker: span.row,
            below: true,
            slot: None,
        });
    };
    Some(if y < span.top + span.height / 2.0 {
        DropSpot {
            list,
            before: Some(item),
            after: None,
            marker: span.row,
            below: false,
            slot: None,
        }
    } else {
        DropSpot {
            list,
            before: None,
            after: Some(item),
            marker: last,
            below: true,
            slot: None,
        }
    })
}

/// The place a dragged group header dropped at height `y` lands (FR-019, C3): over a group, its
/// header and the rows it shows, before that group in the block's upper half and after it in the
/// lower half, the marker on the header's top edge or under the block's last row. Default, the
/// ungrouped rows and the space below are no place for a group.
pub fn group_hit_test(y: f32, heights: &[f32], entries: &[SidebarEntry]) -> Option<DropSpot> {
    let spans = entry_spans(heights, entries);
    let lists = entry_lists(entries);
    let mut slot = 0;
    let mut at = 0;
    while at < spans.len() {
        let Some(Some(id)) = lists[at] else {
            at += 1;
            continue;
        };
        let end = (at..spans.len())
            .take_while(|&e| lists[e] == Some(Some(id)))
            .last()
            .unwrap_or(at);
        let (top, bottom) = (spans[at].top, spans[end].top + spans[end].height);
        if y >= top && y < bottom + ROW_GAP {
            let before = y < top + (bottom - top) / 2.0;
            return Some(DropSpot {
                list: None,
                before: None,
                after: None,
                marker: if before {
                    spans[at].row
                } else {
                    spans[end].row + spans[end].rows - 1
                },
                below: !before,
                slot: Some(if before { slot } else { slot + 1 }),
            });
        }
        slot += 1;
        at = end + 1;
    }
    None
}

/// This feature's reducer (shape A).
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<Outcome> {
    match msg {
        Msg::Pressed(dragged) => {
            // Any press means the button went down again: a row still held after a cancel whose
            // release the window never heard is let go (M4 review B r2).
            state.sidebar_drag.held = None;
            if can_drag(state, &dragged) {
                state.sidebar_drag = State {
                    armed: Some(dragged),
                    ..State::default()
                };
            }
        }
        Msg::Moved(at) => moved(state, at),
        Msg::Released => return released(state),
        Msg::Cancelled => cancelled(&mut state.sidebar_drag),
    }
    Vec::new()
}

/// Escape or the focus loss: the drag ends with no change, and the row it carried, if it was
/// active, holds its release back until the button is up.
fn cancelled(drag: &mut State) {
    let was = std::mem::take(drag);
    drag.held = match was.armed {
        Some(dragged) if was.active => Some(dragged),
        _ => was.held,
    };
}

/// Whether `dragged` is a row or group the open project's sidebar has; never Default (FR-015).
fn can_drag(state: &crate::app::State, dragged: &Dragged) -> bool {
    if state.workspace.active.is_none() {
        return false;
    }
    match dragged {
        Dragged::Item(LayoutItem::Worktree(dir)) if dir.is_empty() => false,
        Dragged::Item(item) => state.layout_item_exists(item),
        Dragged::Group(id) => state
            .sidebar_layout()
            .is_some_and(|layout| layout.group(*id).is_some()),
    }
}

/// Whether what the drag carries is still in the sidebar; one that left it ends the drag (C3).
fn still_there(state: &crate::app::State) -> bool {
    state
        .sidebar_drag
        .armed
        .as_ref()
        .is_some_and(|dragged| can_drag(state, dragged))
}

/// The pointer moved in the list, or left it (`None`): the first report after the press is where
/// the drag starts from, a report [`ACTIVATION_PX`] from it activates the drag, and while active
/// each report picks the drop spot. A dragged row that left the sidebar ends the drag.
fn moved(state: &mut crate::app::State, at: Option<(u32, u32)>) {
    if state.sidebar_drag.armed.is_none() {
        return;
    }
    if !still_there(state) {
        state.sidebar_drag = State::default();
        return;
    }
    let drag = &mut state.sidebar_drag;
    // A pointer that left the list before the drag is active starts again where it comes back.
    match (drag.active, at, drag.from) {
        (true, _, _) => {}
        (false, None, _) => drag.from = None,
        (false, Some(at), None) => drag.from = Some(at),
        (false, Some(at), Some(from)) => drag.active = moved_far(from, at),
    }
    if !state.sidebar_drag.active {
        return;
    }
    let Some((_, y)) = at else {
        state.sidebar_drag.over = None;
        return;
    };
    let entries = state.sidebar_list();
    let heights = row_heights(&entries);
    state.sidebar_drag.over = match &state.sidebar_drag.armed {
        Some(Dragged::Item(_)) => hit_test(y as f32, &heights, &entries),
        Some(Dragged::Group(_)) => group_hit_test(y as f32, &heights, &entries),
        None => None,
    };
}

/// Whether `to` is [`ACTIVATION_PX`] or more from `from`.
fn moved_far(from: (u32, u32), to: (u32, u32)) -> bool {
    let dx = u64::from(from.0.abs_diff(to.0));
    let dy = u64::from(from.1.abs_diff(to.1));
    dx * dx + dy * dy >= u64::from(ACTIVATION_PX * ACTIVATION_PX)
}

/// The release: an active drag over a target commits the move (C4); anything else ends with no
/// change. The drag ends either way.
fn released(state: &mut crate::app::State) -> Vec<Outcome> {
    let drag = std::mem::take(&mut state.sidebar_drag);
    let (Some(dragged), true, Some(spot)) = (drag.armed, drag.active, drag.over) else {
        return Vec::new();
    };
    let Some(project) = state.workspace.active.clone() else {
        return Vec::new();
    };
    let item = match dragged {
        Dragged::Group(id) => return group_released(state, project, id, &spot),
        Dragged::Item(item) => item,
    };
    if !state.layout_item_exists(&item) {
        return Vec::new();
    }
    // A drawn row the layout does not hold yet is ungrouped, after the held ones, where the
    // sidebar draws it: placed there so the marker's neighbours have a place to count from.
    let mut layout = state.sidebar_layout_placed();
    let Some(to) = drop_target(&layout, &spot) else {
        return Vec::new();
    };
    let was = layout.locate(&item);
    if layout.move_item(&item, to).is_err() || layout.locate(&item) == was {
        return Vec::new();
    }
    // A drop on a collapsed group opens it, to show the row that landed (C4, FR-018).
    if let Some(group) = layout.groups.iter_mut().find(|g| Some(g.id) == spot.list) {
        group.collapsed = false;
    }
    vec![Outcome::SidebarLayoutChanged { project, layout }]
}

/// A dragged group released over `spot`: it takes that place among the groups, its rows with it.
/// Its own place, or a group that is gone, changes nothing.
fn group_released(
    state: &crate::app::State,
    project: std::path::PathBuf,
    id: UserGroupId,
    spot: &DropSpot,
) -> Vec<Outcome> {
    let (Some(slot), Some(held)) = (spot.slot, state.sidebar_layout()) else {
        return Vec::new();
    };
    let mut layout = held.clone();
    if layout.move_group(id, slot).is_err() || layout == *held {
        return Vec::new();
    }
    vec![Outcome::SidebarLayoutChanged { project, layout }]
}

/// The layout's place for `spot`: the stored position of the row it names in its list. A row that
/// is no longer in that list names no place.
fn drop_target(layout: &SidebarLayout, spot: &DropSpot) -> Option<DropTarget> {
    let at = |item: &LayoutItem| {
        layout
            .locate(item)
            .filter(|(list, _)| *list == spot.list)
            .map(|(_, at)| at)
    };
    let index = match (&spot.before, &spot.after) {
        (Some(before), _) => at(before)?,
        (None, Some(after)) => at(after)? + 1,
        (None, None) => 0,
    };
    Some(match spot.list {
        Some(id) => DropTarget::Group { id, index },
        None => DropTarget::Ungrouped { index },
    })
}

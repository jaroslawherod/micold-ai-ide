//! The session bar's figures stay whole inside the transcript pane (feature 944, T037, SC-005).
//!
//! The covered state `session-transcript-insights` (a structured session's pane with the meter in
//! its warning, the session totals, one background task running and a subagent thread open) is
//! laid out twice. In the layout window the meter with "Near compaction", the totals and the
//! running-task count lie on one row; at the smallest window the application allows, beside the
//! default sidebar, the row may wrap, but each of the three is drawn whole and inside the pane.
//! A figure cut, or hanging off the pane, is one the user cannot read at a glance.
//!
//! Compiled into the `layout_snapshot` binary beside `transcript_jump_control_fits`.

use crate::support::covered_states::transcript_pane_insights;
use crate::support::layout::{self as lay, Overflow, StateUnderTest, WINDOW};
use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{layout, Layout};
use iced::{Rectangle, Size, Vector};
use micold_client::app::MIN_WINDOW_SIZE;
use micold_client::ui::transcript::{transcript_scroll_id, NEAR_COMPACTION};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The bounds of the scrollable named `id`.
struct Find(Id, Option<Rectangle>);

impl<T> Operation<T> for Find {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.0) {
            self.1 = Some(bounds);
        }
    }
}

/// The transcript's viewport in a window `window` large: the pane's width, under the bar.
fn viewport(under: &StateUnderTest, window: Size) -> Rectangle {
    let session = under.state.session.active.expect("a session is active");
    let renderer = lay::renderer();
    let mut element = lay::view_of(under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, window);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut find = Find(transcript_scroll_id(session), None);
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut find);
    find.1.expect("the pane holds the transcript's scrollable")
}

/// The meter's label, the totals and the running count, as painted in a window `window` large,
/// each checked whole and inside the pane's width.
fn bar_parts(window: Size) -> [Overflow; 3] {
    let under = StateUnderTest::new(transcript_pane_insights());
    let pane = viewport(&under, window);
    let (painted, _) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
    let find = |what: &str, pick: &dyn Fn(&str) -> bool| -> Overflow {
        let part = painted
            .iter()
            .find(|p| pick(&p.content))
            .unwrap_or_else(|| panic!("the session bar draws {what} at {window:?}"))
            .clone();
        assert!(
            part.natural_width <= part.allowed_width + TOLERANCE,
            "{what} {:?} is drawn whole at {window:?}: wants {} of {}",
            part.content,
            part.natural_width,
            part.allowed_width
        );
        assert!(
            part.origin.x >= pane.x - TOLERANCE
                && part.origin.x + part.natural_width <= pane.x + pane.width + TOLERANCE,
            "{what} {:?} at x {} ({} wide) lies inside the pane {pane:?} at {window:?}",
            part.content,
            part.origin.x,
            part.natural_width
        );
        assert!(
            part.origin.y < pane.y,
            "{what} sits above the transcript's entries"
        );
        part
    };
    [
        find("the meter with its warning", &|c| {
            c.contains(NEAR_COMPACTION)
        }),
        find("the totals", &|c| c.contains(" est. · ")),
        find("the running count", &|c| {
            c.ends_with("background task running")
        }),
    ]
}

#[test]
fn the_meter_totals_and_running_count_share_one_row_in_the_window() {
    let parts = bar_parts(WINDOW);
    let row = parts[0].origin.y;
    for part in &parts {
        assert!(
            (part.origin.y - row).abs() <= TOLERANCE,
            "{:?} at y {} lies on the meter's row at y {row}",
            part.content,
            part.origin.y
        );
    }
}

#[test]
fn at_the_smallest_window_each_figure_is_whole_inside_the_pane() {
    let _ = bar_parts(MIN_WINDOW_SIZE);
}

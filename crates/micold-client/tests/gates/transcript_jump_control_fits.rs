//! "Jump to latest" floats inside the transcript it resumes (feature 939, T038, FR-017).
//!
//! The covered state `session-transcript-pane` (a structured session's pane, detached from the
//! end) is laid out in the whole window. The control must lie inside the transcript's viewport,
//! above its bottom edge, and over no part of the scrollbar along its trailing edge: a control
//! under the bar, or hanging off the pane, is one the user cannot reliably press.
//!
//! Compiled into the `layout_snapshot` binary beside `disclosure_card_header_fits`.

use crate::support::covered_states::transcript_pane_detached;
use crate::support::layout::{self as lay, LayoutRecord, StateUnderTest, WINDOW};
use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{layout, Layout};
use iced::{Rectangle, Vector};
use micold_client::ui::transcript::{transcript_scroll_id, JUMP_TO_LATEST};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The width the shared scrollable's floating bar takes along its trailing edge, margins included
/// (`material/scrollable.rs`'s `BAR_FOOTPRINT`: a 4 px bar and 1 px either side; `material` is
/// private to the crate).
const BAR_FOOTPRINT: f32 = 6.0;

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

fn viewport(under: &StateUnderTest, id: Id) -> Rectangle {
    let renderer = lay::renderer();
    let mut element = lay::view_of(under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(iced::Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut find = Find(id, None);
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut find);
    find.1.expect("the pane holds the transcript's scrollable")
}

#[test]
fn jump_to_latest_lies_inside_the_transcript_clear_of_its_scrollbar() {
    let under = StateUnderTest::new(transcript_pane_detached());
    let session = under.state.session.active.expect("a session is active");
    let viewport = viewport(&under, transcript_scroll_id(session));

    let mut renderer = lay::renderer();
    let (painted, records) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    let label = painted
        .iter()
        .find(|p| p.content == JUMP_TO_LATEST)
        .unwrap_or_else(|| panic!("a detached transcript draws {JUMP_TO_LATEST:?}"));
    // The control is the largest node under the label's origin that is smaller than half the
    // viewport each way: the button, not the full-size layer that floats it.
    let at = label.origin;
    let control: &LayoutRecord = records
        .iter()
        .filter(|r| at.x >= r.x && at.x < r.x + r.width && at.y >= r.y && at.y < r.y + r.height)
        .filter(|r| r.width < viewport.width / 2.0 && r.height < viewport.height / 2.0)
        .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
        .expect("the label sits in a control");

    let (left, top) = (control.x, control.y);
    let (right, bottom) = (control.x + control.width, control.y + control.height);
    assert!(
        left >= viewport.x - TOLERANCE && top >= viewport.y - TOLERANCE,
        "{control:?} starts inside the viewport {viewport:?}"
    );
    assert!(
        bottom < viewport.y + viewport.height - TOLERANCE,
        "{control:?} sits above the viewport's bottom edge {viewport:?}"
    );
    assert!(
        right <= viewport.x + viewport.width - BAR_FOOTPRINT + TOLERANCE,
        "{control:?} covers no part of the scrollbar ({BAR_FOOTPRINT} px along {viewport:?}'s \
         trailing edge)"
    );
}

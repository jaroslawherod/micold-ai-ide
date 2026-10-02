//! The picker's list following its highlight (feature 038, FR-007, SC-007).
//!
//! Up and Down move an index in the reducer, which is render-free and cannot know where a row is.
//! While every row was one menu item tall the list could have been scrolled by arithmetic on that
//! index; an issue's row is as tall as its wrapping lines come to, so the row has to be found on
//! the laid-out tree instead. The shell chains [`picker_highlight_into_view`] after the move.
//!
//! # Why it is not [`focus::into_view`](super::focus)
//!
//! The arithmetic is that module's, shared. The two differ in what they look for and in which
//! panel they move:
//!
//! - The highlight is not focus. The keyboard stays in the search field while the highlight moves
//!   through the list, so there is no focused widget to find; the highlighted row carries
//!   [`PICKER_HIGHLIGHT`] instead.
//! - The focus operation moves every panel whose content overlaps the control, which is right
//!   while everything is in one tree. The list is floated over the window, so its rows overlap the
//!   form under them in window coordinates without being inside the form's panel. This operation
//!   therefore moves only the panel the row was found in.

use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, Scrollable};
use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::{operate, Id, Operation};
use iced::{Rectangle, Task, Vector};

use super::focus::delta_into_view;
use super::material::PICKER_HIGHLIGHT;

/// Scroll the open list so its highlighted row is wholly in view. A row already in view moves
/// nothing, and neither does a list with no highlight.
pub fn picker_highlight_into_view<M: Send + 'static>() -> Task<M> {
    operate(into_view())
}

/// The operation itself, for a caller that drives it against a widget tree: a [`Task`] is opaque,
/// so `tests/picker_highlight_into_view.rs` could not otherwise tell whether the two passes meet.
///
/// Two passes for the reason `focus::scroll_focused_into_view` gives: a panel is told about itself
/// before its children are traversed, so the pass that finds the row has already gone by the panel
/// that has to move.
pub fn into_view<T>() -> impl Operation<T> {
    FindHighlight::default()
}

/// Pass one: where is the highlighted row, and which panel is it in?
#[derive(Default)]
struct FindHighlight {
    /// How many panels the pass has met, which numbers them: both passes walk the same tree in
    /// the same order, so the count names a panel to the second pass.
    panels: usize,
    /// The panel just met, until the traversal into its content claims it.
    entering: Option<usize>,
    /// For each traversal the pass is inside, the panel whose content it is, if it is one's.
    inside: Vec<Option<usize>>,
    found: Option<Highlight>,
}

/// The highlighted row and the panel around it.
#[derive(Clone, Copy)]
struct Highlight {
    row: Rectangle,
    panel: usize,
}

impl<T> Operation<T> for FindHighlight {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        self.inside.push(self.entering.take());
        operate(self);
        self.inside.pop();
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.entering = Some(self.panels);
        self.panels += 1;
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        // A container traverses into its own content next, and that is not a panel's.
        self.entering = None;
        if id != Some(&*PICKER_HIGHLIGHT) {
            return;
        }
        // The innermost panel: a list inside a scrolling form is moved, and the form is not.
        if let Some(panel) = self.inside.iter().rev().flatten().next() {
            self.found = Some(Highlight {
                row: bounds,
                panel: *panel,
            });
        }
    }

    fn finish(&self) -> Outcome<T> {
        match self.found {
            Some(highlight) => Outcome::Chain(Box::new(ShowHighlight {
                highlight,
                panels: 0,
            })),
            // No list is open, or nothing in it is highlighted.
            None => Outcome::None,
        }
    }
}

/// Pass two: move that panel.
struct ShowHighlight {
    highlight: Highlight,
    panels: usize,
}

impl<T> Operation<T> for ShowHighlight {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        let panel = self.panels;
        self.panels += 1;
        if panel != self.highlight.panel {
            return;
        }

        let delta = delta_into_view(
            self.highlight.row,
            bounds.height,
            content_bounds.y,
            translation.y,
        );
        if delta != 0.0 {
            state.scroll_by(AbsoluteOffset { x: 0.0, y: delta }, bounds, content_bounds);
        }
    }
}

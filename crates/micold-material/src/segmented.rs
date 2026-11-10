//! `SegmentedControl` — a group of mutually exclusive choices drawn as one row of buttons
//! (Constitution Principle VIII; feature 1793, R4): the worktree menu's Structured / Terminal view
//! control.
//!
//! Exactly one segment is selected. Pressing the selected segment sends nothing; pressing another
//! sends its message. Every segment is a keyboard stop with its own tooltip, which doubles as its
//! name.
//!
//! Exposed as a chainable builder terminating in `.into()`:
//! `SegmentedControl::new(roles).segment("Structured", true, A).segment("Terminal", false, B).into()`.

use crate::{Button, Tooltip};
use iced::widget::row;
use iced::Element;
use micold_core::tokens::{spacing, Roles};

/// One choice of a [`SegmentedControl`].
pub struct Segment<M> {
    label: String,
    selected: bool,
    on_select: M,
}

impl<M> Segment<M> {
    /// The segment's label, which is also its accessible name.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the segment is the selected one.
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// What pressing the segment sends: nothing while it is selected.
    pub fn press(&self) -> Option<&M> {
        (!self.selected).then_some(&self.on_select)
    }
}

/// A row of mutually exclusive segments. See the module documentation.
pub struct SegmentedControl<M> {
    roles: Roles,
    segments: Vec<Segment<M>>,
}

impl<M> SegmentedControl<M> {
    /// An empty control, themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            segments: Vec::new(),
        }
    }

    /// Append a segment: `label`, drawn selected when `selected`, sending `on_select` when another
    /// one is pressed.
    pub fn segment(mut self, label: impl Into<String>, selected: bool, on_select: M) -> Self {
        self.segments.push(Segment {
            label: label.into(),
            selected,
            on_select,
        });
        self
    }
}

impl<'a, M: Clone + 'a> From<SegmentedControl<M>> for Element<'a, M> {
    fn from(control: SegmentedControl<M>) -> Self {
        let r = control.roles;
        let segments = control.segments.into_iter().map(|segment| {
            let button = Button::outlined(segment.label.clone(), r)
                .selected(segment.selected)
                .on_press_maybe(segment.press().cloned());
            Tooltip::new(button, segment.label.clone(), r).into()
        });
        row(segments).spacing(spacing::XS).into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{roles, Mounted};
    use super::*;

    fn control(selected: usize) -> SegmentedControl<&'static str> {
        SegmentedControl::new(roles())
            .segment("Structured", selected == 0, "structured")
            .segment("Terminal", selected == 1, "terminal")
    }

    /// T002: two segments, in order, one selected, each named by its label.
    #[test]
    fn there_are_two_segments_one_selected_and_each_is_named() {
        for selected in [0, 1] {
            let c = control(selected);
            let names: Vec<_> = c.segments.iter().map(Segment::label).collect();
            assert_eq!(names, ["Structured", "Terminal"]);
            let chosen: Vec<_> = c.segments.iter().map(Segment::is_selected).collect();
            assert_eq!(chosen, [selected == 0, selected == 1]);
        }
    }

    /// T002: only the unselected segment sends its message.
    #[test]
    fn only_the_other_segment_sends_a_message() {
        let c = control(0);
        assert_eq!(
            c.segments[0].press(),
            None,
            "the selected one sends nothing"
        );
        assert_eq!(c.segments[1].press(), Some(&"terminal"));
        let c = control(1);
        assert_eq!(c.segments[0].press(), Some(&"structured"));
        assert_eq!(c.segments[1].press(), None);
    }

    /// T002: the control builds into an element that lays out with both segments.
    #[test]
    fn it_lays_out_as_one_row_of_both_segments() {
        let strings = SegmentedControl::new(roles())
            .segment("Structured", true, String::from("a"))
            .segment("Terminal", false, String::from("b"));
        let mounted = Mounted::new(Element::from(strings));
        assert!(mounted.container().width > 0.0);
    }
}

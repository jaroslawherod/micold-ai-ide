//! `FocusWithin` — report when the keyboard moves among a region's controls (feature 941, T061
//! visual pass F5).
//!
//! # Why this exists
//!
//! A text field says when it takes and loses the keyboard; a button does not. In a region that
//! mixes the two — the composer's text area beside **Send** — Tab from the field lands on the
//! button, and the field's blur is the only report: the window then holds no field, hands the
//! keyboard back to the pane (`State::terminal_focused`), and `KeyboardElsewhere` clears the
//! button's focus on the next event, so Tab never visibly leaves the field.
//!
//! Wrapped around the region, this asks after every event which of its focusables holds the
//! keyboard (a walk of the region only), and reports each change: `true` while one of them holds
//! it, `false` when none does. The reducer then holds the region as the focused field while no
//! field inside it holds the keyboard itself.
//!
//! The walk runs on every event, not only on presses: a focus move is an operation (Tab's
//! `focus_next`, a task focusing a field) that any later event, a redraw included, is the first
//! to see. The region is the composer, a few dozen widgets.
//!
//! It holds no appearance and adds no layout node, which is why it lives in the `cdk`.

use crate::ui::cdk::passthrough::forward_to_child;
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{tree, Id, Operation, Tree, Widget};
use iced::advanced::{mouse, Clipboard, Layout, Shell};
use iced::{Element, Event, Rectangle};

/// Content whose focus changes are reported through `on_change`.
pub struct FocusWithin<'a, M> {
    content: Element<'a, M>,
    on_change: Box<dyn Fn(bool) -> M + 'a>,
}

impl<'a, M> FocusWithin<'a, M> {
    /// Wrap `content`; `on_change(within)` is published each time the focusable holding the
    /// keyboard inside it changes, `within` whether one does.
    pub fn new(content: impl Into<Element<'a, M>>, on_change: impl Fn(bool) -> M + 'a) -> Self {
        Self {
            content: content.into(),
            on_change: Box::new(on_change),
        }
    }
}

/// Which focusable inside held the keyboard as of the last event, by traversal order.
#[derive(Debug, Default)]
struct Seen {
    focused: Option<usize>,
}

/// The traversal index of the focusable holding the keyboard.
#[derive(Default)]
struct FindFocused {
    next: usize,
    focused: Option<usize>,
}

impl Operation for FindFocused {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() && self.focused.is_none() {
            self.focused = Some(self.next);
        }
        self.next += 1;
    }

    fn finish(&self) -> Outcome<()> {
        Outcome::None
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for FocusWithin<'a, M> {
    forward_to_child!(content, M, iced::Theme, iced::Renderer;
        children, diff, size, layout, draw, mouse_interaction, operate, overlay);

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Seen>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Seen::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        // After the content: a press that moved the keyboard has just been handled, and a Tab's
        // move (an operation, between events) shows on the event after it.
        let mut find = FindFocused::default();
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut find as &mut dyn Operation,
        );
        let seen = tree.state.downcast_mut::<Seen>();
        if seen.focused != find.focused {
            seen.focused = find.focused;
            shell.publish((self.on_change)(find.focused.is_some()));
        }
    }
}

impl<'a, M: 'a> From<FocusWithin<'a, M>> for Element<'a, M> {
    fn from(w: FocusWithin<'a, M>) -> Self {
        Element::new(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::Mounted;

    #[test]
    fn each_move_of_the_keyboard_inside_is_reported_and_leaving_reports_false() {
        let fields = iced::widget::column![
            iced::widget::text_input("", "a").on_input(|_| String::new()),
            iced::widget::text_input("", "b").on_input(|_| String::new()),
        ];
        let outside = iced::widget::text_input("", "c").on_input(|_| String::new());
        let region = FocusWithin::new(fields, |within| format!("within {within}"));
        let mut m = Mounted::new(iced::widget::column![region, outside]);
        assert!(m.redraw().is_empty(), "nothing focused, nothing said");
        m.focus_next();
        assert_eq!(m.redraw(), ["within true"], "the first control took it");
        m.focus_next();
        assert_eq!(m.redraw(), ["within true"], "the second control took it");
        m.focus_next();
        assert_eq!(m.redraw(), ["within false"], "the keyboard left the region");
        assert!(m.redraw().is_empty(), "nothing moved, nothing said");
    }
}

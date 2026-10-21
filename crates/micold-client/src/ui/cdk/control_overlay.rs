//! `ControlOverlay` — a control laid over content that may hold controls of its own (feature 1122,
//! research R6, contract U2).
//!
//! A card is a control: a press anywhere on it ripples, takes the keyboard and activates it. A link
//! on that card is a control too, and a press on the link must reach the link alone. Wrapping the
//! card's face in its control chain cannot say that: every wrapper acts on a press in its bounds
//! before its child sees it. So the control is a layer over the face instead, and this wrapper puts
//! the face first:
//!
//! - **Layout**: the content sizes the node; the control is laid out at exactly that size.
//! - **Events**: the content sees each event first. One it captures never reaches the control.
//!   Otherwise the control sees it, with the cursor levitated wherever the content answers a
//!   `mouse_interaction` other than `None` (an interactive child under the pointer), so the control
//!   neither activates nor hovers there.
//! - **Draw**: the content, then the control, given the same levitated cursor.
//! - **Pointer**: the content's interaction where it has one, otherwise the control's.
//! - **Focus order**: the control, then the content, so Tab reaches the card before its link.
//!
//! Unlike iced's `Stack`, which updates its top layer first, nothing here lets the control take a
//! press its content wanted.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Content with a control laid over it. See the module documentation.
pub struct ControlOverlay<'a, M> {
    /// `[content, control]`, in this order: the tree's children follow it.
    layers: [Element<'a, M>; 2],
}

/// Index of the content in [`ControlOverlay::layers`] and in the tree's children.
const CONTENT: usize = 0;
/// Index of the control.
const CONTROL: usize = 1;

impl<'a, M: 'a> ControlOverlay<'a, M> {
    /// `control` laid over `content`, at `content`'s size.
    pub fn new(content: impl Into<Element<'a, M>>, control: impl Into<Element<'a, M>>) -> Self {
        Self {
            layers: [content.into(), control.into()],
        }
    }

    /// The cursor the control sees: levitated wherever the content has an interactive child.
    fn control_cursor(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Cursor {
        let Some(content) = layout.children().next() else {
            return cursor;
        };
        let over_child = self.layers[CONTENT].as_widget().mouse_interaction(
            &tree.children[CONTENT],
            content,
            cursor,
            viewport,
            renderer,
        ) != mouse::Interaction::None;
        if over_child {
            cursor.levitate()
        } else {
            cursor
        }
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for ControlOverlay<'a, M> {
    fn children(&self) -> Vec<Tree> {
        self.layers.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.layers);
    }

    fn size(&self) -> Size<Length> {
        self.layers[CONTENT].as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self.layers[CONTENT].as_widget_mut().layout(
            &mut tree.children[CONTENT],
            renderer,
            limits,
        );
        let size = content.size();
        let control = self.layers[CONTROL].as_widget_mut().layout(
            &mut tree.children[CONTROL],
            renderer,
            &layout::Limits::new(size, size),
        );
        layout::Node::with_children(size, vec![content, control])
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
        let mut parts = layout.children();
        let (Some(content), Some(control)) = (parts.next(), parts.next()) else {
            return;
        };
        self.layers[CONTENT].as_widget_mut().update(
            &mut tree.children[CONTENT],
            event,
            content,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if shell.is_event_captured() {
            return;
        }
        let cursor = self.control_cursor(tree, layout, cursor, viewport, renderer);
        self.layers[CONTROL].as_widget_mut().update(
            &mut tree.children[CONTROL],
            event,
            control,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut parts = layout.children();
        let (Some(content), Some(control)) = (parts.next(), parts.next()) else {
            return;
        };
        self.layers[CONTENT].as_widget().draw(
            &tree.children[CONTENT],
            renderer,
            theme,
            style,
            content,
            cursor,
            viewport,
        );
        let cursor = self.control_cursor(tree, layout, cursor, viewport, renderer);
        self.layers[CONTROL].as_widget().draw(
            &tree.children[CONTROL],
            renderer,
            theme,
            style,
            control,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut parts = layout.children();
        let (Some(content), Some(control)) = (parts.next(), parts.next()) else {
            return mouse::Interaction::None;
        };
        match self.layers[CONTENT].as_widget().mouse_interaction(
            &tree.children[CONTENT],
            content,
            cursor,
            viewport,
            renderer,
        ) {
            mouse::Interaction::None => self.layers[CONTROL].as_widget().mouse_interaction(
                &tree.children[CONTROL],
                control,
                cursor,
                viewport,
                renderer,
            ),
            interaction => interaction,
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            let mut parts = layout.children();
            let (Some(content), Some(control)) = (parts.next(), parts.next()) else {
                return;
            };
            let (contents, controls) = tree.children.split_at_mut(CONTROL);
            let (content_layer, control_layer) = self.layers.split_at_mut(CONTROL);
            control_layer[0].as_widget_mut().operate(
                &mut controls[0],
                control,
                renderer,
                operation,
            );
            content_layer[0].as_widget_mut().operate(
                &mut contents[0],
                content,
                renderer,
                operation,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.layers,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<ControlOverlay<'a, M>> for Element<'a, M> {
    fn from(overlay: ControlOverlay<'a, M>) -> Self {
        Element::new(overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::Mounted;
    use iced::widget::{column, scrollable, Space};
    use iced::Point;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<String>>>;

    /// A block that says what it saw: it publishes `"<name> press <cursor>"` on a primary press,
    /// logs `"<name> drawn <cursor>"` when drawn, answers `interaction` over itself, captures a
    /// press over itself when `captures`, and reports itself to an operation as text `name`.
    struct Probe {
        name: &'static str,
        size: Size<Length>,
        interaction: mouse::Interaction,
        captures: bool,
        log: Log,
    }

    fn probe(name: &'static str, w: Length, h: Length, log: &Log) -> Probe {
        Probe {
            name,
            size: Size::new(w, h),
            interaction: mouse::Interaction::None,
            captures: false,
            log: Rc::clone(log),
        }
    }

    fn seen(cursor: mouse::Cursor, bounds: Rectangle) -> &'static str {
        if cursor.is_levitating() {
            "levitating"
        } else if cursor.is_over(bounds) {
            "over"
        } else {
            "away"
        }
    }

    impl Widget<String, iced::Theme, iced::Renderer> for Probe {
        fn size(&self) -> Size<Length> {
            self.size
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(limits.resolve(self.size.width, self.size.height, Size::ZERO))
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            layout: Layout<'_>,
            cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
            self.log.borrow_mut().push(format!(
                "{} drawn {}",
                self.name,
                seen(cursor, layout.bounds())
            ));
        }

        fn update(
            &mut self,
            _tree: &mut Tree,
            event: &Event,
            layout: Layout<'_>,
            cursor: mouse::Cursor,
            _renderer: &iced::Renderer,
            _clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, String>,
            _viewport: &Rectangle,
        ) {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                let bounds = layout.bounds();
                shell.publish(format!("{} press {}", self.name, seen(cursor, bounds)));
                if self.captures && cursor.is_over(bounds) {
                    shell.capture_event();
                }
            }
        }

        fn mouse_interaction(
            &self,
            _tree: &Tree,
            layout: Layout<'_>,
            cursor: mouse::Cursor,
            _viewport: &Rectangle,
            _renderer: &iced::Renderer,
        ) -> mouse::Interaction {
            if cursor.is_over(layout.bounds()) {
                self.interaction
            } else {
                mouse::Interaction::None
            }
        }

        fn operate(
            &mut self,
            _tree: &mut Tree,
            layout: Layout<'_>,
            _renderer: &iced::Renderer,
            operation: &mut dyn Operation,
        ) {
            operation.text(None, layout.bounds(), self.name);
        }
    }

    impl From<Probe> for Element<'static, String> {
        fn from(p: Probe) -> Self {
            Element::new(p)
        }
    }

    /// Content: a 100 x 20 `link` (Pointer; capturing when `captures`) over a 100 x 60 plain block;
    /// the control fills it and answers `Grab`.
    fn card(captures: bool, log: &Log) -> Mounted<'static> {
        let mut link = probe("link", Length::Fixed(100.0), Length::Fixed(20.0), log);
        link.interaction = mouse::Interaction::Pointer;
        link.captures = captures;
        let content = column![
            link,
            probe("face", Length::Fixed(100.0), Length::Fixed(60.0), log)
        ];
        let mut control = probe("control", Length::Fill, Length::Fill, log);
        control.interaction = mouse::Interaction::Grab;
        Mounted::new(ControlOverlay::new(content, control))
    }

    const ON_LINK: Point = Point::new(10.0, 10.0);
    const ON_FACE: Point = Point::new(10.0, 50.0);

    fn interaction(m: &Mounted<'_>, at: Point) -> mouse::Interaction {
        m.element.as_widget().mouse_interaction(
            &m.tree,
            Layout::new(&m.node),
            mouse::Cursor::Available(at),
            &Rectangle::with_size(m.room),
            &m.renderer,
        )
    }

    #[test]
    fn the_content_sizes_it_and_the_control_is_laid_out_at_that_size() {
        let log = Log::default();
        let m = card(false, &log);
        assert_eq!(m.node.size(), Size::new(100.0, 80.0));
        let control = m.node.children()[CONTROL].bounds();
        assert_eq!(
            control,
            Rectangle::new(Point::ORIGIN, Size::new(100.0, 80.0))
        );
    }

    #[test]
    fn an_event_the_content_captures_never_reaches_the_control() {
        let log = Log::default();
        let mut m = card(true, &log);
        let seen = m.press(ON_LINK);
        assert!(seen.contains(&"link press over".to_string()), "{seen:?}");
        assert!(!seen.iter().any(|s| s.starts_with("control")), "{seen:?}");
    }

    #[test]
    fn the_control_sees_the_rest_levitated_over_an_interactive_child() {
        let log = Log::default();
        let mut m = card(false, &log);
        let on_link = m.press(ON_LINK);
        assert!(
            on_link.contains(&"control press levitating".to_string()),
            "{on_link:?}"
        );
        let on_face = m.press(ON_FACE);
        assert!(
            on_face.contains(&"control press over".to_string()),
            "{on_face:?}"
        );
        assert_eq!(on_face[0], "link press away", "the content sees it first");
    }

    #[test]
    fn it_draws_the_content_then_the_control_with_the_same_cursor() {
        let log = Log::default();
        let mut m = card(false, &log);
        m.drawn_under(iced::Color::WHITE, mouse::Cursor::Available(ON_LINK));
        assert_eq!(
            log.borrow().as_slice(),
            [
                "link drawn over",
                "face drawn away",
                "control drawn levitating"
            ]
        );
        log.borrow_mut().clear();
        m.drawn_under(iced::Color::WHITE, mouse::Cursor::Available(ON_FACE));
        assert_eq!(
            log.borrow().as_slice(),
            ["link drawn away", "face drawn over", "control drawn over"]
        );
    }

    #[test]
    fn the_pointer_is_the_contents_where_it_has_one_else_the_controls() {
        let log = Log::default();
        let m = card(false, &log);
        assert_eq!(interaction(&m, ON_LINK), mouse::Interaction::Pointer);
        assert_eq!(interaction(&m, ON_FACE), mouse::Interaction::Grab);
    }

    #[test]
    fn over_an_inner_scrollbar_the_pointer_is_the_controls() {
        let log = Log::default();
        let body = scrollable(
            Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(400.0)),
        )
        .width(Length::Fixed(100.0))
        .height(Length::Fixed(80.0));
        let mut control = probe("control", Length::Fill, Length::Fill, &log);
        control.interaction = mouse::Interaction::Grab;
        let mut m = Mounted::new(ControlOverlay::new(body, control));
        let bar = Point::new(97.0, 10.0);
        m.hover(bar);
        assert_eq!(interaction(&m, bar), mouse::Interaction::Grab);
    }

    /// The text names an operation visits, in order.
    struct Names(Vec<String>);

    impl Operation<Vec<String>> for Names {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<String>>)) {
            operate(self);
        }

        fn text(&mut self, _id: Option<&iced::advanced::widget::Id>, _b: Rectangle, text: &str) {
            self.0.push(text.to_string());
        }

        fn finish(&self) -> iced::advanced::widget::operation::Outcome<Vec<String>> {
            iced::advanced::widget::operation::Outcome::Some(self.0.clone())
        }
    }

    #[test]
    fn an_operation_visits_the_control_then_the_content() {
        let log = Log::default();
        let mut m = card(false, &log);
        assert_eq!(
            m.drive(Names(Vec::new())).expect("it finishes"),
            ["control", "link", "face"]
        );
    }
}

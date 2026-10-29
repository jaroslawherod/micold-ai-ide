//! `SelectionArea` — the region a text selection is made in: it owns the gesture, the keyboard
//! focus and the edge scroll, while the selectable leaves inside it report where the pointer is
//! (feature 1550, research R5; contracts/selectable-text.md *A-1*–*A-5*; FR-003, FR-004, FR-007,
//! FR-012, FR-013).
//!
//! - **A-1**: a left press a child captured (a button, a toggle, a copy control, a link) publishes
//!   nothing from the area: controls come first (FR-004).
//! - **A-2**: an uncaptured left press inside takes keyboard focus and publishes `on_press` with
//!   the click count, *before* whatever the children published for the same press, so the
//!   gesture's first hit lands in a gesture already begun (FR-007). The children see each mouse
//!   event where it happened, not where the runtime's batch of events ended (#1550 M6).
//! - **A-3**: while `dragging` and the pointer is above (below) the bounds, the children see the
//!   pointer clamped to the top (bottom) row, so the unit at the edge keeps answering, and the area
//!   publishes `on_edge(Up)` (`Down`) on each frame it requests, a frame ahead through
//!   `motion::wake_at`, and only while it is past the edge (FR-003).
//! - **A-4**: a right press anywhere inside, on text or on a control, publishes `on_context` with
//!   where it was and no `on_press` (FR-013).
//! - **A-5**: a left release after a press in the area, or during a drag, publishes `on_release`
//!   and nothing else; the clipboard is the application's to leave alone (FR-012).
//!
//! It is a keyboard focus target (`operation::Focusable`) under the id it is given, and reports
//! focus changes through `on_focus_change`, as the application's focused field must follow it.

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{operation, tree, Id, Operation, Tree, Widget};
use iced::advanced::{mouse, Clipboard, Shell};
use iced::{Element, Event, Point, Rectangle, Size, Vector};

use super::cdk::motion;
use super::selectable_text::Click;

/// Which edge of the area a drag holds the pointer past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Up,
    Down,
}

/// What a right press sends, from where it was in the window.
type OnContext<'a, M> = Rc<dyn Fn((u16, u16)) -> M + 'a>;

/// The region a selection is made in. Builder form (Principle VIII):
/// `SelectionArea::new(content).id(id).dragging(true).on_press(f).on_edge(g).on_release(m)
/// .on_context(h).on_focus_change(k).into()`.
pub struct SelectionArea<'a, M> {
    content: Element<'a, M>,
    id: Option<Id>,
    dragging: bool,
    on_press: Option<Rc<dyn Fn(u8) -> M + 'a>>,
    on_edge: Option<Rc<dyn Fn(Edge) -> M + 'a>>,
    on_release: Option<M>,
    on_context: Option<OnContext<'a, M>>,
    on_focus_change: Option<Rc<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> SelectionArea<'a, M> {
    /// `content`, sending nothing until told what to send.
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            id: None,
            dragging: false,
            on_press: None,
            on_edge: None,
            on_release: None,
            on_context: None,
            on_focus_change: None,
        }
    }

    /// The id a focus operation finds the area by.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Whether a selection drag is in progress (the application's state says so).
    pub fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    /// What an uncaptured left press sends, from its click count (1 to 3).
    pub fn on_press(mut self, f: impl Fn(u8) -> M + 'a) -> Self {
        self.on_press = Some(Rc::new(f));
        self
    }

    /// What each frame of a drag held past an edge sends.
    pub fn on_edge(mut self, f: impl Fn(Edge) -> M + 'a) -> Self {
        self.on_edge = Some(Rc::new(f));
        self
    }

    /// What the left button's release sends.
    pub fn on_release(mut self, message: M) -> Self {
        self.on_release = Some(message);
        self
    }

    /// What a right press sends, from where it was in the window.
    pub fn on_context(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_context = Some(Rc::new(f));
        self
    }

    /// What a change of the area's keyboard focus sends.
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Rc::new(f));
        self
    }
}

/// The area's focus and gesture.
#[derive(Debug, Default)]
struct State {
    focused: bool,
    /// The focus last reported through `on_focus_change`.
    reported: bool,
    /// A left press inside started a gesture that has not been released.
    pressed: bool,
    previous: Option<mouse::Click>,
    /// Where the last move put the pointer, in the window's coordinates.
    pointer: Option<Point>,
    /// From the window's coordinates to the area's: a scrollable around the area shifts the
    /// cursor it hands down by its offset, while a move's `position` stays the window's.
    shift: Vector,
    /// The cursor handed down with the last event that was not a redraw: where its batch ended.
    batch_end: Option<Point>,
    /// The cursor handed down with the last redraw.
    last_redraw: Option<Point>,
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// The edge `p` is past, vertically, when it is outside `bounds`.
fn past(bounds: Rectangle, p: Point) -> Option<Edge> {
    if p.y < bounds.y {
        Some(Edge::Up)
    } else if p.y >= bounds.y + bounds.height {
        Some(Edge::Down)
    } else {
        None
    }
}

/// `p` moved into `bounds`: past the top onto the top row, past the bottom onto the bottom one.
fn clamped(bounds: Rectangle, p: Point) -> Point {
    let inside = |v: f32, from: f32, size: f32| v.clamp(from, from + (size - 1.0).max(0.0));
    Point::new(
        inside(p.x, bounds.x, bounds.width),
        inside(p.y, bounds.y, bounds.height),
    )
}

/// `n` as a window coordinate.
fn coordinate(n: f32) -> u16 {
    n.clamp(0.0, f32::from(u16::MAX)) as u16
}

impl<'a, M: Clone + 'a> Widget<M, iced::Theme, iced::Renderer> for SelectionArea<'a, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.focusable(
            self.id.as_ref(),
            layout.bounds(),
            tree.state.downcast_mut::<State>() as &mut dyn operation::Focusable,
        );
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let bounds = layout.bounds();
        // The runtime hands every event of a batch the cursor where the batch ended, so in a fast
        // drag the press would land where the drag got to (#1550 M6). Each event is seen at its
        // own place instead: a move where it went, a press or release where the last move left the
        // pointer. A frame's redraw is a batch of its own, so when its cursor is where the last
        // batch ended (no moves the area has not seen yet), that cursor less the pointer is what a
        // scrollable around the area shifts positions by. A scroll under a still pointer (a wheel)
        // changes that cursor with no move: two redraws in a row at one cursor outside a gesture
        // teach the new shift too (inside one, a redraw repeats while it publishes hits).
        let at = {
            let state = tree.state.downcast_mut::<State>();
            match event {
                Event::Window(iced::window::Event::RedrawRequested(_)) => {
                    let given = cursor.position();
                    if let (Some(given), Some(pointer)) = (given, state.pointer) {
                        let still = !state.pressed && state.last_redraw == Some(given);
                        if still || state.batch_end == Some(given) {
                            state.shift = given - pointer;
                        }
                    }
                    state.last_redraw = given;
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    state.pointer = Some(*position);
                    state.batch_end = cursor.position();
                }
                Event::Mouse(mouse::Event::CursorLeft) => {
                    state.pointer = None;
                    state.batch_end = None;
                }
                _ => state.batch_end = cursor.position(),
            }
            state.pointer.map(|p| p + state.shift)
        };
        let cursor = match (cursor, at) {
            (mouse::Cursor::Available(_), Some(p)) => mouse::Cursor::Available(p),
            _ => cursor,
        };
        let edge = cursor
            .position()
            .filter(|_| self.dragging)
            .and_then(|p| past(bounds, p));
        let inner = match (edge, cursor.position()) {
            (Some(_), Some(p)) => mouse::Cursor::Available(clamped(bounds, p)),
            _ => cursor,
        };
        // The children's messages wait until the area has published its own: a press's
        // `on_press` must come before the first hit it starts.
        let mut messages = Vec::new();
        let mut inner_shell = Shell::new(&mut messages);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            inner,
            renderer,
            clipboard,
            &mut inner_shell,
            viewport,
        );
        let captured = inner_shell.is_event_captured();
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                match cursor.position_over(bounds) {
                    Some(p) if !captured => {
                        let click = mouse::Click::new(p, mouse::Button::Left, state.previous);
                        state.previous = Some(click);
                        state.focused = true;
                        state.pressed = true;
                        if let Some(on_press) = &self.on_press {
                            shell.publish(on_press(Click::of(click.kind()).count()));
                        }
                    }
                    Some(_) => {}
                    None => state.focused = false,
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let (Some(p), Some(on_context)) =
                    (cursor.position_over(bounds), &self.on_context)
                {
                    shell.publish(on_context((coordinate(p.x), coordinate(p.y))));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed || self.dragging =>
            {
                state.pressed = false;
                if let Some(message) = &self.on_release {
                    shell.publish(message.clone());
                }
            }
            Event::Window(iced::window::Event::RedrawRequested(_)) => {
                if let (Some(edge), Some(on_edge)) = (edge, &self.on_edge) {
                    shell.publish(on_edge(edge));
                }
            }
            _ => {}
        }
        // Past an edge during a drag, the next frame is asked for a frame ahead, and only then:
        // the edge scroll stops asking once the pointer comes back or the button is released.
        let frames = matches!(
            event,
            Event::Window(iced::window::Event::RedrawRequested(_))
                | Event::Mouse(mouse::Event::CursorMoved { .. })
        );
        if frames && edge.is_some() && self.on_edge.is_some() {
            motion::wake_at(shell, std::time::Instant::now() + motion::FRAME);
        }
        if state.focused != state.reported {
            state.reported = state.focused;
            if let Some(on_focus_change) = &self.on_focus_change {
                shell.publish(on_focus_change(state.focused));
            }
        }
        shell.merge(inner_shell, |m| m);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: Clone + 'a> From<SelectionArea<'a, M>> for Element<'a, M> {
    fn from(area: SelectionArea<'a, M>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use super::super::selectable_text::Hit;
    use super::super::test_support::{roles, Mounted};
    use super::super::{Button, Text, TypeRole};
    use super::*;
    use iced::widget::column;

    const TEXT_ROW: f32 = 20.0;

    /// A text row, then a button: the area's content.
    fn area(dragging: bool) -> SelectionArea<'static, String> {
        let r = roles();
        let text = Text::new("abcdef", TypeRole::Body, r)
            .monospace()
            .selectable("t", None, |slot: &str, hit: Hit| {
                format!("hit {slot} {}", hit.offset)
            });
        let content = column![text, Button::filled("Go", r).on_press("button".to_owned())];
        SelectionArea::new(content)
            .dragging(dragging)
            .on_press(|clicks| format!("press {clicks}"))
            .on_edge(|edge| format!("edge {edge:?}"))
            .on_release("release".to_owned())
            .on_context(|(x, y)| format!("context {x} {y}"))
            .on_focus_change(|focused| format!("focus {focused}"))
    }

    fn mounted(dragging: bool) -> Mounted<'static> {
        Mounted::new_in(area(dragging), Size::new(300.0, 120.0))
    }

    fn button_centre(m: &mut Mounted<'_>) -> Point {
        m.node.children()[1].bounds().center()
    }

    fn press_with(m: &mut Mounted<'_>, button: mouse::Button, at: Point) -> Vec<String> {
        m.send(
            Event::Mouse(mouse::Event::ButtonPressed(button)),
            mouse::Cursor::Available(at),
        )
    }

    fn release(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
        m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// A-1: a press a child captured publishes nothing from the area.
    #[test]
    fn a_press_a_control_took_publishes_nothing_from_the_area() {
        let mut m = mounted(false);
        let at = button_centre(&mut m);
        let mut out = m.press(at);
        out.extend(release(&mut m, at));
        assert_eq!(out, vec!["button".to_owned()]);
    }

    /// A-2: an uncaptured press takes focus and publishes `on_press` before the hit it starts.
    #[test]
    fn a_press_on_text_takes_focus_and_comes_before_the_hit() {
        let mut m = mounted(false);
        let out = m.press(Point::new(2.0, TEXT_ROW / 2.0));
        assert_eq!(out, vec!["press 1", "focus true", "hit t 0"]);
        release(&mut m, Point::new(2.0, TEXT_ROW / 2.0));
        let again = m.press(Point::new(2.0, TEXT_ROW / 2.0));
        assert_eq!(again[0], "press 2", "a second click in place");
    }

    /// A-2: inside a scrolled scrollable the cursor handed down is shifted by the offset while a
    /// move's position stays the window's; a press in a batch that ends further on still lands
    /// where it happened, in the area's coordinates (#1550 M6).
    #[test]
    fn a_press_in_a_scrolled_area_lands_where_it_happened() {
        let mut m = mounted(false);
        let scrolled = Vector::new(0.0, 300.0);
        let at = Point::new(2.0, TEXT_ROW / 2.0);
        let moved = |p: Point| {
            Event::Mouse(mouse::Event::CursorMoved {
                position: p - scrolled,
            })
        };
        m.send(moved(at), mouse::Cursor::Available(at));
        m.send(
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            mouse::Cursor::Available(at),
        );
        // One batch: the press, then the drag's move, all handed the cursor where it ended.
        let end = Point::new(250.0, TEXT_ROW / 2.0);
        let out = press_with(&mut m, mouse::Button::Left, end);
        assert_eq!(out, vec!["press 1", "focus true", "hit t 0"]);
    }

    /// A-2: a redraw can come while moves the area has not seen yet wait (its cursor is already
    /// where they go); it teaches no shift then, so the press after them lands where it happened.
    #[test]
    fn a_redraw_ahead_of_the_moves_shifts_nothing() {
        let row = TEXT_ROW / 2.0;
        let (a, b, c) = (
            Point::new(2.0, row),
            Point::new(20.0, row),
            Point::new(40.0, row),
        );
        let expected = mounted(false).press(c);
        let mut m = mounted(false);
        let moved = |p: Point| Event::Mouse(mouse::Event::CursorMoved { position: p });
        let redraw = || {
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            ))
        };
        m.send(moved(a), mouse::Cursor::Available(a));
        m.send(redraw(), mouse::Cursor::Available(a));
        m.send(moved(b), mouse::Cursor::Available(b));
        // The window's cursor is already at `c`; the move there is still queued.
        m.send(redraw(), mouse::Cursor::Available(c));
        m.send(moved(c), mouse::Cursor::Available(c));
        assert_eq!(press_with(&mut m, mouse::Button::Left, c), expected);
    }

    /// A-2: a wheel scroll under a still pointer shifts the cursor handed down with no move; two
    /// redraws at the new cursor teach the new shift, so a press without a move lands right.
    #[test]
    fn a_press_after_a_scroll_with_no_move_lands_where_it_happened() {
        let mut m = mounted(false);
        let at = Point::new(2.0, TEXT_ROW / 2.0);
        let scrolled = Vector::new(0.0, 300.0);
        let redraw = || {
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            ))
        };
        // Before the scroll the window position is the area's (no shift); the scroll then brings
        // the text's start under the still pointer.
        let before = at - scrolled;
        m.send(
            Event::Mouse(mouse::Event::CursorMoved { position: before }),
            mouse::Cursor::Available(before),
        );
        m.send(redraw(), mouse::Cursor::Available(before));
        // The wheel event is handed the old offset.
        let wheel = mouse::ScrollDelta::Lines { x: 0.0, y: -3.0 };
        m.send(
            Event::Mouse(mouse::Event::WheelScrolled { delta: wheel }),
            mouse::Cursor::Available(before),
        );
        m.send(redraw(), mouse::Cursor::Available(at));
        m.send(redraw(), mouse::Cursor::Available(at));
        let out = press_with(&mut m, mouse::Button::Left, at);
        assert_eq!(out, vec!["press 1", "focus true", "hit t 0"]);
    }

    /// A-2: a press outside the area gives the focus up.
    #[test]
    fn a_press_outside_gives_the_focus_up() {
        let mut m = Mounted::new(area(false));
        m.press(Point::new(2.0, TEXT_ROW / 2.0));
        release(&mut m, Point::new(2.0, TEXT_ROW / 2.0));
        let out = m.press(Point::new(2.0, 700.0));
        assert_eq!(out, vec!["focus false"]);
    }

    /// A-3: past the top (bottom) while dragging, the children see the pointer on the top row and
    /// each requested frame publishes `on_edge`.
    #[test]
    fn a_drag_past_an_edge_clamps_the_pointer_and_scrolls_each_frame() {
        let mut m = Mounted::new_in(area(true), Size::new(300.0, 120.0));
        let bounds = m.node.bounds();
        let above = Point::new(30.0, bounds.y - 40.0);
        let (out, redraw) = m.tick(std::time::Instant::now(), mouse::Cursor::Available(above));
        assert!(out.contains(&"edge Up".to_owned()), "{out:?}");
        assert_ne!(redraw, iced::window::RedrawRequest::Wait, "the next frame");
        let below = Point::new(2.0, bounds.y + bounds.height + 40.0);
        let (out, _) = m.tick(std::time::Instant::now(), mouse::Cursor::Available(below));
        assert!(out.contains(&"edge Down".to_owned()), "{out:?}");
        // The bottom pixel row is `bottom - 1`: a pointer exactly at `bottom` is already past it.
        let bottom = bounds.y + bounds.height;
        let (out, _) = m.tick(
            std::time::Instant::now(),
            mouse::Cursor::Available(Point::new(2.0, bottom)),
        );
        assert!(out.contains(&"edge Down".to_owned()), "{out:?}");
        let (out, _) = m.tick(
            std::time::Instant::now(),
            mouse::Cursor::Available(Point::new(2.0, bottom - 1.0)),
        );
        assert!(!out.contains(&"edge Down".to_owned()), "{out:?}");

        // The text at the top row answers the clamped pointer during a drag.
        let mut m = Mounted::new_in(area(true), Size::new(300.0, 120.0));
        m.press(Point::new(200.0, TEXT_ROW / 2.0));
        let out = m.hover(Point::new(2.0, -50.0));
        assert!(out.contains(&"hit t 0".to_owned()), "{out:?}");
    }

    /// A-3: without a drag, a pointer outside publishes no edge.
    #[test]
    fn no_edge_without_a_drag() {
        let mut m = mounted(false);
        let (out, _) = m.tick(
            std::time::Instant::now(),
            mouse::Cursor::Available(Point::new(2.0, -40.0)),
        );
        assert!(out.is_empty(), "{out:?}");
    }

    /// A-4: a right press on text or on a control publishes `on_context` and no `on_press`.
    #[test]
    fn a_right_press_opens_the_context_menu_on_text_and_on_controls() {
        let mut m = mounted(false);
        let on_text = press_with(&mut m, mouse::Button::Right, Point::new(4.0, 6.0));
        assert_eq!(on_text, vec!["context 4 6"]);
        let at = button_centre(&mut m);
        let on_button = press_with(&mut m, mouse::Button::Right, at);
        assert_eq!(
            on_button,
            vec![format!("context {} {}", at.x as u16, at.y as u16)]
        );
    }

    /// A-5: the left release publishes `on_release` and nothing else.
    #[test]
    fn a_left_release_publishes_on_release_only() {
        let mut m = mounted(false);
        m.press(Point::new(2.0, TEXT_ROW / 2.0));
        assert_eq!(
            release(&mut m, Point::new(2.0, TEXT_ROW / 2.0)),
            vec!["release"]
        );
    }

    /// The area is a focus target under its id.
    #[test]
    fn the_area_is_a_focus_target() {
        let mut m = Mounted::new(area(false).id("transcript"));
        let found = m
            .drive(super::super::test_support::Focusables(0))
            .unwrap_or(0);
        assert!(found >= 1, "the area counts as a focusable");
    }
}

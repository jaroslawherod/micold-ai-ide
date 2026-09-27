//! The tooltip's behaviour half (029 BUG-001, FR-013).
//!
//! A hover label floated beside its trigger. Where it floats is the whole of this module: the panel
//! opens on the side it was asked for and, when that side has no room for it, on the other side of
//! the trigger — never over the trigger itself.
//!
//! The rendering stack's own tooltip placed the panel on the asked-for side and then, to keep it
//! inside the window, slid it back by the shortfall. For a trigger at the window's bottom edge that
//! slide is straight up, onto the trigger: the last row of a full sidebar disappeared under the
//! panel describing it. The stack's tooltip cannot be told otherwise — its side is fixed at
//! construction and its overlay cannot be moved from outside — so the flip needs an overlay of its
//! own, and an overlay of its own lives here.
//!
//! **It decides nothing about appearance.** The panel arrives already styled from
//! `material::Tooltip`; this module lays it out and draws it where it belongs.
//!
//! This is one of the modules in `cdk/` besides `overlay` that floats its own content, and
//! `tests/one_overlay_implementation.rs` holds it to saying why.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{window, Element, Event, Length, Padding, Point, Rectangle, Size, Vector};

/// Which side of its trigger a tooltip asks for. The rendering stack's own type, so the material
/// layer's public `TooltipPosition` keeps meaning what it always meant.
pub use iced::widget::tooltip::Position;

/// The space kept between the panel and the window's edge, and around the panel's content — the
/// figure the rendering stack's tooltip uses, kept so that nothing that already fits moves.
const EDGE_PADDING: f32 = 5.0;

/// A trigger with a hover label floated beside it.
pub struct Tooltip<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    tooltip: Element<'a, M, Theme, Renderer>,
    position: Position,
    gap: f32,
}

impl<'a, M, Theme, Renderer> Tooltip<'a, M, Theme, Renderer> {
    /// Float `tooltip` beside `content` on the `position` side while the pointer is over `content`.
    pub fn new(
        content: impl Into<Element<'a, M, Theme, Renderer>>,
        tooltip: impl Into<Element<'a, M, Theme, Renderer>>,
        position: Position,
    ) -> Self {
        Self {
            content: content.into(),
            tooltip: tooltip.into(),
            position,
            gap: 0.0,
        }
    }

    /// The space between the trigger and the panel.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// Whether the panel is showing, and where the pointer was when it last moved over the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum State {
    #[default]
    Idle,
    Open {
        cursor: Point,
    },
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Tooltip<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.tooltip)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content, &self.tooltip]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// Only the trigger is laid out; the panel never takes space from its neighbours.
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_)) = event {
            let state = tree.state.downcast_mut::<State>();
            match (*state, cursor.position_over(layout.bounds())) {
                (State::Idle, Some(at)) => {
                    *state = State::Open { cursor: at };
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                (State::Open { cursor: last }, Some(at))
                    if self.position == Position::FollowCursor && last != at =>
                {
                    *state = State::Open { cursor: at };
                    shell.request_redraw();
                }
                (State::Open { .. }, None) => {
                    *state = State::Idle;
                    shell.invalidate_layout();
                    if !matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
                        shell.request_redraw();
                    }
                }
                (State::Open { .. }, Some(_)) | (State::Idle, None) => {}
            }
        }

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
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
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

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    /// The trigger's own overlay (a picker inside it, say), plus the panel while it is open.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let state = *tree.state.downcast_ref::<State>();
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().expect("the trigger's tree"),
            layout,
            renderer,
            viewport,
            translation,
        );

        let panel = match state {
            State::Open { cursor } => {
                let bounds = layout.bounds();
                Some(overlay::Element::new(Box::new(Panel {
                    tooltip: &mut self.tooltip,
                    tree: children.next().expect("the panel's tree"),
                    trigger: Rectangle {
                        x: bounds.x + translation.x,
                        y: bounds.y + translation.y,
                        ..bounds
                    },
                    cursor: cursor + translation,
                    position: self.position,
                    gap: self.gap,
                })))
            }
            State::Idle => None,
        };

        if content.is_none() && panel.is_none() {
            return None;
        }
        Some(overlay::Group::with_children(content.into_iter().chain(panel).collect()).overlay())
    }
}

/// The floating panel: where it sits, and nothing else — it takes no input.
struct Panel<'a, 'b, M, Theme, Renderer> {
    tooltip: &'b mut Element<'a, M, Theme, Renderer>,
    tree: &'b mut Tree,
    /// The trigger's on-screen rectangle; everything is placed from it.
    trigger: Rectangle,
    cursor: Point,
    position: Position,
    gap: f32,
}

impl<M, Theme, Renderer> overlay::Overlay<M, Theme, Renderer> for Panel<'_, '_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let content = self.tooltip.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds).shrink(Padding::new(EDGE_PADDING)),
        );

        let panel = place(
            self.position,
            self.trigger,
            content.size(),
            bounds,
            self.gap,
            self.cursor,
        );

        layout::Node::with_children(
            panel.size(),
            vec![content.translate(Vector::new(EDGE_PADDING, EDGE_PADDING))],
        )
        .translate(Vector::new(panel.x, panel.y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(content) = layout.children().next() {
            self.tooltip.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                content,
                cursor,
                &Rectangle::with_size(Size::INFINITE),
            );
        }
    }
}

/// Where a panel whose content measures `content` goes, beside `trigger`, in a window of `window`.
///
/// On the `position` side, kept inside the window. When that lands on the trigger, the opposite
/// side instead (FR-013). When the panel fits neither, the side with more room, where the window's
/// slide covers the least of the trigger.
fn place(
    position: Position,
    trigger: Rectangle,
    content: Size,
    window: Size,
    gap: f32,
    cursor: Point,
) -> Rectangle {
    let asked = inside(beside(position, trigger, content, gap, cursor), window);
    let Some(other) = opposite(position) else {
        return asked;
    };
    if !overlaps(asked, trigger) {
        return asked;
    }
    let flipped = inside(beside(other, trigger, content, gap, cursor), window);
    if !overlaps(flipped, trigger) || room(other, trigger, window) > room(position, trigger, window)
    {
        flipped
    } else {
        asked
    }
}

/// How far the window extends past `trigger` on `side`.
fn room(side: Position, trigger: Rectangle, window: Size) -> f32 {
    match side {
        Position::Top => trigger.y,
        Position::Bottom => window.height - (trigger.y + trigger.height),
        Position::Left => trigger.x,
        Position::Right => window.width - (trigger.x + trigger.width),
        Position::FollowCursor => 0.0,
    }
}

/// The other side of the trigger. A panel that follows the cursor has none.
fn opposite(position: Position) -> Option<Position> {
    match position {
        Position::Top => Some(Position::Bottom),
        Position::Bottom => Some(Position::Top),
        Position::Left => Some(Position::Right),
        Position::Right => Some(Position::Left),
        Position::FollowCursor => None,
    }
}

/// The panel on `side` of the trigger, before the window has any say: centred along the trigger,
/// `gap` away from it, padded by [`EDGE_PADDING`] all round.
fn beside(side: Position, trigger: Rectangle, content: Size, gap: f32, cursor: Point) -> Rectangle {
    let size = Size::new(
        content.width + EDGE_PADDING * 2.0,
        content.height + EDGE_PADDING * 2.0,
    );
    let centred_x = trigger.x + (trigger.width - size.width) / 2.0;
    let centred_y = trigger.y + (trigger.height - size.height) / 2.0;
    let origin = match side {
        Position::Top => Point::new(centred_x, trigger.y - gap - size.height),
        Position::Bottom => Point::new(centred_x, trigger.y + trigger.height + gap),
        Position::Left => Point::new(trigger.x - gap - size.width, centred_y),
        Position::Right => Point::new(trigger.x + trigger.width + gap, centred_y),
        Position::FollowCursor => Point::new(
            cursor.x - EDGE_PADDING,
            cursor.y - content.height - EDGE_PADDING,
        ),
    };
    Rectangle::new(origin, size)
}

/// `panel`, slid back inside a window of `window` where it pokes out.
fn inside(mut panel: Rectangle, window: Size) -> Rectangle {
    panel.x = panel.x.min(window.width - panel.width).max(0.0);
    panel.y = panel.y.min(window.height - panel.height).max(0.0);
    panel
}

/// Whether two rectangles share any area — touching edges do not count.
fn overlaps(a: Rectangle, b: Rectangle) -> bool {
    let x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    x > 0.0 && y > 0.0
}

impl<'a, M, Theme, Renderer> From<Tooltip<'a, M, Theme, Renderer>>
    for Element<'a, M, Theme, Renderer>
where
    M: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(t: Tooltip<'a, M, Theme, Renderer>) -> Self {
        Element::new(t)
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    const GAP: f32 = 4.0;

    /// A panel too tall for either side of its trigger stays on the side with more room: the
    /// window then has to slide it onto the trigger, and the side with more room is the one where
    /// it covers the least of it (plan § Bugfix BUG-001, the fallback clause).
    #[test]
    fn a_panel_that_fits_neither_side_takes_the_side_with_more_room() {
        let window = Size::new(400.0, 150.0);
        // 70px above the trigger, 20px below it; the panel needs 90.
        let trigger = Rectangle::new(Point::new(100.0, 70.0), Size::new(100.0, 60.0));
        let content = Size::new(80.0, 80.0);

        let panel = place(Position::Top, trigger, content, window, GAP, Point::ORIGIN);

        assert_eq!(
            panel.y, 0.0,
            "asked for the top, which has 70px of room to the bottom's 20, so the panel stays at \
             the top of the window rather than flipping to the side where it would cover the whole \
             trigger: got {panel:?}",
        );
    }

    /// The flip is across the trigger on either axis: a panel asked for on the right of a trigger
    /// at the window's right edge opens on its left, clear of it (plan § Bugfix BUG-001, "every
    /// tooltip").
    #[test]
    fn a_panel_with_no_room_on_the_right_opens_on_the_left() {
        let window = Size::new(400.0, 300.0);
        let trigger = Rectangle::new(Point::new(340.0, 100.0), Size::new(60.0, 40.0));
        let content = Size::new(120.0, 30.0);

        let panel = place(
            Position::Right,
            trigger,
            content,
            window,
            GAP,
            Point::ORIGIN,
        );

        assert!(
            panel.x + panel.width <= trigger.x,
            "no room to the right of a trigger at the window's right edge, so the panel opens to \
             its left, clear of it: trigger {trigger:?}, panel {panel:?}",
        );
    }
}

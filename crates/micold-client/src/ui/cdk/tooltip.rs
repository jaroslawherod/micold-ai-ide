//! The tooltip's behaviour half (029 BUG-001, FR-013).
//!
//! A hover label floated beside its trigger. Where it floats is the whole of this module: the panel
//! opens on the side it was asked for and, when that side has no room for it, on the other side of
//! the trigger rather than over it.
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

use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{window, Element, Event, Length, Padding, Point, Rectangle, Size, Vector};

use micold_core::tooltip::{RestTimer, ShowTimer};

use super::motion::{self, Progress};

/// Which side of its trigger a tooltip asks for, or that it follows the pointer.
///
/// Its own type rather than the rendering stack's: a side flips across the trigger when it has no
/// room (029 FR-013), which the stack's tooltip cannot be told to do. [`Position::FollowCursor`] has
/// no side: it is placed by [`place_at_pointer`] beside the pointer instead of by [`place`] beside
/// the trigger (feature 430).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Above the trigger.
    Top,
    /// Below the trigger — the default.
    Bottom,
    /// To the left of the trigger.
    Left,
    /// To the right of the trigger.
    Right,
    /// Beside the pointer, moving with it while it is over the trigger.
    FollowCursor,
}

/// What a tooltip waits for before it opens.
///
/// A mode rather than a flag per delay: the setter called last wins (430 research R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Wait {
    /// Open while the pointer is over the trigger.
    #[default]
    Hover,
    /// Open once the pointer has been over the trigger for the delay, counted from entering and not
    /// restarted by movement (feature 430).
    Delay(Duration),
    /// Open once the pointer has rested on the trigger for the delay (feature 038).
    Rest(Duration),
}

/// The space kept between the panel and the window's edge, and around the panel's content — the
/// figure the rendering stack's tooltip uses, kept so that nothing that already fits moves.
pub const EDGE_PADDING: f32 = 5.0;

/// The part of a laid-out `panel` that is drawn: inside its [`EDGE_PADDING`]. Shared with the
/// tests, which place the pointer against what the user sees.
pub fn visible_part(panel: Rectangle) -> Rectangle {
    Rectangle::new(
        Point::new(panel.x + EDGE_PADDING, panel.y + EDGE_PADDING),
        Size::new(
            panel.width - EDGE_PADDING * 2.0,
            panel.height - EDGE_PADDING * 2.0,
        ),
    )
}

/// A trigger with a hover label floated beside it.
pub struct Tooltip<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    tooltip: Element<'a, M, Theme, Renderer>,
    position: Position,
    gap: f32,
    /// What the pointer must do over the trigger before the panel opens.
    wait: Wait,
    /// What the trigger describes, when the same place in the tree can come to describe another
    /// thing.
    subject: Option<u64>,
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
            wait: Wait::Hover,
            subject: None,
        }
    }

    /// The space between the trigger and the panel.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Open only after the cursor has rested on the trigger for `delay` (feature 038, FR-015).
    ///
    /// Resting is the rule of `micold_core::tooltip::RestTimer`: movement beyond a small tolerance
    /// starts the wait again, a press on the trigger closes the panel until the cursor has left,
    /// and leaving closes it. The wait costs one timed frame request, not a frame loop (FR-018).
    pub fn after_rest(mut self, delay: Duration) -> Self {
        self.wait = Wait::Rest(delay);
        self
    }

    /// Open only once the pointer has been over the trigger for `delay`, counted from entering
    /// (feature 430, FR-003).
    ///
    /// Unlike [`Tooltip::after_rest`] movement does not restart it; only leaving does. It works
    /// with [`Position::FollowCursor`]: the panel then opens beside the pointer's place at that
    /// moment. An alternative to `after_rest`: the one called last wins. The wait costs one timed
    /// frame request, not a frame loop.
    pub fn show_delay(mut self, delay: Duration) -> Self {
        self.wait = Wait::Delay(delay);
        self
    }

    /// What the trigger describes (FR-017).
    ///
    /// A list reuses its rows: the widget at one place in the tree describes one issue now and
    /// another after the list narrows or scrolls. When `key` changes, an open panel closes and a
    /// rest-delay wait starts from nothing, as it would for a trigger the cursor had just reached.
    pub fn subject(mut self, key: u64) -> Self {
        self.subject = Some(key);
        self
    }
}

/// Whether the panel is showing.
///
/// `shown` exists only to ask for the frame that paints an open or a close. The rendering layer
/// asks for frames through `Progress` alone (`tests/idle_requests_no_frames.rs`), so the change is
/// a transition that takes no time: it asks for one frame, arrives on it, and asks for nothing
/// more.
#[derive(Debug, Clone, Copy, PartialEq)]
struct State {
    open: bool,
    shown: Progress,
    /// Where a rest-delay tooltip is in its wait. Unused without `after_rest`.
    rest: RestTimer,
    /// Where a show-delay tooltip is in its wait, and the press rule of a pointer-following one that
    /// opens on hover. Unused otherwise.
    show: ShowTimer,
    /// The pointer over the trigger, in the trigger's own coordinate space; read by `overlay()`.
    /// Recorded only for [`Position::FollowCursor`].
    pointer: Option<Point>,
    /// The subject the open panel, or the wait under way, belongs to.
    subject: Option<u64>,
}

impl State {
    fn describing(subject: Option<u64>) -> Self {
        Self {
            open: false,
            shown: Progress::new(0.0),
            rest: RestTimer::default(),
            show: ShowTimer::default(),
            pointer: None,
            subject,
        }
    }

    /// The trigger describes `subject` now. Another one than before closes the panel and forgets
    /// the wait: both belonged to what was described before.
    fn describe(&mut self, subject: Option<u64>) {
        if self.subject != subject {
            self.subject = subject;
            self.rest.reset();
            self.show.reset();
            self.open = false;
        }
    }
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Tooltip<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::describing(self.subject))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.tooltip)]
    }

    /// A changed subject closes the panel here, before the rebuilt view is laid out, so the panel
    /// of the old subject is never laid out over the new one.
    fn diff(&self, tree: &mut Tree) {
        tree.state.downcast_mut::<State>().describe(self.subject);
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
            state.describe(self.subject);
            let follow = self.position == Position::FollowCursor;
            let bounds = layout.bounds();
            let (open, wake) = match self.wait {
                Wait::Hover if !follow => (cursor.is_over(bounds), None),
                wait => {
                    // Observed on every redraw as well as on every mouse event: a list that
                    // scrolls or narrows moves the trigger from under a cursor that did not move.
                    let over = cursor.position_over(bounds);
                    // A redraw carries its instant; a mouse event carries none.
                    let now = match event {
                        Event::Window(window::Event::RedrawRequested(now)) => *now,
                        _ => Instant::now(),
                    };
                    // The press is the trigger's own and is passed on below, not captured.
                    let pressed = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
                        && over.is_some();
                    let rest = match wait {
                        Wait::Rest(delay) => {
                            if pressed {
                                state.rest.press();
                            }
                            let at = over.map(|point| (point.x, point.y));
                            state.rest.observe(at, now, delay)
                        }
                        // A show delay counts from entering; pointer-following opening on hover is
                        // the same rule with no delay, for its press rule (430 research R5).
                        Wait::Delay(_) | Wait::Hover => {
                            let delay = match wait {
                                Wait::Delay(delay) => delay,
                                _ => Duration::ZERO,
                            };
                            if pressed {
                                state.show.press();
                            }
                            state.show.observe(over.is_some(), now, delay)
                        }
                    };
                    if follow && state.pointer != over {
                        state.pointer = over;
                        // Only a panel that is showing has a place to move; the open or close
                        // below does its own relayout. The layout change is what gets it painted:
                        // a frame asked for outside `Progress` would break idle quiescence.
                        if state.open && rest.open {
                            shell.invalidate_layout();
                        }
                    }
                    (rest.open, rest.wake_at)
                }
            };
            if open != state.open {
                state.open = open;
                shell.invalidate_layout();
            }
            let target = if state.open { 1.0 } else { 0.0 };
            state.shown.on_frame(event, target, Duration::ZERO, shell);
            if let Some(end_of_wait) = wake {
                motion::wake_at(shell, end_of_wait);
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
        let (open, pointer) = {
            let state = tree.state.downcast_ref::<State>();
            (state.open, state.pointer)
        };
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().expect("the trigger's tree"),
            layout,
            renderer,
            viewport,
            translation,
        );

        let bounds = layout.bounds();
        let trigger = Rectangle {
            x: bounds.x + translation.x,
            y: bounds.y + translation.y,
            ..bounds
        };
        // A pointer-following panel needs a pointer to follow: none, and it is not shown.
        let follow = match self.position {
            Position::FollowCursor => pointer_to_follow(
                pointer.map(|at| Point::new(at.x + translation.x, at.y + translation.y)),
                trigger,
                viewport.size(),
            )
            .map(Some),
            _ => Some(None),
        };
        let panel = follow.filter(|_| open).map(|pointer| {
            overlay::Element::new(Box::new(Panel {
                tooltip: &mut self.tooltip,
                tree: children.next().expect("the panel's tree"),
                trigger,
                pointer,
                position: self.position,
                gap: self.gap,
            }))
        });

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
    /// Where the pointer is on screen: `Some` exactly for [`Position::FollowCursor`].
    pointer: Option<Point>,
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

        let panel = match self.pointer {
            Some(pointer) => place_at_pointer(pointer, content.size(), bounds, self.gap),
            None => place(
                self.position,
                self.trigger,
                content.size(),
                bounds,
                self.gap,
            ),
        };

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
) -> Rectangle {
    let asked = inside(beside(position, trigger, content, gap), window);
    if !covers(asked, trigger) {
        return asked;
    }
    let other = opposite(position);
    let flipped = inside(beside(other, trigger, content, gap), window);
    if !covers(flipped, trigger) || room(other, trigger, window) > room(position, trigger, window) {
        flipped
    } else {
        asked
    }
}

/// Where a panel whose content measures `content` goes beside the pointer, in a window of `window`.
///
/// Per axis, after the pointer (right, below) with its visible edge `gap` from it; when that runs
/// past the window's edge, before it (left, above); when neither fits, the side with more room.
/// The window then slides the panel back inside. The panel never sits under the pointer while the
/// window has room for it beside the pointer on either axis (feature 430, FR-002).
fn place_at_pointer(pointer: Point, content: Size, window: Size, gap: f32) -> Rectangle {
    let size = Size::new(
        content.width + EDGE_PADDING * 2.0,
        content.height + EDGE_PADDING * 2.0,
    );
    let origin = Point::new(
        along(pointer.x, size.width, window.width, gap),
        along(pointer.y, size.height, window.height, gap),
    );
    inside(Rectangle::new(origin, size), window)
}

/// Where a panel `extent` long starts along one axis of a window `window` long, for a pointer at
/// `at`: after the pointer when it fits there, else before it, else on the side with more room.
fn along(at: f32, extent: f32, window: f32, gap: f32) -> f32 {
    // The panel is padded, but only the part inside the padding is visible: `gap` is measured to it.
    let after = at + gap - EDGE_PADDING;
    let before = at - gap + EDGE_PADDING - extent;
    if after + extent <= window {
        after
    } else if before >= 0.0 {
        before
    } else if window - at >= at {
        after
    } else {
        before
    }
}

/// The pointer a pointer-following panel is placed from, when there is one to follow: it is known,
/// the trigger has an area and the window has a size. Otherwise nothing is shown (contract,
/// Behaviour 6).
fn pointer_to_follow(pointer: Option<Point>, trigger: Rectangle, window: Size) -> Option<Point> {
    let pointer = pointer.filter(|at| at.x.is_finite() && at.y.is_finite())?;
    let sized = |w: f32, h: f32| w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0;
    (sized(trigger.width, trigger.height) && sized(window.width, window.height)).then_some(pointer)
}

/// How far the window extends past `trigger` on `side`.
fn room(side: Position, trigger: Rectangle, window: Size) -> f32 {
    match side {
        Position::Top => trigger.y,
        Position::Bottom | Position::FollowCursor => window.height - (trigger.y + trigger.height),
        Position::Left => trigger.x,
        Position::Right => window.width - (trigger.x + trigger.width),
    }
}

/// The other side of the trigger.
fn opposite(position: Position) -> Position {
    match position {
        Position::Top => Position::Bottom,
        Position::Bottom | Position::FollowCursor => Position::Top,
        Position::Left => Position::Right,
        Position::Right => Position::Left,
    }
}

/// The panel on `side` of the trigger, before the window has any say: centred along the trigger,
/// `gap` away from it, padded by [`EDGE_PADDING`] all round.
fn beside(side: Position, trigger: Rectangle, content: Size, gap: f32) -> Rectangle {
    let size = Size::new(
        content.width + EDGE_PADDING * 2.0,
        content.height + EDGE_PADDING * 2.0,
    );
    let centred_x = trigger.x + (trigger.width - size.width) / 2.0;
    let centred_y = trigger.y + (trigger.height - size.height) / 2.0;
    let origin = match side {
        Position::Top => Point::new(centred_x, trigger.y - gap - size.height),
        Position::Bottom | Position::FollowCursor => {
            Point::new(centred_x, trigger.y + trigger.height + gap)
        }
        Position::Left => Point::new(trigger.x - gap - size.width, centred_y),
        Position::Right => Point::new(trigger.x + trigger.width + gap, centred_y),
    };
    Rectangle::new(origin, size)
}

/// `panel`, slid back inside a window of `window` where it pokes out.
fn inside(mut panel: Rectangle, window: Size) -> Rectangle {
    panel.x = panel.x.min(window.width - panel.width).max(0.0);
    panel.y = panel.y.min(window.height - panel.height).max(0.0);
    panel
}

/// Whether the visible part of `panel` — inside its [`EDGE_PADDING`], which is drawn as nothing —
/// shares any area with `trigger`. Touching edges do not count.
fn covers(panel: Rectangle, trigger: Rectangle) -> bool {
    let (left, top) = (panel.x + EDGE_PADDING, panel.y + EDGE_PADDING);
    let (right, bottom) = (
        panel.x + panel.width - EDGE_PADDING,
        panel.y + panel.height - EDGE_PADDING,
    );
    let x = right.min(trigger.x + trigger.width) - left.max(trigger.x);
    let y = bottom.min(trigger.y + trigger.height) - top.max(trigger.y);
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

    fn tip() -> Tooltip<'static, ()> {
        Tooltip::new(
            iced::widget::text("a"),
            iced::widget::text("b"),
            Position::Bottom,
        )
    }

    /// 430 research R1: the two waits are alternatives and the one set last wins.
    #[test]
    fn the_wait_set_last_wins() {
        let d = Duration::from_secs(1);
        assert_eq!(tip().after_rest(d).show_delay(d).wait, Wait::Delay(d));
        assert_eq!(tip().show_delay(d).after_rest(d).wait, Wait::Rest(d));
        assert_eq!(tip().wait, Wait::Hover);
    }

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

        let panel = place(Position::Top, trigger, content, window, GAP);

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

        let panel = place(Position::Right, trigger, content, window, GAP);

        assert!(
            panel.x + panel.width <= trigger.x,
            "no room to the right of a trigger at the window's right edge, so the panel opens to \
             its left, clear of it: trigger {trigger:?}, panel {panel:?}",
        );
    }

    // ---- the pointer-following placement (feature 430) ------------------------------------------

    const WINDOW: Size = Size::new(400.0, 300.0);
    const CONTENT: Size = Size::new(120.0, 30.0);

    use super::visible_part as visible;

    fn contains_pointer(panel: Rectangle, pointer: Point) -> bool {
        let v = visible(panel);
        pointer.x > v.x
            && pointer.x < v.x + v.width
            && pointer.y > v.y
            && pointer.y < v.y + v.height
    }

    /// US1.1: with room, the panel is after the pointer on both axes, its visible edge exactly `gap`
    /// from it.
    #[test]
    fn a_pointer_panel_sits_beside_the_pointer_offset_by_the_gap() {
        let pointer = Point::new(100.0, 80.0);

        let v = visible(place_at_pointer(pointer, CONTENT, WINDOW, GAP));

        assert_eq!(
            v.x,
            pointer.x + GAP,
            "visible left edge is `gap` right of the pointer"
        );
        assert_eq!(
            v.y,
            pointer.y + GAP,
            "visible top edge is `gap` below the pointer"
        );
    }

    /// US1.2: no room on the right, so the panel opens on the pointer's left.
    #[test]
    fn a_pointer_panel_with_no_room_on_the_right_flips_left() {
        let pointer = Point::new(380.0, 100.0);

        let v = visible(place_at_pointer(pointer, CONTENT, WINDOW, GAP));

        assert_eq!(
            v.x + v.width,
            pointer.x - GAP,
            "visible right edge is `gap` left of it"
        );
        assert_eq!(v.y, pointer.y + GAP, "vertically still below");
    }

    /// US1.2: no room below, so the panel opens above the pointer.
    #[test]
    fn a_pointer_panel_with_no_room_below_flips_above() {
        let pointer = Point::new(100.0, 285.0);

        let v = visible(place_at_pointer(pointer, CONTENT, WINDOW, GAP));

        assert_eq!(
            v.y + v.height,
            pointer.y - GAP,
            "visible bottom edge is `gap` above it"
        );
        assert_eq!(v.x, pointer.x + GAP, "horizontally still after it");
    }

    /// US3.2: in the window's corner it flips on both axes, stays inside and is clear of the pointer.
    #[test]
    fn a_pointer_panel_in_the_corner_flips_on_both_axes() {
        let pointer = Point::new(395.0, 295.0);

        let panel = place_at_pointer(pointer, CONTENT, WINDOW, GAP);
        let v = visible(panel);

        assert_eq!(v.x + v.width, pointer.x - GAP);
        assert_eq!(v.y + v.height, pointer.y - GAP);
        assert!(panel.x >= 0.0 && panel.y >= 0.0, "inside: {panel:?}");
        assert!(panel.x + panel.width <= WINDOW.width && panel.y + panel.height <= WINDOW.height);
    }

    /// Neither side fits on an axis: the side with more room, slid back inside.
    #[test]
    fn a_pointer_panel_that_fits_neither_side_takes_the_side_with_more_room() {
        let window = Size::new(400.0, 150.0);
        // 110px of panel in 50px above and 100px below the pointer.
        let content = Size::new(120.0, 100.0);
        let below = place_at_pointer(Point::new(100.0, 50.0), content, window, GAP);
        let above = place_at_pointer(Point::new(100.0, 100.0), content, window, GAP);

        assert_eq!(
            below.y, 40.0,
            "more room below: after the pointer, slid up to fit: {below:?}"
        );
        assert_eq!(
            above.y, 0.0,
            "more room above: before the pointer, slid down to fit: {above:?}"
        );
        assert!(
            !contains_pointer(below, Point::new(100.0, 50.0)),
            "x axis stays clear"
        );
    }

    /// Edge case: a window smaller than the panel keeps it as far inside as it goes.
    #[test]
    fn a_pointer_panel_in_a_window_too_small_stays_inside() {
        let panel = place_at_pointer(
            Point::new(30.0, 20.0),
            Size::new(80.0, 60.0),
            Size::new(60.0, 40.0),
            GAP,
        );

        assert_eq!((panel.x, panel.y), (0.0, 0.0), "{panel:?}");
    }

    /// SC-001, US1: across the window, inside it and never under the pointer.
    #[test]
    fn a_pointer_panel_is_inside_the_window_and_never_under_the_pointer() {
        for ix in 0..=40 {
            for iy in 0..=30 {
                let pointer = Point::new(ix as f32 * 10.0, iy as f32 * 10.0);

                let panel = place_at_pointer(pointer, CONTENT, WINDOW, GAP);

                assert!(
                    panel.x >= 0.0
                        && panel.y >= 0.0
                        && panel.x + panel.width <= WINDOW.width
                        && panel.y + panel.height <= WINDOW.height,
                    "pointer {pointer:?} placed {panel:?} outside the window",
                );
                assert!(
                    !contains_pointer(panel, pointer),
                    "pointer {pointer:?} placed {panel:?} under it",
                );
            }
        }
    }

    /// Behaviour 6: nothing to follow, nothing opens.
    #[test]
    fn a_pointer_panel_needs_a_pointer_a_trigger_with_an_area_and_a_window_with_a_size() {
        let at = Some(Point::new(10.0, 10.0));
        let trigger = Rectangle::new(Point::new(0.0, 0.0), Size::new(100.0, 40.0));
        let no_width = Rectangle::new(Point::ORIGIN, Size::new(0.0, 40.0));
        let no_height = Rectangle::new(Point::ORIGIN, Size::new(100.0, 0.0));

        assert_eq!(
            pointer_to_follow(at, trigger, WINDOW),
            at,
            "the working case"
        );
        assert_eq!(pointer_to_follow(None, trigger, WINDOW), None, "no pointer");
        assert_eq!(
            pointer_to_follow(at, no_width, WINDOW),
            None,
            "a trigger with no width"
        );
        assert_eq!(
            pointer_to_follow(at, no_height, WINDOW),
            None,
            "a trigger with no height"
        );
        assert_eq!(
            pointer_to_follow(at, trigger, Size::ZERO),
            None,
            "a window of no size"
        );
        assert_eq!(
            pointer_to_follow(at, trigger, Size::new(f32::INFINITY, 300.0)),
            None
        );
    }

    // ---- every fixed placement keeps clear of its trigger (029 FR-013; 430 SC-003, SC-006) ------

    /// US3.1: a trigger at each of the window's four edges, every side asked for: the panel does not
    /// cover the trigger, as the window has room on one side or the other for it.
    #[test]
    fn every_fixed_placement_keeps_clear_of_a_trigger_at_each_window_edge() {
        let size = Size::new(100.0, 40.0);
        let triggers = [
            ("left", Rectangle::new(Point::new(0.0, 130.0), size)),
            ("right", Rectangle::new(Point::new(300.0, 130.0), size)),
            ("top", Rectangle::new(Point::new(150.0, 0.0), size)),
            ("bottom", Rectangle::new(Point::new(150.0, 260.0), size)),
        ];
        for position in [
            Position::Top,
            Position::Bottom,
            Position::Left,
            Position::Right,
        ] {
            for (edge, trigger) in triggers {
                let panel = place(position, trigger, CONTENT, WINDOW, GAP);

                assert!(
                    !covers(panel, trigger),
                    "{position:?} over a trigger at the {edge} edge covers it: {panel:?}",
                );
            }
        }
    }
}

//! `SplitView` — lays a [`PaneLayout`]'s panes out side by side and stacked, and draws the dividers
//! between them (feature 484, FR-001).
//!
//! Layout and divider drawing only: which terminal a pane shows, which pane has focus and what a
//! pane draws are the caller's. Children are given in tree order (`PaneLayout::panes`), one per
//! leaf. Gestures (feature 484, FR-005, FR-008) are reported, never applied: dragging a divider,
//! a double press on it, and dragging a pane's header onto another pane. Each child's layout must
//! lead with its header (the first child of the child's layout).

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size};
use micold_core::pane_layout::{Axis, Divider, PaneId, PaneLayout, Placement, Rect};
use micold_core::tokens::Roles;

use super::style;

/// The visible rule between two panes.
pub const RULE: f32 = 1.0;

/// The divider's hit area across its axis: wide enough to grab with a pointer (the sidebar handle's
/// width), centred on the rule.
pub const HIT: f32 = super::resize_handle::WIDTH;

/// A divider's grab zone: the divider's line widened to [`HIT`] across its axis.
pub fn hit_area(d: Divider) -> Rectangle {
    let r = d.rect;
    match d.axis {
        Axis::Vertical => Rectangle {
            x: r.x - (HIT - r.w) / 2.0,
            y: r.y,
            width: HIT,
            height: r.h,
        },
        Axis::Horizontal => Rectangle {
            x: r.x,
            y: r.y - (HIT - r.h) / 2.0,
            width: r.w,
            height: HIT,
        },
    }
}

/// Where everything goes in an area of `size`, with `min` the smallest pane.
pub fn placement(layout: &PaneLayout, size: Size, min: (f32, f32)) -> Placement {
    layout.place((size.width, size.height), min)
}

/// How a pane header shows focus: its fill and the colour of its leading accent strip. Colour is
/// never the only cue: the strip is a shape, and an unfocused header has none.
pub fn pane_focus_mark(focused: bool, r: Roles) -> (iced::Color, Option<iced::Color>) {
    if focused {
        (
            style::color(r.secondary_container),
            Some(style::color(r.primary)),
        )
    } else {
        (style::color(r.surface), None)
    }
}

/// What the user did to the split, for the caller to apply to its [`PaneLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitEvent {
    /// The pointer dragged divider `index` (the order of [`Placement::dividers`]) to this share
    /// of its split, in ten-thousandths, already within both children's minimum sizes.
    Drag {
        /// Which divider.
        index: usize,
        /// The first child's share, 0..=10000.
        basis_points: u16,
    },
    /// The drag ended.
    Release,
    /// A double press on divider `index`: back to equal sizes.
    Reset(usize),
    /// A pane's header was dropped on another pane: swap their terminals.
    Swap(PaneId, PaneId),
}

/// Where things are, relative to the widget's top-left.
struct Geometry {
    placement: Placement,
    headers: Vec<(PaneId, Rectangle)>,
}

fn contains(r: Rect, p: Point) -> bool {
    p.x >= r.x && p.x < r.x + r.w && p.y >= r.y && p.y < r.y + r.h
}

/// The pointer state machine: divider drags, double presses and header drags.
#[derive(Default)]
struct Gesture {
    dragging: Option<usize>,
    header: Option<PaneId>,
    last_click: Option<mouse::Click>,
}

impl Gesture {
    fn divider_at(g: &Geometry, p: Point) -> Option<usize> {
        g.placement
            .dividers
            .iter()
            .position(|d| hit_area(*d).contains(p))
    }

    /// Divider gestures. `Some` means the event is the split view's: do not pass it on.
    fn divider(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        g: &Geometry,
        layout: &PaneLayout,
        size: Size,
        min: (f32, f32),
    ) -> Option<Option<SplitEvent>> {
        let Event::Mouse(e) = event else {
            return None;
        };
        match e {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let i = Self::divider_at(g, cursor?)?;
                let click = mouse::Click::new(cursor?, mouse::Button::Left, self.last_click);
                if click.kind() == mouse::click::Kind::Double {
                    self.last_click = None;
                    self.dragging = None;
                    return Some(Some(SplitEvent::Reset(i)));
                }
                self.last_click = Some(click);
                self.dragging = Some(i);
                Some(None)
            }
            mouse::Event::CursorMoved { .. } => {
                let i = self.dragging?;
                let p = cursor?;
                let d = g.placement.dividers.get(i)?;
                let (at, start, extent) = match d.axis {
                    Axis::Vertical => (p.x, d.area.x, d.area.w),
                    Axis::Horizontal => (p.y, d.area.y, d.area.h),
                };
                if extent <= 0.0 {
                    return Some(None);
                }
                let mut trial = layout.clone();
                trial.set_ratio(i, (at - start) / extent, (size.width, size.height), min);
                let ratio = trial.ratio(i)?;
                Some(Some(SplitEvent::Drag {
                    index: i,
                    basis_points: (ratio * 10_000.0).round() as u16,
                }))
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                self.dragging.take().map(|_| Some(SplitEvent::Release))
            }
            _ => self.dragging.map(|_| None),
        }
    }

    /// Header gestures, after the children had their turn (`captured`: one of them took the event,
    /// as a header button does).
    fn header(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        g: &Geometry,
        captured: bool,
    ) -> Option<SplitEvent> {
        let Event::Mouse(e) = event else {
            return None;
        };
        match e {
            mouse::Event::ButtonPressed(mouse::Button::Left) if !captured => {
                let p = cursor?;
                self.header = g
                    .headers
                    .iter()
                    .find(|(_, r)| r.contains(p))
                    .map(|(id, _)| *id);
                None
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let src = self.header.take()?;
                let p = cursor?;
                let (target, _) = g
                    .placement
                    .panes
                    .iter()
                    .find(|(id, r)| *id != src && contains(*r, p))?;
                Some(SplitEvent::Swap(src, *target))
            }
            _ => None,
        }
    }
}

/// The panes the children were last diffed against, in order, and the pointer gesture in flight.
struct State {
    ids: Vec<PaneId>,
    gesture: Gesture,
}

/// The panes of a layout, tiled; builder form (Principle VIII):
/// `SplitView::new(&layout, min, roles, children).into()`.
pub struct SplitView<'a, M> {
    layout: PaneLayout,
    ids: Vec<PaneId>,
    min: (f32, f32),
    roles: Roles,
    children: Vec<Element<'a, M>>,
    on_event: Option<Box<dyn Fn(SplitEvent) -> M + 'a>>,
}

impl<'a, M> SplitView<'a, M> {
    /// `children` are one per pane, in `layout.panes()` order.
    pub fn new(
        layout: &PaneLayout,
        min: (f32, f32),
        roles: Roles,
        children: Vec<Element<'a, M>>,
    ) -> Self {
        Self {
            ids: layout.panes().iter().map(|p| p.id()).collect(),
            layout: layout.clone(),
            min,
            roles,
            children,
            on_event: None,
        }
    }

    /// Report gestures as messages; without it the dividers are fixed and headers do not drag.
    pub fn on_event(mut self, f: impl Fn(SplitEvent) -> M + 'a) -> Self {
        self.on_event = Some(Box::new(f));
        self
    }

    fn geometry(&self, layout: Layout<'_>) -> Geometry {
        let origin = layout.bounds().position();
        let headers = self
            .ids
            .iter()
            .zip(layout.children())
            .filter_map(|(id, l)| {
                let b = l.children().next()?.bounds();
                Some((
                    *id,
                    Rectangle {
                        x: b.x - origin.x,
                        y: b.y - origin.y,
                        ..b
                    },
                ))
            })
            .collect();
        Geometry {
            placement: placement(&self.layout, layout.bounds().size(), self.min),
            headers,
        }
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for SplitView<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            ids: Vec::new(),
            gesture: Gesture::default(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    /// Children are matched to their old state by pane, not by position: a split inserts a pane in
    /// the middle of the tree order, and a positional diff would hand the new pane its neighbour's
    /// measured size and terminal state.
    fn diff(&self, tree: &mut Tree) {
        let before = tree.state.downcast_ref::<State>().ids.clone();
        let mut old: Vec<Option<Tree>> = std::mem::take(&mut tree.children)
            .into_iter()
            .map(Some)
            .collect();
        tree.children = self
            .ids
            .iter()
            .zip(&self.children)
            .map(|(id, child)| {
                let mut t = before
                    .iter()
                    .position(|b| b == id)
                    .and_then(|i| old.get_mut(i).and_then(Option::take))
                    .unwrap_or_else(|| Tree::new(child));
                child.as_widget().diff(&mut t);
                t
            })
            .collect();
        tree.state.downcast_mut::<State>().ids = self.ids.clone();
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.max();
        let placed = placement(&self.layout, size, self.min);
        let nodes = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(placed.panes.iter())
            .map(|((child, state), (_, r))| {
                let fixed = layout::Limits::new(Size::ZERO, Size::new(r.w, r.h));
                child
                    .as_widget_mut()
                    .layout(state, renderer, &fixed)
                    .move_to(Point::new(r.x, r.y))
            })
            .collect();
        layout::Node::with_children(size, nodes)
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
        let origin = layout.bounds().position();
        let relative = |p: Point| Point::new(p.x - origin.x, p.y - origin.y);
        // A moving pointer is judged where the event says it is, which the cursor may not yet.
        let at = match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => Some(relative(*position)),
            _ => cursor.position().map(relative),
        };
        if self.on_event.is_some() {
            let g = self.geometry(layout);
            let size = layout.bounds().size();
            let gesture = &mut tree.state.downcast_mut::<State>().gesture;
            if let Some(out) = gesture.divider(event, at, &g, &self.layout, size, self.min) {
                if let (Some(out), Some(f)) = (out, &self.on_event) {
                    shell.publish(f(out));
                }
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }
        for ((child, state), l) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, l, cursor, renderer, clipboard, shell, viewport,
            );
        }
        if let Some(f) = &self.on_event {
            let g = self.geometry(layout);
            let captured = shell.is_event_captured();
            let gesture = &mut tree.state.downcast_mut::<State>().gesture;
            if let Some(out) = gesture.header(event, at, &g, captured) {
                shell.publish(f(out));
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if self.on_event.is_some() {
            let gesture = &tree.state.downcast_ref::<State>().gesture;
            if gesture.header.is_some() {
                return mouse::Interaction::Grabbing;
            }
            let origin = layout.bounds().position();
            let over = cursor
                .position()
                .map(|p| Point::new(p.x - origin.x, p.y - origin.y));
            let placed = placement(&self.layout, layout.bounds().size(), self.min);
            let hit = gesture.dragging.or_else(|| {
                over.and_then(|p| {
                    placed
                        .dividers
                        .iter()
                        .position(|d| hit_area(*d).contains(p))
                })
            });
            if let Some(d) = hit.and_then(|i| placed.dividers.get(i)) {
                return match d.axis {
                    Axis::Vertical => mouse::Interaction::ResizingHorizontally,
                    Axis::Horizontal => mouse::Interaction::ResizingVertically,
                };
            }
        }
        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((c, s), l)| {
                c.as_widget()
                    .mouse_interaction(s, l, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let origin = layout.bounds().position();
        for ((child, state), l) in self
            .children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style_, l, cursor, viewport);
        }
        let size = layout.bounds().size();
        let placed = placement(&self.layout, size, self.min);
        if let (Some(src), Some(p)) = (
            tree.state.downcast_ref::<State>().gesture.header,
            cursor.position(),
        ) {
            let p = Point::new(p.x - origin.x, p.y - origin.y);
            if let Some((_, r)) = placed
                .panes
                .iter()
                .find(|(id, r)| *id != src && contains(*r, p))
            {
                renderer::Renderer::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds: Rectangle {
                            x: origin.x + r.x,
                            y: origin.y + r.y,
                            width: r.w,
                            height: r.h,
                        },
                        border: iced::Border {
                            color: style::color(self.roles.primary),
                            width: 2.0,
                            radius: 0.0.into(),
                        },
                        ..renderer::Quad::default()
                    },
                    iced::Color::TRANSPARENT,
                );
            }
        }
        for d in placed.dividers {
            let r = d.rect;
            let (w, h) = match d.axis {
                Axis::Vertical => (RULE, r.h),
                Axis::Horizontal => (r.w, RULE),
            };
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: Rectangle {
                        x: origin.x + r.x,
                        y: origin.y + r.y,
                        width: w,
                        height: h,
                    },
                    ..renderer::Quad::default()
                },
                style::separator(self.roles),
            );
        }
    }
}

impl<'a, M: 'a> From<SplitView<'a, M>> for Element<'a, M> {
    fn from(v: SplitView<'a, M>) -> Self {
        Element::new(v)
    }
}

#[cfg(test)]
mod tests {
    //! Geometry of the split view (feature 484, T010): where the panes and dividers go, in both
    //! themes (the divider colour is a role, so each theme resolves it).
    use super::*;
    use micold_core::pane_layout::{MIN_PANE_COLS, MIN_PANE_ROWS};
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::roles;

    const MIN: (f32, f32) = (MIN_PANE_COLS as f32 * 8.0, MIN_PANE_ROWS as f32 * 16.0);

    fn two() -> PaneLayout {
        let mut l = PaneLayout::single();
        let p = l.focused();
        l.split(p, Axis::Vertical, (1000.0, 600.0), MIN, None)
            .unwrap();
        l
    }

    #[test]
    fn two_panes_tile_the_area_side_by_side() {
        let p = placement(&two(), Size::new(1000.0, 600.0), MIN);
        assert_eq!(p.panes.len(), 2);
        let (a, b) = (p.panes[0].1, p.panes[1].1);
        assert_eq!((a.x, a.y, a.h), (0.0, 0.0, 600.0));
        assert!((a.w + b.w - 1000.0).abs() < 0.5, "{a:?} {b:?}");
        assert!((b.x - a.w).abs() < 0.5);
    }

    #[test]
    fn a_divider_hit_area_is_usable_size_and_centred_on_the_rule() {
        let p = placement(&two(), Size::new(1000.0, 600.0), MIN);
        assert_eq!(p.dividers.len(), 1);
        let d = p.dividers[0];
        let hit = hit_area(d);
        assert!(
            hit.width >= 6.0,
            "a grab zone narrower than the sidebar handle: {hit:?}"
        );
        assert_eq!(hit.height, 600.0);
        assert!((hit.x + hit.width / 2.0 - (d.rect.x + d.rect.w / 2.0)).abs() < 0.01);
    }

    #[test]
    fn a_horizontal_divider_is_widened_vertically() {
        let mut l = PaneLayout::single();
        let p = l.focused();
        l.split(p, Axis::Horizontal, (1000.0, 600.0), MIN, None)
            .unwrap();
        let d = placement(&l, Size::new(1000.0, 600.0), MIN).dividers[0];
        let hit = hit_area(d);
        assert!(hit.height >= 6.0 && hit.width == 1000.0, "{hit:?}");
    }

    #[test]
    fn panes_keep_the_minimum_and_a_small_window_scales_them_down_without_overlap() {
        let big = placement(&two(), Size::new(1000.0, 600.0), MIN);
        for (_, r) in &big.panes {
            assert!(r.w >= MIN.0 && r.h >= MIN.1, "{r:?} below {MIN:?}");
        }
        let small = placement(&two(), Size::new(200.0, 100.0), MIN);
        let (a, b) = (small.panes[0].1, small.panes[1].1);
        assert!(a.x + a.w <= b.x + 0.5, "overlap: {a:?} {b:?}");
        assert!(b.x + b.w <= 200.5, "outside the area: {b:?}");
    }

    #[test]
    fn the_divider_colour_resolves_in_both_themes() {
        let light = style::separator(roles(ColorScheme::Light));
        let dark = style::separator(roles(ColorScheme::Dark));
        assert_ne!(
            light, dark,
            "a divider drawn the same in both themes is invisible in one"
        );
    }

    // ---- T026: gestures -----------------------------------------------------------------------

    const SIZE: Size = Size::new(1000.0, 600.0);
    const HEADER: f32 = 30.0;

    fn geometry(l: &PaneLayout) -> Geometry {
        let placement = placement(l, SIZE, MIN);
        let headers = placement
            .panes
            .iter()
            .map(|(id, r)| {
                (
                    *id,
                    Rectangle {
                        x: r.x,
                        y: r.y,
                        width: r.w,
                        height: HEADER,
                    },
                )
            })
            .collect();
        Geometry { placement, headers }
    }

    fn mouse_ev(e: mouse::Event) -> Event {
        Event::Mouse(e)
    }
    fn press() -> Event {
        mouse_ev(mouse::Event::ButtonPressed(mouse::Button::Left))
    }
    fn release() -> Event {
        mouse_ev(mouse::Event::ButtonReleased(mouse::Button::Left))
    }
    fn moved(x: f32, y: f32) -> Event {
        mouse_ev(mouse::Event::CursorMoved {
            position: Point::new(x, y),
        })
    }

    fn drive(g: &mut Gesture, l: &PaneLayout, e: &Event, at: Point) -> Option<Option<SplitEvent>> {
        g.divider(e, Some(at), &geometry(l), l, SIZE, MIN)
    }

    #[test]
    fn dragging_a_divider_reports_its_share_and_the_release() {
        let l = two();
        let mut g = Gesture::default();
        let on = Point::new(500.0, 300.0);
        assert_eq!(
            drive(&mut g, &l, &press(), on),
            Some(None),
            "the press is taken"
        );
        let out = drive(&mut g, &l, &moved(300.0, 300.0), Point::new(300.0, 300.0));
        assert_eq!(
            out,
            Some(Some(SplitEvent::Drag {
                index: 0,
                basis_points: 3000
            }))
        );
        // Events during the drag are the split view's, even away from the divider.
        assert!(drive(&mut g, &l, &moved(10.0, 10.0), Point::new(10.0, 10.0)).is_some());
        assert_eq!(
            drive(&mut g, &l, &release(), Point::new(10.0, 10.0)),
            Some(Some(SplitEvent::Release))
        );
        assert!(drive(&mut g, &l, &moved(10.0, 10.0), Point::new(10.0, 10.0)).is_none());
    }

    /// No pane below the minimum, however far the pointer goes.
    #[test]
    fn a_drag_never_shrinks_a_pane_below_the_minimum() {
        let l = two();
        for (x, side) in [(-500.0, 0), (5000.0, 1)] {
            let mut g = Gesture::default();
            drive(&mut g, &l, &press(), Point::new(500.0, 300.0));
            let Some(Some(SplitEvent::Drag { basis_points, .. })) =
                drive(&mut g, &l, &moved(x, 300.0), Point::new(x, 300.0))
            else {
                panic!("no drag")
            };
            let mut applied = l.clone();
            applied.set_ratio(
                0,
                f32::from(basis_points) / 10_000.0,
                (1.0, 1.0),
                (0.0, 0.0),
            );
            let p = placement(&applied, SIZE, MIN);
            for (_, r) in &p.panes {
                assert!(r.w >= MIN.0 - 0.5, "side {side}: {r:?}");
            }
        }
    }

    #[test]
    fn a_double_press_on_a_divider_resets_it_and_does_not_start_a_drag() {
        let l = two();
        let mut g = Gesture::default();
        let on = Point::new(500.0, 300.0);
        assert_eq!(drive(&mut g, &l, &press(), on), Some(None));
        drive(&mut g, &l, &release(), on);
        assert_eq!(
            drive(&mut g, &l, &press(), on),
            Some(Some(SplitEvent::Reset(0)))
        );
        assert!(g.dragging.is_none());
    }

    #[test]
    fn a_press_off_the_divider_is_the_panes_own() {
        let l = two();
        let mut g = Gesture::default();
        assert!(drive(&mut g, &l, &press(), Point::new(100.0, 300.0)).is_none());
    }

    #[test]
    fn a_header_dropped_on_another_pane_swaps_them() {
        let l = two();
        let ids: Vec<_> = l.panes().iter().map(|p| p.id()).collect();
        let geo = geometry(&l);
        let mut g = Gesture::default();
        let from = Point::new(100.0, 10.0);
        assert_eq!(g.header(&press(), Some(from), &geo, false), None);
        let out = g.header(&release(), Some(Point::new(800.0, 300.0)), &geo, false);
        assert_eq!(out, Some(SplitEvent::Swap(ids[0], ids[1])));
        // Dropped back on itself: nothing.
        g.header(&press(), Some(from), &geo, false);
        assert_eq!(g.header(&release(), Some(from), &geo, false), None);
        // A press a header button took starts no drag.
        g.header(&press(), Some(from), &geo, true);
        assert_eq!(
            g.header(&release(), Some(Point::new(800.0, 300.0)), &geo, false),
            None
        );
        // A press in a pane body starts none either.
        g.header(&press(), Some(Point::new(100.0, 200.0)), &geo, false);
        assert_eq!(
            g.header(&release(), Some(Point::new(800.0, 300.0)), &geo, false),
            None
        );
    }
}

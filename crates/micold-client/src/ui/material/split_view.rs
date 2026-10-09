//! `SplitView` — lays a [`PaneLayout`]'s panes out side by side and stacked, and draws the dividers
//! between them (feature 484, FR-001).
//!
//! Layout and divider drawing only: which terminal a pane shows, which pane has focus and what a
//! pane draws are the caller's. Children are given in tree order (`PaneLayout::panes`), one per
//! leaf. Dragging a divider is a later milestone; the hit area is computed here so it is already
//! one usable size.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size};
use micold_core::pane_layout::{Axis, Divider, PaneLayout, Placement};
use micold_core::tokens::Roles;

use super::style;

/// The visible rule between two panes.
pub const RULE: f32 = 1.0;

/// The divider's hit area across its axis: wide enough to grab with a pointer (the sidebar handle's
/// width), centred on the rule.
#[allow(dead_code)] // The divider drag (M3) reads it; until then the tests pin its size.
pub const HIT: f32 = super::resize_handle::WIDTH;

/// A divider's grab zone: the divider's line widened to [`HIT`] across its axis.
#[allow(dead_code)] // The divider drag (M3) reads it; until then the tests pin its size.
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

/// The panes of a layout, tiled; builder form (Principle VIII):
/// `SplitView::new(&layout, min, roles, children).into()`.
pub struct SplitView<'a, M> {
    layout: PaneLayout,
    min: (f32, f32),
    roles: Roles,
    children: Vec<Element<'a, M>>,
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
            layout: layout.clone(),
            min,
            roles,
            children,
        }
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for SplitView<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((c, s), l)| c.as_widget().mouse_interaction(s, l, cursor, viewport, renderer))
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
        for d in placement(&self.layout, size, self.min).dividers {
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
        assert!(hit.width >= 6.0, "a grab zone narrower than the sidebar handle: {hit:?}");
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
        assert_ne!(light, dark, "a divider drawn the same in both themes is invisible in one");
    }
}

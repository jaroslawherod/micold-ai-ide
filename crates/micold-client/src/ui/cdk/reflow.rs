//! `Reflow` — a leading block and a trailing cluster that share one line while both fit, and stack
//! when they do not.
//!
//! A list row with a name and a set of actions has exactly one flexible child, the name, so a plain
//! `row![]` answers a narrow width by giving the name away: it wraps into a second line the row
//! clips, then shrinks to nothing, and after that the actions themselves run out of room and lose
//! their labels (003 BUG-002). Nothing in the rendering stack moves a cluster onto a line of its own
//! at a width it chooses — `Row::wrap` breaks between *children*, and a `Length::Fill` child takes
//! the whole line before any break is considered.
//!
//! So this measures. It lays the trailing cluster out at its natural width, and if what is left
//! beside it is still at least the leading block's minimum, both sit on one line with the cluster
//! at the trailing edge. Otherwise the leading block takes the whole width and the cluster moves to
//! the line below, where it is laid out again at that width — so a cluster that can wrap (a
//! `Row::wrap`) still has somewhere to go at a width too narrow even for a line of its own.
//!
//! It fills the width it is offered, except in a parent sized by its content (a `Shrink`
//! container), where it is as wide as its two children need and no wider — which is how the
//! snackbar keeps its action whole beside a message long enough to wrap (018 BUG-015).
//!
//! Layout only. It draws nothing of its own, which is why it lives in the behaviour layer.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Pixels, Point, Rectangle, Size, Vector};

/// A leading block and a trailing cluster that stack below a width. Builder form (Principle VIII):
/// `Reflow::new(lead, trail).spacing(8).lead_min(240).into()`.
pub struct Reflow<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    /// `[lead, trail]`, in the order their tree entries and layout nodes appear.
    children: [Element<'a, M, Theme, Renderer>; 2],
    spacing: f32,
    lead_min: f32,
}

impl<'a, M, Theme, Renderer> Reflow<'a, M, Theme, Renderer> {
    /// `lead` fills the line; `trail` sits at its trailing edge, or below it when the line is too
    /// narrow for both. The trailing cluster should be content-sized — it is measured at its natural
    /// width, and a cluster that fills whatever it is offered has no natural width to measure.
    pub fn new(
        lead: impl Into<Element<'a, M, Theme, Renderer>>,
        trail: impl Into<Element<'a, M, Theme, Renderer>>,
    ) -> Self {
        Self {
            children: [lead.into(), trail.into()],
            spacing: 0.0,
            lead_min: 0.0,
        }
    }

    /// The gap between the two — across when they share a line, down when they are stacked.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// The narrowest the leading block may be while it shares its line. Below this the cluster
    /// moves to a line of its own rather than take the width away.
    pub fn lead_min(mut self, width: impl Into<Pixels>) -> Self {
        self.lead_min = width.into().0;
        self
    }
}

/// Whether a line `width` wide holds a lead of at least `lead_min` beside a trail `trail` wide.
fn shares_a_line(width: f32, trail: f32, spacing: f32, lead_min: f32) -> bool {
    width - trail - spacing >= lead_min
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Reflow<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Shrink);
        let width = limits.max().width;
        // A parent sized by its content (a `Shrink` container) lays this out compressed, and wants
        // back the width of what it holds rather than every pixel it offered — otherwise a snackbar
        // holding one word would be as wide as its cap (018 BUG-015). The lead is then measured
        // compressed too, so a filling lead reports its content's width; the resolve below turns
        // that into this widget's own width. Uncompressed, both are exactly what they were.
        let compressed = limits.compression().width;
        let bounded = |max: Size| {
            let bounded = layout::Limits::new(Size::ZERO, max);
            if compressed {
                bounded.width(Length::Shrink)
            } else {
                bounded
            }
        };
        let [lead, trail] = &mut self.children;
        let (lead_tree, trail_tree) = tree.children.split_at_mut(1);
        let (lead_tree, trail_tree) = (&mut lead_tree[0], &mut trail_tree[0]);

        // The cluster's one-line width: laid out with nowhere to wrap to.
        let natural = trail.as_widget_mut().layout(
            trail_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, limits.max().height)),
        );

        if shares_a_line(width, natural.size().width, self.spacing, self.lead_min) {
            let lead_width = width - natural.size().width - self.spacing;
            let lead_node = lead.as_widget_mut().layout(
                lead_tree,
                renderer,
                &bounded(Size::new(lead_width, limits.max().height)),
            );
            let (lead_size, trail_size) = (lead_node.size(), natural.size());
            let height = lead_size.height.max(trail_size.height);
            let own = limits.resolve(
                Length::Fill,
                Length::Shrink,
                Size::new(lead_size.width + self.spacing + trail_size.width, height),
            );
            let lead_node = lead_node.move_to(Point::new(0.0, (height - lead_size.height) / 2.0));
            let trail_node = natural.move_to(Point::new(
                own.width - trail_size.width,
                (height - trail_size.height) / 2.0,
            ));
            return layout::Node::with_children(own, vec![lead_node, trail_node]);
        }

        // Stacked: each gets the whole width, the cluster starting on the line below.
        let line = bounded(Size::new(width, f32::INFINITY));
        let lead_node = lead.as_widget_mut().layout(lead_tree, renderer, &line);
        let top = lead_node.size().height + self.spacing;
        let trail_node = trail
            .as_widget_mut()
            .layout(trail_tree, renderer, &line)
            .move_to(Point::new(0.0, top));
        let height = top + trail_node.size().height;
        let content = lead_node.size().width.max(trail_node.size().width);
        layout::Node::with_children(
            limits.resolve(Length::Fill, Length::Shrink, Size::new(content, height)),
            vec![lead_node, trail_node],
        )
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
        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, child_tree), child_layout)| {
                child.as_widget().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
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
        for ((child, child_tree), child_layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                child_tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
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
            for ((child, child_tree), child_layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(child_tree, child_layout, renderer, operation);
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M, Theme, Renderer> From<Reflow<'a, M, Theme, Renderer>>
    for Element<'a, M, Theme, Renderer>
where
    M: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(reflow: Reflow<'a, M, Theme, Renderer>) -> Self {
        Element::new(reflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::Space;

    const SPACING: f32 = 8.0;
    const LEAD_MIN: f32 = 200.0;
    const TRAIL: f32 = 100.0;

    /// Lays a 20dp-tall filling lead beside a fixed `TRAIL`-wide, 40dp-tall cluster out at `width`,
    /// against the null renderer, and returns the widget's own box and its two children's.
    fn laid_out(width: f32) -> (Rectangle, Rectangle, Rectangle) {
        laid_out_in(
            Space::new().width(Length::Fill).height(20.0).into(),
            layout::Limits::new(Size::ZERO, Size::new(width, 1000.0)),
        )
    }

    /// Lays a filling lead that holds `content` wide of content out beside the `TRAIL`-wide
    /// cluster, in a parent sized by what it holds (`Length::Shrink`) at most `width` wide.
    fn laid_out_shrunk(content: f32, width: f32) -> (Rectangle, Rectangle, Rectangle) {
        laid_out_in(
            iced::widget::container(Space::new().width(content).height(20.0))
                .width(Length::Fill)
                .into(),
            layout::Limits::new(Size::ZERO, Size::new(width, 1000.0)).width(Length::Shrink),
        )
    }

    fn laid_out_in(
        lead: Element<'_, (), iced::Theme, ()>,
        limits: layout::Limits,
    ) -> (Rectangle, Rectangle, Rectangle) {
        let mut reflow: Reflow<'_, (), iced::Theme, ()> =
            Reflow::new(lead, Space::new().width(TRAIL).height(40.0))
                .spacing(SPACING)
                .lead_min(LEAD_MIN);
        let mut tree = Tree::new(&reflow as &dyn Widget<(), iced::Theme, ()>);
        let node = Widget::<(), iced::Theme, ()>::layout(&mut reflow, &mut tree, &(), &limits);
        let layout = Layout::new(&node);
        let mut children = layout.children();
        let lead = children.next().expect("a lead node").bounds();
        let trail = children.next().expect("a trail node").bounds();
        (layout.bounds(), lead, trail)
    }

    #[test]
    fn both_share_a_line_while_the_lead_keeps_its_minimum() {
        let (own, lead, trail) = laid_out(400.0);
        assert_eq!(own.size(), Size::new(400.0, 40.0));
        // The lead takes what the cluster leaves, centred on the taller of the two.
        assert_eq!(
            lead,
            Rectangle::new(Point::new(0.0, 10.0), Size::new(292.0, 20.0))
        );
        // The cluster sits at the trailing edge at its natural width.
        assert_eq!(
            trail,
            Rectangle::new(Point::new(300.0, 0.0), Size::new(TRAIL, 40.0))
        );
    }

    #[test]
    fn the_cluster_moves_below_when_the_lead_would_fall_under_its_minimum() {
        let (own, lead, trail) = laid_out(250.0);
        assert_eq!(lead, Rectangle::new(Point::ORIGIN, Size::new(250.0, 20.0)));
        assert_eq!(
            trail,
            Rectangle::new(Point::new(0.0, 28.0), Size::new(TRAIL, 40.0))
        );
        assert_eq!(own.size(), Size::new(250.0, 68.0));
    }

    #[test]
    fn the_break_is_exactly_where_the_lead_minimum_is_reached() {
        let breakpoint = LEAD_MIN + SPACING + TRAIL;
        let (_, _, shared) = laid_out(breakpoint);
        assert!(
            shared.y == 0.0 && shared.x > 0.0,
            "at the breakpoint both share a line"
        );
        let (_, _, stacked) = laid_out(breakpoint - 1.0);
        assert!(
            stacked.y > 0.0 && stacked.x == 0.0,
            "1dp under it the cluster stacks"
        );
    }

    #[test]
    fn in_a_content_sized_parent_it_is_as_wide_as_what_it_holds() {
        // A parent that sizes itself by its content (a `Shrink` container, 018 BUG-015) must get
        // back the width of the content, not every pixel it could have had.
        let (own, lead, trail) = laid_out_shrunk(120.0, 400.0);
        assert_eq!(own.size(), Size::new(120.0 + SPACING + TRAIL, 40.0));
        assert_eq!(
            lead,
            Rectangle::new(Point::new(0.0, 10.0), Size::new(120.0, 20.0))
        );
        assert_eq!(
            trail,
            Rectangle::new(Point::new(120.0 + SPACING, 0.0), Size::new(TRAIL, 40.0))
        );
    }

    #[test]
    fn in_a_content_sized_parent_a_lead_wider_than_the_line_takes_only_what_the_cluster_leaves() {
        // The cluster is measured first, so the lead cannot take its width however much it holds.
        let (own, lead, trail) = laid_out_shrunk(1000.0, 400.0);
        assert_eq!(own.size(), Size::new(400.0, 40.0));
        assert_eq!(
            lead,
            Rectangle::new(Point::new(0.0, 10.0), Size::new(292.0, 20.0))
        );
        assert_eq!(
            trail,
            Rectangle::new(Point::new(300.0, 0.0), Size::new(TRAIL, 40.0))
        );
    }

    #[test]
    fn in_a_content_sized_parent_a_stacked_pair_is_as_wide_as_the_wider_line() {
        let (own, lead, trail) = laid_out_shrunk(120.0, 250.0);
        assert_eq!(lead, Rectangle::new(Point::ORIGIN, Size::new(120.0, 20.0)));
        assert_eq!(
            trail,
            Rectangle::new(Point::new(0.0, 28.0), Size::new(TRAIL, 40.0))
        );
        assert_eq!(own.size(), Size::new(120.0, 68.0));
    }

    #[test]
    fn it_has_one_tree_entry_per_child() {
        let reflow: Reflow<'_, (), iced::Theme, ()> =
            Reflow::new(Space::new(), Space::new().width(TRAIL));
        assert_eq!(Widget::<(), iced::Theme, ()>::children(&reflow).len(), 2);
    }
}

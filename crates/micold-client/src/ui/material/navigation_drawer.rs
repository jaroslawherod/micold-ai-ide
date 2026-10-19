//! `NavigationDrawer` — a side panel that slides away and leaves a rail behind.
//!
//! This is the transition the other wrappers could not absorb. A `Fade` or an `Expand` animates one
//! child toward nothing, and nothing is a perfectly good thing to end up as. A drawer does not:
//! at zero width the panel is *replaced* by a collapsed rail, so whatever owns the slide must own
//! both elements and decide, every frame, which of them is the one on screen.
//!
//! That decision is what kept the sidebar's track in the application's central animator long after
//! the other five had moved out — the binary was reading a progress value purely to answer "rail or
//! panel?". It owns both children now, so it can answer that itself, and the caller is left saying
//! only whether the drawer is open.
//!
//! The resize handle rides along as a third child rather than sitting beside the drawer in the
//! caller's layout, because it shares the same fate: it belongs to the open panel and must vanish
//! with it. It is deliberately *outside* the slide, though — the panel is clipped as it retracts,
//! while the edge beside it stays whole.

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::ui::cdk::motion::Progress;
use micold_core::tokens::motion::{duration, EMPHASIZED};

/// Below this the panel is gone and the rail has taken over. Not zero: a track converges *toward*
/// its target, and a drawer that only swaps at exactly zero would leave a sliver of panel on screen
/// for the last fraction of a pixel.
const CLOSED: f32 = 0.001;

/// How long the panel takes to slide its full range.
///
/// Owned here, not passed in: how long a thing takes is part of how it looks, and that is the
/// design system's business rather than the application's — the same reasoning that put the
/// dialog's timing in `modal` and the menu's in `menu`.
pub(super) const SLIDE: Duration = Duration::from_millis(duration::MEDIUM_4);

/// A panel that slides in and out beside the content, leaving `rail` behind when closed. Builder
/// form (Principle VIII): `NavigationDrawer::new(panel, rail).open(b).handle(h).into()`.
pub struct NavigationDrawer<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    panel: Element<'a, M, Theme, Renderer>,
    rail: Element<'a, M, Theme, Renderer>,
    handle: Option<Element<'a, M, Theme, Renderer>>,
    open: bool,
    retracted: bool,
}

impl<'a, M, Theme, Renderer> NavigationDrawer<'a, M, Theme, Renderer> {
    /// A drawer showing `panel` when open and `rail` when closed.
    pub fn new(
        panel: impl Into<Element<'a, M, Theme, Renderer>>,
        rail: impl Into<Element<'a, M, Theme, Renderer>>,
    ) -> Self {
        Self {
            panel: panel.into(),
            rail: rail.into(),
            handle: None,
            open: true,
            retracted: false,
        }
    }

    /// Whether the drawer is open. A destination, not a position — how far along it currently is
    /// is the drawer's own business.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Hide the whole drawer, rail and handle included, without touching whether it is open: the
    /// drawer slides off at its left edge and gives its width to the content beside it, and comes
    /// back as it was given when no longer retracted (feature 970, plan D2). A drawer built
    /// retracted starts at zero width, not sliding there.
    pub fn retracted(mut self, retracted: bool) -> Self {
        self.retracted = retracted;
        self
    }

    /// A resize affordance shown at the open panel's edge, and hidden with it. Omitted, the drawer
    /// is a fixed width.
    pub fn handle(mut self, handle: impl Into<Element<'a, M, Theme, Renderer>>) -> Self {
        self.handle = Some(handle.into());
        self
    }

    /// The children, in the order their layout nodes and tree entries appear throughout.
    fn parts(&self) -> Vec<&Element<'a, M, Theme, Renderer>> {
        let mut out = vec![&self.panel, &self.rail];
        out.extend(self.handle.iter());
        out
    }

    /// Where the retraction is headed: 1 shown, 0 retracted.
    fn presence_target(&self) -> f32 {
        if self.retracted {
            0.0
        } else {
            1.0
        }
    }

    /// Which child is on screen at `progress` — the drawer's whole reason to exist.
    fn showing_rail(&self, progress: f32) -> bool {
        !self.open && progress <= CLOSED
    }
}

/// The slide, owned here rather than by the application.
struct Track {
    progress: Progress,
    rail_showing: bool,
    /// How much of the drawer is on screen: 1 shown, 0 retracted (feature 970, plan D2). Separate
    /// from `progress`, which is whether it is open, so retraction gives back what it took.
    presence: Progress,
    /// Retracted and settled: every child parked, so no method reaches one. Set by `layout`.
    gone: bool,
}

/// Built at `target` and eased on `emphasized`, the curve 018 §6.3's *sidebar slide* row names.
fn emphasized_track(target: f32) -> Progress {
    Progress::new(target).easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)
}

/// A node parked where it cannot be seen. The inactive child still needs a layout entry — the tree,
/// the node list and the child list must stay index-aligned — but it must not occupy space.
pub(super) fn parked(node: layout::Node) -> layout::Node {
    node.translate(Vector::new(-f32::MAX / 4.0, 0.0))
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for NavigationDrawer<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Track>()
    }

    fn state(&self) -> tree::State {
        // Built at its destination, not animating toward it: a drawer that starts open must not
        // slide open on the first frame of the session. On `emphasized`, the curve 018 §6.3's
        // *sidebar slide* row names.
        tree::State::new(Track {
            progress: emphasized_track(if self.open { 1.0 } else { 0.0 }),
            rail_showing: !self.open,
            presence: emphasized_track(self.presence_target()),
            gone: self.retracted,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.parts()
            .into_iter()
            .map(|c| Tree::new(c.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children_custom(
            &self.parts(),
            |child_tree, child| child_tree.diff(child.as_widget()),
            |child| Tree::new(child.as_widget()),
        );
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self.layout_shown(tree, renderer, limits);
        let track = tree.state.downcast_mut::<Track>();
        let presence = track.presence.value();
        track.gone = self.retracted && presence <= CLOSED;
        if track.gone {
            let height = node.size().height;
            let parked_all = node.children().iter().cloned().map(parked).collect();
            return layout::Node::with_children(Size::new(0.0, height), parked_all);
        }
        if presence >= 1.0 {
            return node;
        }
        // Retracting: the drawer narrows to its shown fraction and slides left by the rest, so it
        // leaves at its left edge and the content beside it follows its right edge.
        let full = node.size();
        let width = (full.width * presence).clamp(0.0, full.width);
        let shift = Vector::new(width - full.width, 0.0);
        let moved = node
            .children()
            .iter()
            .map(|child| child.clone().translate(shift))
            .collect();
        layout::Node::with_children(Size::new(width, full.height), moved)
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
        // Advance first, so the children are updated against the arrangement they are about to be
        // drawn in rather than the previous frame's.
        let target = if self.open { 1.0 } else { 0.0 };
        let track = tree.state.downcast_mut::<Track>();
        // `on_layout_frame`, not `on_frame`: this widget's `layout` reads the progress to size the
        // revealed panel, and iced re-lays-out only when asked (BUG-001).
        track.progress.on_layout_frame(event, target, SLIDE, shell);
        // The retraction, on the same slide; reversing midway retargets from where it is.
        track
            .presence
            .on_layout_frame(event, self.presence_target(), SLIDE, shell);
        // Which child `layout` put on screen, not what the advanced progress would pick: the event
        // is delivered through that layout, until the next one.
        let rail_showing = track.rail_showing;
        if track.gone {
            // A release still reaches every child, with no cursor: a drag or press begun before
            // the drawer left must end, or the resize handle comes back still dragging.
            if let Event::Mouse(mouse::Event::ButtonReleased(_)) = event {
                let mut children = self.parts_mut();
                for ((child, child_tree), child_layout) in children
                    .iter_mut()
                    .zip(&mut tree.children)
                    .zip(layout.children())
                {
                    child.as_widget_mut().update(
                        child_tree,
                        event,
                        child_layout,
                        mouse::Cursor::Unavailable,
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    );
                }
            }
            return;
        }

        // Only the child on screen hears about the event. The parked one is not merely invisible —
        // it is somewhere the pointer can never be, so forwarding to it would be feeding it
        // coordinates that mean nothing.
        let mut children = self.parts_mut();
        for (index, (child, child_tree)) in children.iter_mut().zip(&mut tree.children).enumerate()
        {
            let on_screen = if index == 1 {
                rail_showing
            } else {
                !rail_showing
            };
            let Some(child_layout) = layout.children().nth(index) else {
                continue;
            };
            if !on_screen {
                continue;
            }
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
        let track = tree.state.downcast_ref::<Track>();
        if track.gone {
            return mouse::Interaction::default();
        }
        let rail_showing = track.rail_showing;
        self.parts()
            .into_iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(index, _)| {
                if *index == 1 {
                    rail_showing
                } else {
                    !rail_showing
                }
            })
            .map(|(_, ((child, child_tree), child_layout))| {
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
        let track = tree.state.downcast_ref::<Track>();
        if track.gone {
            return;
        }
        if track.presence.value() < 1.0 {
            // Mid-retraction the children hang past the drawer's left edge: clip them to it.
            renderer.with_layer(layout.bounds(), |renderer| {
                self.draw_children(tree, renderer, theme, style, layout, cursor, viewport);
            });
        } else {
            self.draw_children(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        // Retracted, nothing in the drawer can take the keyboard or be found (FR-002) — from the
        // moment the retraction starts, not when it settles, so Tab never lands on a leaving row.
        if self.retracted {
            return;
        }
        let mut children = self.parts_mut();
        for ((child, child_tree), child_layout) in children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let track = tree.state.downcast_ref::<Track>();
        if track.gone {
            return None;
        }
        let rail_showing = track.rail_showing;
        let index = usize::from(rail_showing);
        let child_layout = layout.children().nth(index)?;
        let child = if rail_showing {
            &mut self.rail
        } else {
            &mut self.panel
        };
        child.as_widget_mut().overlay(
            &mut tree.children[index],
            child_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<M, Theme, Renderer> NavigationDrawer<'_, M, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    /// Draws the child on screen; the panel clipped to the width it occupies.
    #[allow(clippy::too_many_arguments)]
    fn draw_children(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let rail_showing = tree.state.downcast_ref::<Track>().rail_showing;

        for (index, ((child, child_tree), child_layout)) in self
            .parts()
            .into_iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            let on_screen = if index == 1 {
                rail_showing
            } else {
                !rail_showing
            };
            if !on_screen {
                continue;
            }
            if index == 0 {
                // The retracting panel is clipped to the width it currently occupies, so its
                // content is cut off at the edge instead of spilling past it.
                renderer.with_layer(child_layout.bounds(), |renderer| {
                    child.as_widget().draw(
                        child_tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        viewport,
                    );
                });
            } else {
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
    }

    /// The layout as open or closed, before retraction narrows it.
    fn layout_shown(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let progress = tree.state.downcast_ref::<Track>().progress.value();

        let (panel_tree, rest) = tree.children.split_at_mut(1);
        let (rail_tree, handle_tree) = rest.split_at_mut(1);

        let panel = self
            .panel
            .as_widget_mut()
            .layout(&mut panel_tree[0], renderer, limits);
        let rail = self
            .rail
            .as_widget_mut()
            .layout(&mut rail_tree[0], renderer, limits);
        let handle = self.handle.as_mut().map(|handle| {
            handle
                .as_widget_mut()
                .layout(&mut handle_tree[0], renderer, limits)
        });

        // A closing panel that is no wider than the rail has nothing left to slide, so the rail
        // takes over there rather than at `CLOSED`: `emphasized` spends a long, slow tail in
        // between, and the panel would sit frozen on screen for all of it. The decision is kept
        // for the other methods, which are handed this layout and must agree with it.
        let full = panel.size();
        let handle_width = handle.as_ref().map_or(0.0, |h| h.size().width);
        let rail_floor = rail.size().width - handle_width;
        let rail_showing =
            self.showing_rail(progress) || (!self.open && full.width * progress <= rail_floor);
        tree.state.downcast_mut::<Track>().rail_showing = rail_showing;

        if rail_showing {
            let size = rail.size();
            let mut nodes = vec![parked(panel), rail];
            nodes.extend(handle.map(parked));
            return layout::Node::with_children(size, nodes);
        }

        // The panel reveals from its right edge: the space it occupies shrinks while the content
        // slides left behind it, so the part that disappears is the far side, not the near one.
        // Never narrower than the rail, though, which an opening panel starts out as: the content
        // beside the drawer would otherwise dip past the rail's edge before it moves out.
        let width = (full.width * progress)
            .max(rail_floor)
            .min(full.width)
            .max(0.0);
        let panel = panel.translate(Vector::new(-(full.width - width), 0.0));

        let height = full
            .height
            .max(handle.as_ref().map_or(0.0, |h| h.size().height));
        let mut nodes = vec![panel, parked(rail)];
        nodes.extend(handle.map(|h| h.translate(Vector::new(width, 0.0))));

        layout::Node::with_children(Size::new(width + handle_width, height), nodes)
    }
}

impl<'a, M, Theme, Renderer> NavigationDrawer<'a, M, Theme, Renderer> {
    /// The children as mutable references, in the same order as [`Self::parts`].
    fn parts_mut(&mut self) -> Vec<&mut Element<'a, M, Theme, Renderer>> {
        let mut out = vec![&mut self.panel, &mut self.rail];
        out.extend(self.handle.iter_mut());
        out
    }
}

impl<'a, M, Theme, Renderer> From<NavigationDrawer<'a, M, Theme, Renderer>>
    for Element<'a, M, Theme, Renderer>
where
    M: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(drawer: NavigationDrawer<'a, M, Theme, Renderer>) -> Self {
        Element::new(drawer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::Space;

    fn drawer<'a>() -> NavigationDrawer<'a, ()> {
        NavigationDrawer::new(Space::new(), Space::new())
    }

    /// Open, the panel is on screen regardless of where the slide has got to — including the very
    /// first frame of an entrance, when progress is still zero.
    #[test]
    fn an_open_drawer_always_shows_its_panel() {
        assert!(!drawer().open(true).showing_rail(0.0));
        assert!(!drawer().open(true).showing_rail(0.5));
        assert!(!drawer().open(true).showing_rail(1.0));
    }

    /// The case the whole component exists for. Closing sets `open` to false immediately, but the
    /// panel has to stay on screen for the length of the slide — swapping to the rail the moment
    /// the flag flips would make the sidebar disappear instantly instead of retracting.
    #[test]
    fn a_closing_drawer_keeps_its_panel_until_the_slide_is_done() {
        assert!(!drawer().open(false).showing_rail(1.0));
        assert!(!drawer().open(false).showing_rail(0.5));
        assert!(!drawer().open(false).showing_rail(0.01));
    }

    /// And once it has arrived, the rail takes over.
    #[test]
    fn a_closed_drawer_shows_the_rail() {
        assert!(drawer().open(false).showing_rail(0.0));
    }

    /// A track converges *toward* its target rather than landing exactly on it, so the swap has to
    /// tolerate a residue. Testing at the threshold pins that this is deliberate, not a stray
    /// epsilon someone may later "tidy" to `== 0.0` — which would strand a sliver of panel on
    /// screen for good.
    #[test]
    fn the_swap_tolerates_a_residual_fraction() {
        assert!(drawer().open(false).showing_rail(CLOSED));
        assert!(!drawer().open(false).showing_rail(CLOSED * 2.0));
    }

    /// Layout, the widget tree and the node list are addressed by a shared index, so the three
    /// have to agree on how many children there are and in what order. Getting this wrong is a
    /// panic or a child drawn in another's place, so it is pinned rather than assumed.
    #[test]
    fn the_child_list_matches_with_and_without_a_handle() {
        let plain = drawer();
        assert_eq!(plain.parts().len(), 2);
        assert_eq!(Widget::<(), _, _>::children(&plain).len(), 2);

        let with_handle = drawer().handle(Space::new());
        assert_eq!(with_handle.parts().len(), 3);
        assert_eq!(Widget::<(), _, _>::children(&with_handle).len(), 3);
    }

    /// A drawer is built already at its destination, never travelling toward it: a sidebar that
    /// was open when the window closed must be open on the first frame of the next session, not
    /// slide itself in as though the user had just asked for it.
    #[test]
    fn a_drawer_starts_at_its_destination_rather_than_animating_to_it() {
        let open = drawer().open(true);
        let state = Widget::<(), iced::Theme, iced::Renderer>::state(&open);
        assert_eq!(state.downcast_ref::<Track>().progress.value(), 1.0);

        let closed = drawer().open(false);
        let state = Widget::<(), iced::Theme, iced::Renderer>::state(&closed);
        assert_eq!(state.downcast_ref::<Track>().progress.value(), 0.0);
    }

    /// `cubic-bezier(0.2, 0, 0, 1)` — the *emphasized* curve — at x = 0.25, solved by bisection.
    /// Stated rather than computed because `cdk::motion::ease` is private.
    const EMPHASIZED_AT_A_QUARTER: f32 = 0.607;

    /// The sidebar slide is `medium_4` on `emphasized` (018 §6.3, FR-003), not linear. Instants 0,
    /// +64 ms and +84 ms are a quarter of the way in linear time: a track's first frame steps one
    /// `FRAME` (16 ms), and later gaps are measured, capped at 64 ms.
    #[test]
    fn the_sidebar_slides_on_the_emphasized_curve() {
        let closed = drawer().open(false);
        let mut state = Widget::<(), iced::Theme, iced::Renderer>::state(&closed);
        let track = state.downcast_mut::<Track>();
        let mut linear = Progress::new(0.0);

        let start = std::time::Instant::now();
        let mut messages = Vec::<()>::new();
        for at in [0, 64, 84] {
            let frame = Event::Window(iced::window::Event::RedrawRequested(
                start + Duration::from_millis(at),
            ));
            track
                .progress
                .on_frame(&frame, 1.0, SLIDE, &mut Shell::new(&mut messages));
            linear.on_frame(&frame, 1.0, SLIDE, &mut Shell::new(&mut messages));
        }

        assert!(
            (linear.value() - 0.25).abs() <= 0.001,
            "the instants are a quarter of the slide in linear time, got {}",
            linear.value()
        );
        let sidebar = track.progress.value();
        assert!(
            (sidebar - 0.25).abs() > 0.05,
            "the sidebar moved linearly: {sidebar} at a quarter of the slide"
        );
        assert!(
            (sidebar - EMPHASIZED_AT_A_QUARTER).abs() <= 0.01,
            "expected the emphasized curve's {EMPHASIZED_AT_A_QUARTER}, got {sidebar}"
        );
    }

    /// The sidebar's collapsed strip: `sidebar.rs`'s `STRIP_WIDTH`, its surface plus a 1 px border.
    const RAIL_WIDTH: f32 = 32.0;

    /// An opening panel starts out narrower than the rail it replaces, and `emphasized` lingers
    /// there. Laid out that narrow, the content beside the drawer would dip left of the rail's edge
    /// before moving out; so the panel is held at the rail's width until it outgrows it. Above that
    /// the slide is untouched, and a panel narrower than the floor is never stretched past itself.
    #[test]
    fn a_sliding_drawer_is_never_narrower_than_its_rail() {
        let handle = crate::ui::material::resize_handle::WIDTH;
        let renderer = crate::ui::material::test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(1000.0, 100.0));
        let floor = RAIL_WIDTH - handle;

        // (open, progress, panel width, the panel's revealed width)
        for (open, progress, panel_width, revealed) in [
            (true, 0.0, 300.0, floor),
            (true, 0.05, 300.0, floor),
            (false, 0.125, 300.0, 37.5),
            (false, 0.5, 300.0, 150.0),
            (true, 1.0, 10.0, 10.0),
        ] {
            let expected = revealed + handle;
            let mut drawer: Element<'_, ()> = NavigationDrawer::new(
                Space::new().width(panel_width).height(100.0),
                Space::new().width(RAIL_WIDTH).height(100.0),
            )
            .open(open)
            .handle(Space::new().width(handle))
            .into();
            let mut tree = Tree::new(drawer.as_widget());
            tree.state.downcast_mut::<Track>().progress = Progress::new(progress);
            let node = drawer.as_widget_mut().layout(&mut tree, &renderer, &limits);

            assert_eq!(
                node.size().width,
                expected,
                "open {open} at progress {progress}: the drawer's width"
            );
            let layout = Layout::new(&node);
            let panel = layout.children().next().expect("the panel's node").bounds();
            let edge = layout
                .children()
                .nth(2)
                .expect("the handle's node")
                .bounds();
            assert_eq!(
                (edge.x, panel.x + panel.width),
                (revealed, revealed),
                "open {open} at progress {progress}: the handle and the panel's right edge"
            );
        }
    }

    /// Once a closing panel is no wider than its rail there is nothing left to slide, so the rail
    /// takes over there rather than at `CLOSED`: on `emphasized` the tail between the two lasts
    /// ~150 ms, a frozen sliver of panel. An opening drawer never swaps back early, and a drawer
    /// with nothing to floor at (the settings view's empty rail) still swaps at `CLOSED`.
    #[test]
    fn a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider() {
        const PANEL: f32 = 300.0;
        let handle = crate::ui::material::resize_handle::WIDTH;
        let renderer = crate::ui::material::test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(1000.0, 100.0));

        // (open, progress, rail width, with a handle, rail on screen)
        for (open, progress, rail, with_handle, expected) in [
            (false, 0.05, RAIL_WIDTH, true, true),
            (false, 0.085, RAIL_WIDTH, true, true),
            (false, 0.1, RAIL_WIDTH, true, false),
            (true, 0.05, RAIL_WIDTH, true, false),
            (false, 0.01, 0.0, false, false),
            (false, CLOSED, 0.0, false, true),
        ] {
            let mut drawer = NavigationDrawer::new(
                Space::new().width(PANEL).height(100.0),
                Space::new().width(rail).height(100.0),
            )
            .open(open);
            if with_handle {
                drawer = drawer.handle(Space::new().width(handle));
            }
            let mut drawer: Element<'_, ()> = drawer.into();
            let mut tree = Tree::new(drawer.as_widget());
            tree.state.downcast_mut::<Track>().progress = Progress::new(progress);
            let node = drawer.as_widget_mut().layout(&mut tree, &renderer, &limits);

            let rail_node = Layout::new(&node)
                .children()
                .nth(1)
                .expect("the rail's node")
                .bounds();
            assert_eq!(
                rail_node.x == 0.0,
                expected,
                "open {open} at progress {progress} beside a {rail} rail: the rail on screen"
            );
            assert_eq!(
                tree.state.downcast_ref::<Track>().rail_showing,
                expected,
                "open {open} at progress {progress} beside a {rail} rail: what draw and update read"
            );
        }
    }

    // Feature 970 (plan D2; contracts/ui.md *Geometry gates — Drawer*): retraction hides the whole
    // drawer at its left edge without touching whether it is open.

    /// The panel's width in the retraction gates.
    const PANEL_WIDTH: f32 = 300.0;

    /// How long a retraction may take to settle (FR-005).
    const SETTLED_BY: u64 = 500;

    /// `cubic-bezier(0.2, 0, 0, 1)` at linear time `t`, solved by bisection: the reference curve
    /// the retraction is checked against, independent of `cdk::motion`'s solver.
    fn emphasized(t: f32) -> f32 {
        fn bezier(p1: f32, p2: f32, u: f32) -> f32 {
            let v = 1.0 - u;
            3.0 * v * v * u * p1 + 3.0 * v * u * u * p2 + u * u * u
        }
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if bezier(EMPHASIZED.x1, EMPHASIZED.x2, mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        bezier(EMPHASIZED.y1, EMPHASIZED.y2, (lo + hi) / 2.0)
    }

    /// The sidebar as the app builds it: a panel, the collapsed strip and a resize handle.
    fn sidebar<'a>(open: bool, retracted: bool) -> Element<'a, ()> {
        NavigationDrawer::new(
            Space::new().width(PANEL_WIDTH).height(100.0),
            Space::new().width(RAIL_WIDTH).height(100.0),
        )
        .open(open)
        .handle(Space::new().width(crate::ui::material::resize_handle::WIDTH))
        .retracted(retracted)
        .into()
    }

    /// A drawer in the widget tree, rebuilt each frame as a view would build it.
    struct Harness {
        tree: Tree,
        node: layout::Node,
        start: std::time::Instant,
    }

    impl Harness {
        fn new(open: bool, retracted: bool) -> Self {
            let drawer = sidebar(open, retracted);
            let mut harness = Self {
                tree: Tree::new(drawer.as_widget()),
                node: layout::Node::new(Size::ZERO),
                start: std::time::Instant::now(),
            };
            harness.lay_out(open, retracted);
            harness
        }

        fn lay_out(&mut self, open: bool, retracted: bool) -> &layout::Node {
            let mut drawer = sidebar(open, retracted);
            self.tree.diff(drawer.as_widget());
            let renderer = crate::ui::material::test_support::renderer();
            let limits = layout::Limits::new(Size::ZERO, Size::new(1000.0, 100.0));
            self.node = drawer
                .as_widget_mut()
                .layout(&mut self.tree, &renderer, &limits);
            &self.node
        }

        /// One redraw at `at` ms through the drawer's own `update`, then the layout it asks for.
        fn frame(&mut self, open: bool, retracted: bool, at: u64) -> &layout::Node {
            let mut drawer = sidebar(open, retracted);
            self.tree.diff(drawer.as_widget());
            let renderer = crate::ui::material::test_support::renderer();
            let event = Event::Window(iced::window::Event::RedrawRequested(
                self.start + Duration::from_millis(at),
            ));
            let mut messages = Vec::<()>::new();
            let mut shell = Shell::new(&mut messages);
            drawer.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::new(iced::Point::ORIGIN, Size::new(1000.0, 100.0)),
            );
            self.lay_out(open, retracted)
        }

        fn width(&self) -> f32 {
            self.node.size().width
        }

        /// The x of child `index` (panel 0, rail 1, handle 2).
        fn child_x(&self, index: usize) -> f32 {
            Layout::new(&self.node)
                .children()
                .nth(index)
                .expect("a child node")
                .bounds()
                .x
        }

        fn every_child_parked(&self) -> bool {
            (0..3).all(|index| self.child_x(index) < -1.0e30)
        }
    }

    /// One event through a sidebar with a working resize handle, then the layout after it.
    fn send(
        tree: &mut Tree,
        node: &mut layout::Node,
        retracted: bool,
        event: &Event,
        cursor: mouse::Cursor,
    ) -> Vec<f32> {
        let sidebar = || -> Element<'_, f32> {
            NavigationDrawer::new(
                Space::new().width(PANEL_WIDTH).height(100.0),
                Space::new().width(RAIL_WIDTH).height(100.0),
            )
            .open(true)
            .handle(
                crate::ui::material::ResizeHandle::new(crate::ui::material::test_support::roles())
                    .on_resize(|x| x),
            )
            .retracted(retracted)
            .into()
        };
        let renderer = crate::ui::material::test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(1000.0, 100.0));
        let mut drawer = sidebar();
        tree.diff(drawer.as_widget());
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        drawer.as_widget_mut().update(
            tree,
            event,
            Layout::new(node),
            cursor,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::new(iced::Point::ORIGIN, Size::new(1000.0, 100.0)),
        );
        *node = drawer.as_widget_mut().layout(tree, &renderer, &limits);
        messages
    }

    /// How many focusable widgets an operation finds in a sidebar whose panel holds a text field.
    fn focusables(retracted: bool) -> usize {
        use iced::advanced::widget::operation::{focusable, Outcome};
        let mut drawer: Element<'_, ()> = NavigationDrawer::new(
            iced::widget::text_input("", "").on_input(|_| ()),
            Space::new().width(RAIL_WIDTH).height(100.0),
        )
        .open(true)
        .retracted(retracted)
        .into();
        // Built shown, so the retraction has only just started when the operation runs.
        let mut tree = Tree::new(sidebar(true, false).as_widget());
        tree.diff(drawer.as_widget());
        let renderer = crate::ui::material::test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(1000.0, 100.0));
        let node = drawer.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let mut count = focusable::count();
        drawer.as_widget_mut().operate(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &mut iced::advanced::widget::operation::black_box::<_, ()>(&mut count),
        );
        match count.finish() {
            Outcome::Some(count) => count.total,
            _ => 0,
        }
    }

    /// Tab cannot land in a drawer that has started to leave, even before it settles.
    #[test]
    fn a_retracting_drawer_takes_no_focus() {
        assert_eq!(
            focusables(false),
            1,
            "shown: the panel's field is reachable"
        );
        assert_eq!(focusables(true), 0, "retracting: nothing in it is");
    }

    /// A drag on the resize handle that outlives the retraction ends at the release, so the drawer
    /// does not come back still following the pointer.
    #[test]
    fn a_release_while_retracted_ends_a_drag_begun_before() {
        let mut tree = Tree::empty();
        let mut node = layout::Node::new(Size::ZERO);
        let none = mouse::Cursor::Unavailable;
        let tick = Event::Mouse(mouse::Event::CursorLeft);
        send(&mut tree, &mut node, false, &tick, none);
        let handle = Layout::new(&node)
            .children()
            .nth(2)
            .expect("the handle")
            .bounds();
        let on_handle = mouse::Cursor::Available(handle.center());
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        send(&mut tree, &mut node, false, &press, on_handle);
        let start = std::time::Instant::now();
        for at in (0..=SETTLED_BY).step_by(16) {
            let redraw = Event::Window(iced::window::Event::RedrawRequested(
                start + Duration::from_millis(at),
            ));
            send(&mut tree, &mut node, true, &redraw, none);
        }
        assert_eq!(node.size().width, 0.0, "the drawer is gone");
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        send(&mut tree, &mut node, true, &release, none);
        for at in (SETTLED_BY..=2 * SETTLED_BY).step_by(16) {
            let redraw = Event::Window(iced::window::Event::RedrawRequested(
                start + Duration::from_millis(at),
            ));
            send(&mut tree, &mut node, false, &redraw, none);
        }
        let position = iced::Point::new(200.0, 50.0);
        let moved = Event::Mouse(mouse::Event::CursorMoved { position });
        let resized = send(
            &mut tree,
            &mut node,
            false,
            &moved,
            mouse::Cursor::Available(position),
        );
        assert!(
            resized.is_empty(),
            "the drag ended at the release: {resized:?}"
        );
    }

    #[test]
    fn a_retracted_drawer_settles_at_zero_width_with_every_child_parked() {
        for open in [true, false] {
            let mut harness = Harness::new(open, false);
            assert!(harness.width() > 0.0, "open {open}: shown first");
            for at in (0..=SETTLED_BY).step_by(16) {
                harness.frame(open, true, at);
            }
            assert_eq!(harness.width(), 0.0, "open {open}: settled at zero width");
            assert!(
                harness.every_child_parked(),
                "open {open}: panel, rail and handle parked"
            );
        }
    }

    #[test]
    fn a_drawer_built_retracted_starts_at_zero_width() {
        for open in [true, false] {
            let harness = Harness::new(open, true);
            assert_eq!(harness.width(), 0.0, "open {open}: the first layout");
            assert!(
                harness.every_child_parked(),
                "open {open}: nothing on screen"
            );
        }
    }

    #[test]
    fn a_retracting_drawer_narrows_on_the_emphasized_curve() {
        let full = PANEL_WIDTH + crate::ui::material::resize_handle::WIDTH;
        let slide = SLIDE.as_millis() as f32;
        for retracting in [true, false] {
            let mut harness = Harness::new(true, !retracting);
            let mut last = harness.width();
            for (step, at) in (0..=SETTLED_BY).step_by(16).enumerate() {
                harness.frame(true, retracting, at);
                // A track's first frame steps one 16 ms frame; later gaps are measured.
                let t = ((step as f32 + 1.0) * 16.0 / slide).min(1.0);
                let shown = if retracting {
                    1.0 - emphasized(t)
                } else {
                    emphasized(t)
                };
                let width = harness.width();
                assert!(
                    (width - full * shown).abs() <= 0.5,
                    "retracting {retracting} at {at} ms: width {width}, expected {}",
                    full * shown
                );
                assert!(width >= 0.0, "never negative");
                if retracting {
                    assert!(width <= last, "monotone at {at} ms: {last} then {width}");
                } else {
                    assert!(width >= last, "monotone at {at} ms: {last} then {width}");
                }
                if width > 0.0 {
                    assert!(
                        (harness.child_x(0) - (width - full)).abs() <= 0.01,
                        "at {at} ms the panel is translated left by what is hidden"
                    );
                }
                last = width;
            }
            let settled = if retracting { 0.0 } else { full };
            assert_eq!(harness.width(), settled, "settled by {SETTLED_BY} ms");
        }
    }

    #[test]
    fn a_retraction_reversed_midway_continues_from_its_width() {
        let full = PANEL_WIDTH + crate::ui::material::resize_handle::WIDTH;
        let mut harness = Harness::new(true, false);
        let mut last = harness.width();
        let mut largest_step = 0.0_f32;
        let mut at = 0;
        while at <= 200 {
            harness.frame(true, true, at);
            largest_step = largest_step.max((last - harness.width()).abs());
            last = harness.width();
            at += 16;
        }
        harness.frame(true, false, at);
        let width = harness.width();
        assert!(
            (width - last).abs() <= largest_step + 0.5,
            "reversed at {at} ms: {last} then {width}, a jump"
        );
        assert!(
            width > 0.0 && width < full,
            "no jump to either end: {width}"
        );
    }

    #[test]
    fn leaving_retraction_shows_the_drawer_as_given() {
        let handle = crate::ui::material::resize_handle::WIDTH;
        for (open, given) in [(true, PANEL_WIDTH + handle), (false, RAIL_WIDTH)] {
            let mut harness = Harness::new(open, true);
            for at in (0..=SETTLED_BY).step_by(16) {
                harness.frame(open, false, at);
            }
            assert_eq!(harness.width(), given, "open {open}: as given");
        }
    }
}

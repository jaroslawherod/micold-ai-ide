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
use micold_core::tokens::motion::duration;

/// Below this the panel is gone and the rail has taken over. Not zero: a track converges *toward*
/// its target, and a drawer that only swaps at exactly zero would leave a sliver of panel on screen
/// for the last fraction of a pixel.
const CLOSED: f32 = 0.001;

/// How long the panel takes to slide its full range.
///
/// Owned here, not passed in: how long a thing takes is part of how it looks, and that is the
/// design system's business rather than the application's — the same reasoning that put the
/// dialog's timing in `modal` and the menu's in `menu`.
const SLIDE: Duration = Duration::from_millis(duration::MEDIUM_4);

/// A panel that slides in and out beside the content, leaving `rail` behind when closed. Builder
/// form (Principle VIII): `NavigationDrawer::new(panel, rail).open(b).handle(h).into()`.
pub struct NavigationDrawer<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    panel: Element<'a, M, Theme, Renderer>,
    rail: Element<'a, M, Theme, Renderer>,
    handle: Option<Element<'a, M, Theme, Renderer>>,
    open: bool,
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
        }
    }

    /// Whether the drawer is open. A destination, not a position — how far along it currently is
    /// is the drawer's own business.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
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

    /// Which child is on screen at `progress` — the drawer's whole reason to exist.
    fn showing_rail(&self, progress: f32) -> bool {
        !self.open && progress <= CLOSED
    }
}

/// The slide, owned here rather than by the application.
struct Track {
    progress: Progress,
}

/// A node parked where it cannot be seen. The inactive child still needs a layout entry — the tree,
/// the node list and the child list must stay index-aligned — but it must not occupy space.
fn parked(node: layout::Node) -> layout::Node {
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
        // slide open on the first frame of the session.
        tree::State::new(Track {
            progress: Progress::new(if self.open { 1.0 } else { 0.0 }),
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
        let progress = tree.state.downcast_ref::<Track>().progress.value();
        let rail_showing = self.showing_rail(progress);

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

        if rail_showing {
            let size = rail.size();
            let mut nodes = vec![parked(panel), rail];
            nodes.extend(handle.map(parked));
            return layout::Node::with_children(size, nodes);
        }

        // The panel reveals from its right edge: the space it occupies shrinks while the content
        // slides left behind it, so the part that disappears is the far side, not the near one.
        let full = panel.size();
        let width = (full.width * progress).max(0.0);
        let panel = panel.translate(Vector::new(-(full.width - width), 0.0));

        let handle_width = handle.as_ref().map_or(0.0, |h| h.size().width);
        let height = full
            .height
            .max(handle.as_ref().map_or(0.0, |h| h.size().height));
        let mut nodes = vec![panel, parked(rail)];
        nodes.extend(handle.map(|h| h.translate(Vector::new(width, 0.0))));

        layout::Node::with_children(Size::new(width + handle_width, height), nodes)
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
        let progress = track.progress.on_layout_frame(event, target, SLIDE, shell);
        let rail_showing = self.showing_rail(progress);

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
        let progress = tree.state.downcast_ref::<Track>().progress.value();
        let rail_showing = self.showing_rail(progress);
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
        let progress = tree.state.downcast_ref::<Track>().progress.value();
        let rail_showing = self.showing_rail(progress);

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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
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
        let progress = tree.state.downcast_ref::<Track>().progress.value();
        let rail_showing = !self.open && progress <= CLOSED;
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
}

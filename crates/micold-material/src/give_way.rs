//! The give-way planner: who yields first when a bar is too narrow for every child at its natural
//! width (feature 1124 R4, feature 1127 R3).
//!
//! One rule for both bars. The [`StatusBar`](crate::StatusBar) and the [`Toolbar`](crate::Toolbar)
//! are each a [`Lane`]: a fixed-height row that measures its children and asks [`plan`] for the
//! width each one gets. Every [`GiveWay::Drop`] child vanishes, lowest rank first, before any
//! [`GiveWay::Shorten`] child is narrowed, lowest rank first, down to its floor. [`GiveWay::Keep`]
//! never yields. A dropped child gets a zero-area node, so it neither draws nor has a hit area, and
//! what is drawn is clipped to the lane's bounds as a second line of defence.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// How a child yields when its bar is too narrow for every child at its natural width (1124 R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GiveWay {
    /// Never yields.
    #[default]
    Keep,
    /// Vanishes; lower ranks first, and every `Drop` before any `Shorten`.
    Drop(u8),
    /// Ellipsized down to its dot, one character and "…"; lower ranks first.
    Shorten(u8),
}

/// One child's widths for the give-way pass.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Widths {
    /// How this child yields.
    pub(crate) give_way: GiveWay,
    /// Its width with nothing taken away.
    pub(crate) natural: f32,
    /// The narrowest a `Shorten` child may become.
    pub(crate) floor: f32,
}

/// The width each child gets in `available`, separated by `gap`; `None` is a dropped child (R4).
///
/// While the total is over: drop `Drop` children by rank (within a rank, the last first, so of two
/// key hints the rightmost goes first); then shorten `Shorten` children by rank, each by as much of
/// the excess as its floor allows.
pub(crate) fn plan(items: &[Widths], gap: f32, available: f32) -> Vec<Option<f32>> {
    let mut widths: Vec<Option<f32>> = items.iter().map(|w| Some(w.natural)).collect();
    let total = |widths: &[Option<f32>]| {
        let shown: Vec<f32> = widths.iter().flatten().copied().collect();
        shown.iter().sum::<f32>() + gap * shown.len().saturating_sub(1) as f32
    };

    let mut drops: Vec<(u8, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, w)| match w.give_way {
            GiveWay::Drop(rank) => Some((rank, i)),
            _ => None,
        })
        .collect();
    drops.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    for (_, i) in drops {
        if total(&widths) <= available {
            break;
        }
        widths[i] = None;
    }

    let mut shortens: Vec<(u8, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, w)| match w.give_way {
            GiveWay::Shorten(rank) => Some((rank, i)),
            _ => None,
        })
        .collect();
    shortens.sort();
    for (_, i) in shortens {
        let excess = total(&widths) - available;
        if excess <= 0.0 {
            break;
        }
        if let Some(w) = widths[i] {
            widths[i] = Some((w - excess).max(items[i].floor.min(w)));
        }
    }
    widths
}

/// How narrow a `Shorten` child may become.
pub(crate) enum Floor {
    /// Its natural width: it never narrows.
    Natural,
    /// Its natural width less this much: what shortening its text to its floor text saves.
    Saving(f32),
    /// The width it lays out at with no room at all: whatever it cannot give up (its padding, a
    /// glyph, an ellipsis).
    Squeezed,
}

/// One child of a [`Lane`] beside its element.
pub(crate) struct Slot {
    /// How it yields.
    pub(crate) give_way: GiveWay,
    /// Whether it belongs to the trailing group, placed from the lane's trailing end.
    pub(crate) trailing: bool,
    /// How narrow it may become when it shortens.
    pub(crate) floor: Floor,
}

/// What a lane paints under its children: its surface, a separator.
pub(crate) type Backdrop<'a> = Box<dyn Fn(&mut iced::Renderer, Rectangle) + 'a>;

/// A row laid out by [`plan`]: a leading group from the leading end and a trailing group from the
/// trailing end, `padding` from each end and `gap` apart, every child centred vertically.
///
/// Two shapes. A **bar** ([`Lane::new`]) is a fixed height and the full width it is given, and
/// clips what it draws to its bounds. A **hugging** lane ([`Lane::hugging`]) is as wide and as tall
/// as its children, like a `row!`, and only narrower than that when its room is: a labelled button's
/// glyph, label and mark, where the label is the part that shortens.
pub(crate) struct Lane<'a, M> {
    children: Vec<Element<'a, M>>,
    slots: Vec<Slot>,
    /// `None` hugs: as tall as the tallest child.
    height: Option<f32>,
    padding: f32,
    gap: f32,
    /// As wide as the children rather than the whole width given, and no clip.
    hugging: bool,
    backdrop: Option<Backdrop<'a>>,
}

impl<'a, M> Lane<'a, M> {
    /// A lane `height` tall with `padding` at each end and `gap` between children.
    pub(crate) fn new(height: f32, padding: f32, gap: f32) -> Self {
        Self {
            children: Vec::new(),
            slots: Vec::new(),
            height: Some(height),
            padding,
            gap,
            hugging: false,
            backdrop: None,
        }
    }

    /// A lane as wide and as tall as its children, `gap` apart, with no padding of its own.
    pub(crate) fn hugging(gap: f32) -> Self {
        Self {
            height: None,
            padding: 0.0,
            hugging: true,
            ..Self::new(0.0, 0.0, gap)
        }
    }

    /// A child after those already added; its group is the slot's.
    pub(crate) fn push(mut self, child: Element<'a, M>, slot: Slot) -> Self {
        self.children.push(child);
        self.slots.push(slot);
        self
    }

    /// Paint `backdrop` under the children, over the lane's whole bounds.
    pub(crate) fn backdrop(mut self, backdrop: Backdrop<'a>) -> Self {
        self.backdrop = Some(backdrop);
        self
    }

    /// The children shown, with their trees and laid-out nodes; a dropped one is left out.
    fn shown<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
    ) -> impl Iterator<Item = ((&'b mut Element<'a, M>, &'b mut Tree), Layout<'b>)> {
        let hugging = self.hugging;
        self.children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .filter(move |(_, l)| !dropped(hugging, *l))
    }
}

/// Whether a laid-out child of a bar was dropped: a zero-area node. A hugging lane drops nothing,
/// and keeps a child that happens to be empty as a `row!` would.
fn dropped(hugging: bool, layout: Layout<'_>) -> bool {
    let b = layout.bounds();
    !hugging && (b.width <= 0.0 || b.height <= 0.0)
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Lane<'_, M> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        let width = if self.hugging {
            Length::Shrink
        } else {
            Length::Fill
        };
        Size::new(width, self.height.map_or(Length::Shrink, Length::Fixed))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let room = self.height.unwrap_or(limits.max().height);
        // The room as given, not `resolve(Fill)`: inside a shrinking parent the limits compress, and
        // a compressed `Fill` resolves to nothing, which would shorten every label to its floor.
        let available = (limits.max().width - 2.0 * self.padding).max(0.0);
        let within = |width: f32| layout::Limits::new(Size::ZERO, Size::new(width, room));

        // Each child at its natural width first. When they all fit, those are the nodes: no floor
        // to measure and nothing to lay out again, so an ordinary button or bar lays each child
        // out once, as a row would.
        let naturals: Vec<layout::Node> = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .map(|(child, state)| {
                child
                    .as_widget_mut()
                    .layout(state, renderer, &within(f32::INFINITY))
            })
            .collect();
        let total = naturals.iter().map(|n| n.size().width).sum::<f32>()
            + self.gap * naturals.len().saturating_sub(1) as f32;
        let (planned, mut nodes) = if total <= available {
            let planned = naturals.iter().map(|n| Some(n.size().width)).collect();
            (planned, naturals)
        } else {
            let widths: Vec<Widths> = self
                .children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(&self.slots)
                .zip(&naturals)
                .map(|(((child, state), slot), node)| {
                    let natural = node.size().width;
                    let floor = match (&slot.floor, slot.give_way) {
                        (Floor::Saving(saved), GiveWay::Shorten(_)) => {
                            (natural - saved.max(0.0)).ceil()
                        }
                        (Floor::Squeezed, GiveWay::Shorten(_)) => child
                            .as_widget_mut()
                            .layout(state, renderer, &within(0.0))
                            .size()
                            .width
                            .min(natural),
                        _ => natural,
                    };
                    Widths {
                        give_way: slot.give_way,
                        natural,
                        floor,
                    }
                })
                .collect();
            let planned = plan(&widths, self.gap, available);
            let nodes = self
                .children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(&planned)
                .map(|((child, state), width)| match width {
                    Some(w) => child.as_widget_mut().layout(state, renderer, &within(*w)),
                    None => layout::Node::new(Size::ZERO),
                })
                .collect();
            (planned, nodes)
        };

        // A dropped child takes no place; an empty one in a hugging lane still does, as in a row.
        let placed = |node: &layout::Node, width: &Option<f32>| {
            width.is_some() && (self.hugging || node.size().width > 0.0)
        };
        let height = self
            .height
            .unwrap_or_else(|| nodes.iter().map(|n| n.size().height).fold(0.0, f32::max));
        let size = if self.hugging {
            let shown: Vec<f32> = nodes
                .iter()
                .zip(&planned)
                .filter(|(n, w)| placed(n, w))
                .map(|(n, _)| n.size().width)
                .collect();
            let content = shown.iter().sum::<f32>()
                + self.gap * shown.len().saturating_sub(1) as f32
                + 2.0 * self.padding;
            limits.resolve(Length::Shrink, Length::Shrink, Size::new(content, height))
        } else {
            limits.resolve(Length::Fill, Length::Fixed(height), Size::ZERO)
        };

        let centred = |node: &layout::Node, x: f32| {
            Point::new(x, ((height - node.size().height) / 2.0).max(0.0))
        };
        let mut x = self.padding;
        for ((node, slot), width) in nodes.iter_mut().zip(&self.slots).zip(&planned) {
            if slot.trailing || !placed(node, width) {
                continue;
            }
            let at = centred(node, x);
            node.move_to_mut(at);
            x += node.size().width + self.gap;
        }
        let mut x = size.width - self.padding;
        for ((node, slot), width) in nodes.iter_mut().zip(&self.slots).zip(&planned).rev() {
            if !slot.trailing || !placed(node, width) {
                continue;
            }
            x -= node.size().width;
            let at = centred(node, x);
            node.move_to_mut(at);
            x -= self.gap;
        }
        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.traverse(&mut |operation| {
            for ((child, state), l) in self.shown(tree, layout) {
                child.as_widget_mut().operate(state, l, renderer, operation);
            }
        });
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
        for ((child, state), l) in self.shown(tree, layout) {
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
            .filter(|(_, l)| !dropped(self.hugging, *l))
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
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        if let Some(backdrop) = &self.backdrop {
            backdrop(renderer, bounds);
        }
        let draw_children = |renderer: &mut iced::Renderer| {
            for ((child, state), l) in self
                .children
                .iter()
                .zip(tree.children.iter())
                .zip(layout.children())
            {
                if dropped(self.hugging, l) {
                    continue;
                }
                child
                    .as_widget()
                    .draw(state, renderer, theme, style, l, cursor, viewport);
            }
        };
        // A bar clips what it draws to its bounds, a second line of defence behind the give-way
        // pass; a hugging lane draws as a row does.
        if self.hugging {
            draw_children(renderer);
            return;
        }
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(visible, draw_children);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let overlays: Vec<_> = self
            .shown(tree, layout)
            .filter_map(|((child, state), l)| {
                child
                    .as_widget_mut()
                    .overlay(state, l, renderer, viewport, translation)
            })
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, M: 'a> From<Lane<'a, M>> for Element<'a, M> {
    fn from(lane: Lane<'a, M>) -> Self {
        Element::new(lane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(give_way: GiveWay, natural: f32, floor: f32) -> Widths {
        Widths {
            give_way,
            natural,
            floor,
        }
    }

    /// B4, the order: `Drop(0)` before `Drop(1)`, every `Drop` before any `Shorten`.
    #[test]
    fn drops_go_by_rank_before_any_shortening() {
        let items = [
            w(GiveWay::Shorten(2), 100.0, 20.0),
            w(GiveWay::Shorten(1), 100.0, 20.0),
            w(GiveWay::Drop(1), 50.0, 50.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
        ];
        // All fit: 300 + 3 gaps of 10.
        assert_eq!(
            plan(&items, 10.0, 330.0),
            vec![Some(100.0), Some(100.0), Some(50.0), Some(50.0)]
        );
        // 10 over: the rank-0 drop goes, and nothing else changes.
        assert_eq!(
            plan(&items, 10.0, 320.0),
            vec![Some(100.0), Some(100.0), Some(50.0), None]
        );
        // Still over without the rank-1 drop too: both drops go before any shortening.
        assert_eq!(
            plan(&items, 10.0, 260.0),
            vec![Some(100.0), Some(100.0), None, None]
        );
        // Over with both drops gone: Shorten(1) gives first.
        assert_eq!(
            plan(&items, 10.0, 180.0),
            vec![Some(100.0), Some(70.0), None, None]
        );
    }

    /// B4: `Shorten(0)` before `Shorten(1)` before `Shorten(2)`, each down to its floor.
    #[test]
    fn shortening_goes_by_rank_down_to_the_floor() {
        let items = [
            w(GiveWay::Shorten(2), 100.0, 20.0),
            w(GiveWay::Shorten(1), 100.0, 20.0),
            w(GiveWay::Shorten(0), 100.0, 20.0),
        ];
        assert_eq!(
            plan(&items, 0.0, 250.0),
            vec![Some(100.0), Some(100.0), Some(50.0)]
        );
        assert_eq!(
            plan(&items, 0.0, 180.0),
            vec![Some(100.0), Some(60.0), Some(20.0)]
        );
        assert_eq!(
            plan(&items, 0.0, 50.0),
            vec![Some(20.0), Some(20.0), Some(20.0)],
            "nothing goes below its floor"
        );
    }

    /// Of two key hints at the same rank, the rightmost goes first.
    #[test]
    fn of_two_hints_the_rightmost_drops_first() {
        let items = [
            w(GiveWay::Keep, 100.0, 100.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
        ];
        assert_eq!(
            plan(&items, 0.0, 160.0),
            vec![Some(100.0), Some(50.0), None]
        );
    }
}

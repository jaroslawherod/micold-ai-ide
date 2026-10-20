//! `CardGrid`: the Overview's responsive grid of [`OverviewCard`]s (feature 813, FR-021).
//!
//! The number of columns follows the width the grid is given (iced `responsive`): as many as fit at
//! the minimum card width, never fewer than one and at most [`MAX_COLUMNS`] (five, feature 1024).
//! The cards share the width equally, so a wider grid makes wider cards, and the grid scrolls
//! vertically.

use std::time::Duration;

use crate::ui::cdk::motion::Progress;
use crate::ui::material::navigation_drawer::SLIDE;
use crate::ui::material::{OverviewCard, Scrollable};
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Id, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};
use micold_core::tokens::motion::duration;
use micold_core::tokens::{spacing, Roles};

/// The narrowest a card is drawn.
pub const MIN_WIDTH: f32 = 280.0;

/// The most cards a row holds (feature 1024): wider panes make wider cards, not more of them.
pub const MAX_COLUMNS: usize = 5;

/// The columns that fit in `width`, capped at `max` (`0` is treated as `1`):
/// `clamp(floor((width + gap) / (min + gap)), 1, max)`.
pub fn columns(width: f32, min: f32, gap: f32, max: usize) -> usize {
    (((width + gap) / (min + gap)).floor() as usize).clamp(1, max.max(1))
}

/// The grid. See the module documentation.
pub struct CardGrid<'a, M> {
    cards: Vec<OverviewCard<'a, M>>,
    roles: Roles,
    min_width: f32,
    spacing: f32,
    max_columns: usize,
    on_scroll: Option<M>,
    zoom: Zoom,
    on_reflow: Option<OnReflow<'a, M>>,
    scroll_id: Option<Id>,
}

/// What a grid asks its owner when a zoom moves the card at the top of the view.
type OnReflow<'a, M> = Box<dyn Fn(AbsoluteOffset) -> M + 'a>;

impl<'a, M: Clone + 'a> CardGrid<'a, M> {
    /// A grid of `cards`, themed by `roles`.
    pub fn new(cards: Vec<OverviewCard<'a, M>>, roles: Roles) -> Self {
        Self {
            cards,
            roles,
            min_width: MIN_WIDTH,
            spacing: spacing::MD,
            max_columns: MAX_COLUMNS,
            on_scroll: None,
            zoom: Zoom::default(),
            on_reflow: None,
            scroll_id: None,
        }
    }

    /// The narrowest a card may be drawn.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// The gap between cards, across and down.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// The most cards a row holds; `0` is treated as `1`. Defaults to [`MAX_COLUMNS`].
    pub fn max_columns(mut self, n: usize) -> Self {
        self.max_columns = n;
        self
    }

    /// Publish `message` whenever the grid scrolls, so the root can close a card menu the grid
    /// scrolled beneath (feature 972, FR-012).
    pub fn on_scroll(mut self, message: M) -> Self {
        self.on_scroll = Some(message);
        self
    }
}

/// How the Overview is zoomed (feature 970): whether it is in full-screen mode, and how much
/// width that mode gives it (the sidebar's width when expanded, its strip when collapsed).
///
/// A flip of `full_screen` between two builds of the grid starts a zoom; see [`ZoomState`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Zoom {
    /// Whether the Overview is full screen.
    pub full_screen: bool,
    /// The width full screen adds to the grid, in logical pixels.
    pub freed: f32,
}

/// How far `width` lies from `start` toward `end`, clamped to `0..=1` (either order of ends).
pub fn zoom_fraction(start: f32, end: f32, width: f32) -> f32 {
    if (end - start).abs() < f32::EPSILON {
        return 1.0;
    }
    ((width - start) / (end - start)).clamp(0.0, 1.0)
}

/// The width of a card in a grid `width` wide at rest (feature 1024's rule).
fn rest_width(width: f32, min: f32, gap: f32, max_columns: usize) -> f32 {
    let n = columns(width, min, gap, max_columns) as f32;
    (width - (n - 1.0) * gap) / n
}

/// Tolerances, in pixels: where a card's span is taken to start on a row, and the narrowest
/// piece worth drawing.
const SNAP: f32 = 1e-3;
const MIN_PIECE: f32 = 0.01;

/// One drawn part of a card: the rectangle it shows and where the whole card would sit.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// The card's index.
    pub card: usize,
    /// The visible rectangle, in the grid's coordinates; the clip the card is drawn under.
    pub rect: Rectangle,
    /// Where the whole card's top-left falls for this piece (left of `rect` when the card begins
    /// on the row before).
    pub origin: Point,
    /// The index of the layout node the piece is drawn from.
    node: usize,
}

/// Cards of width `w` on a ribbon: card `i` starts at `i · (w + gap)` along rows `width + gap`
/// long, so row `r` shows the stretch `r · (width + gap) .. + width` and the `gap` after it is
/// not shown. A card that runs past a row's end is cut there and continues at the next row's
/// start; at a rest width (`width + gap = n · (w + gap)`) no card is cut.
///
/// Cards keep reading order and a constant pitch, so no two pieces of different cards intersect,
/// and a card never disappears: of its `w` it loses at most the `gap`.
pub fn ribbon(count: usize, w: f32, gap: f32, width: f32, card_height: f32) -> Vec<Piece> {
    let line = width + gap;
    let mut pieces = Vec::with_capacity(count + 2);
    for card in 0..count {
        let start = card as f32 * (w + gap);
        let first = ((start + SNAP) / line).floor().max(0.0);
        for row in [first, first + 1.0] {
            let origin = start - row * line;
            let left = origin.max(0.0);
            let right = (origin + w).min(width);
            if right - left >= MIN_PIECE {
                let y = row * (card_height + gap);
                pieces.push(Piece {
                    card,
                    rect: Rectangle::new(Point::new(left, y), Size::new(right - left, card_height)),
                    origin: Point::new(origin, y),
                    node: 0,
                });
            }
            if origin + w <= line + SNAP {
                break;
            }
        }
    }
    pieces
}

/// The zoom a grid is in, kept in the widget tree between layouts (data-model.md).
#[derive(Debug)]
struct ZoomState {
    /// `Zoom::full_screen` at the last layout, to see it flip.
    full_screen_seen: bool,
    /// The widths a zoom runs between: the width before the flip, and that width ± `freed`.
    ends: Option<(f32, f32)>,
    /// Runs from 0 at a flip to 1 when the slide and 100 ms are over: the zoom's time limit, and
    /// the one thing that asks for frames while it runs (through `Progress`, FR-025).
    clock: Progress,
    /// The width of the last layout, which a flip takes as its start.
    last_width: Option<f32>,
    /// What the last layout drew.
    pieces: Vec<Piece>,
    /// The card at the top of the view and where its top was when `update` last looked (the
    /// anchor: R6).
    anchor: Option<(usize, f32)>,
    /// Whether the last `update` saw a zoom, so the layout that ends it is answered too.
    was_zooming: bool,
}

impl ZoomState {
    fn new(full_screen: bool) -> Self {
        Self {
            full_screen_seen: full_screen,
            ends: None,
            clock: Progress::new(1.0),
            last_width: None,
            pieces: Vec::new(),
            anchor: None,
            was_zooming: false,
        }
    }

    /// Takes the next layout's `width`: starts a zoom on a flip, keeps it on a reversal, and ends
    /// it when the width reaches the end it heads for, leaves the range, or the slide is over.
    fn advance(&mut self, zoom: Zoom, width: f32) -> Option<(f32, f32)> {
        if zoom.full_screen != self.full_screen_seen {
            self.full_screen_seen = zoom.full_screen;
            self.clock.restart_at(0.0);
            if self.ends.is_none() && zoom.freed >= 1.0 {
                if let Some(before) = self.last_width {
                    let after = if zoom.full_screen {
                        before + zoom.freed
                    } else {
                        before - zoom.freed
                    };
                    self.ends = Some((before, after));
                }
            }
        }
        if let Some((a, b)) = self.ends {
            let (low, high) = (a.min(b), a.max(b));
            let heading = if zoom.full_screen { high } else { low };
            if (width - heading).abs() <= END_REACHED
                || width < low - END_REACHED
                || width > high + END_REACHED
                || self.clock.value() >= 1.0
            {
                self.ends = None;
            }
        }
        self.last_width = Some(width);
        self.ends
    }
}

/// The first card with a piece in the band `top..bottom` and where its top is: the smallest
/// card index, at the highest of its pieces.
fn first_visible(pieces: &[Piece], top: f32, bottom: f32) -> Option<(usize, f32)> {
    let card = pieces
        .iter()
        .filter(|p| p.rect.y < bottom && p.rect.y + p.rect.height > top)
        .map(|p| p.card)
        .min()?;
    card_top(pieces, card).map(|y| (card, y))
}

/// Where `card`'s top is: its highest piece.
fn card_top(pieces: &[Piece], card: usize) -> Option<f32> {
    pieces
        .iter()
        .filter(|p| p.card == card)
        .map(|p| p.rect.y)
        .reduce(f32::min)
}

/// How long a zoom may last: the sidebar's slide, and a little more.
const ZOOM_LIMIT: Duration = SLIDE.saturating_add(Duration::from_millis(duration::SHORT_2));

/// How close to an end width counts as there.
const END_REACHED: f32 = 0.5;

impl<'a, M: Clone + 'a> CardGrid<'a, M> {
    /// Zoom with the sidebar (feature 970): a flip of `zoom.full_screen` between two builds of the
    /// grid moves the cards from the layout they had to the one the new width gives, in step with
    /// the width. Defaults to no zoom.
    pub fn zoom(mut self, zoom: Zoom) -> Self {
        self.zoom = zoom;
        self
    }

    /// Keep the card at the top of the view in view while a zoom moves the cards (feature 970,
    /// FR-013): each layout step that moves it, `f` gets the scroll offset that undoes the move,
    /// for the owner to apply to the grid's scrollable ([`Self::scroll_id`]). Nothing is asked at
    /// rest.
    pub fn on_reflow(mut self, f: impl Fn(AbsoluteOffset) -> M + 'a) -> Self {
        self.on_reflow = Some(Box::new(f));
        self
    }

    /// Name the grid's scrollable, so `operation::scroll_to` can reach it.
    pub fn scroll_id(mut self, id: impl Into<Id>) -> Self {
        self.scroll_id = Some(id.into());
        self
    }

    /// The grid proper, without its scrollable.
    fn into_grid(self) -> Grid<'a, M> {
        Grid {
            cards: self
                .cards
                .iter()
                .map(|card| Element::from(card.clone()))
                .collect(),
            min_width: self.min_width,
            spacing: self.spacing,
            max_columns: self.max_columns,
            zoom: self.zoom,
            on_reflow: self.on_reflow,
        }
    }
}

/// The cards laid out by the rest rule, or on the ribbon during a zoom. It owns the cards, so it
/// can place one in two pieces at a row end.
struct Grid<'a, M> {
    cards: Vec<Element<'a, M>>,
    min_width: f32,
    spacing: f32,
    max_columns: usize,
    zoom: Zoom,
    on_reflow: Option<OnReflow<'a, M>>,
}

impl<M> Grid<'_, M> {
    /// Where the pointer is on `card`'s own (primary) box: unchanged at rest; during a zoom, the
    /// pointer over any piece of the card translated to the box, and none elsewhere.
    fn cursor_for_state(
        state: &ZoomState,
        origin: Point,
        card: usize,
        node_at: Point,
        cursor: mouse::Cursor,
    ) -> mouse::Cursor {
        if state.ends.is_none() {
            return cursor;
        }
        let Some(position) = cursor.position() else {
            return cursor;
        };
        let local = Point::new(position.x - origin.x, position.y - origin.y);
        state
            .pieces
            .iter()
            .filter(|piece| piece.card == card)
            .find(|piece| piece.rect.contains(local))
            .map_or(mouse::Cursor::Unavailable, |piece| {
                mouse::Cursor::Available(Point::new(
                    local.x - piece.origin.x + node_at.x,
                    local.y - piece.origin.y + node_at.y,
                ))
            })
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Grid<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ZoomState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ZoomState::new(self.zoom.full_screen))
    }

    fn children(&self) -> Vec<Tree> {
        self.cards
            .iter()
            .map(|card| Tree::new(card.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children_custom(
            &self.cards,
            |child_tree, card| child_tree.diff(card.as_widget()),
            |card| Tree::new(card.as_widget()),
        );
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width;
        let width = if width.is_finite() {
            width
        } else {
            self.min_width
        };
        let (min, gap, max) = (self.min_width, self.spacing, self.max_columns);
        let state = tree.state.downcast_mut::<ZoomState>();
        let card_width = match state.advance(self.zoom, width) {
            Some((start, end)) => {
                let f = zoom_fraction(start, end, width);
                let at_start = rest_width(start, min, gap, max);
                let at_end = rest_width(end, min, gap, max);
                at_start + (at_end - at_start) * f
            }
            None => rest_width(width, min, gap, max),
        };
        let mut pieces = ribbon(self.cards.len(), card_width, gap, width, CARD_HEIGHT);

        // One node per card, at its largest piece; the other piece of a split card follows.
        let mut primary: Vec<Option<usize>> = vec![None; self.cards.len()];
        for (index, piece) in pieces.iter().enumerate() {
            let best = primary[piece.card].map_or(0.0, |i| pieces[i].rect.width);
            if piece.rect.width > best {
                primary[piece.card] = Some(index);
            }
        }
        let card_limits = layout::Limits::new(
            Size::new(card_width, CARD_HEIGHT),
            Size::new(card_width, CARD_HEIGHT),
        );
        let mut nodes = Vec::with_capacity(self.cards.len() + 2);
        for (i, (card, card_tree)) in self.cards.iter_mut().zip(&mut tree.children).enumerate() {
            let node = card
                .as_widget_mut()
                .layout(card_tree, renderer, &card_limits);
            let at = primary[i].map_or(Point::ORIGIN, |p| pieces[p].origin);
            nodes.push(node.move_to(at));
        }
        let mut extra = Vec::new();
        for (index, piece) in pieces.iter_mut().enumerate() {
            if primary[piece.card] == Some(index) {
                piece.node = piece.card;
            } else {
                piece.node = nodes.len() + extra.len();
                extra.push(nodes[piece.card].clone().move_to(piece.origin));
            }
        }
        nodes.extend(extra);

        let rows = pieces
            .iter()
            .map(|piece| (piece.rect.y / (CARD_HEIGHT + gap)).round() as usize + 1)
            .max()
            .unwrap_or(0);
        let height = if rows == 0 {
            0.0
        } else {
            rows as f32 * CARD_HEIGHT + (rows - 1) as f32 * gap
        };
        tree.state.downcast_mut::<ZoomState>().pieces = pieces;
        layout::Node::with_children(Size::new(width, height), nodes)
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
        let zooming = tree.state.downcast_ref::<ZoomState>().ends.is_some();
        // The widths move with the sidebar and the zoom ends on a layout, so ask for both while it
        // runs; nothing is asked for at rest.
        if zooming {
            tree.state
                .downcast_mut::<ZoomState>()
                .clock
                .on_layout_frame(event, 1.0, ZOOM_LIMIT, shell);
        }
        if let Some(on_reflow) = &self.on_reflow {
            let state = tree.state.downcast_mut::<ZoomState>();
            // The visible top, in the grid's own coordinates (`viewport` is the scrollable's
            // window in the content's).
            let top = viewport.y - layout.position().y;
            let mut shift = 0.0;
            if zooming || state.was_zooming {
                let moved = state
                    .anchor
                    .and_then(|(card, was)| card_top(&state.pieces, card).map(|now| now - was));
                shift = moved.unwrap_or(0.0);
            }
            if shift.abs() >= MIN_PIECE {
                shell.publish(on_reflow(AbsoluteOffset {
                    x: 0.0,
                    y: (top + shift).max(0.0),
                }));
            }
            state.anchor = first_visible(&state.pieces, top + shift, top + shift + viewport.height);
            state.was_zooming = zooming;
        }
        let origin = layout.position();
        let (state_tree, children) = (&tree.state, &mut tree.children);
        let state = state_tree.downcast_ref::<ZoomState>();
        for (i, (card, card_tree)) in self.cards.iter_mut().zip(children).enumerate() {
            let Some(card_layout) = layout.children().nth(i) else {
                continue;
            };
            let at = card_layout.position();
            let local = Grid::<M>::cursor_for_state(state, origin, i, at, cursor);
            card.as_widget_mut().update(
                card_tree,
                event,
                card_layout,
                local,
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
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let origin = layout.position();
        let state = tree.state.downcast_ref::<ZoomState>();
        self.cards
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .map(|(i, ((card, card_tree), card_layout))| {
                let local =
                    Grid::<M>::cursor_for_state(state, origin, i, card_layout.position(), cursor);
                card.as_widget().mouse_interaction(
                    card_tree,
                    card_layout,
                    local,
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
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<ZoomState>();
        let origin = layout.position();
        for piece in &state.pieces {
            let card_layout = layout.child(piece.node);
            let bounds = card_layout.bounds();
            if !bounds.intersects(viewport) {
                continue;
            }
            let clip = Rectangle {
                x: origin.x + piece.rect.x,
                y: origin.y + piece.rect.y,
                width: piece.rect.width,
                height: piece.rect.height,
            };
            let whole = (clip.width - bounds.width).abs() < MIN_PIECE;
            let draw_card = |renderer: &mut iced::Renderer| {
                let local = if state.ends.is_none()
                    || cursor.position().is_some_and(|p| clip.contains(p))
                {
                    cursor
                } else {
                    mouse::Cursor::Unavailable
                };
                self.cards[piece.card].as_widget().draw(
                    &tree.children[piece.card],
                    renderer,
                    theme,
                    style,
                    card_layout,
                    local,
                    viewport,
                );
            };
            if whole {
                draw_card(renderer);
            } else {
                renderer.with_layer(clip, draw_card);
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((card, card_tree), card_layout) in self
            .cards
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            card.as_widget_mut()
                .operate(card_tree, card_layout, renderer, operation);
        }
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
            &mut self.cards,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// The height of every card (feature 813).
const CARD_HEIGHT: f32 = super::overview_card::HEIGHT;

impl<'a, M: Clone + 'a> From<CardGrid<'a, M>> for Element<'a, M> {
    fn from(g: CardGrid<'a, M>) -> Self {
        let on_scroll = g.on_scroll.clone();
        let roles = g.roles;
        let scroll_id = g.scroll_id.clone();
        let mut scrollable =
            Scrollable::new(Element::new(g.into_grid()), roles).height(Length::Fill);
        if let Some(id) = scroll_id {
            scrollable = scrollable.id(id);
        }
        match on_scroll {
            Some(message) => scrollable.on_scroll(message).into(),
            None => scrollable.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{laid_out, leaves, roles};
    use iced::{Rectangle, Size};
    use micold_core::overview::{CardModel, CardReason, CardState, LocationType};
    use micold_core::session::SessionLocation;

    /// The gap between cards, and the tolerance for comparing laid-out pixels.
    const GAP: f32 = 16.0;
    const EPS: f32 = 0.01;

    fn cards<M: 'static>(n: usize) -> Vec<OverviewCard<'static, M>> {
        (0..n)
            .map(|i| {
                OverviewCard::new(
                    &CardModel {
                        location: SessionLocation::Worktree(format!("w{i}")),
                        display_name: format!("feat-{i}"),
                        branch: None,
                        kind: LocationType::Feature,
                        sessions: (1, 0),
                        state: CardState::Idle,
                        reason: CardReason::Idle,
                        title: "s".into(),
                        prs: micold_core::overview::PrDisplay::Hidden,
                        progress: Default::default(),
                        activity: None,
                        activation: micold_core::overview::Activation::RevealOnly(
                            micold_core::session::SessionLocation::Default,
                        ),
                    },
                    roles(),
                )
            })
            .collect()
    }

    fn grid(n: usize) -> Element<'static, ()> {
        CardGrid::new(cards(n), roles())
            .min_width(280.0)
            .spacing(GAP)
            .into()
    }

    /// The card-sized boxes (full card height) of a laid-out grid, left to right, top to bottom.
    fn card_boxes(node: &iced::advanced::layout::Node) -> Vec<Rectangle> {
        fn walk(n: &iced::advanced::layout::Node, at: iced::Vector, out: &mut Vec<Rectangle>) {
            let b = n.bounds() + at;
            let before = out.len();
            for c in n.children() {
                walk(c, iced::Vector::new(b.x, b.y), out);
            }
            // The innermost box of the card's height is the card: its row is as tall, but holds it.
            if out.len() == before && (b.height - super::super::overview_card::HEIGHT).abs() < EPS {
                out.push(b);
            }
        }
        let mut out = Vec::new();
        walk(node, iced::Vector::ZERO, &mut out);
        out
    }

    /// U4 (813): columns = max(1, floor((w + gap) / (min + gap))).
    #[test]
    fn columns_follow_the_width_and_never_drop_below_one() {
        assert_eq!(columns(279.0, 280.0, GAP, MAX_COLUMNS), 1);
        assert_eq!(columns(280.0, 280.0, GAP, MAX_COLUMNS), 1);
        assert_eq!(columns(575.0, 280.0, GAP, MAX_COLUMNS), 1);
        assert_eq!(columns(576.0, 280.0, GAP, MAX_COLUMNS), 2);
        assert_eq!(columns(1000.0, 280.0, GAP, MAX_COLUMNS), 3);
        assert_eq!(columns(0.0, 280.0, GAP, MAX_COLUMNS), 1);
    }

    /// U4 (813): at 279 px the grid is one column, at 900 px three.
    #[test]
    fn the_laid_out_grid_has_the_columns_its_width_gives() {
        for (width, expected) in [(279.0, 1), (900.0, 3), (1463.0, 4)] {
            let node = laid_out(grid(6), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), 6, "{width}");
            let first_row = boxes.iter().filter(|b| b.y == boxes[0].y).count();
            assert_eq!(first_row, expected, "{width}");
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + EPS),
                "{width}"
            );
        }
    }

    /// U4 (813): no card overlaps another, and no piece of one grid overlaps any other.
    #[test]
    fn no_card_overlaps_another() {
        for width in [279.0, 600.0, 1000.0, 1464.0, 2400.0, 3840.0] {
            let node = laid_out(grid(7), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + EPS),
                "{width}: a card passes the right edge"
            );
            for (i, a) in boxes.iter().enumerate() {
                for b in &boxes[i + 1..] {
                    let apart = a.x + a.width <= b.x + EPS
                        || b.x + b.width <= a.x + EPS
                        || a.y + a.height <= b.y + EPS
                        || b.y + b.height <= a.y + EPS;
                    assert!(apart, "{width}: {a:?} overlaps {b:?}");
                }
            }
            assert!(!leaves(&node).is_empty());
        }
    }

    /// Feature 972, FR-012: with `on_scroll`, a wheel scroll over the grid publishes its message,
    /// so the root can close a card menu the grid scrolled beneath.
    #[test]
    fn a_wheel_scroll_over_the_grid_publishes_on_scroll() {
        use crate::ui::material::test_support::Mounted;
        use iced::advanced::mouse::{Cursor, Event as Mouse, ScrollDelta};
        let mut grid = Mounted::new(
            CardGrid::new(cards::<String>(30), roles()).on_scroll("scrolled".to_string()),
        );
        let at = iced::Point::new(100.0, 100.0);
        grid.hover(at);
        let published = grid.send(
            iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: -3.0 },
            }),
            Cursor::Available(at),
        );
        assert!(
            published.iter().any(|m| m == "scrolled"),
            "a wheel scroll publishes on_scroll: {published:?}"
        );
    }

    /// U4 (813): 30 cards in a 400 px viewport scroll: the content is taller than the grid. Feature 1024
    /// (U7): at 2400 px too, and the grid never grows sideways.
    #[test]
    fn thirty_cards_scroll_vertically() {
        for width in [600.0, 2400.0] {
            let node = laid_out(grid(30), Size::new(width, 400.0));
            let boxes = card_boxes(&node);
            let bottom = boxes.iter().map(|b| b.y + b.height).fold(0.0, f32::max);
            assert_eq!(boxes.len(), 30, "{width}");
            assert!(bottom > 400.0, "{width}: content ends at {bottom}");
            assert!(node.size().height <= 400.0, "{width}: {:?}", node.size());
            assert!(node.size().width <= width, "{width}: {:?}", node.size());
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + EPS),
                "{width}"
            );
        }
    }

    /// The card width a full row of `n` columns gives in `width`, with the tests' 16 px gap.
    fn row_width(width: f32, n: f32) -> f32 {
        (width - (n - 1.0) * GAP) / n
    }

    /// The cards of each row, top to bottom.
    fn row_counts(boxes: &[Rectangle]) -> Vec<usize> {
        let mut counts: Vec<(f32, usize)> = Vec::new();
        for b in boxes {
            match counts.iter_mut().find(|(y, _)| (*y - b.y).abs() < EPS) {
                Some((_, n)) => *n += 1,
                None => counts.push((b.y, 1)),
            }
        }
        counts.into_iter().map(|(_, n)| n).collect()
    }

    /// U1 (1024): the column count stops at `max`; one pixel under five columns' threshold is four.
    #[test]
    fn columns_are_capped_at_max() {
        assert_eq!(columns(1463.0, 280.0, GAP, MAX_COLUMNS), 4);
        for width in [1464.0, 2400.0, 3840.0, 10_000.0] {
            assert_eq!(columns(width, 280.0, GAP, MAX_COLUMNS), 5, "{width}");
        }
        assert_eq!(
            columns(2400.0, 280.0, GAP, 0),
            1,
            "a cap of 0 is treated as 1"
        );
    }

    /// U2 (1024): from the five-card threshold up, full rows hold five equal cards and the last,
    /// partial row starts where the first does.
    #[test]
    fn full_rows_hold_five_cards_from_the_threshold_up() {
        for width in [1464.0, 2400.0, 3840.0] {
            let node = laid_out(grid(12), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), 12, "{width}");
            assert_eq!(row_counts(&boxes), vec![5, 5, 2], "{width}");
            let w = row_width(width, 5.0);
            for b in &boxes {
                assert!((b.width - w).abs() < EPS, "{width}: {} != {w}", b.width);
            }
            let last_y = boxes.iter().map(|b| b.y).fold(0.0, f32::max);
            let last_x = boxes
                .iter()
                .filter(|b| (b.y - last_y).abs() < EPS)
                .map(|b| b.x)
                .fold(f32::MAX, f32::min);
            assert!(
                (last_x - boxes[0].x).abs() < EPS,
                "{width}: last row not left-aligned"
            );
        }
    }

    /// U3 (1024): a grid with fewer cards than columns keeps the five-card width, from the left.
    #[test]
    fn fewer_cards_than_columns_keep_the_five_card_width() {
        let w = row_width(2400.0, 5.0);
        for n in [3, 1] {
            let node = laid_out(grid(n), Size::new(2400.0, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), n);
            for b in &boxes {
                assert!((b.width - w).abs() < EPS, "{n} cards: {} != {w}", b.width);
            }
            assert!(
                boxes[0].x.abs() < EPS,
                "{n} cards: first starts at {}",
                boxes[0].x
            );
        }
    }

    /// U4 (1024): `max_columns` sets the cap; without it the cap is `MAX_COLUMNS`.
    #[test]
    fn max_columns_sets_the_cap() {
        let capped: Element<'static, ()> = CardGrid::new(cards(6), roles())
            .min_width(280.0)
            .spacing(GAP)
            .max_columns(3)
            .into();
        let node = laid_out(capped, Size::new(2400.0, 800.0));
        assert_eq!(row_counts(&card_boxes(&node))[0], 3);
        let default: Element<'static, ()> = CardGrid::new(cards(12), roles()).into();
        let node = laid_out(default, Size::new(2400.0, 800.0));
        assert_eq!(row_counts(&card_boxes(&node))[0], MAX_COLUMNS);
    }

    /// U5 (1024): at 2400 px every card is at least half again as wide as feature 813's eight
    /// columns made it, and every card keeps the one card height.
    #[test]
    fn cards_at_2400_are_half_again_as_wide_as_before_and_as_tall() {
        let before = row_width(2400.0, 8.0);
        let node = laid_out(grid(12), Size::new(2400.0, 800.0));
        let boxes = card_boxes(&node);
        assert_eq!(boxes.len(), 12);
        for b in &boxes {
            assert!(b.width >= 1.5 * before, "{} < 1.5 × {before}", b.width);
            assert!((b.height - super::super::overview_card::HEIGHT).abs() < EPS);
        }
    }

    // ---- feature 970: the cards zoom with the sidebar ----

    const H: f32 = super::super::overview_card::HEIGHT;
    const PAIRS: [(f32, f32); 3] = [(1150.0, 1450.0), (1300.0, 1650.0), (1464.0, 1770.0)];

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    fn rest(width: f32) -> f32 {
        rest_width(width, MIN_WIDTH, GAP, MAX_COLUMNS)
    }

    /// The pieces a zoom from `start` to `end` shows at fraction `f` (the window `lerp(start, end, f)`
    /// wide, the cards `lerp` of the two rest widths).
    fn ribbon_at(count: usize, start: f32, end: f32, f: f32) -> Vec<Piece> {
        ribbon(
            count,
            lerp(rest(start), rest(end), f),
            GAP,
            lerp(start, end, f),
            H,
        )
    }

    /// The three width pairs, entering and leaving.
    fn both_ways() -> Vec<(f32, f32)> {
        PAIRS.iter().flat_map(|&(a, b)| [(a, b), (b, a)]).collect()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn same(a: &Piece, b: &Piece) -> bool {
        a.card == b.card
            && near(a.rect.x, b.rect.x)
            && near(a.rect.y, b.rect.y)
            && near(a.rect.width, b.rect.width)
            && near(a.origin.x, b.origin.x)
            && near(a.origin.y, b.origin.y)
    }

    fn assert_same(got: &[Piece], want: &[Piece]) {
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!(same(g, w), "{g:?} vs {w:?}");
        }
    }

    /// U18 (970): `zoom_fraction` is how far `w` lies from `start` toward `end`, clamped.
    #[test]
    fn zoom_fraction_is_the_clamped_position_between_the_ends() {
        for (start, end, w, want) in [
            (1150.0, 1450.0, 1300.0, 0.5),
            (1150.0, 1450.0, 1000.0, 0.0),
            (1150.0, 1450.0, 1600.0, 1.0),
            (1450.0, 1150.0, 1300.0, 0.5),
            (1450.0, 1150.0, 1500.0, 0.0),
            (1450.0, 1150.0, 1000.0, 1.0),
            (1150.0, 1450.0, 1150.0, 0.0),
            (1150.0, 1450.0, 1450.0, 1.0),
        ] {
            let got = zoom_fraction(start, end, w);
            assert!(near(got, want), "{start} to {end} at {w}: {got} != {want}");
        }
    }

    /// U19 (970, FR-004): at either end of a zoom the ribbon is feature 1024's rest grid.
    #[test]
    fn the_ribbon_at_either_end_equals_the_rest_grid() {
        for (start, end) in both_ways() {
            for count in [1, 7, 30] {
                for (f, width) in [(0.0, start), (1.0, end)] {
                    let got = ribbon_at(count, start, end, f);
                    let n = columns(width, MIN_WIDTH, GAP, MAX_COLUMNS);
                    let w = rest(width);
                    assert_eq!(got.len(), count, "one piece per card at {start} to {end}");
                    for (i, piece) in got.iter().enumerate() {
                        assert_eq!(piece.card, i);
                        assert!(near(piece.rect.x, (i % n) as f32 * (w + GAP)), "{piece:?}");
                        assert!(near(piece.rect.y, (i / n) as f32 * (H + GAP)), "{piece:?}");
                        assert!(near(piece.rect.width, w), "{piece:?} (w {w})");
                        assert!(near(piece.rect.height, H));
                    }
                }
            }
        }
    }

    /// U20 (970, FR-006, SC-002): on every sampled frame of every zoom no two cards overlap, every
    /// card is drawn (one piece, or two across a row end), every piece lies inside the grid and the
    /// card width stays between its two end values.
    #[test]
    fn ribbon_pieces_never_overlap_and_stay_in_range() {
        let mut two_piece_cards = 0;
        for (start, end) in both_ways() {
            for count in [1, 7, 30] {
                for step in 0..=20 {
                    let f = step as f32 * 0.05;
                    let width = lerp(start, end, f);
                    let w = lerp(rest(start), rest(end), f);
                    assert!(w >= rest(start).min(rest(end)) - EPS);
                    assert!(w <= rest(start).max(rest(end)) + EPS);
                    let pieces = ribbon_at(count, start, end, f);
                    assert!(!pieces.is_empty(), "the grid is never empty");
                    for card in 0..count {
                        let own: Vec<&Piece> = pieces.iter().filter(|p| p.card == card).collect();
                        assert!((1..=2).contains(&own.len()), "card {card}: {own:?}");
                        // A card that runs through the gap at a row end loses at most the gap.
                        let total: f32 = own.iter().map(|p| p.rect.width).sum();
                        assert!(total <= w + EPS && total >= w - GAP - EPS, "{total} vs {w}");
                        if let [a, b] = own[..] {
                            two_piece_cards += 1;
                            assert!(near(b.rect.y - a.rect.y, H + GAP), "{a:?} {b:?}");
                        }
                    }
                    for (i, a) in pieces.iter().enumerate() {
                        assert!(a.rect.x >= -EPS && a.rect.x + a.rect.width <= width + EPS);
                        for b in &pieces[i + 1..] {
                            assert!(
                                a.card == b.card
                                    || !crate::ui::material::test_support::overlap(
                                        &a.rect, &b.rect
                                    ),
                                "{start} to {end} f {f}: {a:?} overlaps {b:?}"
                            );
                        }
                    }
                }
            }
        }
        assert!(
            two_piece_cards > 0,
            "some card ran through a row end, so the two-piece check ran"
        );
    }

    /// A tree kept across layouts, as the running application keeps it: the zoom is state that
    /// lives from one layout to the next.
    struct Rig {
        tree: Option<iced::advanced::widget::Tree>,
    }

    impl Rig {
        fn new() -> Self {
            Self { tree: None }
        }

        /// Lays `cards` out `width` wide with `zoom`, as the next frame of the same widget.
        fn lay(
            &mut self,
            cards: Vec<OverviewCard<'static, ()>>,
            zoom: Zoom,
            width: f32,
        ) -> layout::Node {
            let mut element: Element<'static, ()> = Element::new(
                CardGrid::new(cards, roles())
                    .min_width(MIN_WIDTH)
                    .spacing(GAP)
                    .zoom(zoom)
                    .into_grid(),
            );
            self.lay_element(&mut element, width)
        }

        /// Diffs `element` into the kept tree and lays it out `width` wide.
        fn lay_element<M>(&mut self, element: &mut Element<'_, M>, width: f32) -> layout::Node {
            let tree = match &mut self.tree {
                Some(tree) => {
                    tree.diff(element.as_widget());
                    tree
                }
                None => self
                    .tree
                    .insert(iced::advanced::widget::Tree::new(element.as_widget())),
            };
            element.as_widget_mut().layout(
                tree,
                &crate::ui::material::test_support::renderer(),
                &layout::Limits::new(Size::ZERO, Size::new(width, 10_000.0)),
            )
        }

        /// The next frame of a grid whose `on_reflow` publishes the offset's y: lays it out
        /// `width` wide, then lets it see an event in a viewport `VIEW` tall scrolled to `scroll`.
        /// Returns what it published.
        fn frame(&mut self, zoom: Zoom, width: f32, scroll: f32) -> Vec<f32> {
            let mut element: Element<'static, f32> = Element::new(
                CardGrid::new(cards::<f32>(30), roles())
                    .min_width(MIN_WIDTH)
                    .spacing(GAP)
                    .zoom(zoom)
                    .on_reflow(|offset| offset.y)
                    .into_grid(),
            );
            let node = self.lay_element(&mut element, width);
            let mut published = Vec::new();
            let mut shell = iced::advanced::Shell::new(&mut published);
            element.as_widget_mut().update(
                self.tree.as_mut().expect("laid out"),
                &Event::Window(iced::window::Event::RedrawRequested(
                    std::time::Instant::now(),
                )),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &crate::ui::material::test_support::renderer(),
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::new(Point::new(0.0, scroll), Size::new(width, VIEW)),
            );
            published
        }

        fn state(&mut self) -> &mut ZoomState {
            self.tree
                .as_mut()
                .expect("laid out")
                .state
                .downcast_mut::<ZoomState>()
        }

        fn pieces(&mut self) -> Vec<Piece> {
            self.state().pieces.clone()
        }
    }

    fn full(freed: f32) -> Zoom {
        Zoom {
            full_screen: true,
            freed,
        }
    }

    fn normal(freed: f32) -> Zoom {
        Zoom {
            full_screen: false,
            freed,
        }
    }

    /// A rig that has laid 7 cards out normally at 1150 px and then flipped to full screen at
    /// 1300 px: a zoom from 1150 to 1450 at its midpoint.
    fn midway() -> Rig {
        let mut rig = Rig::new();
        rig.lay(cards(7), normal(300.0), 1150.0);
        rig.lay(cards(7), full(300.0), 1300.0);
        rig
    }

    /// The viewport height of the anchor tests.
    const VIEW: f32 = 400.0;

    /// Where card `i`'s top is in the rig's last layout.
    fn top_of(rig: &mut Rig, i: usize) -> f32 {
        card_top(&rig.pieces(), i).expect("the card is drawn")
    }

    /// U27 (970, FR-013, US2 #5): across a zoom the first card in view stays where it was in the
    /// view, because each layout step publishes the scroll that undoes the move; a scroll in
    /// mid-zoom hands the anchor to the new first card; a grid at rest publishes nothing.
    #[test]
    fn the_first_visible_card_stays_in_view_across_a_zoom() {
        let row = CARD_HEIGHT + GAP;
        let mut rig = Rig::new();
        rig.frame(normal(350.0), 1300.0, 0.0);
        let mut scroll = 5.0 * row;
        assert!(
            rig.frame(normal(350.0), 1300.0, scroll).is_empty(),
            "a grid at rest publishes nothing"
        );
        assert!(
            rig.frame(normal(350.0), 1280.0, scroll).is_empty(),
            "a resize at rest publishes nothing"
        );
        rig.frame(normal(350.0), 1300.0, scroll);
        // Card 20 is first in view: row 5 of 4 columns. At 5 columns it is row 4.
        let mut watched = 20;
        let mut rel = top_of(&mut rig, 20) - scroll;
        assert_eq!(rel, 0.0, "card 20 starts the view");

        let widths = [
            1300.0, 1350.0, 1400.0, 1450.0, 1500.0, 1550.0, 1600.0, 1650.0,
        ];
        let mut asked = false;
        for (step, width) in widths.into_iter().enumerate() {
            if let Some(&y) = rig.frame(full(350.0), width, scroll).last() {
                scroll = y;
                asked = true;
            }
            let now = top_of(&mut rig, watched) - scroll;
            assert!(
                (now - rel).abs() <= 1.0,
                "step {step} ({width} px): card {watched} sits {now} from the view's top, began at {rel}"
            );
            if step == 3 {
                // The user scrolls two rows up in mid-zoom: the anchor becomes the first card in
                // the new view, and it is that one the next steps keep in place.
                scroll -= 2.0 * row;
                rig.frame(full(350.0), width, scroll);
                (watched, _) = first_visible(&rig.pieces(), scroll, scroll + VIEW).expect("a card");
                rel = top_of(&mut rig, watched) - scroll;
                assert_ne!(watched, 20, "a scroll mid-zoom changed the anchor");
            }
        }
        assert!(asked, "the zoom moved card 20, so it asked to scroll");
    }

    /// U21 (970, FR-011): a grid built full screen is at rest, with no zoom.
    #[test]
    fn a_grid_built_full_screen_lays_out_at_rest() {
        let mut rig = Rig::new();
        rig.lay(cards(7), full(300.0), 1300.0);
        assert_eq!(rig.state().ends, None);
        assert_same(&rig.pieces(), &ribbon(7, rest(1300.0), GAP, 1300.0, H));
    }

    /// The seven-card zoom from 1150 to 1450 at f = 0.5, worked by hand and written out so the
    /// expectation does not call `ribbon`.
    ///
    /// Rest widths: 1150 holds `floor(1166 / 296)` = 3 columns, so a card is (1150 - 32) / 3 =
    /// 372.667; 1450 holds `floor(1466 / 296)` = 4, so (1450 - 48) / 4 = 350.5. Half way the window
    /// is 1300 wide (a row plus its gap is 1316) and a card 361.583 wide, one pitch (card plus
    /// gap) being 377.583. Card `i` starts `i` pitches along the ribbon: 0, 377.583, 755.167,
    /// 1132.75, 1510.333, 1887.917, 2265.5. Cards 0-2 sit on row 0. Card 3 starts at 1132.75 and
    /// runs 1494.333, past the 1300 edge: 167.25 stay on row 0 and 178.333 (to 1494.333 - 1316)
    /// continue at x 0 of row 1, which starts 172 + 16 = 188 down. Cards 4-6 start on row 1 at
    /// x = start - 1316; card 6 is cut at 1300 (350.5 wide).
    fn seven_cards_at_half_way() -> Vec<Piece> {
        // (card, x, y, width, origin x), every piece 172 tall.
        let table: [(usize, f32, f32, f32, f32); 8] = [
            (0, 0.0, 0.0, 361.583, 0.0),
            (1, 377.583, 0.0, 361.583, 377.583),
            (2, 755.167, 0.0, 361.583, 755.167),
            (3, 1132.75, 0.0, 167.25, 1132.75),
            (3, 0.0, 188.0, 178.333, -183.25),
            (4, 194.333, 188.0, 361.583, 194.333),
            (5, 571.917, 188.0, 361.583, 571.917),
            (6, 949.5, 188.0, 350.5, 949.5),
        ];
        table
            .iter()
            .map(|&(card, x, y, width, origin_x)| Piece {
                card,
                rect: Rectangle::new(Point::new(x, y), Size::new(width, H)),
                origin: Point::new(origin_x, y),
                node: 0,
            })
            .collect()
    }

    /// U22 (970, FR-004, FR-006): a flip starts a zoom whose ends are the width before the flip and
    /// that width plus the freed width; the frame between them is the ribbon at that fraction.
    #[test]
    fn a_flip_starts_a_zoom_between_the_two_end_widths() {
        let mut rig = midway();
        assert_eq!(rig.state().ends, Some((1150.0, 1450.0)));
        assert_same(&rig.pieces(), &seven_cards_at_half_way());
        rig.lay(cards(7), full(300.0), 1450.0);
        assert_eq!(rig.state().ends, None, "the zoom ends at its end width");
        assert_same(&rig.pieces(), &ribbon(7, rest(1450.0), GAP, 1450.0, H));
    }

    /// U23 (970): a zoom ends at an end width, when the width leaves its range, and after the slide.
    #[test]
    fn a_zoom_ends_at_an_end_outside_the_range_or_after_the_slide() {
        let mut rig = midway();
        rig.lay(cards(7), full(300.0), 1449.8);
        assert_eq!(rig.state().ends, None, "within 0.5 px of the end");
        assert_same(&rig.pieces(), &ribbon(7, rest(1449.8), GAP, 1449.8, H));

        let mut rig = midway();
        rig.lay(cards(7), full(300.0), 1700.0);
        assert_eq!(rig.state().ends, None, "a resize past the end");
        let mut rig = midway();
        rig.lay(cards(7), full(300.0), 1000.0);
        assert_eq!(rig.state().ends, None, "a resize past the start");

        let mut rig = midway();
        rig.state().clock.restart_at(1.0);
        rig.lay(cards(7), full(300.0), 1300.0);
        assert_eq!(rig.state().ends, None, "the slide and 100 ms are over");
        assert_same(&rig.pieces(), &ribbon(7, rest(1300.0), GAP, 1300.0, H));
    }

    /// Feature 970, Story 3 #1 (FR-008): when the drawer snaps (*Reduce motion*), a flip whose
    /// first layout is already at the end width runs no zoom; the grid is the rest grid.
    #[test]
    fn a_flip_laid_out_at_the_end_width_runs_no_zoom() {
        let mut rig = Rig::new();
        rig.lay(cards(7), normal(300.0), 1150.0);
        rig.lay(cards(7), full(300.0), 1450.0);
        assert_eq!(rig.state().ends, None, "no zoom runs");
        assert_same(&rig.pieces(), &ribbon(7, rest(1450.0), GAP, 1450.0, H));
    }

    /// Story 3 #2: and back, leaving full screen onto the start width.
    #[test]
    fn leaving_full_screen_onto_the_start_width_runs_no_zoom() {
        let mut rig = Rig::new();
        rig.lay(cards(7), normal(300.0), 1150.0);
        rig.lay(cards(7), full(300.0), 1450.0);
        rig.lay(cards(7), normal(300.0), 1150.0);
        assert_eq!(rig.state().ends, None);
        assert_same(&rig.pieces(), &ribbon(7, rest(1150.0), GAP, 1150.0, H));
    }

    /// Story 3 #3: a zoom at f = 0.4 whose next layout comes at the end width (the drawer
    /// snapped) ends there.
    #[test]
    fn a_snapped_width_mid_zoom_ends_the_zoom_on_the_next_layout() {
        let mut rig = Rig::new();
        rig.lay(cards(7), normal(300.0), 1150.0);
        rig.lay(cards(7), full(300.0), 1270.0);
        assert_eq!(rig.state().ends, Some((1150.0, 1450.0)));
        assert_same(&rig.pieces(), &ribbon_at(7, 1150.0, 1450.0, 0.4));
        rig.lay(cards(7), full(300.0), 1450.0);
        assert_eq!(rig.state().ends, None, "the snapped layout ends the zoom");
        assert_same(&rig.pieces(), &ribbon(7, rest(1450.0), GAP, 1450.0, H));
    }

    /// U24 (970, FR-007): flipping back midway keeps the ends, and the frame does not move.
    #[test]
    fn a_reversal_midway_keeps_the_ends() {
        let mut rig = midway();
        let before = rig.pieces();
        rig.lay(cards(7), normal(300.0), 1300.0);
        assert_eq!(rig.state().ends, Some((1150.0, 1450.0)));
        assert_same(&rig.pieces(), &before);
        rig.lay(cards(7), normal(300.0), 1150.0);
        assert_eq!(rig.state().ends, None, "back at its start");
        assert_same(&rig.pieces(), &ribbon(7, rest(1150.0), GAP, 1150.0, H));
    }

    /// U25 (970, US2 #5): a change of cards mid-zoom neither restarts nor ends the zoom.
    #[test]
    fn a_card_count_change_mid_zoom_keeps_the_zoom() {
        let mut rig = midway();
        let clock = rig.state().clock.value();
        let changed: Vec<OverviewCard<'static, ()>> = cards(9);
        rig.lay(changed, full(300.0), 1320.0);
        assert_eq!(rig.state().ends, Some((1150.0, 1450.0)));
        assert_eq!(rig.state().clock.value(), clock);
        let pieces = rig.pieces();
        assert_same(&pieces, &ribbon_at(9, 1150.0, 1450.0, 170.0 / 300.0));
        for (i, a) in pieces.iter().enumerate() {
            for b in &pieces[i + 1..] {
                assert!(
                    a.card == b.card
                        || !crate::ui::material::test_support::overlap(&a.rect, &b.rect)
                );
            }
        }
    }

    /// U26 (970, SC-001): at rest in full screen a full row spans the whole freed width.
    #[test]
    fn full_screen_rows_span_the_freed_width() {
        let freed = f32::from(crate::app::SIDEBAR_DEFAULT_WIDTH)
            + crate::ui::material::resize_handle::WIDTH;
        let span = |width: f32, zoom: Zoom| {
            let mut rig = Rig::new();
            rig.lay(cards(30), zoom, width);
            let row: Vec<Piece> = rig
                .pieces()
                .into_iter()
                .filter(|p| p.rect.y < 1.0)
                .collect();
            row.iter()
                .map(|p| p.rect.x + p.rect.width)
                .fold(0.0, f32::max)
        };
        let window = 1400.0 - 48.0;
        let normal_row = span(window - freed, normal(freed));
        let full_row = span(window, full(freed));
        assert!(near(full_row, window), "{full_row} != {window}");
        assert!(
            full_row >= normal_row + freed - EPS,
            "{full_row} vs {normal_row}"
        );
    }

    /// 970: the pointer maps onto the card's primary box when the grid is not at (0, 0) and the
    /// card is split at a row end; outside every piece it is unavailable.
    #[test]
    fn cursor_maps_into_the_primary_box_with_a_grid_origin() {
        let mut state = ZoomState::new(true);
        state.ends = Some((100.0, 200.0));
        let piece = |x: f32, y: f32, ox: f32, node: usize| Piece {
            card: 3,
            rect: Rectangle::new(Point::new(x, y), Size::new(50.0, 40.0)),
            origin: Point::new(ox, y),
            node,
        };
        // Primary piece at (10, 0) in the grid; the tail is on the next row, card begins at -30.
        state.pieces = vec![piece(10.0, 0.0, 10.0, 0), piece(0.0, 60.0, -30.0, 0)];
        let grid_origin = Point::new(24.0, 80.0);
        let node_at = Point::new(grid_origin.x + 10.0, grid_origin.y);
        let at = |p: Point| {
            Grid::<()>::cursor_for_state(
                &state,
                grid_origin,
                3,
                node_at,
                mouse::Cursor::Available(p),
            )
        };
        let expect = |c: mouse::Cursor, x: f32, y: f32| {
            let p = c.position().expect("available");
            assert!(near(p.x, x) && near(p.y, y), "{p:?} != ({x}, {y})");
        };
        // Over the primary piece, 5 right and 7 down of its corner.
        expect(
            at(Point::new(24.0 + 15.0, 80.0 + 7.0)),
            node_at.x + 5.0,
            node_at.y + 7.0,
        );
        // Over the tail, 5 right of its corner: card x = 5 - (-30) = 35 into the card.
        expect(
            at(Point::new(24.0 + 5.0, 80.0 + 65.0)),
            node_at.x + 35.0,
            node_at.y + 5.0,
        );
        assert!(at(Point::new(24.0 + 200.0, 80.0)).position().is_none());
    }
}

//! `CardGrid`: the Overview's responsive grid of [`OverviewCard`]s (feature 813, FR-021).
//!
//! The number of columns follows the width the grid is given (iced `responsive`): as many as fit at
//! the minimum card width, never fewer than one and at most [`MAX_COLUMNS`] (five, feature 1024).
//! The cards share the width equally, so a wider grid makes wider cards, and the grid scrolls
//! vertically.

use std::time::{Duration, Instant};

use crate::ui::cdk::motion::{self, Progress};
use crate::ui::material::card_lift::Lifts;
use crate::ui::material::navigation_drawer::SLIDE;
use crate::ui::material::overview_card::RestMessages;
use crate::ui::material::{style, OverviewCard, Scrollable};
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Id, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::widget::scrollable::AbsoluteOffset;
use iced::{window, Element, Event, Length, Point, Rectangle, Size, Vector};
use micold_core::overview_expansion::EXPAND_REST;
use micold_core::session::SessionLocation;
use micold_core::tokens::motion::duration;
use micold_core::tokens::{elevation, spacing, Roles};
use micold_core::tooltip::ShowTimer;

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
    unscrolled: bool,
    expanded: Option<(usize, OverviewCard<'a, M>)>,
    on_window_blur: Option<M>,
    rest_scope: u64,
    instant: bool,
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
            unscrolled: false,
            expanded: None,
            on_window_blur: None,
            rest_scope: 0,
            instant: false,
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

    /// Publish `message` when the window loses focus, so the owner can collapse an expanded card
    /// through its own message (feature 1121, FR-005, contract U4).
    pub fn on_window_blur(mut self, message: M) -> Self {
        self.on_window_blur = Some(message);
        self
    }

    /// Name the set the cards belong to (the project): another scope drops the pointer's rest,
    /// so a card at the same location in the new set rests anew (review A F2, M3 round 2).
    pub fn rest_scope(mut self, scope: impl std::hash::Hash) -> Self {
        use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
        self.rest_scope = BuildHasherDefault::<DefaultHasher>::default().hash_one(scope);
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
    /// The grid row the piece is on.
    row: usize,
}

/// The ribbon's pieces placed along their rows, each with its row and no height yet: the
/// horizontal half of the ribbon (feature 970).
fn ribbon_rows(count: usize, w: f32, gap: f32, width: f32) -> Vec<Piece> {
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
                pieces.push(Piece {
                    card,
                    rect: Rectangle::new(Point::new(left, 0.0), Size::new(right - left, 0.0)),
                    origin: Point::new(origin, 0.0),
                    node: 0,
                    row: row as usize,
                });
            }
            if origin + w <= line + SNAP {
                break;
            }
        }
    }
    pieces
}

/// How many rows `pieces` fill.
fn row_count(pieces: &[Piece]) -> usize {
    pieces.iter().map(|p| p.row + 1).max().unwrap_or(0)
}

/// The narrowest piece that counts toward its row's height. A zoom slides a card through a row
/// end, so for a frame a tall card can show a sub-pixel sliver in the next row; letting that sliver
/// stretch the whole row would make every row below it jump mid-zoom. Such a piece is still drawn.
const ROW_PIECE: f32 = 1.0;

/// Each row's height: the tallest `naturals[card]` among the cards with a piece at least
/// [`ROW_PIECE`] wide in it, and never under the minimum card height (feature 1120, FR-014, R12).
fn row_heights(pieces: &[Piece], naturals: &[f32]) -> Vec<f32> {
    let mut heights = vec![MIN_CARD_HEIGHT; row_count(pieces)];
    for piece in pieces.iter().filter(|p| p.rect.width >= ROW_PIECE) {
        heights[piece.row] = heights[piece.row].max(naturals[piece.card]);
    }
    heights
}

/// Stacks the rows top to bottom, row `r` being `heights[r]` tall and a `gap` below the one
/// before: the vertical half of the ribbon (feature 1120, FR-014).
fn stack_rows(pieces: &mut [Piece], heights: &[f32], gap: f32) {
    let tops: Vec<f32> = heights
        .iter()
        .scan(0.0, |top, h| {
            let this = *top;
            *top += h + gap;
            Some(this)
        })
        .collect();
    for piece in pieces {
        piece.rect.y = tops[piece.row];
        piece.rect.height = heights[piece.row];
        piece.origin.y = tops[piece.row];
    }
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
    /// The view the grid was last drawn or updated in, in its own coordinates: an expanded
    /// card is capped at its height and placed inside it (feature 1121, R6).
    view: std::cell::Cell<Option<Rectangle>>,
    /// The expanded card's node at its slot, from the last layout (feature 1121, R4).
    expanded: Option<layout::Node>,
    /// The view height the last layout capped the expanded card at, when the room left it
    /// unbounded (in a scrollable): `update` lays out again when the view's height is another
    /// (review A F1, M2). `INFINITY` when that layout had no view yet.
    capped_at: Option<f32>,
    /// The pointer's rest on the topmost card under it (feature 1121, R5, U3).
    rest: ShowTimer,
    /// That card's location, and whether its rest was published: the card, not its place in
    /// the list, so a re-sort under the pointer leaves it (review A F1, M3).
    resting: Option<(SessionLocation, bool)>,
    /// Whether the pointer is on that card's link: a change starts the wait again (feature 1122,
    /// R8).
    on_link: bool,
    /// Whether the rest on the current part (link or the rest of the card) was published.
    part_rested: bool,
    /// The rest scope `resting` was seen in: another drops it (review A F2, M3 round 2).
    rest_scope: u64,
    /// How far each lifted or settling card's shadow has eased in (feature 1299, D3).
    lifts: Lifts,
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
            view: std::cell::Cell::new(None),
            expanded: None,
            capped_at: None,
            rest: ShowTimer::default(),
            resting: None,
            on_link: false,
            part_rested: false,
            rest_scope: 0,
            lifts: Lifts::default(),
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

    /// Draw the grid as tall as its rows, with no scrollable of its own: for a grid in a page
    /// that already scrolls (the showcase's poses). The scrollable it otherwise gets fills its
    /// room's height, and a scrolling page offers none, so there the grid collapses to nothing.
    pub fn unscrolled(mut self) -> Self {
        self.unscrolled = true;
        self
    }

    /// Draw card `index` expanded (feature 1121, research R4, contract U2): `cards[index]` is the
    /// expanded card and `twin` the same card collapsed, built with no id, focus wiring or
    /// messages; the grid gives it the expanded card's issue link, which takes a line of its own
    /// when the card has no branch line (feature 1122). The grid measures the twin for that
    /// card's row, so every row and slot is what it is with nothing expanded (FR-009); the
    /// expanded card takes its slot's x, width and top, as tall as its content up to the view's
    /// height, and is drawn over its neighbours, moved up just enough to lie inside the view
    /// (`overview_expansion::place`, FR-008).
    pub fn expanded(mut self, index: usize, twin: OverviewCard<'a, M>) -> Self {
        self.expanded = Some((index, twin));
        self
    }

    /// *Reduce motion* (feature 1299, FR-007): every card's focus, and the expanded card's
    /// shadow, appear and go at once. Defaults to `false`.
    pub fn instant(mut self, instant: bool) -> Self {
        self.instant = instant;
        self
    }

    /// The grid proper, without its scrollable.
    fn into_grid(self) -> Grid<'a, M> {
        let instant = self.instant;
        let (expanded, twin) = match self.expanded.clone() {
            Some((index, twin)) if index < self.cards.len() => (
                Some(index),
                Some(Element::from(twin.twin_of(&self.cards[index]))),
            ),
            _ => (None, None),
        };
        Grid {
            roles: self.roles,
            instant,
            expanded,
            twin,
            rests: self.cards.iter().map(OverviewCard::rest_messages).collect(),
            on_window_blur: self.on_window_blur,
            rest_scope: self.rest_scope,
            cards: self
                .cards
                .iter()
                .map(|card| {
                    // The grid's Reduce motion adds to a card's own; it never takes it away. The
                    // grid draws the expanded card's shadow, eased (feature 1299, D3).
                    let card = card.clone().shadow_by_owner();
                    Element::from(if instant { card.instant(true) } else { card })
                })
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
    /// The card drawn expanded over its neighbours (feature 1121, R4).
    expanded: Option<usize>,
    /// That card collapsed: measured for its row, never drawn, focused or sent events. Its tree
    /// is the one after the cards'.
    twin: Option<Element<'a, M>>,
    /// Each card's location, rest messages and link id (feature 1121, R5; feature 1122, R8).
    rests: Vec<RestMessages<M>>,
    /// Published when the window loses focus (feature 1121, FR-005).
    on_window_blur: Option<M>,
    /// The cards' set, hashed: [`CardGrid::rest_scope`].
    rest_scope: u64,
    /// The roles the expanded card's shadow is drawn in (feature 1299, D3).
    roles: Roles,
    /// *Reduce motion*: the shadow appears and goes at once ([`CardGrid::instant`]).
    instant: bool,
}

/// The region a card's shadow at `strength` can paint: the card's `bounds` grown by the blur and
/// the offset. It differs per strength, so each frame of an ease damages the last frame's reach
/// and the next one's (issue 1336).
fn lift_damage(roles: Roles, bounds: Rectangle, strength: f32) -> Rectangle {
    let shadow = style::eased_shadow(roles, elevation::MENU, strength);
    bounds.expand(shadow.blur_radius + shadow.offset.x.abs().max(shadow.offset.y.abs()))
}

/// The expanded card's layout where it is drawn: its slot's node moved up by `place` (K3) to lie
/// inside `view`; a slot below the view stays where it is.
fn placed<'n>(node: &'n layout::Node, origin: Point, view: Rectangle) -> Layout<'n> {
    let slot = node.bounds() + Vector::new(origin.x, origin.y);
    let dy = if slot.y < view.y + view.height {
        micold_core::overview_expansion::place(
            (slot.y, slot.height),
            slot.height,
            (view.y, view.height),
        )
        .top - slot.y
    } else {
        0.0
    };
    Layout::with_offset(Vector::new(origin.x, origin.y + dy), node)
}

/// The view the grid was last updated or drawn in, in its own coordinates; its bounds before any.
fn last_view(state: &ZoomState, layout: Layout<'_>) -> Rectangle {
    state.view.get().unwrap_or(layout.bounds())
}

impl<M: Clone> Grid<'_, M> {
    /// Feature 1121 R5, U3: time the pointer's rest on the topmost card under it (`pointer`: its
    /// index, the expanded card first, and whether the pointer is on its link), publishing that
    /// card's `on_rest`, or its `on_link_rest` on the link (feature 1122, R8), once it has rested
    /// `EXPAND_REST`, and its `on_leave` when the pointer leaves it after. A change of the card
    /// under the pointer, or between its link and the rest of it, starts the wait again, even with
    /// no frame outside every card. Nothing happens while a zoom runs.
    fn observe_rest(
        &self,
        state: &mut ZoomState,
        event: &Event,
        pointer: Option<(usize, bool)>,
        shell: &mut Shell<'_, M>,
    ) {
        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) => *now,
            Event::Mouse(_) => Instant::now(),
            _ => return,
        };
        if state.ends.is_some() {
            return;
        }
        let topmost = pointer.and_then(|(card, _)| self.rests.get(card).map(|r| &r.at));
        let on_link = pointer.is_some_and(|(_, on_link)| on_link);
        let messages = |at: &SessionLocation| self.rests.iter().find(|r| &r.at == at);
        if topmost != state.resting.as_ref().map(|(card, _)| card) {
            if let Some((left, true)) = &state.resting {
                if let Some(on_leave) = messages(left).and_then(|r| r.on_leave.as_ref()) {
                    shell.publish(on_leave.clone());
                }
            }
            state.rest.reset();
            state.resting = topmost.map(|card| (card.clone(), false));
            state.on_link = on_link;
            state.part_rested = false;
        } else if topmost.is_some() && on_link != state.on_link {
            // Between the card and its link: the same card, a new wait and no leave.
            state.rest.reset();
            state.on_link = on_link;
            state.part_rested = false;
        }
        let rest = state.rest.observe(topmost.is_some(), now, EXPAND_REST);
        if let Some((card, rested)) = state.resting.as_mut() {
            if rest.open && !state.part_rested {
                state.part_rested = true;
                *rested = true;
                let message = messages(card).and_then(|r| {
                    if state.on_link {
                        r.on_link_rest.as_ref()
                    } else {
                        r.on_rest.as_ref()
                    }
                });
                if let Some(message) = message {
                    shell.publish(message.clone());
                }
            }
        }
        if let Some(wake) = rest.wake_at {
            motion::wake_at(shell, wake);
        }
    }
}

impl<M> Grid<'_, M> {
    /// The expanded card's location.
    fn expanded_at(&self) -> Option<&SessionLocation> {
        self.expanded
            .and_then(|index| self.rests.get(index))
            .map(|r| &r.at)
    }

    /// Card `index`'s shadow at `strength`, under a card drawn at `bounds` (feature 1299, D3):
    /// a transparent quad of the card's radius, so the opaque card face drawn next covers its
    /// inside.
    fn draw_lift(&self, renderer: &mut iced::Renderer, bounds: Rectangle, strength: f32) {
        use iced::advanced::Renderer as _;
        if strength <= 0.0 {
            return;
        }
        // The software renderer damages a quad's bounds only, not its shadow, so a shadow that
        // eases away would leave pixels outside the repainted region: an invisible quad over the
        // shadow's reach, changing with every frame, damages it (issue 1336).
        renderer.fill_quad(
            renderer::Quad {
                bounds: lift_damage(self.roles, bounds, strength),
                ..renderer::Quad::default()
            },
            iced::Color::TRANSPARENT,
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: iced::Border {
                    radius: super::overview_card::RADIUS.into(),
                    ..iced::Border::default()
                },
                shadow: style::eased_shadow(self.roles, elevation::MENU, strength),
                ..renderer::Quad::default()
            },
            iced::Color::TRANSPARENT,
        );
    }

    /// Whether `cursor` is on one of card `i`'s links (its issue link, a pull request number),
    /// laid out in `layout`: each link's bounds found by its id over the card's subtree, on screen
    /// and clipped to the expanded body's view (`ui::focus::ScreenBounds`, research R7; feature
    /// 1123, R6).
    fn over_link(
        &mut self,
        i: usize,
        card_tree: &mut Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> bool {
        let ids = match self.rests.get(i) {
            Some(r) if !r.link_ids.is_empty() => &r.link_ids,
            _ => return false,
        };
        let mut find = crate::ui::focus::ScreenBounds::of_any(ids);
        self.cards[i]
            .as_widget_mut()
            .operate(card_tree, layout, renderer, &mut find);
        find.all_visible()
            .iter()
            .any(|bounds| cursor.is_over(*bounds))
    }

    /// The expanded card's index and drawn layout, in `view`.
    fn expanded_layout<'n>(
        &self,
        state: &'n ZoomState,
        origin: Point,
        view: Rectangle,
    ) -> Option<(usize, Layout<'n>)> {
        let index = self.expanded?;
        Some((index, placed(state.expanded.as_ref()?, origin, view)))
    }

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

impl<M: Clone> Widget<M, iced::Theme, iced::Renderer> for Grid<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ZoomState>()
    }

    fn state(&self) -> tree::State {
        let mut state = ZoomState::new(self.zoom.full_screen);
        // A card expanded when the grid appears is lifted already (feature 1299, R8).
        state.lifts = Lifts::seeded(self.expanded_at());
        tree::State::new(state)
    }

    fn children(&self) -> Vec<Tree> {
        self.cards
            .iter()
            .chain(&self.twin)
            .map(|card| Tree::new(card.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        // The twin's tree follows the cards': set it aside while the cards are diffed.
        let twin_tree = (tree.children.len() > self.cards.len())
            .then(|| tree.children.pop())
            .flatten();
        tree.diff_children_custom(
            &self.cards,
            |child_tree, card| child_tree.diff(card.as_widget()),
            |card| Tree::new(card.as_widget()),
        );
        if let Some(twin) = &self.twin {
            let mut twin_tree = twin_tree.unwrap_or_else(|| Tree::new(twin.as_widget()));
            twin_tree.diff(twin.as_widget());
            tree.children.push(twin_tree);
        }
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
        // The expanded card is capped at the view's height: the room's when it is bounded, else
        // (in a scrollable) the last view's (feature 1121, R6).
        let bounded = limits.max().height.is_finite();
        let view_height = if bounded {
            limits.max().height
        } else {
            state.view.get().map_or(f32::INFINITY, |v| v.height)
        };
        let card_width = match state.advance(self.zoom, width) {
            Some((start, end)) => {
                let f = zoom_fraction(start, end, width);
                let at_start = rest_width(start, min, gap, max);
                let at_end = rest_width(end, min, gap, max);
                at_start + (at_end - at_start) * f
            }
            None => rest_width(width, min, gap, max),
        };
        let mut pieces = ribbon_rows(self.cards.len(), card_width, gap, width);

        // Measure-then-stretch (feature 1120, R12): each card at its width and at least the
        // minimum height; a row is as tall as the tallest card with a piece in it, and a card is
        // laid out at the tallest of its rows (a split card's piece in a shorter row is clipped).
        let measure = layout::Limits::new(
            Size::new(card_width, MIN_CARD_HEIGHT),
            Size::new(card_width, f32::INFINITY),
        );
        let mut nodes: Vec<layout::Node> = self
            .cards
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(card, card_tree)| card.as_widget_mut().layout(card_tree, renderer, &measure))
            .collect();
        let mut naturals: Vec<f32> = nodes.iter().map(|n| n.size().height).collect();
        // The expanded card's row is as tall as its collapsed twin makes it (FR-009).
        if let (Some(index), Some(twin), Some(twin_tree)) = (
            self.expanded,
            self.twin.as_mut(),
            tree.children.get_mut(self.cards.len()),
        ) {
            naturals[index] = twin
                .as_widget_mut()
                .layout(twin_tree, renderer, &measure)
                .size()
                .height;
        }
        let heights = row_heights(&pieces, &naturals);
        stack_rows(&mut pieces, &heights, gap);
        let mut card_heights = vec![MIN_CARD_HEIGHT; self.cards.len()];
        for piece in &pieces {
            card_heights[piece.card] = card_heights[piece.card].max(piece.rect.height);
        }
        if let Some(index) = self.expanded {
            // As tall as its content, never shorter than its stretched slot (FR-008, FR-009).
            card_heights[index] = nodes[index]
                .size()
                .height
                .max(card_heights[index])
                .min(view_height);
        }

        // One node per card, at its largest piece; the other piece of a split card follows.
        let mut primary: Vec<Option<usize>> = vec![None; self.cards.len()];
        for (index, piece) in pieces.iter().enumerate() {
            let best = primary[piece.card].map_or(0.0, |i| pieces[i].rect.width);
            if piece.rect.width > best {
                primary[piece.card] = Some(index);
            }
        }
        for (i, (card, card_tree)) in self.cards.iter_mut().zip(&mut tree.children).enumerate() {
            let height = card_heights[i];
            if (nodes[i].size().height - height).abs() >= MIN_PIECE {
                let exact = layout::Limits::new(
                    Size::new(card_width, height),
                    Size::new(card_width, height),
                );
                nodes[i] = card.as_widget_mut().layout(card_tree, renderer, &exact);
            }
            let at = primary[i].map_or(Point::ORIGIN, |p| pieces[p].origin);
            nodes[i] = std::mem::take(&mut nodes[i]).move_to(at);
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

        let height = if heights.is_empty() {
            0.0
        } else {
            heights.iter().sum::<f32>() + (heights.len() - 1) as f32 * gap
        };
        let state = tree.state.downcast_mut::<ZoomState>();
        state.pieces = pieces;
        state.expanded = self.expanded.map(|index| nodes[index].clone());
        state.capped_at = (!bounded).then_some(view_height);
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
        if let (Event::Window(window::Event::Unfocused), Some(on_blur)) =
            (event, &self.on_window_blur)
        {
            shell.publish(on_blur.clone());
        }
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
        if self.expanded.is_some() {
            let state = tree.state.downcast_mut::<ZoomState>();
            state.view.set(Some(*viewport));
            // Against the height the layout used, not the view stored: `draw` stores it too.
            if state.capped_at.is_some_and(|h| h != viewport.height) {
                shell.invalidate_layout();
            }
        }
        let (state_tree, children) = (&tree.state, &mut tree.children);
        let state = state_tree.downcast_ref::<ZoomState>();
        let expanded = self.expanded_layout(state, origin, *viewport);
        let over_expanded = expanded.is_some_and(|(_, drawn)| cursor.is_over(drawn.bounds()));
        // The card under the pointer, the expanded one first, and whether the pointer is on its
        // link: what the rest timer follows, and where a press on a link goes (feature 1122).
        let pointer = if state.ends.is_none()
            && matches!(
                event,
                Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_))
            ) {
            let under = if over_expanded {
                expanded
            } else {
                cursor.position().and_then(|at| {
                    layout
                        .children()
                        .take(self.cards.len())
                        .enumerate()
                        .find(|(_, card)| card.bounds().contains(at))
                })
            };
            under.map(|(i, card)| {
                let on_link = self.over_link(i, &mut children[i], card, cursor, renderer);
                (i, on_link)
            })
        } else {
            None
        };
        // A primary press on a link is that card's alone: another card holding the keyboard keeps
        // it, as it would not on a press outside it (R7).
        let only = match (event, pointer) {
            (Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), Some((i, true))) => {
                Some(i)
            }
            _ => None,
        };
        let others = |i: usize| only.is_some_and(|card| card != i);
        // The expanded card takes events first; beneath its drawn rectangle the others see the
        // pointer levitate (FR-010).
        if let Some((index, drawn)) = expanded.filter(|(index, _)| !others(*index)) {
            self.cards[index].as_widget_mut().update(
                &mut children[index],
                event,
                drawn,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        for (i, (card, card_tree)) in self.cards.iter_mut().zip(children).enumerate() {
            if Some(i) == self.expanded || others(i) {
                continue;
            }
            let Some(card_layout) = layout.children().nth(i) else {
                continue;
            };
            let at = card_layout.position();
            let mut local = Grid::<M>::cursor_for_state(state, origin, i, at, cursor);
            if over_expanded {
                local = local.levitate();
            }
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
        let state = tree.state.downcast_mut::<ZoomState>();
        if state.rest_scope != self.rest_scope {
            // A new set of cards (another project's): the old set's rest is gone with its owner's
            // state, nothing to leave, and its lifts with it, so a card of the new set at the old
            // one's location fades no shadow it never cast (feature 1299).
            state.rest_scope = self.rest_scope;
            state.resting = None;
            state.lifts = Lifts::seeded(self.expanded_at());
        }
        self.observe_rest(state, event, pointer, shell);
        // The expanded card's shadow eases in, a collapsed one's out (feature 1299, D3); a zoom
        // draws cards in pieces, so while one runs no card is left settling over them (C3).
        let instant = self.instant || state.ends.is_some();
        let rests = &self.rests;
        state.lifts.step(
            event,
            self.expanded_at(),
            |at| rests.iter().any(|r| &r.at == at),
            instant,
            shell,
        );
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
        let expanded = self.expanded_layout(state, origin, *viewport);
        if let Some((index, drawn)) = expanded {
            let interaction = self.cards[index].as_widget().mouse_interaction(
                &tree.children[index],
                drawn,
                cursor,
                viewport,
                renderer,
            );
            // Over the expanded card the pointer is its own (FR-010).
            if cursor.is_over(drawn.bounds()) {
                return interaction;
            }
        }
        self.cards
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .map(|(i, ((card, card_tree), card_layout))| {
                if Some(i) == self.expanded {
                    let drawn = expanded.map_or(card_layout, |(_, drawn)| drawn);
                    return card
                        .as_widget()
                        .mouse_interaction(card_tree, drawn, cursor, viewport, renderer);
                }
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
        if self.expanded.is_some() {
            state.view.set(Some(*viewport));
        }
        let instant = self.instant || state.ends.is_some();
        // The cards whose shadow is fading out (feature 1299, D3): drawn whole after the others,
        // so the cards after them in the grid do not cut their shadow.
        let settling: Vec<usize> = if instant {
            Vec::new()
        } else {
            state
                .lifts
                .settling(self.expanded_at())
                .filter_map(|at| self.rests.iter().position(|r| &r.at == at))
                .collect()
        };
        for piece in &state.pieces {
            if Some(piece.card) == self.expanded || settling.contains(&piece.card) {
                continue;
            }
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
            let whole = (clip.width - bounds.width).abs() < MIN_PIECE
                && (clip.height - bounds.height).abs() < MIN_PIECE;
            // A split piece hands its clip down as the viewport too: a layer the card opens itself
            // (a `stack`'s upper child) clips to the viewport it is given, not to this layer.
            let draw_card = |renderer: &mut iced::Renderer, viewport: &Rectangle| {
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
                draw_card(renderer, viewport);
            } else if let Some(view) = clip.intersection(viewport) {
                // A layer does not inherit its parent's clip: a piece's layer is cut to the
                // viewport too, or a lower-row piece paints below the scrolled grid (1120).
                renderer.with_layer(view, |renderer| draw_card(renderer, &view));
            }
        }
        // Each settling card whole at its slot, after its shadow, over its neighbours (1299, D3).
        // A settling card off screen, shadow and all, is not drawn.
        let reach = {
            let shadow = style::eased_shadow(self.roles, elevation::MENU, 1.0);
            shadow.blur_radius + shadow.offset.x.abs().max(shadow.offset.y.abs())
        };
        for &index in &settling {
            let Some(card_layout) = layout.children().nth(index) else {
                continue;
            };
            if !card_layout.bounds().expand(reach).intersects(viewport) {
                continue;
            }
            let strength = state.lifts.strength(&self.rests[index].at, false, instant);
            renderer.with_layer(*viewport, |renderer| {
                self.draw_lift(renderer, card_layout.bounds(), strength);
                self.cards[index].as_widget().draw(
                    &tree.children[index],
                    renderer,
                    theme,
                    style,
                    card_layout,
                    cursor,
                    viewport,
                );
            });
        }
        // The expanded card last, in a layer of its own, over its neighbours (R4), after its
        // shadow (feature 1299, D3).
        if let Some((index, drawn)) = self.expanded_layout(state, origin, *viewport) {
            let strength = self
                .expanded_at()
                .map_or(1.0, |at| state.lifts.strength(at, true, instant));
            renderer.with_layer(*viewport, |renderer| {
                self.draw_lift(renderer, drawn.bounds(), strength);
                self.cards[index].as_widget().draw(
                    &tree.children[index],
                    renderer,
                    theme,
                    style,
                    drawn,
                    cursor,
                    viewport,
                );
            });
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let (state_tree, children) = (&tree.state, &mut tree.children);
        let state = state_tree.downcast_ref::<ZoomState>();
        let expanded = self.expanded_layout(state, layout.position(), last_view(state, layout));
        for (i, ((card, card_tree), card_layout)) in self
            .cards
            .iter_mut()
            .zip(children)
            .zip(layout.children())
            .enumerate()
        {
            let card_layout = match expanded {
                Some((index, drawn)) if index == i => drawn,
                _ => card_layout,
            };
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
        let (state_tree, children) = (&tree.state, &mut tree.children);
        let state = state_tree.downcast_ref::<ZoomState>();
        // In the grid's own coordinates, as in `operate`: a scrollable hands `overlay` its
        // window on the screen, not on the content (review A F2, M2).
        let expanded = self.expanded_layout(state, layout.position(), last_view(state, layout));
        let overlays: Vec<_> = self
            .cards
            .iter_mut()
            .zip(children)
            .zip(layout.children())
            .enumerate()
            .filter_map(|(i, ((card, card_tree), card_layout))| {
                let card_layout = match expanded {
                    Some((index, drawn)) if index == i => drawn,
                    _ => card_layout,
                };
                card.as_widget_mut().overlay(
                    card_tree,
                    card_layout,
                    renderer,
                    viewport,
                    translation,
                )
            })
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

/// The least height of a card and of a grid row (feature 1120, FR-014).
const MIN_CARD_HEIGHT: f32 = super::overview_card::MIN_HEIGHT;

impl<'a, M: Clone + 'a> From<CardGrid<'a, M>> for Element<'a, M> {
    fn from(g: CardGrid<'a, M>) -> Self {
        if g.unscrolled {
            return Element::new(g.into_grid());
        }
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
    use super::super::overview_card::MIN_HEIGHT;
    use super::*;
    use crate::ui::material::test_support::{laid_out, leaves, roles, Mounted};
    use iced::{Rectangle, Size};
    use micold_core::overview::{CardModel, CardReason, CardState, LocationType};
    use micold_core::session::SessionLocation;

    /// Cards of width `w` on a ribbon: card `i` starts at `i · (w + gap)` along rows `width + gap`
    /// long, so row `r` shows the stretch `r · (width + gap) .. + width` and the `gap` after it is
    /// not shown. A card that runs past a row's end is cut there and continues at the next row's
    /// start; at a rest width (`width + gap = n · (w + gap)`) no card is cut. Every row is
    /// `card_height` tall.
    ///
    /// Cards keep reading order and a constant pitch, so no two pieces of different cards intersect,
    /// and a card never disappears: of its `w` it loses at most the `gap`. The grid itself stacks rows
    /// of differing heights (`ribbon_rows`, `stack_rows`); this uniform form is feature 970's tested
    /// model of it.
    fn ribbon(count: usize, w: f32, gap: f32, width: f32, card_height: f32) -> Vec<Piece> {
        let mut pieces = ribbon_rows(count, w, gap, width);
        let heights = vec![card_height; row_count(&pieces)];
        stack_rows(&mut pieces, &heights, gap);
        pieces
    }

    /// The gap between cards, and the tolerance for comparing laid-out pixels.
    const GAP: f32 = 16.0;
    const EPS: f32 = 0.01;

    /// Card `i` with no optional row: the sparse card.
    fn sparse(i: usize) -> CardModel {
        CardModel {
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
            dir: std::path::PathBuf::from("/repo"),
            issue_link: None,
            pull_requests: None,
        }
    }

    /// Card `i` with every optional row: branch, activity, progress with a bar, pull requests.
    fn rich(i: usize) -> CardModel {
        use micold_core::task_progress::{Phase, Progress, Ratio, Source};
        CardModel {
            branch: Some(format!("feat/{i}-overview")),
            activity: Some(micold_core::hook_activity::CurrentActivity {
                line: "Editing card_grid.rs".into(),
                ratio: None,
            }),
            progress: Progress {
                phase: Some(Phase::Implement),
                ratio: Some(Ratio { done: 6, total: 10 }),
                source: Source::TaskList,
            },
            prs: micold_core::overview::PrDisplay::Unknown(
                micold_core::overview::PrUnknown::Unreachable,
            ),
            ..sparse(i)
        }
    }

    fn of<M: 'static>(models: &[CardModel]) -> Vec<OverviewCard<'static, M>> {
        models
            .iter()
            .map(|m| OverviewCard::new(m, roles()))
            .collect()
    }

    fn cards<M: 'static>(n: usize) -> Vec<OverviewCard<'static, M>> {
        of(&(0..n).map(sparse).collect::<Vec<_>>())
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
            if out.len() == before
                && (b.height - super::super::overview_card::MIN_HEIGHT).abs() < EPS
            {
                out.push(b);
            }
        }
        let mut out = Vec::new();
        walk(node, iced::Vector::ZERO, &mut out);
        // A card's fill and its target-mark layer, stacked over it, are each such a box (1120).
        out.dedup();
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
            assert!((b.height - super::super::overview_card::MIN_HEIGHT).abs() < EPS);
        }
    }

    // ---- feature 1120: a minimum height, and even rows ----

    /// The grid of `models` laid out `width` wide at rest: its height and each card's box.
    fn even(models: &[CardModel], width: f32) -> (f32, Vec<Rectangle>) {
        let node = Rig::new().lay(of(models), normal(0.0), width);
        let boxes = node.children()[..models.len()]
            .iter()
            .map(layout::Node::bounds)
            .collect();
        (node.size().height, boxes)
    }

    /// FR-014, US4.2, US4.3: in a row where one card has every optional row, every card is as
    /// tall as that card, which is taller than the minimum; a sparse card alone in its row is the
    /// minimum, and the next row starts a gap below the tallest card.
    #[test]
    fn every_card_in_a_row_is_as_tall_as_the_tallest() {
        let models = [sparse(0), rich(1), sparse(2), sparse(3)];
        let (height, boxes) = even(&models, 900.0);
        let tall = boxes[1].height;
        assert!(
            tall > MIN_HEIGHT,
            "the full card grows past the minimum: {tall}"
        );
        for b in &boxes[..3] {
            assert!((b.height - tall).abs() < EPS, "{b:?} != {tall}");
            assert!(b.y.abs() < EPS, "{b:?}");
        }
        assert!((boxes[3].height - MIN_HEIGHT).abs() < EPS, "{:?}", boxes[3]);
        assert!((boxes[3].y - (tall + GAP)).abs() < EPS, "{:?}", boxes[3]);
        assert!((height - (tall + GAP + MIN_HEIGHT)).abs() < EPS, "{height}");
    }

    /// Feature 1121 US1 sc.5 (feature 1120 FR-014): a card whose title wraps to three lines makes
    /// its row taller, every card in the row takes its height, and no two cards overlap.
    #[test]
    fn a_wrapped_title_sets_its_rows_height_and_nothing_overlaps() {
        let plain = [rich(0), rich(1), rich(2)];
        let (_, before) = even(&plain, 900.0);
        let mut wrapped = plain.clone();
        wrapped[1].title =
            "Overview cards wrap the title onto three lines before an ellipsis ".repeat(4);
        let (_, boxes) = even(&wrapped, 900.0);
        let tall = boxes[1].height;
        assert!(
            tall > before[1].height + EPS,
            "the wrapped title makes the card taller: {tall} vs {}",
            before[1].height
        );
        for (i, a) in boxes.iter().enumerate() {
            assert!((a.height - tall).abs() < EPS, "{a:?} != {tall}");
            assert!(a.y.abs() < EPS, "one row: {a:?}");
            for b in &boxes[i + 1..] {
                assert!(
                    !super::super::test_support::overlap(a, b),
                    "{a:?} overlaps {b:?}"
                );
            }
        }
    }

    /// Review A F1 (M3): a tall card's sub-pixel sliver at the start of the next row, mid-zoom,
    /// does not stretch that row; the same card's full piece still sets its own row.
    #[test]
    fn a_sliver_does_not_stretch_its_row() {
        let pieces = ribbon_rows(2, 100.0, 0.0, 199.5);
        let sliver = pieces
            .iter()
            .find(|p| p.card == 1 && p.row == 1)
            .expect("card 1 runs half a pixel into row 1");
        assert!(sliver.rect.width < ROW_PIECE, "{sliver:?}");
        let tall = MIN_HEIGHT + 100.0;
        let heights = row_heights(&pieces, &[MIN_HEIGHT, tall]);
        assert_eq!(heights.len(), 2);
        assert!((heights[0] - tall).abs() < EPS, "{heights:?}");
        assert!((heights[1] - MIN_HEIGHT).abs() < EPS, "{heights:?}");
    }

    /// Spec edge case "Pane narrower than one minimum-width card": at 200 px every card has a row
    /// of its own and no card overlaps another.
    #[test]
    fn a_pane_under_one_minimum_card_stacks_the_cards() {
        let models = [rich(0), sparse(1), rich(2)];
        let (_, boxes) = even(&models, 200.0);
        for (i, a) in boxes.iter().enumerate() {
            assert!(a.x.abs() < EPS && (a.width - 200.0).abs() < EPS, "{a:?}");
            assert!(a.height >= MIN_HEIGHT, "{a:?}");
            for b in &boxes[i + 1..] {
                assert!(a.y + a.height <= b.y + EPS, "{a:?} overlaps {b:?}");
            }
        }
    }

    /// A card split across two rows during a zoom is laid out at its taller row's height, and its
    /// piece in the shorter row is cut to that row.
    #[test]
    fn a_card_split_across_rows_takes_its_taller_row() {
        let models: Vec<CardModel> = (0..7)
            .map(|i| if i == 4 { rich(i) } else { sparse(i) })
            .collect();
        let mut rig = Rig::new();
        rig.lay(of(&models), normal(300.0), 1150.0);
        let node = rig.lay(of(&models), full(300.0), 1300.0);
        let pieces = rig.pieces();
        let split: Vec<&Piece> = pieces.iter().filter(|p| p.card == 3).collect();
        assert_eq!(
            split.len(),
            2,
            "card 3 runs through the row end: {pieces:?}"
        );
        let tall = node.children()[4].size().height;
        assert!(tall > MIN_HEIGHT);
        assert!(
            (split[0].rect.height - MIN_HEIGHT).abs() < EPS,
            "{:?}",
            split[0]
        );
        assert!((split[1].rect.height - tall).abs() < EPS, "{:?}", split[1]);
        assert!(
            (split[1].rect.y - (MIN_HEIGHT + GAP)).abs() < EPS,
            "{:?}",
            split[1]
        );
        assert!((node.children()[3].size().height - tall).abs() < EPS);
    }

    /// The pieces the grid somewhere in `tree` drew at its last layout.
    fn zoom_pieces(tree: &iced::advanced::widget::Tree) -> Option<Vec<Piece>> {
        if let tree::State::Some(state) = &tree.state {
            if let Some(zoom) = state.downcast_ref::<ZoomState>() {
                return Some(zoom.pieces.clone());
            }
        }
        tree.children.iter().find_map(zoom_pieces)
    }

    /// What a grid of `models` paints mid-zoom, from `w` wide at rest to `w + freed / 2` wide on
    /// the way to full screen, in a viewport `view_h` tall with room below it: the zoom's pieces,
    /// and every painted pixel outside all of them (within 2 px), or below the grid's bottom edge.
    fn painted_mid_zoom(
        models: &[CardModel],
        targeted: impl Fn(usize) -> bool,
        w: f32,
        freed: f32,
        view_h: f32,
    ) -> (Vec<Piece>, Vec<(u32, u32)>) {
        use iced::advanced::renderer::Headless as _;
        use iced::advanced::Renderer as _;
        const BELOW: f32 = 80.0;
        const SCROLLBAR: f32 = 16.0;
        let grid = |full_screen: bool| -> Element<'static, String> {
            let cards = of::<String>(models)
                .into_iter()
                .enumerate()
                .map(|(i, card)| card.targeted(targeted(i)))
                .collect();
            CardGrid::new(cards, roles())
                .min_width(MIN_WIDTH)
                .spacing(GAP)
                .zoom(Zoom { full_screen, freed })
                .into()
        };
        let mut m = Mounted::new_in(grid(false), Size::new(w, view_h));
        m.element = grid(true);
        m.tree.diff(m.element.as_widget());
        m.room = Size::new(w + freed / 2.0, view_h);
        m.relayout();
        let pieces = zoom_pieces(&m.tree).expect("the grid's zoom state");

        let size = Size::new(w + freed / 2.0, view_h + BELOW);
        let window = Rectangle::with_size(size);
        m.renderer.reset(window);
        m.element.as_widget().draw(
            &m.tree,
            &mut m.renderer,
            &crate::ui::material::theme(micold_core::theme::ColorScheme::Light),
            &renderer::Style {
                text_color: iced::Color::BLACK,
            },
            Layout::new(&m.node),
            mouse::Cursor::Unavailable,
            &window,
        );
        let (pw, ph) = (size.width.ceil() as u32, size.height.ceil() as u32);
        let pixels =
            m.renderer
                .screenshot(Size::new(pw, ph), 1.0, iced::Color::from_rgb(1.0, 0.0, 1.0));
        // A pixel counts as inside a piece within 2 px of it (antialiasing), above the grid's edge.
        // The scrollable's own bar, along the right edge above that edge, is not a card's.
        let bar = |x: f32, y: f32| x > size.width - SCROLLBAR && y < view_h;
        let inside = |x: f32, y: f32| {
            bar(x, y)
                || y < view_h + 2.0
                    && pieces.iter().any(|p| {
                        x > p.rect.x - 2.0
                            && x < p.rect.x + p.rect.width + 2.0
                            && y > p.rect.y - 2.0
                            && y < p.rect.y + p.rect.height + 2.0
                    })
        };
        let stray = (0..ph)
            .flat_map(|y| (0..pw).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let i = ((y * pw + x) * 4) as usize;
                pixels[i..i + 3] != [255, 0, 255] && !inside(x as f32 + 0.5, y as f32 + 0.5)
            })
            .collect();
        (pieces, stray)
    }

    /// 1120 (T043, close visual pass): mid-zoom, nothing of a split card's lower-row piece shows
    /// below the scrolled grid's bottom edge.
    #[test]
    fn a_split_card_mid_zoom_paints_only_inside_its_pieces() {
        const VIEW_H: f32 = 320.0;
        // Three columns at W and four at W + FREED: mid-zoom, the ribbon cuts cards across rows.
        let models: Vec<CardModel> = (0..12).map(sparse).collect();
        let (pieces, stray) = painted_mid_zoom(&models, |i| i % 3 == 1, 1000.0, 300.0, VIEW_H);
        let split = |card: usize| pieces.iter().filter(|p| p.card == card).count() == 2;
        assert!(
            pieces
                .iter()
                .any(|p| split(p.card) && p.rect.y + p.rect.height > VIEW_H + 2.0),
            "mid-zoom, a split card's piece crosses the grid's bottom edge: {pieces:?}"
        );
        let below = stray.iter().filter(|&&(_, y)| y as f32 >= VIEW_H).count();
        assert!(
            stray.is_empty(),
            "{} pixels painted outside every piece ({below} below the grid), first {:?}",
            stray.len(),
            &stray[..stray.len().min(5)]
        );
    }

    /// 1120 (T043, review A F1/F6): mid-zoom, a targeted card split across a short row and a tall
    /// one is laid out at the tall row's height, and its piece in the short row is cut to that
    /// row. Its target mark, drawn in a layer of its own, stays in the piece: nothing of the card
    /// shows in the gap under the short row.
    #[test]
    fn a_targeted_split_card_keeps_its_mark_inside_its_pieces() {
        // The geometry of `a_card_split_across_rows_takes_its_taller_row`: card 3 runs through the
        // end of a sparse row into a row with the rich card 4.
        let models: Vec<CardModel> = (0..7)
            .map(|i| if i == 4 { rich(i) } else { sparse(i) })
            .collect();
        let (pieces, stray) = painted_mid_zoom(&models, |i| i == 3, 1150.0, 300.0, 700.0);
        let split: Vec<&Piece> = pieces.iter().filter(|p| p.card == 3).collect();
        assert_eq!(split.len(), 2, "card 3 is split: {pieces:?}");
        assert!(
            split[0].rect.height < split[1].rect.height - GAP,
            "card 3's upper piece is cut short of the card: {split:?}"
        );
        assert!(
            stray.is_empty(),
            "{} pixels painted outside every piece, first {:?}",
            stray.len(),
            &stray[..stray.len().min(5)]
        );
    }

    // ---- feature 970: the cards zoom with the sidebar ----

    const H: f32 = super::super::overview_card::MIN_HEIGHT;
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
        let row = MIN_HEIGHT + GAP;
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
                row: (y / (H + GAP)).round() as usize,
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

    /// Finding 11: a layout 2 px short of the end width is still a running zoom.
    #[test]
    fn a_layout_two_pixels_short_of_the_end_is_still_zooming() {
        let mut state = ZoomState::new(false);
        state.advance(normal(300.0), 1150.0);
        state.advance(full(300.0), 1300.0);
        assert_eq!(state.advance(full(300.0), 1448.0), Some((1150.0, 1450.0)));
        assert_eq!(state.advance(full(300.0), 1450.0), None, "at the end");
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
            row: 0,
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

    // ---- Feature 1121: the expanded card (U2, U3) ----

    /// [`rich`] card `i`, titled `t<i>`, so each card's title is its own.
    fn titled(i: usize) -> CardModel {
        CardModel {
            title: format!("t{i}"),
            ..rich(i)
        }
    }

    /// Card `i` with every text far longer than a card: the expanded card's content.
    fn long(i: usize) -> CardModel {
        CardModel {
            title: format!(
                "long {i} {}",
                "and a title too long for any card ".repeat(5)
            ),
            branch: Some(format!("feat/{}", "b".repeat(115))),
            dir: std::path::PathBuf::from(format!("/{}", "d".repeat(119))),
            activity: Some(micold_core::hook_activity::CurrentActivity {
                line: "Editing card_grid.rs and then ".repeat(12),
                ratio: None,
            }),
            ..rich(i)
        }
    }

    /// `models` as pressable cards: card `i` publishes `open i` on a press and `menu i` on a
    /// secondary press.
    fn wired(models: &[CardModel]) -> Vec<OverviewCard<'static, String>> {
        models
            .iter()
            .enumerate()
            .map(|(i, m)| {
                OverviewCard::new(m, roles())
                    .on_press(format!("open {i}"))
                    .on_secondary_press(move |_| format!("menu {i}"))
            })
            .collect()
    }

    /// The grid of `models`, card `index` expanded with `content` (its twin from
    /// `models[index]`); `pressable`: whether the expanded card takes a press.
    fn expanded_grid(
        models: &[CardModel],
        index: usize,
        content: &CardModel,
        pressable: bool,
    ) -> CardGrid<'static, String> {
        let mut cards = wired(models);
        let card = OverviewCard::new(content, roles())
            .on_body_scroll(|up, down| format!("body {up} {down}"))
            .expanded(true);
        cards[index] = if pressable {
            card.on_press(format!("open {index}"))
                .on_secondary_press(move |_| format!("menu {index}"))
        } else {
            card
        };
        CardGrid::new(cards, roles())
            .min_width(280.0)
            .spacing(GAP)
            .expanded(index, OverviewCard::new(&models[index], roles()))
    }

    fn plain_grid(models: &[CardModel]) -> CardGrid<'static, String> {
        CardGrid::new(wired(models), roles())
            .min_width(280.0)
            .spacing(GAP)
    }

    /// `grid`, unscrolled, in a `width` × `height` window.
    fn mounted(grid: CardGrid<'static, String>, width: f32, height: f32) -> Mounted<'static> {
        Mounted::new_in(grid.unscrolled(), Size::new(width, height))
    }

    /// The first `n` card nodes of a laid-out grid.
    fn slots(node: &iced::advanced::layout::Node, n: usize) -> Vec<Rectangle> {
        node.children()[..n].iter().map(|c| c.bounds()).collect()
    }

    /// The `y` at which `m` paints the paragraph `content`, if it does.
    fn painted_y(m: &mut Mounted<'_>, content: &str) -> Option<f32> {
        m.painted_text()
            .into_iter()
            .find(|p| p.content == content)
            .map(|p| p.bounds.y)
    }

    /// Feature 1121 U2, FR-009, US2 sc.2, US4 sc.5: with card 1 expanded, at 900 px (3 columns)
    /// and 279 px (1 column), every row and every other card's slot is that of the grid with
    /// nothing expanded, even though the expanded card's text is far longer than its twin's; on
    /// the first layout the expanded node has its slot's x, y and width and is as tall as its
    /// content up to the view's height.
    #[test]
    fn an_expanded_card_keeps_every_slot_and_takes_its_own() {
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        for (width, height) in [(900.0, 4000.0), (900.0, 300.0), (279.0, 4000.0)] {
            let plain = mounted(plain_grid(&models), width, height);
            let grown = mounted(expanded_grid(&models, 1, &long(1), true), width, height);
            assert_eq!(
                grown.node.size(),
                plain.node.size(),
                "{width}: the rows moved"
            );
            let (want, got) = (slots(&plain.node, 6), slots(&grown.node, 6));
            for i in [0, 2, 3, 4, 5] {
                assert_eq!(got[i], want[i], "{width}: card {i}'s slot moved");
            }
            let natural = laid_out(
                OverviewCard::<String>::new(&long(1), roles())
                    .expanded(true)
                    .into(),
                Size::new(want[1].width, f32::INFINITY),
            )
            .size()
            .height;
            assert!(
                natural > want[1].height,
                "the content is no taller than the slot"
            );
            let e = got[1];
            assert_eq!(
                (e.x, e.y, e.width),
                (want[1].x, want[1].y, want[1].width),
                "{width}"
            );
            assert!(
                (e.height - natural.min(height)).abs() < EPS,
                "{width}x{height}: {e:?}, natural {natural}"
            );
        }
    }

    /// Feature 1121 U3: the expanded card is drawn after every other card.
    #[test]
    fn an_expanded_card_is_drawn_after_every_other_card() {
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let content = long(1);
        let mut m = mounted(expanded_grid(&models, 1, &content, true), 900.0, 4000.0);
        let painted = m.painted_text();
        let title = painted
            .iter()
            .position(|p| p.content == content.title)
            .expect("the expanded title is painted");
        let last_other = painted
            .iter()
            .rposition(|p| (0..6).any(|i| i != 1 && p.content == format!("t{i}")))
            .expect("other cards are painted");
        assert!(title > last_other, "{title} drawn before {last_other}");
    }

    /// Feature 1121 FR-008, US2 sc.5, K3: in a view ending inside the last row, the last row's
    /// expanded card is moved up by `place`, so it ends inside the view.
    #[test]
    fn an_expanded_card_in_the_last_row_is_moved_up_into_the_view() {
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let height = 700.0;
        let plain_title = {
            let mut plain = mounted(plain_grid(&models), 900.0, height);
            let slot = slots(&plain.node, 6)[4];
            (
                slot,
                painted_y(&mut plain, &models[4].title).expect("card 4's title"),
            )
        };
        let (slot, collapsed_y) = plain_title;
        let content = long(4);
        let mut m = mounted(expanded_grid(&models, 4, &content, true), 900.0, height);
        let drawn = slots(&m.node, 6)[4].height;
        let p = micold_core::overview_expansion::place((slot.y, slot.height), drawn, (0.0, height));
        assert!(
            p.top < slot.y,
            "the case does not need a shift: {p:?} {slot:?}"
        );
        assert!(p.top + p.height <= height + EPS, "{p:?}");
        let y = painted_y(&mut m, &content.title).expect("the expanded title");
        assert!(
            (y - collapsed_y - (p.top - slot.y)).abs() < 0.5,
            "drawn at {y}, the slot's title at {collapsed_y}, placed {p:?} for {slot:?}"
        );
    }

    /// Feature 1121 FR-010, US4 sc.1: a press, a release and a secondary press over the part of
    /// the expanded card lying on a neighbour publish only the expanded card's messages, and the
    /// pointer is the expanded card's.
    #[test]
    fn the_part_over_a_neighbour_belongs_to_the_expanded_card() {
        use iced::advanced::mouse::{Button, Cursor, Event as Mouse};
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let content = long(1);
        let mut m = mounted(expanded_grid(&models, 1, &content, true), 900.0, 4000.0);
        let (expanded, under) = (slots(&m.node, 6)[1], slots(&m.node, 6)[4]);
        let at = iced::Point::new(under.center_x(), under.y + 20.0);
        assert!(expanded.contains(at), "card 1 does not reach over card 4");
        let mut published = m.press(at);
        published.extend(m.send(
            iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            Cursor::Available(at),
        ));
        published.extend(m.send(
            iced::Event::Mouse(Mouse::ButtonPressed(Button::Right)),
            Cursor::Available(at),
        ));
        assert_eq!(published, ["open 1", "menu 1"]);
        let still = mounted(expanded_grid(&models, 1, &content, false), 900.0, 4000.0);
        let pointer = still.element.as_widget().mouse_interaction(
            &still.tree,
            iced::advanced::Layout::new(&still.node),
            Cursor::Available(at),
            &Rectangle::with_size(still.room),
            &still.renderer,
        );
        assert_ne!(
            pointer,
            mouse::Interaction::Pointer,
            "the pressable neighbour's pointer shows through"
        );
    }

    /// Feature 1121 R4: the twin takes no keyboard traversal: the expanded grid offers as many
    /// focusable cards as the plain one.
    #[test]
    fn the_twin_takes_no_focus_traversal() {
        use crate::ui::material::test_support::Focusables;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut plain = mounted(plain_grid(&models), 900.0, 4000.0);
        let mut grown = mounted(expanded_grid(&models, 1, &long(1), true), 900.0, 4000.0);
        assert_eq!(grown.drive(Focusables(0)), plain.drive(Focusables(0)));
    }

    /// Feature 1121 Edge Case *content changes*, FR-009: the expanded card given a longer
    /// activity stays at its slot with the new text, and the rows do not move.
    #[test]
    fn an_expanded_card_takes_new_text_in_place() {
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut m = mounted(expanded_grid(&models, 1, &rich(1), true), 900.0, 4000.0);
        let before = slots(&m.node, 6);
        let size = m.node.size();
        // Longer, yet short enough for the card to lie whole within the grid's two rows, the
        // harness's viewport.
        let longer = CardModel {
            activity: Some(micold_core::hook_activity::CurrentActivity {
                line: "Editing card_grid.rs and then ".repeat(4),
                ratio: None,
            }),
            ..rich(1)
        };
        m.element = expanded_grid(&models, 1, &longer, true).unscrolled().into();
        m.tree.diff(m.element.as_widget());
        m.relayout();
        let after = slots(&m.node, 6);
        assert_eq!(m.node.size(), size, "the rows moved");
        assert_eq!((after[1].x, after[1].y), (before[1].x, before[1].y));
        assert!(after[1].height > before[1].height, "{after:?}");
        let activity = micold_core::overview_text::ExpandedText::of(&longer, None)
            .activity
            .expect("an activity");
        assert!(
            m.painted_text()
                .iter()
                .any(|p| p.content == format!("NOW  {activity}")),
            "the new activity is not painted"
        );
    }

    /// Feature 1121 US2 sc.6, FR-008: an expanded card taller than the view is as tall as the
    /// view; a wheel over it scrolls its body first, and at the body's end the Overview. Within
    /// one wheel gesture iced keeps the wheel on the body it started in (scroll latching, research
    /// R6); once the gesture ends (the pointer moves after 100 ms), the wheel goes on to the
    /// Overview.
    #[test]
    fn a_wheel_over_a_tall_expanded_card_scrolls_its_body_then_the_overview() {
        use iced::advanced::mouse::{Cursor, Event as Mouse, ScrollDelta};
        let models: Vec<CardModel> = (0..9).map(titled).collect();
        let mut m = Mounted::new_in(
            expanded_grid(&models, 0, &long(0), true).on_scroll("grid".to_string()),
            Size::new(900.0, 400.0),
        );
        let at = iced::Point::new(150.0, 300.0);
        // A draw, then an event: the grid sees its view and asks for the layout that caps the
        // card at it (review A F1, M2: the draw alone must not leave the card uncapped).
        m.painted_text();
        m.hover(at);
        let card = m.node.children()[0].children()[0].bounds();
        assert!((card.height - 400.0).abs() < EPS, "{card:?}");
        let wheel = |m: &mut Mounted<'_>, dy: f32| {
            m.send(
                iced::Event::Mouse(Mouse::WheelScrolled {
                    delta: ScrollDelta::Pixels { x: 0.0, y: dy },
                }),
                Cursor::Available(at),
            )
        };
        assert_eq!(wheel(&mut m, -20.0), ["body true true"]);
        assert_eq!(wheel(&mut m, -10_000.0), ["body true false"]);
        assert!(
            wheel(&mut m, -20.0).is_empty(),
            "the gesture left the body it started in"
        );
        // iced ends a wheel transaction on a cursor move only once 100 ms have passed since the
        // last scroll (`iced_widget::scrollable`); this waits that out. It is iced's gesture
        // window, not a design-system duration.
        const ICED_WHEEL_TRANSACTION_MS: u64 = 120;
        std::thread::sleep(std::time::Duration::from_millis(ICED_WHEEL_TRANSACTION_MS));
        m.send(
            iced::Event::Mouse(Mouse::CursorMoved { position: at }),
            Cursor::Available(at),
        );
        assert_eq!(
            wheel(&mut m, -20.0),
            ["grid"],
            "the Overview did not scroll"
        );
    }

    /// Review A F1 (M2): a view that shrinks under an expanded card caps it at the new height on
    /// the next event, even when a draw saw the new view first.
    #[test]
    fn an_expanded_card_follows_a_shrinking_view() {
        let models: Vec<CardModel> = (0..9).map(titled).collect();
        let mut m = Mounted::new_in(
            expanded_grid(&models, 0, &long(0), true),
            Size::new(900.0, 400.0),
        );
        let at = iced::Point::new(150.0, 100.0);
        m.painted_text();
        m.hover(at);
        m.room = Size::new(900.0, 300.0);
        m.relayout();
        m.painted_text();
        m.hover(at);
        let card = m.node.children()[0].children()[0].bounds();
        assert!((card.height - 300.0).abs() < EPS, "{card:?}");
    }

    /// Review A F3 (M2), FR-008, FR-009: an expanded card shorter than the row a neighbour
    /// stretches is still as tall as its own slot.
    #[test]
    fn an_expanded_card_is_never_shorter_than_its_stretched_slot() {
        let mut models: Vec<CardModel> = (0..6).map(sparse).collect();
        models[1] = CardModel {
            title: "a neighbour's title that wraps onto three lines in a card ".repeat(3),
            ..rich(1)
        };
        let content = sparse(0);
        let m = mounted(expanded_grid(&models, 0, &content, true), 900.0, 4000.0);
        let got = slots(&m.node, 6);
        let natural = laid_out(
            OverviewCard::<String>::new(&content, roles())
                .expanded(true)
                .into(),
            Size::new(got[0].width, f32::INFINITY),
        )
        .size()
        .height;
        assert!(
            natural < got[1].height,
            "the neighbour does not stretch the row past the expanded card: {natural} {got:?}"
        );
        assert!(
            (got[0].height - got[1].height).abs() < EPS,
            "the expanded card is shorter than its slot: {got:?}"
        );
    }

    /// Review A F2 (M2): a scrollable hands `overlay` its window on the screen, not the view the
    /// grid was drawn in; a tooltip of the expanded card still floats by the card as drawn, below
    /// its state word, not where that other window would place the card.
    #[test]
    fn an_expanded_cards_tooltip_floats_by_it_as_drawn() {
        let models: Vec<CardModel> = (0..15).map(titled).collect();
        let content = long(7);
        let mut m = mounted(expanded_grid(&models, 7, &content, true), 900.0, 4000.0);
        // Update and draw in one view: the window is the grid.
        m.room = m.node.size();
        m.relayout();
        let slot = slots(&m.node, 15)[7];
        let painted = m.painted_text();
        let title = painted
            .iter()
            .position(|p| p.content == content.title)
            .expect("the expanded title is painted");
        // The header's state word, two paragraphs before the title (glyph, word, type word).
        let word = painted[title - 2].bounds;
        assert!(
            word.y > 0.0 && word.y < slot.y,
            "the card is not moved up within the view: {word:?} {slot:?}"
        );
        // A window 200 px short of the grid would move the card up by another 200 px.
        let screen = Rectangle::with_size(Size::new(m.room.width, m.room.height - 200.0));
        let floated = m.floated_in(word.center(), screen);
        assert!(!floated.is_empty(), "nothing floats over the state word");
        for p in &floated {
            let below = p.bounds.y - (word.y + word.height);
            assert!(
                (0.0..=60.0).contains(&below),
                "the tooltip floats at {:?}, not just below the state word at {word:?}",
                p.bounds
            );
        }
    }

    /// `grid` with every card publishing `rest i` on a rest and `leave i` on leaving.
    fn with_rests(mut grid: CardGrid<'static, String>) -> CardGrid<'static, String> {
        grid.cards = grid
            .cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                card.on_rest(format!("rest {i}"))
                    .on_leave(format!("leave {i}"))
            })
            .collect();
        grid
    }

    /// A frame at `now` with the pointer at `at`: what the grid publishes.
    fn frame_at(m: &mut Mounted<'_>, at: Point, now: std::time::Instant) -> Vec<String> {
        m.send(
            Event::Window(iced::window::Event::RedrawRequested(now)),
            mouse::Cursor::Available(at),
        )
    }

    const NOTHING: [&str; 0] = [];

    /// Feature 1121 U3, R5, FR-002, SC-006: a pointer resting on a card publishes its `on_rest`
    /// once, after `EXPAND_REST` and not before; leaving it publishes its `on_leave`.
    #[test]
    fn a_resting_pointer_publishes_the_cards_rest_then_its_leave() {
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut m = mounted(with_rests(plain_grid(&models)), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let (on, t0) = (slot.center(), std::time::Instant::now());
        assert_eq!(frame_at(&mut m, on, t0), NOTHING);
        /// A millisecond short of the rest: not a motion duration, the smallest step before it.
        const JUST_SHORT_MS: u64 = 1;
        let early = t0 + EXPAND_REST - std::time::Duration::from_millis(JUST_SHORT_MS);
        assert_eq!(frame_at(&mut m, on, early), NOTHING, "rested too early");
        assert_eq!(frame_at(&mut m, on, t0 + EXPAND_REST), ["rest 1"]);
        assert_eq!(
            frame_at(&mut m, on, t0 + EXPAND_REST * 3),
            NOTHING,
            "rested twice"
        );
        let gap = Point::new(slot.x - GAP / 2.0, slot.center_y());
        assert_eq!(frame_at(&mut m, gap, t0 + EXPAND_REST * 4), ["leave 1"]);
        assert_eq!(frame_at(&mut m, gap, t0 + EXPAND_REST * 6), NOTHING);
    }

    /// Feature 1121 U3, R5: from the expanded card's drawn area straight onto the card beneath it,
    /// with no frame outside every card, the expanded card is left and the one beneath rests.
    #[test]
    fn moving_off_the_expanded_card_onto_the_card_beneath_rests_on_it() {
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let grid = with_rests(expanded_grid(&models, 1, &rich(1), false));
        let mut m = mounted(grid, 900.0, 4000.0);
        let (drawn, under) = (slots(&m.node, 6)[1], slots(&m.node, 6)[4]);
        let over_both = Point::new(drawn.center_x(), under.y + 2.0);
        let beneath = Point::new(drawn.center_x(), drawn.y + drawn.height + 2.0);
        assert!(
            drawn.contains(over_both),
            "the expanded card does not reach over card 4"
        );
        assert!(
            under.contains(beneath),
            "card 4 does not show below the expanded card"
        );
        let t0 = std::time::Instant::now();
        assert_eq!(frame_at(&mut m, over_both, t0), NOTHING);
        assert_eq!(frame_at(&mut m, over_both, t0 + EXPAND_REST), ["rest 1"]);
        let t1 = t0 + EXPAND_REST * 2;
        assert_eq!(frame_at(&mut m, beneath, t1), ["leave 1"]);
        assert_eq!(frame_at(&mut m, beneath, t1 + EXPAND_REST), ["rest 4"]);
    }

    /// Feature 1121 Edge Case *scrolling under the pointer*: a scroll that brings another card
    /// under a pointer at rest leaves the old card and rests on the new one, with no mouse event.
    #[test]
    fn a_scroll_bringing_another_card_under_the_pointer_rests_on_it() {
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut m = mounted(with_rests(plain_grid(&models)), 900.0, 4000.0);
        let (first, next) = (slots(&m.node, 6)[1].center(), slots(&m.node, 6)[4].center());
        let t0 = std::time::Instant::now();
        frame_at(&mut m, first, t0);
        assert_eq!(frame_at(&mut m, first, t0 + EXPAND_REST), ["rest 1"]);
        // The window does not move; the content scrolled, so the pointer is over card 4 now.
        let t1 = t0 + EXPAND_REST * 2;
        assert_eq!(frame_at(&mut m, next, t1), ["leave 1"]);
        assert_eq!(frame_at(&mut m, next, t1 + EXPAND_REST), ["rest 4"]);
    }

    /// Feature 1121 R5, FR-005 (review A F1, M3): the cards re-sorted under a pointer at rest, so
    /// that another card now holds the rested card's place, leave the rested card and rest on
    /// the new one: the rest follows the card, not its place in the list.
    #[test]
    fn a_reorder_under_a_resting_pointer_leaves_the_card_and_rests_on_the_new_one() {
        use micold_core::overview_expansion::EXPAND_REST;
        let keyed = |models: &[CardModel]| {
            let cards = models
                .iter()
                .map(|m| {
                    OverviewCard::new(m, roles())
                        .on_rest(format!("rest {:?}", m.location))
                        .on_leave(format!("leave {:?}", m.location))
                })
                .collect();
            CardGrid::new(cards, roles())
                .min_width(280.0)
                .spacing(GAP)
                .unscrolled()
        };
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut m = Mounted::new_in(keyed(&models), Size::new(900.0, 4000.0));
        let on = slots(&m.node, 6)[2].center();
        let t0 = std::time::Instant::now();
        frame_at(&mut m, on, t0);
        assert_eq!(
            frame_at(&mut m, on, t0 + EXPAND_REST),
            [format!("rest {:?}", models[2].location)]
        );
        let mut sorted = models.clone();
        sorted.swap(0, 2);
        m.element = keyed(&sorted).into();
        m.tree.diff(&m.element);
        m.relayout();
        let t1 = t0 + EXPAND_REST * 2;
        assert_eq!(
            frame_at(&mut m, on, t1),
            [format!("leave {:?}", models[2].location)]
        );
        assert_eq!(
            frame_at(&mut m, on, t1 + EXPAND_REST),
            [format!("rest {:?}", models[0].location)]
        );
    }

    /// Feature 1121 R5 (review A F2, M3 round 2): a new rest scope (another project) under a
    /// pointer resting on a card at the same location, such as Default, rests on it anew.
    #[test]
    fn a_new_rest_scope_under_a_resting_pointer_rests_anew() {
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let scoped = |scope: &str| with_rests(plain_grid(&models)).rest_scope(scope);
        let mut m = mounted(scoped("/a"), 900.0, 4000.0);
        let on = slots(&m.node, 6)[1].center();
        let t0 = std::time::Instant::now();
        frame_at(&mut m, on, t0);
        assert_eq!(frame_at(&mut m, on, t0 + EXPAND_REST), ["rest 1"]);
        m.element = scoped("/b").unscrolled().into();
        m.tree.diff(&m.element);
        m.relayout();
        let t1 = t0 + EXPAND_REST * 2;
        assert_eq!(
            frame_at(&mut m, on, t1),
            NOTHING,
            "left a card of the old set"
        );
        assert_eq!(frame_at(&mut m, on, t1 + EXPAND_REST), ["rest 1"]);
    }

    /// Feature 1121 FR-005, U4: the window losing focus publishes the grid's `on_window_blur`,
    /// and regaining it publishes nothing.
    #[test]
    fn the_window_losing_focus_publishes_on_window_blur() {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        let grid = plain_grid(&models).on_window_blur("blur".to_string());
        let mut m = mounted(grid, 900.0, 4000.0);
        let away = mouse::Cursor::Unavailable;
        assert_eq!(
            m.send(Event::Window(iced::window::Event::Unfocused), away),
            ["blur"]
        );
        assert_eq!(
            m.send(Event::Window(iced::window::Event::Focused), away),
            NOTHING
        );
    }

    /// Feature 1121 R5, US4 sc.6: nothing rests while a zoom runs.
    #[test]
    fn nothing_rests_while_a_zoom_runs() {
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..6).map(titled).collect();
        let mut m = mounted(with_rests(plain_grid(&models)), 900.0, 4000.0);
        m.tree.state.downcast_mut::<ZoomState>().ends = Some((900.0, 1200.0));
        let on = slots(&m.node, 6)[1].center();
        let t0 = std::time::Instant::now();
        let mut published = frame_at(&mut m, on, t0);
        published.extend(frame_at(&mut m, on, t0 + EXPAND_REST * 2));
        assert_eq!(published, NOTHING);
    }

    // ---- Feature 1122: a card's issue link in the grid (contract U5; R7, R8) ----

    const LINK: &str = "Issue #1122";

    /// `model` with the issue link `LINK`.
    fn with_issue(model: CardModel) -> CardModel {
        CardModel {
            issue_link: Some(micold_core::issue_link::IssueLink {
                label: LINK.into(),
                address: "https://github.com/o/r/issues/1122".into(),
            }),
            ..model
        }
    }

    /// `card` with its issue link wired: `link i`, id `link-i`, `link rest i`.
    fn link_wired(card: OverviewCard<'static, String>, i: usize) -> OverviewCard<'static, String> {
        card.on_issue_link(format!("link {i}"))
            .link_id(iced::advanced::widget::Id::from(format!("link-{i}")))
            .on_link_rest(format!("link rest {i}"))
    }

    /// Six cards, each with an issue link (`link i`, id `link-i`, `link rest i`), its rest and
    /// leave messages, and its keyboard reports.
    fn linked_grid() -> CardGrid<'static, String> {
        let models: Vec<CardModel> = (0..6).map(|i| with_issue(titled(i))).collect();
        let mut grid = with_rests(plain_grid(&models));
        grid.cards = grid
            .cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| link_wired(card, i))
            .collect();
        grid
    }

    /// The centre of the link painted inside `slot`.
    fn link_in(m: &mut Mounted<'_>, slot: Rectangle) -> Point {
        m.painted_text()
            .into_iter()
            .find(|p| p.content == LINK && slot.contains(p.bounds.center()))
            .unwrap_or_else(|| panic!("no link painted in {slot:?}"))
            .bounds
            .center()
    }

    /// A point in `slot`'s left padding: on the card, off its link.
    fn off_link_in(slot: Rectangle) -> Point {
        Point::new(slot.x + 6.0, slot.y + slot.height * 0.6)
    }

    /// R7, FR-005: a press on card 1's link reaches card 1 only, so card 0, holding the keyboard
    /// by Tab, keeps it: Enter still activates card 0.
    #[test]
    fn a_click_on_a_link_leaves_the_keyboard_with_the_card_that_holds_it() {
        let mut m = mounted(linked_grid(), 900.0, 4000.0);
        m.focus_next();
        let slot = slots(&m.node, 6)[1];
        let at = link_in(&mut m, slot);
        assert!(m.press(at).is_empty());
        let released = m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        );
        assert_eq!(released, ["link 1"]);
        let enter = super::super::test_support::named_key(iced::keyboard::key::Named::Enter);
        assert_eq!(m.send(enter, mouse::Cursor::Unavailable), ["open 0"]);
    }

    /// R8, FR-007: a rest on a card's link publishes its `on_link_rest`; moving onto the rest of
    /// the card starts the wait again and publishes its `on_rest`, with no `on_leave` between;
    /// leaving the card publishes `on_leave` once.
    #[test]
    fn a_rest_on_a_link_and_then_on_its_card_publish_each_their_own() {
        use micold_core::overview_expansion::EXPAND_REST;
        let mut m = mounted(linked_grid(), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let (on, off) = (link_in(&mut m, slot), off_link_in(slot));
        let t0 = std::time::Instant::now();
        assert_eq!(frame_at(&mut m, on, t0), NOTHING);
        assert_eq!(frame_at(&mut m, on, t0 + EXPAND_REST), ["link rest 1"]);
        let t1 = t0 + EXPAND_REST * 2;
        assert_eq!(
            frame_at(&mut m, off, t1),
            NOTHING,
            "a part change is no leave"
        );
        assert_eq!(frame_at(&mut m, off, t1 + EXPAND_REST), ["rest 1"]);
        let t2 = t1 + EXPAND_REST * 2;
        assert_eq!(frame_at(&mut m, on, t2), NOTHING);
        assert_eq!(frame_at(&mut m, on, t2 + EXPAND_REST), ["link rest 1"]);
        let gap = Point::new(slot.x - GAP / 2.0, slot.center_y());
        assert_eq!(frame_at(&mut m, gap, t2 + EXPAND_REST * 2), ["leave 1"]);
    }

    /// R8: a pointer that crosses from the card onto its link before the rest is over waits again
    /// from the crossing; and having left the link early, the card's leave still follows a rest.
    #[test]
    fn crossing_onto_the_link_starts_the_wait_again() {
        use micold_core::overview_expansion::EXPAND_REST;
        let mut m = mounted(linked_grid(), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let (on, off) = (link_in(&mut m, slot), off_link_in(slot));
        let t0 = std::time::Instant::now();
        frame_at(&mut m, off, t0);
        let t1 = t0 + EXPAND_REST / 2;
        assert_eq!(frame_at(&mut m, on, t1), NOTHING);
        assert_eq!(
            frame_at(&mut m, on, t0 + EXPAND_REST),
            NOTHING,
            "not re-armed"
        );
        assert_eq!(frame_at(&mut m, on, t1 + EXPAND_REST), ["link rest 1"]);
        let gap = Point::new(slot.x - GAP / 2.0, slot.center_y());
        let t2 = t1 + EXPAND_REST * 2;
        assert_eq!(frame_at(&mut m, off, t2), NOTHING);
        assert_eq!(frame_at(&mut m, gap, t2 + EXPAND_REST / 2), ["leave 1"]);
    }

    /// G3: the rest on a link asks for one timed wake while it waits and nothing once it is over.
    #[test]
    fn a_rest_on_a_link_asks_for_no_frame_once_it_is_over() {
        use iced::window::RedrawRequest;
        use micold_core::overview_expansion::EXPAND_REST;
        let mut m = mounted(linked_grid(), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let on = link_in(&mut m, slot);
        let asked = |m: &mut Mounted<'_>, now: std::time::Instant| {
            m.tick(now, mouse::Cursor::Available(on)).1
        };
        let t0 = std::time::Instant::now();
        assert!(matches!(asked(&mut m, t0), RedrawRequest::At(_)));
        assert_eq!(asked(&mut m, t0 + EXPAND_REST), RedrawRequest::Wait);
        assert_eq!(asked(&mut m, t0 + EXPAND_REST * 2), RedrawRequest::Wait);
    }

    /// Review A M2 F1, feature 1121 FR-009: with no branch line the link takes a line of its own,
    /// and the expanded card's twin, built with no messages, is laid out with it, so every slot is
    /// that of the grid with nothing expanded (one column, where the twin alone sets the row).
    #[test]
    fn the_twin_of_a_linked_card_with_no_branch_keeps_its_row() {
        let models: Vec<CardModel> = (0..4)
            .map(|i| {
                with_issue(CardModel {
                    branch: None,
                    ..titled(i)
                })
            })
            .collect();
        let linked = |models: &[CardModel]| -> Vec<OverviewCard<'static, String>> {
            wired(models)
                .into_iter()
                .enumerate()
                .map(|(i, card)| link_wired(card, i))
                .collect()
        };
        let rest = mounted(
            CardGrid::new(linked(&models), roles())
                .min_width(280.0)
                .spacing(GAP),
            279.0,
            4000.0,
        );
        let mut cards = linked(&models);
        cards[1] = cards[1].clone().expanded(true);
        let open = mounted(
            CardGrid::new(cards, roles())
                .min_width(280.0)
                .spacing(GAP)
                .expanded(1, OverviewCard::new(&models[1], roles())),
            279.0,
            4000.0,
        );
        let (at_rest, expanded) = (slots(&rest.node, 4), slots(&open.node, 4));
        assert_eq!(
            at_rest[2], expanded[2],
            "the row under the expanded card moved"
        );
        assert_eq!(at_rest[3], expanded[3]);
    }

    /// Review A M2 F2, R7, R8: a link scrolled out of the expanded body's view is not under the
    /// pointer, even where its bounds, translated by the scroll, lie over the title above the
    /// body: a rest there is the card's, not the link's.
    #[test]
    fn a_link_scrolled_out_of_the_body_is_not_under_the_pointer() {
        use iced::advanced::mouse::{Cursor, Event as Mouse, ScrollDelta};
        use micold_core::overview_expansion::EXPAND_REST;
        let models: Vec<CardModel> = (0..9).map(titled).collect();
        let content = with_issue(long(0));
        let mut cards = with_rests(plain_grid(&models)).cards;
        cards[0] = link_wired(
            OverviewCard::new(&content, roles())
                .on_press("open 0".to_string())
                .on_rest("rest 0".to_string())
                .on_leave("leave 0".to_string())
                .on_body_scroll(|up, down| format!("body {up} {down}"))
                .expanded(true),
            0,
        );
        let mut m = Mounted::new_in(
            CardGrid::new(cards, roles())
                .min_width(280.0)
                .spacing(GAP)
                .expanded(0, OverviewCard::new(&models[0], roles())),
            Size::new(900.0, 400.0),
        );
        let body = Point::new(150.0, 300.0);
        m.painted_text();
        m.hover(body);
        let text = m.painted_text();
        let link = text
            .iter()
            .find(|p| p.content == LINK)
            .expect("the link is painted")
            .bounds;
        let title = text
            .iter()
            .find(|p| p.content.starts_with("long 0"))
            .expect("the title is painted")
            .bounds;
        // Scroll the body so the link's translated bounds lie on the title's first line.
        let on_title = Point::new(link.center_x(), title.y + 4.0);
        let scrolled = m.send(
            iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Pixels {
                    x: 0.0,
                    y: on_title.y - link.center_y(),
                },
            }),
            Cursor::Available(body),
        );
        assert_eq!(scrolled, ["body true true"]);
        let t0 = std::time::Instant::now();
        let mut rested = frame_at(&mut m, on_title, t0);
        rested.extend(frame_at(&mut m, on_title, t0 + EXPAND_REST));
        assert_eq!(
            rested,
            ["rest 0"],
            "the pointer on the title counted as on the link"
        );
    }

    // ---- Feature 1123 (T021, contract U4): a pull request number is a link of its card ----

    /// [`linked_grid`] with one pull request row on every card, "#12", its number wired: `pr i
    /// 12`, id `pr-i-12`.
    fn numbered_grid() -> CardGrid<'static, String> {
        let mut grid = linked_grid();
        grid.cards = grid
            .cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                card.pull_requests(
                    vec![super::super::CardPrRow {
                        number: 12,
                        label: "#12".into(),
                        mark: super::super::PrMark::Open,
                        phrase: Some("checks failing".into()),
                        tone: super::super::PrTone::Bad,
                    }],
                    0,
                )
                .on_pr_link(move |n| format!("pr {i} {n}"))
                .pr_link_id(move |n| iced::advanced::widget::Id::from(format!("pr-{i}-{n}")))
            })
            .collect();
        grid
    }

    /// The centre of `content` painted inside `slot`.
    fn painted_in(m: &mut Mounted<'_>, slot: Rectangle, content: &str) -> Point {
        m.painted_text()
            .into_iter()
            .find(|p| p.content == content && slot.contains(p.bounds.center()))
            .unwrap_or_else(|| panic!("no {content:?} painted in {slot:?}"))
            .bounds
            .center()
    }

    /// U4, 1122 FR-007: a rest on a card's number, as on its issue link, publishes the card's
    /// `on_link_rest`; on the rest of the card its `on_rest`.
    #[test]
    fn a_rest_on_a_number_is_a_rest_on_a_link_of_its_card() {
        use micold_core::overview_expansion::EXPAND_REST;
        for link in ["#12", LINK] {
            let mut m = mounted(numbered_grid(), 900.0, 4000.0);
            let slot = slots(&m.node, 6)[1];
            let (on, off) = (painted_in(&mut m, slot, link), off_link_in(slot));
            let t0 = std::time::Instant::now();
            assert_eq!(frame_at(&mut m, on, t0), NOTHING, "{link}");
            assert_eq!(
                frame_at(&mut m, on, t0 + EXPAND_REST),
                ["link rest 1"],
                "{link}"
            );
            let t1 = t0 + EXPAND_REST * 2;
            assert_eq!(frame_at(&mut m, off, t1), NOTHING, "{link}");
            assert_eq!(
                frame_at(&mut m, off, t1 + EXPAND_REST),
                ["rest 1"],
                "{link}"
            );
        }
    }

    /// U4, R7: a primary press on card 1's number reaches card 1 only, so card 0, holding the
    /// keyboard by Tab, keeps it.
    #[test]
    fn a_click_on_a_number_leaves_the_keyboard_with_the_card_that_holds_it() {
        let mut m = mounted(numbered_grid(), 900.0, 4000.0);
        m.focus_next();
        let slot = slots(&m.node, 6)[1];
        let at = painted_in(&mut m, slot, "#12");
        assert!(m.press(at).is_empty());
        let released = m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        );
        assert_eq!(released, ["pr 1 12"]);
        let enter = super::super::test_support::named_key(iced::keyboard::key::Named::Enter);
        assert_eq!(m.send(enter, mouse::Cursor::Unavailable), ["open 0"]);
    }

    /// U4, R7 (tdd-verify close r1 F1): a number that is a Tab stop, its focus reports wired,
    /// still does not take the keyboard on a click: it reports nothing, it is not focused, and
    /// Enter acts on card 0.
    #[test]
    fn a_click_on_a_focusable_number_leaves_the_keyboard_with_the_card_that_holds_it() {
        let mut grid = numbered_grid();
        grid.cards = grid
            .cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                card.on_pr_link_keyboard_focus(move |n, f| format!("pr {i} {n} focus {f}"))
            })
            .collect();
        let mut m = mounted(grid, 900.0, 4000.0);
        m.focus_next();
        let slot = slots(&m.node, 6)[1];
        let at = painted_in(&mut m, slot, "#12");
        assert!(m.press(at).is_empty(), "the press took the keyboard");
        let released = m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        );
        assert_eq!(released, ["pr 1 12"]);
        let number = iced::advanced::widget::Id::from("pr-1-12".to_string());
        assert_eq!(
            m.drive(iced::advanced::widget::operation::focusable::is_focused(
                number
            )),
            Some(false),
            "the click gave the number the keyboard"
        );
        let enter = super::super::test_support::named_key(iced::keyboard::key::Named::Enter);
        assert_eq!(m.send(enter, mouse::Cursor::Unavailable), ["open 0"]);
    }

    /// G3: a rest on a number asks for one timed wake while it waits, none once it is over.
    #[test]
    fn a_rest_on_a_number_asks_for_no_frame_once_it_is_over() {
        use iced::window::RedrawRequest;
        use micold_core::overview_expansion::EXPAND_REST;
        let mut m = mounted(numbered_grid(), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let on = painted_in(&mut m, slot, "#12");
        let mut asked = |now: std::time::Instant| m.tick(now, mouse::Cursor::Available(on));
        let t0 = std::time::Instant::now();
        assert!(matches!(asked(t0).1, RedrawRequest::At(_)));
        assert_eq!(
            asked(t0 + EXPAND_REST),
            (vec!["link rest 1".to_string()], RedrawRequest::Wait)
        );
        assert_eq!(asked(t0 + EXPAND_REST * 2), (vec![], RedrawRequest::Wait));
    }

    /// Review A M2 r1 F2: the gutter round a number is its card's, as it is for the press: a
    /// rest just above "#12", inside its gutter, is a rest on the card, not on a link.
    #[test]
    fn a_rest_in_a_numbers_gutter_is_a_rest_on_the_card() {
        use micold_core::overview_expansion::EXPAND_REST;
        let mut m = mounted(numbered_grid(), 900.0, 4000.0);
        let slot = slots(&m.node, 6)[1];
        let label = m
            .painted_text()
            .into_iter()
            .find(|p| p.content == "#12" && slot.contains(p.bounds.center()))
            .expect("#12 painted on card 1")
            .bounds;
        let at = Point::new(label.center_x(), label.y - 3.0);
        let t0 = std::time::Instant::now();
        assert_eq!(frame_at(&mut m, at, t0), NOTHING);
        assert_eq!(frame_at(&mut m, at, t0 + EXPAND_REST), ["rest 1"]);
    }

    // --- Feature 1299: Reduce motion reaches the cards (D4) ------------------------------------

    use super::super::test_support::{at, strengths, DONE, MIDWAY};
    use iced::window::RedrawRequest;

    fn focus_grid(instant: bool) -> Mounted<'static> {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        mounted(plain_grid(&models).instant(instant), 900.0, 4000.0)
    }

    /// The Reduce motion test (US3-1, FR-007, SC-002): with `instant`, Tab onto a card shows its
    /// focus at full on the first frame, Tab away at none on the first frame, and neither asks
    /// for a next one.
    #[test]
    fn an_instant_grid_shows_and_drops_a_cards_focus_on_the_first_frame() {
        let mut m = focus_grid(true);
        m.focus_next();
        let onto = m.frames(0, 0, strengths);
        assert_eq!(onto[0].1[..2], [1.0, 0.0]);
        assert_eq!(onto[0].2, RedrawRequest::Wait);
        m.focus_next();
        let away = m.frames(16, 16, strengths);
        assert_eq!(away[0].1[..2], [0.0, 1.0]);
        assert_eq!(away[0].2, RedrawRequest::Wait);
    }

    /// The flag reaches the cards: a default grid's card is part-way at ~75 ms.
    #[test]
    fn a_default_grids_card_focus_fades() {
        let mut m = focus_grid(false);
        m.focus_next();
        let mid = at(&m.frames(0, DONE, strengths), MIDWAY)[0];
        assert!(
            0.0 < mid && mid < 1.0,
            "at ~75 ms the card's focus is at {mid}"
        );
    }

    /// FR-008: a grid rebuilt `instant` mid-fade is at its end state on the next frame.
    #[test]
    fn reduce_motion_saved_mid_fade_ends_it_on_the_next_frame() {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        let mut m = focus_grid(false);
        m.focus_next();
        let mid = at(&m.frames(0, MIDWAY, strengths), MIDWAY)[0];
        assert!(mid < 1.0, "{mid}");
        m.rebuild(plain_grid(&models).instant(true).unscrolled());
        let next = m.frames(MIDWAY + 16, MIDWAY + 16, strengths);
        assert_eq!(next[0].1[0], 1.0);
        assert_eq!(next[0].2, RedrawRequest::Wait);
    }

    // --- Feature 1299: the expanded card's shadow eases (D3, D4) --------------------------------

    use super::super::card_lift::Lifts;
    use super::super::test_support::FRAME_MS;
    use iced::widget::{column, container, Space};

    /// Issue 1336: the software renderer damages a quad's bounds, not its shadow, so the region a
    /// card reports while its shadow eases must hold the shadow, and differ from frame to frame,
    /// or the shadow's last pixels outside it are never repainted.
    #[test]
    fn a_lift_reports_damage_covering_its_shadow_at_every_strength() {
        let card = Rectangle::new(Point::new(40.0, 50.0), Size::new(300.0, 200.0));
        let mut last = None;
        for step in 1..=10 {
            let strength = step as f32 / 10.0;
            let shadow = style::eased_shadow(roles(), elevation::MENU, strength);
            let painted = Rectangle {
                x: card.x + shadow.offset.x - shadow.blur_radius,
                y: card.y + shadow.offset.y - shadow.blur_radius,
                width: card.width + 2.0 * shadow.blur_radius,
                height: card.height + 2.0 * shadow.blur_radius,
            };
            let damage = lift_damage(roles(), card, strength);
            assert!(
                damage.x <= painted.x + 0.01
                    && damage.y <= painted.y + 0.01
                    && damage.x + damage.width >= painted.x + painted.width - 0.01
                    && damage.y + damage.height >= painted.y + painted.height - 0.01,
                "strength {strength}: damage {damage:?} misses shadow {painted:?}"
            );
            assert_ne!(
                Some(damage),
                last,
                "strength {strength}: damage did not change"
            );
            last = Some(damage);
        }
    }

    /// The grid of `models`, card `index` (if any) expanded with its own model, `gap` apart.
    fn lift_grid(
        models: &[CardModel],
        index: Option<usize>,
        gap: f32,
    ) -> CardGrid<'static, String> {
        let mut cards = wired(models);
        let grid = |cards: Vec<OverviewCard<'static, String>>| {
            CardGrid::new(cards, roles()).min_width(280.0).spacing(gap)
        };
        match index {
            Some(i) => {
                cards[i] = cards[i].clone().expanded(true);
                grid(cards).expanded(i, OverviewCard::new(&models[i], roles()))
            }
            None => grid(cards),
        }
    }

    fn lifts<'m>(m: &'m Mounted<'_>) -> &'m Lifts {
        &m.tree.state.downcast_ref::<ZoomState>().lifts
    }

    fn strength_of(m: &Mounted<'_>, models: &[CardModel], i: usize, expanded: bool) -> f32 {
        lifts(m).strength(&models[i].location, expanded, false)
    }

    /// A frame tick `ms` after the rig's origin, the pointer away: the frame asked for next.
    fn tick_at(m: &mut Mounted<'_>, ms: u64) -> RedrawRequest {
        m.tick(
            m.origin + std::time::Duration::from_millis(ms),
            mouse::Cursor::Unavailable,
        )
        .1
    }

    /// The pixels of `buffer` (`width` wide) inside `area`, RGB.
    fn pixels_in(buffer: &(Vec<u8>, u32), area: Rectangle) -> Vec<[u8; 3]> {
        let (pixels, width) = buffer;
        let mut out = Vec::new();
        for y in area.y.ceil() as u32..(area.y + area.height).floor() as u32 {
            for x in area.x.ceil() as u32..(area.x + area.width).floor() as u32 {
                let i = ((y * width + x) * 4) as usize;
                out.push([pixels[i], pixels[i + 1], pixels[i + 2]]);
            }
        }
        out
    }

    fn backdrop() -> iced::Color {
        crate::ui::material::style::color(roles().surface)
    }

    /// FR-010, R8: a grid created with a card expanded draws its shadow at full on its first
    /// frame, and the pixels below and beside it are those of the card casting the menu shadow
    /// itself (feature 1121).
    #[test]
    fn an_expanded_card_at_rest_paints_the_shadow_it_cast_itself() {
        let models: Vec<CardModel> = (0..1).map(titled).collect();
        let below = || Space::new().height(200.0);
        let mut grid = Mounted::new_in(
            column![lift_grid(&models, Some(0), GAP).unscrolled(), below()].width(Length::Fill),
            Size::new(900.0, 4000.0),
        );
        let slot = grid.node.children()[0].children()[0].bounds();
        let card = OverviewCard::<String>::new(&models[0], roles())
            .on_press("open 0".to_string())
            .on_secondary_press(|_| "menu 0".to_string())
            .expanded(true);
        let mut own = Mounted::new_in(
            column![container(card).width(Length::Fixed(slot.width)), below()].width(Length::Fill),
            Size::new(900.0, 4000.0),
        );
        // The grid's row is the collapsed twin's; the expanded card is drawn over what follows.
        assert!(own.node.size().height >= grid.node.size().height);
        let card = own.node.children()[0].bounds();
        let (drawn, by_card) = (grid.drawn(backdrop()), own.drawn(backdrop()));
        let shadowed = Rectangle {
            y: card.y + card.height,
            height: 10.0,
            ..card
        };
        let plain = [backdrop().r, backdrop().g, backdrop().b].map(|c| (c * 255.0).round() as u8);
        assert!(
            pixels_in(&drawn, shadowed).iter().any(|p| *p != plain),
            "no shadow below the card on the first frame"
        );
        assert!(
            drawn.0 == by_card.0[..drawn.0.len()],
            "the grid's shadow is not the card's own"
        );
    }

    /// The 3×3 grid of the draw-order tests: cards with no gap, so a shadow reaches the next card.
    const TIGHT: f32 = 0.0;

    fn nine() -> Vec<CardModel> {
        (0..9).map(titled).collect()
    }

    /// FR-003 draw order, SC-005, US2-5: collapsing the middle card of a 3×3 grid, part-way its
    /// shadow shows over the card below it and the card to its right, it is drawn after every
    /// other card, and on every tick of the expand and the collapse every slot is where it is with
    /// nothing expanded.
    #[test]
    fn a_settling_card_is_drawn_over_its_neighbours_and_nothing_moves() {
        let models = nine();
        let mut m = mounted(lift_grid(&models, None, TIGHT), 900.0, 4000.0);
        let rest_slots = slots(&m.node, 9);
        let at_rest = m.drawn(backdrop());
        m.rebuild(lift_grid(&models, Some(4), TIGHT).unscrolled());
        let mut ms = 0;
        while ms <= DONE {
            tick_at(&mut m, ms);
            // The expanded card's own node grows at once (feature 1121, D7); no other moves.
            let (now, mut want) = (slots(&m.node, 9), rest_slots.clone());
            want[4] = now[4];
            assert_eq!(now, want, "expanding, at {ms} ms");
            ms += FRAME_MS;
        }
        assert_eq!(strength_of(&m, &models, 4, true), 1.0);
        m.rebuild(lift_grid(&models, None, TIGHT).unscrolled());
        let start = ms + 100;
        let mut ms = start;
        while ms <= start + 2 * FRAME_MS {
            tick_at(&mut m, ms);
            assert_eq!(slots(&m.node, 9), rest_slots, "collapsing, at {ms} ms");
            ms += FRAME_MS;
        }
        let mid = strength_of(&m, &models, 4, false);
        assert!(0.0 < mid && mid < 1.0, "{mid}");
        let settling = m.drawn(backdrop());
        for neighbour in [5, 7] {
            assert_ne!(
                pixels_in(&settling, rest_slots[neighbour]),
                pixels_in(&at_rest, rest_slots[neighbour]),
                "card 4's shadow is cut by card {neighbour}"
            );
        }
        let painted = m.painted_text();
        let title = painted
            .iter()
            .position(|p| p.content == models[4].title)
            .expect("card 4's title");
        let last_other = painted
            .iter()
            .rposition(|p| (0..9).any(|i| i != 4 && p.content == models[i].title))
            .expect("the other titles");
        assert!(title > last_other, "{title} drawn before {last_other}");
        while ms <= start + 2 * DONE {
            tick_at(&mut m, ms);
            assert_eq!(slots(&m.node, 9), rest_slots, "collapsing, at {ms} ms");
            ms += FRAME_MS;
        }
        assert_eq!(strength_of(&m, &models, 4, false), 0.0);
        assert!(
            m.drawn(backdrop()).0 == at_rest.0,
            "the settled grid is not the rest grid"
        );
    }

    /// A new rest scope (another project) drops the old set's lifts: a card of the new set at the
    /// old expanded card's location fades no shadow and asks for no frame.
    #[test]
    fn a_new_rest_scope_drops_the_old_sets_lifts() {
        let models = nine();
        let mut m = mounted(
            lift_grid(&models, Some(4), GAP).rest_scope("/a"),
            900.0,
            4000.0,
        );
        assert_eq!(strength_of(&m, &models, 4, true), 1.0, "seeded lifted");
        m.rebuild(lift_grid(&models, None, GAP).rest_scope("/b").unscrolled());
        assert_eq!(tick_at(&mut m, 0), RedrawRequest::Wait);
        assert_eq!(strength_of(&m, &models, 4, false), 0.0);
        assert_eq!(lifts(&m).settling(None).count(), 0);
    }

    /// C3, R8: while a zoom runs a collapsing card's shadow ends at once, asking for no frame.
    #[test]
    fn a_zoom_ends_the_lifts_at_once() {
        let models = nine();
        let mut m = mounted(lift_grid(&models, Some(4), GAP), 900.0, 4000.0);
        assert_eq!(strength_of(&m, &models, 4, true), 1.0, "seeded lifted");
        m.rebuild(lift_grid(&models, None, GAP).unscrolled());
        m.tree.state.downcast_mut::<ZoomState>().ends = Some((900.0, 1200.0));
        assert_eq!(tick_at(&mut m, 0), RedrawRequest::Wait);
        assert_eq!(strength_of(&m, &models, 4, false), 0.0);
        assert_eq!(lifts(&m).settling(None).count(), 0);
    }

    /// US2-4, SC-001: Tab onto a card of a default grid, which the application answers by
    /// expanding it: at ~75 ms its ring and its lift are both part-way, and both full by 150 ms
    /// and a frame.
    #[test]
    fn focus_and_lift_ease_in_together() {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        let mut m = mounted(lift_grid(&models, None, GAP), 900.0, 4000.0);
        m.focus_next();
        m.rebuild(lift_grid(&models, Some(0), GAP).unscrolled());
        let mut read: Vec<(u64, (f32, f32))> = Vec::new();
        for ms in (0..=DONE).step_by(FRAME_MS as usize) {
            tick_at(&mut m, ms);
            read.push((
                ms,
                (strengths(&m.tree)[0], strength_of(&m, &models, 0, true)),
            ));
        }
        let at_ms = |ms: u64| read.iter().find(|(t, _)| *t == ms).expect("a tick").1;
        let (ring, lift) = at_ms(MIDWAY);
        assert!(0.0 < ring && ring < 1.0, "ring {ring} at ~75 ms");
        assert!(0.0 < lift && lift < 1.0, "lift {lift} at ~75 ms");
        assert_eq!(
            at_ms(DONE),
            (1.0, 1.0),
            "ring and lift by 150 ms and a frame"
        );
    }

    /// **The Reduce motion test** (FR-012, SC-002, US3-1, US3-2): in an `instant` grid, on the
    /// first tick after gaining and after losing the keyboard focus the card's ring is full / none,
    /// and after expanding and collapsing (by hover, by focus, by Escape: each is the application
    /// rebuilding the grid with or without the card expanded) its shadow is full / none, each
    /// tick asking for no next frame.
    #[test]
    fn reduce_motion_makes_card_focus_and_expansion_instant() {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        let build = |index| lift_grid(&models, index, GAP).instant(true).unscrolled();
        let mut m = mounted(lift_grid(&models, None, GAP).instant(true), 900.0, 4000.0);
        let mut ms = 0;
        let mut first_tick = |m: &mut Mounted<'_>| {
            let asked = tick_at(m, ms);
            ms += FRAME_MS;
            asked
        };
        // Focus onto card 0, and the application expands it (focus).
        m.focus_next();
        m.rebuild(build(Some(0)));
        assert_eq!(first_tick(&mut m), RedrawRequest::Wait);
        assert_eq!(
            strengths(&m.tree)[0],
            1.0,
            "the ring after gaining the focus"
        );
        assert_eq!(
            strength_of(&m, &models, 0, true),
            1.0,
            "the shadow after expanding"
        );
        let drawn_lifted = m.drawn(backdrop());
        // Escape: collapsed, the focus kept.
        m.rebuild(build(None));
        assert_eq!(first_tick(&mut m), RedrawRequest::Wait);
        assert_eq!(
            strength_of(&m, &models, 0, false),
            0.0,
            "the shadow after Escape"
        );
        assert_eq!(lifts(&m).settling(None).count(), 0);
        // Hover expands card 1; leaving collapses it.
        m.rebuild(build(Some(1)));
        assert_eq!(first_tick(&mut m), RedrawRequest::Wait);
        assert_eq!(
            strength_of(&m, &models, 1, true),
            1.0,
            "the shadow after a hover"
        );
        m.rebuild(build(None));
        assert_eq!(first_tick(&mut m), RedrawRequest::Wait);
        assert_eq!(
            strength_of(&m, &models, 1, false),
            0.0,
            "the shadow after leaving"
        );
        // Focus away from card 0 (to card 1): its ring is none at once.
        m.focus_next();
        assert_eq!(first_tick(&mut m), RedrawRequest::Wait);
        assert_eq!(
            strengths(&m.tree)[..2],
            [0.0, 1.0],
            "the ring after losing the focus"
        );
        // The first drawn frame of an instant expansion is the full shadow: what a seeded,
        // lifted grid draws.
        let mut seeded = mounted(lift_grid(&models, Some(0), GAP), 900.0, 4000.0);
        seeded.focus_next();
        seeded.frames(0, DONE, strengths);
        assert!(
            drawn_lifted.0 == seeded.drawn(backdrop()).0,
            "an instant lift is not full"
        );
    }

    /// FR-008, US3-3: a grid mid-lift rebuilt `instant` (Reduce motion saved) draws the end
    /// shadow on the next tick, and asks for no frame.
    #[test]
    fn reduce_motion_saved_mid_lift_ends_it_on_the_next_tick() {
        let models: Vec<CardModel> = (0..3).map(titled).collect();
        let mut m = mounted(lift_grid(&models, None, GAP), 900.0, 4000.0);
        m.rebuild(lift_grid(&models, Some(0), GAP).unscrolled());
        let mut ms = 0;
        while ms <= MIDWAY {
            tick_at(&mut m, ms);
            ms += FRAME_MS;
        }
        assert!(strength_of(&m, &models, 0, true) < 1.0);
        m.rebuild(lift_grid(&models, Some(0), GAP).instant(true).unscrolled());
        assert_eq!(tick_at(&mut m, ms), RedrawRequest::Wait);
        assert_eq!(strength_of(&m, &models, 0, true), 1.0);
    }
}

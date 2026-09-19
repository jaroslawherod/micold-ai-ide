//! `terminal_pane` — the reusable colour terminal widget (Constitution Principle VIII, feature
//! 006). A custom iced `advanced::Widget` that renders a session's `alacritty_terminal` grid on
//! a canvas with full ANSI colour + text styling, and (US1) focuses on click.
//!
//! Adapted from `iced_term 0.6.0` `view.rs` (MIT © Ilya Shvyryalkin). Key/mouse input and the
//! full focus gate land in feature 006 US2/US3; this file covers colour rendering + click focus.

use crate::app::{route_key, KeyRouting, Message};
use crate::features::session::Msg as SessionMsg;
use crate::features::session::SelectKind;
use crate::grid::GridCache;
use crate::keymap;
use crate::selection::Selection;
use crate::ui::terminal::{
    cell_font, encode_mouse_report, shows_cursor, wire_cell_colors, CellMetrics, TermPalette,
    TERM_FONT_SIZE,
};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::NamedColor;
use iced::advanced::clipboard::Kind as ClipboardKind;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::{click, Click};
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{
    alignment, keyboard, mouse, Color, Element, Event, Length, Point, Rectangle, Renderer, Size,
    Theme,
};
use micold_core::link::{LinkContext, LinkRows, ResolvedLink};
use micold_core::protocol::grid::{LineId, WireColor, WireStyle};
use micold_core::session::SessionId;
use micold_core::tokens::state::FOCUS_RING_WIDTH;

/// Reports the terminal area's size in *cells* to the app, whatever is currently drawn in it
/// (BUG-003, FR-014a).
///
/// [`TerminalPane`] already reports its own size, and for a long time that looked sufficient: the
/// pane is the terminal area. But the pane is only mounted while a session is displayed **and** its
/// first grid frame has arrived — before that the same rectangle holds an empty state or a
/// "Starting…" placeholder, and nothing measures it. So on a cold start the app knew no size at the
/// one moment it most needs one: the first session the user clicks is started before any pane has
/// ever been laid out, and is therefore spawned at the service's default (`010` FR-020a) and
/// corrected a frame later, with its first output laid out for the wrong screen.
///
/// This wraps the terminal area itself rather than its contents, so the measurement exists from the
/// first frame after launch and does not depend on what is inside. Reporting is deduplicated per
/// instance: only a *change* in the computed `(cols, rows)` publishes, so a steady window is silent.
pub struct GridSizeReporter<'a, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

/// The last `(cols, rows)` this instance published, so an unchanged size stays off the wire.
#[derive(Default)]
struct ReporterState {
    last_grid: (u16, u16),
}

impl<'a> GridSizeReporter<'a> {
    /// Wrap the element that occupies the terminal area.
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl<Theme, Renderer> Widget<Message, Theme, Renderer> for GridSizeReporter<'_, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ReporterState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ReporterState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The character area, not the whole rectangle: the pane keeps a focus-ring gutter on every
        // side at every focus state (FR-010b), so that is the size the process is given.
        let content = content_bounds(layout.bounds());
        let grid = CellMetrics::new(TERM_FONT_SIZE).grid_size(content.width, content.height);
        let state = tree.state.downcast_mut::<ReporterState>();
        if grid != state.last_grid {
            state.last_grid = grid;
            shell.publish(Message::Session(SessionMsg::TerminalResized {
                cols: grid.0,
                rows: grid.1,
            }));
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<GridSizeReporter<'a>> for Element<'a, Message> {
    fn from(r: GridSizeReporter<'a>) -> Self {
        Element::new(r)
    }
}

/// Per-widget interaction state (drag + tracked modifiers + click cadence for single/double/
/// triple selection).
#[derive(Default)]
struct PaneState {
    dragging: bool,
    /// The viewport cell a local left press landed in, until the pointer first leaves it. While
    /// set, motion is still a click and extends nothing (FR-013e, BUG-008): the pane decides this
    /// in screen cells because a selection's `LineId` anchors drift under streaming output.
    press_cell: Option<(u16, u16)>,
    modifiers: keyboard::Modifiers,
    last_click: Option<Click>,
    /// While dragging the scrollbar thumb: the cursor's offset below the thumb's top edge, so the
    /// grabbed point stays under the pointer (FR-016).
    scrollbar_grab: Option<f32>,
    /// The mouse button currently held down and being reported to the process (FR-013a), or
    /// `None` when no reported button is down. Held so the matching *release* can be encoded —
    /// without it the process sees a button go down and never come up.
    reporting_button: Option<u8>,
    /// The last grid cell reported to the process, so motion reports are emitted once per cell
    /// crossed rather than once per pixel of pointer movement.
    reported_cell: Option<(u16, u16)>,
    /// Sub-line wheel travel not yet turned into a scrolled line, in pixels. High-resolution
    /// touchpads deliver deltas smaller than one cell, which would otherwise round away to nothing
    /// (BUG-002). See [`wheel_lines`].
    scroll_residual: f32,
    /// The link under the pointer (feature 031, research R2).
    hover: Option<HoverCache>,
    /// A Ctrl/Cmd press on a link awaiting its release (research R6).
    link_press: Option<LinkPress>,
}

/// The pane's character area: `bounds` less a gutter of the 018 focus ring's width on every side
/// (FR-010b, BUG-005).
///
/// Reserved whether or not the pane is focused. Drawn over the cells, the ring would clip the first
/// and last columns; inset only while focused, the character area — and with it the size reported
/// to the process — would change every time focus moved, reflowing the shell's output.
pub(crate) fn content_bounds(bounds: Rectangle) -> Rectangle {
    let inset = FOCUS_RING_WIDTH;
    Rectangle {
        x: bounds.x + inset,
        y: bounds.y + inset,
        width: (bounds.width - 2.0 * inset).max(0.0),
        height: (bounds.height - 2.0 * inset).max(0.0),
    }
}

/// The grid cell (col, line) under a cursor position within the character area `bounds`.
fn grid_at(pos: Point, bounds: Rectangle, metrics: CellMetrics) -> (u16, u16) {
    let col = ((pos.x - bounds.x) / metrics.width).floor().max(0.0) as u16;
    let line = ((pos.y - bounds.y) / metrics.height).floor().max(0.0) as u16;
    (col, line)
}

/// Who consumes a mouse-button press over the pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressRouting {
    /// Forward the press to the process as a mouse report (FR-013a).
    MouseReport,
    /// Handle it in the pane — left-drag selects text, right-click opens the context menu
    /// (FR-013 / FR-013b).
    HandleLocally,
}

/// Route a button press between the process and the pane's own gestures.
///
/// Mouse reports are process *input*, so they are only produced for a focused pane whose process
/// has enabled mouse reporting. Holding Shift overrides that, which is what keeps selection and
/// copy reachable under a full-screen program that owns the mouse (FR-013b). Every other
/// combination — including any press on an unfocused pane — is handled locally.
/// Whether this press is the one that makes the pane the keyboard holder (FR-008b).
///
/// Pure, and tested, because it is a rule rather than wiring: Principle I's GUI-wiring exception
/// covers glue with no decision of its own, and "which press takes the keyboard" is a decision. Its
/// answer is also the argument [`press_routing`] needs — routing the granting press on the previous
/// view's `false` is what stopped a mouse-aware program ever seeing it (research R5).
///
/// Which button is **not** part of the decision. FR-007 puts the pane's own context menu in the
/// same class as its scrollbar and status bar — furniture, which leaves the terminal holding the
/// keyboard, "giving it the keyboard, per FR-008b, if it did not already hold it" — so a
/// right-click on an unfocused pane takes it exactly as a left one does.
pub(crate) fn press_grants_focus(focused: bool, over_bounds: bool) -> bool {
    !focused && over_bounds
}

pub(crate) fn press_routing(focused: bool, mouse_mode: bool, shift: bool) -> PressRouting {
    if focused && mouse_mode && !shift {
        PressRouting::MouseReport
    } else {
        PressRouting::HandleLocally
    }
}

/// Selection granularity for a click cadence: one click selects characters, two the word under
/// the pointer, three the whole line (FR-013).
///
/// Factored out of `on_event` for the same reason as [`press_routing`] and [`wheel_routing`] —
/// `on_event` takes a concrete `&iced::Renderer` that needs a GPU device, so anything left inline
/// there cannot be unit-tested (T057).
pub(crate) fn select_kind(kind: click::Kind) -> SelectKind {
    match kind {
        click::Kind::Single => SelectKind::Simple,
        click::Kind::Double => SelectKind::Semantic,
        click::Kind::Triple => SelectKind::Lines,
    }
}

/// What a wheel event does once [`wheel_lines`] has resolved it to whole lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WheelRouting {
    /// Forward the scroll to the process as wheel reports (FR-013a) — `count` reports of
    /// `button`, one per line.
    MouseReport { button: u8, count: u32 },
    /// Move the pane's own scrollback view by `lines` (FR-016).
    ScrollLocally { lines: i32 },
    /// Nothing to do: the travel so far is under one line and stays banked in the residual.
    Ignore,
}

/// Route a wheel scroll between the process and the pane's own scrollback.
///
/// Mirrors [`press_routing`] for the wheel: reports are process *input*, so only a focused pane
/// over a mouse-reporting process produces them; everything else scrolls our own view. `lines`
/// is the already-accumulated whole-line count from [`wheel_lines`], so a gesture made entirely
/// of sub-line touchpad deltas routes to [`WheelRouting::Ignore`] until the residual crosses a
/// cell — which is the BUG-002 case, and why this is factored out where it can be tested.
pub(crate) fn wheel_routing(lines: i32, focused: bool, mouse_mode: bool) -> WheelRouting {
    if lines == 0 {
        WheelRouting::Ignore
    } else if focused && mouse_mode {
        WheelRouting::MouseReport {
            // Wheel-up is button 64, wheel-down 65, one report per line travelled.
            button: if lines > 0 { 64 } else { 65 },
            count: lines.unsigned_abs(),
        }
    } else {
        WheelRouting::ScrollLocally { lines }
    }
}

/// Whole lines of scrollback for a wheel `delta`, carrying the sub-line remainder in `residual`.
///
/// Discrete wheels (and X11 touchpads, which arrive as legacy button-4/5 events) deliver
/// [`ScrollDelta::Lines`] and pass straight through. High-resolution touchpads — the norm under
/// Wayland, and on macOS/Windows precision devices — deliver [`ScrollDelta::Pixels`] in increments
/// far smaller than one cell. Quantizing each event on its own would round every one of them to
/// zero and scrolling would never happen at all (BUG-002), so the fraction is retained here until
/// it accumulates into a line. Reversing direction drops the stale residual so an interrupted
/// gesture cannot cancel out the new one.
///
/// [`ScrollDelta::Lines`]: mouse::ScrollDelta::Lines
/// [`ScrollDelta::Pixels`]: mouse::ScrollDelta::Pixels
pub(crate) fn wheel_lines(delta: mouse::ScrollDelta, cell_height: f32, residual: &mut f32) -> i32 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => {
            *residual = 0.0;
            y.round() as i32
        }
        mouse::ScrollDelta::Pixels { y, .. } => {
            if cell_height <= 0.0 {
                return 0;
            }
            if y != 0.0 && *residual != 0.0 && residual.signum() != y.signum() {
                *residual = 0.0;
            }
            *residual += y;
            let lines = (*residual / cell_height).trunc();
            *residual -= lines * cell_height;
            lines as i32
        }
    }
}

// ---- Links (feature 031): hover, the Ctrl/Cmd+click gesture and the address hint ----

/// The rows `micold_core::link` reads, lent by the grid cache (contract link-recognition §1).
///
/// `row` is relative to the viewport's top line as drawn, so the scrollback offset is part of the
/// mapping. Every row asked for is recorded, so the hover can later tell whether the rows its link
/// was read from changed (research R2).
pub(crate) struct GridRows<'g> {
    grid: &'g GridCache,
    display_offset: usize,
    consulted: std::cell::RefCell<Vec<i64>>,
}

impl<'g> GridRows<'g> {
    pub(crate) fn new(grid: &'g GridCache, display_offset: usize) -> Self {
        Self {
            grid,
            display_offset,
            consulted: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn id(&self, row: i64) -> LineId {
        LineId(self.grid.viewport_top().0 - self.display_offset as i64 + row)
    }

    fn line(&self, row: i64) -> Option<&'g crate::grid::CachedLine> {
        self.consulted.borrow_mut().push(row);
        self.grid.line(self.id(row))
    }

    /// Whether `row` lies above everything the terminal holds or ever held: above the first line it
    /// printed, or above a screen with no history at all (the alternate screen of `less` or `vim`).
    /// Nothing can continue into such a row, so it reads as an empty one (contract L7).
    fn above_everything(&self, row: i64) -> bool {
        let id = self.id(row).0;
        let oldest = self.grid.oldest_available().0;
        id < oldest && (id < 0 || oldest == self.grid.viewport_top().0)
    }

    /// The rows read so far, sorted and without repeats.
    fn consulted(&self) -> Vec<i64> {
        let mut rows = self.consulted.borrow().clone();
        rows.sort_unstable();
        rows.dedup();
        rows
    }
}

impl LinkRows for GridRows<'_> {
    fn text(&self, row: i64) -> Option<&str> {
        match self.line(row) {
            Some(line) => Some(line.text.as_str()),
            None if self.above_everything(row) => Some(""),
            None => None,
        }
    }

    fn wrapped(&self, row: i64) -> bool {
        self.line(row).is_some_and(|line| line.wrapped)
    }

    fn hyperlink(&self, row: i64, col: u16) -> Option<&str> {
        self.line(row)?
            .extras
            .iter()
            .find(|extra| extra.col == col)?
            .hyperlink
            .as_deref()
    }

    fn spacer(&self, row: i64, col: u16) -> bool {
        self.line(row)
            .and_then(|line| style_at(line, col))
            .is_some_and(|style| {
                Flags::from_bits_truncate(style.flags)
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            })
    }
}

/// The style of the cell at `col`, from the line's run-length style runs.
fn style_at(line: &crate::grid::CachedLine, col: u16) -> Option<WireStyle> {
    let mut start = 0u16;
    for (len, style) in &line.runs {
        if col < start.saturating_add(*len) {
            return Some(*style);
        }
        start = start.saturating_add(*len);
    }
    None
}

/// The link under the pointer, and what it was resolved from (data-model §2, research R2).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HoverCache {
    pub session: Option<SessionId>,
    pub context: LinkContext,
    pub cell: (u16, u16),
    pub display_offset: usize,
    pub grid_version: (u64, u64),
    /// The rows `link_at` read, relative to the viewport top.
    pub rows: Vec<i64>,
    pub rows_hash: u64,
    pub resolved: Option<ResolvedLink>,
}

/// What a hover is asked about: the pane's inputs at this event.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HoverKey<'k> {
    pub session: Option<SessionId>,
    pub context: &'k LinkContext,
    pub cell: (u16, u16),
    pub display_offset: usize,
    pub grid_version: (u64, u64),
}

/// Whether a cached hover still answers `key` (research R2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HoverRefresh {
    /// Nothing it depends on moved.
    Reuse,
    /// The grid moved but the rows the link was read from did not: keep it at the new version.
    Revalidated,
    /// Run `link_at` again.
    Resolve,
}

/// Decide whether `cached` still answers `key`. `rows_hash` hashes the given rows of the current
/// grid, and is called only when the grid version moved.
pub(crate) fn hover_refresh(
    cached: Option<&HoverCache>,
    key: &HoverKey<'_>,
    rows_hash: impl FnOnce(&[i64]) -> u64,
) -> HoverRefresh {
    let Some(cached) = cached else {
        return HoverRefresh::Resolve;
    };
    if cached.session != key.session
        || cached.cell != key.cell
        || cached.display_offset != key.display_offset
        || &cached.context != key.context
    {
        return HoverRefresh::Resolve;
    }
    if cached.grid_version == key.grid_version {
        HoverRefresh::Reuse
    } else if rows_hash(&cached.rows) == cached.rows_hash {
        HoverRefresh::Revalidated
    } else {
        HoverRefresh::Resolve
    }
}

/// Hash of `rows` as `grid` holds them now: text, soft wrap, declared links and cell styles.
fn rows_hash(grid: &GridCache, display_offset: usize, rows: &[i64]) -> u64 {
    use std::hash::{Hash, Hasher};
    let lent = GridRows::new(grid, display_offset);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for &row in rows {
        row.hash(&mut hasher);
        match lent.line(row) {
            Some(line) => {
                line.text.hash(&mut hasher);
                line.wrapped.hash(&mut hasher);
                line.runs.hash(&mut hasher);
                for extra in &line.extras {
                    (extra.col, &extra.hyperlink).hash(&mut hasher);
                }
            }
            None => lent.above_everything(row).hash(&mut hasher),
        }
    }
    hasher.finish()
}

/// Resolve the link at `cell` from the grid, as a fresh [`HoverCache`] for `key`.
pub(crate) fn resolve_hover(grid: &GridCache, key: &HoverKey<'_>) -> HoverCache {
    let lent = GridRows::new(grid, key.display_offset);
    let (col, row) = key.cell;
    let resolved = if col < grid.cols() && row < grid.rows() {
        micold_core::link::line::link_at(&lent, row as i64, col)
            .and_then(|link| micold_core::link::resolve::resolve(link, key.context))
    } else {
        None
    };
    let rows = lent.consulted();
    HoverCache {
        session: key.session,
        context: key.context.clone(),
        cell: key.cell,
        display_offset: key.display_offset,
        grid_version: key.grid_version,
        rows_hash: rows_hash(grid, key.display_offset, &rows),
        rows,
        resolved,
    }
}

/// The link the pane marks: the hovered one, except under mouse reporting without Shift, where the
/// pointer belongs to the program (FR-016).
pub(crate) fn marked_link(
    hover: Option<&HoverCache>,
    mouse_mode: bool,
    shift: bool,
) -> Option<&ResolvedLink> {
    if mouse_mode && !shift {
        return None;
    }
    hover?.resolved.as_ref()
}

/// The pointer over the pane: a hand over a marked link only while the link modifier is held, so it
/// shows exactly when a click would open it (FR-007, clarification 2026-09-16).
pub(crate) fn pane_interaction(
    over: bool,
    marked: bool,
    modifiers: keyboard::Modifiers,
) -> mouse::Interaction {
    if !over {
        mouse::Interaction::Idle
    } else if marked && modifiers.command() {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::Text
    }
}

/// A Ctrl/Cmd press on a link, waiting for its release (research R6).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinkPress {
    pub link: ResolvedLink,
    pub cell: (u16, u16),
}

/// What the pane saw, as far as the link gesture cares (contract link-opening §1).
#[derive(Clone, Copy, Debug)]
pub(crate) enum GestureEvent<'a> {
    LeftPress {
        cell: (u16, u16),
        command: bool,
        routing: PressRouting,
        cadence: click::Kind,
        marked: Option<&'a ResolvedLink>,
    },
    CursorMoved {
        cell: (u16, u16),
    },
    LeftRelease {
        cell: (u16, u16),
        marked: Option<&'a ResolvedLink>,
    },
}

/// What the gesture does with an event.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GestureStep {
    /// Not the gesture's event: today's handling runs.
    PassThrough,
    /// A link press begins; no selection starts.
    Press(LinkPress),
    /// The pointer left the press cell: the press becomes a selection from that cell.
    SelectFrom { col: u16, line: u16 },
    /// The press ends; the link under the pointer at release, if any, opens.
    Release(Option<ResolvedLink>),
}

/// The link gesture's state machine (contract link-opening §1, G1–G9).
///
/// Only a single left press with the link modifier, handled by the pane and over a marked link,
/// starts it; a double or triple press selects as it always did (G3b). No step writes to the
/// program or scrolls (FR-014).
pub(crate) fn link_gesture(event: GestureEvent<'_>, pressed: Option<&LinkPress>) -> GestureStep {
    match (event, pressed) {
        (
            GestureEvent::LeftPress {
                cell,
                command: true,
                routing: PressRouting::HandleLocally,
                cadence: click::Kind::Single,
                marked: Some(link),
            },
            _,
        ) => GestureStep::Press(LinkPress {
            link: link.clone(),
            cell,
        }),
        (GestureEvent::CursorMoved { cell }, Some(press)) if cell != press.cell => {
            GestureStep::SelectFrom {
                col: press.cell.0,
                line: press.cell.1,
            }
        }
        (GestureEvent::LeftRelease { cell, marked }, Some(press)) => {
            GestureStep::Release(marked.filter(|_| cell == press.cell).cloned())
        }
        _ => GestureStep::PassThrough,
    }
}

/// Inner padding of the address hint, in pixels.
const HINT_PADDING: f32 = 4.0;

/// Where the address hint sits: bottom-left of the content, or top-left when the pointer's row
/// is within the hint's height of the bottom edge, so it never covers the link (research R7).
pub(crate) fn link_hint_rect(content: Rectangle, pointer_row: u16, hint_size: Size) -> Rectangle {
    let size = Size::new(
        hint_size.width.min(content.width),
        hint_size.height.min(content.height),
    );
    let metrics = CellMetrics::new(TERM_FONT_SIZE);
    let bottom = content.y + content.height;
    let row_bottom = content.y + (pointer_row as f32 + 1.0) * metrics.height;
    let y = if row_bottom > bottom - size.height {
        content.y
    } else {
        bottom - size.height
    };
    Rectangle::new(Point::new(content.x, y), size)
}

/// `text` cut to at most `max_chars` chars by replacing its middle with `…` (FR-008).
pub(crate) fn elide_middle(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    let kept = max_chars - 1;
    let tail = kept / 2;
    let head = kept - tail;
    let mut label: String = text.chars().take(head).collect();
    label.push('…');
    label.extend(text.chars().skip(count - tail));
    label
}

/// Width of the scrollback scrollbar track/thumb, in pixels.
const SCROLLBAR_WIDTH: f32 = 10.0;
/// Smallest thumb height so it stays grabbable over a deep history (FR-016).
const MIN_THUMB_HEIGHT: f32 = 24.0;

/// Geometry of the scrollback scrollbar thumb within its track (both measured from the pane top).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scrollbar {
    pub thumb_top: f32,
    pub thumb_height: f32,
}

/// The thumb height used for a given track/viewport/history — proportional to the visible fraction,
/// clamped to a grabbable minimum and to the track itself.
fn thumb_height(track_height: f32, screen_lines: usize, history_size: usize) -> f32 {
    let total = (history_size + screen_lines).max(1) as f32;
    let proportional = track_height * screen_lines as f32 / total;
    proportional.clamp(MIN_THUMB_HEIGHT.min(track_height), track_height)
}

/// Thumb geometry for the scrollback scrollbar, or `None` when it should be hidden — parked at the
/// live bottom (`display_offset == 0`), no history, or a degenerate track. The thumb is pinned to
/// the top when fully scrolled up and to the bottom when barely scrolled (feature 006, FR-016).
pub(crate) fn scrollbar_metrics(
    track_height: f32,
    screen_lines: usize,
    history_size: usize,
    display_offset: usize,
) -> Option<Scrollbar> {
    if history_size == 0 || display_offset == 0 || track_height <= 0.0 {
        return None;
    }
    let thumb_height = thumb_height(track_height, screen_lines, history_size);
    let travel = track_height - thumb_height;
    // frac: 1.0 just off the bottom, 0.0 at the very top of the history.
    let frac = (history_size.saturating_sub(display_offset)) as f32 / history_size as f32;
    Some(Scrollbar {
        thumb_top: travel * frac,
        thumb_height,
    })
}

/// The `display_offset` that places the thumb's top edge at `thumb_top` px — the inverse of
/// [`scrollbar_metrics`], used while dragging. Clamped to `[0, history_size]`.
pub(crate) fn offset_for_thumb_top(
    track_height: f32,
    screen_lines: usize,
    history_size: usize,
    thumb_top: f32,
) -> usize {
    if history_size == 0 {
        return 0;
    }
    let travel = track_height - thumb_height(track_height, screen_lines, history_size);
    if travel <= 0.0 {
        return history_size;
    }
    let frac = (thumb_top / travel).clamp(0.0, 1.0);
    (history_size as f32 * (1.0 - frac)).round() as usize
}

/// The relative delta to reach an absolute scrollback `target` from the `current` offset. Computed
/// at apply time against the live offset so that a burst of drag messages converges on the target
/// rather than accumulating stale relative deltas (drag flicker fix, FR-016).
pub fn target_offset_delta(current: usize, target: usize) -> i32 {
    target as i32 - current as i32
}

/// The colour terminal widget for a live session runtime (Principle VIII builder form):
/// `TerminalPane::new(rt, palette).focused(bool).into()`.
pub struct TerminalPane<'a> {
    grid: &'a GridCache,
    selection: Option<&'a Selection>,
    display_offset: usize,
    palette: TermPalette,
    focused: bool,
    session: Option<SessionId>,
    link_context: LinkContext,
}

/// What a link resolves against when the caller names no context: this machine, unsandboxed.
pub(crate) fn local_link_context() -> LinkContext {
    LinkContext {
        host_names: Vec::new(),
        windows_host: cfg!(windows),
        sandbox: None,
    }
}

/// The style used for a cell not covered by the line's runs (a protocol violation that must never
/// panic the renderer): default fg/bg, no flags.
const DEFAULT_STYLE: WireStyle = WireStyle {
    fg: WireColor::Named(NamedColor::Foreground as u16),
    bg: WireColor::Named(NamedColor::Background as u16),
    flags: 0,
    underline_color: None,
};

impl<'a> TerminalPane<'a> {
    /// A terminal pane rendering `grid` (a session's daemon-streamed cache) with `palette`.
    /// Unfocused, live (no scrollback), and unselected by default.
    pub fn new(grid: &'a GridCache, palette: TermPalette) -> Self {
        Self {
            grid,
            selection: None,
            display_offset: 0,
            palette,
            focused: false,
            session: None,
            link_context: local_link_context(),
        }
    }

    /// The session this pane shows: a hover never outlives a switch to another one (FR-007).
    pub fn session(mut self, session: SessionId) -> Self {
        self.session = Some(session);
        self
    }

    /// What the pane's links resolve against (FR-012, FR-018).
    pub fn link_context(mut self, context: LinkContext) -> Self {
        self.link_context = context;
        self
    }

    /// The active text selection to highlight and copy from (client-side, `LineId`-anchored).
    pub fn selection(mut self, selection: Option<&'a Selection>) -> Self {
        self.selection = selection;
        self
    }

    /// How many lines the view is scrolled up into the scrollback (0 = live bottom).
    pub fn display_offset(mut self, display_offset: usize) -> Self {
        self.display_offset = display_offset;
        self
    }

    /// Mark the pane focused: keyboard input is routed to it, and it draws the 018 focus ring
    /// (FR-010, FR-010b).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    // ---- terminal-mode-derived helpers, reconstructed from the cache's `mode()` bits so the
    // daemon-rendered pane reproduces the local pane's mouse/key behaviour exactly ----

    fn mode(&self) -> TermMode {
        TermMode::from_bits_truncate(self.grid.mode())
    }

    fn mouse_mode(&self) -> bool {
        self.mode().intersects(TermMode::MOUSE_MODE)
    }

    fn mouse_motion_mode(&self) -> bool {
        self.mode()
            .intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
    }

    fn mouse_report_bytes(
        &self,
        button: u8,
        col: u16,
        line: u16,
        pressed: bool,
        mods: keymap::Mods,
    ) -> Option<Vec<u8>> {
        encode_mouse_report(self.mode(), button, col, line, pressed, mods)
    }

    fn key_term_mode(&self) -> keymap::TermMode {
        let m = self.mode();
        keymap::TermMode {
            app_cursor: m.contains(TermMode::APP_CURSOR),
            alt_screen: m.contains(TermMode::ALT_SCREEN),
        }
    }

    /// Visible grid size in cells (cols, rows).
    fn size(&self) -> (u16, u16) {
        (self.grid.cols(), self.grid.rows())
    }

    /// Retained scrollback depth: lines below the viewport top still held in the cache.
    fn history_size(&self) -> usize {
        (self.grid.viewport_top().0 - self.grid.oldest_available().0).max(0) as usize
    }

    /// The absolute `LineId` shown at rendered viewport `row`, accounting for scrollback.
    fn line_at_row(&self, row: usize) -> LineId {
        LineId(self.grid.viewport_top().0 - self.display_offset as i64 + row as i64)
    }

    /// What the hover at `cell` depends on, as this view has it.
    fn hover_key(&self, cell: (u16, u16)) -> HoverKey<'_> {
        HoverKey {
            session: self.session,
            context: &self.link_context,
            cell,
            display_offset: self.display_offset,
            grid_version: (self.grid.generation(), self.grid.seq()),
        }
    }

    /// The currently-selected text (for copy), or empty when nothing is selected.
    fn selectable_content(&self) -> String {
        self.selection
            .map(|s| s.text(|id| self.grid.line(id).map(|l| l.text.clone())))
            .unwrap_or_default()
    }
}

impl<'a> From<TerminalPane<'a>> for Element<'a, Message> {
    fn from(pane: TerminalPane<'a>) -> Self {
        Element::new(pane)
    }
}

impl Widget<Message, Theme, Renderer> for TerminalPane<'_> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<PaneState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(PaneState::default())
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content = content_bounds(bounds);
        let metrics = CellMetrics::new(TERM_FONT_SIZE);
        let default_bg = self.palette.background();
        let state = tree.state.downcast_ref::<PaneState>();
        // The link the pointer marks, underlined and named in the hint (FR-007, FR-008).
        let marked = marked_link(
            state.hover.as_ref(),
            self.mouse_mode(),
            state.modifiers.shift(),
        );

        // Geometry below is drawn in absolute window coordinates, so the canvas frame must span
        // from the window origin to the pane's bottom-right corner. Sizing it to `viewport` breaks
        // when a parent clips the viewport (e.g. a `stack` overlay passes `clipped_viewport`),
        // which would cut off — and blank out — the part of the pane beyond the shrunken frame.
        let mut frame = Frame::new(
            renderer,
            Size::new(bounds.x + bounds.width, bounds.y + bounds.height),
        );
        {
            // Pane background.
            frame.fill_rectangle(bounds.position(), bounds.size(), default_bg);

            let rows = self.grid.rows() as usize;
            let cursor = self.grid.cursor();
            let show_cursor = shows_cursor(self.mode()) && cursor.visible;
            let display_offset = self.display_offset;

            for row in 0..rows {
                // The absolute line shown at this viewport row (accounting for scrollback). A line
                // the cache has not (yet) received renders blank.
                let line_id = self.line_at_row(row);
                let y = content.y + (row as f32) * metrics.height;
                let Some(cached) = self.grid.line(line_id) else {
                    continue;
                };

                // Expand the line's RLE style runs into a per-cell lookup.
                let mut styles: Vec<WireStyle> = Vec::with_capacity(cached.text.len());
                for (len, style) in &cached.runs {
                    for _ in 0..*len {
                        styles.push(*style);
                    }
                }

                let link_cols: Vec<std::ops::Range<u16>> = marked
                    .map(|link| {
                        link.link
                            .cells
                            .iter()
                            .filter(|span| span.row == row as i64)
                            .map(|span| span.cols.clone())
                            .collect()
                    })
                    .unwrap_or_default();

                for (col, ch) in cached.text.chars().enumerate() {
                    let x = content.x + (col as f32) * metrics.width;
                    let style = styles.get(col).copied().unwrap_or(DEFAULT_STYLE);
                    let flags = Flags::from_bits_truncate(style.flags);
                    // The cursor is anchored to an absolute `LineId`, so it draws only when its line
                    // is within the rendered window (never while scrolled back past it).
                    let is_cursor =
                        show_cursor && line_id == cursor.line && col as u16 == cursor.col;
                    let selected = self
                        .selection
                        .is_some_and(|s| s.contains(line_id, col as u16));
                    let (fg, bg) = wire_cell_colors(&self.palette, &style, selected);

                    // Per-cell background when it differs from the default.
                    if bg != default_bg {
                        frame.fill_rectangle(
                            iced::Point::new(x, y),
                            Size::new(metrics.width, metrics.height),
                            bg,
                        );
                    }

                    // Cursor block (drawn behind the glyph).
                    if is_cursor {
                        frame.fill_rectangle(
                            iced::Point::new(x, y),
                            Size::new(metrics.width, metrics.height),
                            self.palette.foreground(),
                        );
                    }

                    if ch != ' ' && ch != '\t' && ch != '\0' {
                        // Invert the glyph over the cursor block for legibility.
                        let glyph_fg = if is_cursor { default_bg } else { fg };
                        frame.fill_text(Text {
                            content: ch.to_string(),
                            // New in 0.14; unbounded, matching the previous single-glyph behaviour.
                            max_width: f32::INFINITY,
                            position: iced::Point::new(
                                x + metrics.width / 2.0,
                                y + metrics.height / 2.0,
                            ),
                            color: glyph_fg,
                            size: iced::Pixels(metrics.size),
                            font: cell_font(flags),
                            align_x: iced::widget::text::Alignment::Center,
                            align_y: alignment::Vertical::Center,
                            line_height: iced::widget::text::LineHeight::Absolute(iced::Pixels(
                                metrics.height,
                            )),
                            shaping: iced::widget::text::Shaping::Advanced,
                        });
                    }

                    // The marked link's underline, in the glyph's own colour so the text, the
                    // cursor and the selection stay visible under it (FR-009).
                    if link_cols.iter().any(|cols| cols.contains(&(col as u16))) {
                        let uy = y + metrics.height - 1.0;
                        let underline = if is_cursor { default_bg } else { fg };
                        frame.stroke(
                            &Path::line(
                                iced::Point::new(x, uy),
                                iced::Point::new(x + metrics.width, uy),
                            ),
                            Stroke::default().with_width(1.0).with_color(underline),
                        );
                    }

                    // Underline / strikethrough.
                    if flags.contains(Flags::UNDERLINE) {
                        let uy = y + metrics.height - 1.0;
                        frame.stroke(
                            &Path::line(
                                iced::Point::new(x, uy),
                                iced::Point::new(x + metrics.width, uy),
                            ),
                            Stroke::default().with_width(1.0).with_color(fg),
                        );
                    }
                    if flags.contains(Flags::STRIKEOUT) {
                        let sy = y + metrics.height / 2.0;
                        frame.stroke(
                            &Path::line(
                                iced::Point::new(x, sy),
                                iced::Point::new(x + metrics.width, sy),
                            ),
                            Stroke::default().with_width(1.0).with_color(fg),
                        );
                    }
                }
            }

            // Scrollback scrollbar: a right-edge track + thumb, shown only while scrolled back into
            // history so it stays out of the way during a live session (FR-016).
            let screen_lines = self.grid.rows() as usize;
            if let Some(sb) = scrollbar_metrics(
                content.height,
                screen_lines,
                self.history_size(),
                display_offset,
            ) {
                let fg = self.palette.foreground();
                let track_x = content.x + content.width - SCROLLBAR_WIDTH;
                frame.fill_rectangle(
                    iced::Point::new(track_x, content.y),
                    Size::new(SCROLLBAR_WIDTH, content.height),
                    Color { a: 0.08, ..fg },
                );
                let pad = 2.0;
                let thumb_w = SCROLLBAR_WIDTH - pad * 2.0;
                let thumb = Path::rounded_rectangle(
                    iced::Point::new(track_x + pad, content.y + sb.thumb_top),
                    Size::new(thumb_w, sb.thumb_height),
                    (thumb_w / 2.0).into(),
                );
                frame.fill(&thumb, Color { a: 0.5, ..fg });
            }

            // The address hint (FR-008, research R7): the marked link's `display`, middle-elided to
            // the pane's width, bottom-left unless the pointer is down there.
            if let (Some(link), Some(hover)) = (marked, state.hover.as_ref()) {
                let max_chars = ((content.width - 2.0 * HINT_PADDING) / metrics.width)
                    .floor()
                    .max(0.0) as usize;
                let label = elide_middle(&link.display, max_chars);
                let size = Size::new(
                    label.chars().count() as f32 * metrics.width + 2.0 * HINT_PADDING,
                    metrics.height + 2.0 * HINT_PADDING,
                );
                let rect = link_hint_rect(content, hover.cell.1, size);
                frame.fill_rectangle(rect.position(), rect.size(), self.palette.hint_container());
                for (i, ch) in label.chars().enumerate() {
                    let x = rect.x + HINT_PADDING + i as f32 * metrics.width;
                    if x + metrics.width > rect.x + rect.width {
                        break;
                    }
                    frame.fill_text(Text {
                        content: ch.to_string(),
                        max_width: f32::INFINITY,
                        position: iced::Point::new(
                            x + metrics.width / 2.0,
                            rect.y + rect.height / 2.0,
                        ),
                        color: self.palette.hint_content(),
                        size: iced::Pixels(metrics.size),
                        font: cell_font(Flags::empty()),
                        align_x: iced::widget::text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        line_height: iced::widget::text::LineHeight::Absolute(iced::Pixels(
                            metrics.height,
                        )),
                        shaping: iced::widget::text::Shaping::Advanced,
                    });
                }
            }

            // The focus indicator (FR-010, FR-010b, BUG-005): the 018 focus ring — `secondary`,
            // `FOCUS_RING_WIDTH` — filling the gutter `content_bounds` keeps on every side. Drawn
            // here and only here, so focus changes what is painted and never what is laid out: the
            // bar below keeps its children (feature 023) and the process keeps its size.
            if self.focused {
                let ring = self.palette.accent();
                let w = FOCUS_RING_WIDTH;
                for (origin, size) in [
                    (bounds.position(), Size::new(bounds.width, w)),
                    (
                        iced::Point::new(bounds.x, bounds.y + bounds.height - w),
                        Size::new(bounds.width, w),
                    ),
                    (bounds.position(), Size::new(w, bounds.height)),
                    (
                        iced::Point::new(bounds.x + bounds.width - w, bounds.y),
                        Size::new(w, bounds.height),
                    ),
                ] {
                    frame.fill_rectangle(origin, size, ring);
                }
            }
        }

        renderer.draw_geometry(frame.into_geometry());
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<PaneState>();
        let bounds = layout.bounds();
        // Presses anywhere on the pane — its focus gutter included — belong to it; cells and the
        // scrollbar are located inside the gutter, where `draw` puts them (FR-010b).
        let content = content_bounds(bounds);
        let metrics = CellMetrics::new(TERM_FONT_SIZE);

        // The visible grid size is reported by [`GridSizeReporter`], which wraps the terminal area
        // rather than living in it (BUG-003, FR-014a) — the pane is absent from the tree until a
        // session is displayed and its first frame has arrived, so a report owned here could not
        // exist at the moment a cold-started app most needs one. Reporting it in both places would
        // put two `SessionResize` messages on the wire for every resize.

        // Track modifiers (even when unfocused) so Shift-forces-selection works (FR-013b).
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(m)) = &event {
            state.modifiers = *m;
        }

        // ---- Links: keep the link under the pointer current (feature 031, research R2, R7).
        // Only on the events that can change it: the pointer moving, a modifier (Shift under mouse
        // reporting, and the link modifier for the pointer), a press or release, and a redraw, which
        // is where output that moved under a resting pointer is noticed.
        if matches!(
            event,
            Event::Mouse(
                mouse::Event::CursorMoved { .. }
                    | mouse::Event::CursorLeft
                    | mouse::Event::ButtonPressed(mouse::Button::Left)
                    | mouse::Event::ButtonReleased(mouse::Button::Left)
            ) | Event::Keyboard(keyboard::Event::ModifiersChanged(_))
                | Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            // What is drawn or shown for the link: the marked link, the pointer's row while one is
            // marked (the hint's side follows it), and the link modifier (the pointer).
            let shown = |state: &PaneState| {
                let marked = marked_link(
                    state.hover.as_ref(),
                    self.mouse_mode(),
                    state.modifiers.shift(),
                )
                .cloned();
                let row = marked
                    .as_ref()
                    .and(state.hover.as_ref())
                    .map(|hover| hover.cell.1);
                (marked, row, state.modifiers.command())
            };
            let before = shown(state);
            // A press over the scrollbar strip pages the view, so nothing under it is a link.
            let strip = scrollbar_metrics(
                content.height,
                self.size().1 as usize,
                self.history_size(),
                self.display_offset,
            )
            .is_some();
            let cell = cursor
                .position_over(content)
                .filter(|position| {
                    !(strip && position.x >= content.x + content.width - SCROLLBAR_WIDTH)
                })
                .map(|position| grid_at(position, content, metrics));
            match cell {
                None => state.hover = None,
                Some(cell) => {
                    let key = self.hover_key(cell);
                    match hover_refresh(state.hover.as_ref(), &key, |rows| {
                        rows_hash(self.grid, self.display_offset, rows)
                    }) {
                        HoverRefresh::Reuse => {}
                        HoverRefresh::Revalidated => {
                            if let Some(hover) = state.hover.as_mut() {
                                hover.grid_version = key.grid_version;
                            }
                        }
                        HoverRefresh::Resolve => {
                            state.hover = Some(resolve_hover(self.grid, &key));
                        }
                    }
                }
            }
            // Not a frame request: `idle_requests_no_frames.rs` keeps the only one in `cdk::motion`.
            // A stale tree is what makes the runtime repaint, and this fires only on a change of
            // the marked link or the modifier, so the render loop still settles (see `select.rs`).
            if shown(state) != before {
                shell.invalidate_widgets();
            }
        }

        // Deliberately nothing here for a press *outside* the pane (feature 023, FR-005/FR-006).
        // Until 023 this published `TerminalFocusReleased`, and that one rule cost two presses on
        // every control in the bar below: the release re-ran `view()` between the press and its
        // release, the bar dropped its focus-conditional child, every sibling after it shifted one
        // index, and iced's positional tree diff handed the pressed control its neighbour's node —
        // `is_pressed` gone, `on_press` never published (research R1). A press on something that
        // is none of the pane's business now changes nothing about the keyboard holder.

        // ---- Scrollbar: drag the right-edge thumb or click the track to page (FR-016). Handled
        // before selection so a drag on the scrollbar never starts a text selection.
        {
            let screen_lines = self.size().1 as usize;
            let history = self.history_size();
            let offset = self.display_offset;
            match &event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if let Some(pos) = cursor.position() {
                        let over_strip = cursor.is_over(bounds)
                            && pos.x >= content.x + content.width - SCROLLBAR_WIDTH;
                        if over_strip {
                            if let Some(sb) =
                                scrollbar_metrics(content.height, screen_lines, history, offset)
                            {
                                let thumb_y0 = content.y + sb.thumb_top;
                                let thumb_y1 = thumb_y0 + sb.thumb_height;
                                if (thumb_y0..thumb_y1).contains(&pos.y) {
                                    // Grab the thumb; remember where within it we grabbed.
                                    state.scrollbar_grab = Some(pos.y - thumb_y0);
                                } else {
                                    // Click the track: page a viewport toward the click.
                                    let delta = if pos.y < thumb_y0 {
                                        screen_lines as i32
                                    } else {
                                        -(screen_lines as i32)
                                    };
                                    shell.publish(Message::Session(SessionMsg::TerminalScrolled(
                                        delta,
                                    )));
                                }
                                shell.capture_event();
                                return;
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { position })
                    if state.scrollbar_grab.is_some() =>
                {
                    let grab_dy = state.scrollbar_grab.unwrap_or(0.0);
                    let thumb_top = position.y - content.y - grab_dy;
                    let target =
                        offset_for_thumb_top(content.height, screen_lines, history, thumb_top);
                    // Publish an absolute target, not a relative delta: several CursorMoved events
                    // are batched before the app update runs, so relative deltas computed against
                    // the pre-batch offset would accumulate and jump the view (drag flicker).
                    shell.publish(Message::Session(SessionMsg::TerminalScrolledTo(target)));
                    shell.capture_event();
                    return;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    if state.scrollbar_grab.is_some() =>
                {
                    state.scrollbar_grab = None;
                    shell.capture_event();
                    return;
                }
                _ => {}
            }
        }

        // ---- Mouse: selection (local) + mouse reporting (process input) ----
        match &event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                // This press's own answer, not the previous view's. `self.focused` is the flag
                // from the frame already on screen, so routing on it means the press that grants
                // focus is reported to nothing — in a TUI the first press does nothing and the
                // user presses again (FR-008b, research R5).
                let grants = press_grants_focus(self.focused, true);
                if grants {
                    shell.publish(Message::Session(SessionMsg::TerminalFocused));
                }
                let focused_now = self.focused || grants;
                let pos = cursor.position().unwrap_or_default();
                let (col, line) = grid_at(pos, content, metrics);
                let shift = state.modifiers.shift();
                if press_routing(focused_now, self.mouse_mode(), shift) == PressRouting::MouseReport
                {
                    if let Some(seq) =
                        self.mouse_report_bytes(0, col, line, true, to_keymap_mods(state.modifiers))
                    {
                        shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                    }
                    // Remember the held button so its release is reported too (FR-013a).
                    state.reporting_button = Some(0);
                    state.reported_cell = Some((col, line));
                } else {
                    // Every local press counts towards the click cadence, a link press included,
                    // so the second press of a Ctrl/Cmd double click selects a word (G3b).
                    let c = Click::new(pos, mouse::Button::Left, state.last_click);
                    state.last_click = Some(c);
                    let marked = marked_link(state.hover.as_ref(), self.mouse_mode(), shift);
                    let step = link_gesture(
                        GestureEvent::LeftPress {
                            cell: (col, line),
                            command: state.modifiers.command(),
                            routing: PressRouting::HandleLocally,
                            cadence: c.kind(),
                            marked,
                        },
                        state.link_press.as_ref(),
                    );
                    if let GestureStep::Press(press) = step {
                        state.link_press = Some(press);
                    } else {
                        state.link_press = None;
                        state.dragging = true;
                        state.press_cell = Some((col, line));
                        shell.publish(Message::Session(SessionMsg::TerminalSelectStart {
                            col,
                            line,
                            kind: select_kind(c.kind()),
                        }));
                    }
                }
                shell.capture_event();
                return;
            }
            // Motion while a reported button is held (FR-013a). Only for processes that asked
            // for motion (MOUSE_DRAG / MOUSE_MOTION), and only once per grid cell crossed —
            // reporting every pixel would flood the PTY. Button code +32 marks it as motion.
            Event::Mouse(mouse::Event::CursorMoved { position })
                if state.reporting_button.is_some() =>
            {
                let (col, line) = grid_at(*position, content, metrics);
                let button = state.reporting_button.unwrap_or(0);
                if self.mouse_motion_mode() && state.reported_cell != Some((col, line)) {
                    if let Some(seq) = self.mouse_report_bytes(
                        button + 32,
                        col,
                        line,
                        true,
                        to_keymap_mods(state.modifiers),
                    ) {
                        shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                    }
                }
                state.reported_cell = Some((col, line));
                shell.capture_event();
                return;
            }
            // Release of a reported button (FR-013a). Must come before the selection arms: in
            // mouse mode `state.dragging` was never set, so the release previously fell through
            // to `_ => {}` and the process saw a button that never came up.
            Event::Mouse(mouse::Event::ButtonReleased(_)) if state.reporting_button.is_some() => {
                let button = state.reporting_button.take().unwrap_or(0);
                state.reported_cell = None;
                let pos = cursor.position().unwrap_or_default();
                let (col, line) = grid_at(pos, content, metrics);
                if let Some(seq) = self.mouse_report_bytes(
                    button,
                    col,
                    line,
                    false,
                    to_keymap_mods(state.modifiers),
                ) {
                    shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                }
                shell.capture_event();
                return;
            }
            // A link press the pointer dragged off its cell is a selection after all (G2).
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.link_press.is_some() => {
                let (col, line) = grid_at(*position, content, metrics);
                if let GestureStep::SelectFrom {
                    col: from_col,
                    line: from_line,
                } = link_gesture(
                    GestureEvent::CursorMoved { cell: (col, line) },
                    state.link_press.as_ref(),
                ) {
                    state.link_press = None;
                    state.dragging = true;
                    shell.publish(Message::Session(SessionMsg::TerminalSelectStart {
                        col: from_col,
                        line: from_line,
                        kind: SelectKind::Simple,
                    }));
                    shell.publish(Message::Session(SessionMsg::TerminalSelectUpdate {
                        col,
                        line,
                    }));
                    shell.capture_event();
                    return;
                }
            }
            // The release of a link press: the link under the pointer now opens, resolved from the
            // grid as it is at release (G1, G9, FR-017). Nothing is written, selected or scrolled.
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.link_press.is_some() =>
            {
                // Released outside the pane: no cell is under the pointer, so nothing opens.
                let Some(position) = cursor.position_over(content) else {
                    state.link_press = None;
                    shell.capture_event();
                    return;
                };
                let cell = grid_at(position, content, metrics);
                let now = resolve_hover(self.grid, &self.hover_key(cell));
                let marked = marked_link(Some(&now), self.mouse_mode(), state.modifiers.shift());
                let step = link_gesture(
                    GestureEvent::LeftRelease { cell, marked },
                    state.link_press.as_ref(),
                );
                state.link_press = None;
                if let GestureStep::Release(Some(link)) = step {
                    shell.publish(Message::Session(SessionMsg::LinkActivated(link)));
                }
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                let (col, line) = grid_at(*position, content, metrics);
                // Jitter inside the pressed cell is still a click (FR-013e). Once the pointer has
                // left it, every cell counts — the pressed one included, so one character stays
                // selectable by dragging out and back. The top and left focus gutters belong to
                // the edge cell `grid_at` clamps them onto, as a press there does; past the last
                // column or row is a cell of its own. A position outside the pane has left the
                // pressed cell even where `grid_at` clamps it back onto it.
                if bounds.contains(*position) && state.press_cell == Some((col, line)) {
                    shell.capture_event();
                    return;
                }
                state.press_cell = None;
                shell.publish(Message::Session(SessionMsg::TerminalSelectUpdate {
                    col,
                    line,
                }));
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                state.press_cell = None;
                // Auto-copy on release (FR-013), decided by the shell against the selection as
                // this gesture left it. `self.selection` is the one this pane was built with: when
                // the press and release arrive in one batch it predates the press, and copying it
                // would put the previous selection over the clipboard (FR-013e, BUG-008).
                shell.publish(Message::Session(SessionMsg::TerminalSelectionReleased));
                shell.capture_event();
                return;
            }
            // Middle-click pastes the clipboard into the focused process (FR-013) — unless the
            // process is tracking the mouse, in which case the gesture belongs to it (FR-013a).
            // Shift forces the local behaviour, matching the left-button rule above and the
            // convention every other terminal follows.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
                if self.focused && cursor.is_over(bounds) =>
            {
                let shift = state.modifiers.shift();
                if self.mouse_mode() && !shift {
                    let (col, line) =
                        grid_at(cursor.position().unwrap_or_default(), content, metrics);
                    if let Some(seq) =
                        self.mouse_report_bytes(1, col, line, true, to_keymap_mods(state.modifiers))
                    {
                        shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                    }
                    state.reporting_button = Some(1);
                    state.reported_cell = Some((col, line));
                } else if let Some(pasted) = clipboard.read(ClipboardKind::Standard) {
                    shell.publish(Message::Session(SessionMsg::TerminalBytes(
                        keymap::paste_bytes(&pasted, self.grid.bracketed_paste()),
                    )));
                }
                shell.capture_event();
                return;
            }
            // Right-click opens the copy/paste context menu at the cursor (FR-013) — unless the
            // process is tracking the mouse, in which case it is forwarded (FR-013a). Shift
            // forces the menu, so it is always reachable.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if cursor.is_over(bounds) =>
            {
                // Furniture, but furniture that takes the keyboard if the pane did not have it
                // (FR-007 → FR-008b): a right-click is a press on the pane, and no press may be
                // consumed solely to grant focus.
                let grants = press_grants_focus(self.focused, true);
                if grants {
                    shell.publish(Message::Session(SessionMsg::TerminalFocused));
                }
                let focused_now = self.focused || grants;
                let pos = cursor.position().unwrap_or_default();
                let shift = state.modifiers.shift();
                if press_routing(focused_now, self.mouse_mode(), shift) == PressRouting::MouseReport
                {
                    let (col, line) = grid_at(pos, content, metrics);
                    if let Some(seq) =
                        self.mouse_report_bytes(2, col, line, true, to_keymap_mods(state.modifiers))
                    {
                        shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                    }
                    state.reporting_button = Some(2);
                    state.reported_cell = Some((col, line));
                } else {
                    let x = (pos.x - bounds.x).max(0.0) as u16;
                    let y = (pos.y - bounds.y).max(0.0) as u16;
                    shell.publish(Message::Session(SessionMsg::TerminalContextMenuOpened {
                        x,
                        y,
                    }));
                }
                shell.capture_event();
                return;
            }
            // Wheel scrolls the local scrollback, or forwards to a mouse-reporting program on
            // the alternate screen (FR-016 + wheel edge case).
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                // Sub-line deltas from a high-resolution touchpad accumulate rather than being
                // rounded away, or fine-grained scrolling would never move at all (BUG-002).
                let lines = wheel_lines(*delta, metrics.height, &mut state.scroll_residual);
                match wheel_routing(lines, self.focused, self.mouse_mode()) {
                    WheelRouting::MouseReport { button, count } => {
                        let (col, line) = cursor
                            .position()
                            .map(|p| grid_at(p, content, metrics))
                            .unwrap_or((0, 0));
                        let km = to_keymap_mods(state.modifiers);
                        for _ in 0..count {
                            if let Some(seq) = self.mouse_report_bytes(button, col, line, true, km)
                            {
                                shell.publish(Message::Session(SessionMsg::TerminalBytes(seq)));
                            }
                        }
                        shell.capture_event();
                        return;
                    }
                    WheelRouting::ScrollLocally { lines } => {
                        // Scrolling under a held button puts other text under the pointer, so
                        // the pressed screen cell no longer holds the pressed text (FR-013e). A
                        // turn the shell's clamp absorbs moves nothing and ends nothing.
                        let offset = self.display_offset as i64;
                        let history = self.history_size() as i64;
                        if (offset + i64::from(lines)).clamp(0, history) != offset {
                            state.press_cell = None;
                        }
                        shell.publish(Message::Session(SessionMsg::TerminalScrolled(lines)));
                        shell.capture_event();
                        return;
                    }
                    // Sub-line travel is banked in the residual; leave the event unhandled.
                    WheelRouting::Ignore => {}
                }
            }
            _ => {}
        }

        // Keyboard input reaches the process ONLY while focused (FR-006/FR-008/FR-009), decided
        // by the tested pure `route_key` (Constitution Principle I: the focus-gate decision lives
        // in tested core logic, not only in this untestable widget method — see
        // `tests/terminal_focus.rs`).
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) = event
        {
            let Some(k) = to_keymap_key(key) else {
                return;
            };
            let input = keymap::KeyInput {
                key: k,
                mods: to_keymap_mods(*modifiers),
                text: text.as_ref().map(|t| t.to_string()),
            };
            let output = keymap::encode(&input, self.key_term_mode());
            match route_key(self.focused, output) {
                // Unfocused: the key belongs to the surrounding app, so leave it uncaptured.
                KeyRouting::App => {}
                KeyRouting::Write(bytes) => {
                    shell.publish(Message::Session(SessionMsg::TerminalBytes(bytes)));
                    shell.capture_event();
                }
                KeyRouting::ReleaseFocus => {
                    shell.publish(Message::Session(SessionMsg::TerminalFocusReleased));
                    shell.capture_event();
                }
                KeyRouting::NewTerminalInstance => {
                    shell.publish(Message::Session(SessionMsg::ShellInstanceOpenRequested));
                    shell.capture_event();
                }
                KeyRouting::Copy => {
                    // Nothing selected, nothing to copy: the chord is still the terminal's, but the
                    // clipboard keeps whatever the user put there (FR-013c, BUG-004) — the same rule
                    // the shell's auto-copy on release follows in `selection::copy_request`.
                    let selected = self.selectable_content();
                    if !selected.is_empty() {
                        clipboard.write(ClipboardKind::Standard, selected);
                    }
                    shell.capture_event();
                }
                KeyRouting::Paste => {
                    if let Some(pasted) = clipboard.read(ClipboardKind::Standard) {
                        shell.publish(Message::Session(SessionMsg::TerminalBytes(
                            keymap::paste_bytes(&pasted, self.grid.bracketed_paste()),
                        )));
                    }
                    shell.capture_event();
                }
                KeyRouting::Ignore => {}
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<PaneState>();
        let marked = marked_link(
            state.hover.as_ref(),
            self.mouse_mode(),
            state.modifiers.shift(),
        )
        .is_some();
        pane_interaction(cursor.is_over(layout.bounds()), marked, state.modifiers)
    }
}

/// Map iced modifiers onto the pure `keymap::Mods`.
fn to_keymap_mods(m: keyboard::Modifiers) -> keymap::Mods {
    keymap::Mods {
        shift: m.shift(),
        ctrl: m.control(),
        alt: m.alt(),
        logo: m.logo(),
    }
}

/// Map an iced logical key onto the pure `keymap::Key` (returns `None` for keys the terminal
/// does not encode, e.g. `Unidentified`).
fn to_keymap_key(key: &keyboard::Key) -> Option<keymap::Key> {
    use keyboard::key::Named;
    use keymap::{Key as MK, NamedKey};
    match key {
        keyboard::Key::Character(s) => s.chars().next().map(MK::Char),
        keyboard::Key::Named(named) => {
            let nk = match named {
                Named::Enter => NamedKey::Enter,
                Named::Backspace => NamedKey::Backspace,
                Named::Tab => NamedKey::Tab,
                Named::Escape => NamedKey::Escape,
                Named::Space => NamedKey::Space,
                Named::Insert => NamedKey::Insert,
                Named::Delete => NamedKey::Delete,
                Named::Home => NamedKey::Home,
                Named::End => NamedKey::End,
                Named::PageUp => NamedKey::PageUp,
                Named::PageDown => NamedKey::PageDown,
                Named::ArrowUp => NamedKey::ArrowUp,
                Named::ArrowDown => NamedKey::ArrowDown,
                Named::ArrowLeft => NamedKey::ArrowLeft,
                Named::ArrowRight => NamedKey::ArrowRight,
                Named::F1 => NamedKey::F(1),
                Named::F2 => NamedKey::F(2),
                Named::F3 => NamedKey::F(3),
                Named::F4 => NamedKey::F(4),
                Named::F5 => NamedKey::F(5),
                Named::F6 => NamedKey::F(6),
                Named::F7 => NamedKey::F(7),
                Named::F8 => NamedKey::F(8),
                Named::F9 => NamedKey::F(9),
                Named::F10 => NamedKey::F(10),
                Named::F11 => NamedKey::F(11),
                Named::F12 => NamedKey::F(12),
                _ => return None,
            };
            Some(MK::Named(nk))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! Bin unit tests for the pane's pointer→grid mapping (feature 006 US2, T016). Run with
    //! `cargo test --features gui`.
    use super::*;

    // --- Left-press routing: local selection vs mouse report (FR-013a / FR-013b) ---

    #[test]
    fn press_is_handled_locally_when_the_process_has_no_mouse_reporting() {
        // The ordinary case: a shell that never enabled mouse mode. Dragging must select text.
        assert_eq!(
            press_routing(true, false, false),
            PressRouting::HandleLocally
        );
    }

    #[test]
    fn press_is_reported_to_a_mouse_reporting_process() {
        // A full-screen program that owns the mouse gets the event instead (FR-013a).
        assert_eq!(press_routing(true, true, false), PressRouting::MouseReport);
    }

    #[test]
    fn shift_forces_local_handling_even_under_mouse_reporting() {
        // FR-013b: selection and copy must stay reachable no matter what the process asked for.
        // This is the documented escape hatch when a TUI has grabbed the mouse.
        assert_eq!(press_routing(true, true, true), PressRouting::HandleLocally);
    }

    #[test]
    fn press_on_an_unfocused_pane_is_handled_locally_rather_than_reported() {
        // Mouse reports are process input, so an unfocused pane must not generate them even
        // while the process has mouse mode on — but selecting for copy is still allowed.
        assert_eq!(
            press_routing(false, true, false),
            PressRouting::HandleLocally
        );
    }

    // --- No press outside the pane reaches the process (feature 023, T011, FR-003/SC-008) ---
    //
    // A prohibition is the one kind of claim a visual pass is bad at: watching the screen tells you
    // what did happen, never that nothing arrived at the far end of a pipe. So this drives the real
    // widget through `Widget::update` on a headless CPU renderer and reads what it published.
    //
    // Inline rather than in `tests/` because `ui::material` is `pub(crate)` — deliberately, so a
    // call site cannot style a widget by hand — and `TerminalPane` is unreachable from an
    // integration test.

    mod presses {
        use super::*;
        use iced::advanced::renderer::Headless;
        use iced::advanced::widget::Tree;
        use iced::advanced::{clipboard, layout::Limits, Layout, Shell};
        use iced::{Element, Point, Rectangle, Size};

        const WINDOW: Size = Size::new(1200.0, 800.0);
        /// Comfortably outside a pane that fills the window.
        const OUTSIDE: Point = Point::new(-40.0, -40.0);

        /// Poll a future known to be immediately ready — the tiny-skia headless constructor does
        /// no I/O, so one poll suffices and no executor has to be pulled in.
        pub(super) fn block_on<F: std::future::Future>(f: F) -> F::Output {
            let mut f = Box::pin(f);
            let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
            loop {
                if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                    return v;
                }
                std::hint::spin_loop();
            }
        }

        /// `Some("tiny-skia")` is load-bearing: `iced_wgpu`'s `Headless::new` returns `None` on its
        /// first line when the hint is not `"wgpu"`, so the CPU rasteriser is picked without a GPU
        /// ever being probed.
        pub(super) fn headless() -> Renderer {
            block_on(<Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("the tiny-skia headless renderer must construct without a GPU")
        }

        /// Dispatch one event at `at` into a pane filling the window; return everything it published.
        fn dispatch(at: Point, focused: bool, event: Event) -> Vec<Message> {
            let renderer = headless();
            let grid = crate::grid::GridCache::default();
            let mut element: Element<'_, Message> = TerminalPane::new(
                &grid,
                crate::ui::terminal::TermPalette::from_scheme(
                    micold_core::theme::ColorScheme::Dark,
                ),
            )
            .focused(focused)
            .into();

            let mut tree = Tree::new(&element);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, WINDOW),
            );

            let mut messages: Vec<Message> = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(at),
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
            messages
        }

        fn reaches_the_process(m: &Message) -> bool {
            matches!(m, Message::Session(SessionMsg::TerminalBytes(_)))
        }

        #[test]
        fn no_press_outside_the_pane_reaches_the_process() {
            for button in [
                mouse::Button::Left,
                mouse::Button::Right,
                mouse::Button::Middle,
            ] {
                for focused in [false, true] {
                    let published = dispatch(
                        OUTSIDE,
                        focused,
                        Event::Mouse(mouse::Event::ButtonPressed(button)),
                    );
                    assert!(
                        !published.iter().any(reaches_the_process),
                        "a {button:?} press outside the pane (focused={focused}) produced input at \
                         the attached process (FR-003, SC-008): {published:?}"
                    );
                }
            }
        }

        #[test]
        fn a_press_outside_the_pane_does_not_touch_focus() {
            // The rule feature 023 deleted (research R1). A press on something that is none of the
            // pane's business must leave the keyboard exactly where it was (FR-005/FR-006) — and
            // publishing a release here is what shifted the bar's children mid-click and swallowed
            // the press underneath.
            let published = dispatch(
                OUTSIDE,
                true,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            );
            assert!(
                !published.iter().any(|m| matches!(
                    m,
                    Message::Session(SessionMsg::TerminalFocusReleased)
                        | Message::Session(SessionMsg::TerminalFocused)
                )),
                "a press outside the pane must not change the keyboard holder \
                 (FR-005, FR-006, FR-008a): {published:?}"
            );
        }

        #[test]
        fn a_press_inside_an_unfocused_pane_asks_for_focus() {
            // The complement, so the test above cannot pass by the pane having gone silent.
            let published = dispatch(
                Point::new(400.0, 300.0),
                false,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            );
            assert!(
                published
                    .iter()
                    .any(|m| matches!(m, Message::Session(SessionMsg::TerminalFocused))),
                "a press inside an unfocused pane takes the keyboard (FR-008b): {published:?}"
            );
        }
    }

    // --- Copy and paste gestures against a real clipboard (BUG-004, BUG-006) ---
    //
    // `clipboard::Null` reads nothing and forgets every write, so the defects both bugs are about —
    // a write that should not happen, bytes that should have been wrapped — were invisible to the
    // `presses` apparatus above. This drives the same real `update` through a clipboard that
    // remembers.

    mod clipboard_gestures {
        use super::*;
        use iced::advanced::widget::Tree;
        use iced::advanced::{layout::Limits, Layout, Shell};
        use iced::keyboard::key::{NativeCode, Physical};
        use micold_core::protocol::grid::{
            GridFrame, StyleRun, WireCursor, WireCursorShape, WireLine,
        };
        use micold_core::session::SessionId;

        const WINDOW: Size = Size::new(1200.0, 800.0);
        const INSIDE: Point = Point::new(400.0, 300.0);
        /// The text the user copied somewhere else before touching the terminal.
        const PRIOR: &str = "copied from the browser";

        /// A clipboard that remembers what it holds and every write made to it.
        struct RecordingClipboard {
            contents: Option<String>,
            writes: Vec<String>,
        }

        impl RecordingClipboard {
            fn holding(text: &str) -> Self {
                Self {
                    contents: Some(text.to_string()),
                    writes: Vec::new(),
                }
            }
        }

        impl Clipboard for RecordingClipboard {
            fn read(&self, _kind: ClipboardKind) -> Option<String> {
                self.contents.clone()
            }

            fn write(&mut self, _kind: ClipboardKind, contents: String) {
                self.writes.push(contents.clone());
                self.contents = Some(contents);
            }
        }

        /// A three-row grid whose top line reads `hello world`, with the given terminal mode bits.
        fn grid(mode: u32) -> GridCache {
            grid_with_history(mode, 0)
        }

        /// [`grid`] with `history` lines of scrollback above the screen, viewed at the live bottom.
        fn grid_with_history(mode: u32, history: i64) -> GridCache {
            let text = "hello world";
            let top = LineId(history);
            let mut cache = GridCache::default();
            cache.apply(&GridFrame {
                session: SessionId::new(),
                seq: 1,
                generation: 1,
                full: true,
                viewport_top: top,
                oldest_available: LineId(0),
                cols: 20,
                rows: 3,
                cursor: WireCursor {
                    line: top,
                    col: 0,
                    shape: WireCursorShape::Block,
                    visible: false,
                    blinking: false,
                },
                styles: vec![DEFAULT_STYLE],
                hyperlinks: Vec::new(),
                lines: vec![WireLine {
                    id: top,
                    text: text.to_string(),
                    runs: vec![StyleRun {
                        len: text.len() as u16,
                        style: 0,
                    }],
                    extras: Vec::new(),
                    wrapped: false,
                }],
                mode,
                input_serial: None,
            });
            cache
        }

        /// The platform's copy (`"c"`) or paste (`"v"`) chord (FR-013).
        fn chord(c: &str) -> Event {
            #[cfg(target_os = "macos")]
            let modifiers = keyboard::Modifiers::LOGO;
            #[cfg(not(target_os = "macos"))]
            let modifiers = keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT;
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(c.into()),
                modified_key: keyboard::Key::Character(c.to_uppercase().into()),
                physical_key: Physical::Unidentified(NativeCode::Unidentified),
                location: keyboard::Location::Standard,
                modifiers,
                text: None,
                repeat: false,
            })
        }

        fn middle_click() -> Event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
        }

        /// Dispatch `event` into a focused pane over `grid`; return what it published.
        fn dispatch(
            grid: &GridCache,
            selection: Option<&Selection>,
            event: Event,
            clipboard: &mut RecordingClipboard,
        ) -> Vec<Message> {
            let renderer = super::presses::headless();
            let mut element: Element<'_, Message> = TerminalPane::new(
                grid,
                TermPalette::from_scheme(micold_core::theme::ColorScheme::Dark),
            )
            .selection(selection)
            .focused(true)
            .into();
            let mut tree = Tree::new(&element);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, WINDOW),
            );
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(INSIDE),
                &renderer,
                clipboard,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
            messages
        }

        fn bytes_sent(published: &[Message]) -> Vec<Vec<u8>> {
            published
                .iter()
                .filter_map(|m| match m {
                    Message::Session(SessionMsg::TerminalBytes(b)) => Some(b.clone()),
                    _ => None,
                })
                .collect()
        }

        // BUG-004 (FR-013c).

        #[test]
        fn a_copy_chord_with_nothing_selected_leaves_the_clipboard_untouched() {
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = dispatch(&grid, None, chord("c"), &mut clipboard);

            assert!(
                clipboard.writes.is_empty(),
                "a copy with nothing selected wrote {:?} to the clipboard, destroying what the \
                 user had copied (FR-013c, BUG-004)",
                clipboard.writes
            );
            assert_eq!(clipboard.contents.as_deref(), Some(PRIOR));
            assert!(
                bytes_sent(&published).is_empty(),
                "the copy chord belongs to the terminal, selection or none (FR-013)"
            );
        }

        #[test]
        fn a_copy_chord_with_a_selection_copies_exactly_that_text() {
            // The complement, so the test above cannot pass by the chord having stopped copying.
            let grid = grid(0);
            let selection = Selection::start(
                crate::selection::Anchor::new(LineId(0), 0),
                crate::selection::SelectGranularity::Line,
                |id| grid.line(id).map(|l| l.text.clone()),
            );
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            dispatch(&grid, Some(&selection), chord("c"), &mut clipboard);

            assert_eq!(clipboard.writes, vec!["hello world".to_string()]);
        }

        // BUG-008 (FR-013e).

        /// A point inside grid cell `(col, line)`, at fraction `(fx, fy)` of the cell's width and
        /// height from its top-left corner, for a pane laid out at `node`.
        fn in_cell(node: Rectangle, col: u16, line: u16, fx: f32, fy: f32) -> Point {
            let content = content_bounds(node);
            let cell = CellMetrics::new(TERM_FONT_SIZE);
            Point::new(
                content.x + (f32::from(col) + fx) * cell.width,
                content.y + (f32::from(line) + fy) * cell.height,
            )
        }

        /// One pointer step of a gesture, at a point inside a cell (see [`in_cell`]).
        #[derive(Clone, Copy)]
        enum Pointer {
            Press(u16, u16, f32, f32),
            Move(u16, u16, f32, f32),
            Release(u16, u16, f32, f32),
            /// A wheel turn of `lines` lines with the pointer at the middle of cell `(col, line)`.
            Wheel(u16, u16, f32),
        }

        /// Deliver a whole gesture to one focused pane, built once with `selection`, and return
        /// everything it published in order.
        ///
        /// One pane for every step is what iced does within a batch: every event since the last
        /// redraw reaches the same widget, and the published messages are applied only afterwards
        /// — so a release in the same batch as its press sees the selection from before it.
        fn gesture(
            grid: &GridCache,
            selection: Option<&Selection>,
            steps: &[Pointer],
            clipboard: &mut RecordingClipboard,
        ) -> Vec<Message> {
            let renderer = super::presses::headless();
            let mut element: Element<'_, Message> = TerminalPane::new(
                grid,
                TermPalette::from_scheme(micold_core::theme::ColorScheme::Dark),
            )
            .selection(selection)
            .focused(true)
            .into();
            let mut tree = Tree::new(&element);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, WINDOW),
            );
            let bounds = node.bounds();
            let mut messages = Vec::new();
            for step in steps {
                let (event, at) = match *step {
                    Pointer::Press(c, l, fx, fy) => (
                        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        in_cell(bounds, c, l, fx, fy),
                    ),
                    Pointer::Move(c, l, fx, fy) => {
                        let position = in_cell(bounds, c, l, fx, fy);
                        (
                            Event::Mouse(mouse::Event::CursorMoved { position }),
                            position,
                        )
                    }
                    Pointer::Release(c, l, fx, fy) => (
                        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                        in_cell(bounds, c, l, fx, fy),
                    ),
                    Pointer::Wheel(c, l, lines) => (
                        Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
                        }),
                        in_cell(bounds, c, l, 0.5, 0.5),
                    ),
                };
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    &mut tree,
                    &event,
                    Layout::new(&node),
                    mouse::Cursor::Available(at),
                    &renderer,
                    clipboard,
                    &mut shell,
                    &Rectangle::with_size(WINDOW),
                );
            }
            messages
        }

        fn select_updates(published: &[Message]) -> Vec<(u16, u16)> {
            published
                .iter()
                .filter_map(|m| match m {
                    Message::Session(SessionMsg::TerminalSelectUpdate { col, line }) => {
                        Some((*col, *line))
                    }
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn pointer_jitter_inside_the_pressed_cell_is_not_a_drag() {
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(6, 0, 0.3, 0.4),
                    Pointer::Move(6, 0, 0.7, 0.6),
                    Pointer::Move(6, 0, 0.5, 0.9),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                Vec::<(u16, u16)>::new(),
                "motion that never left the pressed cell is a click, not a drag, so it must not \
                 extend the selection (FR-013e)"
            );
        }

        #[test]
        fn motion_into_another_cell_and_back_extends_the_selection_each_time() {
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(6, 0, 0.5, 0.5),
                    Pointer::Move(7, 0, 0.5, 0.5),
                    Pointer::Move(6, 0, 0.5, 0.5),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                vec![(7, 0), (6, 0)],
                "once the pointer has left the pressed cell it is a drag, and every cell it \
                 enters — including the pressed one again — extends the selection (FR-013a)"
            );
        }

        #[test]
        fn motion_after_a_scroll_while_held_is_a_drag_even_in_the_pressed_screen_cell() {
            // Scrollback to scroll into, so the wheel turn really moves the view.
            let grid = grid_with_history(0, 10);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(6, 0, 0.5, 0.5),
                    Pointer::Wheel(6, 0, 3.0),
                    Pointer::Move(6, 0, 0.7, 0.5),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                vec![(6, 0)],
                "scrolling while the button is held puts other text under the pointer, so the \
                 pressed screen cell no longer holds the pressed text and motion there is a drag \
                 (FR-013e)"
            );
        }

        #[test]
        fn motion_past_the_panes_edge_is_a_drag_even_where_it_clamps_to_the_pressed_cell() {
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(0, 0, 0.5, 0.5),
                    Pointer::Move(0, 0, -2.0, 0.5),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                vec![(0, 0)],
                "a pointer that has left the pane has left the pressed cell, even though the \
                 position clamps back onto it (FR-013e)"
            );
        }

        #[test]
        fn jitter_in_the_focus_gutter_beside_the_pressed_edge_cell_is_not_a_drag() {
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            // A fifth of a cell left of column 0 is inside the pane's focus gutter, where a press
            // still belongs to the pane and lands on the edge cell.
            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(0, 0, -0.1, 0.5),
                    Pointer::Move(0, 0, -0.2, 0.5),
                    Pointer::Move(0, 0, 0.3, 0.5),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                Vec::<(u16, u16)>::new(),
                "a press in the gutter belongs to the edge cell beside it, and motion that stays \
                 in that gutter or that cell never entered another cell (FR-013e)"
            );
        }

        #[test]
        fn a_wheel_turn_that_cannot_scroll_leaves_jitter_a_click() {
            // No scrollback: the view is already at both ends, so the text under the pointer
            // stays where it was.
            let grid = grid(0);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                None,
                &[
                    Pointer::Press(6, 0, 0.5, 0.5),
                    Pointer::Wheel(6, 0, 3.0),
                    Pointer::Wheel(6, 0, -3.0),
                    Pointer::Move(6, 0, 0.7, 0.5),
                ],
                &mut clipboard,
            );

            assert_eq!(
                select_updates(&published),
                Vec::<(u16, u16)>::new(),
                "a wheel turn that moves nothing leaves the pressed text under the pointer, so \
                 motion inside the pressed cell is still a click (FR-013e)"
            );
        }

        /// A selection the user made earlier: the whole of `hello world`.
        fn held_line(grid: &GridCache) -> Selection {
            Selection::start(
                crate::selection::Anchor::new(LineId(0), 0),
                crate::selection::SelectGranularity::Line,
                |id| grid.line(id).map(|l| l.text.clone()),
            )
        }

        #[test]
        fn a_tap_over_a_held_selection_writes_nothing_to_the_clipboard() {
            let grid = grid(0);
            let held = held_line(&grid);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            gesture(
                &grid,
                Some(&held),
                &[
                    Pointer::Press(6, 0, 0.5, 0.5),
                    Pointer::Release(6, 0, 0.5, 0.5),
                ],
                &mut clipboard,
            );

            assert!(
                clipboard.writes.is_empty(),
                "a press and release delivered together copied {:?} — the selection from before \
                 the press — over the user's clipboard (FR-013e, BUG-008)",
                clipboard.writes
            );
        }

        #[test]
        fn a_release_asks_for_the_copy_after_its_press_starts_the_selection() {
            let grid = grid(0);
            let held = held_line(&grid);
            let mut clipboard = RecordingClipboard::holding(PRIOR);

            let published = gesture(
                &grid,
                Some(&held),
                &[
                    Pointer::Press(6, 0, 0.5, 0.5),
                    Pointer::Release(6, 0, 0.5, 0.5),
                ],
                &mut clipboard,
            );

            let start = published.iter().position(|m| {
                matches!(m, Message::Session(SessionMsg::TerminalSelectStart { .. }))
            });
            let released = published
                .iter()
                .position(|m| matches!(m, Message::Session(SessionMsg::TerminalSelectionReleased)));
            assert!(
                matches!((start, released), (Some(s), Some(r)) if s < r),
                "the release must ask the shell to copy the selection as its own press left it, \
                 so the request has to follow the press's TerminalSelectStart (FR-013): start at \
                 {start:?}, release request at {released:?}"
            );
        }

        // BUG-006 (FR-013d).

        const BRACKETED_PASTE: u32 = TermMode::BRACKETED_PASTE.bits();
        const TWO_LINES: &str = "echo AAA\necho BBB\n";

        fn bracketed(text: &str) -> Vec<u8> {
            [b"\x1b[200~".as_slice(), text.as_bytes(), b"\x1b[201~"].concat()
        }

        #[test]
        fn a_paste_chord_into_a_bracketed_paste_process_is_one_bracketed_block() {
            let grid = grid(BRACKETED_PASTE);
            let mut clipboard = RecordingClipboard::holding(TWO_LINES);

            let published = dispatch(&grid, None, chord("v"), &mut clipboard);

            assert_eq!(
                bytes_sent(&published),
                vec![bracketed(TWO_LINES)],
                "the process enabled bracketed paste, so the paste must arrive wrapped — raw, \
                 every embedded newline is an Enter (FR-013d, BUG-006)"
            );
        }

        #[test]
        fn a_middle_click_paste_into_a_bracketed_paste_process_is_one_bracketed_block() {
            let grid = grid(BRACKETED_PASTE);
            let mut clipboard = RecordingClipboard::holding(TWO_LINES);

            let published = dispatch(&grid, None, middle_click(), &mut clipboard);

            assert_eq!(bytes_sent(&published), vec![bracketed(TWO_LINES)]);
        }

        #[test]
        fn a_pasted_end_marker_cannot_close_the_block_early() {
            let grid = grid(BRACKETED_PASTE);
            let mut clipboard = RecordingClipboard::holding("safe\x1b[201~rm -rf ~\n");

            let published = dispatch(&grid, None, chord("v"), &mut clipboard);

            assert_eq!(
                bytes_sent(&published),
                vec![bracketed("saferm -rf ~\n")],
                "an end marker inside the clipboard would end the block and run the rest as \
                 keystrokes (FR-013d)"
            );
        }

        #[test]
        fn without_bracketed_paste_both_gestures_deliver_the_text_unchanged() {
            let grid = grid(0);
            for event in [chord("v"), middle_click()] {
                let mut clipboard = RecordingClipboard::holding(TWO_LINES);
                let published = dispatch(&grid, None, event.clone(), &mut clipboard);
                assert_eq!(
                    bytes_sent(&published),
                    vec![TWO_LINES.as_bytes().to_vec()],
                    "{event:?}: a process that did not ask for bracketing gets the bytes as pasted"
                );
            }
        }
    }

    // --- The focus indicator (BUG-005, FR-010 / FR-010b) ---
    //
    // Layout gates cannot see this — the ring lives only in `draw` — so the pane is rasterised on
    // the CPU and its pixels read back.

    mod focus_indicator {
        use super::*;
        use iced::advanced::renderer::{Headless, Style};
        use iced::advanced::widget::Tree;
        use iced::advanced::{layout::Limits, Layout, Shell};
        use micold_core::protocol::grid::{
            GridFrame, StyleRun, WireCursor, WireCursorShape, WireLine,
        };
        use micold_core::session::SessionId;
        use micold_core::theme::ColorScheme;
        use micold_core::tokens;

        const W: u32 = 240;
        const H: u32 = 120;

        /// The top-left cells painted solid red, so where the character area begins is visible.
        const RED: WireStyle = WireStyle {
            fg: WireColor::Named(NamedColor::Foreground as u16),
            bg: WireColor::Rgb(255, 0, 0),
            flags: 0,
            underline_color: None,
        };

        fn grid() -> GridCache {
            let mut cache = GridCache::default();
            cache.apply(&GridFrame {
                session: SessionId::new(),
                seq: 1,
                generation: 1,
                full: true,
                viewport_top: LineId(0),
                oldest_available: LineId(0),
                cols: 20,
                rows: 3,
                cursor: WireCursor {
                    line: LineId(0),
                    col: 0,
                    shape: WireCursorShape::Block,
                    visible: false,
                    blinking: false,
                },
                styles: vec![RED],
                hyperlinks: Vec::new(),
                lines: vec![WireLine {
                    id: LineId(0),
                    text: "    ".to_string(),
                    runs: vec![StyleRun { len: 4, style: 0 }],
                    extras: Vec::new(),
                    wrapped: false,
                }],
                mode: 0,
                input_serial: None,
            });
            cache
        }

        /// RGBA of the pane rendered at `W`×`H`, in `scheme`, focused or not.
        fn render(scheme: ColorScheme, focused: bool) -> Vec<u8> {
            let mut renderer = super::presses::headless();
            let grid = grid();
            let mut element: Element<'_, Message> =
                TerminalPane::new(&grid, TermPalette::from_scheme(scheme))
                    .focused(focused)
                    .into();
            let size = Size::new(W as f32, H as f32);
            let mut tree = Tree::new(&element);
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, size),
            );
            let viewport = Rectangle::with_size(size);
            iced::advanced::Renderer::reset(&mut renderer, viewport);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &Theme::Dark,
                &Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            renderer.screenshot(Size::new(W, H), 1.0, Color::BLACK)
        }

        fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 3] {
            let i = ((y * W + x) * 4) as usize;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        }

        fn rgb(c: tokens::Rgb) -> [u8; 3] {
            [c.r, c.g, c.b]
        }

        fn close(a: [u8; 3], b: [u8; 3]) -> bool {
            a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2)
        }

        /// One point on each edge, one pixel in — inside a 3dp ring wherever it is drawn.
        const EDGES: [(u32, u32); 4] = [(W / 2, 1), (W / 2, H - 2), (W - 2, H / 2), (1, H / 2)];

        #[test]
        fn a_focused_pane_is_outlined_in_secondary_and_an_unfocused_one_is_not() {
            for scheme in [ColorScheme::Light, ColorScheme::Dark] {
                let r = tokens::roles(scheme);
                let focused = render(scheme, true);
                let unfocused = render(scheme, false);
                for (x, y) in EDGES {
                    assert!(
                        close(at(&focused, x, y), rgb(r.secondary)),
                        "{scheme:?}: focused pane at ({x},{y}) is {:?}, not the 018 focus \
                         indicator's secondary {:?} (FR-010b, BUG-005)",
                        at(&focused, x, y),
                        rgb(r.secondary)
                    );
                    assert!(
                        close(at(&unfocused, x, y), rgb(r.surface)),
                        "{scheme:?}: unfocused pane at ({x},{y}) is {:?}, not the terminal \
                         background {:?} — the indicator must be absent without focus",
                        at(&unfocused, x, y),
                        rgb(r.surface)
                    );
                }
            }
        }

        #[test]
        fn the_ring_sits_in_a_gutter_reserved_at_every_focus_state() {
            // Column 0 starts inside the ring's width whether or not the pane is focused. Drawn over
            // the cells, the ring would clip the first column; inset only while focused, the
            // character area — and so the process's size — would change with focus.
            let inset = tokens::state::FOCUS_RING_WIDTH as u32;
            for focused in [false, true] {
                let pixels = render(ColorScheme::Dark, focused);
                assert!(
                    !close(at(&pixels, 1, inset + 4), [255, 0, 0]),
                    "focused={focused}: the first cell's paint reaches the pane's edge, so there \
                     is no gutter for the focus ring (FR-010b)"
                );
                assert!(
                    close(at(&pixels, inset + 1, inset + 4), [255, 0, 0]),
                    "focused={focused}: the first cell does not start just inside the gutter"
                );
            }
        }

        #[test]
        fn the_reported_size_is_the_character_area_inside_the_gutter() {
            // 100 cells of 7.8px plus 2px of slack: measured edge to edge this is 100 columns, the
            // last of which would sit under the ring; inside the gutter it is 99.
            let metrics = CellMetrics::new(TERM_FONT_SIZE);
            let inset = tokens::state::FOCUS_RING_WIDTH;
            let size = Size::new(metrics.width * 100.0 + 2.0, metrics.height * 30.0 + 2.0);
            let renderer = super::presses::headless();
            let grid = GridCache::default();
            let mut element: Element<'_, Message> = GridSizeReporter::new(TerminalPane::new(
                &grid,
                TermPalette::from_scheme(ColorScheme::Dark),
            ))
            .into();
            let mut tree = Tree::new(&element);
            let node =
                element
                    .as_widget_mut()
                    .layout(&mut tree, &renderer, &Limits::new(size, size));
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &Event::Window(iced::window::Event::RedrawRequested(
                    std::time::Instant::now(),
                )),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::with_size(size),
            );
            let reported = messages.iter().find_map(|m| match m {
                Message::Session(SessionMsg::TerminalResized { cols, rows }) => {
                    Some((*cols, *rows))
                }
                _ => None,
            });
            assert_eq!(
                reported,
                Some(metrics.grid_size(size.width - 2.0 * inset, size.height - 2.0 * inset)),
                "the process must be sized to the character area inside the focus gutter (FR-010b)"
            );
            assert_eq!(reported, Some((99, 29)));
        }
    }

    // --- The press that grants focus (feature 023, FR-008b / research R5) ---

    #[test]
    fn a_press_over_an_unfocused_pane_grants_focus() {
        assert!(
            press_grants_focus(false, true),
            "this is the press that makes the terminal the keyboard holder (FR-008b)"
        );
    }

    #[test]
    fn nothing_grants_focus_to_a_pane_that_already_has_it() {
        // Not merely redundant: publishing `TerminalFocused` again on every press inside a focused
        // pane would be a message per click for a state that never changes.
        assert!(!press_grants_focus(true, true));
    }

    #[test]
    fn a_press_outside_the_bounds_grants_nothing() {
        // The press belongs to whatever is under it. Feature 023 deleted the rule that made an
        // outside press release focus; it must not gain one that makes it take focus either.
        assert!(!press_grants_focus(false, false));
    }

    #[test]
    fn a_right_click_on_an_unfocused_pane_grants_focus_too() {
        // FR-007 files the pane's own context menu with its scrollbar and status bar: furniture,
        // which leaves the terminal holding the keyboard — and takes it if the pane did not have
        // it. Which button was pressed is not part of the decision, which is why the function does
        // not ask.
        assert!(press_grants_focus(false, true));
    }

    #[test]
    fn the_granting_press_is_routed_as_if_focused() {
        // The bug's mirror image (research R5): routing this press on the *previous* view's
        // `false` means a mouse-aware program never sees it, and the user presses twice — once to
        // focus, once for the program. `focused_now` is what makes one press enough.
        let focused_now = press_grants_focus(false, true);
        assert_eq!(
            press_routing(focused_now, true, false),
            PressRouting::MouseReport,
            "the press that grants focus must reach a mouse-reporting process (FR-008b, SC-009)"
        );
        // Shift still overrides, exactly as on an already-focused pane (FR-013b).
        assert_eq!(
            press_routing(focused_now, true, true),
            PressRouting::HandleLocally
        );
        // And with no mouse mode it selects, as it always did.
        assert_eq!(
            press_routing(focused_now, false, false),
            PressRouting::HandleLocally
        );
    }

    // --- Click cadence → selection granularity (T057, FR-013) ---

    #[test]
    fn a_single_click_starts_a_plain_character_selection() {
        assert_eq!(select_kind(click::Kind::Single), SelectKind::Simple);
    }

    #[test]
    fn a_double_click_selects_the_word_under_the_pointer() {
        // "Semantic" is alacritty's word-granularity selection — the usual double-click-a-word.
        assert_eq!(select_kind(click::Kind::Double), SelectKind::Semantic);
    }

    #[test]
    fn a_triple_click_selects_whole_lines() {
        assert_eq!(select_kind(click::Kind::Triple), SelectKind::Lines);
    }

    // --- Wheel delta → lines (BUG-002: touchpad scrolling under Wayland) ---

    #[test]
    fn wheel_lines_accumulates_sub_line_pixel_deltas() {
        let cell = CellMetrics::new(TERM_FONT_SIZE).height; // 18.2
        let mut residual = 0.0;

        // A high-resolution touchpad emits deltas far smaller than one cell. Individually they
        // must not scroll, but they must NOT be discarded either.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual
            ),
            0,
            "a 5px delta is less than one 18.2px line, so it cannot scroll yet"
        );
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual
            ),
            0,
            "10px accumulated is still under one line"
        );
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual
            ),
            0,
            "15px accumulated is still under one line"
        );
        // 20px total now exceeds one 18.2px line.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual
            ),
            1,
            "four 5px deltas sum past one line and must produce exactly one line of scroll"
        );
    }

    #[test]
    fn wheel_lines_keeps_the_fraction_after_emitting_a_line() {
        let cell = 10.0;
        let mut residual = 0.0;
        // 15px = one whole line plus 5px left over.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 15.0 },
                cell,
                &mut residual
            ),
            1
        );
        // The retained 5px plus another 5px completes the second line.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual
            ),
            1,
            "the 5px remainder must carry over rather than being thrown away"
        );
    }

    #[test]
    fn wheel_lines_drops_residual_on_direction_change() {
        let cell = 10.0;
        let mut residual = 0.0;
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 8.0 },
                cell,
                &mut residual
            ),
            0
        );
        // Reversing direction must not let the stale upward 8px cancel the new downward motion.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: -8.0 },
                cell,
                &mut residual
            ),
            0
        );
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: -4.0 },
                cell,
                &mut residual
            ),
            -1,
            "12px of downward travel since the reversal is one line down"
        );
    }

    #[test]
    fn wheel_lines_passes_through_discrete_line_deltas() {
        let cell = 18.2;
        let mut residual = 0.0;
        // X11 / discrete wheels deliver whole lines; behavior must be unchanged.
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
                cell,
                &mut residual
            ),
            1
        );
        assert_eq!(
            wheel_lines(
                mouse::ScrollDelta::Lines { x: 0.0, y: -3.0 },
                cell,
                &mut residual
            ),
            -3
        );
    }

    // --- Wheel routing: local scrollback vs mouse report (T055, BUG-002 / FR-013a / FR-016b) ---

    #[test]
    fn wheel_scrolls_the_local_scrollback_when_the_process_has_no_mouse_reporting() {
        // The ordinary case: the wheel drives our own scrollback view, not the process.
        assert_eq!(
            wheel_routing(3, true, false),
            WheelRouting::ScrollLocally { lines: 3 }
        );
        assert_eq!(
            wheel_routing(-2, true, false),
            WheelRouting::ScrollLocally { lines: -2 }
        );
    }

    #[test]
    fn wheel_is_reported_to_a_focused_mouse_reporting_process() {
        // A full-screen program that owns the mouse scrolls itself (FR-013a). Wheel-up is
        // button 64 and wheel-down 65, one report per accumulated line.
        assert_eq!(
            wheel_routing(1, true, true),
            WheelRouting::MouseReport {
                button: 64,
                count: 1
            }
        );
        assert_eq!(
            wheel_routing(-3, true, true),
            WheelRouting::MouseReport {
                button: 65,
                count: 3
            }
        );
    }

    #[test]
    fn wheel_on_an_unfocused_pane_scrolls_locally_rather_than_reporting() {
        // Mouse reports are process input, so an unfocused pane must not generate them even
        // while the process has mouse mode on — the wheel still moves our own view.
        assert_eq!(
            wheel_routing(2, false, true),
            WheelRouting::ScrollLocally { lines: 2 }
        );
    }

    #[test]
    fn a_wheel_event_that_accumulated_no_whole_line_does_nothing() {
        // The sub-line case must fall through unhandled so the event stays available to other
        // widgets — it is banked in the residual, not consumed (BUG-002).
        assert_eq!(wheel_routing(0, true, false), WheelRouting::Ignore);
        assert_eq!(wheel_routing(0, true, true), WheelRouting::Ignore);
    }

    #[test]
    fn a_sub_line_touchpad_gesture_eventually_scrolls_the_local_scrollback() {
        // The whole BUG-002 defect end-to-end over the two helpers on_event composes: a gesture
        // made only of sub-line pixel deltas must reach a real scroll rather than vanishing.
        let cell = CellMetrics::new(TERM_FONT_SIZE).height; // 18.2
        let mut residual = 0.0;
        let mut scrolled = 0;

        for _ in 0..4 {
            let lines = wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual,
            );
            if let WheelRouting::ScrollLocally { lines } = wheel_routing(lines, true, false) {
                scrolled += lines;
            }
        }

        assert_eq!(
            scrolled, 1,
            "four 5px touchpad deltas must scroll exactly one line, not zero (BUG-002)"
        );
    }

    #[test]
    fn a_sub_line_touchpad_gesture_eventually_reports_a_wheel_to_the_process() {
        // The same gesture over the FR-013a path: a touchpad must generate wheel reports for a
        // mouse-reporting process too, which the pre-BUG-002 quantization also prevented.
        let cell = CellMetrics::new(TERM_FONT_SIZE).height;
        let mut residual = 0.0;
        let mut reports = Vec::new();

        for _ in 0..4 {
            let lines = wheel_lines(
                mouse::ScrollDelta::Pixels { x: 0.0, y: 5.0 },
                cell,
                &mut residual,
            );
            if let WheelRouting::MouseReport { button, count } = wheel_routing(lines, true, true) {
                reports.push((button, count));
            }
        }

        assert_eq!(
            reports,
            vec![(64, 1)],
            "the gesture must produce exactly one wheel-up report once the residual crosses a line"
        );
    }

    #[test]
    fn grid_at_maps_pixels_to_cells() {
        let metrics = CellMetrics::new(TERM_FONT_SIZE); // width 7.8, height 18.2
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
        };
        // Just inside the origin cell.
        assert_eq!(grid_at(Point::new(11.0, 21.0), bounds, metrics), (0, 0));
        // One cell right, two cells down.
        let (col, line) = grid_at(
            Point::new(10.0 + metrics.width * 1.5, 20.0 + metrics.height * 2.5),
            bounds,
            metrics,
        );
        assert_eq!((col, line), (1, 2));
    }

    #[test]
    fn grid_at_clamps_above_and_left_of_bounds_to_origin() {
        let metrics = CellMetrics::new(TERM_FONT_SIZE);
        let bounds = Rectangle {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };
        assert_eq!(grid_at(Point::new(0.0, 0.0), bounds, metrics), (0, 0));
    }

    #[test]
    fn scrollbar_hidden_at_live_bottom_and_without_history() {
        // Parked at the bottom (offset 0) → hidden, even with history to show.
        assert_eq!(scrollbar_metrics(100.0, 30, 500, 0), None);
        // No scrollback history at all → hidden.
        assert_eq!(scrollbar_metrics(100.0, 30, 0, 0), None);
        // Degenerate track height → hidden (nothing to draw).
        assert_eq!(scrollbar_metrics(0.0, 30, 500, 10), None);
    }

    #[test]
    fn scrollbar_thumb_reaches_top_when_fully_scrolled() {
        // display_offset == history_size: viewing the oldest line → thumb pinned to the top.
        let sb = scrollbar_metrics(200.0, 40, 200, 200).unwrap();
        assert_eq!(sb.thumb_top, 0.0);
    }

    #[test]
    fn scrollbar_thumb_sits_near_bottom_when_barely_scrolled() {
        // Scrolled up a single line out of a deep history → thumb near the track bottom.
        let track = 200.0;
        let sb = scrollbar_metrics(track, 40, 200, 1).unwrap();
        let travel = track - sb.thumb_height;
        assert!(
            (sb.thumb_top - travel).abs() < 1.0,
            "thumb_top {} should sit near the track bottom {travel}",
            sb.thumb_top
        );
    }

    #[test]
    fn scrollbar_thumb_is_proportional_to_the_viewport() {
        // Viewport is 40 of 240 total lines → thumb ≈ 1/6 of a 240px track (40px).
        let sb = scrollbar_metrics(240.0, 40, 200, 100).unwrap();
        assert!(
            (sb.thumb_height - 40.0).abs() < 0.5,
            "thumb_height {} should be ~40px",
            sb.thumb_height
        );
    }

    #[test]
    fn scrollbar_thumb_respects_minimum_height_and_stays_within_track() {
        // A one-line viewport over a huge history would give a sub-pixel thumb; clamp to the min.
        let sb = scrollbar_metrics(300.0, 1, 100_000, 50_000).unwrap();
        assert!(sb.thumb_height >= MIN_THUMB_HEIGHT - f32::EPSILON);
        assert!(sb.thumb_top >= 0.0 && sb.thumb_top + sb.thumb_height <= 300.0 + 0.01);
    }

    #[test]
    fn offset_for_thumb_top_inverts_scrollbar_metrics() {
        let (track, screen, history) = (200.0, 40, 200);
        for offset in [1usize, 50, 137, 200] {
            let sb = scrollbar_metrics(track, screen, history, offset).unwrap();
            let back = offset_for_thumb_top(track, screen, history, sb.thumb_top);
            assert_eq!(back, offset, "round-trip failed for offset {offset}");
        }
    }

    #[test]
    fn offset_for_thumb_top_clamps_positions_outside_the_track() {
        let (track, screen, history) = (200.0, 40, 200);
        // Dragged above the top → fully scrolled up (max offset).
        assert_eq!(offset_for_thumb_top(track, screen, history, -50.0), history);
        // Dragged below the bottom → back to the live bottom (offset 0).
        assert_eq!(offset_for_thumb_top(track, screen, history, 10_000.0), 0);
    }

    #[test]
    fn scrollbar_drag_targets_converge_and_do_not_accumulate() {
        // Models the apply loop: several drag messages are batched, then each resolves its delta
        // against the LIVE offset (as main.rs does via `target_offset_delta`) and alacritty applies
        // it additively and clamped. The absolute approach converges on the last drag position.
        let history = 100i32;
        let apply = |offset: i32, delta: i32| (offset + delta).clamp(0, history);
        let start = 40usize;
        let targets = [55usize, 72, 61];

        let mut live = start as i32;
        for &t in &targets {
            live = apply(live, target_offset_delta(live as usize, t));
        }
        assert_eq!(
            live, 61,
            "absolute targeting lands on the last drag position"
        );

        // The reported flicker: computing every batched delta against the pre-batch offset makes
        // the additive applies overshoot and clamp to an extreme instead of converging.
        let mut batched = start as i32;
        for &t in &targets {
            batched = apply(batched, t as i32 - start as i32);
        }
        assert_ne!(
            batched, 61,
            "stale relative deltas accumulate instead of converging"
        );
    }

    // --- Links (feature 031): hover, the gesture and the address hint ---
    //
    // The pure decisions are tested as functions; the wiring through the real `update`,
    // `mouse_interaction` and widget state is driven headless, with one tree kept across events so
    // the pane's hover and press survive between them as they do on screen.

    mod links {
        use super::*;
        use iced::advanced::widget::Tree;
        use iced::advanced::{clipboard, layout::Limits, Layout, Shell};
        use micold_core::link::{CellSpan, LinkOrigin, Target};
        use micold_core::protocol::grid::{
            CellExtras, GridFrame, StyleRun, WireCursor, WireCursorShape, WireLine,
        };
        use std::ops::Range;

        const COLS: u16 = 60;
        const ROWS: u16 = 6;
        const SENTENCE: &str = "See https://example.com/docs/page.html for details.";
        const ADDRESS: &str = "https://example.com/docs/page.html";
        /// The address's cells in [`SENTENCE`].
        const ADDRESS_COLS: Range<u16> = 4..38;
        const OUTSIDE: Point = Point::new(-40.0, -40.0);

        /// A spacer cell's style: the wide char before it is two cells wide.
        const SPACER_STYLE: WireStyle = WireStyle {
            flags: Flags::WIDE_CHAR_SPACER.bits(),
            ..DEFAULT_STYLE
        };
        const LEADING_SPACER_STYLE: WireStyle = WireStyle {
            flags: Flags::LEADING_WIDE_CHAR_SPACER.bits(),
            ..DEFAULT_STYLE
        };

        /// One grid row as a test spells it.
        #[derive(Clone, Default)]
        struct Row {
            text: String,
            wrapped: bool,
            declared: Vec<(Range<u16>, String)>,
            spacers: Vec<u16>,
            leading_spacers: Vec<u16>,
        }

        fn row(text: &str) -> Row {
            Row {
                text: text.to_string(),
                ..Row::default()
            }
        }

        impl Row {
            fn wrapped(mut self) -> Self {
                self.wrapped = true;
                self
            }
            fn declare(mut self, cols: Range<u16>, uri: &str) -> Self {
                self.declared.push((cols, uri.to_string()));
                self
            }
            fn spacer(mut self, col: u16) -> Self {
                self.spacers.push(col);
                self
            }
            fn leading_spacer(mut self, col: u16) -> Self {
                self.leading_spacers.push(col);
                self
            }
        }

        fn wire(id: i64, row: &Row, hyperlinks: &mut Vec<String>) -> WireLine {
            let mut text: String = row.text.clone();
            while text.chars().count() < COLS as usize {
                text.push(' ');
            }
            let style_at = |col: u16| {
                if row.spacers.contains(&col) {
                    1
                } else if row.leading_spacers.contains(&col) {
                    2
                } else {
                    0
                }
            };
            let mut runs: Vec<StyleRun> = Vec::new();
            for col in 0..text.chars().count() as u16 {
                let style = style_at(col);
                match runs.last_mut() {
                    Some(run) if run.style == style => run.len += 1,
                    _ => runs.push(StyleRun { len: 1, style }),
                }
            }
            let mut extras = Vec::new();
            for (cols, uri) in &row.declared {
                let index = match hyperlinks.iter().position(|u| u == uri) {
                    Some(i) => i,
                    None => {
                        hyperlinks.push(uri.clone());
                        hyperlinks.len() - 1
                    }
                };
                for col in cols.clone() {
                    extras.push(CellExtras {
                        col,
                        zerowidth: Vec::new(),
                        hyperlink: Some(index as u16),
                    });
                }
            }
            WireLine {
                id: LineId(id),
                text,
                runs,
                extras,
                wrapped: row.wrapped,
            }
        }

        fn frame(
            seq: u64,
            lines: &[(i64, Row)],
            viewport_top: i64,
            oldest: i64,
            mode: u32,
        ) -> GridFrame {
            let mut hyperlinks = Vec::new();
            let lines = lines
                .iter()
                .map(|(id, row)| wire(*id, row, &mut hyperlinks))
                .collect();
            GridFrame {
                session: SessionId::from_uuid(uuid::Uuid::nil()),
                seq,
                generation: 1,
                full: seq == 1,
                viewport_top: LineId(viewport_top),
                oldest_available: LineId(oldest),
                cols: COLS,
                rows: ROWS,
                cursor: WireCursor {
                    line: LineId(viewport_top),
                    col: 0,
                    shape: WireCursorShape::Block,
                    visible: false,
                    blinking: false,
                },
                styles: vec![DEFAULT_STYLE, SPACER_STYLE, LEADING_SPACER_STYLE],
                hyperlinks,
                lines,
                mode,
                input_serial: None,
            }
        }

        /// A grid whose screen starts at line 0 and holds `rows` from the top.
        fn screen(rows: &[Row]) -> GridCache {
            screen_in_mode(rows, 0)
        }

        fn screen_in_mode(rows: &[Row], mode: u32) -> GridCache {
            let lines: Vec<(i64, Row)> = rows
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, r)| (i as i64, r))
                .collect();
            let mut cache = GridCache::default();
            cache.apply(&frame(1, &lines, 0, 0, mode));
            cache
        }

        fn mouse_reporting() -> u32 {
            TermMode::MOUSE_REPORT_CLICK.bits()
        }

        /// The centre of the cell at `col`, `row` in a pane laid out at the window origin.
        fn at(col: u16, row: u16) -> Point {
            let m = CellMetrics::new(TERM_FONT_SIZE);
            Point::new(
                FOCUS_RING_WIDTH + (col as f32 + 0.5) * m.width,
                FOCUS_RING_WIDTH + (row as f32 + 0.5) * m.height,
            )
        }

        fn window() -> Size {
            let m = CellMetrics::new(TERM_FONT_SIZE);
            Size::new(
                COLS as f32 * m.width + 2.0 * FOCUS_RING_WIDTH,
                ROWS as f32 * m.height + 2.0 * FOCUS_RING_WIDTH,
            )
        }

        fn local() -> LinkContext {
            local_link_context()
        }

        fn url_link(address: &str, row: i64, cols: Range<u16>, origin: LinkOrigin) -> ResolvedLink {
            ResolvedLink {
                link: Link {
                    address: address.to_string(),
                    origin,
                    cells: vec![CellSpan { row, cols }],
                },
                display: address.to_string(),
                target: Target::Url(address.to_string()),
                needs_confirmation: false,
            }
        }

        use micold_core::link::Link;

        /// A pane kept across events, as the window keeps it.
        struct Pane {
            grid: GridCache,
            display_offset: usize,
            focused: bool,
            session: SessionId,
            context: LinkContext,
            tree: Option<Tree>,
            renderer: Renderer,
            /// Whether the last event marked the widget tree stale, which is what repaints it.
            invalidated: bool,
        }

        impl Pane {
            fn new(grid: GridCache) -> Self {
                Self {
                    grid,
                    display_offset: 0,
                    focused: true,
                    session: SessionId::from_uuid(uuid::Uuid::nil()),
                    context: local(),
                    tree: None,
                    renderer: super::presses::headless(),
                    invalidated: false,
                }
            }

            fn unfocused(mut self) -> Self {
                self.focused = false;
                self
            }

            fn send(&mut self, cursor: Point, event: Event) -> Vec<Message> {
                let mut element: Element<'_, Message> = TerminalPane::new(
                    &self.grid,
                    TermPalette::from_scheme(micold_core::theme::ColorScheme::Dark),
                )
                .display_offset(self.display_offset)
                .focused(self.focused)
                .session(self.session)
                .link_context(self.context.clone())
                .into();
                if self.tree.is_none() {
                    self.tree = Some(Tree::new(&element));
                }
                let tree = self.tree.as_mut().expect("just made");
                tree.diff(&element);
                let node = element.as_widget_mut().layout(
                    tree,
                    &self.renderer,
                    &Limits::new(Size::ZERO, window()),
                );
                let mut messages = Vec::new();
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    tree,
                    &event,
                    Layout::new(&node),
                    mouse::Cursor::Available(cursor),
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &Rectangle::with_size(window()),
                );
                self.invalidated = shell.are_widgets_invalid();
                messages
            }

            fn interaction(&mut self, cursor: Point) -> mouse::Interaction {
                let element: Element<'_, Message> = TerminalPane::new(
                    &self.grid,
                    TermPalette::from_scheme(micold_core::theme::ColorScheme::Dark),
                )
                .display_offset(self.display_offset)
                .focused(self.focused)
                .session(self.session)
                .link_context(self.context.clone())
                .into();
                let mut element = element;
                if self.tree.is_none() {
                    self.tree = Some(Tree::new(&element));
                }
                let tree = self.tree.as_mut().expect("just made");
                tree.diff(&element);
                let node = element.as_widget_mut().layout(
                    tree,
                    &self.renderer,
                    &Limits::new(Size::ZERO, window()),
                );
                element.as_widget().mouse_interaction(
                    tree,
                    Layout::new(&node),
                    mouse::Cursor::Available(cursor),
                    &Rectangle::with_size(window()),
                    &self.renderer,
                )
            }

            fn state(&self) -> &PaneState {
                self.tree
                    .as_ref()
                    .expect("the pane has seen an event")
                    .state
                    .downcast_ref::<PaneState>()
            }

            /// The link the pane marks now.
            fn marked(&self) -> Option<ResolvedLink> {
                let mode = TermMode::from_bits_truncate(self.grid.mode());
                marked_link(
                    self.state().hover.as_ref(),
                    mode.intersects(TermMode::MOUSE_MODE),
                    self.state().modifiers.shift(),
                )
                .cloned()
            }

            fn hover(&mut self, col: u16, row: u16) -> Vec<Message> {
                self.send(at(col, row), moved(at(col, row)))
            }

            fn hold(&mut self, cursor: Point, modifiers: keyboard::Modifiers) -> Vec<Message> {
                self.send(
                    cursor,
                    Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
                )
            }

            /// Press and release the left button at the cell, with whatever modifiers are held.
            fn click(&mut self, col: u16, row: u16) -> Vec<Message> {
                let mut published = self.send(at(col, row), press());
                published.extend(self.send(at(col, row), release()));
                published
            }
        }

        fn moved(position: Point) -> Event {
            Event::Mouse(mouse::Event::CursorMoved { position })
        }

        fn press() -> Event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        }

        fn release() -> Event {
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        }

        fn redraw() -> Event {
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            ))
        }

        fn activations(published: &[Message]) -> Vec<ResolvedLink> {
            published
                .iter()
                .filter_map(|m| match m {
                    Message::Session(SessionMsg::LinkActivated(link)) => Some(link.clone()),
                    _ => None,
                })
                .collect()
        }

        fn a_link() -> ResolvedLink {
            url_link(ADDRESS, 0, ADDRESS_COLS, LinkOrigin::Detected)
        }

        // --- The gesture, as a function (contract link-opening §1) ---

        fn press_at(
            cell: (u16, u16),
            command: bool,
            routing: PressRouting,
            cadence: click::Kind,
            marked: Option<&ResolvedLink>,
        ) -> GestureEvent<'_> {
            GestureEvent::LeftPress {
                cell,
                command,
                routing,
                cadence,
                marked,
            }
        }

        /// U106, G1.
        #[test]
        fn a_command_press_released_on_its_cell_opens_the_link() {
            let link = a_link();
            let step = link_gesture(
                press_at(
                    (10, 0),
                    true,
                    PressRouting::HandleLocally,
                    click::Kind::Single,
                    Some(&link),
                ),
                None,
            );
            let pressed = LinkPress {
                link: link.clone(),
                cell: (10, 0),
            };
            assert_eq!(
                step,
                GestureStep::Press(pressed.clone()),
                "the press waits for its release"
            );
            assert_eq!(
                link_gesture(
                    GestureEvent::LeftRelease {
                        cell: (10, 0),
                        marked: Some(&link)
                    },
                    Some(&pressed)
                ),
                GestureStep::Release(Some(link)),
                "released on the press cell, the link opens (FR-004)"
            );
        }

        /// U107, G2.
        #[test]
        fn leaving_the_press_cell_turns_the_press_into_a_selection_from_it() {
            let pressed = LinkPress {
                link: a_link(),
                cell: (10, 0),
            };
            assert_eq!(
                link_gesture(GestureEvent::CursorMoved { cell: (11, 0) }, Some(&pressed)),
                GestureStep::SelectFrom { col: 10, line: 0 }
            );
            assert_eq!(
                link_gesture(GestureEvent::CursorMoved { cell: (10, 0) }, Some(&pressed)),
                GestureStep::PassThrough,
                "moving within the press cell is still a click"
            );
            assert_eq!(
                link_gesture(GestureEvent::CursorMoved { cell: (11, 0) }, None),
                GestureStep::PassThrough,
                "without a link press, motion is today's"
            );
        }

        /// U108 and U110: G3, G4, and the repeated presses of G3b.
        #[test]
        fn a_plain_press_or_a_repeated_click_is_not_the_gesture() {
            let link = a_link();
            for cadence in [
                click::Kind::Single,
                click::Kind::Double,
                click::Kind::Triple,
            ] {
                assert_eq!(
                    link_gesture(
                        press_at(
                            (10, 0),
                            false,
                            PressRouting::HandleLocally,
                            cadence,
                            Some(&link)
                        ),
                        None
                    ),
                    GestureStep::PassThrough,
                    "a {cadence:?} press without the link modifier selects as today"
                );
            }
            for cadence in [click::Kind::Double, click::Kind::Triple] {
                assert_eq!(
                    link_gesture(
                        press_at(
                            (10, 0),
                            true,
                            PressRouting::HandleLocally,
                            cadence,
                            Some(&link)
                        ),
                        None
                    ),
                    GestureStep::PassThrough,
                    "a {cadence:?} press selects even with the link modifier (G3b)"
                );
            }
            assert_eq!(
                link_gesture(
                    press_at(
                        (10, 0),
                        true,
                        PressRouting::HandleLocally,
                        click::Kind::Single,
                        None
                    ),
                    None
                ),
                GestureStep::PassThrough,
                "a command press over plain text is not the gesture"
            );
        }

        /// U111, G5.
        #[test]
        fn a_press_routed_to_the_program_is_not_the_gesture() {
            let link = a_link();
            assert_eq!(
                link_gesture(
                    press_at(
                        (10, 0),
                        true,
                        PressRouting::MouseReport,
                        click::Kind::Single,
                        Some(&link)
                    ),
                    None
                ),
                GestureStep::PassThrough
            );
        }

        /// U114, G9.
        #[test]
        fn the_release_opens_the_link_under_the_pointer_then_or_nothing() {
            let pressed = LinkPress {
                link: a_link(),
                cell: (10, 0),
            };
            let other = url_link("https://other.example", 0, 4..25, LinkOrigin::Detected);
            assert_eq!(
                link_gesture(
                    GestureEvent::LeftRelease {
                        cell: (10, 0),
                        marked: Some(&other)
                    },
                    Some(&pressed)
                ),
                GestureStep::Release(Some(other))
            );
            assert_eq!(
                link_gesture(
                    GestureEvent::LeftRelease {
                        cell: (10, 0),
                        marked: None
                    },
                    Some(&pressed)
                ),
                GestureStep::Release(None)
            );
            assert_eq!(
                link_gesture(
                    GestureEvent::LeftRelease {
                        cell: (10, 0),
                        marked: Some(&a_link())
                    },
                    None
                ),
                GestureStep::PassThrough,
                "a release with no link press is today's"
            );
        }

        // --- The gesture through the pane ---

        /// U106 and U115: one activation, and nothing written, selected or scrolled.
        #[test]
        fn a_command_click_on_a_link_publishes_one_activation_and_nothing_else() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let published = pane.click(10, 0);
            assert_eq!(
                published,
                vec![Message::Session(SessionMsg::LinkActivated(a_link()))],
                "exactly one LinkActivated, and no TerminalBytes, selection or scroll (FR-014)"
            );
            assert_eq!(pane.state().link_press, None, "the release ends the press");
        }

        /// U107 through the pane.
        #[test]
        fn a_command_press_dragged_off_its_cell_selects_from_the_press_cell() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let pressed = pane.send(at(10, 0), press());
            assert!(
                !pressed
                    .iter()
                    .any(|m| matches!(m, Message::Session(SessionMsg::TerminalSelectStart { .. }))),
                "a link press starts no selection: {pressed:?}"
            );
            let dragged = pane.send(at(14, 0), moved(at(14, 0)));
            assert!(
                dragged.contains(&Message::Session(SessionMsg::TerminalSelectStart {
                    col: 10,
                    line: 0,
                    kind: SelectKind::Simple
                })),
                "the drag selects from where it was pressed: {dragged:?}"
            );
            let released = pane.send(at(14, 0), release());
            assert!(activations(&released).is_empty(), "a drag opens nothing");
        }

        /// U116 (SC-004) and U109 (G3b).
        #[test]
        fn plain_drags_and_double_and_triple_clicks_on_links_never_activate() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            let mut published = Vec::new();
            for script in 0..100 {
                match script % 3 {
                    0 => {
                        published.extend(pane.hover(6, 0));
                        published.extend(pane.send(at(6, 0), press()));
                        published.extend(pane.send(at(20, 0), moved(at(20, 0))));
                        published.extend(pane.send(at(20, 0), release()));
                    }
                    1 => {
                        published.extend(pane.hover(16, 0));
                        published.extend(pane.click(16, 0));
                        published.extend(pane.click(16, 0));
                    }
                    _ => {
                        published.extend(pane.hover(26, 0));
                        published.extend(pane.click(26, 0));
                        published.extend(pane.click(26, 0));
                        published.extend(pane.click(26, 0));
                    }
                }
            }
            assert_eq!(
                activations(&published).len(),
                0,
                "100 selections on links open nothing (SC-004)"
            );

            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let first = pane.click(10, 0);
            let second = pane.click(10, 0);
            assert_eq!(
                activations(&first).len(),
                1,
                "the first click of a modifier double-click opens"
            );
            assert_eq!(
                activations(&second).len(),
                0,
                "its second press never opens again"
            );
            assert!(
                second.contains(&Message::Session(SessionMsg::TerminalSelectStart {
                    col: 10,
                    line: 0,
                    kind: SelectKind::Semantic
                })),
                "the second press counts as a double click because the link press recorded it: {second:?}"
            );
        }

        /// U142, G7.
        #[test]
        fn a_middle_click_or_wheel_over_a_link_behaves_as_today() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let mut published = pane.send(
                at(10, 0),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)),
            );
            published.extend(pane.send(
                at(10, 0),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)),
            ));
            let wheel = pane.send(
                at(10, 0),
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
                }),
            );
            assert!(activations(&published).is_empty() && activations(&wheel).is_empty());
            assert_eq!(
                wheel,
                vec![Message::Session(SessionMsg::TerminalScrolled(1))],
                "the wheel scrolls as today"
            );
        }

        /// U111 through the pane: G5.
        #[test]
        fn under_mouse_reporting_a_command_click_goes_to_the_program() {
            let mut pane = Pane::new(screen_in_mode(&[row(SENTENCE)], mouse_reporting()));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let published = pane.click(10, 0);
            assert!(
                activations(&published).is_empty(),
                "the program owns the click (FR-016)"
            );
            assert_eq!(
                published
                    .iter()
                    .filter(|m| matches!(m, Message::Session(SessionMsg::TerminalBytes(_))))
                    .count(),
                2,
                "the press and the release are reported: {published:?}"
            );
        }

        /// U112, G6.
        #[test]
        fn under_mouse_reporting_shift_and_command_open_the_link() {
            let mut pane = Pane::new(screen_in_mode(&[row(SENTENCE)], mouse_reporting()));
            pane.hover(10, 0);
            pane.hold(
                at(10, 0),
                keyboard::Modifiers::COMMAND | keyboard::Modifiers::SHIFT,
            );
            let published = pane.click(10, 0);
            assert_eq!(activations(&published), vec![a_link()]);
            assert!(!published
                .iter()
                .any(|m| matches!(m, Message::Session(SessionMsg::TerminalBytes(_)))));
        }

        /// U113, G8.
        #[test]
        fn a_command_click_on_an_unfocused_pane_focuses_it_and_opens_the_link() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)])).unfocused();
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            let published = pane.click(10, 0);
            assert!(
                published.contains(&Message::Session(SessionMsg::TerminalFocused)),
                "{published:?}"
            );
            assert_eq!(activations(&published), vec![a_link()]);
        }

        /// U114 through the pane: the output changed between the press and the release.
        #[test]
        fn the_link_at_release_opens_when_the_output_changed_under_the_press() {
            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            pane.send(at(10, 0), press());
            let other = "See https://example.org/changed/page for details.";
            pane.grid.apply(&frame(2, &[(0, row(other))], 0, 0, 0));
            let published = pane.send(at(10, 0), release());
            let opened: Vec<String> = activations(&published)
                .into_iter()
                .map(|l| l.link.address)
                .collect();
            assert_eq!(opened, vec!["https://example.org/changed/page".to_string()]);

            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            pane.send(at(10, 0), press());
            pane.grid
                .apply(&frame(2, &[(0, row("plain text now"))], 0, 0, 0));
            assert!(
                activations(&pane.send(at(10, 0), release())).is_empty(),
                "no link at release, nothing opens"
            );
        }

        // --- Hover (research R2, R7) ---

        fn key(cell: (u16, u16), version: (u64, u64), context: &LinkContext) -> HoverKey<'_> {
            HoverKey {
                session: Some(SessionId::from_uuid(uuid::Uuid::nil())),
                context,
                cell,
                display_offset: 0,
                grid_version: version,
            }
        }

        fn cached(cell: (u16, u16), version: (u64, u64), hash: u64) -> HoverCache {
            HoverCache {
                session: Some(SessionId::from_uuid(uuid::Uuid::nil())),
                context: local(),
                cell,
                display_offset: 0,
                grid_version: version,
                rows: vec![0],
                rows_hash: hash,
                resolved: Some(a_link()),
            }
        }

        /// U118.
        #[test]
        fn hover_follows_the_pointer_from_cell_to_cell() {
            let ctx = local();
            let hover = cached((10, 0), (1, 1), 7);
            assert_eq!(
                hover_refresh(Some(&hover), &key((11, 0), (1, 1), &ctx), |_| 7),
                HoverRefresh::Resolve
            );
            assert_eq!(
                hover_refresh(None, &key((11, 0), (1, 1), &ctx), |_| 7),
                HoverRefresh::Resolve
            );

            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(1, 0);
            assert_eq!(pane.marked(), None, "`See` is no link");
            for col in [ADDRESS_COLS.start, 20, ADDRESS_COLS.end - 1] {
                pane.hover(col, 0);
                assert_eq!(
                    pane.marked(),
                    Some(a_link()),
                    "any char of the address marks exactly it (FR-007)"
                );
            }
            pane.hover(ADDRESS_COLS.end, 0);
            assert_eq!(pane.marked(), None, "the space after it is no link");
            pane.send(OUTSIDE, moved(OUTSIDE));
            assert_eq!(
                pane.state().hover.as_ref().and_then(|h| h.resolved.clone()),
                None,
                "leaving the pane drops the hover"
            );
        }

        /// U119.
        #[test]
        fn a_redraw_after_the_output_changed_under_a_resting_pointer_re_resolves_it() {
            let ctx = local();
            let hover = cached((10, 0), (1, 1), 7);
            assert_eq!(
                hover_refresh(Some(&hover), &key((10, 0), (1, 2), &ctx), |_| 8),
                HoverRefresh::Resolve
            );

            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            assert_eq!(pane.marked(), Some(a_link()));
            // A program redraws the row in place: the same LineId, other text.
            let other = "See https://example.org/changed/page for details.";
            pane.grid.apply(&frame(2, &[(0, row(other))], 0, 0, 0));
            pane.send(at(10, 0), redraw());
            assert_eq!(
                pane.marked().map(|l| l.link.address),
                Some("https://example.org/changed/page".to_string()),
                "the hint never shows the address the row used to hold (SC-006)"
            );
        }

        /// U120.
        #[test]
        fn a_grid_that_moved_without_touching_the_hovered_rows_keeps_the_hover() {
            let ctx = local();
            let hover = cached((10, 0), (1, 1), 7);
            assert_eq!(
                hover_refresh(Some(&hover), &key((10, 0), (1, 1), &ctx), |_| panic!(
                    "nothing moved, so nothing is hashed"
                )),
                HoverRefresh::Reuse
            );
            assert_eq!(
                hover_refresh(Some(&hover), &key((10, 0), (1, 2), &ctx), |rows| {
                    assert_eq!(
                        rows,
                        &[0],
                        "only the rows the link was read from are hashed"
                    );
                    7
                }),
                HoverRefresh::Revalidated
            );

            let mut pane = Pane::new(screen(&[row(SENTENCE), row("")]));
            pane.hover(10, 0);
            pane.grid
                .apply(&frame(2, &[(1, row("more output"))], 0, 0, 0));
            pane.send(at(10, 0), redraw());
            let hover = pane.state().hover.clone().expect("hovering");
            assert_eq!(hover.grid_version, (1, 2), "revalidated at the new version");
            assert_eq!(hover.resolved, Some(a_link()));
        }

        /// U161. A pointer past the grid's edge read no rows, so no row hash can say the grid grew
        /// under it (a resize that catches up with a wider pane): re-resolve on every grid move.
        #[test]
        fn a_hover_off_the_grid_re_resolves_when_the_grid_moves() {
            let ctx = local();
            let off_grid = HoverCache {
                rows: Vec::new(),
                resolved: None,
                ..cached((90, 0), (1, 1), 7)
            };
            assert_eq!(
                hover_refresh(Some(&off_grid), &key((90, 0), (1, 1), &ctx), |_| 7),
                HoverRefresh::Reuse,
                "nothing moved"
            );
            assert_eq!(
                hover_refresh(Some(&off_grid), &key((90, 0), (1, 2), &ctx), |_| 7),
                HoverRefresh::Resolve,
                "the grid moved, and no row vouches for the cell"
            );
        }

        /// U121.
        #[test]
        fn a_switch_of_session_or_context_re_resolves() {
            let ctx = local();
            let hover = cached((10, 0), (1, 1), 7);
            let mut other_session = key((10, 0), (1, 1), &ctx);
            other_session.session = Some(SessionId::new());
            assert_eq!(
                hover_refresh(Some(&hover), &other_session, |_| 7),
                HoverRefresh::Resolve
            );
            let sandboxed = LinkContext {
                host_names: vec!["devbox".to_string()],
                ..local()
            };
            assert_eq!(
                hover_refresh(Some(&hover), &key((10, 0), (1, 1), &sandboxed), |_| 7),
                HoverRefresh::Resolve
            );
            let mut scrolled = key((10, 0), (1, 1), &ctx);
            scrolled.display_offset = 3;
            assert_eq!(
                hover_refresh(Some(&hover), &scrolled, |_| 7),
                HoverRefresh::Resolve,
                "a scroll moves other lines under the pointer"
            );
        }

        /// U122.
        #[test]
        fn under_mouse_reporting_a_link_is_marked_only_while_shift_is_held() {
            let hover = cached((10, 0), (1, 1), 7);
            assert_eq!(marked_link(Some(&hover), false, false), Some(&a_link()));
            assert_eq!(marked_link(Some(&hover), true, false), None);
            assert_eq!(marked_link(Some(&hover), true, true), Some(&a_link()));

            let mut pane = Pane::new(screen_in_mode(&[row(SENTENCE)], mouse_reporting()));
            pane.hover(10, 0);
            assert_eq!(pane.marked(), None, "the pointer is the program's");
            pane.hold(at(10, 0), keyboard::Modifiers::SHIFT);
            assert_eq!(pane.marked(), Some(a_link()), "Shift takes it back");
            pane.hold(at(10, 0), keyboard::Modifiers::empty());
            assert_eq!(pane.marked(), None);
        }

        /// U123 (clarification 2026-09-16).
        #[test]
        fn the_pointer_is_a_hand_over_a_link_only_while_the_link_modifier_is_held() {
            assert_eq!(
                pane_interaction(true, true, keyboard::Modifiers::COMMAND),
                mouse::Interaction::Pointer
            );
            assert_eq!(
                pane_interaction(true, true, keyboard::Modifiers::empty()),
                mouse::Interaction::Text
            );
            assert_eq!(
                pane_interaction(true, false, keyboard::Modifiers::COMMAND),
                mouse::Interaction::Text
            );
            assert_eq!(
                pane_interaction(false, true, keyboard::Modifiers::COMMAND),
                mouse::Interaction::Idle
            );

            let mut pane = Pane::new(screen(&[row(SENTENCE)]));
            pane.hover(10, 0);
            assert_eq!(
                pane.interaction(at(10, 0)),
                mouse::Interaction::Text,
                "no modifier, no hand"
            );
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            assert_eq!(pane.interaction(at(10, 0)), mouse::Interaction::Pointer);
            pane.hover(1, 0);
            assert_eq!(
                pane.interaction(at(1, 0)),
                mouse::Interaction::Text,
                "plain text keeps the text pointer"
            );
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::empty());
            assert_eq!(
                pane.interaction(at(10, 0)),
                mouse::Interaction::Text,
                "released, back to the text pointer"
            );

            let mut pane = Pane::new(screen_in_mode(&[row(SENTENCE)], mouse_reporting()));
            pane.hover(10, 0);
            pane.hold(at(10, 0), keyboard::Modifiers::COMMAND);
            assert_eq!(
                pane.interaction(at(10, 0)),
                mouse::Interaction::Text,
                "under mouse reporting Ctrl alone is the program's"
            );
            pane.hold(
                at(10, 0),
                keyboard::Modifiers::COMMAND | keyboard::Modifiers::SHIFT,
            );
            assert_eq!(pane.interaction(at(10, 0)), mouse::Interaction::Pointer);
        }

        /// U124 (FR-022).
        #[test]
        fn hover_and_press_stay_in_the_pane_under_the_pointer() {
            let mut under = Pane::new(screen(&[row(SENTENCE)]));
            let mut other = Pane::new(screen(&[row(SENTENCE)]));
            let mut published = Vec::new();
            for (cursor, event) in [
                (at(10, 0), moved(at(10, 0))),
                (
                    at(10, 0),
                    Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::COMMAND,
                    )),
                ),
                (at(10, 0), press()),
                (at(10, 0), release()),
            ] {
                published.extend(under.send(cursor, event.clone()));
                let elsewhere = other.send(OUTSIDE, event);
                assert!(other
                    .state()
                    .hover
                    .as_ref()
                    .and_then(|h| h.resolved.as_ref())
                    .is_none());
                assert_eq!(other.state().link_press, None);
                assert!(
                    activations(&elsewhere).is_empty(),
                    "the other pane opens nothing"
                );
            }
            assert_eq!(
                activations(&published).len(),
                1,
                "the pane under the pointer opens it once"
            );
        }

        // --- The address hint (research R7) ---

        fn content() -> Rectangle {
            Rectangle::new(Point::new(3.0, 3.0), Size::new(600.0, 18.2 * 20.0))
        }

        /// U125.
        #[test]
        fn the_hint_sits_bottom_left_by_default() {
            let hint = Size::new(200.0, 22.0);
            let rect = link_hint_rect(content(), 0, hint);
            assert_eq!(
                rect.position(),
                Point::new(content().x, content().y + content().height - hint.height)
            );
            assert_eq!(rect.size(), hint);
        }

        /// U126.
        #[test]
        fn the_hint_moves_top_left_when_the_pointer_nears_the_bottom() {
            let hint = Size::new(200.0, 22.0);
            let m = CellMetrics::new(TERM_FONT_SIZE);
            let bottom = content().y + content().height;
            let rows = (content().height / m.height) as u16;
            // The last row whose cells end above the hint's band, and the one below it.
            let clear = (0..rows)
                .rev()
                .find(|r| content().y + (*r as f32 + 1.0) * m.height <= bottom - hint.height)
                .unwrap();
            assert_eq!(
                link_hint_rect(content(), clear, hint).y,
                bottom - hint.height,
                "row {clear} keeps it bottom-left"
            );
            assert_eq!(
                link_hint_rect(content(), clear + 1, hint).position(),
                content().position(),
                "row {} flips it top-left",
                clear + 1
            );
            assert_eq!(
                link_hint_rect(content(), rows - 1, hint).position(),
                content().position()
            );
        }

        /// U127.
        #[test]
        fn the_hint_always_lies_inside_the_content() {
            for hint in [
                Size::new(10.0, 10.0),
                Size::new(900.0, 22.0),
                Size::new(100.0, 900.0),
            ] {
                for row in [0u16, 5, 19, 40] {
                    let rect = link_hint_rect(content(), row, hint);
                    let c = content();
                    assert!(
                        rect.x >= c.x
                            && rect.y >= c.y
                            && rect.x + rect.width <= c.x + c.width + 0.01
                            && rect.y + rect.height <= c.y + c.height + 0.01,
                        "{rect:?} leaves {c:?} (hint {hint:?}, row {row})"
                    );
                }
            }
        }

        /// U128.
        #[test]
        fn a_long_address_is_elided_in_the_middle_and_display_stays_whole() {
            let long = "https://example.com/a/very/long/path/that/does/not/fit/in/the/pane.html";
            let label = elide_middle(long, 24);
            assert_eq!(label.chars().count(), 24);
            assert!(label.contains('…'));
            assert!(
                label.starts_with("https://exa") && label.ends_with("pane.html"),
                "{label}"
            );
            assert_eq!(
                elide_middle(ADDRESS, 60),
                ADDRESS,
                "what fits is shown whole"
            );

            let grid = screen(&[row(long)]);
            let ctx = local();
            let hover = resolve_hover(&grid, &key((5, 0), (grid.generation(), grid.seq()), &ctx));
            assert_eq!(
                hover.resolved.map(|r| r.display),
                Some(long.to_string()),
                "the model keeps it complete (SC-006)"
            );
        }

        // --- Declared links (T037, U129) ---

        fn resolved_at(grid: &GridCache, col: u16, row: u16) -> Option<ResolvedLink> {
            let ctx = local();
            resolve_hover(
                grid,
                &key((col, row), (grid.generation(), grid.seq()), &ctx),
            )
            .resolved
        }

        /// U129: the run, and its declared address as `display`.
        #[test]
        fn hovering_a_declared_run_marks_exactly_the_run_and_resolves_its_address() {
            let grid =
                screen(&[row("Read the docs today").declare(9..13, "https://example.com/manual")]);
            let link = resolved_at(&grid, 10, 0).expect("docs is a link");
            assert_eq!(link.display, "https://example.com/manual");
            assert_eq!(link.link.origin, LinkOrigin::Declared);
            assert_eq!(
                link.link.cells,
                vec![CellSpan {
                    row: 0,
                    cols: 9..13
                }]
            );
            assert_eq!(
                resolved_at(&grid, 8, 0),
                None,
                "the space before the run is no link"
            );
        }

        /// U129: US2 scenario 3.
        #[test]
        fn adjacent_runs_with_different_addresses_are_two_links() {
            let grid = screen(&[row("docsother")
                .declare(0..4, "https://example.com/manual")
                .declare(4..9, "https://example.com/other")]);
            assert_eq!(
                resolved_at(&grid, 3, 0).map(|l| (l.display, l.link.cells)),
                Some((
                    "https://example.com/manual".to_string(),
                    vec![CellSpan { row: 0, cols: 0..4 }]
                ))
            );
            assert_eq!(
                resolved_at(&grid, 4, 0).map(|l| (l.display, l.link.cells)),
                Some((
                    "https://example.com/other".to_string(),
                    vec![CellSpan { row: 0, cols: 4..9 }]
                ))
            );
        }

        /// U129: US2 scenario 4.
        #[test]
        fn same_address_runs_apart_are_marked_one_at_a_time() {
            let uri = "https://example.com/manual";
            let grid = screen(&[row("docs and docs").declare(0..4, uri).declare(9..13, uri)]);
            assert_eq!(
                resolved_at(&grid, 1, 0).map(|l| l.link.cells),
                Some(vec![CellSpan { row: 0, cols: 0..4 }])
            );
            assert_eq!(
                resolved_at(&grid, 10, 0).map(|l| l.link.cells),
                Some(vec![CellSpan {
                    row: 0,
                    cols: 9..13
                }])
            );
        }

        /// U129: US2 scenario 5.
        #[test]
        fn address_shaped_text_resolves_to_the_declared_address() {
            let grid = screen(&[row("https://a.example").declare(0..17, "https://b.example")]);
            let link = resolved_at(&grid, 3, 0).expect("declared");
            assert_eq!(
                (link.display.as_str(), link.target),
                (
                    "https://b.example",
                    Target::Url("https://b.example".to_string())
                )
            );
        }

        // --- The rows the pane lends (T028; M1 review B) ---

        /// U156.
        #[test]
        fn rows_above_everything_printed_read_as_empty_and_rows_not_held_as_unavailable() {
            // A session's first line: nothing was ever above it.
            let grid = screen(&[row("https://a.example/x")]);
            let rows = GridRows::new(&grid, 0);
            assert_eq!(
                rows.text(-1),
                Some(""),
                "above the first line printed is an empty row"
            );
            assert!(!rows.wrapped(-1));
            assert_eq!(
                resolved_at(&grid, 0, 0).map(|l| l.link.address),
                Some("https://a.example/x".to_string()),
                "so an address at column 0 of the first line is a link"
            );

            // The alternate screen (`less`, `vim`): no history, so nothing above its top.
            let mut alt = GridCache::default();
            alt.apply(&frame(
                1,
                &[(500, row("https://a.example/x"))],
                500,
                500,
                TermMode::ALT_SCREEN.bits(),
            ));
            assert_eq!(GridRows::new(&alt, 0).text(-1), Some(""));

            // History trimmed below the watermark, and history not fetched yet, may continue.
            let mut deep = GridCache::default();
            deep.apply(&frame(1, &[(500, row("x"))], 500, 400, 0));
            let rows = GridRows::new(&deep, 0);
            assert_eq!(rows.text(-150), None, "trimmed from scrollback");
            assert_eq!(rows.text(-50), None, "not cached yet");
            assert_eq!(rows.text(1), None, "below the screen");
            assert_eq!(rows.text(0), Some(format!("{:<60}", "x").as_str()));
        }

        /// U157.
        #[test]
        fn spacer_cells_come_from_the_style_run_flags() {
            let grid = screen(&[row("界 x ")
                .spacer(1)
                .leading_spacer(4)
                .wrapped()
                .declare(0..2, "https://example.com/wide")]);
            let rows = GridRows::new(&grid, 0);
            assert!(!rows.spacer(0, 0));
            assert!(rows.spacer(0, 1), "a wide char's second cell");
            assert!(!rows.spacer(0, 2));
            assert!(
                rows.spacer(0, 4),
                "the padding a wrapped wide char left at the row's end"
            );
            assert!(rows.wrapped(0));
            assert_eq!(rows.hyperlink(0, 1), Some("https://example.com/wide"));
            assert_eq!(rows.hyperlink(0, 3), None);
        }

        /// The scrollback offset is part of the row mapping.
        #[test]
        fn rows_follow_the_scrollback_offset() {
            let mut grid = GridCache::default();
            grid.apply(&frame(
                1,
                &[(7, row("history")), (10, row("screen"))],
                10,
                0,
                0,
            ));
            let rows = GridRows::new(&grid, 3);
            assert_eq!(rows.text(0).map(str::trim_end), Some("history"));
            assert_eq!(rows.text(3).map(str::trim_end), Some("screen"));
        }

        // --- M3 review fixes ---

        /// U158: a link press released outside the pane opens nothing, even when the press was on
        /// the top-left cell a missing pointer position would clamp to.
        #[test]
        fn a_link_press_released_outside_the_pane_opens_nothing() {
            let mut pane = Pane::new(screen(&[row(ADDRESS)]));
            pane.hover(0, 0);
            pane.hold(at(0, 0), keyboard::Modifiers::COMMAND);
            pane.send(at(0, 0), press());
            assert!(
                pane.state().link_press.is_some(),
                "the press is the gesture"
            );
            let published = pane.send(Point::new(-40.0, -40.0), release());
            assert_eq!(
                activations(&published),
                vec![],
                "released outside, nothing opens"
            );
            assert_eq!(pane.state().link_press, None, "the release ends the press");
        }

        /// U159: over the scrollbar strip a press pages the view, so a link there is not marked.
        #[test]
        fn a_link_under_the_scrollbar_strip_is_not_marked() {
            let text = format!("{:>width$}", ADDRESS, width = COLS as usize);
            // The strip shows only while scrolled up: the link is the top row one line up.
            let mut lines: Vec<(i64, Row)> = (0..9).map(|i| (i, row("history"))).collect();
            lines.push((9, row(&text)));
            lines.push((10, row("$ ")));
            let mut grid = GridCache::default();
            grid.apply(&frame(1, &lines, 10, 0, 0));
            let mut pane = Pane::new(grid);
            pane.display_offset = 1;
            pane.hover(COLS - 1, 0);
            pane.hold(at(COLS - 1, 0), keyboard::Modifiers::COMMAND);
            assert_eq!(pane.marked(), None, "no link is marked under the strip");
            assert_eq!(
                pane.interaction(at(COLS - 1, 0)),
                mouse::Interaction::Text,
                "no hand over the strip"
            );
            pane.hover(COLS - 20, 0);
            assert!(
                pane.marked().is_some(),
                "the same link left of the strip is marked"
            );
        }

        /// U160: moving to another row of the same marked link repaints, since the hint's side
        /// follows the pointer's row.
        #[test]
        fn moving_to_another_row_of_the_marked_link_repaints() {
            let wrapped = &ADDRESS[..30];
            let rest = &ADDRESS[30..];
            let rows: Vec<Row> = (0..ROWS)
                .map(|r| match r {
                    0 => row(&format!("{:>width$}", wrapped, width = COLS as usize)).wrapped(),
                    1 => row(rest),
                    _ => row(""),
                })
                .collect();
            let mut pane = Pane::new(screen(&rows));
            pane.hover(COLS - 1, 0);
            assert!(pane.marked().is_some(), "the wrapped link is marked");
            pane.hover(0, 1);
            assert!(
                pane.invalidated,
                "the pointer changed rows over the same link"
            );
            pane.hover(1, 1);
            assert!(
                !pane.invalidated,
                "a move within the row changes nothing drawn"
            );
        }
    }
}

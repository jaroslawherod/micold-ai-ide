//! `terminal_pane` — the reusable colour terminal widget (Constitution Principle VIII, feature
//! 006). A custom iced `advanced::Widget` that renders a session's `alacritty_terminal` grid on
//! a canvas with full ANSI colour + text styling, and (US1) focuses on click.
//!
//! Adapted from `iced_term 0.6.0` `view.rs` (MIT © Ilya Shvyryalkin). Key/mouse input and the
//! full focus gate land in feature 006 US2/US3; this file covers colour rendering + click focus.

use crate::ui::terminal::{
    cell_colors, cell_font, shows_cursor, CellMetrics, RuntimeTerminal, TermPalette, TERM_FONT_SIZE,
};
use iced::advanced::clipboard::Kind as ClipboardKind;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::{click, Click};
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{
    alignment, event, keyboard, mouse, Color, Element, Event, Length, Point, Rectangle, Renderer,
    Size, Theme,
};
use micold_ai_ide::app::{Message, SelectKind};
use micold_ai_ide::keymap::{self, KeyOutput};

/// Per-widget interaction state (drag + tracked modifiers + click cadence for single/double/
/// triple selection).
#[derive(Default)]
struct PaneState {
    dragging: bool,
    modifiers: keyboard::Modifiers,
    last_click: Option<Click>,
    /// Last reported grid size, to detect resizes and notify the PTY (FR-014/FR-015).
    last_grid: (u16, u16),
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
}

/// The grid cell (col, line) under a cursor position within `bounds`.
fn grid_at(pos: Point, bounds: Rectangle, metrics: CellMetrics) -> (u16, u16) {
    let col = ((pos.x - bounds.x) / metrics.width).floor().max(0.0) as u16;
    let line = ((pos.y - bounds.y) / metrics.height).floor().max(0.0) as u16;
    (col, line)
}

/// The viewport row (0 = top) for a `buffer_line` yielded by `display_iter`, given the current
/// scrollback `display_offset` and the number of visible `rows`. Buffer lines are 0 at the screen
/// top and negative up in the scrollback history; `None` when the line is outside the viewport.
pub(crate) fn viewport_row(buffer_line: i32, display_offset: usize, rows: usize) -> Option<usize> {
    let row = buffer_line + display_offset as i32;
    usize::try_from(row).ok().filter(|&r| r < rows)
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
pub(crate) fn press_routing(focused: bool, mouse_mode: bool, shift: bool) -> PressRouting {
    if focused && mouse_mode && !shift {
        PressRouting::MouseReport
    } else {
        PressRouting::HandleLocally
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
pub(crate) fn target_offset_delta(current: usize, target: usize) -> i32 {
    target as i32 - current as i32
}

/// The colour terminal widget for a live session runtime (Principle VIII builder form):
/// `TerminalPane::new(rt, palette).focused(bool).into()`.
pub struct TerminalPane<'a> {
    rt: &'a RuntimeTerminal,
    palette: TermPalette,
    focused: bool,
}

impl<'a> TerminalPane<'a> {
    /// A terminal pane rendering `rt`'s grid with `palette`. Unfocused by default.
    pub fn new(rt: &'a RuntimeTerminal, palette: TermPalette) -> Self {
        Self {
            rt,
            palette,
            focused: false,
        }
    }

    /// Mark the pane focused (routes keyboard input to it; no border is drawn for this).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
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
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let metrics = CellMetrics::new(TERM_FONT_SIZE);
        let default_bg = self.palette.background();

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

            let content = self.rt.renderable();
            let cursor_point = content.cursor.point;
            let show_cursor = shows_cursor(content.mode);
            // The active text-selection range (if any); each cell inside it is drawn highlighted
            // with fg/bg swapped (FR-013).
            let selection = content.selection;
            // Buffer lines from `display_iter` are 0 at the screen top and negative up in the
            // scrollback; map them to viewport rows using the current scroll offset (FR-016).
            let display_offset = content.display_offset;
            let rows = self.rt.size().1 as usize;

            for indexed in content.display_iter {
                let Some(row) = viewport_row(indexed.point.line.0, display_offset, rows) else {
                    continue;
                };
                let col = indexed.point.column.0 as f32;
                let x = bounds.x + col * metrics.width;
                let y = bounds.y + (row as f32) * metrics.height;

                let flags = indexed.cell.flags;
                let selected = selection.is_some_and(|range| range.contains(indexed.point));
                let (fg, bg) = cell_colors(
                    &self.palette,
                    indexed.cell.fg,
                    indexed.cell.bg,
                    flags,
                    selected,
                );

                // Per-cell background when it differs from the default.
                if bg != default_bg {
                    frame.fill_rectangle(
                        iced::Point::new(x, y),
                        Size::new(metrics.width, metrics.height),
                        bg,
                    );
                }

                // Cursor block (drawn behind the glyph).
                if show_cursor && indexed.point == cursor_point {
                    frame.fill_rectangle(
                        iced::Point::new(x, y),
                        Size::new(metrics.width, metrics.height),
                        self.palette.foreground(),
                    );
                }

                let ch = indexed.cell.c;
                if ch != ' ' && ch != '\t' && ch != '\0' {
                    // Invert the glyph over the cursor block for legibility.
                    let glyph_fg = if show_cursor && indexed.point == cursor_point {
                        default_bg
                    } else {
                        fg
                    };
                    frame.fill_text(Text {
                        content: ch.to_string(),
                        position: iced::Point::new(
                            x + metrics.width / 2.0,
                            y + metrics.height / 2.0,
                        ),
                        color: glyph_fg,
                        size: iced::Pixels(metrics.size),
                        font: cell_font(flags),
                        horizontal_alignment: alignment::Horizontal::Center,
                        vertical_alignment: alignment::Vertical::Center,
                        line_height: iced::widget::text::LineHeight::Absolute(iced::Pixels(
                            metrics.height,
                        )),
                        shaping: iced::widget::text::Shaping::Advanced,
                    });
                }

                // Underline / strikethrough.
                use alacritty_terminal::term::cell::Flags;
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

            // Scrollback scrollbar: a right-edge track + thumb, shown only while scrolled back into
            // history so it stays out of the way during a live session (FR-016).
            let screen_lines = self.rt.size().1 as usize;
            if let Some(sb) = scrollbar_metrics(
                bounds.height,
                screen_lines,
                self.rt.history_size(),
                display_offset,
            ) {
                let fg = self.palette.foreground();
                let track_x = bounds.x + bounds.width - SCROLLBAR_WIDTH;
                frame.fill_rectangle(
                    iced::Point::new(track_x, bounds.y),
                    Size::new(SCROLLBAR_WIDTH, bounds.height),
                    Color { a: 0.08, ..fg },
                );
                let pad = 2.0;
                let thumb_w = SCROLLBAR_WIDTH - pad * 2.0;
                let thumb = Path::rounded_rectangle(
                    iced::Point::new(track_x + pad, bounds.y + sb.thumb_top),
                    Size::new(thumb_w, sb.thumb_height),
                    (thumb_w / 2.0).into(),
                );
                frame.fill(&thumb, Color { a: 0.5, ..fg });
            }
        }

        renderer.draw_geometry(frame.into_geometry());
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<PaneState>();
        let bounds = layout.bounds();
        let metrics = CellMetrics::new(TERM_FONT_SIZE);

        // Report the visible grid size to the process whenever it changes (FR-014/FR-015).
        let grid = metrics.grid_size(bounds.width, bounds.height);
        if grid != state.last_grid {
            state.last_grid = grid;
            shell.publish(Message::TerminalResized {
                cols: grid.0,
                rows: grid.1,
            });
        }

        // Track modifiers (even when unfocused) so Shift-forces-selection works (FR-013b).
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(m)) = &event {
            state.modifiers = *m;
        }

        // A click outside the focused pane releases focus back to the app (FR-011). The event is
        // not captured, so the click still reaches whatever is under it.
        if self.focused {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = &event {
                if !cursor.is_over(bounds) {
                    shell.publish(Message::TerminalFocusReleased);
                }
            }
        }

        // ---- Scrollbar: drag the right-edge thumb or click the track to page (FR-016). Handled
        // before selection so a drag on the scrollbar never starts a text selection.
        {
            let screen_lines = self.rt.size().1 as usize;
            let history = self.rt.history_size();
            let offset = self.rt.display_offset();
            match &event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if let Some(pos) = cursor.position() {
                        let over_strip = cursor.is_over(bounds)
                            && pos.x >= bounds.x + bounds.width - SCROLLBAR_WIDTH;
                        if over_strip {
                            if let Some(sb) =
                                scrollbar_metrics(bounds.height, screen_lines, history, offset)
                            {
                                let thumb_y0 = bounds.y + sb.thumb_top;
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
                                    shell.publish(Message::TerminalScrolled(delta));
                                }
                                return event::Status::Captured;
                            }
                        }
                    }
                }
                Event::Mouse(mouse::Event::CursorMoved { position })
                    if state.scrollbar_grab.is_some() =>
                {
                    let grab_dy = state.scrollbar_grab.unwrap_or(0.0);
                    let thumb_top = position.y - bounds.y - grab_dy;
                    let target =
                        offset_for_thumb_top(bounds.height, screen_lines, history, thumb_top);
                    // Publish an absolute target, not a relative delta: several CursorMoved events
                    // are batched before the app update runs, so relative deltas computed against
                    // the pre-batch offset would accumulate and jump the view (drag flicker).
                    shell.publish(Message::TerminalScrolledTo(target));
                    return event::Status::Captured;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    if state.scrollbar_grab.is_some() =>
                {
                    state.scrollbar_grab = None;
                    return event::Status::Captured;
                }
                _ => {}
            }
        }

        // ---- Mouse: selection (local) + mouse reporting (process input) ----
        match &event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                if !self.focused {
                    shell.publish(Message::TerminalFocused);
                }
                let pos = cursor.position().unwrap_or_default();
                let (col, line) = grid_at(pos, bounds, metrics);
                let shift = state.modifiers.shift();
                if press_routing(self.focused, self.rt.mouse_mode(), shift)
                    == PressRouting::MouseReport
                {
                    if let Some(seq) = self.rt.mouse_report_bytes(
                        0,
                        col,
                        line,
                        true,
                        to_keymap_mods(state.modifiers),
                    ) {
                        shell.publish(Message::TerminalBytes(seq));
                    }
                    // Remember the held button so its release is reported too (FR-013a).
                    state.reporting_button = Some(0);
                    state.reported_cell = Some((col, line));
                } else {
                    let c = Click::new(pos, mouse::Button::Left, state.last_click);
                    let kind = match c.kind() {
                        click::Kind::Single => SelectKind::Simple,
                        click::Kind::Double => SelectKind::Semantic,
                        click::Kind::Triple => SelectKind::Lines,
                    };
                    state.last_click = Some(c);
                    state.dragging = true;
                    shell.publish(Message::TerminalSelectStart { col, line, kind });
                }
                return event::Status::Captured;
            }
            // Motion while a reported button is held (FR-013a). Only for processes that asked
            // for motion (MOUSE_DRAG / MOUSE_MOTION), and only once per grid cell crossed —
            // reporting every pixel would flood the PTY. Button code +32 marks it as motion.
            Event::Mouse(mouse::Event::CursorMoved { position })
                if state.reporting_button.is_some() =>
            {
                let (col, line) = grid_at(*position, bounds, metrics);
                let button = state.reporting_button.unwrap_or(0);
                if self.rt.mouse_motion_mode() && state.reported_cell != Some((col, line)) {
                    if let Some(seq) = self.rt.mouse_report_bytes(
                        button + 32,
                        col,
                        line,
                        true,
                        to_keymap_mods(state.modifiers),
                    ) {
                        shell.publish(Message::TerminalBytes(seq));
                    }
                }
                state.reported_cell = Some((col, line));
                return event::Status::Captured;
            }
            // Release of a reported button (FR-013a). Must come before the selection arms: in
            // mouse mode `state.dragging` was never set, so the release previously fell through
            // to `_ => {}` and the process saw a button that never came up.
            Event::Mouse(mouse::Event::ButtonReleased(_)) if state.reporting_button.is_some() => {
                let button = state.reporting_button.take().unwrap_or(0);
                state.reported_cell = None;
                let pos = cursor.position().unwrap_or_default();
                let (col, line) = grid_at(pos, bounds, metrics);
                if let Some(seq) = self.rt.mouse_report_bytes(
                    button,
                    col,
                    line,
                    false,
                    to_keymap_mods(state.modifiers),
                ) {
                    shell.publish(Message::TerminalBytes(seq));
                }
                return event::Status::Captured;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                let (col, line) = grid_at(*position, bounds, metrics);
                shell.publish(Message::TerminalSelectUpdate { col, line });
                return event::Status::Captured;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                // Auto-copy the selection to the clipboard on release (FR-013).
                let selected = self.rt.selectable_content();
                if !selected.is_empty() {
                    clipboard.write(ClipboardKind::Standard, selected);
                }
                return event::Status::Captured;
            }
            // Middle-click pastes the clipboard into the focused process (FR-013) — unless the
            // process is tracking the mouse, in which case the gesture belongs to it (FR-013a).
            // Shift forces the local behaviour, matching the left-button rule above and the
            // convention every other terminal follows.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
                if self.focused && cursor.is_over(bounds) =>
            {
                let shift = state.modifiers.shift();
                if self.rt.mouse_mode() && !shift {
                    let (col, line) =
                        grid_at(cursor.position().unwrap_or_default(), bounds, metrics);
                    if let Some(seq) = self.rt.mouse_report_bytes(
                        1,
                        col,
                        line,
                        true,
                        to_keymap_mods(state.modifiers),
                    ) {
                        shell.publish(Message::TerminalBytes(seq));
                    }
                    state.reporting_button = Some(1);
                    state.reported_cell = Some((col, line));
                } else if let Some(pasted) = clipboard.read(ClipboardKind::Standard) {
                    shell.publish(Message::TerminalBytes(pasted.into_bytes()));
                }
                return event::Status::Captured;
            }
            // Right-click opens the copy/paste context menu at the cursor (FR-013) — unless the
            // process is tracking the mouse, in which case it is forwarded (FR-013a). Shift
            // forces the menu, so it is always reachable.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if cursor.is_over(bounds) =>
            {
                let pos = cursor.position().unwrap_or_default();
                let shift = state.modifiers.shift();
                if press_routing(self.focused, self.rt.mouse_mode(), shift)
                    == PressRouting::MouseReport
                {
                    let (col, line) = grid_at(pos, bounds, metrics);
                    if let Some(seq) = self.rt.mouse_report_bytes(
                        2,
                        col,
                        line,
                        true,
                        to_keymap_mods(state.modifiers),
                    ) {
                        shell.publish(Message::TerminalBytes(seq));
                    }
                    state.reporting_button = Some(2);
                    state.reported_cell = Some((col, line));
                } else {
                    let x = (pos.x - bounds.x).max(0.0) as u16;
                    let y = (pos.y - bounds.y).max(0.0) as u16;
                    shell.publish(Message::TerminalContextMenuOpened { x, y });
                }
                return event::Status::Captured;
            }
            // Wheel scrolls the local scrollback, or forwards to a mouse-reporting program on
            // the alternate screen (FR-016 + wheel edge case).
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                // Sub-line deltas from a high-resolution touchpad accumulate rather than being
                // rounded away, or fine-grained scrolling would never move at all (BUG-002).
                let lines = wheel_lines(*delta, metrics.height, &mut state.scroll_residual);
                match wheel_routing(lines, self.focused, self.rt.mouse_mode()) {
                    WheelRouting::MouseReport { button, count } => {
                        let (col, line) = cursor
                            .position()
                            .map(|p| grid_at(p, bounds, metrics))
                            .unwrap_or((0, 0));
                        let km = to_keymap_mods(state.modifiers);
                        for _ in 0..count {
                            if let Some(seq) =
                                self.rt.mouse_report_bytes(button, col, line, true, km)
                            {
                                shell.publish(Message::TerminalBytes(seq));
                            }
                        }
                        return event::Status::Captured;
                    }
                    WheelRouting::ScrollLocally { lines } => {
                        shell.publish(Message::TerminalScrolled(lines));
                        return event::Status::Captured;
                    }
                    // Sub-line travel is banked in the residual; leave the event unhandled.
                    WheelRouting::Ignore => {}
                }
            }
            _ => {}
        }

        // Keyboard input reaches the process ONLY while focused (FR-006/FR-008/FR-009).
        if !self.focused {
            return event::Status::Ignored;
        }
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) = event
        {
            let Some(k) = to_keymap_key(&key) else {
                return event::Status::Ignored;
            };
            let input = keymap::KeyInput {
                key: k,
                mods: to_keymap_mods(modifiers),
                text: text.map(|t| t.to_string()),
            };
            return match keymap::encode(&input, self.rt.key_term_mode()) {
                KeyOutput::Bytes(bytes) => {
                    shell.publish(Message::TerminalBytes(bytes));
                    event::Status::Captured
                }
                KeyOutput::ReleaseFocus => {
                    shell.publish(Message::TerminalFocusReleased);
                    event::Status::Captured
                }
                KeyOutput::Copy => {
                    clipboard.write(ClipboardKind::Standard, self.rt.selectable_content());
                    event::Status::Captured
                }
                KeyOutput::Paste => {
                    if let Some(pasted) = clipboard.read(ClipboardKind::Standard) {
                        shell.publish(Message::TerminalBytes(pasted.into_bytes()));
                    }
                    event::Status::Captured
                }
                KeyOutput::Ignore => event::Status::Ignored,
            };
        }
        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::Idle
        }
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
    fn viewport_row_maps_buffer_line_to_screen_row_at_zero_offset() {
        // With no scrollback offset, buffer lines are viewport rows 1:1.
        assert_eq!(viewport_row(0, 0, 30), Some(0));
        assert_eq!(viewport_row(29, 0, 30), Some(29));
    }

    #[test]
    fn viewport_row_shifts_scrollback_history_into_the_viewport() {
        // Scrolled up by 3: the three revealed history lines (-3..=-1) fill the top rows and
        // every on-screen line slides down by 3 — nothing is dropped from the bottom (FR-016).
        // This is the reported bug: without the offset, negative history lines were discarded and
        // the visible text stayed frozen while the bottom rows blanked.
        assert_eq!(viewport_row(-3, 3, 30), Some(0));
        assert_eq!(viewport_row(-1, 3, 30), Some(2));
        assert_eq!(viewport_row(0, 3, 30), Some(3));
        assert_eq!(viewport_row(26, 3, 30), Some(29));
    }

    #[test]
    fn viewport_row_rejects_lines_outside_the_visible_area() {
        // One line above the top row and one below the bottom row both map to nothing.
        assert_eq!(viewport_row(-4, 3, 30), None);
        assert_eq!(viewport_row(27, 3, 30), None);
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
}

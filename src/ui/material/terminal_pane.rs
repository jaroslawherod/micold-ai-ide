//! `terminal_pane` — the reusable colour terminal widget (Constitution Principle VIII, feature
//! 006). A custom iced `advanced::Widget` that renders a session's `alacritty_terminal` grid on
//! a canvas with full ANSI colour + text styling, and (US1) focuses on click.
//!
//! Adapted from `iced_term 0.6.0` `view.rs` (MIT © Ilya Shvyryalkin). Key/mouse input and the
//! full focus gate land in feature 006 US2/US3; this file covers colour rendering + click focus.

use crate::ui::terminal::{cell_colors, cell_font, shows_cursor, CellMetrics, RuntimeTerminal, TermPalette, TERM_FONT_SIZE};
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::mouse::{click, Click};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{
    alignment, event, keyboard, mouse, Element, Event, Length, Point, Rectangle, Renderer, Size,
    Theme,
};
use iced::advanced::clipboard::Kind as ClipboardKind;
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
}

/// The grid cell (col, line) under a cursor position within `bounds`.
fn grid_at(pos: Point, bounds: Rectangle, metrics: CellMetrics) -> (u16, u16) {
    let col = ((pos.x - bounds.x) / metrics.width).floor().max(0.0) as u16;
    let line = ((pos.y - bounds.y) / metrics.height).floor().max(0.0) as u16;
    (col, line)
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
        Self { rt, palette, focused: false }
    }

    /// Mark the pane focused (draws the accent focus border).
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
        Size { width: Length::Fill, height: Length::Fill }
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let metrics = CellMetrics::new(TERM_FONT_SIZE);
        let default_bg = self.palette.background();

        let mut frame = Frame::new(renderer, viewport.size());
        {
            // Pane background.
            frame.fill_rectangle(bounds.position(), bounds.size(), default_bg);

            let content = self.rt.renderable();
            let cursor_point = content.cursor.point;
            let show_cursor = shows_cursor(content.mode);

            for indexed in content.display_iter {
                let line = indexed.point.line.0;
                if line < 0 {
                    continue;
                }
                let col = indexed.point.column.0 as f32;
                let x = bounds.x + col * metrics.width;
                let y = bounds.y + (line as f32) * metrics.height;

                let flags = indexed.cell.flags;
                let (fg, bg) = cell_colors(&self.palette, indexed.cell.fg, indexed.cell.bg, flags);

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
                    let glyph_fg =
                        if show_cursor && indexed.point == cursor_point { default_bg } else { fg };
                    frame.fill_text(Text {
                        content: ch.to_string(),
                        position: iced::Point::new(x + metrics.width / 2.0, y + metrics.height / 2.0),
                        color: glyph_fg,
                        size: iced::Pixels(metrics.size),
                        font: cell_font(flags),
                        horizontal_alignment: alignment::Horizontal::Center,
                        vertical_alignment: alignment::Vertical::Center,
                        line_height: iced::widget::text::LineHeight::Absolute(iced::Pixels(metrics.height)),
                        shaping: iced::widget::text::Shaping::Advanced,
                    });
                }

                // Underline / strikethrough.
                use alacritty_terminal::term::cell::Flags;
                if flags.contains(Flags::UNDERLINE) {
                    let uy = y + metrics.height - 1.0;
                    frame.stroke(
                        &Path::line(iced::Point::new(x, uy), iced::Point::new(x + metrics.width, uy)),
                        Stroke::default().with_width(1.0).with_color(fg),
                    );
                }
                if flags.contains(Flags::STRIKEOUT) {
                    let sy = y + metrics.height / 2.0;
                    frame.stroke(
                        &Path::line(iced::Point::new(x, sy), iced::Point::new(x + metrics.width, sy)),
                        Stroke::default().with_width(1.0).with_color(fg),
                    );
                }
            }

            // Focus indicator: an accent border when the terminal holds input focus (FR-010).
            if self.focused {
                frame.stroke(
                    &Path::rectangle(bounds.position(), bounds.size()),
                    Stroke::default().with_width(1.5).with_color(self.palette.accent()),
                );
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
            shell.publish(Message::TerminalResized { cols: grid.0, rows: grid.1 });
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
                if self.focused && self.rt.mouse_mode() && !shift {
                    if let Some(seq) = self.rt.mouse_report_bytes(
                        0, col, line, true, shift, state.modifiers.alt(), state.modifiers.control(),
                    ) {
                        shell.publish(Message::TerminalBytes(seq));
                    }
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
            // Middle-click pastes the clipboard into the focused process (FR-013).
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
                if self.focused && cursor.is_over(bounds) =>
            {
                if let Some(pasted) = clipboard.read(ClipboardKind::Standard) {
                    shell.publish(Message::TerminalBytes(pasted.into_bytes()));
                }
                return event::Status::Captured;
            }
            // Wheel scrolls the local scrollback, or forwards to a mouse-reporting program on
            // the alternate screen (FR-016 + wheel edge case).
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y.round() as i32,
                    mouse::ScrollDelta::Pixels { y, .. } => (y / metrics.height).round() as i32,
                };
                if lines != 0 {
                    if self.focused && self.rt.mouse_mode() {
                        let (col, line) =
                            cursor.position().map(|p| grid_at(p, bounds, metrics)).unwrap_or((0, 0));
                        let btn = if lines > 0 { 64 } else { 65 };
                        let m = state.modifiers;
                        for _ in 0..lines.abs() {
                            if let Some(seq) = self.rt.mouse_report_bytes(
                                btn, col, line, true, m.shift(), m.alt(), m.control(),
                            ) {
                                shell.publish(Message::TerminalBytes(seq));
                            }
                        }
                    } else {
                        shell.publish(Message::TerminalScrolled(lines));
                    }
                    return event::Status::Captured;
                }
            }
            _ => {}
        }

        // Keyboard input reaches the process ONLY while focused (FR-006/FR-008/FR-009).
        if !self.focused {
            return event::Status::Ignored;
        }
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, text, .. }) = event {
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
    keymap::Mods { shift: m.shift(), ctrl: m.control(), alt: m.alt(), logo: m.logo() }
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

    #[test]
    fn grid_at_maps_pixels_to_cells() {
        let metrics = CellMetrics::new(TERM_FONT_SIZE); // width 7.8, height 18.2
        let bounds = Rectangle { x: 10.0, y: 20.0, width: 800.0, height: 600.0 };
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
        let bounds = Rectangle { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
        assert_eq!(grid_at(Point::new(0.0, 0.0), bounds, metrics), (0, 0));
    }
}

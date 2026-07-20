//! The embedded terminal: the real `portable-pty`-backed [`TerminalBackend`] impl, VT/ANSI
//! interpretation via `alacritty_terminal`, per-session runtime state, colour palette, and the
//! terminal pane rendering (gui-only). Feature 005 (R1/R2/R4/R6) + feature 006 (real terminal).
//!
//! A session's `claude` process runs in a PTY; a reader thread streams its raw bytes into a
//! shared buffer, fed into an `alacritty_terminal::Term` (a VT emulator) so escape sequences are
//! interpreted into a character grid with per-cell colour/style. Feature 006 renders that grid
//! in colour via a custom canvas widget ([`crate::ui::material::terminal_pane`]) and streams
//! keystrokes back to the PTY.

use crate::ui::material::{ContextMenu, MenuItem, TerminalPane};
use crate::ui::style;
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{viewport_to_point, Config, RenderableContent, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Processor};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Color, Element, Font, Length};
use micold_ai_ide::app::SelectKind;
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::provider::{AiCliProvider, ClaudeProvider};
use micold_ai_ide::session::{SessionId, SessionLifecycle};
use micold_ai_ide::terminal::{claude_args, LaunchSpec, TerminalBackend, TerminalHandle};
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale, Rgb};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// The initial PTY/grid size, used until the pane reports its real size (feature 006 resize).
const INIT_ROWS: u16 = 30;
const INIT_COLS: u16 = 100;

/// The terminal font size (monospace). Cell metrics are derived from it.
pub const TERM_FONT_SIZE: f32 = 13.0;

/// Approximate monospace cell metrics for a given font size. Exact glyph measurement is a
/// future refinement; these ratios keep the grid aligned for typical monospace faces.
#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    pub size: f32,
    pub width: f32,
    pub height: f32,
}

impl CellMetrics {
    pub fn new(size: f32) -> Self {
        Self {
            size,
            width: size * 0.6,
            height: size * 1.4,
        }
    }

    /// The grid dimensions (cols, rows) that fit a pixel area, minimum 1×1 (feature 006, FR-014).
    pub fn grid_size(&self, width: f32, height: f32) -> (u16, u16) {
        let cols = (width / self.width).floor().max(1.0) as u16;
        let rows = (height / self.height).floor().max(1.0) as u16;
        (cols, rows)
    }
}

/// Fixed terminal dimensions for constructing an `alacritty_terminal` grid.
struct TermDimensions {
    rows: usize,
    cols: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// A live PTY session: writer + child, a shared raw-output buffer the reader thread appends to,
/// the VT emulator, and a canvas cache invalidated when new output arrives (feature 006).
pub struct RuntimeTerminal {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    output: Arc<Mutex<Vec<u8>>>,
    term: Term<VoidListener>,
    parser: Processor,
    rows: usize,
    cols: usize,
}

impl RuntimeTerminal {
    /// Feed newly-streamed bytes into the VT emulator (call on the UI tick). Invalidates the
    /// render cache only when bytes were actually applied.
    pub fn pump(&mut self) {
        let bytes = std::mem::take(&mut *self.output.lock().unwrap());
        if !bytes.is_empty() {
            self.parser.advance(&mut self.term, &bytes);
        }
    }

    /// Write raw bytes to the child (FR-014, FR-008).
    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Scroll the display by `delta` lines (+ up into scrollback) (FR-016).
    pub fn scroll(&mut self, delta: i32) {
        use alacritty_terminal::grid::Scroll;
        if delta != 0 {
            self.term.grid_mut().scroll_display(Scroll::Delta(delta));
        }
    }

    /// Lines currently scrolled up into the scrollback (0 = live bottom) (FR-016).
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Number of lines retained in the scrollback history (FR-016).
    pub fn history_size(&self) -> usize {
        self.term.grid().history_size()
    }

    /// Resize the PTY and the VT grid to `cols`×`rows` (FR-014, FR-015).
    pub fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        if cols == 0 || rows == 0 || (cols as usize == self.cols && rows as usize == self.rows) {
            return Ok(());
        }
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;
        self.term
            .resize(TermSize::new(cols as usize, rows as usize));
        self.cols = cols as usize;
        self.rows = rows as usize;
        Ok(())
    }

    /// Terminate the child process (FR-015a).
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Whether the child has exited (drives crash/exit handling, FR-022).
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// The current grid size in cells.
    pub fn size(&self) -> (u16, u16) {
        (self.cols as u16, self.rows as u16)
    }

    /// The interpreted screen content for rendering (borrows the grid).
    pub fn renderable(&self) -> RenderableContent<'_> {
        self.term.renderable_content()
    }

    /// The key-encoding-relevant terminal mode (cursor-key mode, alt-screen) (feature 006).
    pub fn key_term_mode(&self) -> micold_ai_ide::keymap::TermMode {
        let mode = self.term.renderable_content().mode;
        micold_ai_ide::keymap::TermMode {
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            alt_screen: mode.contains(TermMode::ALT_SCREEN),
        }
    }

    /// Whether the process has enabled mouse reporting (feature 006, FR-013a).
    pub fn mouse_mode(&self) -> bool {
        self.term
            .renderable_content()
            .mode
            .intersects(TermMode::MOUSE_MODE)
    }

    /// Whether the process wants pointer *motion* reported, not just button presses
    /// (feature 006, FR-013a). `MOUSE_DRAG` asks for motion while a button is held;
    /// `MOUSE_MOTION` asks for it unconditionally.
    pub fn mouse_motion_mode(&self) -> bool {
        self.term
            .renderable_content()
            .mode
            .intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
    }

    /// The grid point for a viewport cell, accounting for scrollback offset.
    fn viewport_point(&self, col: u16, line: u16) -> Point {
        let display_offset = self.term.grid().display_offset();
        let col = (col as usize).min(self.cols.saturating_sub(1));
        let line = (line as usize).min(self.rows.saturating_sub(1));
        viewport_to_point(display_offset, Point::new(line, Column(col)))
    }

    /// Begin a text selection at a viewport grid cell (FR-013).
    pub fn selection_start(&mut self, col: u16, line: u16, kind: SelectKind) {
        let ty = match kind {
            SelectKind::Simple => SelectionType::Simple,
            SelectKind::Semantic => SelectionType::Semantic,
            SelectKind::Lines => SelectionType::Lines,
        };
        let point = self.viewport_point(col, line);
        self.term.selection = Some(Selection::new(ty, point, Side::Left));
    }

    /// Extend the in-progress selection to a viewport grid cell (FR-013).
    pub fn selection_update(&mut self, col: u16, line: u16) {
        let point = self.viewport_point(col, line);
        if let Some(sel) = self.term.selection.as_mut() {
            sel.update(point, Side::Left);
        }
    }

    /// Clear the current text selection.
    pub fn selection_clear(&mut self) {
        self.term.selection = None;
    }

    /// Encode a mouse event as a terminal mouse-report sequence, or `None` when the process has
    /// not enabled mouse reporting (FR-013a). `button`: 0=left, 1=middle, 2=right, 64/65=wheel.
    pub fn mouse_report_bytes(
        &self,
        button: u8,
        col: u16,
        line: u16,
        pressed: bool,
        mods: micold_ai_ide::keymap::Mods,
    ) -> Option<Vec<u8>> {
        let mode = self.term.renderable_content().mode;
        encode_mouse_report(mode, button, col, line, pressed, mods)
    }

    /// The currently-selected text, or empty when there is no selection (feature 006 FR-013).
    pub fn selectable_content(&self) -> String {
        selection_text(&self.term)
    }
}

/// Extract the selected region as text (feature 006, FR-013).
///
/// Delegates to the VT emulator's own extraction rather than walking `renderable_content`:
/// `display_iter` covers only the visible viewport and yields bare characters, so a hand-rolled
/// walk drops every line break and silently truncates any part of the selection that has
/// scrolled into scrollback. `selection_to_string` reads the grid by absolute line, so it spans
/// scrollback, emits newlines at line ends, and handles tabs and wide characters.
fn selection_text(term: &Term<VoidListener>) -> String {
    term.selection_to_string().unwrap_or_default()
}

impl TerminalHandle for RuntimeTerminal {
    fn write_input(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.write(bytes)
    }
    fn resize(&mut self, rows: u16, cols: u16) -> std::io::Result<()> {
        RuntimeTerminal::resize(self, cols, rows)
    }
    fn kill(&mut self) -> std::io::Result<()> {
        RuntimeTerminal::kill(self)
    }
}

/// Spawn `claude` for `spec` in a PTY and start streaming its output. `scrollback_lines` sets the
/// VT grid's history depth (feature 006, FR-016/FR-020). `initial_size` seeds the PTY/grid at the
/// pane's last-known `(cols, rows)` so a session spawned into an already-sized pane (a new session
/// started after the window has been resized once, a resumed session, an auto-restart) fills it
/// immediately instead of waiting for the next window-resize event to reconcile — falls back to
/// the `INIT_COLS`×`INIT_ROWS` default only when no pane size has been reported yet (bugfix: new
/// terminal not starting fullscreen).
pub fn spawn_pty(
    spec: &LaunchSpec,
    scrollback_lines: usize,
    initial_size: Option<(u16, u16)>,
) -> std::io::Result<RuntimeTerminal> {
    let (cols, rows) = initial_size.unwrap_or((INIT_COLS, INIT_ROWS));
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(std::io::Error::other)?;

    let mut cmd = CommandBuilder::new(ClaudeProvider.command());
    cmd.cwd(&spec.cwd);
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    for arg in claude_args(spec) {
        cmd.arg(arg);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(std::io::Error::other)?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(std::io::Error::other)?;
    let writer = pair.master.take_writer().map_err(std::io::Error::other)?;

    let output = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&output);
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => sink.lock().unwrap().extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    });

    let dims = TermDimensions {
        rows: rows as usize,
        cols: cols as usize,
    };
    let config = Config {
        scrolling_history: scrollback_lines,
        ..Config::default()
    };
    let term = Term::new(config, &dims, VoidListener);
    let parser = Processor::new();

    Ok(RuntimeTerminal {
        master: pair.master,
        writer,
        child,
        output,
        term,
        parser,
        rows: rows as usize,
        cols: cols as usize,
    })
}

/// The production [`TerminalBackend`] (constitution seam). The binary uses [`spawn_pty`] directly
/// so it can also read the interpreted grid for rendering.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct PtyTerminalBackend;

impl TerminalBackend for PtyTerminalBackend {
    fn spawn(&self, spec: LaunchSpec) -> std::io::Result<Box<dyn TerminalHandle>> {
        Ok(Box::new(spawn_pty(&spec, 10_000, None)?))
    }
}

/// Maps `alacritty_terminal` ANSI colours to iced colours (feature 006, FR-001/FR-003). Default
/// foreground/background follow the app's active theme; the 16 ANSI colours use a fixed
/// conventional palette (programs assume standard colours — SC-002).
#[derive(Debug, Clone)]
pub struct TermPalette {
    fg: Color,
    bg: Color,
    accent: Color,
    ansi16: [Color; 16],
}

impl TermPalette {
    /// Build a palette whose default fg/bg follow the active light/dark scheme (FR-003), with
    /// the theme's primary as the focus-indicator accent.
    pub fn from_scheme(scheme: ColorScheme) -> Self {
        let r = tokens::roles(scheme);
        Self {
            fg: rgb_to_color(r.on_surface),
            bg: rgb_to_color(r.surface),
            accent: rgb_to_color(r.primary),
            ansi16: STANDARD_ANSI16,
        }
    }

    /// The default background (the pane's fill).
    pub fn background(&self) -> Color {
        self.bg
    }

    /// The default foreground.
    pub fn foreground(&self) -> Color {
        self.fg
    }

    /// The focus-indicator accent color.
    pub fn accent(&self) -> Color {
        self.accent
    }

    /// Resolve an ANSI colour to an iced colour (16 / bright / 256 / truecolor).
    pub fn color(&self, c: AnsiColor) -> Color {
        match c {
            AnsiColor::Spec(rgb) => Color::from_rgb8(rgb.r, rgb.g, rgb.b),
            AnsiColor::Named(named) => match named {
                NamedColor::Foreground => self.fg,
                NamedColor::Background => self.bg,
                NamedColor::Black => self.ansi16[0],
                NamedColor::Red => self.ansi16[1],
                NamedColor::Green => self.ansi16[2],
                NamedColor::Yellow => self.ansi16[3],
                NamedColor::Blue => self.ansi16[4],
                NamedColor::Magenta => self.ansi16[5],
                NamedColor::Cyan => self.ansi16[6],
                NamedColor::White => self.ansi16[7],
                NamedColor::BrightBlack => self.ansi16[8],
                NamedColor::BrightRed => self.ansi16[9],
                NamedColor::BrightGreen => self.ansi16[10],
                NamedColor::BrightYellow => self.ansi16[11],
                NamedColor::BrightBlue => self.ansi16[12],
                NamedColor::BrightMagenta => self.ansi16[13],
                NamedColor::BrightCyan => self.ansi16[14],
                NamedColor::BrightWhite => self.ansi16[15],
                _ => self.fg,
            },
            AnsiColor::Indexed(i) => {
                if (i as usize) < 16 {
                    self.ansi16[i as usize]
                } else {
                    indexed_256(i)
                }
            }
        }
    }
}

fn rgb_to_color(c: Rgb) -> Color {
    Color::from_rgb8(c.r, c.g, c.b)
}

/// A conventional 16-colour ANSI palette (standard + bright), theme-independent (SC-002).
const STANDARD_ANSI16: [Color; 16] = [
    Color::from_rgb(0.0, 0.0, 0.0),       // black
    Color::from_rgb(0.674, 0.259, 0.259), // red
    Color::from_rgb(0.564, 0.663, 0.349), // green
    Color::from_rgb(0.956, 0.749, 0.459), // yellow
    Color::from_rgb(0.416, 0.623, 0.71),  // blue
    Color::from_rgb(0.666, 0.458, 0.623), // magenta
    Color::from_rgb(0.458, 0.71, 0.666),  // cyan
    Color::from_rgb(0.847, 0.847, 0.847), // white
    Color::from_rgb(0.419, 0.419, 0.419), // bright black
    Color::from_rgb(0.772, 0.333, 0.333), // bright red
    Color::from_rgb(0.667, 0.769, 0.454), // bright green
    Color::from_rgb(0.996, 0.792, 0.533), // bright yellow
    Color::from_rgb(0.509, 0.721, 0.784), // bright blue
    Color::from_rgb(0.76, 0.549, 0.721),  // bright magenta
    Color::from_rgb(0.576, 0.827, 0.764), // bright cyan
    Color::from_rgb(0.972, 0.972, 0.972), // bright white
];

/// The xterm 256-colour cube + grayscale ramp for indices ≥ 16.
fn indexed_256(i: u8) -> Color {
    if i < 16 {
        return STANDARD_ANSI16[i as usize];
    }
    if i < 232 {
        let i = i - 16;
        let r = i / 36;
        let g = (i % 36) / 6;
        let b = i % 6;
        let comp = |v: u8| -> f32 {
            if v == 0 {
                0.0
            } else {
                (v as f32 * 40.0 + 55.0) / 255.0
            }
        };
        Color::from_rgb(comp(r), comp(g), comp(b))
    } else {
        let level = (i - 232) as f32 * 10.0 + 8.0;
        let v = level / 255.0;
        Color::from_rgb(v, v, v)
    }
}

/// Resolve per-cell fg/bg/flags into final draw colours + font (feature 006 rendering helper).
/// `selected` marks a cell inside the active text selection, drawn with fg/bg swapped — the
/// visible highlight (contracts/terminal-render-input.md, FR-013).
pub fn cell_colors(
    palette: &TermPalette,
    fg: AnsiColor,
    bg: AnsiColor,
    flags: Flags,
    selected: bool,
) -> (Color, Color) {
    let mut f = palette.color(fg);
    let mut b = palette.color(bg);
    if flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
        f.a *= 0.7;
    }
    // Both INVERSE and selection swap fg/bg; when both apply they cancel (double swap), so a
    // selected reverse-video cell renders like plain selected text, as in a standalone terminal.
    if flags.contains(Flags::INVERSE) != selected {
        std::mem::swap(&mut f, &mut b);
    }
    if flags.contains(Flags::HIDDEN) {
        f = b;
    }
    (f, b)
}

/// The bold/italic font for a cell's flags.
pub fn cell_font(flags: Flags) -> Font {
    let mut font = Font::MONOSPACE;
    if flags.intersects(Flags::BOLD | Flags::DIM_BOLD | Flags::BOLD_ITALIC) {
        font.weight = iced::font::Weight::Bold;
    }
    if flags.intersects(Flags::ITALIC | Flags::BOLD_ITALIC) {
        font.style = iced::font::Style::Italic;
    }
    font
}

/// Whether the terminal is currently showing its cursor.
pub fn shows_cursor(mode: TermMode) -> bool {
    mode.contains(TermMode::SHOW_CURSOR)
}

/// Encode a mouse event into a terminal mouse-report sequence for `mode`, or `None` when mouse
/// reporting is off (feature 006, FR-013a). Pure over `TermMode` so it is unit-testable.
/// `button`: 0=left, 1=middle, 2=right, 64/65=wheel up/down.
pub fn encode_mouse_report(
    mode: TermMode,
    button: u8,
    col: u16,
    line: u16,
    pressed: bool,
    mods: micold_ai_ide::keymap::Mods,
) -> Option<Vec<u8>> {
    if !mode.intersects(TermMode::MOUSE_MODE) {
        return None;
    }
    let mut mod_bits = 0u8;
    if mods.shift {
        mod_bits += 4;
    }
    if mods.alt {
        mod_bits += 8;
    }
    if mods.ctrl {
        mod_bits += 16;
    }
    if mode.contains(TermMode::SGR_MOUSE) {
        let c = if pressed { 'M' } else { 'm' };
        Some(format!("\x1b[<{};{};{}{}", button + mod_bits, col + 1, line + 1, c).into_bytes())
    } else {
        let b = if pressed {
            button + mod_bits
        } else {
            3 + mod_bits
        };
        let cx = 32 + 1 + (col.min(222) as u8);
        let cy = 32 + 1 + (line.min(222) as u8);
        Some(vec![0x1b, b'[', b'M', 32 + b, cx, cy])
    }
}

/// Render the terminal pane for the active session (FR-012). `terminal` is the active session's
/// live runtime (colour-rendered); `None` renders an empty state.
pub fn pane<'a>(
    state: &'a State,
    terminal: Option<&'a RuntimeTerminal>,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let Some(active) = state.active_session else {
        return container(
            text("Select or start a session to open its terminal.")
                .size(type_scale::BODY)
                .style(style::muted(r)),
        )
        .padding(spacing::LG)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    };

    // The colour-rendering terminal body fills the whole main area (feature 006). Falls back to
    // an empty state if the runtime is not yet available (e.g. the session is still starting).
    let body: Element<'a, Message> = match terminal {
        Some(rt) => TerminalPane::new(rt, TermPalette::from_scheme(scheme))
            .focused(state.terminal_focused)
            .into(),
        None => container(
            text("Starting…")
                .size(type_scale::LABEL)
                .style(style::muted(r)),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
    };

    // While the right-click context menu is open, float it over the terminal body anchored at the
    // clicked point; choosing Copy/Paste or clicking outside dismisses it (FR-013).
    let body: Element<'a, Message> = match state.terminal_context_menu {
        Some((x, y)) => ContextMenu::new(
            body,
            vec![
                MenuItem {
                    icon: None,
                    label: "Copy".to_string(),
                    message: Message::TerminalCopyRequested,
                },
                MenuItem {
                    icon: None,
                    label: "Paste".to_string(),
                    message: Message::TerminalPasteRequested,
                },
            ],
            (x, y),
            Message::TerminalContextMenuClosed,
            r,
        )
        .into(),
        None => body,
    };

    // A slim bottom status bar: the current session name (left) and its lifecycle status
    // (right). A live activity indicator (spinner/idle icon) is a planned follow-up feature.
    let status = session_status(state, active);
    let mut bar = row![
        text(session_title(state, active)).size(type_scale::LABEL),
        Space::with_width(Length::Fill),
        text(status).size(type_scale::LABEL).style(style::muted(r)),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    // While the terminal holds focus, offer an explicit way out (FR-011) alongside the reserved
    // Ctrl+Shift+E chord and click-outside.
    if state.terminal_focused {
        bar = bar.push(
            button(
                text("⎋ release focus (Ctrl+Shift+E)")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
            )
            .padding(spacing::XS)
            .style(style::text_button(r))
            .on_press(Message::TerminalFocusReleased),
        );
    }
    let bottom_bar = container(bar)
        .width(Length::Fill)
        .padding(spacing::SM)
        .style(style::toolbar_surface(r));

    // Feature 006 US2: keystrokes stream live to the PTY through the focused `TerminalPane`;
    // the old line-input box is gone (FR-008). Click the terminal to focus it, then type.
    column![
        container(body).height(Length::Fill).width(Length::Fill),
        bottom_bar,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn session_title(state: &State, id: SessionId) -> String {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.label.display().to_string())
        .unwrap_or_else(|| "Session".to_string())
}

fn session_status(state: &State, id: SessionId) -> &'static str {
    match state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.lifecycle)
    {
        Some(SessionLifecycle::Running) => "running",
        Some(SessionLifecycle::Starting) => "starting…",
        Some(SessionLifecycle::Restarting { .. }) => "restarting…",
        Some(SessionLifecycle::Failed) => "failed",
        Some(SessionLifecycle::Idle) => "idle",
        None => "",
    }
}

#[cfg(test)]
mod tests {
    //! Colour-mapping tests for `TermPalette` (feature 006, FR-001/FR-003). Bin unit tests —
    //! run with `cargo test --features gui`. See contracts/terminal-render-input.md.
    use super::*;
    use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb as AnsiRgb};

    #[test]
    fn spec_maps_to_truecolor() {
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        let c = p.color(AnsiColor::Spec(AnsiRgb {
            r: 10,
            g: 20,
            b: 30,
        }));
        assert_eq!(c, Color::from_rgb8(10, 20, 30));
    }

    #[test]
    fn indexed_0_to_15_use_the_ansi16_palette() {
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        for i in 0u8..16 {
            assert_eq!(p.color(AnsiColor::Indexed(i)), STANDARD_ANSI16[i as usize]);
        }
    }

    #[test]
    fn named_black_and_bright_white_match_ansi16() {
        let p = TermPalette::from_scheme(ColorScheme::Light);
        assert_eq!(
            p.color(AnsiColor::Named(NamedColor::Black)),
            STANDARD_ANSI16[0]
        );
        assert_eq!(
            p.color(AnsiColor::Named(NamedColor::BrightWhite)),
            STANDARD_ANSI16[15]
        );
    }

    #[test]
    fn indexed_256_cube_starts_at_black() {
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        // Index 16 is the first colour-cube entry (0,0,0).
        assert_eq!(
            p.color(AnsiColor::Indexed(16)),
            Color::from_rgb(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn default_fg_bg_follow_the_theme() {
        let light = TermPalette::from_scheme(ColorScheme::Light);
        let dark = TermPalette::from_scheme(ColorScheme::Dark);
        // Default background differs between light and dark schemes (FR-003).
        assert_ne!(light.background(), dark.background());
        // Named Foreground/Background resolve to the theme defaults, not the ANSI palette.
        assert_eq!(
            dark.color(AnsiColor::Named(NamedColor::Foreground)),
            dark.foreground()
        );
        assert_eq!(
            dark.color(AnsiColor::Named(NamedColor::Background)),
            dark.background()
        );
    }

    #[test]
    fn inverse_flag_swaps_fg_and_bg() {
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        let fg = AnsiColor::Named(NamedColor::Foreground);
        let bg = AnsiColor::Named(NamedColor::Background);
        let (f, b) = cell_colors(&p, fg, bg, Flags::INVERSE, false);
        assert_eq!(f, p.background());
        assert_eq!(b, p.foreground());
    }

    #[test]
    fn selected_cells_swap_fg_and_bg() {
        // A cell within the selection range renders with fg/bg swapped — the visible highlight
        // (contracts/terminal-render-input.md: "within selection range → swap fg/bg").
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        let fg = AnsiColor::Named(NamedColor::Foreground);
        let bg = AnsiColor::Named(NamedColor::Background);
        let (f, b) = cell_colors(&p, fg, bg, Flags::empty(), true);
        assert_eq!(f, p.background());
        assert_eq!(b, p.foreground());
    }

    #[test]
    fn selection_over_inverse_cell_cancels_the_swap() {
        // Selection and INVERSE both swap fg/bg; together they cancel, so a selected reverse-video
        // cell renders like a plain unselected one — matching a standalone terminal.
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        let fg = AnsiColor::Named(NamedColor::Foreground);
        let bg = AnsiColor::Named(NamedColor::Background);
        let (f, b) = cell_colors(&p, fg, bg, Flags::INVERSE, true);
        assert_eq!(f, p.foreground());
        assert_eq!(b, p.background());
    }

    /// End-to-end against a real `alacritty_terminal` grid (no PTY, no window): make a selection
    /// exactly as the runtime does, then walk the grid the way `draw` does and confirm the
    /// selected cells come back with fg/bg swapped while the rest are untouched. Reproduces the
    /// reported bug where a drag selected text but nothing was ever highlighted (FR-013).
    #[test]
    fn selected_grid_cells_render_highlighted_like_draw() {
        use alacritty_terminal::index::{Column, Line, Point as GridPoint, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};

        let rows = 3usize;
        let cols = 10usize;
        let dims = TermDimensions { rows, cols };
        let mut term = Term::new(Config::default(), &dims, VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, b"hello");

        // Select the first three columns of the top line, as selection_start/update would.
        let mut sel = Selection::new(
            SelectionType::Simple,
            GridPoint::new(Line(0), Column(0)),
            Side::Left,
        );
        sel.update(GridPoint::new(Line(0), Column(2)), Side::Left);
        term.selection = Some(sel);

        let palette = TermPalette::from_scheme(ColorScheme::Dark);
        let content = term.renderable_content();
        let selection = content
            .selection
            .expect("a non-empty selection must yield a range");

        let (mut saw_selected, mut saw_unselected) = (false, false);
        for indexed in content.display_iter {
            let selected = selection.contains(indexed.point);
            let styled = cell_colors(
                &palette,
                indexed.cell.fg,
                indexed.cell.bg,
                indexed.cell.flags,
                selected,
            );
            let plain = cell_colors(
                &palette,
                indexed.cell.fg,
                indexed.cell.bg,
                indexed.cell.flags,
                false,
            );
            if selected {
                saw_selected = true;
                assert_eq!(styled, (plain.1, plain.0), "selected cell must swap fg/bg");
            } else {
                saw_unselected = true;
                assert_eq!(styled, plain, "unselected cell must be unchanged");
            }
        }
        assert!(saw_selected, "expected some cells inside the selection");
        assert!(saw_unselected, "expected some cells outside the selection");
    }

    /// Build a `Term` with `history` lines of scrollback and feed it `input`.
    fn term_with(rows: usize, cols: usize, history: usize, input: &[u8]) -> Term<VoidListener> {
        let dims = TermDimensions { rows, cols };
        let config = Config {
            scrolling_history: history,
            ..Config::default()
        };
        let mut term = Term::new(config, &dims, VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, input);
        term
    }

    /// Drive a selection the way `selection_start`/`selection_update` do: `Side::Left` on both
    /// ends, so `to` is the cell *after* the last selected character.
    fn select(term: &mut Term<VoidListener>, from: (i32, usize), to: (i32, usize)) {
        use alacritty_terminal::index::{Column, Line, Point as GridPoint, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};
        let mut sel = Selection::new(
            SelectionType::Simple,
            GridPoint::new(Line(from.0), Column(from.1)),
            Side::Left,
        );
        sel.update(GridPoint::new(Line(to.0), Column(to.1)), Side::Left);
        term.selection = Some(sel);
    }

    #[test]
    fn fr_013_no_selection_copies_nothing() {
        let term = term_with(3, 10, 0, b"hello");
        assert_eq!(selection_text(&term), "");
    }

    /// A selection spanning several rows must copy as several lines. The previous implementation
    /// walked `display_iter` pushing bare characters, producing one run-on string.
    #[test]
    fn fr_013_multiline_selection_preserves_line_breaks() {
        let mut term = term_with(3, 10, 0, b"one\r\ntwo\r\nthree");
        select(&mut term, (0, 0), (2, 5));
        assert_eq!(selection_text(&term), "one\ntwo\nthree");
    }

    /// A selection dragged up into scrollback must copy the off-screen part too. `display_iter`
    /// covers only the viewport, so the previous implementation silently truncated it.
    #[test]
    fn fr_013_selection_spanning_scrollback_is_not_truncated() {
        // Five lines through a 2-row grid: "one".."three" scroll into history.
        let mut term = term_with(2, 10, 100, b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
        // Negative lines address scrollback; line 0 is the top of the viewport.
        select(&mut term, (-3, 0), (1, 4));
        assert_eq!(selection_text(&term), "one\ntwo\nthree\nfour\nfive");
    }

    #[test]
    fn no_mouse_report_when_mouse_mode_off() {
        let mods = micold_ai_ide::keymap::Mods::NONE;
        assert_eq!(
            encode_mouse_report(TermMode::empty(), 0, 3, 4, true, mods),
            None
        );
    }

    #[test]
    fn sgr_mouse_report_is_one_based_with_press_marker() {
        let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        let mods = micold_ai_ide::keymap::Mods::NONE;
        // Left button (0) press at grid (col 3, line 4) → CSI < 0 ; 4 ; 5 M.
        let seq = encode_mouse_report(mode, 0, 3, 4, true, mods).unwrap();
        assert_eq!(seq, b"\x1b[<0;4;5M");
        // Release uses the lowercase 'm' terminator.
        let seq = encode_mouse_report(mode, 0, 3, 4, false, mods).unwrap();
        assert_eq!(seq, b"\x1b[<0;4;5m");
    }

    #[test]
    fn sgr_mouse_report_adds_modifier_bits() {
        let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        // Shift adds 4 to the button code.
        let mods = micold_ai_ide::keymap::Mods {
            shift: true,
            ..micold_ai_ide::keymap::Mods::NONE
        };
        let seq = encode_mouse_report(mode, 0, 0, 0, true, mods).unwrap();
        assert_eq!(seq, b"\x1b[<4;1;1M");
    }

    /// FR-013a: the legacy (non-SGR) encoding reports every release as button 3, so a release
    /// must actually be sent — the process cannot infer which button came up.
    #[test]
    fn fr_013a_legacy_release_uses_the_button_release_code() {
        let mode = TermMode::MOUSE_REPORT_CLICK;
        let mods = micold_ai_ide::keymap::Mods::NONE;
        let press = encode_mouse_report(mode, 0, 3, 4, true, mods).unwrap();
        let release = encode_mouse_report(mode, 0, 3, 4, false, mods).unwrap();
        assert_eq!(press, vec![0x1b, b'[', b'M', 32, 36, 37]);
        assert_eq!(release, vec![0x1b, b'[', b'M', 35, 36, 37]);
    }

    /// FR-013a: middle (1) and right (2) are distinct button codes. Both were previously
    /// consumed locally — by middle-click paste and the context menu — and never reached a
    /// mouse-tracking process.
    #[test]
    fn fr_013a_middle_and_right_buttons_encode_distinctly() {
        let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        let mods = micold_ai_ide::keymap::Mods::NONE;
        assert_eq!(
            encode_mouse_report(mode, 1, 0, 0, true, mods).unwrap(),
            b"\x1b[<1;1;1M"
        );
        assert_eq!(
            encode_mouse_report(mode, 2, 0, 0, true, mods).unwrap(),
            b"\x1b[<2;1;1M"
        );
    }

    /// FR-013a: motion is the held button's code plus 32. Nothing emitted motion reports
    /// before, so a drag inside a mouse-tracking program did nothing.
    #[test]
    fn fr_013a_motion_report_adds_the_motion_bit() {
        let mode = TermMode::MOUSE_DRAG | TermMode::SGR_MOUSE;
        let mods = micold_ai_ide::keymap::Mods::NONE;
        // Left button (0) held, moving to col 5 line 6 → button code 0 + 32.
        const MOTION_BIT: u8 = 32;
        assert_eq!(
            encode_mouse_report(mode, MOTION_BIT, 5, 6, true, mods).unwrap(),
            b"\x1b[<32;6;7M"
        );
    }

    /// The widget asks for motion only when the process requested it; plain click-reporting
    /// must not produce a motion stream.
    #[test]
    fn fr_013a_motion_mode_is_distinct_from_click_reporting() {
        assert!(
            !TermMode::MOUSE_REPORT_CLICK.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
        );
        assert!(TermMode::MOUSE_DRAG.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION));
        assert!(TermMode::MOUSE_MOTION.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION));
    }

    #[test]
    fn grid_size_fits_cells_and_floors() {
        let m = CellMetrics::new(TERM_FONT_SIZE); // width 7.8, height 18.2
                                                  // 800x600 → floor(800/7.8)=102 cols, floor(600/18.2)=32 rows.
        assert_eq!(m.grid_size(800.0, 600.0), (102, 32));
    }

    #[test]
    fn grid_size_is_at_least_one_by_one() {
        let m = CellMetrics::new(TERM_FONT_SIZE);
        assert_eq!(m.grid_size(0.0, 0.0), (1, 1));
        assert_eq!(m.grid_size(3.0, 3.0), (1, 1));
    }

    /// End-to-end scroll behaviour against a real `alacritty_terminal` grid (no PTY, no window):
    /// feed more lines than fit, then scroll up and confirm the viewport slides down to reveal
    /// earlier lines while every row stays populated. Reproduces the reported bug where the top
    /// text stayed frozen and the bottom rows blanked (feature 006, FR-016).
    #[test]
    fn scrolling_up_reveals_earlier_lines_and_keeps_the_viewport_full() {
        use crate::ui::material::viewport_row;
        use alacritty_terminal::grid::Scroll;

        let rows = 5usize;
        let cols = 20usize;
        let dims = TermDimensions { rows, cols };
        let config = Config {
            scrolling_history: 100,
            ..Config::default()
        };
        let mut term = Term::new(config, &dims, VoidListener);
        let mut parser: Processor = Processor::new();

        // Print 10 lines (L0..L9), newline-separated with no trailing newline, so the bottom row
        // holds real text (L9) and the earliest lines spill into the scrollback history.
        let total = rows + 5;
        let mut bytes = Vec::new();
        for i in 0..total {
            if i > 0 {
                bytes.extend_from_slice(b"\r\n");
            }
            bytes.extend_from_slice(format!("L{i}").as_bytes());
        }
        parser.advance(&mut term, &bytes);

        // Snapshot the visible rows, mapping buffer lines to viewport rows exactly as `draw` does.
        let snapshot = |term: &Term<VoidListener>| -> Vec<String> {
            let content = term.renderable_content();
            let offset = content.display_offset;
            let mut grid = vec![String::new(); rows];
            for indexed in content.display_iter {
                if let Some(r) = viewport_row(indexed.point.line.0, offset, rows) {
                    grid[r].push(indexed.cell.c);
                }
            }
            grid.into_iter().map(|s| s.trim_end().to_string()).collect()
        };

        let before = snapshot(&term);
        assert_eq!(before, vec!["L5", "L6", "L7", "L8", "L9"]);

        // Scroll two lines up into the scrollback.
        term.grid_mut().scroll_display(Scroll::Delta(2));
        let after = snapshot(&term);

        // Correct behaviour: the whole viewport shifts down by two, revealing L3/L4 at the top,
        // and no row is left blank. The bug produced ["L5","L6","L7","",""] instead.
        assert!(
            after.iter().all(|r| !r.is_empty()),
            "scrolling blanked viewport rows (reported bug): {after:?}"
        );
        assert_eq!(after, vec!["L3", "L4", "L5", "L6", "L7"]);
        assert_eq!(
            after[2], before[0],
            "row 0 should slide down to row 2 after scrolling up two lines"
        );
    }

    /// The scrollbar thumb tracks the real scrollback position: hidden at the live bottom, pinned
    /// to the top when fully scrolled up (feature 006, FR-016). Headless — no PTY, no window.
    #[test]
    fn scrollbar_thumb_tracks_scrollback_position() {
        use crate::ui::material::scrollbar_metrics;
        use alacritty_terminal::grid::Scroll;

        let rows = 5usize;
        let cols = 20usize;
        let dims = TermDimensions { rows, cols };
        let config = Config {
            scrolling_history: 100,
            ..Config::default()
        };
        let mut term = Term::new(config, &dims, VoidListener);
        let mut parser: Processor = Processor::new();

        // 25 lines → well more than the 5 visible rows, so a real history builds up.
        let mut bytes = Vec::new();
        for i in 0..25 {
            if i > 0 {
                bytes.extend_from_slice(b"\r\n");
            }
            bytes.extend_from_slice(format!("L{i}").as_bytes());
        }
        parser.advance(&mut term, &bytes);

        let history = term.grid().history_size();
        assert!(history > 0, "expected scrollback history");

        // At the live bottom (offset 0): the scrollbar is hidden.
        assert!(scrollbar_metrics(100.0, rows, history, term.grid().display_offset()).is_none());

        // Scroll all the way up: the thumb pins to the very top of the track.
        term.grid_mut()
            .scroll_display(Scroll::Delta(history as i32));
        let sb = scrollbar_metrics(100.0, rows, history, term.grid().display_offset())
            .expect("scrollbar visible while scrolled back");
        assert_eq!(sb.thumb_top, 0.0);
    }
}

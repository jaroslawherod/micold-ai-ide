//! Terminal rendering support for the thin client: the colour palette + per-cell resolution
//! helpers the grid-cache pane renders with, and the session terminal `pane()` view.
//!
//! Sessions themselves live in `micold-daemon` — the client owns no PTY. It receives the
//! daemon's streamed grid frames into a [`crate::grid::GridCache`] and renders them via the custom
//! canvas widget ([`crate::ui::material::terminal_pane`]), resolving each cell's colour/flags with
//! the helpers here (`TermPalette`, [`cell_colors`], [`wire_cell_colors`], [`cell_font`]). It keeps
//! `alacritty_terminal` only for those VT-mode/flag/colour *types* (feature 006 rendering).

use crate::app::{Message, State};
use crate::grid::GridCache;
use crate::icons::Icon;
use crate::icons::{mode_glyph, mode_tooltip};
use crate::tokens::{self, spacing, type_scale, Rgb};
use crate::ui::material::{
    ContextMenu, IconButton, MenuItem, TerminalPane, Tooltip, TooltipPosition,
};
use crate::ui::style;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Color, Element, Font, Length};
use micold_core::protocol::grid::{WireColor, WireStyle};
use micold_core::session::{SessionId, SessionLifecycle, ShellLifecycle, TerminalMode};
use micold_core::theme::ColorScheme;

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

impl TermPalette {
    /// Resolve a wire colour (from a daemon [`WireStyle`]) to an iced colour — the inverse of the
    /// daemon framer's `wire_color`. `WireColor::Named(n)` carries the alacritty `NamedColor`
    /// discriminant verbatim; both processes link the same `alacritty_terminal`, so decoding against
    /// that enum's real discriminants (0..=15 → the ANSI-16 palette, `Foreground` (256) → the theme
    /// fg, `Background` (257) → the theme bg, every other special → the default fg, matching
    /// [`TermPalette::color`]'s `_ =>` arm) reproduces the local render exactly. Comparing against
    /// `NamedColor::Foreground as u16` rather than hard-coded numbers keeps this from drifting the
    /// way the old `16`/`17` guesses did (the specials are 256/257, not 16/17).
    pub fn wire_color(&self, c: WireColor) -> Color {
        match c {
            WireColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
            WireColor::Indexed(i) => {
                if (i as usize) < 16 {
                    self.ansi16[i as usize]
                } else {
                    indexed_256(i)
                }
            }
            WireColor::Named(n) if n < 16 => self.ansi16[n as usize],
            WireColor::Named(n) if n == NamedColor::Foreground as u16 => self.fg,
            WireColor::Named(n) if n == NamedColor::Background as u16 => self.bg,
            WireColor::Named(_) => self.fg,
        }
    }
}

/// Resolve a daemon-streamed cell's [`WireStyle`] into final draw colours — the wire-grid twin of
/// [`cell_colors`], sharing the exact DIM/INVERSE/HIDDEN + selection-swap rules so a daemon-rendered
/// pane looks identical to the local one (FR-013). `selected` marks a cell in the active selection.
pub fn wire_cell_colors(
    palette: &TermPalette,
    style: &WireStyle,
    selected: bool,
) -> (Color, Color) {
    let flags = Flags::from_bits_truncate(style.flags);
    let mut f = palette.wire_color(style.fg);
    let mut b = palette.wire_color(style.bg);
    if flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
        f.a *= 0.7;
    }
    // Both INVERSE and selection swap fg/bg; when both apply they cancel (double swap).
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
    mods: crate::keymap::Mods,
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

/// Render the terminal pane for the active session (FR-012). `grid` is the active session's
/// daemon-streamed grid cache (colour-rendered); `None` renders an empty state. `selection` is the
/// active `LineId`-anchored text selection, and `display_offset` how far the view is scrolled back.
pub fn pane<'a>(
    state: &'a State,
    grid: Option<&'a GridCache>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
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
    let body: Element<'a, Message> = match grid {
        Some(grid) => TerminalPane::new(grid, TermPalette::from_scheme(scheme))
            .selection(selection)
            .display_offset(display_offset)
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

    // A slim bottom status bar: the current session name (left) and its attached-process status
    // (right), with the mode toggle anchored in the bottom-right corner as the bar's last
    // element. A live activity indicator (spinner/idle icon) is a planned follow-up feature.
    let mode = session_mode(state, active);
    let status = session_status(state, active);
    let mut bar = row![
        text(session_title(state, active)).size(type_scale::LABEL),
        Space::with_width(Length::Fill),
        text(status).size(type_scale::LABEL).style(style::muted(r)),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    // The attached process (per the current mode) isn't running — offer a manual restart
    // (FR-013; contracts/terminal-mode-lifecycle.md). Absent whenever it's already
    // running/starting, since there is nothing to restart.
    if attached_process_restartable(state, active) {
        bar = bar.push(
            button(
                text("restart")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
            )
            .padding(spacing::SM)
            .style(style::text_button(r))
            .on_press(Message::TerminalRestartRequested),
        );
    }
    // While the terminal holds focus, offer an explicit way out (FR-011) alongside the reserved
    // Ctrl+Shift+E chord and click-outside. Icon-only (with a tooltip carrying the label and
    // chord) rather than an icon+text button — keeps the bar compact; `Icon::ReleaseFocus` reads
    // clearly on its own and the tooltip still surfaces the reserved chord on hover.
    if state.terminal_focused {
        bar = bar.push(
            Tooltip::new(
                IconButton::new(Icon::ReleaseFocus, r)
                    .padding(spacing::SM)
                    .on_press(Message::TerminalFocusReleased),
                "Release focus (Ctrl+Shift+E)",
                r,
            )
            // This control sits mid-bar, not at an edge — opening below would run past the
            // window's bottom edge since the bar is the last row on screen, so open upward.
            .position(TooltipPosition::Top),
        );
    }
    // The instance-switching control: one entry per open Regular Terminal instance, visible only
    // once a session has more than one (feature 011, FR-004/FR-005). Placed just before the
    // "open a new instance" control, both ahead of the primary mode toggle.
    if let Some(switcher) = instance_switcher_row(state, active, r) {
        bar = bar.push(switcher);
    }
    // Open an additional Regular Terminal instance (feature 011, FR-001/FR-005) — visible
    // whenever the session is in Regular mode, regardless of how many instances are already
    // open (including zero or one), so there is always a way to go from one instance to two.
    if mode == TerminalMode::Regular {
        bar = bar.push(
            Tooltip::new(
                IconButton::new(Icon::AddTerminalInstance, r)
                    .padding(spacing::SM)
                    .on_press(Message::ShellInstanceOpenRequested),
                "Open a new terminal instance (Ctrl+Shift+T)",
                r,
            )
            // This button sits at (or very near) the bar's right edge — open the tooltip to the
            // left so it opens inward instead of overflowing past the window edge.
            .position(TooltipPosition::Left),
        );
    }
    // The mode toggle anchors the bar's bottom-right corner (spec Clarifications, 2026-07-18) —
    // pushed last so it always sits at the far right regardless of which other controls are
    // present.
    bar = bar.push(
        Tooltip::new(
            IconButton::new(mode_glyph(mode), r)
                .padding(spacing::SM)
                .on_press(Message::TerminalModeToggled),
            mode_tooltip(mode),
            r,
        )
        // Always the rightmost element in the bar — same reasoning as the "+" button above.
        .position(TooltipPosition::Left),
    );
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

/// The session's currently-attached mode (feature 010) — defaults to `AiCli` if the session
/// can't be found (shouldn't happen for an `active` id, but keeps this total).
fn session_mode(state: &State, id: SessionId) -> TerminalMode {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.mode)
        .unwrap_or_default()
}

/// Status text for the process currently attached to the pane (feature 010) — the AI CLI's
/// `SessionLifecycle` in `AiCli` mode, the shell's `ShellLifecycle` in `Regular` mode
/// (contracts/terminal-mode-lifecycle.md: `TerminalMode` never determines *running*, only
/// *displayed* — so the status shown must follow the same split).
fn session_status(state: &State, id: SessionId) -> &'static str {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return "";
    };
    match session.mode {
        TerminalMode::AiCli => match session.lifecycle {
            SessionLifecycle::Running => "running",
            SessionLifecycle::Starting => "starting…",
            SessionLifecycle::Restarting { .. } => "restarting…",
            SessionLifecycle::Failed => "failed",
            SessionLifecycle::Idle => "idle",
            SessionLifecycle::InterruptedResumable => "interrupted",
        },
        TerminalMode::Regular => match session.active_shell_lifecycle() {
            Some(ShellLifecycle::Running) => "running",
            Some(ShellLifecycle::Starting) => "starting…",
            Some(ShellLifecycle::Exited) => "exited",
            Some(ShellLifecycle::NotStarted) | None => "idle",
        },
    }
}

/// Whether the bottom-bar restart control should show (FR-013): the currently-attached process
/// (per `Session.mode`) is not running (contracts/terminal-mode-lifecycle.md's predicate).
fn attached_process_restartable(state: &State, id: SessionId) -> bool {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return false;
    };
    match session.mode {
        TerminalMode::AiCli => matches!(
            session.lifecycle,
            SessionLifecycle::Idle
                | SessionLifecycle::Failed
                | SessionLifecycle::InterruptedResumable
        ),
        TerminalMode::Regular => matches!(
            session.active_shell_lifecycle(),
            None | Some(ShellLifecycle::NotStarted | ShellLifecycle::Exited)
        ),
    }
}

/// The instance-switching control (feature 011, FR-004/FR-005; contracts/terminal-instance-
/// switcher-ui.md): one tab per open Regular Terminal instance, in creation order, labeled by
/// its `ShellInstanceId`'s numeric value (the only display identity an instance has). `None`
/// when the session isn't found or has zero/one instance — pixel-identical to the pre-feature-011
/// single-instance experience in that case (FR-005).
///
/// Each tab is one `button` spanning the whole entry — a press anywhere on it selects that
/// instance — with the close (and, when shown, restart) controls nested inside as their own
/// buttons. iced's `Button` always gives its content first crack at an event, so a press that
/// lands on the nested close/restart button is captured there and never reaches the tab's own
/// `on_press`; a press anywhere else on the tab falls through to select it. The active tab is
/// marked with a solid fill (`style::filled`) vs. the low-emphasis `style::text_button` every
/// other tab uses — a background-color difference is legible at a glance, unlike a thin edge
/// accent (SC-004: users must be able to tell which instance is active from this row alone).
fn instance_switcher_row<'a>(
    state: &'a State,
    id: SessionId,
    r: tokens::Roles,
) -> Option<Element<'a, Message>> {
    let session = state.active_sessions().iter().find(|s| s.id == id)?;
    if session.shells.len() <= 1 {
        return None;
    }
    let mut entries = row![].spacing(spacing::SM).align_y(Alignment::Center);
    for instance in &session.shells {
        let is_active = session.active_shell == Some(instance.id);
        let label = text(instance.id.0.to_string()).size(type_scale::LABEL);
        let close = Tooltip::new(
            IconButton::new(Icon::Close, r)
                .size(type_scale::LABEL)
                .padding(spacing::XS)
                .circular()
                .on_press(Message::ShellInstanceCloseRequested(id, instance.id)),
            "Close this terminal instance",
            r,
        )
        .position(TooltipPosition::Top);
        let mut content = row![label, close]
            .spacing(spacing::XS)
            .align_y(Alignment::Center);
        // Per-instance restart affordance (feature 011 FR-010): shown exactly when this
        // instance's own lifecycle is not-running, independent of every sibling — a background
        // instance can be restarted without switching to it first.
        if matches!(
            instance.lifecycle,
            ShellLifecycle::NotStarted | ShellLifecycle::Exited
        ) {
            content = content.push(
                button(
                    text("restart")
                        .size(type_scale::LABEL)
                        .style(style::muted(r)),
                )
                .padding(spacing::SM)
                .style(style::text_button(r))
                .on_press(Message::ShellInstanceRestartRequested(id, instance.id)),
            );
        }
        let tab = button(content)
            .padding(spacing::SM)
            .on_press(Message::ShellInstanceSelected(id, instance.id));
        let tab = if is_active {
            tab.style(style::filled(r))
        } else {
            tab.style(style::text_button(r))
        };
        entries = entries.push(tab);
    }
    Some(entries.into())
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
    fn wire_color_matches_the_local_ansi_mapping() {
        // A daemon-streamed WireColor must resolve to the exact same iced colour the local
        // renderer produces for the equivalent AnsiColor — the framer's `wire_color` inverse.
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        assert_eq!(
            p.wire_color(WireColor::Rgb(10, 20, 30)),
            p.color(AnsiColor::Spec(AnsiRgb {
                r: 10,
                g: 20,
                b: 30
            }))
        );
        for i in 0u8..16 {
            assert_eq!(
                p.wire_color(WireColor::Indexed(i)),
                p.color(AnsiColor::Indexed(i))
            );
        }
        assert_eq!(
            p.wire_color(WireColor::Indexed(200)),
            p.color(AnsiColor::Indexed(200))
        );
        // Named discriminants ride the wire verbatim: 0..=15 ANSI-16, then the specials at their
        // real alacritty values (Foreground = 256, Background = 257) — NOT 16/17.
        assert_eq!(
            p.wire_color(WireColor::Named(NamedColor::Red as u16)),
            p.color(AnsiColor::Named(NamedColor::Red))
        );
        assert_eq!(
            p.wire_color(WireColor::Named(NamedColor::Foreground as u16)),
            p.color(AnsiColor::Named(NamedColor::Foreground))
        );
        assert_eq!(
            p.wire_color(WireColor::Named(NamedColor::Background as u16)),
            p.color(AnsiColor::Named(NamedColor::Background))
        );
    }

    // Regression (all-terminals-red): the daemon frames a default-background cell as
    // `NamedColor::Background as u16` (= 257). Decoding it must yield the theme background, never
    // ANSI red — the bug was a `u8` wire that truncated 257 → 1 = red, so every empty cell was red.
    #[test]
    fn default_background_decodes_to_theme_bg_not_red() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let p = TermPalette::from_scheme(scheme);
            assert_eq!(
                p.wire_color(WireColor::Named(NamedColor::Background as u16)),
                p.background(),
            );
            assert_eq!(
                p.wire_color(WireColor::Named(NamedColor::Foreground as u16)),
                p.foreground(),
            );
            assert_ne!(
                p.wire_color(WireColor::Named(NamedColor::Background as u16)),
                p.ansi16[1], // ANSI red — what the truncated wire used to select
            );
        }
    }

    #[test]
    fn wire_cell_colors_matches_local_cell_colors() {
        let p = TermPalette::from_scheme(ColorScheme::Dark);
        // Plain cell.
        let plain = WireStyle {
            fg: WireColor::Named(NamedColor::Red as u16),
            bg: WireColor::Named(NamedColor::Background as u16),
            flags: 0,
            underline_color: None,
        };
        assert_eq!(
            wire_cell_colors(&p, &plain, false),
            cell_colors(
                &p,
                AnsiColor::Named(NamedColor::Red),
                AnsiColor::Named(NamedColor::Background),
                Flags::empty(),
                false
            )
        );
        // Selection swaps fg/bg exactly as the local path does.
        assert_eq!(
            wire_cell_colors(&p, &plain, true),
            cell_colors(
                &p,
                AnsiColor::Named(NamedColor::Red),
                AnsiColor::Named(NamedColor::Background),
                Flags::empty(),
                true
            )
        );
        // INVERSE flag round-trips through the bits.
        let inverse = WireStyle {
            flags: Flags::INVERSE.bits(),
            ..plain
        };
        assert_eq!(
            wire_cell_colors(&p, &inverse, false),
            cell_colors(
                &p,
                AnsiColor::Named(NamedColor::Red),
                AnsiColor::Named(NamedColor::Background),
                Flags::INVERSE,
                false
            )
        );
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

    #[test]
    fn no_mouse_report_when_mouse_mode_off() {
        let mods = crate::keymap::Mods::NONE;
        assert_eq!(
            encode_mouse_report(TermMode::empty(), 0, 3, 4, true, mods),
            None
        );
    }

    #[test]
    fn sgr_mouse_report_is_one_based_with_press_marker() {
        let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        let mods = crate::keymap::Mods::NONE;
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
        let mods = crate::keymap::Mods {
            shift: true,
            ..crate::keymap::Mods::NONE
        };
        let seq = encode_mouse_report(mode, 0, 0, 0, true, mods).unwrap();
        assert_eq!(seq, b"\x1b[<4;1;1M");
    }

    /// FR-013a: the legacy (non-SGR) encoding reports every release as button 3, so a release
    /// must actually be sent — the process cannot infer which button came up.
    #[test]
    fn fr_013a_legacy_release_uses_the_button_release_code() {
        let mode = TermMode::MOUSE_REPORT_CLICK;
        let mods = crate::keymap::Mods::NONE;
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
        let mods = crate::keymap::Mods::NONE;
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
        let mods = crate::keymap::Mods::NONE;
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

    /// The scrollbar thumb tracks the scrollback position: hidden at the live bottom, pinned to the
    /// top when fully scrolled up (feature 006, FR-016). Pure — the offset is now client-tracked.
    #[test]
    fn scrollbar_thumb_tracks_scrollback_position() {
        use crate::ui::material::scrollbar_metrics;
        let (rows, history) = (5usize, 20usize);
        // At the live bottom (offset 0): the scrollbar is hidden.
        assert!(scrollbar_metrics(100.0, rows, history, 0).is_none());
        // Scrolled all the way up: the thumb pins to the very top of the track.
        let sb = scrollbar_metrics(100.0, rows, history, history)
            .expect("scrollbar visible while scrolled back");
        assert_eq!(sb.thumb_top, 0.0);
    }
}

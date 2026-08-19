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
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::{
    self, Button, ButtonVariant, ContextMenu, Divider, GridSizeReporter, IconButton, MenuItem,
    SurfaceKind, TerminalPane, Text, Tooltip, TooltipPosition, TypeRole,
};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Color, Element, Font, Length};
use micold_core::protocol::grid::{WireColor, WireStyle};
use micold_core::session::{SessionId, SessionLifecycle, ShellLifecycle, TerminalMode};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy, spacing, Rgb};

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

    // Every branch below wraps the terminal area in a `GridSizeReporter`, including the ones with no
    // terminal in it (BUG-003, FR-014a). The size of this rectangle is what a session must be
    // *started* at, and the empty state occupies exactly the rectangle the first session will be
    // displayed in — so measuring it here is what lets that session be spawned at the right size
    // instead of corrected a frame after its first output.
    let Some(active) = state.active_session else {
        return GridSizeReporter::new(
            container(
                Text::new(
                    "Select or start a session to open its terminal.",
                    TypeRole::Body,
                    r,
                )
                .muted(),
            )
            .padding(spacing::LG)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .into();
    };

    // The colour-rendering terminal body fills the whole main area (feature 006). Falls back to
    // an empty state if the runtime is not yet available (e.g. the session is still starting).
    let body: Element<'a, Message> = match grid {
        Some(grid) => TerminalPane::new(grid, TermPalette::from_scheme(scheme))
            .selection(selection)
            .display_offset(display_offset)
            .focused(state.terminal_focused())
            .into(),
        None => container(
            Text::new(empty_terminal_message(state, active), TypeRole::Caption, r).muted(),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
    };
    // Measured whether the pane, the placeholder, or nothing at all is inside it (see above).
    let body: Element<'a, Message> = GridSizeReporter::new(body).into();

    // While the right-click context menu is open, float it over the terminal body anchored at the
    // clicked point; choosing Copy/Paste or clicking outside dismisses it (FR-013).
    //
    // Hosted on the terminal body rather than on the window's overlay, because `(x, y)` is
    // pane-local: the pane's origin is not known at render time, so there is nothing to translate
    // the point by. Same primitive, mounted one level down.
    let body: Element<'a, Message> = match state.terminal_context_menu {
        Some((x, y)) => crate::ui::cdk::overlay::Overlay::new(body)
            .push(
                ContextMenu::new(
                    vec![
                        MenuItem::labeled("Copy", Message::TerminalCopyRequested),
                        MenuItem::labeled("Paste", Message::TerminalPasteRequested),
                    ],
                    (x, y),
                    Message::TerminalContextMenuClosed,
                    r,
                )
                .into(),
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
        Text::new(session_title(state, active), TypeRole::Label, r),
        Space::new().width(Length::Fill),
        Text::new(status, TypeRole::Label, r).muted(),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    // The attached process (per the current mode) isn't running — offer a manual restart
    // (FR-013; contracts/terminal-mode-lifecycle.md). Absent whenever it's already
    // running/starting, since there is nothing to restart.
    if attached_process_restartable(state, active) {
        bar = bar.push(
            Button::with_content(
                Text::new("restart", TypeRole::Label, r).muted(),
                ButtonVariant::Text,
                r,
            )
            .padding(spacing::SM)
            .on_press(restart_message(state, active)),
        );
    }
    // The bar carried a release-focus `IconButton` here until BUG-001 (feature 023 FR-021b). It
    // dated from feature 006, when the terminal took focus only from an explicit click and lost it
    // by clicking outside — an always-visible way out was a real safety valve then. Feature 023
    // replaced both halves of that model: navigation acquires the keyboard on its own, and the
    // click-outside release is gone. What survived was a permanently-visible button, disabled in
    // every state where the terminal did not hold the keyboard, duplicating the reserved
    // Ctrl+Shift+E / Cmd+Shift+E chord — which is what actually carries 006 FR-011's "never
    // trapped" guarantee, and still does.
    //
    // Its removal had to be **unconditional**. The bar's child list must not vary with focus
    // (feature 023 FR-008a): a focus-conditional child shifts every sibling after it, and iced's
    // positional `Tree::diff_children` then hands the pressed control its neighbour's node,
    // dropping the `is_pressed` that `on_press` fires from — the press vanishes and the user has
    // to press twice (research R1). A child that never exists cannot shift anything, so deleting
    // it satisfies that rule exactly as pushing it unconditionally did; gating it on focus would
    // not. `tests/terminal_bar_stability.rs` holds both ends: `the_bar_does_not_branch_on_focus`
    // and `the_bar_has_no_release_focus_control`.
    //
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
    let bottom_bar = material::Surface::new(bar, SurfaceKind::Toolbar, r)
        .width(Length::Fill)
        .padding(spacing::SM);

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

/// What the terminal area says when it has no grid to render (feature 025 FR-014, BUG-001;
/// contracts/last-session-memory.md §4.3).
///
/// "No grid" used to have one cause. Feature 006 wrote this branch when the only way to reach it
/// was a session whose process was on its way, and `Starting…` named that cause correctly. Feature
/// 025 added a second: a session restored at launch, which is current, has no process, and has no
/// output — none survives a restart, because output lives in the client's memory and is rebuilt
/// from frames the daemon streams for a *running* process. Left alone, the pane told every such
/// user to wait for something that would never happen, while the bar one row below said
/// `interrupted` and offered `restart`.
///
/// Answered from [`attached_process_restartable`] rather than from a second match on
/// `SessionLifecycle`. That predicate already means "the attached process is not running", already
/// handles the `TerminalMode` split, and is already what decides whether the `restart` control is
/// there to be pointed at. Deriving both from it is what makes the two *unable* to disagree —
/// which they did, for exactly as long as they were two readings of one fact.
fn empty_terminal_message(state: &State, id: SessionId) -> &'static str {
    if attached_process_restartable(state, id) {
        "This session is not running. Choose restart below to resume it."
    } else {
        "Starting…"
    }
}

/// What the bottom-bar restart control must ask for (`012` FR-010, BUG-004).
///
/// The same question [`attached_process_restartable`] asks to decide *whether* to offer a restart —
/// which process is attached — asked again to decide *what* the restart acts on. It used to press
/// `TerminalRestartRequested` unconditionally, which restarts the **session**; in Regular mode the
/// session's AI CLI primary is still alive, so `start_session` took its already-live early return
/// and the control did nothing. With a single instance there is no tab strip either, so FR-010's
/// affordance had no reachable route at all.
///
/// A session with no instance yet keeps the session-level request: there is nothing instance-shaped
/// to name, and that path lazily opens the first one.
fn restart_message(state: &State, id: SessionId) -> Message {
    let instance = state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .filter(|s| s.mode == TerminalMode::Regular)
        .and_then(|s| s.active_shell);
    match instance {
        Some(instance) => Message::ShellInstanceRestartRequested(id, instance),
        None => Message::TerminalRestartRequested,
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

/// The accent an active switcher tab's indicator is drawn in — `None` for an inactive tab
/// (BUG-002, FR-004b).
///
/// A tab strip marks its selected member with an **indicator**, not with a container. BUG-001 had
/// this as `tab_variant`, choosing `Filled` for the active tab and `Outlined` for the rest, and
/// that was the wrong idiom: it read the original defect (one filled pill among loose numbers) as
/// "the entries need containers", when the missing container was only half of it. The half nothing
/// had ever written down is that a tab strip underlines the active tab. Neither tab draws a
/// container now.
///
/// Its own function for the same reason its predecessor was: the rule is then a pure value test
/// rather than a claim about a `view()` no unit test can reach.
pub(crate) fn tab_indicator_colour(is_active: bool, r: tokens::Roles) -> Option<Rgb> {
    is_active.then_some(r.primary)
}

/// A switcher tab's width (BUG-002).
///
/// Uniform and fixed, which is what makes the indicator work at all. The indicator is a rule, and a
/// rule spans the width it is given — `Length::Fill` inside a content-sized tab resolves against the
/// *button's* available space, not the label's, so the active tab stretched to several times its
/// neighbour's width and activation resized it under the pointer. Found by the visual pass; no gate
/// could see it, because every node was exactly where its own layout said it was.
///
/// A fixed width fixes it at the root rather than by measuring: the indicator's `Fill` resolves to
/// this, every tab measures the same whatever it contains, and SC-008 holds by construction instead
/// of by arithmetic. It also survives the rename feature — a name ellipsises inside the tab rather
/// than resizing the strip, which is how a browser tab bar behaves.
///
/// **Derived, not chosen** (BUG-005, FR-004c). It was written as a literal `128.0` — a number that
/// made the three tab states the BUG-002 visual pass drew look right, and that no test could
/// disagree with. There is a fourth state, and 128 is not enough for it: a tab whose instance has
/// stopped carried a restart button too, and the row settled the 54.3dp shortfall by shrinking its
/// trailing children until the button was 0.0 wide and the close control was 45.2, under §7.3's
/// target. Nothing overflowed, so nothing failed.
///
/// BUG-005 answered that by moving the restart affordance out to a context menu (FR-010b) rather
/// than by widening every tab to 204dp for a child only a stopped instance draws. What changed here
/// is that the width is *computed from the things it has to hold*, so it moves when any of them
/// does and a fifth child cannot be added without this sum being confronted.
///
/// It comes to **136**, not the 128 that was written — and the 8dp is worth reading rather than
/// tuning away. The literal reserved about 8dp for the label, which is narrower than the two digits
/// an instance ordinal already reaches and far narrower than any name. It was never noticed because
/// a label smaller than its reserve simply leaves the tab looking roomy, and `1` is 6.8dp wide. The
/// honest way to land back on 128 is to declare that a tab reserves less room for its own name than
/// a two-digit number needs, and that is not true — so the sum stands and the strip grows 8dp a tab.
/// Choosing the reserve to reproduce the old figure is exactly the move FR-004c was rewritten to
/// forbid.
const TAB_WIDTH: f32 = 2.0 * spacing::SM      // the tab button's own padding, both edges
    + TAB_CLOSE_WIDTH                          // leading spacer, balancing the close control
    + spacing::XS
    + TAB_LABEL_MIN_WIDTH
    + spacing::XS
    + TAB_CLOSE_WIDTH; // the close control itself

/// The widest a switcher tab's label may grow before it ellipsises (BUG-002, T054).
///
/// A *maximum*, not the fixed two-digit box BUG-001 used. That box was sized for an ordinal, and an
/// instance is to become renameable from a right-click menu — a tab will show a name, and a width
/// chosen for `99` would have to be undone that day. Content-sized under a ceiling serves both, and
/// costs nothing now.
const TAB_LABEL_MAX_WIDTH: f32 = 120.0;

/// The label's share of the derived [`TAB_WIDTH`] — what a tab reserves for its own name before the
/// two touch targets and the gaps are counted (BUG-005, FR-004c).
///
/// A floor, where [`TAB_LABEL_MAX_WIDTH`] is the ceiling at which a label ellipsises. It is not
/// measured text: a shaped width is not available in a `const`, and reserving one would make the
/// tab's width depend on its content, which is the thing FR-004c forbids. Sized instead by what has
/// to remain legible — comfortably more than the two digits an ordinal needs, and enough of a name
/// to tell two tabs apart once instances can be renamed.
const TAB_LABEL_MIN_WIDTH: f32 = 16.0;

/// The trailing close control's layout footprint. A leading spacer of the same width balances it,
/// putting the label on the tab's midline rather than off-centre toward the leading edge
/// (FR-004a).
///
/// It is §7.3's 48dp minimum touch target, **not** the glyph's visible size: a pressable, non-
/// compact `IconButton` wraps itself in a `MIN_TOUCH_TARGET` box so a small pill still gets a large
/// target (`icon_button.rs`). Measuring the visible pill instead — this was 24 in the first cut of
/// the fix — leaves the spacer narrower than the control it balances, and the label lands
/// `(48 - 24) / 2 = 12`dp left of centre. That is exactly what the visual pass caught, and it is why
/// this reads the anatomy constant rather than naming a number: the two must move together.
const TAB_CLOSE_WIDTH: f32 = anatomy::button::MIN_TOUCH_TARGET;

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
/// marked with a solid fill (`style::filled`) vs. the outlined container every other tab uses —
/// a background-color difference is legible at a glance, unlike a thin edge accent (SC-004: users
/// must be able to tell which instance is active from this row alone).
///
/// # Why every tab draws a container (BUG-001, FR-004a)
///
/// The inactive tabs used `ButtonVariant::Text`, which paints neither background nor outline. The
/// row therefore rendered as one filled pill among loose numbers with close glyphs floating beside
/// them — not a tab strip. Every behavioural test passed and SC-004 was met: you could tell which
/// instance was active, and the row still looked wrong. The active/inactive distinction has to be
/// *emphasis between two containers*, never container-versus-nothing, which is what `Outlined`
/// gives the inactive ones.
///
/// Two sizing rules keep activation from reflowing the row (SC-008). Both variants get the same
/// explicit `padding`, so neither §7.3 default applies and the two tabs measure alike; and the
/// label sits in a fixed-width centred box, so a two-digit instance id does not resize its tab
/// either. Nothing about *which* tab is active changes any child's size, so selecting a tab moves
/// only colour — nothing shifts under the pointer between a press and its release.
///
/// # Why the nested controls are tinted explicitly (BUG-001, FR-011a)
///
/// `IconButton::new` defaults its glyph to the roles' `on_surface`. That is right on a surface and
/// wrong the moment the button is nested inside something painting its own fill: on the active
/// tab, `style::filled` lays down `primary` and `on_surface` over it is near tone-on-tone, so the
/// close control all but disappeared on the one tab a user is most likely to want to close. The
/// tab's *label* was fine — plain `Text` inherits the button's `text_color` — so only the icon
/// opted out. Both nested controls now take `variant.content(r)`, the colour that variant draws
/// its own label in, which is the same rule the label follows and stays right for any variant
/// added later. `tests/icon_roles.rs` holds the contrast arithmetic; `tests/terminal_tabs.rs`
/// holds the call site.
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
        // No tab draws a container (FR-004b). The active one is marked by an indicator and by its
        // label taking the accent; the rest are muted labels.
        let indicator = tab_indicator_colour(session.active_shell == Some(instance.id), r);
        // Every nested control takes the colour this tab draws its own label in (FR-011a). That
        // matters more without a container than it did with one: the accent is now the only thing
        // separating the active tab from its neighbours, so a close glyph left on `on_surface`
        // would read as belonging to a different tab than the label beside it.
        let tint = indicator.unwrap_or(r.on_surface_variant);
        // Content-sized under a ceiling, centred (FR-004a's surviving clause). Not a fixed
        // two-digit box: a tab is to show a name once instances can be renamed.
        let label = container(Text::new(instance.id.0.to_string(), TypeRole::Label, r).tint(tint))
            .max_width(TAB_LABEL_MAX_WIDTH)
            .center_x(Length::Shrink);
        let close = Tooltip::new(
            IconButton::new(Icon::Close, r)
                .size(TypeRole::Label)
                .padding(spacing::XS)
                .circular()
                .tint(tint)
                .on_press(Message::ShellInstanceCloseRequested(id, instance.id)),
            "Close this terminal instance",
            r,
        )
        .position(TooltipPosition::Top);
        // A leading spacer the width of the trailing close control, so the label centres about
        // the tab's own midpoint rather than about the space left over beside the close: the two
        // ends are then equal and the label's box sits exactly between them. A `Fill` spacer
        // would do nothing here — the tab sizes to its content, so there is no slack to push
        // against, and it would only add a gap on one side of the label.
        let content = row![
            Space::new().width(Length::Fixed(TAB_CLOSE_WIDTH)),
            label,
            close,
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center);
        // The per-instance restart affordance used to be pushed here as a fourth child, and the
        // comment above it read "It widens its own tab, which SC-008 permits: that is a lifecycle
        // change, not a change of which tab is active." That was true until BUG-002 gave every tab
        // one fixed width, and false in the same file from that commit on — a fixed width is a
        // budget, and iced settles a shortfall by shrinking the trailing children rather than by
        // overflowing. The button laid out at 0.0dp wide and the close control beside it at 45.2,
        // under §7.3's target. Nothing overflowed, so nothing failed: `mise run test` was green
        // over a control a user could not press, and the instance FR-010 exists for — one that
        // exited in the background — could not be restarted from its own tab at all.
        //
        // It is offered from a context menu on the tab now (BUG-005, FR-010b). Widening the tab
        // instead was measured first and rejected: the derivation comes to 204dp against 136, so
        // three instances would take 628dp of a 1014dp bar that also carries a title, a status, the
        // "+" and the mode toggle — every tab paying for a child only a stopped instance draws.
        // The indicator sits at the tab's **top** edge, not Material's bottom: this bar is anchored
        // to the window's bottom, so the pane a tab selects is *above* it and a bottom indicator
        // would point away from what it marks (FR-004b).
        //
        // Every tab reserves the bar's height whether or not it draws one — an inactive tab gets a
        // transparent rule of the same thickness. An indicator that appeared only on activation
        // would grow its tab by 3dp and push the row, which is exactly the reflow SC-008 forbids,
        // and it would do it under the pointer between a press and its release.
        //
        // `Fill` on the column, not `Shrink`, and that is the *width* half of the same rule. The
        // active tab's `Divider` fills, so its column measures the tab's whole content box and the
        // row below centres in it; an inactive tab's transparent spacer has no width, so a shrinking
        // column would measure only the row and pin it to the leading edge. The label would then sit
        // off the tab's midline on every inactive tab and slide across on activation — under the
        // pointer, which is what SC-008 forbids — by half the slack. That was 0.6dp while the tab was
        // 128 wide and became 4.6dp when FR-004c's derivation corrected it to 136: the same defect,
        // amplified past visibility by a change that had nothing to do with it.
        let marked = column![
            match indicator {
                Some(accent) => Divider::horizontal(r)
                    .thickness(anatomy::tab::INDICATOR)
                    .tint(accent)
                    .into(),
                None => Element::from(Space::new().height(Length::Fixed(anatomy::tab::INDICATOR))),
            },
            content,
        ]
        .width(Length::Fill)
        .align_x(Alignment::Center);
        entries = entries.push(
            ContextArea::new(
                Button::with_content(marked, ButtonVariant::Text, r)
                    // `Text` on every tab: no background, no outline (FR-004b). One fixed width for all
                    // of them, so the indicator's `Fill` resolves to the tab rather than to whatever
                    // space the bar happens to offer, and every tab measures the same (SC-008).
                    .width(Length::Fixed(TAB_WIDTH))
                    .padding(spacing::SM)
                    .on_press(Message::ShellInstanceSelected(id, instance.id)),
            )
            // A secondary press opens this tab's menu; a primary press still selects the instance,
            // because the wrapper lets the child answer first and intercepts only the right button.
            .on_secondary_press(move |(x, y)| {
                Message::ShellInstanceMenuRequested(instance.id, x, y)
            }),
        );
    }
    Some(entries.into())
}

#[cfg(test)]
mod tests {
    //! Colour-mapping tests for `TermPalette` (feature 006, FR-001/FR-003), and the switcher
    //! tab's variant rule (feature 012 BUG-001). Bin unit tests — run with
    //! `cargo test --features gui`. See contracts/terminal-render-input.md.
    use super::*;
    use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb as AnsiRgb};
    use micold_core::project::{Availability, Project};
    use micold_core::session::{Session, SessionLifecycle, SessionLocation};

    /// A state with one project and one session at `lifecycle`, made current.
    fn state_showing(lifecycle: SessionLifecycle) -> (State, SessionId) {
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state.workspace.active = Some(path.clone());
        let mut session = Session::start_new(SessionLocation::Default);
        session.lifecycle = lifecycle;
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.active_session = Some(id);
        (state, id)
    }

    /// `012` BUG-004 / FR-010. The bar's `restart` control restarted the **session**, whose AI CLI
    /// primary is still alive in Regular mode — so `start_session` took its already-live early
    /// return and pressing the control did nothing at all. With a single instance there is no tab
    /// strip either, so FR-010 had no reachable route in the commonest case.
    ///
    /// Asserted on the message rather than on the presence of a control: `attached_process_restartable`
    /// already decides *whether* to offer a restart by asking which process is attached, and the
    /// defect was the button not asking the same question about *what to restart*. Both now come
    /// from one reading, which is what stops them disagreeing again (this feature's BUG-001, and
    /// `025`'s, are the same shape).
    #[test]
    fn the_bars_restart_targets_the_process_the_bar_is_describing() {
        // AI CLI mode: the session is the attached process, so the session-level restart is right.
        let (state, id) = state_showing(SessionLifecycle::Idle);
        assert_eq!(
            restart_message(&state, id),
            Message::TerminalRestartRequested
        );

        // Regular mode with an instance: that instance is what the bar reports on, and what
        // `restart` must act on.
        let (mut state, id) = state_showing(SessionLifecycle::Running);
        let instance = {
            let (_, session) = state.workspace.find_session_mut(id).expect("session");
            session.mode = TerminalMode::Regular;
            session.open_shell_instance()
        };
        assert_eq!(
            restart_message(&state, id),
            Message::ShellInstanceRestartRequested(id, instance),
            "in Regular mode the bar describes the attached shell instance, so its restart must \
             name that instance — restarting the session leaves the dead shell dead"
        );

        // Regular mode with no instance yet: nothing instance-shaped to name, so the session-level
        // request stands and lazily opens the first one.
        let (mut state, id) = state_showing(SessionLifecycle::Running);
        {
            let (_, session) = state.workspace.find_session_mut(id).expect("session");
            session.mode = TerminalMode::Regular;
        }
        assert_eq!(
            restart_message(&state, id),
            Message::TerminalRestartRequested
        );

        // An unknown session must not panic the render path.
        assert_eq!(
            restart_message(&state, SessionId::new()),
            Message::TerminalRestartRequested
        );
    }

    /// BUG-001 (feature 025), FR-014 / contract §4.3.
    ///
    /// The terminal area falls to an empty state whenever it has no grid, and a restored session
    /// has none — output lives only in the client's memory and is rebuilt from frames the daemon
    /// streams for a *running* process, so nothing survives a restart. Saying "Starting…" there
    /// tells the user to wait for an event that will never arrive, while the bar one row below
    /// says `interrupted` and offers `restart`.
    ///
    /// Driven over all three not-running lifecycles rather than the one that reaches this at
    /// launch: they are the set [`attached_process_restartable`] already names, and a fix keyed to
    /// `InterruptedResumable` alone would pass a single-case test while leaving a session that was
    /// restored and then stopped saying the same false thing.
    #[test]
    fn a_session_that_is_not_running_is_not_described_as_starting() {
        for lifecycle in [
            SessionLifecycle::InterruptedResumable,
            SessionLifecycle::Idle,
            SessionLifecycle::Failed,
        ] {
            let (state, id) = state_showing(lifecycle);
            let message = empty_terminal_message(&state, id);
            assert!(
                !message.contains("Starting"),
                "{lifecycle:?} is not starting and nothing will start it, yet the terminal area \
                 said {message:?} (FR-014)"
            );
            assert!(
                message.contains("restart"),
                "{lifecycle:?} offers `restart` in the bar below (attached_process_restartable), \
                 so the empty state should point at the control that resolves it — {message:?}"
            );
        }
    }

    /// The other half, without which the test above passes on a blank string: a session that
    /// genuinely is coming up must still say so. FR-014 asks the wording to *distinguish* the two,
    /// not to stop mentioning starting.
    #[test]
    fn a_session_that_is_starting_still_says_so() {
        for lifecycle in [
            SessionLifecycle::Starting,
            SessionLifecycle::Restarting { attempts: 1 },
            SessionLifecycle::Running,
        ] {
            let (state, id) = state_showing(lifecycle);
            assert_eq!(
                empty_terminal_message(&state, id),
                "Starting…",
                "{lifecycle:?} has a process up or on its way, so an empty pane really is waiting \
                 for the first frame"
            );
        }
    }

    /// The guarantee the fix is built on, and the reason this is one predicate rather than two
    /// mappings of one fact: the body and the bar cannot disagree, because both derive from
    /// `attached_process_restartable`. A future lifecycle added to one is added to the other.
    #[test]
    fn the_empty_state_and_the_restart_control_never_disagree() {
        for lifecycle in [
            SessionLifecycle::Idle,
            SessionLifecycle::Starting,
            SessionLifecycle::Running,
            SessionLifecycle::Restarting { attempts: 1 },
            SessionLifecycle::Failed,
            SessionLifecycle::InterruptedResumable,
        ] {
            let (state, id) = state_showing(lifecycle);
            let restartable = attached_process_restartable(&state, id);
            let says_starting = empty_terminal_message(&state, id).contains("Starting");
            assert_ne!(
                restartable, says_starting,
                "{lifecycle:?}: the bar offers restart = {restartable}, and the body claims to be \
                 starting = {says_starting}. Exactly one of those is true of any session"
            );
        }
    }

    /// BUG-002, FR-004b: exactly the active tab carries an indicator.
    ///
    /// Replaces `tab_variant_always_draws_a_container`, which asserted neither arm was
    /// `ButtonVariant::Text`. That test was right for BUG-001 and is wrong now — every tab is
    /// `Text`, because no tab draws a container. It is replaced rather than deleted: a test that
    /// pins a decision *should* fail when the decision changes, and what would be wrong is leaving
    /// the new rule unpinned afterwards.
    #[test]
    fn only_the_active_tab_carries_an_indicator() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert_eq!(
                tab_indicator_colour(true, r),
                Some(r.primary),
                "{scheme:?}: the active tab must be marked by an accent indicator (FR-004b)"
            );
            assert_eq!(
                tab_indicator_colour(false, r),
                None,
                "{scheme:?}: an inactive tab draws no indicator — the mark is what distinguishes \
                 the active one, so a second bar would say two tabs are selected"
            );
        }
    }

    /// BUG-005, FR-004c: the tab's width is the sum of what it has to hold, not a number.
    ///
    /// The test that would have failed the day `TAB_WIDTH` was written as `128.0`. It cannot catch
    /// a *missing* child on its own — a sum is only as complete as its terms, and the term this bug
    /// was about (the restart affordance) is no longer one of them — so it is the pair to
    /// `tests/gates/tab_children_fit.rs`, which reads what the children were actually given. This
    /// end says the budget is the sum of its parts; that end says nobody was squeezed.
    ///
    /// Restated rather than referenced, deliberately. Writing `assert_eq!(TAB_WIDTH, TAB_WIDTH)`
    /// through the same expression would pass on any value; spelling the arithmetic out means a
    /// term silently dropped from the definition fails here.
    #[test]
    fn the_tab_width_is_the_sum_of_what_a_tab_holds() {
        let padding = 2.0 * spacing::SM;
        let targets = 2.0 * anatomy::button::MIN_TOUCH_TARGET; // leading spacer + close control
        let gaps = 2.0 * spacing::XS;
        assert_eq!(
            TAB_WIDTH,
            padding + targets + gaps + TAB_LABEL_MIN_WIDTH,
            "TAB_WIDTH must be derived from the constants a tab's widest arrangement requires \
             (FR-004c), not chosen against an observed one. A chosen figure is silently wrong the \
             first time a tab gains a child, and wrong in the one way layout does not report: iced \
             settles a shortfall by shrinking the trailing children, so the control disappears \
             instead of the row overflowing."
        );
    }

    /// The leading spacer balances the whole trailing edge, which is what puts the label on the
    /// tab's midline (FR-004a).
    ///
    /// One control on that edge today. It was briefly two — the close and a restart button — and
    /// the label was then off centre by 30dp with nothing to say so, because the spacer balanced
    /// only the close. FR-010b took the restart out; this fails if anything is put back.
    #[test]
    fn the_leading_spacer_balances_the_trailing_edge() {
        assert_eq!(
            TAB_CLOSE_WIDTH,
            anatomy::button::MIN_TOUCH_TARGET,
            "the spacer must balance the control it faces at that control's laid-out footprint, \
             not at its visible pill — a pressable non-compact `IconButton` wraps itself in a \
             MIN_TOUCH_TARGET box, and measuring the pill put the label (48 - 24) / 2 = 12dp left \
             of centre (BUG-002's visual pass)"
        );
    }

    /// The indicator is the *only* difference between the two states, and it must be an accent —
    /// not the surrounding bar's foreground, which would read as a border artefact rather than a
    /// selection (SC-009).
    #[test]
    fn the_indicator_is_an_accent_not_a_surface_colour() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let accent = tab_indicator_colour(true, r).expect("active tab has an indicator");
            assert_ne!(
                accent, r.on_surface,
                "{scheme:?}: the indicator must be an accent, not the bar's own foreground"
            );
            assert_ne!(
                accent, r.surface,
                "{scheme:?}: an indicator painted in the surface colour is invisible"
            );
        }
    }

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

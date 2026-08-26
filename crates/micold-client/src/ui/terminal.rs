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
use crate::ui::material::{
    self, tab_content_colour as content_colour, Button, ButtonVariant, ContextMenu,
    GridSizeReporter, IconButton, MenuItem, SurfaceKind, Tab, TabStrip, TerminalPane, Text,
    Tooltip, TooltipPosition, TypeRole, TAB_WIDTH,
};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::widget::{column, container, row};
use iced::{Alignment, Color, Element, Font, Length, Padding};
use micold_core::protocol::grid::{WireColor, WireStyle};
use micold_core::session::{
    AiCli, SessionId, SessionLifecycle, ShellInstanceId, ShellLifecycle, TerminalMode,
};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Rgb};

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
    // (right), with the tab strip, the "+" and the AI tab filling the rest and finishing flush
    // against the bar's trailing edge (feature 027 FR-001). A live activity indicator
    // (spinner/idle icon) is a planned follow-up feature.
    let status = session_status(state, active);
    let mut bar = row![
        Text::new(session_title(state, active), TypeRole::Label, r),
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
    // The instance-switching control: one tab per open Regular Terminal instance (feature 011,
    // FR-004/FR-005), always visible (026 FR-003). Placed just before the "open a new instance"
    // control and the AI tab, which are what the row's trailing end holds since feature 027
    // deleted the mode toggle that used to sit past them.
    // **The bar's one flexible member** (FR-002c). Everything else here is content-sized, so the
    // strip is what absorbs whatever width is left over — and, crucially, what runs out of it
    // first when there is not enough to go round.
    //
    // It used to be content-sized like its siblings, with a `Length::Fill` spacer between the title
    // and the status doing the pushing. That was silently wrong past about five instances: a row's
    // width is a **budget**, and iced settles a shortfall by shrinking the *trailing* children
    // rather than by overflowing. The bar's last child goes first — it was the mode toggle then,
    // laid out at **0.0dp** in `gates/bar_controls_hold_their_size.rs`'s six-instance state, which
    // is nothing a user can press. It is the AI tab now (027 FR-002), which makes the consequence
    // worse rather than better: the AI tab is the *only* route back to the assistant since the
    // toggle went, so a squeezed one is a pane with no way into it. Nothing overflowed, so nothing
    // failed. This is feature 012's BUG-005 one level out, and its own comment describes the same
    // shape inside a tab.
    //
    // Making the strip `Fill` inverts it: the strip is allotted `bar - everything else`, so every
    // other control keeps the size it measured and the strip is the thing that runs short. What it
    // does when it runs short is FR-002a's business — it scrolls (T033).
    //
    // # Why the region also has to scroll
    //
    // Bounding the strip on its own does not fix the defect; it **relocates** it one level in. The
    // strip is a row too, so a strip given less width than its tabs need settles the shortfall the
    // same way the bar did — by shrinking its trailing children. Measured: at six instances the
    // bar's controls were saved and a tab came out **55.5dp** wide with its close control at 0.0,
    // which `gates/tab_children_fit.rs` reports as the very defect feature 012's BUG-005 was.
    //
    // So the region scrolls (FR-002a). Tabs keep their one fixed width and the ones that do not fit
    // are reachable by the wheel rather than by being made smaller — no shrinking, no ellipsis, no
    // dropping. It comes from `material::Scrollable` rather than from a hand-rolled scroller
    // because that wrapper is where the design system's 4px themed bar lives and where
    // dismiss-on-scroll is reported from; a private one would reintroduce exactly the divergence
    // the component was created to end.
    //
    // The tabs sit at the **trailing** edge of that region (027 FR-003), which reverses what 012
    // FR-002c's second half asked for and is amended there rather than quietly contradicted. The
    // cost that clause named is real and is now paid deliberately: a strip that hugs the trailing
    // edge moves its own first tab left every time an instance is opened. What is bought with it is
    // that the two controls a user reaches for by muscle memory — the "+" and the AI tab — stay
    // anchored to the bar's edge instead of sliding with the tab count, which is the arrangement
    // the toggle's deletion made load-bearing.
    //
    // The push is this container's own **padding**, not a `Length::Fill` and not a `Space`. Inside
    // a horizontal `Scrollable` a `Fill` resolves against an **unbounded** width limit — infinity,
    // not "whatever is left" — so it would carry the strip out of the viewport entirely, and a
    // zero-width `Space` is void, which iced drops from the tree entirely (see `right_aligned_tabs`).
    // `leading_slack` derives the figure from the tab *count* rather than from the measured
    // `tab_strip_content_width`, because the slack is laid out inside that content and feeding the
    // measurement back in makes the strip swing flush-right, flush-left on alternate frames.
    //
    // Pushed **unconditionally** (FR-003). It used to be `if let Some(switcher)`, which is a bar
    // child that comes and goes with the session's shape — the very thing feature 023 FR-008a
    // forbids, and for the reason recorded above the deleted release-focus control: a conditional
    // child shifts every sibling after it, and iced's positional tree diff then hands the pressed
    // control its neighbour's node and drops the press. Opening a second instance renumbered the
    // "+" and everything past it under whatever press was in flight. The "+" itself carried the
    // same latent defect until feature 027 — it was pushed only in `TerminalMode::Regular`, so the
    // AI pane renumbered the bar's tail — and it is unconditional now for the same reason.
    let beyond = strip_overflow(
        state.tab_strip_scroll_offset as f32,
        state.tab_strip_viewport_width as f32,
        strip_tab_count(state, active),
    );
    // FR-002e: the edge fade takes the **indicator's own accent** when the tab beyond it is the
    // marked one, and the surface's tint otherwise. Two states of one cue differing only in role,
    // so the edge is tinted with the very colour the user is scanning for — no glyph, no arrow, no
    // second width. And no scroll-arrow controls at all (FR-002f): they would spend an interactive
    // target's width at each end of the bar FR-002c exists to keep uncrowded, and the wheel over a
    // scrollable is how this application already scrolls its sidebar and its scrollback.
    let marked_beyond = match marked_tab_index(state) {
        Some(index) => scroll_into_view(
            index,
            state.tab_strip_scroll_offset as f32,
            state.tab_strip_viewport_width as f32,
        )
        .map(|target| target < state.tab_strip_scroll_offset as f32),
        None => None,
    };
    bar = bar.push(
        material::EdgeFade::new(
            material::Scrollable::new(right_aligned_tabs(state, active, r), r)
                .direction(material::ScrollDirection::Horizontal)
                .width(Length::Fill)
                .id(TAB_STRIP_SCROLL_ID.clone())
                // Two numbers, one question. See `Scrollable::on_scroll_metrics`: an offset
                // measured against a stale viewport width is a fade that points at nothing. The
                // third number it reports — the content width — is deliberately dropped; see
                // `strip_overflow` for why a measured one cannot be paired with a live viewport.
                .on_scroll_metrics(|offset, width, _content| Message::TabStripScrolled {
                    offset,
                    width,
                })
                // `on_scroll` fires only when something scrolls, and the frame that matters most is
                // the **first** one, where nothing has: a strip that already overflows on its first
                // layout must fade its edge before the user touches it.
                .on_viewport_resize(|size| Message::TabStripViewportResized {
                    width: crate::app::scroll_offset_px(size.width),
                }),
            r,
        )
        .leading(beyond.leading)
        .trailing(beyond.trailing)
        .accent_on(marked_beyond)
        .width(Length::Fill),
    );
    // Open an additional Regular Terminal instance (feature 011, FR-001/FR-005). **Unconditional**
    // since feature 027 FR-004: it used to be drawn only in Regular mode, which was coherent while
    // a mode toggle existed — a session on its AI tab had another way back to a terminal. It has
    // none now, so a "+" that disappears there would strand a session with no instances yet.
    //
    // Pushing it unconditionally is also what feature 023 FR-008a asks for and what the `if` here
    // was quietly breaking: a bar child that comes and goes with the session's mode shifts every
    // sibling after it, and iced's positional `Tree::diff_children` then hands the pressed control
    // its neighbour's node and drops the press.
    //
    // It sits **before** the AI tab rather than after it. Both are anchored at the bar's trailing
    // edge and the strip grows leftward away from them, so neither is displaced by an instance
    // count — which is the half of feature 012's FR-002c that survives right-alignment.
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
    // The AI tab anchors the bar's bottom-right corner (feature 027 FR-001) — pushed last so it
    // always sits at the far right regardless of which other controls are present. Outside the
    // scrolling viewport above, so it keeps that position and stays reachable in one press at any
    // instance count (026 FR-002b, SC-002, SC-008), and pushed unconditionally like every other
    // child here: the bar's child list must not vary (feature 023 FR-008a). The mode toggle
    // held this corner from feature 010 until feature 027 deleted it: with every pane reachable by
    // its own tab, a control meaning "switch to the other one" was a second vocabulary for the same
    // navigation, and the only one that could not say where it was going.
    bar = bar.push(pinned_ai_tab(state, active, r));
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

/// Which AI CLI the open session runs, for the name on its pinned AI tab (FR-016a).
///
/// Falls back to [`AiCli::default`] rather than refusing to render: a bar drawn for a session the
/// projection does not (yet) list is a transient the view has to survive, and the default is the
/// same one a session with nothing recorded resumes on (FR-003, FR-013).
fn session_provider(state: &State, id: SessionId) -> AiCli {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.provider)
        .unwrap_or_default()
}

fn session_title(state: &State, id: SessionId) -> String {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.label.display().to_string())
        .unwrap_or_else(|| "Session".to_string())
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

/// Which member of the strip something refers to (feature 026, `data-model.md`).
///
/// **A closed two-variant enum, not an `Option<ShellInstanceId>`.** Principle V asks that invalid
/// states be unrepresentable, and this is where FR-005's "never zero, never two" is either
/// structural or a rule somebody has to keep. `None` already means something else in this file —
/// "this session has no active instance" — so overloading it to also mean "the AI tab" gives one
/// value two meanings and makes [`marked_tab`] unanswerable in the one case that matters. A closed
/// enum makes the marked tab a **total function** of `(TerminalMode, Option<ShellInstanceId>)`, so
/// exactly one tab is marked because there is nowhere else for the answer to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripTab {
    /// One open Regular Terminal instance.
    Instance(ShellInstanceId),
    /// The session's single AI CLI process.
    Ai,
}

/// Which tab the strip marks — the one whose content the pane is displaying (FR-005).
///
/// Total by construction, which is the whole of FR-005's "never zero, never two". The `Regular`
/// session with no active instance falls to the AI tab rather than to nothing: that is what the
/// pane shows in that state, and a strip that marked no tab there would be the exact defect this
/// feature exists to remove, arriving by a different route.
///
/// It reads `mode`, and since feature 027 nothing else writes it but the tabs themselves, so
/// FR-008's "two routes to this pane must not be able to disagree" is structural rather than a
/// synchronisation effort — there is no second selection to keep in step.
pub fn marked_tab(state: &State, id: SessionId) -> StripTab {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return StripTab::Ai;
    };
    match session.mode {
        TerminalMode::AiCli => StripTab::Ai,
        TerminalMode::Regular => session
            .active_shell
            .map(StripTab::Instance)
            .unwrap_or(StripTab::Ai),
    }
}

/// Whether the process behind `tab` is **stopped** — not running, and restartable (feature 026
/// FR-012d, research R1/R2).
///
/// One predicate for both lifecycle vocabularies, and three things are derived from it: whether a
/// tab wears the stopped mark (FR-012d), whether that tab's menu carries a restart item (FR-006a),
/// and therefore whether the menu opens at all (FR-006b). FR-012d asks the mark and the menu to
/// *agree*, and deriving both from one function is what makes that true by construction rather than
/// by two matches happening to say the same thing.
///
/// This file has already paid twice for the alternative. `empty_terminal_message`'s own comment
/// records that the pane and the bar disagreed "for exactly as long as they were two readings of
/// one fact"; BUG-004 was `restart_message` re-deriving something the predicate beside it already
/// had. [`attached_process_restartable`] is a call into this one for the same reason.
///
/// **The rule is translated, not the names.** FR-012d names `NotStarted` and `Exited`, which are a
/// shell's vocabulary; the AI process has a larger lifecycle of its own. The rule underneath is
/// FR-012d's: the mark appears exactly where a restart can act. `Starting` and `Restarting` are
/// excluded in both rows by FR-012e — they are in progress, and a mark on a state nobody can act on
/// sends a user to a press that does nothing (FR-006b), which is the dead end the mark exists to
/// prevent.
pub fn process_stopped(state: &State, id: SessionId, tab: StripTab) -> bool {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return false;
    };
    match tab {
        StripTab::Ai => matches!(
            session.lifecycle,
            SessionLifecycle::Idle
                | SessionLifecycle::Failed
                | SessionLifecycle::InterruptedResumable
        ),
        StripTab::Instance(instance) => matches!(
            session
                .shells
                .iter()
                .find(|s| s.id == instance)
                .map(|s| s.lifecycle),
            None | Some(ShellLifecycle::NotStarted | ShellLifecycle::Exited)
        ),
    }
}

/// Whether the bottom-bar restart control should show (FR-013): the currently-attached process
/// (per `Session.mode`) is not running (contracts/terminal-mode-lifecycle.md's predicate).
///
/// A thin call into [`process_stopped`] since feature 026 (research R2), asking it about whichever
/// tab the pane is currently showing. Keeping it as its own name rather than inlining the call: the
/// bar's question is "is *the* attached process restartable", which is a different question from
/// the strip's "is *this* tab's process restartable", and it happens to be the strip's question
/// asked about the marked tab.
fn attached_process_restartable(state: &State, id: SessionId) -> bool {
    process_stopped(state, id, marked_tab(state, id))
}

/// The tab strip's scroll viewport, by name, so `operation::scroll_to` can reach it (FR-002d).
///
/// A `LazyLock` for the reason the sidebar's is one: `scrollable::Id` is not a `const`, and the id
/// has to be the *same* one on the widget and in the operation or the operation finds nothing and
/// reports nothing.
pub static TAB_STRIP_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("terminal-tab-strip-scroll"));

/// Which edges of the scrolling region have tabs beyond them (FR-002e).
///
/// Two independent facts, not one enum, because a strip scrolled to the middle has content beyond
/// **both** edges and each fade is drawn on its own side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Beyond {
    /// Tabs lie before the viewport's leading edge.
    pub leading: bool,
    /// Tabs lie after its trailing edge.
    pub trailing: bool,
}

/// Half a pixel — the same tolerance the layout gates use, and for the same reason: a strip
/// scrolled exactly to one end must read as *at* that end rather than a hair short of it.
const EDGE_TOLERANCE: f32 = 0.5;

/// Whether anything lies beyond either edge of a viewport `viewport` wide, holding `content`, and
/// currently scrolled `offset` from the leading end (FR-002e, research R6).
///
/// Pure arithmetic, deliberately. The *fade* is appearance and no layout gate can see it — a
/// gradient occupies the same box whether it is opaque or invisible, which is the family of defect
/// the `visual-pass` skill exists for. What can be gated is the fact behind it, and this is it.
pub(crate) fn overflowing(offset: f32, viewport: f32, content: f32) -> Beyond {
    Beyond {
        leading: offset > EDGE_TOLERANCE,
        trailing: offset + viewport + EDGE_TOLERANCE < content,
    }
}

/// What lies beyond the strip's edges, for a viewport `viewport` wide holding `tabs` tabs and
/// scrolled `offset` from the leading end (FR-002e; feature 027 FR-003).
///
/// # Why the content width is derived here and not measured
///
/// `State::tab_strip_content_width` is only ever written by a scroll event, so between the strip's
/// first layout and the user's first scroll it holds whatever it last held — and since feature 027
/// the content width is a function of the viewport (the slack is laid out inside it), so a stale
/// pair can claim an overflow the strip does not have. It did: the first terminal instance opened
/// in a fresh session drew a trailing fade across its own tab, over the indicator, with nothing
/// beyond that edge at all. Found by the visual pass, invisible to every gate — a gradient occupies
/// the same box whether it is opaque or not.
///
/// Deriving it from the tab count is what makes the fade and the geometry incapable of disagreeing:
/// the strip lays out `leading_slack(..) + natural_strip_width(..)`, so that *is* the content width,
/// and it is known on the same frame the layout is rather than one scroll later.
///
/// A viewport of zero means "not measured yet" — nothing has been laid out, so nothing is known to
/// lie beyond anything, and the honest answer is neither edge.
pub(crate) fn strip_overflow(offset: f32, viewport: f32, tabs: usize) -> Beyond {
    if viewport <= 0.0 {
        return Beyond {
            leading: false,
            trailing: false,
        };
    }
    overflowing(
        offset,
        viewport,
        leading_slack(viewport, tabs) + natural_strip_width(tabs),
    )
}

/// The distance between one tab's leading edge and the next's.
fn tab_pitch() -> f32 {
    TAB_WIDTH + spacing::SM
}

/// Where to scroll a viewport `viewport` wide, currently at `offset`, to bring the tab at `index`
/// fully into view — or `None` if it already is (FR-002d).
///
/// `None` rather than "scroll to where you already are" is the requirement, not an optimisation.
/// FR-002d lets a user scroll away from the marked tab by hand, and a reveal that fires on every
/// selection would yank them back each time — including on selections made from the keyboard
/// rather than with the strip, which are the ones they were not looking at the strip for.
///
/// It scrolls the tab **just** into view rather than centring it: the tabs either side are context,
/// and a strip that recentres on every press moves more than the selection did.
pub fn scroll_into_view(index: usize, offset: f32, viewport: f32) -> Option<f32> {
    let pitch = tab_pitch();
    let leading = index as f32 * pitch;
    let trailing = leading + TAB_WIDTH;
    if leading + EDGE_TOLERANCE < offset {
        Some(leading)
    } else if trailing > offset + viewport + EDGE_TOLERANCE {
        Some(trailing - viewport)
    } else {
        None
    }
}

/// The emphasis a tab's leading slot draws, or `None` when there is nothing to say (FR-012c,
/// FR-012d).
///
/// A thin call into [`process_stopped`] rather than a lifecycle match of its own, and that is the
/// whole point: FR-012d asks the mark and the menu to agree, and one predicate behind both is what
/// makes it true by construction. A second match here would look right for exactly as long as
/// nobody added a lifecycle variant.
///
/// `None` is drawn as an **empty slot of the same width**, never as an absent child — the slot is
/// reserved by `material::tab`'s own `leading_slot`, because a child that comes and goes inside a
/// pressable control shifts every sibling after it and iced's positional `Tree::diff_children`
/// then drops the press (feature 023 FR-008a, research R4).
pub(crate) fn stopped_mark(
    state: &State,
    id: SessionId,
    tab: StripTab,
) -> Option<material::BadgeEmphasis> {
    process_stopped(state, id, tab).then_some(material::BadgeEmphasis::Stopped)
}

/// The marked tab's position **within the scrolling region**, or `None` when the marked tab is the
/// pinned AI one (FR-002b, FR-002d).
///
/// `None` is not a failure and not "nothing is marked" — FR-005 makes that unrepresentable. It is
/// the AI tab, which sits outside the viewport and therefore cannot be scrolled into it. There is
/// nothing to do, which is exactly the right answer.
pub fn marked_tab_index(state: &State) -> Option<usize> {
    let id = state.active_session?;
    let StripTab::Instance(instance) = marked_tab(state, id) else {
        return None;
    };
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)?
        .shells
        .iter()
        .position(|s| s.id == instance)
}

/// The session's tab strip: one tab per open Regular Terminal instance, in creation order, then the
/// session's AI CLI process (feature 026 FR-001, FR-002).
///
/// It was `instance_switcher_row` and it returned `None` below two instances, which is what feature
/// 012's FR-005 asked for — "pixel-identical to the pre-feature-011 single-instance experience".
/// **FR-003 supersedes that.** The strip is drawn whenever a session is displayed, including at zero
/// and one instance, because there is always something in it now: the AI tab. A user who never opens
/// a second terminal is the one this changes, and they are most users.
///
/// It returns an `Element` rather than an `Option` for two reasons that are the same reason. A bar
/// child that comes and goes shifts every sibling after it, and iced's positional tree diff then
/// drops a pressed sibling's press (feature 023 FR-008a). And a session that cannot be found still
/// has an answer — a strip holding just the AI tab — which is the same totality [`marked_tab`] has
/// and for the same reason: FR-005 says never zero tabs, and an `Option` here is a way to return
/// zero.
///
/// # What this function decides, and what it no longer does
///
/// A tab used to be assembled here — a button around a column around a row of three slots, with the
/// indicator rule, the fixed width and the label's bounds all written at the call site. Feature 026
/// promoted that into [`material::Tab`] and [`material::TabStrip`] (FR-013), so what is left is the
/// half that needs a session to answer: which instances there are, which of them is marked, what
/// each tab's controls dispatch, and what colour the controls nested inside a tab take.
///
/// The promotion changed no geometry. It was verified by regenerating nothing — a byte-identical
/// `tests/fixtures/layout_snapshot.txt` is the proof that moving the assembly moved no rectangle.
///
/// # Why the nested close control is tinted explicitly (BUG-001, FR-011a)
///
/// `IconButton::new` defaults its glyph to the roles' `on_surface`. That was wrong the moment the
/// button was nested inside something painting its own fill, and it stayed wrong when the fill went
/// away: without a container the accent is the *only* thing separating the active tab from its
/// neighbours, so a close glyph left on `on_surface` reads as belonging to a different tab than the
/// label beside it. Both take [`content_colour`] for this tab's state, which is the same rule the
/// label follows. `tests/icon_roles.rs` holds the contrast arithmetic; `tests/terminal_tabs.rs`
/// holds the call site.
/// How many members the scrolling part of the strip has — one per open Regular Terminal instance.
///
/// The pinned AI tab is deliberately not counted: it lives outside the scrolling viewport (feature
/// 026 FR-002b), so it is not part of what has to be pushed rightward inside it.
fn strip_tab_count(state: &State, id: SessionId) -> usize {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.shells.len())
        .unwrap_or(0)
}

/// What a strip of `tabs` members measures, derived from the tab's own width and the strip's own
/// spacing (feature 027 FR-001).
///
/// # Why this is derived rather than measured
///
/// `State::tab_strip_content_width` is the same number, reported by the scrollable from the content
/// it actually laid out — and it cannot be used here, because [`leading_slack`]'s result becomes
/// part of that content. Feeding the measurement back in would make the space a function of itself:
/// a strip with room to spare would compute a slack, grow its content by exactly that slack,
/// measure no slack on the next frame, collapse to the left, and flip back — oscillating between
/// flush-right and flush-left for as long as it was on screen. The tab count is an input the strip
/// does not influence, so deriving from it terminates.
pub fn natural_strip_width(tabs: usize) -> f32 {
    match tabs {
        0 => 0.0,
        n => n as f32 * TAB_WIDTH + (n - 1) as f32 * spacing::SM,
    }
}

/// The empty width the strip leaves at its **leading** edge so its tabs finish against the "+" and
/// the AI tab at the bar's trailing end (feature 027 FR-001).
///
/// Zero once the tabs no longer fit: there is no slack to distribute then, and the strip scrolls
/// instead (feature 026 FR-002a). That is also what keeps the edge fades honest — the fade asks
/// whether content exceeds viewport, and a slack that padded an already-full strip would report an
/// overflow that is not there.
pub fn leading_slack(viewport: f32, tabs: usize) -> f32 {
    (viewport - natural_strip_width(tabs)).max(0.0)
}

/// The scrolling strip, pushed to the trailing edge of its own viewport (FR-001).
///
/// The push is a fixed width rather than an alignment because the strip is inside a horizontal
/// `Scrollable`, which lays its content out against an unbounded width limit — a `Fill` there
/// resolves to infinity, not to "whatever is left". A width computed from the tab count is finite
/// by construction and is a pure function two tests can read (`leading_slack`).
///
/// It is spent as this container's **padding**, not as a `Space` beside the strip. A
/// `Space::new().width(Fixed(0.0))` is void, and iced drops a void child from the tree entirely —
/// so the strip would be the wrapper's child 1 while there was slack and its child 0 once the tabs
/// filled the bar. That is a child list varying with state, which feature 023 FR-008a forbids for
/// the reason recorded on the bar itself: `Tree::diff_children` is positional, so the shift hands a
/// pressed tab its neighbour's node and the press is dropped. The same defect that made the
/// sidebar's `Fixed(0)` indent spacer disappear (feature 019 §7.2), one control along. A container
/// with zero padding is still a container.
fn right_aligned_tabs<'a>(
    state: &'a State,
    id: SessionId,
    r: tokens::Roles,
) -> Element<'a, Message> {
    let slack = leading_slack(
        state.tab_strip_viewport_width as f32,
        strip_tab_count(state, id),
    );
    container(tab_strip_row(state, id, r))
        .padding(Padding::ZERO.left(slack))
        .into()
}

fn tab_strip_row<'a>(state: &'a State, id: SessionId, r: tokens::Roles) -> Element<'a, Message> {
    let marked = marked_tab(state, id);
    let shells = state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.shells.as_slice())
        .unwrap_or_default();
    let mut tabs = Vec::with_capacity(shells.len() + 1);
    for instance in shells {
        let is_marked = marked == StripTab::Instance(instance.id);
        // Every nested control takes the colour this tab draws its own label in (FR-011a). That
        // matters more without a container than it did with one: the accent is the only thing
        // separating the marked tab from its neighbours, so a close glyph left on `on_surface`
        // would read as belonging to a different tab than the label beside it.
        let tint = content_colour(is_marked, r);
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
        tabs.push(
            // Labelled by the instance's `ShellInstanceId` — the only display identity an instance
            // has until renaming arrives. The tab is what centres it and what bounds it.
            Tab::new(
                Text::new(instance.id.0.to_string(), TypeRole::Label, r).tint(tint),
                r,
            )
            .active(is_marked)
            // The mark goes in the **leading spacer every tab already reserves** (FR-012c). That
            // space exists only to balance the trailing close control and is empty today, so no tab
            // grows and the derived width is untouched. Passed unconditionally with its emphasis
            // carrying the state, never pushed-or-not (research R4).
            .leading(material::ActivityBadge::for_emphasis(
                stopped_mark(state, id, StripTab::Instance(instance.id)),
                r,
            ))
            .trailing(close)
            .on_press(Message::ShellInstanceSelected(id, instance.id))
            // A secondary press opens this tab's menu; a primary press still selects the instance,
            // because the wrapper lets the child answer first and intercepts only the right button.
            .on_secondary_press(move |(x, y)| {
                Message::StripTabMenuRequested(StripTab::Instance(instance.id), x, y)
            }),
        );
    }
    // The strip's own edge is the default `Top`, not Material's `Bottom`: this bar is anchored to
    // the window's bottom, so the pane a tab selects is *above* it and a bottom indicator would
    // point away from what it marks (feature 012 FR-004b).
    TabStrip::new(tabs, r).into()
}

/// The AI tab, which sits **outside** the scrolling region (feature 026 FR-002b).
///
/// # Why it is a strip of one rather than a bare tab
///
/// It is a member of the strip that happens to be pinned, not a control beside it — FR-010 asks it
/// to be visually consistent with the tabs it sits next to, and building it the same way is how
/// that stops being an intention. It also keeps `gates/tab_children_fit.rs` covering it: that gate
/// finds tabs as the immediate children of an anchored strip, and a lone `Tab` in the bar would
/// have been a tab no gate recognised.
///
/// # Why it is outside the viewport
///
/// FR-002 says the AI tab holds the strip's right-hand end, and FR-002b is what that means under
/// overflow: inside the scrolling region it would be reachable only by scrolling to the far end,
/// which is exactly the state SC-002's "one press" forbids. FR-002 is only a meaningful requirement
/// where there is more than fits.
///
/// # Its trailing slot is reserved and empty (FR-004, FR-010a)
///
/// A session has exactly one AI CLI process and terminating it is not an action offered from this
/// control, so there is no close control to draw. Leaving the slot reserved rather than reclaiming
/// it is what keeps the glyph on the tab's own midline, since the leading slot is the same width.
///
/// # Why it is wider than the tabs beside it
///
/// It is the one tab that does not take the strip's uniform `TAB_WIDTH` (`Tab::content_sized`), and
/// that is a departure from feature 012's BUG-001 — *a strip whose tabs are not all one size reads
/// as a control among controls rather than as a strip* — so it is worth saying why it does not
/// reopen it. BUG-001 is about **members of one strip**, and this is a strip of one: the tabs it
/// sits beside are in a different strip, inside the scrolling viewport, and nothing about a bar
/// whose pinned tab is wider than its scrolling ones reads as a row of loose controls. What forced
/// it is FR-016a — this tab's label is a *word*, `claude` or `copilot`, where every other tab's is
/// an ordinal, and `TAB_WIDTH` reserves 16dp for a label. A fixed tab settles that shortfall by
/// competing its own reserved slot down to 12dp, silently, which is BUG-005's shape rather than
/// BUG-001's and is what `gates/tab_children_fit.rs` catches.
///
/// # Not pressable yet
///
/// User Story 1's claim is that the strip *says* what the pane is showing. User Story 2 makes it a
/// control (T041), and that needs a message which **sets** the mode rather than toggling it: a
/// flipping message would switch away from the AI pane when this tab is pressed while it is already
/// displayed, which FR-007 forbids. That is the whole reason feature 027 could delete the toggle
/// without replacing it — the tab was never the toggle in a different shape.
fn pinned_ai_tab<'a>(state: &'a State, id: SessionId, r: tokens::Roles) -> Element<'a, Message> {
    let marked = marked_tab(state, id) == StripTab::Ai;
    let tint = content_colour(marked, r);
    TabStrip::new(
        vec![Tab::new(
            // The glyph this mode has worn since the toggle carried it, so the tab that replaced
            // the toggle wears the same mark (feature 026-ai-session-tab FR-009) — and beside it
            // the session's CLI by its **command** name, `claude` or `copilot`, the same register
            // the sidebar row uses (026-multi-provider-sessions FR-016/FR-016a).
            //
            // That clarification asked for the name beside "that control's existing icon" at the
            // bar's bottom-right, which was the mode toggle when it was written; feature 027
            // deleted the toggle and this tab took the corner, so the name comes here. Nothing is
            // *added* to the bar by doing so — 027 FR-001's "no control MUST replace it" forbids a
            // second switcher, and this is the tab 027 made the only one, now saying which
            // assistant it goes to rather than only that it goes to one.
            //
            // `IconLabel` rather than a `row![Glyph, Text]` of this module's own, which
            // `tests/composite_call_sites.rs` forbids outright: it is the type that keeps the glyph
            // at the *label's* role, so the two sit on one baseline and the size follows the role
            // rather than a number named here (`tests/material_boundary.rs`).
            material::IconLabel::new(
                Icon::AiCli,
                session_provider(state, id).provider().command(),
                TypeRole::Label,
                r,
            )
            .tint(tint)
            .label_tint(tint),
            r,
        )
        .active(marked)
        // Sized by what it holds, not by the strip's uniform `TAB_WIDTH` — the one tab in the
        // application whose label is a word rather than an ordinal, and the one with no neighbours
        // to be uniform with. See `material::Tab::content_sized`; without it the CLI's name
        // competes the reserved trailing slot down to 12dp, which is `gates/tab_children_fit.rs`'s
        // subject and feature 012's BUG-005 all over again.
        .content_sized()
        // The same mark in the same slot as a terminal tab's (FR-012, FR-010). "In the same place"
        // is the requirement, not an aesthetic: FR-010's "consistent with the tabs it sits beside"
        // is false in the one state that matters if this tab reports its lifecycle differently from
        // its neighbours.
        .leading(material::ActivityBadge::for_emphasis(
            stopped_mark(state, id, StripTab::Ai),
            r,
        ))
        // FR-006: a primary press shows the AI CLI and does nothing else. It **sets** the mode
        // rather than toggling it, which is FR-007 — pressing this tab while the AI CLI is already
        // displayed is a no-op with no visible change.
        .on_press(Message::TerminalAiCliSelected(id))
        // FR-006a: the same menu a terminal tab offers, minus Close. The wrapper lets the child
        // answer first and intercepts only the right button, so the primary press above keeps
        // working through it — the property feature 012 established and this reuses rather than
        // re-establishes. FR-006b is what makes the press silent while the process is running: the
        // menu would be empty, so none opens.
        .on_secondary_press(move |(x, y)| Message::StripTabMenuRequested(StripTab::Ai, x, y))],
        r,
    )
    .into()
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
        let mut session = Session::start_new(
            SessionLocation::Default,
            micold_core::session::AiCli::ClaudeCode,
        );
        session.lifecycle = lifecycle;
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.active_session = Some(id);
        (state, id)
    }

    /// A state with one project and one session in `mode`, holding `shells` instances at the given
    /// lifecycles and marking `active` (an index into `shells`) as the displayed one.
    fn state_with_instances(
        mode: TerminalMode,
        lifecycle: SessionLifecycle,
        shells: &[ShellLifecycle],
        active: Option<usize>,
    ) -> (State, SessionId) {
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state.workspace.active = Some(path.clone());
        let mut session = Session::start_new(
            SessionLocation::Default,
            micold_core::session::AiCli::ClaudeCode,
        );
        session.lifecycle = lifecycle;
        session.mode = mode;
        let mut opened = Vec::new();
        for want in shells {
            let id = session.open_shell_instance();
            // Written rather than driven through the transitions: `open_shell_instance` leaves an
            // instance `Starting`, so `NotStarted` — a state a *restored* session's instances are
            // in, and one of the two FR-012d names — is not reachable by any public transition.
            // A table-driven test has to be able to state its inputs.
            if let Some(instance) = session.shells.iter_mut().find(|s| s.id == id) {
                instance.lifecycle = *want;
            }
            opened.push(id);
        }
        session.active_shell = active.map(|i| opened[i]);
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.active_session = Some(id);
        (state, id)
    }

    /// FR-016a (T058a): the pinned AI tab names the session's own CLI, by its **command** name.
    ///
    /// Asserted through `session_provider`, which is the only input the tab's label has — the label
    /// is `session_provider(..).provider().command()` and nothing else, so a tab naming the wrong
    /// CLI can only be this function answering wrongly. `tests/terminal_bar_stability.rs` holds the
    /// other half, that the label is still built from `command()` rather than `display_name()`;
    /// between them the two registers cannot swap without something going red.
    ///
    /// The fallback is asserted too, and it is not a formality: `pane` draws the bar for whatever
    /// session id is active, and the projection it looks that id up in is refreshed by the daemon.
    /// A frame in which the two disagree must render, and rendering `claude` there is the same
    /// answer a session with nothing recorded resumes on (FR-003, FR-013).
    #[test]
    fn the_bar_reads_the_session_own_cli_by_command_name() {
        for cli in AiCli::ALL {
            let (mut state, id) = state_showing(SessionLifecycle::Running);
            for session in state.workspace.sessions.values_mut() {
                for s in session.iter_mut() {
                    s.provider = cli;
                }
            }
            assert_eq!(
                session_provider(&state, id),
                cli,
                "the bar read a different CLI from the one the session records"
            );
        }

        assert_eq!(
            session_provider(&State::default(), SessionId::new()),
            AiCli::default(),
            "a bar drawn for a session the projection does not list must still render, on the \
             same default a session with nothing recorded resumes on"
        );
    }

    /// FR-012/FR-012d/FR-012e: a tab wears the mark for exactly the states the predicate calls
    /// stopped, for both kinds of member and never for an in-progress one.
    ///
    /// Asserted **through the predicate**, deliberately. The mark could be given its own lifecycle
    /// match and would look right for a while; FR-012d asks the mark and the menu to agree, and the
    /// only way that survives a variant being added is for both to be readings of one function.
    /// This test fails if the mark ever grows a second opinion.
    #[test]
    fn a_tab_wears_the_mark_for_exactly_what_the_predicate_calls_stopped() {
        for (mode, lifecycle, shell) in [
            (
                TerminalMode::AiCli,
                SessionLifecycle::Idle,
                ShellLifecycle::Running,
            ),
            (
                TerminalMode::AiCli,
                SessionLifecycle::Failed,
                ShellLifecycle::Exited,
            ),
            (
                TerminalMode::AiCli,
                SessionLifecycle::InterruptedResumable,
                ShellLifecycle::Starting,
            ),
            (
                TerminalMode::AiCli,
                SessionLifecycle::Running,
                ShellLifecycle::NotStarted,
            ),
            (
                TerminalMode::AiCli,
                SessionLifecycle::Starting,
                ShellLifecycle::Running,
            ),
            (
                TerminalMode::Regular,
                SessionLifecycle::Restarting { attempts: 1 },
                ShellLifecycle::Exited,
            ),
        ] {
            let (state, id) = state_with_instances(mode, lifecycle, &[shell], Some(0));
            let instance = state.active_sessions()[0].shells[0].id;
            for tab in [StripTab::Ai, StripTab::Instance(instance)] {
                assert_eq!(
                    stopped_mark(&state, id, tab).is_some(),
                    process_stopped(&state, id, tab),
                    "{mode:?}/{lifecycle:?}/{shell:?} {tab:?}: the mark and the predicate disagree. \
                     A mark on a state nobody can act on sends a user to a press that does nothing \
                     (FR-006b), which is the dead end the mark exists to prevent."
                );
            }
        }
    }

    /// FR-012a: a tab can be marked-active **and** stopped, and the two cues must not read as one.
    ///
    /// Colour identity, not geometry — where the mark lands is `tab_children_fit`'s question and
    /// the composited result is §8's, but "these two are not the same role" is assertable here and
    /// is the requirement a tone-only cue would have failed. It has to be legible against both the
    /// accent an active tab wears and the muted tint an inactive one does, in both schemes, which
    /// is exactly what a third grey is worst at.
    #[test]
    fn the_mark_is_not_the_indicators_role_in_either_scheme() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let accent = crate::ui::material::tab::indicator_colour(true, r)
                .expect("an active tab has an indicator");
            let muted = crate::ui::material::tab::content_colour(false, r);
            assert_ne!(
                r.error, accent,
                "{scheme:?}: the stopped mark must not be drawn in the indicator's own accent — a \
                 tab that is active *and* stopped would then wear one colour saying two things \
                 (FR-012a)"
            );
            assert_ne!(
                r.error, muted,
                "{scheme:?}: nor in the muted tint an inactive tab wears, which is the other half \
                 of the same requirement"
            );
        }
    }

    /// FR-004/FR-006a/FR-006b: the AI tab's menu is a terminal tab's **minus Close**, in the same
    /// order — and it is empty whenever the AI process is running, so no menu opens at all.
    ///
    /// Stated as "the terminal tab's menu minus Close" rather than as a list, because that is how
    /// FR-006a is worded and for the reason it is worded that way: the two must not be able to
    /// drift into offering different actions. A test that spelled out `["Restart"]` would pass
    /// while the terminal tab's menu grew an item the AI tab never got.
    #[test]
    fn the_ai_tabs_menu_is_a_terminal_tabs_minus_close() {
        for (lifecycle, shell) in [
            (SessionLifecycle::Idle, ShellLifecycle::Exited),
            (SessionLifecycle::Failed, ShellLifecycle::NotStarted),
            (
                SessionLifecycle::InterruptedResumable,
                ShellLifecycle::Exited,
            ),
            (SessionLifecycle::Running, ShellLifecycle::Running),
            (SessionLifecycle::Starting, ShellLifecycle::Starting),
            (
                SessionLifecycle::Restarting { attempts: 2 },
                ShellLifecycle::Starting,
            ),
        ] {
            let (state, id) =
                state_with_instances(TerminalMode::Regular, lifecycle, &[shell], Some(0));
            let instance = state.active_sessions()[0].shells[0].id;

            let for_instance =
                crate::ui::strip_tab_menu_labels(&state, id, StripTab::Instance(instance));
            let for_ai = crate::ui::strip_tab_menu_labels(&state, id, StripTab::Ai);

            let expected: Vec<&str> = for_instance
                .iter()
                .copied()
                .filter(|label| *label != "Close")
                .collect();
            assert_eq!(
                for_ai, expected,
                "{lifecycle:?}: the AI tab's menu must be the terminal tab's with Close filtered \
                 out, in the same order — FR-004 excludes Close by any press, and FR-006a is \
                 worded to stop the two menus drifting apart"
            );
        }
    }

    /// FR-006b: a menu with no items does not open, and that is most of the time.
    ///
    /// With restart the only item and Close excluded, the AI tab's menu is empty whenever the AI
    /// CLI is running — which is the ordinary state. An empty panel is a defect everywhere else in
    /// this application, and a panel whose entire content is inert is one too, so the offer is
    /// absent rather than present-and-useless. It also keeps the strip agreeing with the bar beside
    /// it, which already shows a restart control only for a process that is not running.
    #[test]
    fn a_running_ai_tab_opens_no_menu_at_all() {
        for lifecycle in [
            SessionLifecycle::Running,
            SessionLifecycle::Starting,
            SessionLifecycle::Restarting { attempts: 1 },
        ] {
            let (state, id) = state_with_instances(TerminalMode::AiCli, lifecycle, &[], None);
            assert!(
                crate::ui::strip_tab_menu_labels(&state, id, StripTab::Ai).is_empty(),
                "{lifecycle:?}: a secondary press on a running AI tab must do nothing — an empty \
                 panel says the offer exists and then withholds it"
            );
        }
        for lifecycle in [
            SessionLifecycle::Idle,
            SessionLifecycle::Failed,
            SessionLifecycle::InterruptedResumable,
        ] {
            let (state, id) = state_with_instances(TerminalMode::AiCli, lifecycle, &[], None);
            assert_eq!(
                crate::ui::strip_tab_menu_labels(&state, id, StripTab::Ai),
                vec!["Restart"],
                "{lifecycle:?}: a stopped AI process has exactly one thing that can be done to it, \
                 and the mark on its tab (FR-012d) is what points at it"
            );
        }
    }

    /// FR-002e, research R6: "content lies beyond this edge" is a pure function of the viewport
    /// offset and the content width.
    ///
    /// The *fade* is appearance and no gate can see it — a gradient occupies the same box whether
    /// it is opaque or invisible, which is what the visual pass is for. The **fact behind it** is
    /// arithmetic, and this is where it is held. Four cases, because a strip can be scrolled to
    /// either end, to the middle, or not be scrollable at all, and the last one is the case a naive
    /// `offset > 0` would get wrong in the direction that matters: a fade on a strip with nothing
    /// beyond it tells the user to scroll toward nothing.
    #[test]
    fn an_edge_knows_whether_anything_lies_beyond_it() {
        // Written as literals rather than as named constants on `Beyond` itself. The four names
        // would have to be `#[cfg(test)]` — nothing in the drawing path wants a whole value, since
        // each edge is drawn on its own side and asking "is this exactly BOTH" would couple the two
        // together — and a second `#[cfg(test)]` block in this file breaks
        // `tests/anatomy_call_sites.rs`, which truncates a source at the first one.
        let beyond = |leading, trailing| Beyond { leading, trailing };

        // Nothing to scroll: the content fits, so neither edge says anything.
        assert_eq!(overflowing(0.0, 500.0, 300.0), beyond(false, false));
        // Scrolled hard to the leading end, with more to come.
        assert_eq!(overflowing(0.0, 300.0, 900.0), beyond(false, true));
        // Somewhere in the middle: both.
        assert_eq!(overflowing(300.0, 300.0, 900.0), beyond(true, true));
        // Scrolled hard to the trailing end.
        assert_eq!(overflowing(600.0, 300.0, 900.0), beyond(true, false));
        // Exactly at the trailing end, to the pixel — not "a little more that way".
        assert!(
            !overflowing(600.0, 300.0, 900.0).trailing,
            "an edge with nothing past it must not be faded; a cue that points at nothing is \
             worse than none, because it is the same cue that means something elsewhere"
        );
    }

    /// FR-002e + feature 027 FR-003: the fade asks the tab count, not the last measurement.
    ///
    /// The case this exists for is the one the visual pass found: a strip that fits draws no fade,
    /// on the very first frame, before anything has scrolled. `overflowing` above is still right
    /// about the arithmetic; what was wrong was the numbers it was handed.
    #[test]
    fn a_strip_that_fits_fades_neither_edge_before_anything_has_scrolled() {
        let beyond = |leading, trailing| Beyond { leading, trailing };
        let pitch = TAB_WIDTH + spacing::SM;

        // Never laid out: nothing is known, so nothing is claimed.
        assert_eq!(strip_overflow(0.0, 0.0, 1), beyond(false, false));
        // One tab in a bar with room for five — the state that drew a fade over its own tab.
        assert_eq!(strip_overflow(0.0, 5.0 * pitch, 1), beyond(false, false));
        // Room for exactly as many as there are.
        assert_eq!(
            strip_overflow(0.0, 3.0 * TAB_WIDTH + 2.0 * spacing::SM, 3),
            beyond(false, false)
        );
        // More tabs than room, unscrolled: there is something that way, and only that way.
        assert_eq!(strip_overflow(0.0, 3.0 * pitch, 6), beyond(false, true));
        // Scrolled off the leading end with more still to come.
        assert_eq!(strip_overflow(pitch, 3.0 * pitch, 6), beyond(true, true));
    }

    /// FR-002d: changing the marked tab yields a scroll request for **that** tab, and none at all
    /// when it is already fully visible.
    ///
    /// The second half is the one worth a test. A reveal that did not need to move the strip must
    /// not move it — a user who has scrolled by hand (which FR-002d explicitly allows) would
    /// otherwise be yanked back on every selection, including selections they made from the
    /// keyboard rather than with the strip.
    #[test]
    fn the_marked_tab_is_scrolled_into_view_only_when_it_is_not() {
        let pitch = TAB_WIDTH + spacing::SM;

        // Already fully visible at the leading end: nothing to do.
        assert_eq!(scroll_into_view(0, 0.0, 3.0 * pitch), None);
        assert_eq!(scroll_into_view(1, 0.0, 3.0 * pitch), None);

        // Beyond the trailing edge: scroll just far enough that its **trailing edge** lands on the
        // viewport's, not far enough to centre it — the neighbouring tabs are context, and a strip
        // that recentres on every press moves more than the selection did.
        //
        // The figure is `5·pitch + TAB_WIDTH - viewport`, not `6·pitch - viewport`: a pitch is a
        // tab plus the gap that follows it, and there is no gap after the last tab shown. The 8dp
        // between them is a gap this would have opened at the viewport's trailing edge.
        assert_eq!(
            scroll_into_view(5, 0.0, 3.0 * pitch),
            Some(5.0 * pitch + TAB_WIDTH - 3.0 * pitch),
        );

        // Beyond the leading edge: scroll to put its leading edge on the viewport's.
        assert_eq!(scroll_into_view(0, 4.0 * pitch, 3.0 * pitch), Some(0.0));
        assert_eq!(
            scroll_into_view(2, 4.0 * pitch, 3.0 * pitch),
            Some(2.0 * pitch)
        );
    }

    /// Feature 027 FR-003: the strip sits against the **trailing** edge of its viewport, and the
    /// slack that puts it there is a pure function of the tab count.
    ///
    /// Derived from the count, never from the measured content width. The slack is laid out
    /// *inside* the scrolling content, so it is part of what `State::tab_strip_content_width`
    /// reports back — computing it from that measurement would feed its own output back in and the
    /// strip would swing flush-right, flush-left, flush-right on successive frames. From the count
    /// it terminates by construction, which is why this is arithmetic a test can read rather than a
    /// `Length::Fill`: `Fill` inside a horizontal `Scrollable` resolves against an unbounded width
    /// limit — infinity, not "whatever is left" — so it pushes the strip out of the viewport
    /// entirely.
    #[test]
    fn the_strip_hugs_the_trailing_edge_of_its_viewport() {
        let pitch = TAB_WIDTH + spacing::SM;

        // No tabs, no strip: nothing to push, and a viewport's worth of slack would be a scrollable
        // region made entirely of emptiness.
        assert_eq!(natural_strip_width(0), 0.0);
        assert_eq!(leading_slack(600.0, 0), 600.0);

        // One tab is a tab, not a tab plus a trailing gap — n tabs carry n-1 gaps between them.
        assert_eq!(natural_strip_width(1), TAB_WIDTH);
        assert_eq!(natural_strip_width(3), 3.0 * TAB_WIDTH + 2.0 * spacing::SM);

        // Room to spare: the slack is exactly what is left, so the strip's trailing edge lands on
        // the viewport's however few tabs there are.
        assert_eq!(leading_slack(3.0 * pitch, 1), 3.0 * pitch - TAB_WIDTH);
        assert_eq!(
            leading_slack(3.0 * pitch, 2),
            3.0 * pitch - (2.0 * TAB_WIDTH + spacing::SM)
        );

        // Overflowing: there is no slack to give, and the answer is zero rather than negative. A
        // negative width would be a panic in iced; worse, a slack that shrank as the strip grew
        // would let the strip keep scrolling past its own leading tab.
        assert_eq!(leading_slack(3.0 * pitch, 6), 0.0);
        assert_eq!(leading_slack(0.0, 3), 0.0);
    }

    /// FR-010a: the AI tab measures what a terminal tab measures, and its two slots are equal.
    ///
    /// The width half is what keeps the strip reading as a strip — a strip whose tabs are not all
    /// one size reads as a control among controls, which is the defect feature 012's BUG-001 was
    /// filed for. The slots half is what puts the icon on the tab's own midline: having no close
    /// control (FR-004) must not make the AI tab narrower **or** push its content off centre, so
    /// the trailing slot is left empty rather than reclaimed.
    ///
    /// It is asserted of the *component's* anatomy rather than of a rendered tab, because that is
    /// where the answer is: both tabs are the same component, so "the same width" is not something
    /// the call site can get wrong — what it *can* get wrong is the slots, and a tab that took one
    /// end from the constant and the other from its content's own size is what pulled a stopped
    /// tab's label 20dp off centre in T013's visual pass.
    #[test]
    fn the_ai_tab_measures_what_a_terminal_tab_measures() {
        use crate::ui::material::tab;

        assert_eq!(
            2.0 * tab::SLOT_WIDTH + 2.0 * spacing::XS + tab::LABEL_MIN_WIDTH,
            tab::WIDTH,
            "a tab's width is the sum of what it holds (feature 012 FR-004c), and both slots are \
             a term in it — so a slot that measured its content rather than the slot would make \
             the derivation false without changing the constant. No padding term: a tab draws no \
             inset, because the indicator and the state layer both span the whole tab and an inset \
             makes them stop short of the thing they mark."
        );
    }

    /// FR-005: exactly one tab is marked, for **every** combination of mode and active instance.
    ///
    /// "Never zero, never two" is a claim about **totality**, so this is where it is proved: the
    /// function returns one `StripTab` and there is nowhere else for the answer to go. The case
    /// that does the work is the last one — a `Regular` session whose `active_shell` is `None`,
    /// which is what an `Option<ShellInstanceId>` would have had to answer twice for (Principle V).
    #[test]
    fn exactly_one_tab_is_marked_in_every_state() {
        // AI CLI mode: the AI tab is marked whatever the instances are doing.
        for active in [None, Some(0), Some(1)] {
            let (state, id) = state_with_instances(
                TerminalMode::AiCli,
                SessionLifecycle::Running,
                &[ShellLifecycle::Running, ShellLifecycle::Running],
                active,
            );
            assert_eq!(
                marked_tab(&state, id),
                StripTab::Ai,
                "in AiCli mode the AI tab is the displayed one regardless of active_shell \
                 ({active:?})"
            );
        }

        // Regular mode with an instance selected: that instance's tab, and not the AI tab.
        let (state, id) = state_with_instances(
            TerminalMode::Regular,
            SessionLifecycle::Running,
            &[ShellLifecycle::Running, ShellLifecycle::Running],
            Some(1),
        );
        let selected = state.active_sessions()[0].shells[1].id;
        assert_eq!(marked_tab(&state, id), StripTab::Instance(selected));

        // Regular mode with **no** instance selected. `None` already means "this session has no
        // active instance" in this file, so overloading it to also mean "the AI tab" would make
        // this case unanswerable. The AI tab is what the pane shows, so the AI tab is marked.
        let (state, id) =
            state_with_instances(TerminalMode::Regular, SessionLifecycle::Running, &[], None);
        assert_eq!(
            marked_tab(&state, id),
            StripTab::Ai,
            "a Regular session with nothing to show falls back to the AI tab — never to no tab \
             at all, which is the state FR-005 exists to forbid"
        );

        // A session that is not in the workspace at all still answers, because the return type
        // leaves no room not to.
        let (state, _) =
            state_with_instances(TerminalMode::Regular, SessionLifecycle::Running, &[], None);
        assert_eq!(marked_tab(&state, SessionId::new()), StripTab::Ai);
    }

    /// FR-012d / FR-012e, research R1 and R2: **one** predicate answers "this process is stopped"
    /// for both lifecycle vocabularies.
    ///
    /// Asserted for every variant of both enums **by name**, so a variant added later fails here
    /// rather than silently defaulting into one answer or the other. The rule is the same in both
    /// rows and it is FR-012d's own: the mark appears exactly where a restart can act.
    #[test]
    fn one_predicate_calls_a_process_stopped_in_both_vocabularies() {
        let (state, id) = state_with_instances(
            TerminalMode::AiCli,
            SessionLifecycle::Running,
            &[
                ShellLifecycle::NotStarted,
                ShellLifecycle::Starting,
                ShellLifecycle::Running,
                ShellLifecycle::Exited,
            ],
            None,
        );
        let shells: Vec<_> = state.active_sessions()[0]
            .shells
            .iter()
            .map(|s| s.id)
            .collect();
        for (i, (want, why)) in [
            (true, "a shell that was never started is restartable"),
            (
                false,
                "a starting shell is in progress, not stopped (FR-012e)",
            ),
            (false, "a running shell has nothing to restart"),
            (true, "an exited shell is the case FR-012 exists for"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (w, why))| (i, (*w, *why)))
        {
            assert_eq!(
                process_stopped(&state, id, StripTab::Instance(shells[i])),
                want,
                "{why}"
            );
        }

        for (lifecycle, want, why) in [
            (SessionLifecycle::Idle, true, "a persisted-but-stopped session resumes on request"),
            (SessionLifecycle::Starting, false, "starting is in progress, not stopped (FR-012e)"),
            (SessionLifecycle::Running, false, "a running AI process has nothing to restart"),
            (
                SessionLifecycle::Restarting { attempts: 1 },
                false,
                "an auto-restart is already under way; a mark here would point at an action                  nobody needs to take",
            ),
            (SessionLifecycle::Failed, true, "auto-restart gave up; a manual one is the offer"),
            (
                SessionLifecycle::InterruptedResumable,
                true,
                "the service restarted and found a conversation — resuming it is the one action",
            ),
        ] {
            let (state, id) =
                state_with_instances(TerminalMode::AiCli, lifecycle, &[], None);
            assert_eq!(
                process_stopped(&state, id, StripTab::Ai),
                want,
                "{lifecycle:?}: {why}"
            );
        }
    }

    /// R2's other half: the bar's own restart control and the strip must not be able to disagree.
    ///
    /// `attached_process_restartable` is now a call into the predicate above rather than a second
    /// match statement, so this asserts the two give the same answer for the process the bar is
    /// describing. This file has paid twice for the alternative — `empty_terminal_message` and the
    /// bar disagreed "for exactly as long as they were two readings of one fact", and BUG-004 was
    /// `restart_message` re-deriving something the predicate beside it already had.
    #[test]
    fn the_bar_and_the_strip_read_one_predicate() {
        for (mode, shells, active) in [
            (TerminalMode::AiCli, &[][..], None),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Exited][..],
                Some(0),
            ),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Running][..],
                Some(0),
            ),
            (TerminalMode::Regular, &[][..], None),
        ] {
            for lifecycle in [
                SessionLifecycle::Idle,
                SessionLifecycle::Running,
                SessionLifecycle::Failed,
            ] {
                let (state, id) = state_with_instances(mode, lifecycle, shells, active);
                let attached = match mode {
                    TerminalMode::AiCli => StripTab::Ai,
                    TerminalMode::Regular => state.active_sessions()[0]
                        .active_shell
                        .map(StripTab::Instance)
                        .unwrap_or(StripTab::Ai),
                };
                assert_eq!(
                    attached_process_restartable(&state, id),
                    process_stopped(&state, id, attached),
                    "{mode:?}/{lifecycle:?}: the bar's restart control and the strip's mark are \
                     two readings of one fact and must not be able to differ"
                );
            }
        }
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

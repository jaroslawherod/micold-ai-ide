//! Terminal rendering support for the thin client: the colour palette + per-cell resolution
//! helpers the grid-cache pane renders with, and the session terminal `pane()` view.
//!
//! Sessions themselves live in `micold-daemon` — the client owns no PTY. It receives the
//! daemon's streamed grid frames into a [`crate::grid::GridCache`] and renders them via the custom
//! canvas widget ([`crate::ui::material::terminal_pane`]), resolving each cell's colour/flags with
//! the helpers here (`TermPalette`, [`cell_colors`], [`wire_cell_colors`], [`cell_font`]). It keeps
//! `alacritty_terminal` only for those VT-mode/flag/colour *types* (feature 006 rendering).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::features::structured_transcript::TranscriptMsg;
use crate::grid::GridCache;
use crate::ui::material::{
    Button, ContextMenu, GridSizeReporter, MenuItem, TerminalPane, Text, TypeRole,
};
use crate::ui::transcript::{BlockRenderers, RenderCtx};
// The strip's rules moved to the render-free `features::session_strip` (feature 1126, R7); the
// paths through this module stay, for the tab menu in `ui/mod.rs` and the integration tests.
pub(crate) use crate::features::session_strip::{attached_process_restartable, session_title};
pub use crate::features::session_strip::{
    marked_tab, marked_tab_index, process_stopped, scroll_into_view, tab_reveal_target, StripModel,
    StripTab,
};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Color, Element, Font, Length};
use micold_core::protocol::grid::{WireColor, WireStyle};
use micold_core::protocol::messages::TerminalRef;
use micold_core::session::{SessionId, SessionLifecycle, TerminalMode};
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
    hint_container: Color,
    hint_content: Color,
    ansi16: [Color; 16],
}

impl TermPalette {
    /// Build a palette whose default fg/bg follow the active light/dark scheme (FR-003), with
    /// the scheme's `secondary` as the focus-indicator accent — the 018 focus ring's role, so the
    /// focused terminal is outlined the way every other focused control is (FR-010b, BUG-005).
    ///
    /// The default pair comes from [`tokens::terminal_defaults`], the definition the daemon answers
    /// colour queries from, so the pane is painted with what programs are told (FR-003a, BUG-007).
    pub fn from_scheme(scheme: ColorScheme) -> Self {
        let defaults = tokens::terminal_defaults(scheme);
        Self {
            fg: rgb_to_color(defaults.foreground),
            bg: rgb_to_color(defaults.background),
            accent: rgb_to_color(tokens::roles(scheme).secondary),
            hint_container: rgb_to_color(tokens::roles(scheme).surface_container_highest),
            hint_content: rgb_to_color(tokens::roles(scheme).on_surface),
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

    /// The link address hint's container: a surface role, so it follows the theme (FR-009).
    pub fn hint_container(&self) -> Color {
        self.hint_container
    }

    /// The link address hint's text: the on-surface role over [`Self::hint_container`].
    pub fn hint_content(&self) -> Color {
        self.hint_content
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

/// The terminal context menu's items, in the order it shows them (feature 031, contract
/// link-opening §6).
///
/// Glue: the link items are whatever `session::link_menu_items` lists for the link the menu was
/// opened over, ahead of today's Copy and Paste and with no divider between them (M1, M2). Each
/// item is its label and the message choosing it publishes, so the view and the acceptance tests
/// read one list rather than two copies of it.
pub fn context_menu_items(state: &State) -> Vec<(&'static str, Message)> {
    use crate::features::session::LinkMenuItem;
    let mut items: Vec<(&'static str, Message)> =
        crate::features::session::link_menu_items(state.session.menu_link.as_ref())
            .into_iter()
            .map(|item| match item {
                LinkMenuItem::OpenLink => (
                    "Open Link",
                    Message::Session(SessionMsg::ContextMenuOpenLink),
                ),
                LinkMenuItem::CopyLinkAddress => (
                    "Copy Link Address",
                    Message::Session(SessionMsg::ContextMenuCopyLinkAddress),
                ),
            })
            .collect();
    items.push(("Copy", Message::Session(SessionMsg::TerminalCopyRequested)));
    items.push((
        "Paste",
        Message::Session(SessionMsg::TerminalPasteRequested),
    ));
    items
}

/// What this window's links resolve against (feature 031, FR-012, FR-018).
///
/// Glue: it reads the hostname boot recorded and the sandbox's state, and names neither a platform
/// nor a filesystem — `micold_core::link::resolve` decides everything from it (U134, U136).
pub fn link_context(
    state: &State,
    sandbox: &crate::features::sandbox::Sandbox,
) -> micold_core::link::LinkContext {
    use micold_core::sandbox::lifecycle::SandboxState;

    // A live sandbox — running, or running an image that is now out of date — is the one whose
    // sessions print container paths. Every other state means the sessions are this machine's own,
    // and `Sandbox::locations` answers `None` for all of them, so the two cannot disagree.
    let container = match &sandbox.state {
        SandboxState::Running(id) | SandboxState::Stale(id) => Some(id.0.as_str()),
        SandboxState::Disabled
        | SandboxState::Probing
        | SandboxState::Acquiring(_)
        | SandboxState::Starting
        | SandboxState::Failed(_) => None,
    };
    // The container the view's session is bound to: the bound daemon's own name (feature 491, T026),
    // the fixed single-sandbox name only for a daemon with no container entry of its own.
    let container_name = state
        .session
        .active
        .and_then(|id| state.active_sessions().iter().find(|s| s.id == id))
        .map(|s| match &s.location {
            micold_core::session::SessionLocation::Worktree(dir) => dir.as_str(),
            micold_core::session::SessionLocation::Default => "",
        })
        .and_then(|key| match state.daemon_binding(key) {
            micold_core::daemons::Binding::Bound(id) => {
                crate::links::terminal_container(&state.settings.daemons, id)
            }
            micold_core::daemons::Binding::NoDaemon => None,
        })
        .unwrap_or(micold_core::sandbox::CONTAINER_NAME);
    micold_core::link::LinkContext {
        host_names: state.session.host_names.clone(),
        windows_host: cfg!(windows),
        // `Some` exactly while the sandbox reports locations, which is exactly while a container is
        // up (U134). A container path then translates through what that container really shares, and
        // one outside every location reports as not reachable rather than opening this machine's own
        // file of the same name (C12, C16c).
        sandbox: container.zip(sandbox.locations()).map(|(id, locations)| {
            micold_core::link::SandboxLinkContext {
                host_names: micold_core::link::container_host_names(id, container_name),
                locations: locations.shared.clone(),
                denied: locations.denied.clone(),
            }
        }),
    }
}

/// Render the terminal pane for the active session (FR-012). `grid` is the active session's
/// daemon-streamed grid cache (colour-rendered); `None` renders an empty state. `selection` is the
/// active `LineId`-anchored text selection, and `display_offset` how far the view is scrolled back.
/// `link_context` is what the pane's links resolve against (feature 031, FR-012, FR-018).
pub fn pane<'a>(
    state: &'a State,
    area: Option<&crate::ui::panes::PaneArea<'a>>,
    grid: Option<&'a GridCache>,
    selection: Option<&'a crate::selection::Selection>,
    display_offset: usize,
    scheme: ColorScheme,
    link_context: micold_core::link::LinkContext,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    // Every branch below wraps the terminal area in a `GridSizeReporter`, including the ones with no
    // terminal in it (BUG-003, FR-014a). The size of this rectangle is what a session must be
    // *started* at, and the empty state occupies exactly the rectangle the first session will be
    // displayed in — so measuring it here is what lets that session be spawned at the right size
    // instead of corrected a frame after its first output.
    let Some(active) = state.session.active else {
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
    let several = area.filter(|a| a.layout.len() > 1);
    let body: Element<'a, Message> = if let Some(area) = several {
        // Feature 484: several panes tile the area; each measures itself, so no outer reporter.
        // A pane showing a structured session's primary draws its transcript (feature 939, R15).
        crate::ui::panes::view(
            state,
            area,
            selection,
            display_offset,
            scheme,
            &link_context,
        )
    } else if shows_transcript(state, active) {
        // A structured session's AI process draws no terminal and takes no input (feature 938,
        // FR-022a): its pane is the transcript (feature 939, FR-001). Its shell instances stay
        // terminals.
        transcript_body(state, active, scheme)
    } else {
        match grid {
            Some(grid) => TerminalPane::new(grid, TermPalette::from_scheme(scheme))
                .selection(selection)
                .display_offset(display_offset)
                .focused(state.terminal_focused())
                .session(active)
                .link_context(link_context)
                .into(),
            None => container(
                Text::new(empty_terminal_message(state, active), TypeRole::Caption, r).muted(),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        }
    };
    // Measured whether the pane, the placeholder, or nothing at all is inside it (see above).
    let body: Element<'a, Message> = if several.is_some() {
        body
    } else {
        GridSizeReporter::new(body).into()
    };

    // While the right-click context menu is open, float it over the terminal body anchored at the
    // clicked point; choosing Copy/Paste or clicking outside dismisses it (FR-013).
    //
    // Hosted on the terminal body rather than on the window's overlay, because `(x, y)` is
    // pane-local: the pane's origin is not known at render time, so there is nothing to translate
    // the point by. Same primitive, mounted one level down.
    let body: Element<'a, Message> = match state.session.terminal_context_menu {
        Some((x, y)) => {
            // The link items the menu was opened over come first, with no divider (feature 031,
            // contract link-opening §6 M1). One list, built by `context_menu_items`, so the
            // acceptance tests read what the user sees rather than a copy of it.
            let items = context_menu_items(state)
                .into_iter()
                .map(|(label, message)| MenuItem::labeled(label, message))
                .collect();
            crate::ui::cdk::overlay::Overlay::new(body)
                .push(
                    ContextMenu::new(
                        items,
                        (x, y),
                        Message::Session(SessionMsg::TerminalContextMenuClosed),
                        r,
                    )
                    .into(),
                )
                .into()
        }
        None => body,
    };

    // #943: above a Claude terminal session's own terminal, the held-queue line and **Open in
    // structured view**; an empty element otherwise, so the terminal keeps its place in the tree.
    // Outside the size reporter, so the terminal is sized to the room left under it. A transcript
    // has its switch in its own header.
    let body: Element<'a, Message> = if several.is_some() || shows_transcript(state, active) {
        body
    } else {
        let shown = active_primary(state, active);
        column![terminal_switch_strip(state, shown, r), body].into()
    };

    // A refusal with a single pane (closing the last one) has no pane area to sit in: say it above
    // the terminal, as the multi-pane view does.
    let body: Element<'a, Message> =
        match area.and_then(|a| a.refusal).filter(|_| several.is_none()) {
            Some(reason) => column![
                container(Text::new(reason, TypeRole::Caption, r))
                    .width(Length::Fill)
                    .padding(spacing::SM),
                body
            ]
            .into(),
            None => body,
        };

    // Feature 1126 (FR-001): nothing under the pane area. The bar that sat here — the session's
    // name and status, its tabs, "+", the pinned AI tab and the split buttons — moved into the
    // strip above the panes (`ui/session_area.rs::strip`), with every press it offered.
    container(body)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// A structured session's primary in a pane (feature 939, FR-001): the window's transcript of it,
/// drawn through [`BlockRenderers::RICH`] (feature 940), its messages routed as the session
/// feature's, with its pending permission requests in it (feature 942), over the session's
/// composer (feature 941, FR-001), which takes the height it needs and leaves the rest to the
/// transcript (FR-008).
pub(crate) fn transcript_body<'a>(
    state: &'a State,
    id: SessionId,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    // Feature 944: the window's insights of the session; a running task or subagent of a session
    // whose CLI is not running shows as ended (FR-018, FR-022).
    // #942: and its permission mode, for the Bypass label (FR-014); from whichever project lists
    // the session.
    let shown = state.workspace.find_session(id).map(|(_, s)| s);
    let cli_running = shown.is_some_and(|s| s.lifecycle == SessionLifecycle::Running);
    let mode = shown.map(|s| s.permission_mode).unwrap_or_default();
    let transcript = crate::ui::transcript::view_full(
        &state.structured_transcript,
        crate::ui::transcript::InsightsCtx {
            state: &state.structured_insights,
            cli_running,
            mode,
        },
        id,
        &BlockRenderers::RICH,
        RenderCtx {
            roles: tokens::roles(scheme),
            scheme,
            cache: &state.transcript_rich,
        },
        |msg| match msg {
            TranscriptMsg::Insights(msg) => Message::Session(SessionMsg::Insights(msg)),
            msg => Message::Session(SessionMsg::Transcript(msg)),
        },
        Some(crate::ui::transcript::PromptsView {
            state: &state.permission_prompts,
            on_prompt: Message::PermissionPrompts,
            focused: state.window.focused_field,
            on_focus: |id, focused| {
                Message::Window(crate::features::window::Msg::FieldFocusChanged(id, focused))
            },
        }),
        // #943 FR-019: **Open in terminal** in the transcript's header, from `switch_entries`.
        switch_button(state, id, tokens::roles(scheme)),
    );
    iced::widget::column![
        transcript,
        crate::ui::composer::view(
            state,
            id,
            tokens::roles(scheme),
            // #942: the mode control, and *Waiting for you* while a request waits (plan I-1);
            // #943: the command palette in the completion slot.
            crate::ui::composer::Seams {
                // #943: the offer to open the session in the terminal, else the palette, else
                // the /login offer an authentication failure leaves.
                completion: crate::ui::composer::switch_offer(state, id, tokens::roles(scheme))
                    .or_else(|| crate::ui::composer::palette(state, id, tokens::roles(scheme)))
                    .or_else(|| {
                        crate::ui::composer::login_offer(state, id, tokens::roles(scheme))
                    }),
                ..crate::ui::mode_control::seams(state, id, mode, tokens::roles(scheme))
            },
        ),
    ]
    .into()
}

/// `id`'s switching entry as a button (#943 FR-019, US4 scenarios 3, 4): **Open in terminal** for a
/// structured Claude session, **Open in structured view** for a terminal one, none for another CLI.
pub(crate) fn switch_button<'a>(
    state: &'a State,
    id: SessionId,
    r: tokens::Roles,
) -> Option<Element<'a, Message>> {
    let entry = crate::features::session::switch_entries(state, id)
        .into_iter()
        .next()?;
    Some(
        Button::outlined(entry.label, r)
            .on_press(Message::Composer(
                crate::features::composer::Msg::SwitchRequested {
                    session: id,
                    to: entry.to,
                },
            ))
            .into(),
    )
}

/// The primary terminal of `id` while the single pane shows its AI tab (#943).
fn active_primary(state: &State, id: SessionId) -> Option<TerminalRef> {
    state
        .active_sessions()
        .iter()
        .any(|s| s.id == id && s.mode == TerminalMode::AiCli)
        .then_some(TerminalRef {
            session: id,
            process: micold_core::protocol::messages::SessionProcess::Primary,
        })
}

/// Above a Claude terminal session's terminal (#943 US4 scenario 4, US5 scenario 3): how many
/// queued messages wait for the structured view, and **Open in structured view**. Anything else
/// gets an empty element, so the terminal under it keeps its place in the widget tree.
pub(crate) fn terminal_switch_strip<'a>(
    state: &'a State,
    terminal: Option<TerminalRef>,
    r: tokens::Roles,
) -> Element<'a, Message> {
    let strip = terminal
        .filter(|t| t.process == micold_core::protocol::messages::SessionProcess::Primary)
        .filter(|t| session_kind(state, t.session) == micold_core::session::SessionKind::Terminal)
        .and_then(|t| {
            let button = switch_button(state, t.session, r)?;
            let held = state
                .held_queue_line(t.session)
                .map(|line| Text::new(line, TypeRole::Caption, r).muted());
            Some(
                container(
                    row![]
                        .push(held)
                        .push(Space::new().width(Length::Fill))
                        .push(button)
                        .spacing(spacing::SM)
                        .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([spacing::XS, spacing::SM]),
            )
        });
    // An empty container rather than a bare `Space`: a void child is dropped from the tree, which
    // would move the terminal under it (see `right_aligned_tabs`).
    match strip {
        Some(strip) => strip.into(),
        None => container(Space::new()).into(),
    }
}

/// Whether the pane for `id` is the transcript: a structured session with its AI tab displayed.
pub(crate) fn shows_transcript(state: &State, id: SessionId) -> bool {
    state.active_sessions().iter().any(|s| {
        s.id == id
            && s.kind == micold_core::session::SessionKind::Structured
            && s.mode == TerminalMode::AiCli
    })
}

/// Whether `terminal` is a structured session's primary, which a tiled pane draws as its
/// transcript (feature 939, research R15); a shell instance stays a terminal.
pub(crate) fn is_structured_primary(state: &State, terminal: TerminalRef) -> bool {
    state.is_structured_primary(terminal)
}

/// How session `id` runs its CLI; a terminal session when the projection does not list it.
fn session_kind(state: &State, id: SessionId) -> micold_core::session::SessionKind {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.kind)
        .unwrap_or_default()
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
/// A give-up says more (`010` BUG-017 step 3). The daemon records *why* it stopped trying and how
/// many attempts that cost, and until this the client had both and showed neither: the bar said the
/// bare word `failed` and the pane said the same generic sentence a merely-idle session gets. This
/// is also the surface that **lasts** — the notification the reason raises is announced once, and a
/// window that attaches after the give-up, which is the unattended case the whole state exists for,
/// never saw it.
///
/// Answered from [`attached_process_restartable`] rather than from a second match on
/// `SessionLifecycle`. That predicate already means "the attached process is not running", already
/// handles the `TerminalMode` split, and is already what decides whether the `restart` control is
/// there to be pointed at. Deriving both from it is what makes the two *unable* to disagree —
/// which they did, for exactly as long as they were two readings of one fact.
pub(crate) fn empty_terminal_message(state: &State, id: SessionId) -> String {
    if !attached_process_restartable(state, id) {
        return "Starting…".to_string();
    }
    match give_up_reason(state, id) {
        // A start the daemon refused (`attempts: 0`) carries its own remedy, and pointing at
        // `restart` after it would contradict that remedy: a CLI missing from the image, or a
        // conversation the CLI no longer has, fails the same way on every restart until the user
        // changes something (`029` quickstart §D, finding 1).
        Some((reason, 0)) => reason.to_string(),
        // The recorded sentence already ends in a full stop (`on_unexpected_exit` assembles it), so
        // the pointer at the control follows it as a second sentence.
        Some((reason, _)) => format!("{reason} Choose restart below to resume it."),
        None => "This session is not running. Choose restart below to resume it.".to_string(),
    }
}

/// The sentence a give-up recorded, when there is one (`010` BUG-017).
///
/// `None` means both *"this session did not give up"* and *"it gave up and the reason is empty"*.
/// The wire type permits an empty reason — `catalog_sync::announce_start_failures` is built on
/// exactly that, and a client attached to a daemon from before BUG-017 receives one — so the two
/// have to collapse somewhere. Collapsing them here gives the pane one fallback instead of a blank
/// interpolated into the middle of a sentence.
fn give_up_reason(state: &State, id: SessionId) -> Option<(&str, u8)> {
    let session = state.active_sessions().iter().find(|s| s.id == id)?;
    match &session.lifecycle {
        SessionLifecycle::Failed { reason, attempts } => Some(reason.trim())
            .filter(|r| !r.is_empty())
            .map(|r| (r, *attempts)),
        _ => None,
    }
}

/// The tab strip's scroll viewport, by name, so `operation::scroll_to` can reach it (FR-002d).
///
/// A `LazyLock` for the reason the sidebar's is one: `scrollable::Id` is not a `const`, and the id
/// has to be the *same* one on the widget and in the operation or the operation finds nothing and
/// reports nothing.
pub static TAB_STRIP_SCROLL_ID: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("terminal-tab-strip-scroll"));

#[cfg(test)]
mod tests {
    //! Colour-mapping tests for `TermPalette` (feature 006, FR-001/FR-003), and the switcher
    //! tab's variant rule (feature 012 BUG-001). Bin unit tests — run with
    //! `cargo test --features gui`. See contracts/terminal-render-input.md.
    use super::*;
    use crate::features::session_strip::tests::{failed, state_showing, state_with_instances};
    use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb as AnsiRgb};
    use micold_core::session::{SessionLifecycle, SessionLocation, ShellLifecycle};

    /// FR-007: a plain cell and a bold cell resolve to the bundled Fira Mono Regular and Bold faces
    /// in the global font system, not to a host monospace face.
    #[test]
    fn cell_fonts_resolve_to_the_bundled_fira_mono_faces() {
        let _ = crate::ui::material::test_support::renderer();
        assert!(
            crate::ui::material::test_support::resolves_to_bundled(
                cell_font(Flags::empty()),
                crate::ui::FIRA_MONO_REGULAR_BYTES
            ),
            "a plain terminal cell does not resolve to the bundled Fira Mono Regular"
        );
        assert!(
            crate::ui::material::test_support::resolves_to_bundled(
                cell_font(Flags::BOLD),
                crate::ui::FIRA_MONO_BOLD_BYTES
            ),
            "a bold terminal cell does not resolve to the bundled Fira Mono Bold"
        );
    }

    /// FR-007: the grid's cell width is the width the bundled face's glyphs actually advance, read
    /// from its `hmtx`, at the sizes the terminal is drawn at.
    #[test]
    fn cell_width_equals_the_bundled_face_advance() {
        let face = ttf_parser::Face::parse(crate::ui::FIRA_MONO_REGULAR_BYTES, 0)
            .expect("bundled Fira Mono Regular parses");
        let glyph = face.glyph_index('M').expect("Fira Mono has 'M'");
        let advance = f32::from(face.glyph_hor_advance(glyph).expect("'M' has an advance"))
            / f32::from(face.units_per_em());
        for size in [12.0_f32, 13.0, 14.0] {
            let width = CellMetrics::new(size).width;
            assert!(
                (width - advance * size).abs() < 1e-4,
                "cell width {width} at size {size} is not Fira Mono's advance {}",
                advance * size
            );
        }
    }

    /// Feature 491, T026: a terminal's container is the bound daemon's own, not the fixed
    /// single-sandbox name; a worktree with no container daemon of its own falls back to it.
    #[test]
    fn the_terminal_targets_the_container_of_the_daemon_its_worktree_is_bound_to() {
        use crate::features::sandbox::{Sandbox, SandboxLocations};
        use micold_core::daemons::{ContainerSettings, DaemonRegistry, DaemonRuntime};
        use micold_core::sandbox::lifecycle::SandboxState;
        use micold_core::sandbox::runtime::ContainerId;

        let (mut state, id) = state_showing(SessionLifecycle::Running);
        let project = std::path::PathBuf::from("/repo");
        let mut registry = DaemonRegistry::new(Vec::new(), 1);
        let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
        let boxed = registry
            .add(
                "Box",
                DaemonRuntime::Container(ContainerSettings {
                    profile: Default::default(),
                    container_name: "box-ctr".into(),
                    port: 7001,
                    migration: None,
                }),
                true,
            )
            .unwrap();
        state.settings.daemons = registry;
        let session = &mut state.workspace.sessions.get_mut(&project).unwrap()[0];
        session.location = SessionLocation::Worktree("wt1".into());
        state.workspace.bind(&project, "wt1", boxed);
        state.session.active = Some(id);

        let container = ContainerId("0123456789abcdef0123456789".into());
        let sandbox = Sandbox {
            state: SandboxState::Running(container.clone()),
            locations: Some((container, SandboxLocations::default())),
            ..Sandbox::default()
        };

        let names = |state: &State| {
            link_context(state, &sandbox)
                .sandbox
                .expect("a running sandbox with locations has a link context")
                .host_names
        };
        let bound = names(&state);
        assert!(bound.iter().any(|n| n == "box-ctr"), "{bound:?}");
        assert!(!bound.iter().any(|n| n == "micold-sandbox"), "{bound:?}");

        state.workspace.bind(&project, "wt1", host);
        let fallback = names(&state);
        assert!(
            fallback
                .iter()
                .any(|n| n == micold_core::sandbox::CONTAINER_NAME),
            "a host daemon has no container of its own: {fallback:?}"
        );
        assert!(!fallback.iter().any(|n| n == "box-ctr"), "{fallback:?}");
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
            (failed(), ShellLifecycle::NotStarted),
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
                state_with_instances(TerminalMode::Regular, lifecycle.clone(), &[shell], Some(0));
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
            let (state, id) =
                state_with_instances(TerminalMode::AiCli, lifecycle.clone(), &[], None);
            assert!(
                crate::ui::strip_tab_menu_labels(&state, id, StripTab::Ai).is_empty(),
                "{lifecycle:?}: a secondary press on a running AI tab must do nothing — an empty \
                 panel says the offer exists and then withholds it"
            );
        }
        for lifecycle in [
            SessionLifecycle::Idle,
            failed(),
            SessionLifecycle::InterruptedResumable,
        ] {
            let (state, id) =
                state_with_instances(TerminalMode::AiCli, lifecycle.clone(), &[], None);
            assert_eq!(
                crate::ui::strip_tab_menu_labels(&state, id, StripTab::Ai),
                vec!["Restart"],
                "{lifecycle:?}: a stopped AI process has exactly one thing that can be done to it, \
                 and the mark on its tab (FR-012d) is what points at it"
            );
        }
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
            failed(),
        ] {
            let (state, id) = state_showing(lifecycle.clone());
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
            let (state, id) = state_showing(lifecycle.clone());
            assert_eq!(
                empty_terminal_message(&state, id),
                "Starting…",
                "{lifecycle:?} has a process up or on its way, so an empty pane really is waiting \
                 for the first frame"
            );
        }
    }

    /// The other half of step 3: the pane says *why*, at length, beside the `restart` control that
    /// resolves it.
    ///
    /// The sentence cannot go in the bar. That row is content-sized labels sharing its width with
    /// the tab strip, which is its one flexible member (FR-002c) — so every character of status is
    /// taken from the strip, and the strip is the only route back to the assistant. The pane is
    /// where there is room.
    ///
    /// It is also the surface that *lasts*. The notification the daemon's reason now raises
    /// (`announce_start_failures`) is transient and is announced once; a window that attaches after
    /// the give-up — the unattended case this whole state exists for — never saw it.
    #[test]
    fn the_empty_pane_says_why_a_session_gave_up() {
        let (state, id) = state_showing(SessionLifecycle::Failed {
            reason: "Gave up after 3 restart attempts — last exit: killed by Terminated.".into(),
            attempts: 3,
        });
        let message = empty_terminal_message(&state, id);
        assert!(
            message.contains("killed by Terminated"),
            "the pane must carry the reason the daemon recorded, not a generic sentence about not              running — the diagnosis is the whole point of the give-up state; got {message:?}"
        );
        assert!(
            message.contains("restart"),
            "and must still point at the control that resolves it (025 FR-014); got {message:?}"
        );
    }

    /// A start the daemon refused is shown as its own advice, without the pointer at `restart`
    /// (`029` quickstart §D, finding 1).
    ///
    /// The daemon reports a refusal as `Failed { attempts: 0 }`, and every refusal's sentence
    /// already says what to change. Appending "Choose restart below to resume it." told a user
    /// whose image lacks the CLI to retry something that fails the same way until the image
    /// changes, and one whose conversation is gone to resume what cannot be resumed.
    #[test]
    fn a_refused_start_is_not_pointed_at_restart() {
        let reason = "Pi Coding Agent isn't in registry.example/agents:7, where sessions run, and \
                      this conversation can only continue in it. Choose an image that provides \
                      it, then restart this session.";
        let (state, id) = state_showing(SessionLifecycle::Failed {
            reason: reason.into(),
            attempts: 0,
        });
        assert_eq!(empty_terminal_message(&state, id), reason);
    }

    /// A give-up whose reason is empty must still read as a sentence.
    ///
    /// The wire type permits one — `catalog_sync`'s announcement guard is built on exactly that —
    /// so a client talking to a daemon from before BUG-017 gets one. Asserted as *the same thing
    /// any not-running session says* rather than against a literal, because the claim is structural
    /// (fall back, do not interpolate a blank) and pinning the prose in a second place is what
    /// makes prose unchangeable rather than correct.
    #[test]
    fn a_give_up_with_no_reason_still_reads_as_a_sentence() {
        let (state, id) = state_showing(SessionLifecycle::Failed {
            reason: String::new(),
            attempts: 3,
        });
        let (idle, idle_id) = state_showing(SessionLifecycle::Idle);
        assert_eq!(
            empty_terminal_message(&state, id),
            empty_terminal_message(&idle, idle_id),
            "an empty reason must leave the sentence a not-running session already had, not a              fragment with the reason's place left blank"
        );
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
            failed(),
            SessionLifecycle::InterruptedResumable,
        ] {
            let (state, id) = state_showing(lifecycle.clone());
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

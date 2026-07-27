//! The shared Material icon vocabulary: the single source every surface draws from, so the
//! same concept is always the same glyph (FR-001, FR-002).
//!
//! Pure data with no iced dependency — the name→codepoint mapping and the per-surface color
//! role are testable under `cargo test --no-default-features` (FR-003, FR-008, SC-006). The
//! GUI layer (`src/main.rs`, `src/ui/`) loads the backing font and renders these glyphs.
//!
//! `Icon` is a *closed* enum: an unknown icon is unrepresentable, so a misspelled or missing
//! icon is a compile error rather than a runtime blank box ("tofu"; FR-003, SC-005). Glyph
//! codepoints are pinned from the shipped font and documented in
//! `assets/fonts/PROVENANCE.md`; `tests/icons.rs` regression-locks them.

use micold_core::tokens::{Rgb, Roles};
use micold_core::session::TerminalMode;

/// A named Material symbol representing one concept or action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// App-bar Help action.
    Help,
    /// Help menu → About action.
    About,
    /// Open / choose a project (empty state and "open another").
    OpenProject,
    /// Rename a known project.
    Rename,
    /// Git-repository badge.
    Git,
    /// The active known-project marker.
    ActiveMarker,
    /// An unavailable known-project marker.
    Unavailable,
    /// Navigate up a directory in the project selector.
    NavigateUp,
    /// Add a new session (the "+" action, feature 005).
    AddSession,
    /// Add a new worktree (a branch/tree action, feature 005).
    AddWorktree,
    /// Overflow / "more" menu trigger (three vertical dots).
    Menu,
    /// Light theme mode.
    LightMode,
    /// Dark theme mode.
    DarkMode,
    /// Auto (follow-system) theme mode.
    AutoMode,
    /// Collapse/hide the sidebar.
    HideSidebar,
    /// Expand/show the sidebar.
    ShowSidebar,
    /// Application settings (feature 006).
    Settings,
    /// Delete / trash action (feature 008).
    Delete,
    /// Copy text (e.g. a worktree name) to the system clipboard.
    Copy,
    /// Toggle the sidebar tag-filter panel (feature 009).
    Filter,
    /// The AI CLI (`claude`) terminal mode (feature 010).
    AiCli,
    /// The regular/plain shell terminal mode (feature 010).
    RegularTerminal,
    /// Release the terminal's keyboard focus (feature 006/010 bottom bar).
    ReleaseFocus,
    /// The "Default" project-root sidebar entry (feature 010) — distinct from the git/branch
    /// iconography used for worktree rows (FR-006).
    ProjectRoot,
    /// Open an additional Regular Terminal instance for a session (feature 011, FR-001;
    /// `add_box` — distinct from `AddSession`'s plain `add` glyph).
    AddTerminalInstance,
    /// Close/dismiss action (an "×" glyph) — e.g. a session row's trailing button, or closing a
    /// Regular Terminal instance (feature 011, FR-011). Distinct from [`Icon::Delete`] (a trash
    /// can): closing dismisses something without destroying anything on disk.
    Close,
    /// A session's activity dot for a **live** state — working or awaiting input (feature 010
    /// US2, FR-016d/FR-016e). A ring with a solid centre, so it reads as filled next to
    /// [`Icon::ActivityEnded`]'s empty ring.
    ActivityWorking,
    /// A session's activity dot for a **spent** state — the session ended (FR-016d/FR-016e). An
    /// empty ring, the same outer diameter as [`Icon::ActivityWorking`] so rows stay aligned.
    ActivityEnded,
}

impl Icon {
    /// Every icon variant, for exhaustive iteration and testing.
    pub const ALL: &'static [Icon] = &[
        Icon::Help,
        Icon::About,
        Icon::OpenProject,
        Icon::Rename,
        Icon::Git,
        Icon::ActiveMarker,
        Icon::Unavailable,
        Icon::NavigateUp,
        Icon::AddSession,
        Icon::AddWorktree,
        Icon::Menu,
        Icon::LightMode,
        Icon::DarkMode,
        Icon::AutoMode,
        Icon::HideSidebar,
        Icon::ShowSidebar,
        Icon::Settings,
        Icon::Delete,
        Icon::Copy,
        Icon::Filter,
        Icon::AiCli,
        Icon::RegularTerminal,
        Icon::ReleaseFocus,
        Icon::ProjectRoot,
        Icon::AddTerminalInstance,
        Icon::Close,
        Icon::ActivityWorking,
        Icon::ActivityEnded,
    ];

    /// The font codepoint for this icon (Private Use Area; see `assets/fonts/PROVENANCE.md`).
    /// Total function — every variant maps, no panic path.
    pub const fn glyph(self) -> char {
        match self {
            Icon::Help => '\u{e8fd}',
            Icon::About => '\u{e88e}',
            Icon::OpenProject => '\u{e2c8}',
            Icon::Rename => '\u{f097}',
            Icon::Git => '\u{eaf5}',
            Icon::ActiveMarker => '\u{f0be}',
            Icon::Unavailable => '\u{f8b6}',
            Icon::NavigateUp => '\u{e5d8}',
            Icon::AddSession => '\u{e145}',
            Icon::AddWorktree => '\u{e97a}',
            Icon::Menu => '\u{e5d4}',
            Icon::LightMode => '\u{e518}',
            Icon::DarkMode => '\u{e51c}',
            Icon::AutoMode => '\u{e1ab}',
            Icon::HideSidebar => '\u{f717}',
            Icon::ShowSidebar => '\u{f716}',
            Icon::Settings => '\u{e8b8}',
            Icon::Delete => '\u{e872}',
            Icon::Copy => '\u{e14d}',
            Icon::Filter => '\u{e152}',
            Icon::AiCli => '\u{e65f}',
            Icon::RegularTerminal => '\u{eb8e}',
            Icon::ReleaseFocus => '\u{e31a}',
            Icon::ProjectRoot => '\u{e88a}',
            Icon::AddTerminalInstance => '\u{e146}',
            Icon::Close => '\u{e5cd}',
            // The shipped font is a static instance pinned at FILL=0 (PROVENANCE.md), where the
            // nominally-solid dots (`circle`, `lens`, `fiber_manual_record`) all render as rings —
            // verified by rasterizing them. `radio_button_checked` is the only same-diameter glyph
            // with a genuinely filled centre, so this pair is what actually reads as filled vs
            // hollow in *this* font rather than merely by icon name.
            Icon::ActivityWorking => '\u{e837}',
            Icon::ActivityEnded => '\u{e836}',
        }
    }
}

/// The surface context an icon is drawn on. Determines which foreground color role tints the
/// icon (via [`icon_role`]) so light/dark theming and legibility follow the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSurface {
    /// App-bar / body-text actions — tinted `on_surface`.
    AppBarAction,
    /// Filled primary buttons — tinted `on_primary`.
    PrimaryButton,
    /// Badges, captions, and secondary markers — tinted `on_surface_variant`.
    Badge,
    /// Unavailable / danger markers — tinted with the `error` accent.
    Unavailable,
}

impl IconSurface {
    /// Every surface context, for exhaustive testing.
    pub const ALL: &'static [IconSurface] = &[
        IconSurface::AppBarAction,
        IconSurface::PrimaryButton,
        IconSurface::Badge,
        IconSurface::Unavailable,
    ];
}

/// The foreground color role an icon uses on a given surface (FR-004, FR-007).
///
/// Every arm returns a *foreground/accent* role from the active scheme — never a fill role —
/// so an icon is never tinted invisibly against its surface, in either scheme. Because these
/// are the same roles the existing text uses, the AA-contrast guarantees in `tests/tokens.rs`
/// transitively cover icon legibility (SC-004).
pub const fn icon_role(surface: IconSurface, roles: Roles) -> Rgb {
    match surface {
        IconSurface::AppBarAction => roles.on_surface,
        IconSurface::PrimaryButton => roles.on_primary,
        IconSurface::Badge => roles.on_surface_variant,
        IconSurface::Unavailable => roles.error,
    }
}

/// The icon that identifies `mode` at a glance on the terminal's mode-toggle button (FR-009;
/// contracts/mode-toggle-ui.md). Total and distinct per variant — the button re-derives this
/// on every render rather than caching it, so the indicator is always current by construction.
///
/// Lives in the client (not core `session`): it maps the render-free [`TerminalMode`] onto a
/// client-only [`Icon`], so core stays iced/render-free (FR-040).
pub const fn mode_glyph(mode: TerminalMode) -> Icon {
    match mode {
        TerminalMode::AiCli => Icon::AiCli,
        TerminalMode::Regular => Icon::RegularTerminal,
    }
}

/// The toggle button's tooltip copy for `mode` (FR-009; contracts/mode-toggle-ui.md) — states
/// the current mode and what pressing the button switches to.
pub const fn mode_tooltip(mode: TerminalMode) -> &'static str {
    match mode {
        TerminalMode::AiCli => "AI CLI — switch to Regular Terminal",
        TerminalMode::Regular => "Regular Terminal — switch to AI CLI",
    }
}

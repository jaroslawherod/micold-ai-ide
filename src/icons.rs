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

use crate::tokens::{Rgb, Roles};

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
    /// Toggle the sidebar tag-filter panel (feature 009).
    Filter,
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
        Icon::Filter,
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
            Icon::Filter => '\u{e152}',
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

//! The design system: the single source of every color, type size, spacing step, and
//! corner radius the UI draws from (FR-002, FR-003, SC-007).
//!
//! Pure data with no iced dependency, so the values — notably the AA contrast invariant
//! (SC-005) — are testable under `cargo test --no-default-features`. The GUI layer
//! (`src/ui/style.rs`) converts these into iced types. Values are the durable contract in
//! contracts/design-tokens.md.

use crate::naming::ConventionalType;
use crate::theme::ColorScheme;

/// A plain 8-bit-per-channel sRGB color. The GUI maps this to `iced::Color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Construct a color from a `0xRRGGBB` hex literal (compile-time friendly).
    pub const fn hex(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }
}

/// The Material semantic color roles for one scheme. Each `on_*` role is designed to meet
/// AA contrast against its paired surface (enforced by `tests/tokens.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roles {
    /// Window background.
    pub background: Rgb,
    /// Text/icons on `background`.
    pub on_background: Rgb,
    /// Cards / app bar / dialogs.
    pub surface: Rgb,
    /// Primary text on surfaces.
    pub on_surface: Rgb,
    /// List rows / subtle fills / divider base.
    pub surface_variant: Rgb,
    /// Secondary text (paths, captions, badges).
    pub on_surface_variant: Rgb,
    /// Filled primary actions and accents.
    pub primary: Rgb,
    /// Text/icons on `primary`.
    pub on_primary: Rgb,
    /// Outlined-button borders and focus rings.
    pub outline: Rgb,
    /// Error text / danger actions.
    pub error: Rgb,
    /// Text/icons on `error`.
    pub on_error: Rgb,
    /// Per-type worktree tag fills (FR-005). Each is distinct and, paired with [`on_tag`],
    /// meets AA in this scheme (enforced by `tests/tokens.rs`).
    pub tag_feat: Rgb,
    pub tag_fix: Rgb,
    pub tag_chore: Rgb,
    pub tag_docs: Rgb,
    pub tag_refactor: Rgb,
    pub tag_test: Rgb,
    pub tag_build: Rgb,
    pub tag_ci: Rgb,
    pub tag_perf: Rgb,
    pub tag_style: Rgb,
    /// Jira/issue tag fill — a neutral style distinct from the type colors (FR-004).
    pub tag_issue: Rgb,
    /// Text/icons drawn on every tag chip fill in this scheme.
    pub on_tag: Rgb,
}

impl Roles {
    /// Fill color for a worktree type tag.
    pub fn tag_fill(&self, t: ConventionalType) -> Rgb {
        match t {
            ConventionalType::Feat => self.tag_feat,
            ConventionalType::Fix => self.tag_fix,
            ConventionalType::Chore => self.tag_chore,
            ConventionalType::Docs => self.tag_docs,
            ConventionalType::Refactor => self.tag_refactor,
            ConventionalType::Test => self.tag_test,
            ConventionalType::Build => self.tag_build,
            ConventionalType::Ci => self.tag_ci,
            ConventionalType::Perf => self.tag_perf,
            ConventionalType::Style => self.tag_style,
        }
    }

    /// `(fill, on_fill)` for a worktree type tag chip.
    pub fn type_tag(&self, t: ConventionalType) -> (Rgb, Rgb) {
        (self.tag_fill(t), self.on_tag)
    }

    /// `(fill, on_fill)` for the Jira/issue tag chip.
    pub fn issue_tag(&self) -> (Rgb, Rgb) {
        (self.tag_issue, self.on_tag)
    }
}

/// The light-scheme palette (contracts/design-tokens.md).
pub const LIGHT: Roles = Roles {
    background: Rgb::hex(0xFDFCFF),
    on_background: Rgb::hex(0x1A1C1E),
    surface: Rgb::hex(0xFFFFFF),
    on_surface: Rgb::hex(0x1A1C1E),
    surface_variant: Rgb::hex(0xEEF0F4),
    on_surface_variant: Rgb::hex(0x43474E),
    primary: Rgb::hex(0x005DB8),
    on_primary: Rgb::hex(0xFFFFFF),
    outline: Rgb::hex(0x73777F),
    error: Rgb::hex(0xBA1A1A),
    on_error: Rgb::hex(0xFFFFFF),
    // Deep 900-shade fills: dark enough to be AA as the tonal chip's TEXT on a faint tint of
    // themselves (tests/tokens.rs::tonal_tag_chips_meet_aa_contrast), and to carry white text on
    // the solid filter chip. Distinct hues; feat = green.
    tag_feat: Rgb::hex(0x1B5E20),
    tag_fix: Rgb::hex(0xB71C1C),
    tag_chore: Rgb::hex(0x4E342E),
    tag_docs: Rgb::hex(0x004D40),
    tag_refactor: Rgb::hex(0x4A148C),
    tag_test: Rgb::hex(0x0D47A1),
    tag_build: Rgb::hex(0x7A2600),
    tag_ci: Rgb::hex(0x1A237E),
    tag_perf: Rgb::hex(0x880E4F),
    tag_style: Rgb::hex(0x33691E),
    tag_issue: Rgb::hex(0x37474F),
    on_tag: Rgb::hex(0xFFFFFF),
    // NOTE: tag_build uses deep-orange 900 (not 800) so white text clears AA (4.5:1).
};

/// The dark-scheme palette (contracts/design-tokens.md).
pub const DARK: Roles = Roles {
    background: Rgb::hex(0x1A1C1E),
    on_background: Rgb::hex(0xE2E2E6),
    surface: Rgb::hex(0x212426),
    on_surface: Rgb::hex(0xE2E2E6),
    surface_variant: Rgb::hex(0x2B2F31),
    on_surface_variant: Rgb::hex(0xC3C7CF),
    primary: Rgb::hex(0xA6C8FF),
    on_primary: Rgb::hex(0x00325B),
    outline: Rgb::hex(0x8D9199),
    error: Rgb::hex(0xFFB4AB),
    on_error: Rgb::hex(0x690005),
    // Pale fills carrying dark text (`on_tag`).
    tag_feat: Rgb::hex(0xA5D6A7),
    tag_fix: Rgb::hex(0xEF9A9A),
    tag_chore: Rgb::hex(0xBCAAA4),
    tag_docs: Rgb::hex(0x80CBC4),
    tag_refactor: Rgb::hex(0xCE93D8),
    tag_test: Rgb::hex(0x90CAF9),
    tag_build: Rgb::hex(0xFFAB91),
    tag_ci: Rgb::hex(0x9FA8DA),
    tag_perf: Rgb::hex(0xF48FB1),
    tag_style: Rgb::hex(0xE6EE9C),
    tag_issue: Rgb::hex(0xB0BEC5),
    on_tag: Rgb::hex(0x1A1C1E),
};

/// Select the palette for a resolved scheme.
pub fn roles(scheme: ColorScheme) -> Roles {
    match scheme {
        ColorScheme::Light => LIGHT,
        ColorScheme::Dark => DARK,
    }
}

/// Typography size scale, in logical pixels (contracts/design-tokens.md). Weights are
/// applied in the GUI via `iced::Font`.
pub mod type_scale {
    /// Large empty-state / dialog headline.
    pub const DISPLAY: u16 = 32;
    /// Active-project name, section headers.
    pub const HEADLINE: u16 = 24;
    /// App-bar title, list-item primary text.
    pub const TITLE: u16 = 18;
    /// Default body text, descriptions.
    pub const BODY: u16 = 14;
    /// Paths, captions, badges.
    pub const LABEL: u16 = 12;
}

/// Sidebar-scoped type sizes — 80% of the app-wide scale (FR-012). Applied ONLY within the
/// worktree sidebar; the rest of the app keeps [`type_scale`]. Kept as named constants so the
/// "80%" intent is auditable (`tests/tokens.rs`).
pub mod sidebar {
    /// Worktree display name — 80% of `type_scale::BODY` (14 → 11).
    pub const NAME: u16 = 11;
    /// Tag chip text — 80% of `type_scale::LABEL` (12 → 10).
    pub const TAG: u16 = 10;
    /// Session label — 80% of `type_scale::BODY` (14 → 11).
    pub const SESSION: u16 = 11;
}

/// Spacing scale, in logical pixels. All padding/gaps use these steps (SC-007).
pub mod spacing {
    pub const XS: u16 = 4;
    pub const SM: u16 = 8;
    pub const MD: u16 = 16;
    pub const LG: u16 = 24;
    pub const XL: u16 = 32;
}

/// Corner radii, in logical pixels.
pub mod shape {
    /// Buttons, badges.
    pub const SM: u16 = 8;
    /// Cards / list items / surfaces.
    pub const MD: u16 = 12;
    /// Dialogs.
    pub const LG: u16 = 16;
    /// Pills.
    pub const FULL: u16 = 9999;
}

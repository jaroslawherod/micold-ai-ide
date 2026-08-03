//! The design system: the single source of every color, type size, spacing step, and
//! corner radius the UI draws from (FR-002, FR-003, SC-007).
//!
//! Pure data with no iced dependency, so the values — notably the AA contrast invariant
//! (SC-001) — are testable without building the GUI stack. The GUI layer (`src/ui/style.rs`)
//! converts these into iced types. Values are the durable contract in
//! `specs/018-material3-visual-system/contracts/design-tokens.md`.
//!
//! # Roles are palette-and-tone pairs
//!
//! Feature 018 retired hand-picked hex values. Every role in [`LIGHT`] and [`DARK`] now names a
//! ramp in [`palette`] and a tone on it (contract §1.2), so contrast follows from the tone delta
//! rather than from someone having checked. Both schemes read the *same* ramps at different tones,
//! which is what keeps light and dark structurally locked together: a role added once is correct in
//! both.

use crate::naming::ConventionalType;
use crate::theme::ColorScheme;

pub mod palette;

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

/// The Material 3 semantic color roles for one scheme (contract §1.2).
///
/// Every field is produced by reading a [`palette`] ramp at a stated tone — see [`LIGHT`] and
/// [`DARK`], which are the normative role→tone map in executable form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roles {
    // --- accent ---
    pub primary: Rgb,
    pub on_primary: Rgb,
    pub primary_container: Rgb,
    pub on_primary_container: Rgb,
    pub secondary: Rgb,
    pub on_secondary: Rgb,
    pub secondary_container: Rgb,
    pub on_secondary_container: Rgb,
    pub tertiary: Rgb,
    pub on_tertiary: Rgb,
    pub tertiary_container: Rgb,
    pub on_tertiary_container: Rgb,
    pub error: Rgb,
    pub on_error: Rgb,
    pub error_container: Rgb,
    pub on_error_container: Rgb,

    // --- surfaces ---
    /// Window background.
    pub background: Rgb,
    pub on_background: Rgb,
    pub surface: Rgb,
    pub on_surface: Rgb,
    pub surface_dim: Rgb,
    pub surface_bright: Rgb,
    /// The five container levels elevation reads (contract §4).
    pub surface_container_lowest: Rgb,
    pub surface_container_low: Rgb,
    pub surface_container: Rgb,
    pub surface_container_high: Rgb,
    pub surface_container_highest: Rgb,
    pub surface_variant: Rgb,
    /// Secondary text (paths, captions, badges).
    pub on_surface_variant: Rgb,

    // --- lines and inverses ---
    /// Outlined-control borders and focus rings.
    pub outline: Rgb,
    /// Dividers.
    pub outline_variant: Rgb,
    pub inverse_surface: Rgb,
    pub inverse_on_surface: Rgb,
    pub inverse_primary: Rgb,
    /// Drawn under modal surfaces at the alpha §4 states, never at full strength.
    pub scrim: Rgb,
    /// Drop shadows, likewise drawn at an alpha rather than at full strength.
    pub shadow: Rgb,

    /// Which scheme this role set is, so tag tones can follow it without a second lookup.
    scheme: ColorScheme,
}

impl Roles {
    /// `(fill, text)` for a worktree type tag chip (contract §1.4).
    ///
    /// Unlike feature 003, the text color is **per tag** rather than one shared `on_tag`: each tag
    /// reads its own hue at the text tone, which is what makes AA structural instead of
    /// hand-verified.
    pub fn type_tag(&self, t: ConventionalType) -> (Rgb, Rgb) {
        self.tag(t)
    }

    /// `(fill, text)` for a worktree type tag chip. The name `type_tag` is kept as the call-site
    /// spelling; this is the same thing under the name the contract uses.
    pub fn tag(&self, t: ConventionalType) -> (Rgb, Rgb) {
        let ramp = match t {
            ConventionalType::Feat => palette::TAG_FEAT,
            ConventionalType::Fix => palette::TAG_FIX,
            ConventionalType::Chore => palette::TAG_CHORE,
            ConventionalType::Docs => palette::TAG_DOCS,
            ConventionalType::Refactor => palette::TAG_REFACTOR,
            ConventionalType::Test => palette::TAG_TEST,
            ConventionalType::Build => palette::TAG_BUILD,
            ConventionalType::Ci => palette::TAG_CI,
            ConventionalType::Perf => palette::TAG_PERF,
            ConventionalType::Style => palette::TAG_STYLE,
        };
        self.read_tag(ramp)
    }

    /// `(fill, text)` for the Jira/issue tag chip — the neutral hue (FR-006a).
    pub fn issue_tag(&self) -> (Rgb, Rgb) {
        self.read_tag(palette::TAG_ISSUE)
    }

    /// The tag tone recipe: fill 40 / text 100 in light, fill 80 / text 20 in dark (contract §1.4).
    /// One place, so every tag in both schemes uses the same tone delta and therefore clears AA by
    /// construction.
    fn read_tag(&self, ramp: palette::TagRamp) -> (Rgb, Rgb) {
        match self.scheme {
            ColorScheme::Light => (ramp.at(40), ramp.at(100)),
            ColorScheme::Dark => (ramp.at(80), ramp.at(20)),
        }
    }

    /// The fill of a worktree type tag, without its text color.
    pub fn tag_fill(&self, t: ConventionalType) -> Rgb {
        self.tag(t).0
    }
}

/// The light scheme — the normative role→tone map of contract §1.2, read from [`palette`].
pub const LIGHT: Roles = Roles {
    primary: palette::PRIMARY.at(40),
    on_primary: palette::PRIMARY.at(100),
    primary_container: palette::PRIMARY.at(90),
    on_primary_container: palette::PRIMARY.at(10),
    secondary: palette::SECONDARY.at(40),
    on_secondary: palette::SECONDARY.at(100),
    secondary_container: palette::SECONDARY.at(90),
    on_secondary_container: palette::SECONDARY.at(10),
    tertiary: palette::TERTIARY.at(40),
    on_tertiary: palette::TERTIARY.at(100),
    tertiary_container: palette::TERTIARY.at(90),
    on_tertiary_container: palette::TERTIARY.at(10),
    error: palette::ERROR.at(40),
    on_error: palette::ERROR.at(100),
    error_container: palette::ERROR.at(90),
    on_error_container: palette::ERROR.at(10),

    background: palette::NEUTRAL.at(98),
    on_background: palette::NEUTRAL.at(10),
    surface: palette::NEUTRAL.at(98),
    on_surface: palette::NEUTRAL.at(10),
    surface_dim: palette::NEUTRAL.at(87),
    surface_bright: palette::NEUTRAL.at(98),
    surface_container_lowest: palette::NEUTRAL.at(100),
    surface_container_low: palette::NEUTRAL.at(96),
    surface_container: palette::NEUTRAL.at(94),
    surface_container_high: palette::NEUTRAL.at(92),
    surface_container_highest: palette::NEUTRAL.at(90),
    surface_variant: palette::NEUTRAL_VARIANT.at(90),
    on_surface_variant: palette::NEUTRAL_VARIANT.at(30),

    outline: palette::NEUTRAL_VARIANT.at(50),
    outline_variant: palette::NEUTRAL_VARIANT.at(80),
    inverse_surface: palette::NEUTRAL.at(20),
    inverse_on_surface: palette::NEUTRAL.at(95),
    inverse_primary: palette::PRIMARY.at(80),
    scrim: palette::NEUTRAL.at(0),
    shadow: palette::NEUTRAL.at(0),

    scheme: ColorScheme::Light,
};

/// The dark scheme — the same ramps as [`LIGHT`], read at the dark tones of contract §1.2.
pub const DARK: Roles = Roles {
    primary: palette::PRIMARY.at(80),
    on_primary: palette::PRIMARY.at(20),
    primary_container: palette::PRIMARY.at(30),
    on_primary_container: palette::PRIMARY.at(90),
    secondary: palette::SECONDARY.at(80),
    on_secondary: palette::SECONDARY.at(20),
    secondary_container: palette::SECONDARY.at(30),
    on_secondary_container: palette::SECONDARY.at(90),
    tertiary: palette::TERTIARY.at(80),
    on_tertiary: palette::TERTIARY.at(20),
    tertiary_container: palette::TERTIARY.at(30),
    on_tertiary_container: palette::TERTIARY.at(90),
    error: palette::ERROR.at(80),
    on_error: palette::ERROR.at(20),
    error_container: palette::ERROR.at(30),
    on_error_container: palette::ERROR.at(90),

    background: palette::NEUTRAL.at(6),
    on_background: palette::NEUTRAL.at(90),
    surface: palette::NEUTRAL.at(6),
    on_surface: palette::NEUTRAL.at(90),
    surface_dim: palette::NEUTRAL.at(6),
    surface_bright: palette::NEUTRAL.at(24),
    surface_container_lowest: palette::NEUTRAL.at(4),
    surface_container_low: palette::NEUTRAL.at(10),
    surface_container: palette::NEUTRAL.at(12),
    surface_container_high: palette::NEUTRAL.at(17),
    surface_container_highest: palette::NEUTRAL.at(22),
    surface_variant: palette::NEUTRAL_VARIANT.at(30),
    on_surface_variant: palette::NEUTRAL_VARIANT.at(80),

    outline: palette::NEUTRAL_VARIANT.at(60),
    outline_variant: palette::NEUTRAL_VARIANT.at(30),
    inverse_surface: palette::NEUTRAL.at(90),
    inverse_on_surface: palette::NEUTRAL.at(20),
    inverse_primary: palette::PRIMARY.at(40),
    scrim: palette::NEUTRAL.at(0),
    shadow: palette::NEUTRAL.at(0),

    scheme: ColorScheme::Dark,
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
///
/// Carried across the module split unchanged (T000e). The Material 3 type roles that supersede
/// these arrive with T000h; until then every existing call site keeps its current size.
pub mod type_scale {
    /// Large empty-state / dialog headline.
    pub const DISPLAY: f32 = 32.0;
    /// Active-project name, section headers.
    pub const HEADLINE: f32 = 24.0;
    /// App-bar title, list-item primary text.
    pub const TITLE: f32 = 18.0;
    /// Default body text, descriptions.
    pub const BODY: f32 = 14.0;
    /// Paths, captions, badges.
    pub const LABEL: f32 = 12.0;
}

/// Sidebar-scoped type sizes — 80% of the app-wide scale (FR-012). Applied ONLY within the
/// worktree sidebar; the rest of the app keeps [`type_scale`].
pub mod sidebar {
    /// Worktree display name — 80% of `type_scale::BODY` (14 → 11).
    pub const NAME: f32 = 11.0;
    /// Tag chip text — 80% of `type_scale::LABEL` (12 → 10).
    pub const TAG: f32 = 10.0;
    /// Session label — 80% of `type_scale::BODY` (14 → 11).
    pub const SESSION: f32 = 11.0;
}

/// Spacing scale, in logical pixels. All padding/gaps use these steps (SC-007).
pub mod spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 32.0;
}

/// Corner radii, in logical pixels.
///
/// Carried across the module split unchanged (T000e); the seven-size Material shape scale that
/// supersedes it arrives with T000i.
pub mod shape {
    /// Buttons, badges.
    pub const SM: f32 = 8.0;
    /// Cards / list items / surfaces.
    pub const MD: f32 = 12.0;
    /// Dialogs.
    pub const LG: f32 = 16.0;
    /// Pills.
    pub const FULL: f32 = 9999.0;
}

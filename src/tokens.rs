//! The design system: the single source of every color, type size, spacing step, and
//! corner radius the UI draws from (FR-002, FR-003, SC-007).
//!
//! Pure data with no iced dependency, so the values — notably the AA contrast invariant
//! (SC-005) — are testable under `cargo test --no-default-features`. The GUI layer
//! (`src/ui/style.rs`) converts these into iced types. Values are the durable contract in
//! contracts/design-tokens.md.

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

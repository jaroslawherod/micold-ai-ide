//! Elevation (feature 018, T000i — FR-014, FR-015, FR-016, FR-017; contract §4).
//!
//! Six levels, each carrying **both** a tonal surface role and a drop shadow.
//!
//! Carrying both is the point. A black shadow on a dark background is nearly invisible, so in the
//! dark scheme the tonal shift is what makes a level read at all (FR-016) — a level defined by its
//! shadow alone would be depth that disappears when the user switches theme. The dark-scheme alpha
//! is higher only so the shadow is not lost entirely; the tone remains the primary cue there.
//!
//! **One shadow per level.** The rendering stack exposes a single shadow per widget (research R1),
//! so Material's separate key and ambient shadows are folded into one: the key shadow's offset,
//! with the blur widened to stand in for the ambient spread.

/// Which surface role a level resolves to. Named rather than holding an `Rgb`, because a level is
/// scheme-independent — the same level reads a different colour in light and dark, which is exactly
/// what makes elevation survive a theme switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRole {
    Surface,
    SurfaceContainerLow,
    SurfaceContainer,
    SurfaceContainerHigh,
    SurfaceContainerHighest,
}

/// One drop shadow, drawn in the `shadow` role (black) at the stated alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset_y: f32,
    pub blur: f32,
    pub alpha_light: f32,
    pub alpha_dark: f32,
}

/// One elevation level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub level: u8,
    pub surface: SurfaceRole,
    /// `None` at level 0 only — the resting surface casts no shadow.
    pub shadow: Option<Shadow>,
}

const fn shadow(offset_y: f32, blur: f32) -> Option<Shadow> {
    Some(Shadow {
        offset_y,
        blur,
        alpha_light: 0.30,
        alpha_dark: 0.45,
    })
}

/// The six levels, in order (contract §4).
pub const LEVELS: [Level; 6] = [
    Level {
        level: 0,
        surface: SurfaceRole::Surface,
        shadow: None,
    },
    Level {
        level: 1,
        surface: SurfaceRole::SurfaceContainerLow,
        shadow: shadow(1.0, 4.0),
    },
    Level {
        level: 2,
        surface: SurfaceRole::SurfaceContainer,
        shadow: shadow(2.0, 7.0),
    },
    Level {
        level: 3,
        surface: SurfaceRole::SurfaceContainerHigh,
        shadow: shadow(4.0, 10.0),
    },
    Level {
        level: 4,
        surface: SurfaceRole::SurfaceContainerHigh,
        shadow: shadow(6.0, 12.0),
    },
    Level {
        level: 5,
        surface: SurfaceRole::SurfaceContainerHighest,
        shadow: shadow(8.0, 15.0),
    },
];

/// Modal surfaces draw `scrim` at this alpha over everything beneath them (contract §4).
pub const SCRIM_ALPHA: f32 = 0.32;

// --- level assignment (contract §4) ------------------------------------------------------------
//
// Named so a call site says what a surface *is* rather than which number it picked. Every one of
// these replaces a 1px outline that feature 003's contract used to fake depth.

/// Window background and page content.
pub const PAGE: u8 = 0;
/// The app bar at rest.
pub const APP_BAR_REST: u8 = 0;
/// Cards and the sidebar panel.
pub const CARD: u8 = 1;
/// The app bar once content is scrolled under it (FR-025a).
pub const APP_BAR_SCROLLED: u8 = 2;
/// Menus, context menus and popovers.
pub const MENU: u8 = 2;
/// Dialogs.
pub const DIALOG: u8 = 3;
/// Snackbars.
pub const SNACKBAR: u8 = 3;

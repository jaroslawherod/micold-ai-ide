//! Theme selection: the user's preference, the OS-reported scheme, and the pure resolution
//! from those two into a concrete [`ColorScheme`].
//!
//! No dependency on iced or on the OS-detection crate — the binary maps `dark_light::Mode`
//! into [`SystemScheme`] at the boundary and calls [`resolve`]. All logic here is pure and
//! unit-tested (Constitution Principle I). See contracts/theme-behavior.md for the truth table.

use serde::{Deserialize, Serialize};

/// A concrete, resolved color scheme. Exactly two values — there is no "unset" here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// The light Material theme.
    Light,
    /// The dark Material theme.
    Dark,
}

/// The user's persisted choice of how the app selects its color scheme (FR-005, FR-007).
///
/// Serialized in snake_case as the durable settings value (contracts/settings-schema.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    /// Track the operating system's light/dark preference (the default).
    #[default]
    FollowSystem,
    /// Always use the light theme, regardless of the OS.
    Light,
    /// Always use the dark theme, regardless of the OS.
    Dark,
}

/// What the operating system reports, including "no preference / undetectable".
///
/// Mirrors `dark_light::Mode` without depending on that crate; the binary performs the
/// mapping (`Dark→Dark`, `Light→Light`, `Unspecified→Unspecified`, and any detection
/// `Err→Unspecified`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SystemScheme {
    /// The OS prefers a light appearance.
    Light,
    /// The OS prefers a dark appearance.
    Dark,
    /// The OS reports no preference, or detection is unavailable.
    #[default]
    Unspecified,
}

/// Resolve the rendered [`ColorScheme`] from the user's preference and the OS scheme.
///
/// A fixed `Light`/`Dark` preference ignores the OS. `FollowSystem` tracks the OS, falling
/// back to `Light` when the OS preference is unavailable (FR-018).
pub fn resolve(pref: ThemePreference, system: SystemScheme) -> ColorScheme {
    match pref {
        ThemePreference::Light => ColorScheme::Light,
        ThemePreference::Dark => ColorScheme::Dark,
        ThemePreference::FollowSystem => match system {
            SystemScheme::Light => ColorScheme::Light,
            SystemScheme::Dark => ColorScheme::Dark,
            SystemScheme::Unspecified => ColorScheme::Light,
        },
    }
}

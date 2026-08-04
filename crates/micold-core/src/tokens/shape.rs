//! The shape scale (feature 018, T000i — FR-018, FR-019; contract §3).
//!
//! Seven sizes, superseding feature 003's four radii. Three are new (`none`, `extra_small`,
//! `large`) and two existing assignments move: buttons go from `small` (8) to `full`, and dialogs
//! from 16 to `extra_large` (28).
//!
//! # Why the old names are still here
//!
//! Phase 0 authors token *values*; retargeting call sites onto them is Phase 1's work. Removing
//! [`SM`]/[`MD`]/[`LG`] now would force every call site to move in the same change that introduced
//! the scale, which is exactly the coupling feature 017 separated out — a value change and a call-site
//! change reviewed together are a change nobody can review. They are retained, marked superseded,
//! and removed once the last call site has moved.

// --- the Material 3 shape scale (contract §3) --------------------------------------------------

/// Full-bleed regions, the terminal grid.
pub const NONE: f32 = 0.0;
/// Menus, context menus, snackbars, filled text-field top corners.
pub const EXTRA_SMALL: f32 = 4.0;
/// Small containers, tooltips.
pub const SMALL: f32 = 8.0;
/// Cards, list surfaces, popovers.
pub const MEDIUM: f32 = 12.0;
/// Large containers, the sidebar panel.
pub const LARGE: f32 = 16.0;
/// Dialogs.
pub const EXTRA_LARGE: f32 = 28.0;
/// Buttons, chips, tags, icon buttons — the pill.
pub const FULL: f32 = 9999.0;

/// The whole scale, ascending. Asserted complete and ordered by `tests/tokens_scales.rs`.
pub const ALL: [f32; 7] = [NONE, EXTRA_SMALL, SMALL, MEDIUM, LARGE, EXTRA_LARGE, FULL];

// --- superseded (feature 003) ------------------------------------------------------------------

/// Superseded by [`SMALL`]; buttons move to [`FULL`] in Phase 1 (FR-019).
pub const SM: f32 = 8.0;
/// Superseded by [`MEDIUM`].
pub const MD: f32 = 12.0;
/// Superseded by [`LARGE`]; dialogs move to [`EXTRA_LARGE`] in Phase 1 (FR-018).
pub const LG: f32 = 16.0;

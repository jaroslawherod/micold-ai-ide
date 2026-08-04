//! The shape scale (feature 018, T000i — FR-018, FR-019; contract §3).
//!
//! Seven sizes, superseding feature 003's four radii. Three are new (`none`, `extra_small`,
//! `large`) and two existing assignments move: buttons go from `small` (8) to `full`, and dialogs
//! from 16 to `extra_large` (28).
//!
//! Feature 003's `SM`/`MD`/`LG` were carried alongside this scale through Phase 0 so that authoring
//! the values and retargeting the call sites stayed two reviewable changes. Phase 1 moved the last
//! call site, so they are gone.

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

//! The density scale (feature 018, T000d/T000i — FR-026b, FR-026c; contract §7.2).
//!
//! Density is a **theming axis applied uniformly**, not a per-component enumeration. One scale of
//! four steps, one step size, and every component that has a height honours it.
//!
//! That uniformity is the whole requirement. The moment one list defines its own "compact" variant,
//! density stops being something you can reason about globally and becomes a scattering of
//! independent decisions that drift apart — which is what FR-026b forbids and what
//! `tests/tokens_scales.rs` asserts rather than trusts.
//!
//! The sidebar's deliberate compactness is an *application* of this scale ([`DENSE`], i.e. −3 on
//! the standard row), not a bespoke variant (FR-026c).

/// The four steps. A fifth would mean some component had invented its own.
pub const STEPS: [i8; 4] = [0, -1, -2, -3];

/// How much each step below 0 subtracts, in dp.
pub const STEP_DP: f32 = 4.0;

/// The standard density.
pub const STANDARD: i8 = 0;
/// The dense density — the worktree sidebar tree (FR-026c). 48 − 3×4 = 36dp, matching §7.2.
pub const DENSE: i8 = -3;

// --- base heights, in dp (contract §7.2, §7.3, §7.5, §7.7) -------------------------------------
//
// All multiples of 4, so every step of the scale lands on a whole dp — `tests/tokens_scales.rs`
// asserts no component resolves to a fractional height, because a half-pixel row renders blurred.

/// List and tree rows.
pub const LIST_ROW_BASE: f32 = 48.0;
/// Menu and context-menu items.
pub const MENU_ITEM_BASE: f32 = 48.0;
/// Text fields and the select control.
pub const TEXT_FIELD_BASE: f32 = 56.0;
/// Buttons.
pub const BUTTON_BASE: f32 = 40.0;

/// A component's height at a density step.
///
/// The one place the arithmetic lives, so no call site can apply density its own way.
pub fn height(base: f32, step: i8) -> f32 {
    base + (step as f32) * STEP_DP
}

//! One table per token family, iterated by every emitter (`css.rs`, `ts.rs`).
//!
//! A list of names written out in each emitter is a second source of truth that drifts the first
//! time a token is added to one and not the other. These are the single declarations of what each
//! family contains and what each entry is called; an emitter only decides how to spell them.

use super::motion::{self, Easing};
use super::{shape, spacing, state, Rgb, Roles};

/// Every field of [`Roles`], paired with its own identifier.
///
/// Spelled out rather than derived: a macro over the struct would emit whatever fields happen to
/// exist, so adding a private helper field would silently add a public CSS variable. This list is
/// the emitter's declaration of what the site is allowed to see.
pub fn color_roles(r: &Roles) -> [(&'static str, Rgb); 46] {
    [
        ("primary", r.primary),
        ("on_primary", r.on_primary),
        ("primary_container", r.primary_container),
        ("on_primary_container", r.on_primary_container),
        ("secondary", r.secondary),
        ("on_secondary", r.on_secondary),
        ("secondary_container", r.secondary_container),
        ("on_secondary_container", r.on_secondary_container),
        ("tertiary", r.tertiary),
        ("on_tertiary", r.on_tertiary),
        ("tertiary_container", r.tertiary_container),
        ("on_tertiary_container", r.on_tertiary_container),
        ("error", r.error),
        ("on_error", r.on_error),
        ("error_container", r.error_container),
        ("on_error_container", r.on_error_container),
        ("background", r.background),
        ("on_background", r.on_background),
        ("surface", r.surface),
        ("on_surface", r.on_surface),
        ("surface_dim", r.surface_dim),
        ("surface_bright", r.surface_bright),
        ("surface_container_lowest", r.surface_container_lowest),
        ("surface_container_low", r.surface_container_low),
        ("surface_container", r.surface_container),
        ("surface_container_high", r.surface_container_high),
        ("surface_container_highest", r.surface_container_highest),
        ("surface_variant", r.surface_variant),
        ("on_surface_variant", r.on_surface_variant),
        ("outline", r.outline),
        ("outline_variant", r.outline_variant),
        ("inverse_surface", r.inverse_surface),
        ("inverse_on_surface", r.inverse_on_surface),
        ("inverse_primary", r.inverse_primary),
        ("scrim", r.scrim),
        ("shadow", r.shadow),
        ("diff_added", r.diff_added),
        ("diff_removed", r.diff_removed),
        ("warning", r.warning),
        ("on_warning", r.on_warning),
        ("warning_container", r.warning_container),
        ("on_warning_container", r.on_warning_container),
        ("success", r.success),
        ("on_success", r.on_success),
        ("success_container", r.success_container),
        ("on_success_container", r.on_success_container),
    ]
}

/// The corner-radius scale, listed once: the emitters and their tests all iterate it.
pub const CORNER_RADII: [(&str, f32); 7] = [
    ("none", shape::NONE),
    ("extra-small", shape::EXTRA_SMALL),
    ("small", shape::SMALL),
    ("medium", shape::MEDIUM),
    ("large", shape::LARGE),
    ("extra-large", shape::EXTRA_LARGE),
    ("full", shape::FULL),
];

/// Motion durations in milliseconds.
pub const DURATIONS: [(&str, u64); 12] = [
    ("short-1", motion::duration::SHORT_1),
    ("short-2", motion::duration::SHORT_2),
    ("short-3", motion::duration::SHORT_3),
    ("short-4", motion::duration::SHORT_4),
    ("medium-1", motion::duration::MEDIUM_1),
    ("medium-2", motion::duration::MEDIUM_2),
    ("medium-3", motion::duration::MEDIUM_3),
    ("medium-4", motion::duration::MEDIUM_4),
    ("long-1", motion::duration::LONG_1),
    ("long-2", motion::duration::LONG_2),
    ("long-3", motion::duration::LONG_3),
    ("long-4", motion::duration::LONG_4),
];

/// Motion easing curves.
pub const EASINGS: [(&str, Easing); 6] = [
    ("standard", motion::STANDARD),
    ("standard-accelerate", motion::STANDARD_ACCELERATE),
    ("standard-decelerate", motion::STANDARD_DECELERATE),
    ("emphasized", motion::EMPHASIZED),
    ("emphasized-accelerate", motion::EMPHASIZED_ACCELERATE),
    ("emphasized-decelerate", motion::EMPHASIZED_DECELERATE),
];

/// State-layer opacities (the focus ring width is a separate scalar, `state::FOCUS_RING_WIDTH`).
pub const STATES: [(&str, f32); 7] = [
    ("hover", state::HOVER),
    ("focus", state::FOCUS),
    ("pressed", state::PRESSED),
    ("dragged", state::DRAGGED),
    ("selected", state::SELECTED),
    ("disabled-content", state::DISABLED_CONTENT),
    ("disabled-container", state::DISABLED_CONTAINER),
];

/// The spacing scale, in logical pixels.
pub const SPACING: [(&str, f32); 5] = [
    ("xs", spacing::XS),
    ("sm", spacing::SM),
    ("md", spacing::MD),
    ("lg", spacing::LG),
    ("xl", spacing::XL),
];

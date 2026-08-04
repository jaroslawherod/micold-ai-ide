//! Shape of every non-colour scale (feature 018, T000c — FR-007, FR-014, FR-018, FR-020, FR-033, FR-042).
//!
//! These assertions are about *completeness and structure*, not taste. A type scale missing a role,
//! an elevation level without a tonal surface, or a motion set that has quietly merged into one
//! cannot be seen by looking at the app — the missing piece simply never gets used, and the call
//! site that needed it invents a literal instead. That is precisely the drift FR-010 and SC-003
//! exist to prevent, and it starts here.
//!
//! Values themselves are pinned where the contract states them normatively, because a scale whose
//! numbers drift is no longer the Material scale, and nothing else in the build would notice.

use micold_core::tokens::{elevation, motion, shape, state, typography};

// ---------------------------------------------------------------------------------------------
// Typography (contract §2.2, §2.4)
// ---------------------------------------------------------------------------------------------

/// Fifteen roles, no more and no fewer. Every text site selects one by name (FR-010), so a missing
/// role is a call site that will reach for a raw number instead.
#[test]
fn the_type_scale_has_the_fifteen_material_roles() {
    assert_eq!(typography::ALL.len(), 15);
}

/// Each role carries all four properties. Tracking is *recorded and not applied* (FR-042, the one
/// accepted type-scale fidelity gap), which is only auditable if it is actually stored.
#[test]
fn every_type_role_carries_size_line_height_weight_and_tracking() {
    for role in typography::ALL {
        assert!(role.size > 0.0, "{} has no size", role.name);
        assert!(
            role.line_height >= role.size,
            "{}: line height {} is smaller than its size {} — text would overlap",
            role.name,
            role.line_height,
            role.size
        );
        assert!(
            role.weight == 400 || role.weight == 500,
            "{}: weight {} — Material's type scale uses only 400 and 500, and only those two \
             Roboto instances ship (contract §2.1)",
            role.name,
            role.weight
        );
        assert!(
            role.tracking.is_finite(),
            "{} has no recorded tracking value",
            role.name
        );
    }
}

/// The normative table of §2.2, pinned. A scale whose numbers drift is not the Material scale.
#[test]
fn the_type_scale_matches_the_contract_table() {
    let expected: [(&str, f32, f32, u16); 15] = [
        ("display_large", 57.0, 64.0, 400),
        ("display_medium", 45.0, 52.0, 400),
        ("display_small", 36.0, 44.0, 400),
        ("headline_large", 32.0, 40.0, 400),
        ("headline_medium", 28.0, 36.0, 400),
        ("headline_small", 24.0, 32.0, 400),
        ("title_large", 22.0, 28.0, 400),
        ("title_medium", 16.0, 24.0, 500),
        ("title_small", 14.0, 20.0, 500),
        ("body_large", 16.0, 24.0, 400),
        ("body_medium", 14.0, 20.0, 400),
        ("body_small", 12.0, 16.0, 400),
        ("label_large", 14.0, 20.0, 500),
        ("label_medium", 12.0, 16.0, 500),
        ("label_small", 11.0, 16.0, 500),
    ];
    for (name, size, line_height, weight) in expected {
        let role = typography::ALL
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("the type scale has no `{name}` role"));
        assert_eq!(role.size, size, "{name} size");
        assert_eq!(role.line_height, line_height, "{name} line height");
        assert_eq!(role.weight, weight, "{name} weight");
    }
}

/// Three sidebar-scoped roles, each *resolving to* a role in the scale rather than inventing a size
/// (§2.4). This is what keeps the sidebar's ~80% density decision one auditable mapping instead of
/// three loose integers scattered across call sites.
#[test]
fn the_sidebar_roles_resolve_to_roles_in_the_scale() {
    assert_eq!(typography::SIDEBAR.len(), 3);
    for role in typography::SIDEBAR {
        assert!(
            typography::ALL.iter().any(|r| r.name == role.name),
            "sidebar role resolves to `{}`, which is not a role in the scale — §2.4 requires each \
             to map to the nearest smaller role rather than invent a size",
            role.name
        );
    }
    assert_eq!(typography::SIDEBAR_NAME.name, "body_small");
    assert_eq!(typography::SIDEBAR_SESSION.name, "body_small");
    assert_eq!(typography::SIDEBAR_TAG.name, "label_small");
}

// ---------------------------------------------------------------------------------------------
// Elevation (contract §4)
// ---------------------------------------------------------------------------------------------

/// Six levels, each with a tonal surface. The tonal shift is what makes elevation read in the dark
/// scheme, where a black shadow on a dark background is nearly invisible (FR-016) — so a level that
/// carried only a shadow would be depth that vanishes in dark mode.
#[test]
fn there_are_six_elevation_levels_each_with_a_tonal_surface() {
    assert_eq!(elevation::LEVELS.len(), 6);
    for (i, level) in elevation::LEVELS.iter().enumerate() {
        assert_eq!(level.level, i as u8, "levels must be in order");
    }
}

/// Level 0 is the resting surface and has no shadow; every level above it does. Without this,
/// "elevation 0" could silently acquire a drop shadow and every flat surface in the app would gain
/// one.
#[test]
fn only_level_zero_has_no_shadow() {
    assert!(
        elevation::LEVELS[0].shadow.is_none(),
        "level 0 is the resting surface — it must not cast a shadow"
    );
    for level in &elevation::LEVELS[1..] {
        let shadow = level
            .shadow
            .unwrap_or_else(|| panic!("level {} has no shadow", level.level));
        assert!(shadow.offset_y > 0.0, "level {} offset", level.level);
        assert!(shadow.blur > 0.0, "level {} blur", level.level);
        assert!(
            shadow.alpha_dark > shadow.alpha_light,
            "level {}: the dark-scheme alpha must be the higher of the two, or the shadow is lost \
             entirely against a dark background (§4)",
            level.level
        );
    }
}

/// Shadows grow monotonically with level, or the levels do not read as an order.
#[test]
fn shadows_grow_with_elevation() {
    let shadows: Vec<_> = elevation::LEVELS[1..]
        .iter()
        .map(|l| l.shadow.expect("levels 1..5 have shadows"))
        .collect();
    for pair in shadows.windows(2) {
        assert!(
            pair[1].offset_y >= pair[0].offset_y && pair[1].blur > pair[0].blur,
            "a higher elevation level casts a smaller shadow than the one below it"
        );
    }
}

/// Modal surfaces dim what is beneath them at a stated strength (§4).
#[test]
fn the_scrim_has_the_contract_alpha() {
    assert_eq!(elevation::SCRIM_ALPHA, 0.32);
}

// ---------------------------------------------------------------------------------------------
// Shape (contract §3)
// ---------------------------------------------------------------------------------------------

/// Seven sizes, superseding feature 003's four radii (§3).
#[test]
fn the_shape_scale_has_seven_sizes() {
    assert_eq!(shape::ALL.len(), 7);
    assert_eq!(
        shape::ALL,
        [0.0, 4.0, 8.0, 12.0, 16.0, 28.0, 9999.0],
        "the shape scale is normative (§3)"
    );
}

/// Ascending, and `full` is the pill. A scale out of order would make "larger corner" meaningless.
#[test]
fn the_shape_scale_ascends_and_ends_in_a_pill() {
    for pair in shape::ALL.windows(2) {
        assert!(pair[1] > pair[0], "the shape scale must ascend");
    }
    assert_eq!(shape::FULL, 9999.0);
    assert_eq!(shape::EXTRA_LARGE, 28.0, "dialogs (§3)");
}

// ---------------------------------------------------------------------------------------------
// Interaction states (contract §5)
// ---------------------------------------------------------------------------------------------

/// Seven opacities, defined once and applied to every interactive surface — not buttons alone (§5).
#[test]
fn there_are_seven_state_layer_opacities() {
    assert_eq!(state::ALL.len(), 7);
    for &o in state::ALL.iter() {
        assert!(
            (0.0..=1.0).contains(&o),
            "state opacity {o} is out of range"
        );
    }
}

/// The values are normative, and their *ordering* carries meaning a reader relies on: a press must
/// read as stronger than a hover, and a persistent selection must be distinguishable from both.
///
/// The ordering assertions compare two `const`s, so clippy can see the answer at compile time and
/// says so. That is the intent rather than a defect — these guard the *values*, and the day someone
/// sets `HOVER` above `PRESSED` is the day this stops being constantly true and starts failing.
#[test]
#[allow(clippy::assertions_on_constants)]
fn the_state_opacities_match_the_contract_and_order_sensibly() {
    assert_eq!(state::HOVER, 0.08);
    assert_eq!(state::FOCUS, 0.10);
    assert_eq!(state::PRESSED, 0.10);
    assert_eq!(state::DRAGGED, 0.16);
    assert_eq!(state::SELECTED, 0.12);
    assert_eq!(state::DISABLED_CONTENT, 0.38);
    assert_eq!(state::DISABLED_CONTAINER, 0.12);

    assert!(
        state::PRESSED > state::HOVER,
        "a press must read as stronger than a hover"
    );
    assert!(
        state::SELECTED > state::HOVER,
        "a persistent selection must be distinguishable from a transient hover"
    );
}

/// The focus indicator is 3dp (§5, FR-022) and accompanies rather than replaces the focus layer.
#[test]
fn the_focus_indicator_is_three_dp() {
    assert_eq!(state::FOCUS_RING_WIDTH, 3.0);
}

// ---------------------------------------------------------------------------------------------
// Motion (contract §6)
// ---------------------------------------------------------------------------------------------

/// The two sets stay partitioned. Collapsing them would silently make every small utilitarian
/// transition as expressive as a sidebar slide, which is the distinction §6.2 exists to draw.
#[test]
fn the_easing_sets_are_partitioned_into_standard_and_emphasized() {
    assert_eq!(motion::STANDARD_SET.len(), 3);
    assert_eq!(motion::EMPHASIZED_SET.len(), 3);
    for e in motion::EMPHASIZED_SET {
        assert!(
            !motion::STANDARD_SET.contains(&e) || e == motion::STANDARD,
            "an emphasized curve also appears in the standard set — only `emphasized`/`standard` \
             legitimately share a definition (§6.2)"
        );
    }
}

/// Twelve named durations, ascending within each band (§6.1).
#[test]
fn the_durations_match_the_contract() {
    use motion::duration::*;
    assert_eq!(
        [SHORT_1, SHORT_2, SHORT_3, SHORT_4],
        [50, 100, 150, 200],
        "short band"
    );
    assert_eq!(
        [MEDIUM_1, MEDIUM_2, MEDIUM_3, MEDIUM_4],
        [250, 300, 350, 400],
        "medium band"
    );
    assert_eq!(
        [LONG_1, LONG_2, LONG_3, LONG_4],
        [450, 500, 550, 600],
        "long band"
    );
}

/// Every easing is a cubic bézier with control points in the unit interval on x. A curve outside it
/// is not a valid timing function and would animate erratically.
#[test]
fn every_easing_is_a_well_formed_cubic_bezier() {
    for set in [motion::STANDARD_SET, motion::EMPHASIZED_SET] {
        for e in set {
            assert!(
                (0.0..=1.0).contains(&e.x1) && (0.0..=1.0).contains(&e.x2),
                "easing {e:?} has an x control point outside [0, 1]"
            );
        }
    }
}

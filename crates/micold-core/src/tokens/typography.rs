//! The Material 3 type scale (feature 018, T000h — FR-007, FR-011, FR-042; contract §2.2, §2.4).
//!
//! Fifteen roles, each carrying size, line height, weight and tracking. Every text site selects one
//! **by name** (FR-010) rather than stating a number, which is what keeps the scale a scale: a raw
//! `14.0` at a call site is invisible to every check here and drifts on its own.
//!
//! # Tracking is recorded, not applied
//!
//! The rendering stack cannot express letter-spacing. [`TypeRole::tracking`] carries Material's
//! value anyway so the gap is explicit and auditable rather than silently absent — this is the one
//! accepted type-scale fidelity gap (FR-042), and recording it is what makes it a known gap instead
//! of an oversight.

/// One role in the type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeRole {
    /// The role's name, as the contract spells it. Carried so a call site, a test failure and the
    /// contract table can all be matched up by eye.
    pub name: &'static str,
    /// Size in dp.
    pub size: f32,
    /// Line height in dp. Always at least [`size`](Self::size) — multi-line text uses this rather
    /// than the renderer's default spacing (FR-007).
    pub line_height: f32,
    /// CSS numeric weight. Only 400 and 500 occur: they are the only weights the Material 3 scale
    /// specifies, and the only two Roboto instances that ship (contract §2.1).
    pub weight: u16,
    /// Material's tracking, in dp. **Recorded, never applied** — see the module docs (FR-042).
    pub tracking: f32,
}

const fn role(
    name: &'static str,
    size: f32,
    line_height: f32,
    weight: u16,
    tracking: f32,
) -> TypeRole {
    TypeRole {
        name,
        size,
        line_height,
        weight,
        tracking,
    }
}

pub const DISPLAY_LARGE: TypeRole = role("display_large", 57.0, 64.0, 400, -0.25);
pub const DISPLAY_MEDIUM: TypeRole = role("display_medium", 45.0, 52.0, 400, 0.0);
pub const DISPLAY_SMALL: TypeRole = role("display_small", 36.0, 44.0, 400, 0.0);

pub const HEADLINE_LARGE: TypeRole = role("headline_large", 32.0, 40.0, 400, 0.0);
pub const HEADLINE_MEDIUM: TypeRole = role("headline_medium", 28.0, 36.0, 400, 0.0);
pub const HEADLINE_SMALL: TypeRole = role("headline_small", 24.0, 32.0, 400, 0.0);

pub const TITLE_LARGE: TypeRole = role("title_large", 22.0, 28.0, 400, 0.0);
pub const TITLE_MEDIUM: TypeRole = role("title_medium", 16.0, 24.0, 500, 0.15);
pub const TITLE_SMALL: TypeRole = role("title_small", 14.0, 20.0, 500, 0.10);

pub const BODY_LARGE: TypeRole = role("body_large", 16.0, 24.0, 400, 0.50);
pub const BODY_MEDIUM: TypeRole = role("body_medium", 14.0, 20.0, 400, 0.25);
pub const BODY_SMALL: TypeRole = role("body_small", 12.0, 16.0, 400, 0.40);

pub const LABEL_LARGE: TypeRole = role("label_large", 14.0, 20.0, 500, 0.10);
pub const LABEL_MEDIUM: TypeRole = role("label_medium", 12.0, 16.0, 500, 0.50);
pub const LABEL_SMALL: TypeRole = role("label_small", 11.0, 16.0, 500, 0.50);

/// The whole scale, for tests and for any call site that needs to enumerate it.
pub const ALL: [TypeRole; 15] = [
    DISPLAY_LARGE,
    DISPLAY_MEDIUM,
    DISPLAY_SMALL,
    HEADLINE_LARGE,
    HEADLINE_MEDIUM,
    HEADLINE_SMALL,
    TITLE_LARGE,
    TITLE_MEDIUM,
    TITLE_SMALL,
    BODY_LARGE,
    BODY_MEDIUM,
    BODY_SMALL,
    LABEL_LARGE,
    LABEL_MEDIUM,
    LABEL_SMALL,
];

// --- sidebar-scoped roles (contract §2.4, FR-011) ----------------------------------------------
//
// The worktree sidebar's deliberate ~80% density reduction survives as explicit, named roles rather
// than as an implicit re-derivation at call sites — and, importantly, rather than being silently
// lost when the type scale was re-anchored. Each *resolves to* a role already in the scale instead
// of inventing a size, so the density decision is one auditable mapping table.

/// Worktree display name — `body_small` (12/16 ≈ 80% of `body_medium`'s 14).
pub const SIDEBAR_NAME: TypeRole = BODY_SMALL;
/// Session label — the same role as the worktree name it nests under.
pub const SIDEBAR_SESSION: TypeRole = BODY_SMALL;
/// Tag chip text — `label_small` (11/16 ≈ 80% of `label_medium`'s 12).
pub const SIDEBAR_TAG: TypeRole = LABEL_SMALL;

/// The sidebar mapping, for the test that asserts each entry resolves into [`ALL`].
pub const SIDEBAR: [TypeRole; 3] = [SIDEBAR_NAME, SIDEBAR_SESSION, SIDEBAR_TAG];

//! Interaction state layers (feature 018, T000i — FR-020…FR-024; contract §5).
//!
//! A state layer is the content colour composited over the container at the stated opacity. Defined
//! **once** and applied to every interactive surface — list rows, tree items, menu items, chips,
//! tags, every button variant, text fields and the select control — not to buttons alone. That
//! breadth is the requirement: a hover that only some things respond to reads as a bug, not as a
//! design.

/// Pointer over the element.
pub const HOVER: f32 = 0.08;
/// Keyboard focus. Accompanies, and does not replace, the focus indicator below.
pub const FOCUS: f32 = 0.10;
/// Held down.
pub const PRESSED: f32 = 0.10;
/// Being dragged.
pub const DRAGGED: f32 = 0.16;
/// Persistent selection. Distinct from [`HOVER`], and composable with it.
pub const SELECTED: f32 = 0.12;
/// Applied to text and icons when disabled.
pub const DISABLED_CONTENT: f32 = 0.38;
/// Applied to the container fill when disabled.
pub const DISABLED_CONTAINER: f32 = 0.12;

/// Every opacity, for the test that asserts the set is complete.
pub const ALL: [f32; 7] = [
    HOVER,
    FOCUS,
    PRESSED,
    DRAGGED,
    SELECTED,
    DISABLED_CONTENT,
    DISABLED_CONTAINER,
];

/// The focus indicator's width in dp (FR-022): a `secondary` outline drawn at the element's own
/// shape radius, in addition to the [`FOCUS`] state layer.
///
/// Reachable only on text fields and the select control. Buttons, rows, menu items and chips cannot
/// hold focus in this rendering stack — accepted fidelity gap #2 (FR-043), which is why [`FOCUS`]
/// applies more narrowly than the other opacities here.
pub const FOCUS_RING_WIDTH: f32 = 3.0;

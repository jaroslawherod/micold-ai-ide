//! Motion tokens (feature 018, T000i — FR-033, FR-034, FR-035; contract §6).
//!
//! Named durations and easing curves, split into two sets. The split is the substance: the
//! **standard** set is for small utilitarian transitions, the **emphasized** set for larger, more
//! expressive ones. Collapsing them would make a menu fade as ceremonious as a sidebar slide, which
//! is precisely the distinction a user reads as "this feels like Material".
//!
//! Every animation already in the app keeps its trigger, start state and end state; only duration
//! and easing change (FR-035). Assignment lives in contract §6.3.

/// Durations in milliseconds (contract §6.1).
pub mod duration {
    pub const SHORT_1: u64 = 50;
    pub const SHORT_2: u64 = 100;
    pub const SHORT_3: u64 = 150;
    pub const SHORT_4: u64 = 200;

    pub const MEDIUM_1: u64 = 250;
    pub const MEDIUM_2: u64 = 300;
    pub const MEDIUM_3: u64 = 350;
    pub const MEDIUM_4: u64 = 400;

    pub const LONG_1: u64 = 450;
    pub const LONG_2: u64 = 500;
    pub const LONG_3: u64 = 550;
    pub const LONG_4: u64 = 600;
}

/// A cubic bézier timing function, as its two control points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Easing {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

const fn easing(x1: f32, y1: f32, x2: f32, y2: f32) -> Easing {
    Easing { x1, y1, x2, y2 }
}

// --- standard set: small, utilitarian transitions (contract §6.2) ------------------------------

pub const STANDARD: Easing = easing(0.2, 0.0, 0.0, 1.0);
pub const STANDARD_ACCELERATE: Easing = easing(0.3, 0.0, 1.0, 1.0);
pub const STANDARD_DECELERATE: Easing = easing(0.0, 0.0, 0.0, 1.0);

// --- emphasized set: larger, more expressive transitions ---------------------------------------

/// Shares its definition with [`STANDARD`] — Material defines them identically, and the sets are
/// distinguished by *what they are applied to* rather than by every curve differing.
pub const EMPHASIZED: Easing = easing(0.2, 0.0, 0.0, 1.0);
pub const EMPHASIZED_ACCELERATE: Easing = easing(0.3, 0.0, 0.8, 0.15);
pub const EMPHASIZED_DECELERATE: Easing = easing(0.05, 0.7, 0.1, 1.0);

pub const STANDARD_SET: [Easing; 3] = [STANDARD, STANDARD_ACCELERATE, STANDARD_DECELERATE];
pub const EMPHASIZED_SET: [Easing; 3] = [EMPHASIZED, EMPHASIZED_ACCELERATE, EMPHASIZED_DECELERATE];

//! Components own their presentation state (feature 017, T037 — FR-011, FR-025).
//!
//! Every animated element in the application used to be a variant of one global enumeration,
//! driven by one of two central animators. Adding an animated element meant editing an enum in
//! `ui/mod.rs`, allocating an identity, threading a progress value down through the view, and
//! remembering to clean the track up. Per-row hover fades needed an identity the caller had to
//! invent, so rows were keyed by a *hash of their name* — two worktrees whose names collided
//! would have animated as one.
//!
//! The fix is that a component holds its own progress, in the widget tree, where the renderer
//! already keeps per-instance state. Two instances cannot interfere because neither can see the
//! other, and a removed widget drops its state — so "nothing animates after an element
//! disappears" is structural rather than policed.
//!
//! These tests exercise the primitive that makes that true, [`cdk::motion::Progress`], because
//! that is where the property lives. A test that drove real widgets through a renderer would be
//! testing iced's tree diffing, not this.

use micold_client::ui::cdk::motion::Progress;

/// Advance a track to completion, returning how many frames it took. Guards against a
/// non-converging step turning a failure into a hang.
fn run_to_rest(p: &mut Progress, target: f32, speed: f32) -> usize {
    for frame in 1..=10_000 {
        p.advance_to(target, speed);
        if !p.animating() {
            return frame;
        }
    }
    panic!("progress never came to rest — a step that does not converge would hang the render loop");
}

/// The headline property: two instances are independent. This is what the hashed row-identity
/// scheme could not guarantee, since two rows could hash to the same track.
#[test]
fn two_instances_animate_independently() {
    let mut a = Progress::new(0.0);
    let mut b = Progress::new(0.0);

    // Drive only `a`.
    for _ in 0..3 {
        a.advance_to(1.0, 0.25);
    }

    assert!(a.value() > 0.0, "a should have moved");
    assert_eq!(b.value(), 0.0, "b must not have moved — it was never advanced");

    // And now only `b`, faster. Neither observes the other.
    for _ in 0..2 {
        b.advance_to(1.0, 0.5);
    }
    assert!(
        (b.value() - a.value()).abs() > f32::EPSILON,
        "instances at different speeds must hold different values (a={}, b={})",
        a.value(),
        b.value()
    );
}

/// A component that is removed takes its state with it (FR-025).
///
/// The property is *structural*, and stating it precisely matters: a `Progress` is a plain value
/// with no registry behind it, so there is no central map for a departed widget's track to linger
/// in and nothing to clean up. That is the whole difference from the arrangement it replaces,
/// where a track keyed by a hashed row identity outlived the row unless someone remembered to
/// remove it.
///
/// What can be asserted here is the consequence: a new instance is unaffected by any that came
/// before it. That a widget's tree state is dropped with the widget is iced's contract, not this
/// crate's, and testing it here would be testing tree diffing.
#[test]
fn a_new_instance_inherits_nothing_from_a_departed_one() {
    let mut departed = Progress::new(0.0);
    departed.advance_to(1.0, 0.25);
    assert!(departed.animating(), "precondition: it is mid-flight");

    let fresh = Progress::new(0.0);
    assert_eq!(
        fresh.value(),
        0.0,
        "a new instance starts from its own initial value"
    );
    assert!(
        !fresh.animating(),
        "and at rest — so a component that reappears does not resume a transition it never began"
    );
}

/// Quiescence (FR-025, SC-008): once a track reaches its target it stops asking for frames. An
/// animation that never says it is finished holds the render loop awake forever.
#[test]
fn a_track_stops_asking_for_frames_once_it_arrives() {
    let mut p = Progress::new(0.0);
    let frames = run_to_rest(&mut p, 1.0, 0.25);

    assert_eq!(p.value(), 1.0, "must land exactly on the target, not near it");
    assert!(!p.animating(), "must be quiescent at rest");
    assert!(
        frames <= 8,
        "0→1 at 0.25/frame should take ~4 frames, took {frames}"
    );

    // Still quiescent after further advances toward the same target.
    p.advance_to(1.0, 0.25);
    assert!(!p.animating());
    assert_eq!(p.value(), 1.0);
}

/// It must never overshoot: a scrim alpha above 1.0 or below 0.0 renders as a visible flash.
#[test]
fn a_track_never_overshoots_its_target() {
    let mut p = Progress::new(0.0);
    // A step far larger than the distance.
    p.advance_to(1.0, 10.0);
    assert_eq!(p.value(), 1.0);

    let mut q = Progress::new(1.0);
    q.advance_to(0.0, 10.0);
    assert_eq!(q.value(), 0.0);
}

/// Reversing mid-flight is the common case — hover away before the fade-in finishes — and must
/// pick up from where it is rather than snapping.
#[test]
fn reversing_mid_flight_continues_from_the_current_value() {
    let mut p = Progress::new(0.0);
    p.advance_to(1.0, 0.25);
    p.advance_to(1.0, 0.25);
    let midpoint = p.value();
    assert!(
        midpoint > 0.0 && midpoint < 1.0,
        "precondition: mid-flight, got {midpoint}"
    );

    p.advance_to(0.0, 0.25);
    assert!(
        p.value() < midpoint,
        "reversing must move back from {midpoint}, got {}",
        p.value()
    );
    assert!(p.value() > 0.0, "must not snap to the target");
}

/// A component that starts at its target — a menu built already-open — must not animate into
/// existence, and must not request a frame it does not need.
#[test]
fn starting_at_the_target_animates_nothing() {
    let mut p = Progress::new(1.0);
    assert!(!p.animating());
    p.advance_to(1.0, 0.25);
    assert_eq!(p.value(), 1.0);
    assert!(!p.animating());
}

/// A zero or negative speed would never converge, which would hold the render loop awake for
/// good. It must arrive immediately instead.
#[test]
fn a_degenerate_speed_arrives_rather_than_hanging() {
    for speed in [0.0, -1.0, f32::NAN] {
        let mut p = Progress::new(0.0);
        p.advance_to(1.0, speed);
        assert_eq!(p.value(), 1.0, "speed {speed} must arrive immediately");
        assert!(!p.animating(), "speed {speed} must not keep asking for frames");
    }
}

//! Adding a newly animated element touches only that element (feature 017, T043 — SC-005).
//!
//! SC-005 asks for this to be *demonstrated by adding one*. Doing that by hand — write a component,
//! confirm nothing else needed editing, delete it again — proves it for exactly as long as it takes
//! to read the commit. The demonstration is here instead, as a component that is genuinely new and
//! genuinely animated, so the claim is re-proved on every run.
//!
//! Before this feature, an animated element needed four things it could not provide itself: a
//! variant in the global `MotionKey` enumeration, an entry in the central animator's target table,
//! a progress value threaded down through `ui::view`'s argument list, and a place in the
//! `motion_animating` predicate so the clock knew to keep running. Three of those files had nothing
//! to do with the element being added.
//!
//! What follows needs none of them. It is written against the same public API a real component
//! would use, and the test asserts both halves of the claim: that it animates, and that nothing
//! outside it was required to make that happen.

use std::time::Duration;

use micold_client::ui::cdk::motion::Progress;

/// A newly animated element, invented for this test.
///
/// A "pulse" that ramps to full and back. Deliberately not a wrapper around an existing widget —
/// the point is that a component with a transition nobody anticipated needs no permission from
/// anywhere else to have one. Its entire animation surface is the `Progress` it owns.
struct Pulse {
    progress: Progress,
    lit: bool,
}

impl Pulse {
    /// Resting, dark, and not animating. Note what the constructor does *not* take: no key, no
    /// identity, no handle to a central animator, and no starting progress from a caller.
    fn new() -> Self {
        Self {
            progress: Progress::new(0.0),
            lit: false,
        }
    }

    /// The destination changes; how it gets there is the component's own business.
    fn set(&mut self, lit: bool) {
        self.lit = lit;
    }

    /// One frame. In a real widget this is `Progress::on_frame` inside `Widget::update`, which also
    /// asks the runtime for the next frame while still moving; here the step is applied directly so
    /// the test needs no renderer to drive it.
    fn tick(&mut self) {
        let target = if self.lit { 1.0 } else { 0.0 };
        self.progress
            .advance_to(target, micold_client::ui::cdk::motion::step_for(FULL));
    }

    /// What it would draw with.
    fn value(&self) -> f32 {
        self.progress.value()
    }

    fn animating(&self) -> bool {
        self.progress.animating()
    }
}

/// The pulse's own timing, owned by the pulse. Adding an element does not mean editing a shared
/// table of durations either.
const FULL: Duration = Duration::from_millis(100);

/// Run to rest, returning the frame count. Bounded, so a non-converging step fails rather than
/// hangs.
fn settle(pulse: &mut Pulse) -> usize {
    for frame in 1..=10_000 {
        pulse.tick();
        if !pulse.animating() {
            return frame;
        }
    }
    panic!("the pulse never came to rest");
}

/// Half the claim: it really does animate — it travels, rather than jumping, and then stops.
#[test]
fn the_new_element_animates() {
    let mut pulse = Pulse::new();
    assert_eq!(pulse.value(), 0.0);
    assert!(!pulse.animating(), "a fresh element must start at rest");

    pulse.set(true);
    pulse.tick();
    let after_one_frame = pulse.value();
    assert!(
        after_one_frame > 0.0 && after_one_frame < 1.0,
        "expected a partial value after one frame, got {after_one_frame} — that is a jump, not a \
         transition"
    );

    let frames = settle(&mut pulse);
    assert_eq!(pulse.value(), 1.0);
    assert!(frames > 1, "arriving in a single frame is not an animation");
}

/// It reverses, and from wherever it had got to — not from the far end.
#[test]
fn the_new_element_reverses_from_where_it_is() {
    let mut pulse = Pulse::new();
    pulse.set(true);
    pulse.tick();
    pulse.tick();
    let partway = pulse.value();

    pulse.set(false);
    pulse.tick();
    assert!(
        pulse.value() < partway,
        "reversing must continue from the current value, not restart"
    );
    settle(&mut pulse);
    assert_eq!(pulse.value(), 0.0);
}

/// Two of them are independent, with no identity involved. Under the old scheme this is precisely
/// what went wrong: per-row fades had no natural identity, so rows were keyed by a hash of their
/// name and two colliding names animated as one.
#[test]
fn two_of_them_do_not_interfere() {
    let mut first = Pulse::new();
    let second = Pulse::new();

    first.set(true);
    settle(&mut first);

    assert_eq!(first.value(), 1.0);
    assert_eq!(
        second.value(),
        0.0,
        "the second element moved without being asked — the two share state"
    );
}

/// The other half of SC-005, and the one that would actually regress: adding the element above
/// required editing nothing outside it.
///
/// Asserted against the source, because the failure mode is a *file gaining* something — which no
/// assertion about the code as it stands could notice. The patterns are deliberately
/// declaration-shaped rather than bare names: a registry is something a file *declares* or *holds*,
/// and prose recalling why it was removed is not a regression. `ui/mod.rs` still refers to
/// `MotionKey::Main` in a doc comment explaining what replaced it, and should.
///
/// The component library is covered separately and more strictly, by
/// `component_api_opacity::the_library_never_names_the_global_animation_machinery`, which strips
/// comments and scans every module. This covers the two application files that registry lived in.
#[test]
fn nothing_outside_the_element_had_to_change() {
    let ui = include_str!("../src/ui/mod.rs");
    let app = include_str!("../src/app.rs");

    for (file, source) in [("ui/mod.rs", ui), ("app.rs", app)] {
        for pattern in [
            // The enumeration itself: every animated element used to need a variant here.
            "enum MotionKey",
            // Holding an animator, as a field or a local.
            "Animator<",
            "Animator::new",
            // Threading progress down through the view's arguments.
            "motion: &Animator",
        ] {
            assert!(
                !source.contains(pattern),
                "{file} contains `{pattern}` — a central registry for animated elements is back, \
                 and adding one now means editing a file that has nothing to do with it"
            );
        }
    }
}

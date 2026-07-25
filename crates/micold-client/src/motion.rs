//! Generic, framework-agnostic animation core.
//!
//! An [`Animator`] is a keyed collection of independent animated scalars ([`Track`]s). Each
//! track moves its `value` toward a `target` by a fixed per-tick `speed`, clamped so it never
//! overshoots. Drive it by calling [`Animator::tick`] on a clock; read progress with
//! [`Animator::get`]; gate the clock with [`Animator::animating`].
//!
//! This module has **no dependency on iced or on any application type** — it is generic over
//! the key type `K` and uses only `std` and `f32`. That keeps it unit-testable with
//! `cargo test --no-default-features` (Constitution Principle I) and makes it a self-contained,
//! reusable library that can be lifted into another project unchanged: application-specific
//! concerns (which elements animate, their identifiers, and their timings) live in the
//! consumer, never here.
//!
//! # Example
//!
//! ```
//! use micold_client::motion::Animator;
//!
//! // A consumer defines its own key type; the core imposes no key scheme.
//! #[derive(Clone, Copy, PartialEq, Eq, Hash)]
//! enum Fx { Panel }
//!
//! let mut fx = Animator::new();
//! fx.set(Fx::Panel, 0.0);        // start hidden
//! fx.to(Fx::Panel, 1.0, 0.25);   // animate toward shown, 0.25 per tick
//! while fx.animating() {
//!     fx.tick();                 // advance on each frame of your clock
//! }
//! assert_eq!(fx.get(Fx::Panel), 1.0);
//! ```

use std::collections::HashMap;
use std::hash::Hash;

/// A single animated scalar moving toward a target at a fixed per-tick step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track {
    /// Current progress, conventionally in `[0.0, 1.0]`.
    pub value: f32,
    /// The value `value` is moving toward.
    pub target: f32,
    /// Maximum change applied to `value` per [`Animator::tick`].
    pub speed: f32,
}

/// A keyed collection of independent [`Track`]s. `K` identifies each animated element.
///
/// Generic over the key type so it carries no application-specific assumptions (the reusable,
/// extractable unit).
#[derive(Debug, Clone)]
pub struct Animator<K> {
    tracks: HashMap<K, Track>,
}

impl<K: Copy + Eq + Hash> Animator<K> {
    /// An empty animator.
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
        }
    }

    /// Snap `key` to `value`, settled (`value == target`). No animation — use to initialize a
    /// track before animating it, or to jump a track instantly.
    pub fn set(&mut self, key: K, value: f32) {
        let track = self.tracks.entry(key).or_insert(Track {
            value,
            target: value,
            speed: 0.0,
        });
        track.value = value;
        track.target = value;
    }

    /// Set `key`'s `target` and per-tick `speed`.
    ///
    /// If the key already exists, its current `value` is kept, so it animates from where it is.
    /// If the key is new, it is created **settled at `target`** (`value == target`), so an
    /// element that has never been [`set`](Self::set) never surprise-animates the first time it
    /// is targeted. Deliberate animation from a starting value is `set(key, start)` then
    /// `to(key, target, speed)`.
    pub fn to(&mut self, key: K, target: f32, speed: f32) {
        match self.tracks.get_mut(&key) {
            Some(track) => {
                track.target = target;
                track.speed = speed;
            }
            None => {
                self.tracks.insert(
                    key,
                    Track {
                        value: target,
                        target,
                        speed,
                    },
                );
            }
        }
    }

    /// Current `value` of `key`, or `0.0` if the key was never set (a safe "fully hidden"
    /// default for an element that has never animated).
    pub fn get(&self, key: K) -> f32 {
        self.tracks.get(&key).map(|t| t.value).unwrap_or(0.0)
    }

    /// Advance every track toward its target by its `speed`, clamped so it never overshoots.
    pub fn tick(&mut self) {
        for track in self.tracks.values_mut() {
            track.value = approach(track.value, track.target, track.speed);
        }
    }

    /// True while any track has not yet reached its target. Use it to gate the animation clock
    /// so no work runs at rest.
    pub fn animating(&self) -> bool {
        self.tracks
            .values()
            .any(|t| (t.value - t.target).abs() > f32::EPSILON)
    }
}

impl<K: Copy + Eq + Hash> Default for Animator<K> {
    fn default() -> Self {
        Self::new()
    }
}

/// Move `current` toward `target` by at most `step`, never overshooting.
fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A local key type — proves the core needs no application-specific key scheme (C7).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Key {
        A,
        B,
    }

    /// Run `tick` until settled (bounded, to avoid an infinite loop on a bug).
    fn settle<K: Copy + Eq + Hash>(anim: &mut Animator<K>) -> usize {
        let mut ticks = 0;
        while anim.animating() && ticks < 10_000 {
            anim.tick();
            ticks += 1;
        }
        ticks
    }

    // C1 — converges to target and stops exactly there (no overshoot).
    #[test]
    fn converges_to_target_without_overshoot() {
        let mut anim = Animator::new();
        anim.set(Key::A, 0.0);
        anim.to(Key::A, 1.0, 0.3);
        // Never exceeds the target mid-flight.
        for _ in 0..3 {
            anim.tick();
            assert!(anim.get(Key::A) <= 1.0, "overshot the target");
        }
        settle(&mut anim);
        assert_eq!(anim.get(Key::A), 1.0);
        // Idempotent once settled.
        anim.tick();
        assert_eq!(anim.get(Key::A), 1.0);
    }

    // C1 — also converges downward.
    #[test]
    fn converges_downward() {
        let mut anim = Animator::new();
        anim.set(Key::A, 1.0);
        anim.to(Key::A, 0.0, 0.25);
        settle(&mut anim);
        assert_eq!(anim.get(Key::A), 0.0);
    }

    // C2 — a larger speed converges in fewer ticks than a smaller one.
    #[test]
    fn faster_speed_converges_sooner() {
        let mut fast = Animator::new();
        fast.set(Key::A, 0.0);
        fast.to(Key::A, 1.0, 0.5);

        let mut slow = Animator::new();
        slow.set(Key::A, 0.0);
        slow.to(Key::A, 1.0, 0.1);

        assert!(settle(&mut fast) < settle(&mut slow));
    }

    // C3 — animating() is true mid-flight and false once settled.
    #[test]
    fn animating_flips_true_then_false() {
        let mut anim = Animator::new();
        anim.set(Key::A, 0.0);
        anim.to(Key::A, 1.0, 0.2);
        assert!(anim.animating());
        settle(&mut anim);
        assert!(!anim.animating());
    }

    // C4 — distinct keys advance independently.
    #[test]
    fn keys_animate_independently() {
        let mut anim = Animator::new();
        anim.set(Key::A, 0.0);
        anim.set(Key::B, 0.0);
        anim.to(Key::A, 1.0, 1.0); // settles in one tick
        anim.to(Key::B, 1.0, 0.1); // still moving
        anim.tick();
        assert_eq!(anim.get(Key::A), 1.0);
        assert!(anim.get(Key::B) < 1.0);
        assert!(anim.animating()); // because B is still moving
    }

    // C5 — get on an absent key returns 0.0.
    #[test]
    fn get_absent_key_is_zero() {
        let anim: Animator<Key> = Animator::new();
        assert_eq!(anim.get(Key::A), 0.0);
        assert!(!anim.animating());
    }

    // C6 — `to` on a brand-new key creates it settled at the target (no surprise animation).
    #[test]
    fn to_new_key_is_settled_at_target() {
        let mut anim = Animator::new();
        anim.to(Key::A, 1.0, 0.1);
        assert_eq!(anim.get(Key::A), 1.0);
        assert!(!anim.animating());
    }

    // C7 — the core is usable standalone with only a local key type and std (no app/iced types).
    // Compiling and passing under `cargo test --no-default-features` is the extractability proof.
    #[test]
    fn core_is_usable_standalone() {
        let mut anim: Animator<Key> = Animator::default();
        anim.set(Key::A, 0.25);
        assert_eq!(anim.get(Key::A), 0.25);
    }
}

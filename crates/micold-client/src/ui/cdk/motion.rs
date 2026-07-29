//! Per-instance animation state (feature 017, FR-011, FR-014, FR-025).
//!
//! The application used to hold every animated element's progress centrally: one global
//! enumeration naming them all, and two animators keyed by it. Adding an animated element meant
//! editing that enum, allocating an identity, threading a float down through the view, and
//! remembering to clean the track up afterwards. Per-row fades needed an identity the caller had
//! to invent, so rows were keyed by a *hash of their name* — two worktrees whose names collided
//! would have animated as one.
//!
//! A [`Progress`] is one animated scalar, held by the widget that animates. The renderer already
//! keeps per-instance state in the widget tree, so this needs no store, no identity and no
//! cleanup: two instances cannot interfere because neither can see the other, and a removed
//! widget drops its state — which makes "nothing animates after an element disappears" structural
//! rather than policed.
//!
//! **No appearance.** How *far* a transition has come is behaviour; what it looks like on the way
//! is the material layer's business, and the speed comes from the caller.

use iced::advanced::Shell;
use iced::window;
use iced::Event;
use std::time::{Duration, Instant};

/// The frame interval a per-frame step is derived against (~60fps).
///
/// A track advances by a fixed amount per redraw rather than by elapsed wall-clock time, so a
/// transition's real duration is only nominal: on a slower display it takes proportionally longer.
/// That is exactly what the central animator did — it stepped a fixed amount per 16ms clock tick —
/// so keeping the arithmetic identical is what makes this feature's transitions look unchanged.
/// Moving to elapsed-time interpolation is a visual change, and therefore feature 018's to make.
const FRAME: Duration = Duration::from_millis(16);

/// The per-frame step that carries a track across its full `0.0..=1.0` range in `duration`.
///
/// Timings are stated as durations at the call site because that is what a motion spec is written
/// in; a bare step like `0.14` says nothing about how long anything takes.
pub fn step_for(duration: Duration) -> f32 {
    (FRAME.as_secs_f32() / duration.as_secs_f32()).clamp(f32::EPSILON, 1.0)
}

/// One animated scalar, owned by the widget that animates it.
///
/// Conventionally `0.0` (hidden, idle, collapsed) to `1.0` (shown, highlighted, expanded), though
/// nothing here requires that range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    value: f32,
    target: f32,
    /// The frame this track last advanced on, so it advances exactly once per frame.
    ///
    /// The runtime re-runs `update` with the *same* redraw event when that update invalidated the
    /// layout — which [`Self::on_layout_frame`] does on every moving frame. Without this, such a
    /// transition would step three or four times per frame and run at a multiple of its stated
    /// duration.
    last_frame: Option<Instant>,
}

impl Progress {
    /// A track resting at `initial`. It is not animating: a component built already-open must not
    /// animate into existence.
    pub fn new(initial: f32) -> Self {
        Self {
            value: initial,
            target: initial,
            last_frame: None,
        }
    }

    /// The current value.
    pub fn value(self) -> f32 {
        self.value
    }

    /// Whether the track is still moving — i.e. whether another frame is needed.
    ///
    /// The load-bearing half of idle quiescence (FR-025, SC-008): a track that never says it has
    /// arrived holds the render loop awake for good.
    pub fn animating(self) -> bool {
        (self.value - self.target).abs() > f32::EPSILON
    }

    /// Step toward `target` by at most `speed`, clamped so it never overshoots.
    ///
    /// Overshoot is not a rounding detail — a scrim alpha above 1.0 or below 0.0 renders as a
    /// visible flash at the end of every transition.
    ///
    /// A non-positive or non-finite `speed` would never converge and would hold the render loop
    /// awake, so it arrives immediately instead. Refusing to animate is recoverable; refusing to
    /// stop is not.
    pub fn advance_to(&mut self, target: f32, speed: f32) {
        self.target = target;
        let distance = target - self.value;
        if distance.abs() <= f32::EPSILON {
            self.value = target;
            return;
        }
        if !speed.is_finite() || speed <= 0.0 {
            self.value = target;
            return;
        }
        self.value = if distance.abs() <= speed {
            target
        } else {
            self.value + speed * distance.signum()
        };
    }

    /// Snap to `value` and come to rest there, abandoning whatever was in flight.
    ///
    /// For when a component's *identity* changes under it rather than its target: the main view
    /// showing a different session is not the old view continuing, so it starts its entrance from
    /// the beginning instead of finishing the previous one.
    pub fn restart_at(&mut self, value: f32) {
        self.value = value;
        self.target = value;
    }

    /// [`Self::on_event`], with the step stated as the duration of a full `0.0 → 1.0` traversal.
    ///
    /// The form components use: a motion spec says "90ms", not "0.18 per frame".
    pub fn on_frame<M>(
        &mut self,
        event: &Event,
        target: f32,
        over: Duration,
        shell: &mut Shell<'_, M>,
    ) -> f32 {
        self.on_event(event, target, step_for(over), shell)
    }

    /// [`Self::on_frame`] for a track whose value feeds `Widget::layout`, not only `draw`.
    ///
    /// A redraw re-runs `draw` against the bounds computed by the *last* layout pass. iced
    /// re-lays-out only when a widget asks it to, so a wrapper that animates its own size — the
    /// height of an [`Expand`](crate::ui::material::animation::Expand) reveal, the width of the
    /// navigation drawer's slide — would otherwise report a new size that nothing ever reads: the
    /// element would sit still and clip against stale bounds, painting over its neighbours
    /// (BUG-001).
    ///
    /// Asks only while the value is actually changing, including the final frame that lands on the
    /// target. A track resting at its target asks for nothing, so an element that has stopped
    /// moving does not relayout the window for ever — the layout counterpart of the quiescence
    /// [`Self::on_event`] keeps for redraws.
    pub fn on_layout_frame<M>(
        &mut self,
        event: &Event,
        target: f32,
        over: Duration,
        shell: &mut Shell<'_, M>,
    ) -> f32 {
        let before = self.value;
        let value = self.on_frame(event, target, over, shell);
        if value != before {
            shell.invalidate_layout();
        }
        value
    }

    /// Advance on a frame tick, and ask for the next frame while still moving.
    ///
    /// The whole self-animating contract in one call: a widget hands it the event it received and
    /// where it wants to be, and gets back what to draw. Only a redraw tick advances the track, so
    /// a burst of mouse-move events cannot fast-forward a transition.
    ///
    /// Returns the value to draw with.
    pub fn on_event<M>(
        &mut self,
        event: &Event,
        target: f32,
        speed: f32,
        shell: &mut Shell<'_, M>,
    ) -> f32 {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Once per frame, not once per delivery: a track that invalidates the layout gets this
            // same event handed to it again in the same frame, and stepping again would make the
            // transition run at a multiple of its stated duration.
            if self.last_frame != Some(*now) {
                self.last_frame = Some(*now);
                self.advance_to(target, speed);
            }
        } else if (target - self.target).abs() > f32::EPSILON {
            // The destination changed between frames — start moving now rather than waiting for a
            // tick that nothing has asked for yet.
            self.target = target;
        }
        if self.animating() {
            shell.request_redraw();
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one property the whole arrangement rests on: it stops. Everything else is a detail.
    #[test]
    fn it_converges_and_then_rests() {
        let mut p = Progress::new(0.0);
        for _ in 0..100 {
            p.advance_to(1.0, 0.25);
        }
        assert_eq!(p.value(), 1.0);
        assert!(!p.animating());
    }

    #[test]
    fn a_fresh_track_is_already_at_rest() {
        assert!(!Progress::new(0.0).animating());
        assert!(!Progress::new(1.0).animating());
    }
}

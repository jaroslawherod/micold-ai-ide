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

/// One nominal frame at ~60fps.
///
/// A track steps by the wall-clock time elapsed since its own previous frame, so a transition takes
/// the duration it states whatever rate the window happens to render at. This is the estimate for
/// the *first* frame of a transition, which has no previous frame to measure against: its clock
/// started on the between-frames event that set the destination, and the last redraw this widget
/// saw may have been minutes ago in a quiescent window.
///
/// It used to be the step itself. A track advanced a fixed `FRAME / duration` per redraw — what the
/// central animator this feature replaced did, one step per 16ms clock tick — which made a
/// transition's real duration a frame count rather than a time. Nothing caps this application's
/// frame rate, so measured on it frames arrived every ~5ms and every transition ran roughly three
/// times too fast: a 300ms dialog entrance in 96ms, a 150ms menu exit in 45ms. Nothing was
/// truncated, but 45ms is two or three frames of a 60fps capture, and on the back-loaded
/// `accelerate` curve §6.3 gives an exit those are its flat head — so exits read as elements that
/// vanished rather than left (007 BUG-001).
///
/// Public since feature 022, for the in-crate component tests that have to *hand over* the frames
/// the runtime would deliver — a picker's list only appears once its visibility track has been
/// ticked, so a test that never ticks measures the frame before the one it means to. They need the
/// same interval this steps by, and a second `from_millis(16)` beside a test would be a duration
/// stated rather than named (`motion_tokens.rs`) as well as a number free to drift from this one.
pub const FRAME: Duration = Duration::from_millis(16);

/// The step that carries a track across its full `0.0..=1.0` range in `duration`, per *nominal*
/// frame.
///
/// Timings are stated as durations at the call site because that is what a motion spec is written
/// in; a bare step like `0.14` says nothing about how long anything takes.
///
/// For callers that advance a track a frame at a time on purpose rather than from a redraw event —
/// [`Ripple::advance`](crate::ui::cdk::ripple::Ripple::advance), and the touch test that drives a
/// track by hand. Everything driven by the runtime goes through [`Progress::on_frame`], which
/// measures the frame it was actually given instead of assuming this one.
pub fn step_for(duration: Duration) -> f32 {
    step_across(FRAME, duration)
}

/// The largest gap between two frames a single step will honour: four frames at [`FRAME`].
///
/// A window that stalls — a slow layout pass, the compositor holding a frame back — hands the next
/// redraw an elapsed time covering the whole stall. Honouring it would make a transition jump the
/// distance it "should" have travelled while nothing was on screen to travel it, which is the
/// snap this module exists to avoid. Past this bound a stalled transition runs long instead, which
/// is the recoverable failure.
const MAX_STEP: Duration = Duration::from_millis(64);

/// The step that carries a track across its full range in `over`, given `elapsed` since its last
/// frame.
fn step_across(elapsed: Duration, over: Duration) -> f32 {
    (elapsed.min(MAX_STEP).as_secs_f32() / over.as_secs_f32()).clamp(f32::EPSILON, 1.0)
}
/// A straight line: what every track did before easing existed, and the default.
const LINEAR: (f32, f32, f32, f32) = (0.0, 0.0, 1.0, 1.0);

/// `y` at linear time `t` on the cubic bézier through `(0,0)`, `(x1,y1)`, `(x2,y2)`, `(1,1)`.
///
/// The curve is parameterised by its own `x`, not by time, so `t` has to be solved for first. Ten
/// Newton steps from `t` itself converge well inside a pixel for every curve in §6.2 — these are
/// all monotone and gently sloped, which is what makes the naive start good enough.
fn ease(curve: (f32, f32, f32, f32), t: f32) -> f32 {
    let (x1, y1, x2, y2) = curve;
    if curve == LINEAR {
        return t;
    }
    let bezier = |a: f32, b: f32, u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * u * a + 3.0 * v * u * u * b + u * u * u
    };
    let slope = |a: f32, b: f32, u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * (a) + 6.0 * v * u * (b - a) + 3.0 * u * u * (1.0 - b)
    };
    let mut u = t.clamp(0.0, 1.0);
    for _ in 0..10 {
        let dx = bezier(x1, x2, u) - t;
        if dx.abs() < 1e-5 {
            break;
        }
        let d = slope(x1, x2, u);
        if d.abs() < 1e-6 {
            break;
        }
        u = (u - dx / d).clamp(0.0, 1.0);
    }
    bezier(y1, y2, u)
}

/// One animated scalar, owned by the widget that animates it.
///
/// Conventionally `0.0` (hidden, idle, collapsed) to `1.0` (shown, highlighted, expanded), though
/// nothing here requires that range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    value: f32,
    target: f32,
    /// Where the current transition started, so the eased curve has something to interpolate from.
    from: f32,
    /// Linear time through the current transition, `0.0..=1.0`.
    ///
    /// Separate from [`value`](Self::value) because easing is a mapping *from* time *to* position:
    /// a track that stepped its own position by a curve would be applying the curve to whatever it
    /// had already reached, which compounds and is not the shape Material specifies.
    t: f32,
    /// The cubic-bézier control points, as four plain numbers.
    ///
    /// Not a token type: `tests/cdk_no_appearance.rs` fails the build if this layer names one, so a
    /// curve arrives the same way a duration does — from the caller. The default is linear, which
    /// is what every track did before easing existed, so a track nobody has given a curve behaves
    /// exactly as it used to.
    curve: (f32, f32, f32, f32),
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
            from: initial,
            t: 1.0,
            curve: LINEAR,
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
        self.retarget(target);
        let distance = target - self.value;
        if distance.abs() <= f32::EPSILON {
            self.value = target;
            self.t = 1.0;
            return;
        }
        if !speed.is_finite() || speed <= 0.0 {
            self.value = target;
            self.t = 1.0;
            return;
        }
        self.t = (self.t + speed).min(1.0);
        self.value = if self.t >= 1.0 {
            target
        } else {
            self.from + (target - self.from) * ease(self.curve, self.t)
        };
    }

    /// Point the track at a new destination, restarting its eased clock.
    ///
    /// The one place `target` is assigned, and it must stay that way. Easing made this a
    /// *transition* rather than a value: `t` is linear time through the current one and `from` is
    /// where it began, so a `target` written on its own leaves the pair describing the transition
    /// before it. Concretely, a track parked at `t == 1.0` — which is every track at rest, and
    /// every track [`restart_at`](Self::restart_at) has just snapped — would step to `min(1 + speed,
    /// 1) == 1` on its very next frame and arrive instantly.
    ///
    /// That is not hypothetical: the ripple did exactly this. Its press arrives as a mouse event
    /// rather than a frame, so [`on_event`](Self::on_event) took the between-frames branch, which
    /// assigned `target` and nothing else — and the circle jumped to full size on frame one and
    /// then faded, which reads as a flash rather than as a press.
    ///
    /// No-ops when the destination has not moved, so stepping toward an unchanged target does not
    /// rewind the transition already under way.
    fn retarget(&mut self, target: f32) {
        if (target - self.target).abs() > f32::EPSILON {
            self.from = self.value;
            self.t = 0.0;
            self.target = target;
        }
    }

    /// Ease this track along a cubic bézier instead of at a constant rate.
    ///
    /// Four plain numbers rather than a token, for the reason given on [`Self::curve`]. The
    /// material layer names the curve from contract §6.2 and passes it here.
    pub fn easing(mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        self.curve = (x1, y1, x2, y2);
        self
    }

    /// Change the curve on a track already in the widget tree.
    ///
    /// Needed because a transition's curve depends on its *direction*: §6.3 gives an overlay
    /// `emphasized_decelerate` on the way in and `emphasized_accelerate` on the way out, and a
    /// track built once has to be told which it is doing before each step.
    pub fn set_easing(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.curve = (x1, y1, x2, y2);
    }

    /// Snap to `value` and come to rest there, abandoning whatever was in flight.
    ///
    /// For when a component's *identity* changes under it rather than its target: the main view
    /// showing a different session is not the old view continuing, so it starts its entrance from
    /// the beginning instead of finishing the previous one.
    pub fn restart_at(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.from = value;
        self.t = 1.0;
    }

    /// Point the track at `target` without stepping it, and ask for the frame that will.
    ///
    /// For a caller that runs two tracks in sequence: the frame on which the first *arrives* is the
    /// frame on which the second becomes due, and an arrived track asks for nothing — so nobody
    /// would ask for the frame that starts the second one, and the pair would stop mid-sequence
    /// until an unrelated event happened to wake the render loop. Handing the second track its
    /// destination here starts it asking without advancing it, so its full duration is still ahead
    /// of it.
    pub fn aim<M>(&mut self, target: f32, shell: &mut Shell<'_, M>) {
        self.retarget(target);
        self.request_frame(shell);
    }

    /// Ask for another frame while — and only while — this track is still moving.
    ///
    /// The single sanctioned frame request for the whole rendering layer (FR-025, SC-008), which is
    /// why it is one function rather than a line repeated at each call site:
    /// `tests/idle_requests_no_frames.rs` asserts there is exactly one, and that the guard is on
    /// the line above it.
    fn request_frame<M>(&self, shell: &mut Shell<'_, M>) {
        if self.animating() {
            shell.request_redraw();
        }
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
        self.on_event(event, target, over, shell)
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
    /// How far each tick advances comes from the time between it and the previous one, bounded by
    /// [`MAX_STEP`] — see [`FRAME`] for why it is measured rather than assumed.
    ///
    /// Returns the value to draw with.
    pub fn on_event<M>(
        &mut self,
        event: &Event,
        target: f32,
        over: Duration,
        shell: &mut Shell<'_, M>,
    ) -> f32 {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Once per frame, not once per delivery: a track that invalidates the layout gets this
            // same event handed to it again in the same frame, and stepping again would make the
            // transition run at a multiple of its stated duration.
            if self.last_frame != Some(*now) {
                let step = match self.last_frame {
                    // Mid-transition: step by the time that actually passed, so the transition
                    // takes `over` and not a frame count (007 BUG-001).
                    Some(previous) if self.t > 0.0 => {
                        step_across(now.saturating_duration_since(previous), over)
                    }
                    // The first frame of a transition has no previous frame of its own. `t == 0.0`
                    // means the clock was rewound by `retarget` on some between-frames event, and
                    // `last_frame` is whatever redraw this widget last saw — in a window that has
                    // been quiescent, minutes ago. One nominal frame is the honest estimate.
                    _ => step_for(over),
                };
                self.last_frame = Some(*now);
                self.advance_to(target, step);
            }
        } else {
            // The destination may have changed between frames — a press, a hover, an overlay
            // opening. Start the transition now rather than waiting for a tick nothing has asked
            // for yet, and start it *properly*: through `retarget`, so the eased clock rewinds. See
            // its docs for what assigning `target` alone here used to cost.
            self.retarget(target);
        }
        self.request_frame(shell);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A traversal that takes ten nominal frames, so one frame is a tenth of the way.
    ///
    /// These tests predate durations reaching this far in — they were written against a bare step
    /// of `0.1`, and they assert on tenths throughout. Naming the duration that *is* that step at
    /// [`FRAME`] keeps every one of their assertions arithmetically unchanged.
    const TENTH: Duration = Duration::from_millis(160);

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

    /// A destination handed over between frames still takes its full time.
    ///
    /// This is the path almost every transition in the application actually takes: a press, a
    /// hover, a message opening an overlay — none of them arrive as a redraw, so the track learns
    /// where it is going on a non-frame event and starts moving on the next frame.
    ///
    /// A track at rest sits at `t == 1.0`, because that is what "arrived" means. So a `target`
    /// written without rewinding the clock leaves the very next step computing `min(1 + speed, 1)`
    /// and jumping straight to the end — the whole transition in one frame, which reads as a
    /// flicker or, for the ripple, as a flash. Both endpoints are still correct, which is why every
    /// test that only checked where a track starts and stops stayed green.
    #[test]
    fn a_destination_set_between_frames_still_takes_its_time() {
        use iced::window;

        let mut p = Progress::new(0.0);
        let start = Instant::now();
        let mut messages: Vec<()> = Vec::new();

        // The non-frame event that changes where it is headed.
        p.on_event(
            &Event::Mouse(iced::mouse::Event::CursorEntered),
            1.0,
            TENTH,
            &mut Shell::new(&mut messages),
        );
        assert_eq!(
            p.value(),
            0.0,
            "learning the destination is not moving toward it"
        );

        // The first frame after it moves by one step, not all the way.
        p.on_event(
            &Event::Window(window::Event::RedrawRequested(
                start + Duration::from_millis(16),
            )),
            1.0,
            TENTH,
            &mut Shell::new(&mut messages),
        );
        assert!(
            p.value() > 0.0 && p.value() < 0.5,
            "one frame at a tenth of the way per frame put the track at {} — it arrived at once",
            p.value()
        );
    }

    /// Drive `p` for `frames` redraw ticks toward `target` over `over`, starting the frame clock at
    /// `from`. Returns the frame count consumed, so a caller can keep the clock monotonic across
    /// several legs — `on_event` ignores a repeated timestamp, so two legs sharing one would lose a
    /// frame silently.
    fn tick<M>(
        p: &mut Progress,
        target: f32,
        over: Duration,
        frames: usize,
        from: usize,
        messages: &mut Vec<M>,
    ) {
        use iced::window;
        for i in 0..frames {
            p.on_event(
                &Event::Window(window::Event::RedrawRequested(
                    Instant::now() + Duration::from_millis(16 * (from + i + 1) as u64),
                )),
                target,
                over,
                &mut Shell::new(messages),
            );
        }
    }

    /// An interrupted transition **resumes from where it is**; it does not snap to either end.
    ///
    /// This is the clause several manual passes could not answer — a screenshot pipeline cannot
    /// reliably catch a chosen frame of a 200 ms fade, so "does a reversal resume or snap?" stayed
    /// open on 007 (§5 reopen-during-exit / rapid-toggle) and on 022 (§B2's interrupted
    /// transition). It does not need a display: reversal is decided here, by `retarget`, and the
    /// answer is renderer-independent.
    ///
    /// `retarget` sets `from = self.value` — the live mid-flight value — and rewinds `t` to 0. So
    /// the reversal starts at exactly the value on screen and takes its own full duration from
    /// there. The two things it must not do are jump on the interrupting event itself, and arrive
    /// on the first frame after it.
    #[test]
    fn an_interrupted_transition_resumes_from_where_it_is() {
        let mut p = Progress::new(0.0);
        let mut messages: Vec<()> = Vec::new();

        // Enter, then stop partway — an overlay caught mid-fade-in.
        p.on_event(
            &Event::Mouse(iced::mouse::Event::CursorEntered),
            1.0,
            TENTH,
            &mut Shell::new(&mut messages),
        );
        tick(&mut p, 1.0, TENTH, 4, 0, &mut messages);
        let caught = p.value();
        assert!(
            caught > 0.0 && caught < 1.0,
            "the track was meant to be mid-flight, and is at {caught}"
        );

        // The interruption arrives as an ordinary event, the way Cancel or Esc does.
        p.on_event(
            &Event::Mouse(iced::mouse::Event::CursorLeft),
            0.0,
            TENTH,
            &mut Shell::new(&mut messages),
        );
        assert_eq!(
            p.value(),
            caught,
            "the reversal moved the track on the event itself — it must resume from where it is"
        );

        // The first frame of the reversal steps back from `caught`, and only by a step: not to 0,
        // and not to 1 either (the `min(1 + speed, 1)` arrival this module's other test describes).
        tick(&mut p, 0.0, TENTH, 1, 5, &mut messages);
        assert!(
            p.value() < caught && p.value() > 0.0,
            "one frame of the reversal put the track at {} (from {caught}) — it snapped",
            p.value()
        );
        assert!(p.animating(), "the reversal is still under way");

        // And it takes the reversal's own full duration from there rather than a residue of the
        // entrance: ten frames at a tenth per frame is exactly one traversal.
        tick(&mut p, 0.0, TENTH, 9, 6, &mut messages);
        assert_eq!(
            p.value(),
            0.0,
            "the reversal did not complete in its own time"
        );
        assert!(!p.animating(), "and it comes to rest");
    }

    /// A transition takes the time it states, whatever rate the window renders at (007 BUG-001).
    ///
    /// A track used to advance a fixed `FRAME / over` per redraw, which made its duration a frame
    /// count rather than a time. This application renders uncapped: measured on it, frames arrived
    /// every ~5 ms and a 150 ms menu exit finished in 45 ms. Nothing was truncated — every
    /// intermediate value was drawn and every track reached its target — but a 60 fps capture sees
    /// two or three frames of a 45 ms transition, and on the back-loaded `accelerate` curve §6.3
    /// gives an exit those first frames are its flat head. So an exit read as an element that
    /// vanished rather than one that left, and BUG-001 reported it as truncation.
    ///
    /// Both halves are asserted, because a fix that only shortened the fast run would satisfy
    /// neither: the fast run must take the *same wall clock* as the slow one, and must get there by
    /// drawing proportionally *more* intermediate values rather than by arriving early.
    #[test]
    fn a_transition_takes_its_stated_time_however_fast_frames_arrive() {
        const OVER: Duration = Duration::from_millis(200);

        /// One full exit driven at `interval` per frame: how long it took, and how many
        /// intermediate values it drew on the way.
        fn exit(interval: Duration) -> (Duration, usize) {
            let mut p = Progress::new(1.0);
            let mut messages: Vec<()> = Vec::new();
            let start = Instant::now();

            // The destination arrives between frames, as every real transition's does.
            p.on_event(
                &Event::Mouse(iced::mouse::Event::CursorLeft),
                0.0,
                OVER,
                &mut Shell::new(&mut messages),
            );

            for i in 1..10_000u32 {
                p.on_event(
                    &Event::Window(iced::window::Event::RedrawRequested(start + interval * i)),
                    0.0,
                    OVER,
                    &mut Shell::new(&mut messages),
                );
                if !p.animating() {
                    // The frame it arrived on is not an intermediate value; it is the target.
                    return (interval * i, i as usize - 1);
                }
            }
            panic!("the exit never arrived");
        }

        let (slow, slow_frames) = exit(FRAME);
        let (fast, fast_frames) = exit(FRAME / 4);

        assert!(
            fast.abs_diff(slow) <= FRAME,
            "the same {OVER:?} exit took {slow:?} at one frame per {FRAME:?} and {fast:?} at four \
             times that rate — its duration is a frame count, not a time"
        );
        assert!(
            fast_frames > slow_frames * 2,
            "the fast run drew {fast_frames} intermediate values against the slow run's \
             {slow_frames} — it reached the end early rather than drawing more of the way"
        );
        assert!(
            slow_frames > 1,
            "even at {FRAME:?} a frame the exit drew only {slow_frames} intermediate values"
        );
    }

    /// Rapid toggling leaves nothing stuck part-way (007 §5).
    ///
    /// Reverse on every other frame for a while, then stop asking and let it settle. The value
    /// stays inside its range throughout — no overshoot from a rewound clock — and the track
    /// converges on whichever destination it was last given.
    #[test]
    fn rapid_toggling_never_sticks_part_way() {
        let mut p = Progress::new(0.0);
        let mut messages: Vec<()> = Vec::new();

        let mut frame = 0usize;
        for i in 0..12 {
            let target = if i % 2 == 0 { 1.0 } else { 0.0 };
            p.on_event(
                &Event::Mouse(iced::mouse::Event::CursorEntered),
                target,
                TENTH,
                &mut Shell::new(&mut messages),
            );
            tick(&mut p, target, TENTH, 2, frame, &mut messages);
            frame += 2;
            assert!(
                (0.0..=1.0).contains(&p.value()),
                "toggle {i} put the track outside its range, at {}",
                p.value()
            );
        }

        // Stop toggling: it must land, and land closed (the last target above is 0.0, at i == 11).
        tick(&mut p, 0.0, TENTH, 20, frame, &mut messages);
        assert_eq!(p.value(), 0.0, "a rapidly toggled track stuck part-way");
        assert!(!p.animating());
    }
}

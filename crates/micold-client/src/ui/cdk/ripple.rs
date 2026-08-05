//! Per-instance press indication (feature 018, T029 — FR-024b, FR-024d, FR-024e, FR-024f, FR-039e).
//!
//! Material's ripple: a circle that grows from the press point until it covers the element, then
//! fades. This is the *behaviour* half — where it started, how far it has grown, how much strength
//! is left, and when it is over. What it looks like is the material layer's business, and this
//! carries no colour and no opacity of its own (`tests/cdk_no_appearance.rs` enforces that).
//!
//! # Why the state lives here rather than centrally
//!
//! FR-024e puts origin, progress and lifetime **inside the component instance** — no central
//! registry, no animation key. That is not a preference. Feature 017 found per-row fades keyed by a
//! *hash of the row's name*, so two worktrees whose names collided animated as one; the fix was
//! exactly this shape, and a centrally-keyed ripple would reintroduce the same class of bug in a
//! form rare enough to survive a long time unnoticed.
//!
//! Holding it per instance makes concurrency structural rather than coordinated: pressing a second
//! element cannot disturb the first, because neither can see the other. It also makes cleanup
//! structural — a removed widget drops its state, so "nothing animates after an element
//! disappears" needs no policing.
//!
//! # Timings come from the caller
//!
//! The two durations are parameters rather than constants here, for the same reason `Progress`
//! takes its speed from the caller: a motion token is part of the design system, and this layer
//! names nothing from it (`tests/cdk_no_appearance.rs` fails the build otherwise). The material
//! layer knows the ripple expands over `medium_2` and fades over `short_4`; this knows only that
//! it expands and then fades.
//!
//! # Frames
//!
//! Every frame is requested through [`Progress`], never directly, so feature 017's single
//! sanctioned frame-request path stays at one entry (FR-039e). A ripple that asked the runtime for
//! frames itself would be a second thing capable of holding the render loop awake, and idle
//! quiescence (SC-017) would stop being checkable by reading one branch.

use iced::advanced::Shell;
use iced::{Event, Point, Size};

use super::motion::Progress;
use std::time::Duration;

/// One element's press indication.
///
/// Held by the widget that ripples. Two instances cannot interfere; a dropped widget drops its
/// ripple.
#[derive(Debug, Clone)]
pub struct Ripple {
    /// Where the press landed, in the **element's own** coordinate space, clamped into it.
    ///
    /// `None` at rest. Released when the ripple finishes, so a resting component has nothing to
    /// draw — the load-bearing half of idle quiescence for this component (FR-024e, SC-017).
    origin: Option<Point>,
    /// The element's size at press time. The end radius depends on it, and the element may be
    /// re-laid-out mid-ripple, so it is captured rather than re-read.
    extent: Size,
    /// 0 → 1 as the circle grows to cover the element.
    expand: Progress,
    /// 1 → 0 as it fades. Starts falling only once [`expand`](Self::expand) has arrived, or the
    /// circle would vanish before it had finished growing.
    fade: Progress,
}

impl Default for Ripple {
    fn default() -> Self {
        Self::new()
    }
}

impl Ripple {
    /// A ripple at rest.
    pub fn new() -> Self {
        Self {
            origin: None,
            extent: Size::ZERO,
            expand: Progress::new(0.0),
            fade: Progress::new(1.0),
        }
    }

    /// Begin a ripple.
    ///
    /// `at` is **element-relative** — the frame `Cursor::position_in` reports, not the absolute
    /// window coordinates of `Cursor::position`. Passing the latter would place the origin far
    /// outside the element for anything below the top of the window (FR-024g).
    ///
    /// `None` means the press carried no pointer position (a keyboard or programmatic activation),
    /// and the ripple starts from the centre (FR-024b) — never from `(0, 0)`, which would read as a
    /// rendering bug rather than as a press.
    ///
    /// Re-pressing restarts from the new point rather than continuing: a second click elsewhere on
    /// a row should ripple from *there*.
    pub fn press(&mut self, at: Option<Point>, extent: Size) {
        let centre = Point::new(extent.width / 2.0, extent.height / 2.0);
        // Clamped because an origin outside the element leaves the expanding circle centred outside
        // the shape it is clipped to — most of it invisible, the rest sliding in from an edge
        // (FR-024d). Reachable from a press reported a frame late, or a pointer leaving mid-drag.
        let origin = at.map_or(centre, |p| {
            Point::new(p.x.clamp(0.0, extent.width), p.y.clamp(0.0, extent.height))
        });
        self.origin = Some(origin);
        self.extent = extent;
        self.expand.restart_at(0.0);
        self.fade.restart_at(1.0);
    }

    /// The press point, or `None` at rest.
    pub fn origin(&self) -> Option<Point> {
        self.origin
    }

    /// How far the circle has grown, `0.0..=1.0`.
    pub fn expansion(&self) -> f32 {
        self.expand.value()
    }

    /// How much of the ripple remains, `1.0..=0.0`.
    ///
    /// A *fraction*, deliberately, not an opacity: the material layer multiplies it by the pressed
    /// state-layer opacity. Naming the opacity here would put an appearance value in the behaviour
    /// layer.
    ///
    /// Named `strength` rather than `fade` because `material::fade` is the overlay-transition
    /// helper, and `component_api_opacity.rs` uses that signature as the canary for its
    /// wrapped-parameter scan — two `pub fn fade`s in the library make it read the wrong one.
    pub fn strength(&self) -> f32 {
        self.fade.value()
    }

    /// The radius at full expansion: the distance from the origin to the element's furthest corner
    /// (contract §5.1).
    ///
    /// Anything smaller leaves a corner uncovered, which reads as the ripple stopping short rather
    /// than as the element filling.
    pub fn end_radius(&self) -> f32 {
        let Some(o) = self.origin else {
            return 0.0;
        };
        let (w, h) = (self.extent.width, self.extent.height);
        // The furthest corner is always the one diagonally opposite the nearer edges.
        let dx = o.x.max(w - o.x);
        let dy = o.y.max(h - o.y);
        let r = dx.hypot(dy);
        // A zero-sized element can occur for a frame during layout; a non-finite radius would
        // poison the canvas path rather than draw nothing.
        if r.is_finite() {
            r
        } else {
            0.0
        }
    }

    /// The radius to draw this frame.
    pub fn radius(&self) -> f32 {
        self.end_radius() * self.expansion()
    }

    /// Whether the ripple has nothing left to do.
    pub fn is_idle(&self) -> bool {
        self.origin.is_none()
    }

    /// Advance one frame, requesting the next through [`Progress`] when still moving.
    ///
    /// The rendering entry point. `advance` exists beside it for tests, which have no `Shell`.
    pub fn on_frame<M>(
        &mut self,
        event: &Event,
        expand_over: Duration,
        fade_over: Duration,
        shell: &mut Shell<'_, M>,
    ) {
        if self.is_idle() {
            return;
        }
        if self.expand.value() < 1.0 {
            self.expand.on_frame(event, 1.0, expand_over, shell);
            if self.expand.value() >= 1.0 {
                // The expansion *arrived* on this frame, and an arrived `Progress` asks for
                // nothing — so nothing would ask for the frame that starts the fade, and the
                // ripple would stop here: fully grown, at the full pressed opacity, until some
                // unrelated event happened to wake the render loop. Aiming the fade rather than
                // stepping it hands the sequence over without eating a frame of the fade.
                self.fade.aim(0.0, shell);
            }
        } else {
            self.fade.on_frame(event, 0.0, fade_over, shell);
        }
        self.settle();
    }

    /// Advance one frame without a `Shell`.
    ///
    /// Same progression as [`Self::on_frame`], for driving the primitive in tests the way
    /// `idle_requests_no_frames.rs` drives the motion primitive.
    pub fn advance(&mut self, expand_over: Duration, fade_over: Duration) {
        if self.is_idle() {
            return;
        }
        if self.expand.value() < 1.0 {
            self.expand
                .advance_to(1.0, super::motion::step_for(expand_over));
        } else {
            self.fade
                .advance_to(0.0, super::motion::step_for(fade_over));
        }
        self.settle();
    }

    /// Release the state once the fade is done, so a resting component has nothing to draw.
    fn settle(&mut self) {
        if self.expand.value() >= 1.0 && self.fade.value() <= 0.0 {
            self.origin = None;
            self.extent = Size::ZERO;
        }
    }
}

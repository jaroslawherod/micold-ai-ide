//! Ripple state lives in the component instance (feature 018, T024 — FR-024b, FR-024d, FR-024e).
//!
//! Material's press indication. Without it a press is a flat colour swap, which is one of the
//! loudest reasons an interface does not read as Material.
//!
//! **Where the state lives is the requirement**, not an implementation detail. FR-024e puts which
//! element is rippling, from where, and how far along, *inside the component instance* — no central
//! registry, no animation key. Feature 017 learned that lesson the expensive way: per-row fades
//! keyed by a hash of the row's name meant two worktrees whose names collided animated as one. A
//! ripple keyed centrally would reintroduce exactly that, and the failure would be rare enough to
//! survive a long time.
//!
//! So these drive the primitive directly, the way `idle_requests_no_frames.rs` drives the motion
//! primitive. Two instances cannot interfere because neither can see the other — and that is
//! asserted here rather than assumed, because "they are separate objects" is only true until
//! someone adds a cache.

use std::time::{Duration, Instant};

use iced::advanced::Shell;
use iced::{window, Event, Point, Size};
use micold_client::ui::cdk::ripple::Ripple;
use micold_core::tokens::motion::duration;

/// Contract §5.1's timings, supplied by the caller the way the material layer supplies them.
const EXPAND: Duration = Duration::from_millis(duration::MEDIUM_2);
const FADE: Duration = Duration::from_millis(duration::SHORT_4);

/// Advance one frame at the contract timings.
fn step(r: &mut Ripple) {
    r.advance(EXPAND, FADE);
}

/// A realistic element: wider than it is tall, like a row or a button.
const BOUNDS: Size = Size {
    width: 200.0,
    height: 40.0,
};

fn pressed_at(x: f32, y: f32) -> Ripple {
    let mut r = Ripple::new();
    r.press(Some(Point::new(x, y)), BOUNDS);
    r
}

// ---------------------------------------------------------------------------------------------
// Origin
// ---------------------------------------------------------------------------------------------

/// The press point becomes the origin, in the element's own coordinate space.
///
/// Element-relative is the whole of FR-024g's risk: `Cursor::position()` reports *absolute window*
/// coordinates, and a ripple given one of those would originate somewhere off in the window — for
/// a row near the bottom of a tall window, entirely outside the element. `Cursor::position_in`
/// converts, and this pins the convention the component expects.
#[test]
fn the_press_point_becomes_the_origin() {
    let r = pressed_at(30.0, 10.0);
    assert_eq!(r.origin(), Some(Point::new(30.0, 10.0)));
}

/// With no known pointer position the ripple starts from the centre (FR-024b).
///
/// This is the keyboard/programmatic case, and it must not be a ripple from `(0, 0)` — a press
/// that visibly starts at the top-left corner of a row reads as a rendering bug rather than as a
/// press.
#[test]
fn an_unknown_position_ripples_from_the_centre() {
    let mut r = Ripple::new();
    r.press(None, BOUNDS);
    assert_eq!(
        r.origin(),
        Some(Point::new(BOUNDS.width / 2.0, BOUNDS.height / 2.0))
    );
}

/// An origin outside the element is clamped into it (FR-024d).
///
/// Reachable in practice: a press that begins inside and is reported a frame later, a pointer that
/// leaves during a drag, or a rounding difference at the boundary. Unclamped, the expanding circle
/// is centred outside the shape it is clipped to, so most of it is invisible and the visible part
/// slides in from an edge — which looks like a glitch, not a press.
#[test]
fn an_origin_outside_the_element_is_clamped_into_it() {
    for (given, expected) in [
        ((-20.0, 10.0), (0.0, 10.0)),
        ((500.0, 10.0), (BOUNDS.width, 10.0)),
        ((30.0, -5.0), (30.0, 0.0)),
        ((30.0, 90.0), (30.0, BOUNDS.height)),
        ((-20.0, 90.0), (0.0, BOUNDS.height)),
    ] {
        let r = pressed_at(given.0, given.1);
        assert_eq!(
            r.origin(),
            Some(Point::new(expected.0, expected.1)),
            "a press at {given:?} should clamp to {expected:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------------------------

/// The circle grows until it covers the element from wherever it started — the distance to the
/// furthest corner (contract §5.1).
///
/// Anything smaller leaves a corner uncovered at full expansion, which reads as the ripple stopping
/// short rather than as the element being filled.
#[test]
fn the_end_radius_reaches_the_furthest_corner() {
    // From a corner, the furthest point is the diagonally opposite corner.
    let r = pressed_at(0.0, 0.0);
    let diagonal = (BOUNDS.width.powi(2) + BOUNDS.height.powi(2)).sqrt();
    assert!(
        (r.end_radius() - diagonal).abs() < 0.001,
        "from the top-left the end radius should be the full diagonal {diagonal}, got {}",
        r.end_radius()
    );

    // From the centre it is half the diagonal.
    let mut c = Ripple::new();
    c.press(None, BOUNDS);
    assert!(
        (c.end_radius() - diagonal / 2.0).abs() < 0.001,
        "from the centre the end radius should be half the diagonal"
    );

    // From an off-centre point, the furthest corner is the one diagonally away from it.
    let o = pressed_at(BOUNDS.width - 10.0, 5.0);
    let furthest = (BOUNDS.width - 10.0_f32).hypot(BOUNDS.height - 5.0);
    assert!(
        (o.end_radius() - furthest).abs() < 0.001,
        "expected {furthest}, got {}",
        o.end_radius()
    );
}

/// A press on a zero-sized element does not produce a degenerate or infinite radius. Rare, but a
/// layout can produce one for a frame, and a `NaN` radius poisons the canvas path rather than
/// drawing nothing.
#[test]
fn a_zero_sized_element_produces_a_finite_radius() {
    let mut r = Ripple::new();
    r.press(None, Size::new(0.0, 0.0));
    assert!(r.end_radius().is_finite());
    assert!(r.end_radius() >= 0.0);
}

// ---------------------------------------------------------------------------------------------
// Lifetime
// ---------------------------------------------------------------------------------------------

/// A fresh ripple is idle and has no origin — a component built at rest must not draw one.
#[test]
fn a_fresh_ripple_is_idle() {
    let r = Ripple::new();
    assert!(r.is_idle());
    assert_eq!(r.origin(), None);
}

/// Pressing starts it.
#[test]
fn pressing_starts_the_ripple() {
    let r = pressed_at(30.0, 10.0);
    assert!(!r.is_idle());
    assert!(r.expansion() < 1.0, "it starts from nothing and grows");
    assert_eq!(r.strength(), 1.0, "it starts at full strength");
}

/// A completed ripple releases its state, so nothing is retained at rest (FR-024e, SC-017).
///
/// The load-bearing half of idle quiescence. A ripple that finishes but keeps its origin is a
/// component that still has something to draw, and — worse — one that may still ask for frames.
/// The application would burn a core forever after the last click, which is invisible until a
/// laptop fan explains it.
#[test]
fn a_completed_ripple_releases_its_state() {
    let mut r = pressed_at(30.0, 10.0);
    for _ in 0..1_000 {
        if r.is_idle() {
            break;
        }
        step(&mut r);
    }
    assert!(r.is_idle(), "the ripple never finished");
    assert_eq!(
        r.origin(),
        None,
        "a finished ripple kept its origin — it still has something to draw, and at rest there is \
         nothing to draw (FR-024e)"
    );
    assert_eq!(r.strength(), 0.0, "a finished ripple is fully faded");
}

/// It actually takes time. A ripple that completes in one step is not an animation, and this would
/// otherwise pass trivially against a stub.
#[test]
fn the_ripple_takes_more_than_one_frame() {
    let mut r = pressed_at(30.0, 10.0);
    let mut frames = 0;
    while !r.is_idle() && frames < 1_000 {
        step(&mut r);
        frames += 1;
    }
    assert!(
        frames > 5,
        "the ripple finished in {frames} frame(s) — that is a flash, not a press indication"
    );
}

/// Expansion runs to completion before the fade takes it away, or the circle vanishes before it
/// has covered the element.
#[test]
fn it_expands_before_it_fades() {
    let mut r = pressed_at(30.0, 10.0);
    let mut saw_full_expansion_at_full_strength = false;
    for _ in 0..1_000 {
        if r.is_idle() {
            break;
        }
        if r.expansion() >= 1.0 && r.strength() > 0.99 {
            saw_full_expansion_at_full_strength = true;
        }
        step(&mut r);
    }
    assert!(
        saw_full_expansion_at_full_strength,
        "the ripple began fading before it finished expanding, so it disappears mid-growth"
    );
}

/// It asks for the *next* frame on every frame that is not its last (FR-024f, FR-039e).
///
/// The tests above drive `advance`, which needs no `Shell` and therefore cannot see this: the
/// application does not step the ripple on a timer, it steps it on a redraw it asked for, so a
/// frame the ripple forgets to ask for is a frame that never happens. The gap was between the two
/// phases — a `Progress` that has arrived asks for nothing, so the frame on which the expansion
/// landed requested nothing and the frame that would have started the fade never came. The ripple
/// stopped there: fully grown, at the full pressed opacity, until an unrelated event happened to
/// wake the render loop. Every other test still passed, because every other test drives the clock
/// itself.
#[test]
fn it_asks_for_a_frame_on_every_frame_but_its_last() {
    let mut r = pressed_at(30.0, 10.0);
    let start = Instant::now();

    for frame in 0..1_000u32 {
        if r.is_idle() {
            assert!(frame > 5, "the ripple finished in {frame} frames");
            return;
        }
        // A distinct instant per frame: `Progress` advances once per frame and tells frames apart
        // by the timestamp the redraw carries.
        let now = start + Duration::from_millis(16 * u64::from(frame) + 16);
        let mut messages: Vec<()> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        r.on_frame(
            &Event::Window(window::Event::RedrawRequested(now)),
            EXPAND,
            FADE,
            &mut shell,
        );
        // The frame that finishes the ripple owes nothing; every other frame owes the next one.
        assert!(
            r.is_idle() || matches!(shell.redraw_request(), window::RedrawRequest::NextFrame),
            "frame {frame} asked for no successor while the ripple was still running \
             (expansion {}, strength {}) — it is stuck there until something unrelated redraws",
            r.expansion(),
            r.strength()
        );
    }
    panic!("the ripple never settled");
}

/// The circle *grows*. It does not appear already covering the element.
///
/// Driven through `Shell` rather than through `advance`, because that is the difference: `advance`
/// calls the primitive's stepping function directly, and the widget path reaches it through
/// `on_event`, which retargets a track by a route of its own. This asserts the shape of the
/// expansion rather than only its endpoints — a ripple that arrives at full size on its first frame
/// and then fades still settles in the same number of frames as one that grows, and still asks for
/// a frame on every frame but its last, so both of the tests above pass while what is on screen is
/// a flash rather than a ripple.
#[test]
fn the_expansion_takes_the_time_it_is_given() {
    let mut r = pressed_at(30.0, 10.0);
    let start = Instant::now();

    // The press itself, which is the event the widget hands the ripple before any frame arrives.
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    r.on_frame(
        &Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        EXPAND,
        FADE,
        &mut shell,
    );

    let mut frames_to_full = None;
    for frame in 0..1_000u32 {
        let now = start + Duration::from_millis(16 * u64::from(frame) + 16);
        let mut messages: Vec<()> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        r.on_frame(
            &Event::Window(window::Event::RedrawRequested(now)),
            EXPAND,
            FADE,
            &mut shell,
        );
        if r.expansion() >= 1.0 {
            frames_to_full = Some(frame + 1);
            break;
        }
    }

    let frames = frames_to_full.expect("the expansion never completed");
    // `medium_2` over 16ms frames is about a dozen. Bounded loosely at both ends: the point is that
    // it is neither instant nor unbounded, and pinning the exact count would break on any re-value
    // of the scale.
    assert!(
        frames >= 5,
        "the circle reached full size in {frames} frame(s) — it is meant to grow over {}ms, and \
         arriving at once reads as a flash rather than as a press",
        EXPAND.as_millis()
    );
    assert!(
        frames <= 40,
        "the circle took {frames} frames to grow, well past the {}ms it is given",
        EXPAND.as_millis()
    );
}

// ---------------------------------------------------------------------------------------------
// Independence — the requirement FR-024e is really about
// ---------------------------------------------------------------------------------------------

/// Pressing element B mid-ripple leaves A untouched (FR-024e).
///
/// The property a central registry cannot give you. Two instances hold their own state, so
/// concurrency is structural rather than coordinated — and this asserts it instead of trusting it,
/// because "they are separate objects" stays true only until someone adds a cache keyed by
/// something.
#[test]
fn pressing_a_second_element_does_not_disturb_the_first() {
    let mut a = pressed_at(10.0, 10.0);
    for _ in 0..4 {
        step(&mut a);
    }
    let (a_origin, a_expansion) = (a.origin(), a.expansion());

    let mut b = pressed_at(180.0, 30.0);
    step(&mut b);

    assert_eq!(a.origin(), a_origin, "B's press moved A's origin");
    assert_eq!(
        a.expansion(),
        a_expansion,
        "B's press advanced A's progress"
    );
    assert_eq!(
        b.origin(),
        Some(Point::new(180.0, 30.0)),
        "B did not take its own origin"
    );
    assert_ne!(
        a.origin(),
        b.origin(),
        "the two ripples share an origin — they are not independent"
    );
}

/// Re-pressing the same element restarts it from the new point rather than continuing the old one.
/// A second click somewhere else on a row should ripple from *there*.
#[test]
fn re_pressing_restarts_from_the_new_point() {
    let mut r = pressed_at(10.0, 10.0);
    for _ in 0..4 {
        step(&mut r);
    }
    let mid = r.expansion();
    assert!(mid > 0.0);

    r.press(Some(Point::new(150.0, 20.0)), BOUNDS);
    assert_eq!(r.origin(), Some(Point::new(150.0, 20.0)));
    assert!(
        r.expansion() < mid,
        "re-pressing continued the previous expansion instead of starting again"
    );
    assert_eq!(r.strength(), 1.0, "re-pressing returns to full strength");
}

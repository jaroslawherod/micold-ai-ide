//! The session options row grows and collapses (feature 1792 US3, FR-005..FR-008, FR-011,
//! FR-013, SC-002), driven frame by frame on the composer the app builds.
//!
//! Heights are the composer's own: the row is the only part that changes between the poses, so
//! the composer's height is the row's reveal plus a constant.

use std::time::Duration;

use crate::ui::material::test_support::{renderer, roles, Mounted, FRAME_MS};
use iced::advanced::widget::operation::focusable;
use iced::advanced::{layout, mouse};
use iced::widget::text;
use iced::{Element, Event, Point, Size};
use micold_core::tokens::motion::duration;

use super::*;
use crate::features::composer::tests::{add_structured_session, structured_state};

/// A room wide enough for the three controls on one line.
const WIDE: Size = Size::new(1200.0, 800.0);
/// The standard duration the row takes (research R3).
const OVER: Duration = Duration::from_millis(duration::MEDIUM_2);
/// Half a pixel, as the geometry gates allow.
const TOLERANCE: f32 = 0.5;

fn posed(open: bool, reduce_motion: bool) -> (State, SessionId) {
    let (mut state, s) = structured_state();
    state.composers.options_open = open;
    state.settings.reduce_motion = reduce_motion;
    (state, s)
}

/// `session`'s composer with the permission control in its seam, as the pane builds it.
fn composer(state: &State, session: SessionId) -> Element<'_, String> {
    let seams = Seams {
        control: Some(text("Permission").into()),
        ..Seams::default()
    };
    view(state, session, roles(), seams).map(|m| format!("{m:?}"))
}

fn height(m: &Mounted<'_>) -> f32 {
    m.node.bounds().height
}

/// Frame ticks from `*clock` ms (after the mount's origin) for `span`, one per frame: the ms since
/// the first tick and the composer's height after each. `clock` moves on, so a later run starts
/// after this one ended; a track ignores a frame it has already seen.
fn run(m: &mut Mounted<'_>, clock: &mut u64, span: u64) -> Vec<(u64, f32)> {
    let start = *clock;
    let mut seen = Vec::new();
    while *clock <= start + span {
        let at = m.origin + Duration::from_millis(*clock);
        m.tick(at, mouse::Cursor::Unavailable);
        seen.push((*clock - start, height(m)));
        *clock += FRAME_MS;
    }
    seen
}

/// The first ms of `seen` at which the height is `end`: there, not half a pixel short, since the
/// decelerating tail spends its last frames inside that half pixel.
fn arrival(seen: &[(u64, f32)], end: f32) -> Option<u64> {
    seen.iter()
        .find(|(_, h)| (h - end).abs() <= 1e-3)
        .map(|(ms, _)| *ms)
}

/// T013 (FR-005, FR-006, FR-013): the row takes the standard motion token, and no time under
/// Reduce motion.
#[test]
fn the_row_takes_the_standard_duration_and_none_under_reduce_motion() {
    assert_eq!(options_motion(&posed(true, false).0), OVER);
    assert_eq!(options_motion(&posed(true, true).0), Duration::ZERO);
}

/// T013 (SC-002, US3 scenarios 1 and 2): opening grows the row over the standard duration, within
/// one frame of it, through heights between, never a full-height jump; closing collapses it the
/// same way.
#[test]
fn opening_and_closing_take_the_standard_duration_without_a_jump() {
    let (closed, s) = posed(false, false);
    let (open, _) = posed(true, false);
    let shut = height(&Mounted::new_in(composer(&closed, s), WIDE));
    let full = height(&Mounted::new_in(composer(&open, s), WIDE));
    assert!(
        full > shut + TOLERANCE,
        "the open row takes room: {shut} vs {full}"
    );

    let mut m = Mounted::new_in(composer(&closed, s), WIDE);
    let mut clock = 0;
    for (to, end) in [(&open, full), (&closed, shut)] {
        m.rebuild(composer(to, s));
        let seen = run(&mut m, &mut clock, OVER.as_millis() as u64 * 2);
        let took = arrival(&seen, end).expect("the row reaches its end height");
        assert!(
            took.abs_diff(OVER.as_millis() as u64) <= FRAME_MS,
            "took {took} ms, not {OVER:?} within a frame: {seen:?}"
        );
        let between = seen
            .iter()
            .filter(|(_, h)| (h - shut).abs() > TOLERANCE && (h - full).abs() > TOLERANCE)
            .count();
        assert!(between > 3, "too few heights between: {seen:?}");
        let first = seen[0].1;
        assert!(
            (first - end).abs() > (full - shut) / 2.0,
            "the first frame jumped most of the way: {seen:?}"
        );
    }
}

/// T013 (FR-013, US3 scenario 4): under Reduce motion the row is at its end height on the first
/// frame.
#[test]
fn under_reduce_motion_the_row_switches_in_one_step() {
    let (closed, s) = posed(false, true);
    let (open, _) = posed(true, true);
    let full = height(&Mounted::new_in(composer(&open, s), WIDE));
    let mut m = Mounted::new_in(composer(&closed, s), WIDE);
    m.rebuild(composer(&open, s));
    let seen = run(&mut m, &mut 0, 0);
    assert!((seen[0].1 - full).abs() <= TOLERANCE, "{seen:?} vs {full}");
}

/// T014 (FR-007, US3 scenario 3): a toggle mid-way reverses from the height reached, and rapid
/// toggles leave the row settled where the last one points, never stuck half open.
#[test]
fn a_toggle_mid_way_reverses_from_the_current_height() {
    let (closed, s) = posed(false, false);
    let (open, _) = posed(true, false);
    let shut = height(&Mounted::new_in(composer(&closed, s), WIDE));
    let full = height(&Mounted::new_in(composer(&open, s), WIDE));
    let step = (full - shut) / 3.0;

    let mut m = Mounted::new_in(composer(&closed, s), WIDE);
    let mut clock = 0;
    m.rebuild(composer(&open, s));
    let reached = run(&mut m, &mut clock, OVER.as_millis() as u64 / 2)
        .last()
        .expect("frames ran")
        .1;
    assert!(
        reached > shut + TOLERANCE && reached < full - TOLERANCE,
        "mid-way: {reached}"
    );
    m.rebuild(composer(&closed, s));
    let back = run(&mut m, &mut clock, OVER.as_millis() as u64 * 2);
    assert!(
        (back[0].1 - reached).abs() < step,
        "the reversal jumped from {reached} to {}",
        back[0].1
    );
    assert!(
        back.windows(2).all(|w| w[1].1 <= w[0].1 + TOLERANCE),
        "the collapse went back up: {back:?}"
    );
    assert!((back.last().expect("frames ran").1 - shut).abs() <= TOLERANCE);

    for i in 0..9 {
        m.rebuild(composer(if i % 2 == 0 { &open } else { &closed }, s));
        run(&mut m, &mut clock, 0);
    }
    // Nine toggles from closed: the last one opens.
    let end = run(&mut m, &mut clock, OVER.as_millis() as u64 * 2);
    assert!(
        (end.last().expect("frames ran").1 - full).abs() <= TOLERANCE,
        "{end:?}"
    );
}

/// T015 (FR-008): each session's composer keeps its own track. A composer shown while another is
/// mid-way arrives settled, and the other goes on from where it was.
#[test]
fn two_sessions_keep_separate_tracks() {
    let (mut closed, a) = posed(false, false);
    let b = add_structured_session(&mut closed);
    let mut open = closed.clone();
    open.composers.options_open = true;
    let pair = |state| pair(state, a, b);
    let heights = |m: &Mounted<'_>| {
        let c = m.node.children();
        (c[0].bounds().height, c[1].bounds().height)
    };
    let full = heights(&Mounted::new_in(pair(&open), WIDE)).0;

    let mut m = Mounted::new_in(composer_column(&closed, a), WIDE);
    let mut clock = 0;
    m.rebuild(composer_column(&open, a));
    run(&mut m, &mut clock, OVER.as_millis() as u64 / 3);
    m.rebuild(pair(&open));
    run(&mut m, &mut clock, 0);
    let (first, second) = heights(&m);
    assert!(
        first < full - TOLERANCE,
        "session A is still on its way: {first} vs {full}"
    );
    assert!(
        (second - full).abs() <= TOLERANCE,
        "session B arrives settled: {second}"
    );
    run(&mut m, &mut clock, OVER.as_millis() as u64 * 2);
    assert!(
        (heights(&m).0 - full).abs() <= TOLERANCE,
        "session A arrives"
    );
}

fn pair(state: &State, a: SessionId, b: SessionId) -> Element<'_, String> {
    iced::widget::column![composer(state, a), composer(state, b)].into()
}

fn composer_column(state: &State, session: SessionId) -> Element<'_, String> {
    iced::widget::column![composer(state, session)].into()
}

/// Press and release at `at`: what the composer published.
fn click(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
    let mut published = m.press(at);
    published.extend(m.send(
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(at),
    ));
    published
}

/// Every message a click anywhere on the composer, on a 12 px grid, publishes.
fn clicks_everywhere(m: &mut Mounted<'_>) -> Vec<String> {
    let size = m.node.bounds();
    let mut published = Vec::new();
    let mut y = 2.0;
    while y < size.height {
        let mut x = 2.0;
        while x < size.width {
            published.extend(click(m, Point::new(x, y)));
            x += 12.0;
        }
        y += 12.0;
    }
    published
}

/// T016 (research R3, contract §2): a collapsed row's controls take no keyboard stop and no
/// click; the open row's do, which proves the probes reach them.
#[test]
fn a_collapsed_rows_controls_are_not_reachable() {
    let (closed, s) = posed(false, false);
    let (open, _) = posed(true, false);
    let picks = |published: &[String]| {
        published
            .iter()
            .filter(|m| m.contains("ModelPickerRequested") || m.contains("EffortPickerToggled"))
            .count()
    };

    let mut shut = Mounted::new_in(composer(&closed, s), WIDE);
    let mut shown = Mounted::new_in(composer(&open, s), WIDE);
    let stops = |m: &mut Mounted<'_>| m.drive(focusable::count()).expect("counted").total;
    let (open_stops, shut_stops) = (stops(&mut shown), stops(&mut shut));
    assert!(
        open_stops > shut_stops,
        "the open row adds its triggers' keyboard stops: {shut_stops} vs {open_stops}"
    );
    assert!(
        picks(&clicks_everywhere(&mut shown)) > 0,
        "the open row's triggers take a click"
    );
    assert_eq!(
        picks(&clicks_everywhere(&mut shut)),
        0,
        "the collapsed row's take none"
    );
}

/// The options row alone, laid out in the composer's inner width of `room`.
fn row_height(state: &State, session: SessionId, room: Size) -> f32 {
    let mut row: Element<'_, Message> =
        options_row(state, session, roles(), Some(text("Permission").into()));
    let renderer = renderer();
    let mut tree = iced::advanced::widget::Tree::new(row.as_widget());
    let inner = Size::new(room.width - 2.0 * spacing::MD, room.height);
    row.as_widget_mut()
        .layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, inner),
        )
        .bounds()
        .height
}

/// T018 (FR-011, research R5): at a narrow width the row ends at its wrapped height, and in a
/// window shorter than the row at the height it is given there.
#[test]
fn at_a_narrow_width_the_row_ends_at_its_wrapped_height() {
    let (closed, s) = posed(false, false);
    let (open, _) = posed(true, false);
    let one_line = row_height(&open, s, WIDE);
    let narrow = Size::new(320.0, 800.0);
    let row = row_height(&open, s, narrow);
    assert!(
        row > one_line + TOLERANCE,
        "the narrow row wraps: {one_line} vs {row}"
    );
    let grown = grow(&closed, &open, s, narrow);
    // The pose's notice line (no service connected) sits above the row, so the row brings the gap
    // under that line with it.
    assert!(
        (grown - row - spacing::SM).abs() <= TOLERANCE,
        "the narrow row grew by {grown}, not its wrapped {row} and its gap"
    );

    // Shorter than the composer: the row ends at the height the composer gives it there, as the
    // row shown at once does, and no taller.
    let short = Size::new(320.0, 60.0);
    let at_once = height(&Mounted::new_in(composer(&open, s), short))
        - height(&Mounted::new_in(composer(&closed, s), short));
    let grown = grow(&closed, &open, s, short);
    assert!(
        (grown - at_once).abs() <= TOLERANCE,
        "in a short window the row grew by {grown}, not the {at_once} it has there"
    );
}

/// How much the composer grows in `room` from `closed` to `open`, run to its end.
fn grow(closed: &State, open: &State, s: SessionId, room: Size) -> f32 {
    let mut m = Mounted::new_in(composer(closed, s), room);
    let shut = height(&m);
    let mut clock = 0;
    m.rebuild(composer(open, s));
    let end = run(&mut m, &mut clock, OVER.as_millis() as u64 * 2)
        .last()
        .expect("frames ran")
        .1;
    end - shut
}

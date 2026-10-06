//! The pointer-following tooltip, driven as the runtime drives it (feature 430; contracts/
//! tooltip-api.md, Behaviour 1 to 6).
//!
//! `place_at_pointer` holds where the panel goes and `micold_core::tooltip::ShowTimer` when it
//! opens; both have their own tables. What is held here is the widget that feeds them: which events
//! it observes, with which cursor, and what it shows as a result.

#[path = "support/mod.rs"]
mod support;

use std::time::{Duration, Instant};

use iced::mouse::Cursor;
use iced::window::RedrawRequest;
use iced::{Point, Rectangle};

use support::tooltip::{delayed_tooltip, follow_tooltip, Driven, FOLLOW_GAP, TRIGGER, WINDOW};

/// The padding drawn as nothing around the panel's content (`cdk::tooltip::EDGE_PADDING`).
const EDGE_PADDING: f32 = 5.0;

const FRAME: Duration = Duration::from_millis(16);

fn following() -> Driven {
    Driven::new(follow_tooltip(None))
}

fn at(x: f32, y: f32) -> Cursor {
    Cursor::Available(Point::new(x, y))
}

/// The drawn part of `panel`: inside its padding.
fn visible(panel: Rectangle) -> Rectangle {
    Rectangle {
        x: panel.x + EDGE_PADDING,
        y: panel.y + EDGE_PADDING,
        width: panel.width - EDGE_PADDING * 2.0,
        height: panel.height - EDGE_PADDING * 2.0,
    }
}

/// US1.1: the panel opens beside the pointer, `gap` from it, not under it.
#[test]
fn the_follow_panel_opens_at_the_pointer() {
    let mut tip = following();
    let cursor = at(80.0, 60.0);

    tip.frame(Instant::now(), cursor);

    let panel = tip.panel().expect("open over the trigger");
    let v = visible(panel);
    assert_eq!(
        (v.x, v.y),
        (80.0 + FOLLOW_GAP, 60.0 + FOLLOW_GAP),
        "{panel:?}"
    );
}

/// US1.1: a pointer move while open lays the panel out beside the pointer's new place.
#[test]
fn a_pointer_move_while_open_moves_the_panel() {
    let mut tip = following();
    let start = Instant::now();
    tip.frame(start, at(80.0, 60.0));

    tip.frame(start + FRAME, at(120.0, 70.0));

    let v = visible(tip.panel().expect("still open"));
    assert_eq!((v.x, v.y), (120.0 + FOLLOW_GAP, 70.0 + FOLLOW_GAP));
}

/// FR-007: a pointer that does not move asks for nothing, open.
#[test]
fn a_still_pointer_requests_nothing() {
    let mut tip = following();
    let start = Instant::now();
    let cursor = at(80.0, 60.0);
    tip.frame(start, cursor);

    for n in 1..=10 {
        let seen = tip.frame(start + FRAME * n, cursor);
        assert_eq!(
            seen.redraw,
            RedrawRequest::Wait,
            "frame {n} with a still pointer"
        );
    }
}

/// US1.3: leaving closes it.
#[test]
fn leaving_the_trigger_closes_the_follow_panel() {
    let mut tip = following();
    let start = Instant::now();
    tip.frame(start, at(80.0, 60.0));
    assert!(tip.is_open(), "precondition");

    tip.frame(start + FRAME, at(300.0, 300.0));

    assert!(!tip.is_open());
}

/// Edge case: a press closes it until the pointer has left and come back.
#[test]
fn a_press_closes_the_follow_panel_until_the_pointer_leaves() {
    let mut tip = following();
    let start = Instant::now();
    let cursor = at(80.0, 60.0);
    tip.frame(start, cursor);

    tip.pressed(cursor);
    assert!(!tip.is_open(), "the press closes it");
    tip.frame(start + FRAME, at(90.0, 60.0));
    assert!(!tip.is_open(), "moving on the trigger does not reopen it");

    tip.frame(start + FRAME * 2, at(300.0, 300.0));
    tip.frame(start + FRAME * 3, cursor);
    assert!(tip.is_open(), "back on the trigger after leaving");
}

/// Edge case: a trigger that now describes something else is a new tooltip. What the old subject's
/// press had spent is forgotten, and with no delay the new one opens at the pointer at once.
#[test]
fn a_subject_change_starts_the_follow_tooltip_afresh() {
    let mut tip = Driven::new(follow_tooltip(Some(1)));
    let start = Instant::now();
    let cursor = at(80.0, 60.0);
    tip.frame(start, cursor);
    tip.pressed(cursor);
    assert!(
        !tip.is_open(),
        "precondition: the press closed it for this subject"
    );

    tip.rebuild(follow_tooltip(Some(2)));
    tip.frame(start + FRAME, cursor);

    let v = visible(tip.panel().expect("the new subject opens"));
    assert_eq!((v.x, v.y), (80.0 + FOLLOW_GAP, 60.0 + FOLLOW_GAP));
}

/// Edge case: a trigger scrolled from under a still pointer closes the panel on the next redraw.
#[test]
fn a_trigger_scrolled_from_under_a_still_pointer_closes_the_panel() {
    let mut tip = following();
    let start = Instant::now();
    let cursor = at(80.0, 60.0);
    tip.frame(start, cursor);
    assert!(tip.is_open(), "precondition");

    tip.at = Point::new(50.0, 50.0 + TRIGGER.height + 100.0);
    tip.frame(start + FRAME, cursor);

    assert!(!tip.is_open());
}

// ---------------------------------------------------------------------------------------------
// The show delay (US2, FR-003, FR-004, SC-002)
// ---------------------------------------------------------------------------------------------

const D: Duration = Duration::from_millis(500);

fn delayed(follow: bool) -> Driven {
    Driven::new(delayed_tooltip(D, follow, None))
}

/// US2.1, SC-002: nothing before the delay has run from entering; the panel at it.
#[test]
fn nothing_shows_before_the_delay_and_the_panel_shows_at_it() {
    for follow in [false, true] {
        let mut tip = delayed(follow);
        let start = Instant::now();
        let cursor = tip.over(20.0);

        tip.frame(start, cursor);
        assert!(!tip.is_open(), "follow={follow}: on entering");
        tip.frame(start + D - Duration::from_millis(1), cursor);
        assert!(!tip.is_open(), "follow={follow}: just before the delay");
        tip.frame(start + D, cursor);
        assert!(tip.is_open(), "follow={follow}: at the delay");
    }
}

/// US2.2: leaving cancels the wait, and coming back starts it again from entering.
#[test]
fn leaving_cancels_the_delay() {
    let mut tip = delayed(false);
    let start = Instant::now();
    let cursor = tip.over(20.0);
    tip.frame(start, cursor);

    tip.frame(start + D / 2, at(300.0, 300.0));
    tip.frame(start + D / 2 + FRAME, cursor);
    tip.frame(start + D, cursor);
    assert!(!tip.is_open(), "the first entry's wait was cancelled");

    tip.frame(start + D / 2 + FRAME + D, cursor);
    assert!(tip.is_open(), "the second entry's own delay has run");
}

/// FR-003: the delay counts from entering; movement over the trigger does not restart it.
#[test]
fn movement_during_the_wait_does_not_restart_it() {
    let mut tip = delayed(false);
    let start = Instant::now();
    tip.frame(start, tip.over(20.0));

    for n in 1..5 {
        tip.frame(start + D * n / 5, tip.over(20.0 + 30.0 * n as f32));
    }
    assert!(!tip.is_open(), "precondition: still waiting");
    tip.frame(start + D, tip.over(170.0));

    assert!(
        tip.is_open(),
        "{D:?} after entering, however the pointer moved"
    );
}

/// US2.3, FR-004: no delay opens at once.
#[test]
fn a_zero_delay_opens_at_once() {
    for follow in [false, true] {
        let mut tip = Driven::new(delayed_tooltip(Duration::ZERO, follow, None));
        tip.frame(Instant::now(), tip.over(20.0));
        assert!(tip.is_open(), "follow={follow}");
    }
}

/// US2.4: the delayed follow panel opens beside where the pointer is *then*, and tracks it on.
#[test]
fn a_delayed_follow_panel_opens_at_the_current_pointer_and_then_tracks_it() {
    let mut tip = delayed(true);
    let start = Instant::now();
    tip.frame(start, at(80.0, 60.0));
    tip.frame(start + D / 2, at(100.0, 65.0));

    tip.frame(start + D, at(140.0, 70.0));
    let v = visible(tip.panel().expect("open at the delay"));
    assert_eq!((v.x, v.y), (140.0 + FOLLOW_GAP, 70.0 + FOLLOW_GAP));

    tip.frame(start + D + FRAME, at(150.0, 75.0));
    let v = visible(tip.panel().expect("still open"));
    assert_eq!((v.x, v.y), (150.0 + FOLLOW_GAP, 75.0 + FOLLOW_GAP));
}

/// Edge case, SC-003: at each window edge the delayed follow panel flips rather than cover the
/// pointer.
#[test]
fn a_delayed_follow_panel_never_covers_the_pointer_at_a_window_edge() {
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(WINDOW.width - TRIGGER.width, 0.0),
        Point::new(0.0, WINDOW.height - TRIGGER.height),
        Point::new(WINDOW.width - TRIGGER.width, WINDOW.height - TRIGGER.height),
    ];
    for corner in corners {
        for (dx, dy) in [(2.0, 2.0), (TRIGGER.width - 2.0, TRIGGER.height - 2.0)] {
            let mut tip = delayed(true);
            tip.at = corner;
            let start = Instant::now();
            let pointer = Point::new(corner.x + dx, corner.y + dy);
            tip.frame(start, Cursor::Available(pointer));
            tip.frame(start + D, Cursor::Available(pointer));

            let panel = tip.panel().expect("open at the delay");
            assert!(
                !visible(panel).contains(pointer),
                "trigger at {corner:?}, pointer {pointer:?}: {panel:?}"
            );
        }
    }
}

/// Edge case: a trigger that now describes something else starts its delay afresh.
#[test]
fn a_subject_change_during_the_delay_restarts_it() {
    let mut tip = Driven::new(delayed_tooltip(D, false, Some(1)));
    let start = Instant::now();
    let cursor = tip.over(20.0);
    tip.frame(start, cursor);

    tip.rebuild(delayed_tooltip(D, false, Some(2)));
    tip.frame(start + D / 2, cursor);
    tip.frame(start + D, cursor);
    assert!(
        !tip.is_open(),
        "the new subject's wait began at {:?}",
        D / 2
    );

    tip.frame(start + D / 2 + D, cursor);
    assert!(tip.is_open());
}

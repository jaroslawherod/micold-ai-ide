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

use support::tooltip::{follow_tooltip, Driven, FOLLOW_GAP, TRIGGER};

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

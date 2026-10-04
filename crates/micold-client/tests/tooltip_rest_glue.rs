//! The tooltip's rest mode, driven as the runtime drives it (feature 038; contracts/rest-tooltip.md
//! §2, §4).
//!
//! `micold_core::tooltip::RestTimer` holds the rule and `tests/tooltip_rest.rs` in core holds the
//! rule to its table. What is held here is the widget that feeds it: which events it observes, with
//! which cursor and which clock, and what it shows as a result.
//!
//! The line limit (`material::Tooltip::max_lines`, contract §5) is held beside its widget, in
//! `src/ui/material/line_clamp.rs`: `ui::material` is not reachable from an integration test.

#[path = "support/mod.rs"]
mod support;

use std::time::{Duration, Instant};

use iced::keyboard;
use iced::mouse;
use iced::window::RedrawRequest;
use iced::{Event, Point};

use support::tooltip::{tooltip, Driven, Msg, DELAY};

const MS: Duration = Duration::from_millis(1);

fn rest_mode() -> Driven {
    Driven::new(tooltip(Some(DELAY), None))
}

fn a_key_press() -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(keyboard::key::Named::Enter),
        modified_key: keyboard::Key::Named(keyboard::key::Named::Enter),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    })
}

/// U65, first half: nothing before the delay, the panel at the delay.
#[test]
fn the_panel_shows_only_after_the_delay_at_rest() {
    let mut tip = rest_mode();
    let start = Instant::now();
    let cursor = tip.over(20.0);

    tip.frame(start, cursor);
    assert!(!tip.is_open(), "the cursor has only just arrived");

    tip.frame(start + DELAY - MS, cursor);
    assert!(!tip.is_open(), "1 ms short of the delay");

    tip.frame(start + DELAY, cursor);
    assert!(tip.is_open(), "at rest for the whole delay");
}

/// U65, second half: 10 px every 100 ms for 10 s opens nothing (SC-004).
#[test]
fn the_panel_does_not_show_while_the_cursor_moves() {
    let mut tip = rest_mode();
    let start = Instant::now();

    for step in 0..=100_u32 {
        // Back and forth inside the trigger: every step is 10 px or more from the one before.
        let dx = 10.0 + 10.0 * (step % 15) as f32;
        let now = start + Duration::from_millis(100) * step;
        tip.frame(now, tip.over(dx));
        assert!(!tip.is_open(), "open after {step} moves");
    }
}

/// A real cursor move is observed too, against the wall clock: it starts a wait and opens nothing.
#[test]
fn a_cursor_move_onto_the_trigger_starts_the_wait() {
    let mut tip = rest_mode();
    let before = Instant::now();

    let seen = tip.moved(tip.over(20.0));

    let after = Instant::now();
    assert!(!tip.is_open());
    let RedrawRequest::At(wake) = seen.redraw else {
        panic!("a waiting tooltip asks to be woken, got {:?}", seen.redraw);
    };
    assert!(
        wake >= before + DELAY && wake <= after + DELAY,
        "woken when the delay has run from the move",
    );
}

/// U66: a press closes the panel, and is the trigger's press all the same.
#[test]
fn a_press_closes_the_panel_and_still_reaches_the_trigger() {
    let mut tip = rest_mode();
    let opened = tip.rest_until_open(Instant::now());
    let cursor = tip.over(20.0);

    let seen = tip.pressed(cursor);

    assert_eq!(seen.messages, [Msg::Pressed], "the trigger takes the press");
    assert!(!tip.is_open(), "the press closes the panel");
    tip.frame(opened + DELAY * 4, cursor);
    assert!(!tip.is_open(), "and it stays closed while the cursor stays");
}

/// U67: another subject at the same place in the tree is another tooltip.
#[test]
fn a_changed_subject_closes_the_panel_and_starts_the_wait_again() {
    let mut tip = Driven::new(tooltip(Some(DELAY), Some(1)));
    let opened = tip.rest_until_open(Instant::now());
    let cursor = tip.over(20.0);

    tip.rebuild(tooltip(Some(DELAY), Some(2)));
    let seen_at = opened + MS;
    tip.frame(seen_at, cursor);

    assert!(!tip.is_open(), "the panel described the first subject");
    tip.frame(seen_at + DELAY - MS, cursor);
    assert!(!tip.is_open(), "the second subject waits the whole delay");
    tip.frame(seen_at + DELAY, cursor);
    assert!(tip.is_open());
}

/// The same subject rebuilt is the same tooltip: a view rebuild alone closes nothing.
#[test]
fn an_unchanged_subject_keeps_the_panel_open_across_a_rebuild() {
    let mut tip = Driven::new(tooltip(Some(DELAY), Some(1)));
    let opened = tip.rest_until_open(Instant::now());
    let cursor = tip.over(20.0);

    tip.rebuild(tooltip(Some(DELAY), Some(1)));
    tip.frame(opened + MS, cursor);

    assert!(tip.is_open());
}

/// U69: without `after_rest` nothing waits.
#[test]
fn a_tooltip_without_a_rest_delay_opens_at_once() {
    let mut tip = Driven::new(tooltip(None, None));

    let seen = tip.moved(tip.over(20.0));

    assert!(tip.is_open(), "hovering opens it, as before this feature");
    assert_eq!(
        seen.redraw,
        RedrawRequest::NextFrame,
        "and it asks for the frame that paints it",
    );
}

/// U78, first half: the cursor did not move, the row did.
#[test]
fn a_trigger_that_moves_from_under_a_still_cursor_closes_the_panel() {
    let mut tip = rest_mode();
    let opened = tip.rest_until_open(Instant::now());
    let cursor = tip.over(20.0);

    // The list scrolled: the trigger is laid out 200 px further down, the cursor where it was.
    tip.at = Point::new(tip.at.x, tip.at.y + 200.0);
    tip.frame(opened + MS, cursor);

    assert!(!tip.is_open());
}

/// U78, second half: the list narrowed and another row arrived under the still cursor.
#[test]
fn another_subject_arriving_under_a_still_cursor_waits_the_full_delay() {
    let mut tip = Driven::new(tooltip(Some(DELAY), Some(1)));
    let start = Instant::now();
    let cursor = tip.over(20.0);
    // Two seconds into the first row's wait…
    tip.frame(start, cursor);
    tip.frame(start + Duration::from_secs(2), cursor);

    // …the row under the cursor becomes another issue's.
    tip.rebuild(tooltip(Some(DELAY), Some(2)));
    let arrived = start + Duration::from_secs(2) + MS;
    tip.frame(arrived, cursor);

    tip.frame(start + DELAY, cursor);
    assert!(
        !tip.is_open(),
        "3 s from the first row's arrival is 1 s from the second's",
    );
    tip.frame(arrived + DELAY, cursor);
    assert!(tip.is_open());
}

/// U79: no cursor, no panel — and the trigger is still a control.
#[test]
fn with_no_cursor_no_panel_opens_and_the_trigger_still_takes_keys() {
    let mut tip = rest_mode();
    let start = Instant::now();

    for seconds in 0..10 {
        let seen = tip.frame(
            start + Duration::from_secs(seconds),
            mouse::Cursor::Unavailable,
        );
        assert_eq!(seen.redraw, RedrawRequest::Wait, "nothing to wait for");
        assert!(!tip.is_open());
    }

    let seen = tip.send(a_key_press(), mouse::Cursor::Unavailable);
    assert_eq!(seen.messages, [Msg::Key]);
}

/// U80: the wait is the widget tree's, not the view's.
#[test]
fn two_trees_from_the_same_view_keep_separate_rest_state() {
    let mut first = rest_mode();
    let mut second = rest_mode();
    let start = Instant::now();

    first.rest_until_open(start);

    assert!(!second.is_open(), "nothing rested on the second");
    let cursor = second.over(20.0);
    second.frame(start + DELAY, cursor);
    assert!(
        !second.is_open(),
        "the second starts its own wait when its own cursor arrives",
    );
    second.frame(start + DELAY * 2, cursor);
    assert!(second.is_open());
    assert!(first.is_open());
}

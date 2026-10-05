//! The show-delay rule of a tooltip (430 data-model.md, `ShowTimer`).
//!
//! `ShowTimer` decides when a tooltip that waits a delay after the pointer enters is open. It
//! reads no clock and no pointer of its own: both are handed in, so every case here is exact.

use std::time::{Duration, Instant};

use micold_core::tooltip::{Rest, ShowTimer};

const DELAY: Duration = Duration::from_millis(500);
const MS: Duration = Duration::from_millis(1);

/// A timer whose pointer entered the trigger at `start`.
fn entered(start: Instant) -> ShowTimer {
    let mut timer = ShowTimer::default();
    timer.observe(true, start, DELAY);
    timer
}

/// A timer that has opened: the pointer entered at `start` and stayed for the delay.
fn opened(start: Instant) -> ShowTimer {
    let mut timer = entered(start);
    assert!(timer.observe(true, start + DELAY, DELAY).open, "fixture opens");
    timer
}

#[test]
fn it_is_not_open_before_the_delay() {
    let start = Instant::now();
    let mut timer = entered(start);

    let rest = timer.observe(true, start + DELAY - MS, DELAY);

    assert!(!rest.open, "one tick early: {timer:?}");
}

#[test]
fn it_opens_at_the_delay() {
    let start = Instant::now();
    let mut timer = entered(start);

    let rest = timer.observe(true, start + DELAY, DELAY);

    assert!(rest.open, "at the deadline: {timer:?}");
}

#[test]
fn it_stays_open_while_the_pointer_is_over() {
    let start = Instant::now();
    let mut timer = opened(start);

    assert!(timer.observe(true, start + DELAY * 10, DELAY).open);
}

#[test]
fn the_first_observation_starts_the_wait_and_asks_for_a_wake_at_the_deadline() {
    let start = Instant::now();
    let mut timer = ShowTimer::default();

    let rest = timer.observe(true, start, DELAY);

    assert_eq!(
        rest,
        Rest {
            open: false,
            wake_at: Some(start + DELAY)
        }
    );
}

#[test]
fn repeated_observations_never_restart_the_wait() {
    let start = Instant::now();
    let mut timer = entered(start);
    // Many observations (movement) at times before the deadline.
    for step in 1..5 {
        let rest = timer.observe(true, start + MS * (step * 90), DELAY);
        assert!(!rest.open);
        assert_eq!(rest.wake_at, Some(start + DELAY), "deadline stays put");
    }

    assert!(timer.observe(true, start + DELAY, DELAY).open);
}

#[test]
fn leaving_cancels_the_wait() {
    let start = Instant::now();
    let mut timer = entered(start);

    let rest = timer.observe(false, start + DELAY / 2, DELAY);
    assert_eq!(
        rest,
        Rest {
            open: false,
            wake_at: None
        }
    );

    // Coming back starts from nothing.
    timer.observe(true, start + DELAY, DELAY);
    assert!(!timer.observe(true, start + DELAY * 2 - MS, DELAY).open);
    assert!(timer.observe(true, start + DELAY * 2, DELAY).open);
}

#[test]
fn leaving_closes_an_open_tooltip() {
    let start = Instant::now();
    let mut timer = opened(start);

    assert!(!timer.observe(false, start + DELAY, DELAY).open);
}

#[test]
fn a_zero_delay_opens_at_once() {
    let start = Instant::now();
    let mut timer = ShowTimer::default();

    let rest = timer.observe(true, start, Duration::ZERO);

    assert_eq!(
        rest,
        Rest {
            open: true,
            wake_at: None
        }
    );
}

#[test]
fn a_press_closes_it_until_the_pointer_has_left() {
    let start = Instant::now();
    let mut timer = opened(start);

    timer.press();
    assert!(!timer.observe(true, start + DELAY * 2, DELAY).open, "spent");

    timer.observe(false, start + DELAY * 3, DELAY);
    timer.observe(true, start + DELAY * 4, DELAY);
    assert!(timer.observe(true, start + DELAY * 5, DELAY).open, "reopens");
}

#[test]
fn a_press_while_waiting_also_spends_it() {
    let start = Instant::now();
    let mut timer = entered(start);

    timer.press();

    assert!(!timer.observe(true, start + DELAY * 2, DELAY).open);
}

#[test]
fn a_reset_closes_it_and_starts_from_nothing() {
    let start = Instant::now();
    let mut timer = opened(start);

    timer.reset();
    let rest = timer.observe(true, start + DELAY * 2, DELAY);

    assert!(!rest.open, "waiting again: {timer:?}");
    assert_eq!(rest.wake_at, Some(start + DELAY * 3));
}

#[test]
fn a_wake_is_only_asked_for_while_waiting() {
    let start = Instant::now();
    let mut timer = ShowTimer::default();

    assert_eq!(timer.observe(false, start, DELAY).wake_at, None, "away");
    assert!(timer.observe(true, start, DELAY).wake_at.is_some(), "waiting");
    assert_eq!(timer.observe(true, start + DELAY, DELAY).wake_at, None, "open");
    timer.press();
    assert_eq!(timer.observe(true, start + DELAY, DELAY).wake_at, None, "spent");
}

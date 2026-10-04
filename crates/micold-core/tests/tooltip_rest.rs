//! The rest-delay rule of a tooltip (038 contracts/rest-tooltip.md §1, data-model §5).
//!
//! `RestTimer` decides when a tooltip that waits for a cursor at rest is open. It reads no clock
//! and no cursor of its own: both are handed in, so every case here is exact.

use std::time::{Duration, Instant};

use micold_core::tooltip::{Rest, RestTimer, REST_TOLERANCE};

/// The delay the issue list uses (FR-015).
const DELAY: Duration = Duration::from_secs(3);
const MS: Duration = Duration::from_millis(1);
/// Where the cursor comes to rest in these cases.
const AT: (f32, f32) = (100.0, 50.0);

/// A timer whose cursor arrived over the trigger at `AT`, at `start`.
fn resting(start: Instant) -> RestTimer {
    let mut timer = RestTimer::default();
    timer.observe(Some(AT), start, DELAY);
    timer
}

/// A timer that has opened: the cursor arrived at `start` and stayed for the delay.
fn opened(start: Instant) -> RestTimer {
    let mut timer = resting(start);
    let rest = timer.observe(Some(AT), start + DELAY, DELAY);
    assert!(rest.open, "the fixture opens: {timer:?}");
    timer
}

#[test]
fn a_cursor_still_for_the_delay_opens_it() {
    let start = Instant::now();
    let mut timer = resting(start);

    let rest = timer.observe(Some(AT), start + DELAY, DELAY);

    assert!(rest.open, "still for the whole delay: {timer:?}");
}

#[test]
fn a_cursor_still_for_a_millisecond_less_leaves_it_closed() {
    let start = Instant::now();
    let mut timer = resting(start);

    let rest = timer.observe(Some(AT), start + DELAY - MS, DELAY);

    assert!(!rest.open, "1 ms short of the delay: {timer:?}");
    assert_eq!(
        timer,
        RestTimer::Waiting {
            anchor: AT,
            since: start
        },
        "and it is still waiting from the instant the cursor arrived",
    );
}

#[test]
fn a_move_beyond_the_tolerance_restarts_the_wait() {
    let start = Instant::now();
    let mut timer = resting(start);
    let moved_at = start + Duration::from_secs(2);
    let moved_to = (AT.0 + REST_TOLERANCE + 0.5, AT.1);

    timer.observe(Some(moved_to), moved_at, DELAY);

    assert_eq!(
        timer,
        RestTimer::Waiting {
            anchor: moved_to,
            since: moved_at
        },
        "the wait starts again where and when the cursor moved",
    );
    assert!(
        !timer.observe(Some(moved_to), start + DELAY, DELAY).open,
        "3 s after the first arrival is only 1 s after the move",
    );
    assert!(
        timer.observe(Some(moved_to), moved_at + DELAY, DELAY).open,
        "the full delay after the move opens it",
    );
}

#[test]
fn a_move_of_exactly_the_tolerance_is_at_rest_and_the_anchor_does_not_drift() {
    let start = Instant::now();
    let mut timer = resting(start);

    // Exactly 4.0 away, along each axis: figures `f32` holds exactly.
    timer.observe(Some((AT.0 + REST_TOLERANCE, AT.1)), start + MS, DELAY);
    timer.observe(Some((AT.0, AT.1 - REST_TOLERANCE)), start + MS * 2, DELAY);
    assert_eq!(
        timer,
        RestTimer::Waiting {
            anchor: AT,
            since: start
        },
        "a distance of exactly {REST_TOLERANCE} is within tolerance",
    );

    // Many small moves, each within tolerance of the *first* anchor. Had the anchor followed the
    // cursor, the last one — 4.0 from where it started — would be 1.0 from a drifted anchor too,
    // so the drift is shown the other way: step out to 4.0, then one more pixel restarts.
    for (n, dx) in [1.0_f32, 2.0, 3.0, 4.0].into_iter().enumerate() {
        timer.observe(Some((AT.0 + dx, AT.1)), start + MS * (10 + n as u32), DELAY);
    }
    assert_eq!(
        timer,
        RestTimer::Waiting {
            anchor: AT,
            since: start
        },
        "small moves keep the first anchor and the first instant",
    );
    let beyond = (AT.0 + 5.0, AT.1);
    let at = start + MS * 20;
    timer.observe(Some(beyond), at, DELAY);
    assert_eq!(
        timer,
        RestTimer::Waiting {
            anchor: beyond,
            since: at
        },
        "5.0 from the first anchor restarts, though it is 1.0 from the previous position",
    );
}

#[test]
fn a_cursor_that_keeps_moving_never_opens_it() {
    let start = Instant::now();
    let mut timer = RestTimer::default();

    // 10 px every 100 ms for 10 s (SC-004), with a redraw between the moves.
    for step in 0..=100_u32 {
        let now = start + Duration::from_millis(100) * step;
        let at = (10.0 * step as f32, 50.0);
        let moved = timer.observe(Some(at), now, DELAY);
        let redrawn = timer.observe(Some(at), now + Duration::from_millis(50), DELAY);
        assert!(
            !moved.open && !redrawn.open,
            "open after {step} moves: {timer:?}"
        );
    }
}

#[test]
fn once_open_movement_over_the_trigger_keeps_it_open() {
    let start = Instant::now();
    let mut timer = opened(start);

    let rest = timer.observe(Some((AT.0 + 80.0, AT.1 + 12.0)), start + DELAY + MS, DELAY);

    assert_eq!(
        rest,
        Rest {
            open: true,
            wake_at: None
        }
    );
}

#[test]
fn leaving_closes_it_and_the_next_entry_waits_the_full_delay() {
    let start = Instant::now();
    let mut timer = opened(start);
    let left = start + DELAY + MS;

    let rest = timer.observe(None, left, DELAY);
    assert!(!rest.open, "the cursor left the trigger");
    assert_eq!(timer, RestTimer::Away);

    let back = left + MS;
    assert!(!timer.observe(Some(AT), back, DELAY).open);
    assert!(!timer.observe(Some(AT), back + DELAY - MS, DELAY).open);
    assert!(timer.observe(Some(AT), back + DELAY, DELAY).open);
}

#[test]
fn a_press_closes_it_until_the_cursor_has_left() {
    let start = Instant::now();
    let mut timer = opened(start);

    timer.press();

    let long_after = start + DELAY * 10;
    assert_eq!(
        timer.observe(Some(AT), long_after, DELAY),
        Rest {
            open: false,
            wake_at: None
        },
        "pressed: closed however long the cursor then rests",
    );
    timer.observe(None, long_after + MS, DELAY);
    let back = long_after + MS * 2;
    timer.observe(Some(AT), back, DELAY);
    assert!(
        timer.observe(Some(AT), back + DELAY, DELAY).open,
        "after leaving, the trigger opens again after the delay",
    );
}

#[test]
fn a_press_while_waiting_also_spends_it() {
    let start = Instant::now();
    let mut timer = resting(start);

    timer.press();

    assert!(!timer.observe(Some(AT), start + DELAY, DELAY).open);
}

#[test]
fn a_reset_closes_it_and_waits_the_full_delay_again() {
    let start = Instant::now();
    let mut timer = opened(start);

    timer.reset();

    let seen = start + DELAY + MS;
    assert!(!timer.observe(Some(AT), seen, DELAY).open, "reset: closed");
    assert!(!timer.observe(Some(AT), seen + DELAY - MS, DELAY).open);
    assert!(timer.observe(Some(AT), seen + DELAY, DELAY).open);
}

#[test]
fn it_asks_to_be_woken_only_while_waiting() {
    let start = Instant::now();
    let mut timer = RestTimer::default();

    assert_eq!(
        timer.observe(None, start, DELAY).wake_at,
        None,
        "away: nothing to wake for",
    );
    assert_eq!(
        timer.observe(Some(AT), start, DELAY).wake_at,
        Some(start + DELAY),
        "waiting: the instant the delay runs out",
    );
    assert_eq!(
        timer.observe(Some(AT), start + DELAY - MS, DELAY).wake_at,
        Some(start + DELAY),
        "still waiting: the same instant, not a later one",
    );
    assert_eq!(
        timer.observe(Some(AT), start + DELAY, DELAY).wake_at,
        None,
        "open: nothing more to wake for",
    );
    timer.press();
    assert_eq!(
        timer.observe(Some(AT), start + DELAY + MS, DELAY).wake_at,
        None,
        "spent: nothing to wake for",
    );
}

/// A delay the clock cannot add is a tooltip that never opens, not a panic (review A M4 F2).
#[test]
fn a_delay_too_long_for_the_clock_waits_without_a_wake() {
    let mut timer = RestTimer::default();
    let now = Instant::now();

    let rest = timer.observe(Some((10.0, 10.0)), now, Duration::MAX);

    assert!(!rest.open);
    assert_eq!(rest.wake_at, None);
}

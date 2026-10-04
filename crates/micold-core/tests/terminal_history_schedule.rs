//! Feature 041, milestone M3: when a running terminal's history is due for a save (data-model §4,
//! research R6; FR-003, FR-004, FR-007, SC-003).
//!
//! Time is a value here: every reading is `start + some seconds`, and nothing sleeps.

use std::time::{Duration, Instant};

use micold_core::terminal_history::schedule::{SaveSchedule, SAVER_TICK, SAVE_SPACING};

/// The output count of a terminal when its schedule was made.
const STARTED_AT: u64 = 7;

fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

#[test]
fn the_spacing_is_30_seconds_and_the_tick_5() {
    assert_eq!(SAVE_SPACING, secs(30));
    assert_eq!(SAVER_TICK, secs(5));
}

/// U65 (FR-004).
#[test]
fn u65_a_new_schedule_is_not_due_while_the_count_is_unchanged() {
    let start = Instant::now();
    let schedule = SaveSchedule::new(STARTED_AT);

    assert!(!schedule.due(start, STARTED_AT));
    assert!(!schedule.due(start + secs(3600), STARTED_AT));
}

/// U66 (FR-003).
#[test]
fn u66_it_is_due_when_the_count_moved_and_no_save_was_tried() {
    let start = Instant::now();
    let schedule = SaveSchedule::new(STARTED_AT);

    assert!(schedule.due(start, STARTED_AT + 1));
}

/// U67 (FR-003).
#[test]
fn u67_it_is_not_due_just_under_30_s_after_a_save_with_a_moved_count() {
    let start = Instant::now();
    let mut schedule = SaveSchedule::new(STARTED_AT);
    schedule.saved(start, STARTED_AT + 1);

    assert!(!schedule.due(start, STARTED_AT + 2));
    assert!(!schedule.due(start + Duration::from_millis(29_999), STARTED_AT + 2));
}

/// U68 (FR-003).
#[test]
fn u68_it_is_due_at_exactly_30_s_after_a_save_with_a_moved_count() {
    let start = Instant::now();
    let mut schedule = SaveSchedule::new(STARTED_AT);
    schedule.saved(start, STARTED_AT + 1);

    assert!(schedule.due(start + secs(30), STARTED_AT + 2));
    assert!(
        !schedule.due(start + secs(30), STARTED_AT + 1),
        "and not with the count of that save"
    );
}

/// U69 (SC-003): the saver's tick against a terminal that never stops printing.
#[test]
fn u69_ticking_every_5_s_for_600_s_with_a_count_that_always_moves_gives_20_saves() {
    let start = Instant::now();
    let mut schedule = SaveSchedule::new(0);
    let mut saves = 0;

    for tick in 1..=(600 / SAVER_TICK.as_secs()) {
        let now = start + SAVER_TICK * tick as u32;
        let count = tick;
        if schedule.due(now, count) {
            schedule.saved(now, count);
            saves += 1;
        }
    }

    assert_eq!(saves, 20);
}

/// U70 (FR-007).
#[test]
fn u70_a_failed_save_is_due_again_30_s_later_with_the_same_count() {
    let start = Instant::now();
    let moved = STARTED_AT + 1;
    let mut schedule = SaveSchedule::new(STARTED_AT);
    schedule.failed(start);

    assert!(!schedule.due(start + Duration::from_millis(29_999), moved));
    assert!(schedule.due(start + secs(30), moved));
}

/// U71 (FR-027).
#[test]
fn u71_mark_due_makes_it_due_with_an_unchanged_count_spaced_from_the_last_attempt() {
    let start = Instant::now();
    let mut never_tried = SaveSchedule::new(STARTED_AT);
    never_tried.mark_due();
    assert!(
        never_tried.due(start, STARTED_AT),
        "no attempt to space from"
    );

    let mut saved = SaveSchedule::new(STARTED_AT);
    saved.saved(start, STARTED_AT);
    saved.mark_due();
    assert!(!saved.due(start + Duration::from_millis(29_999), STARTED_AT));
    assert!(saved.due(start + secs(30), STARTED_AT));
}

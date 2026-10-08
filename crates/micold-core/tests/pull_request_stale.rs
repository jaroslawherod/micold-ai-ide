//! When a held reading counts as stale (feature 040, R13, data-model §3): older than two
//! refresh intervals, i.e. more than 600 seconds.

use micold_core::pull_request::is_stale;

#[test]
fn a_reading_exactly_ten_minutes_old_is_not_stale() {
    assert!(!is_stale(1_000, 1_600));
}

#[test]
fn a_reading_one_second_over_ten_minutes_old_is_stale() {
    assert!(is_stale(1_000, 1_601));
}

#[test]
fn a_clock_that_went_backwards_is_not_stale() {
    assert!(!is_stale(1_000, 400));
}

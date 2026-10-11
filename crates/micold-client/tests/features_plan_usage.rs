//! Feature 490, T028: what the window holds of the plan's usage, and what it shows of it
//! (data-model *Client state*; US1 s1–s4, US3 s1–s4, s6, s7).

use chrono::FixedOffset;
use micold_client::features::plan_usage::State;
use micold_core::plan_usage::{LimitWindow, UsageReading, WarnPercent};

/// 2026-10-11 12:00:00 UTC.
const NOON: i64 = 1_791_720_000;
const HOUR: i64 = 3600;

fn utc() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

fn window(key: &str, percent: u32, resets_at: Option<i64>) -> LimitWindow {
    LimitWindow {
        key: key.to_string(),
        used_tenths: percent * 10,
        resets_at,
    }
}

/// A short window at 42% resetting at 15:30 and a weekly one at 18% in four days.
fn reading(obtained_at: i64) -> UsageReading {
    UsageReading {
        windows: vec![
            window("five_hour", 42, Some(NOON + 3 * HOUR + 1800)),
            window("seven_day", 18, Some(NOON + 96 * HOUR)),
        ],
        obtained_at,
    }
}

#[test]
fn a_fresh_window_holds_no_reading_and_shows_nothing() {
    let state = State::default();
    assert_eq!(state.reading, None);
    assert!(state.view(NOON, utc()).is_none(), "US3 s7");
}

#[test]
fn a_reading_from_the_service_is_kept_and_a_newer_one_replaces_it() {
    let mut state = State::default();
    state.received(Some(reading(NOON - 60)));
    assert_eq!(state.reading, Some(reading(NOON - 60)));

    let mut newer = reading(NOON);
    newer.windows[0].used_tenths = 500;
    state.received(Some(newer.clone()));
    assert_eq!(
        state.reading,
        Some(newer),
        "US1 s3, US3 s6: the newest wins"
    );
}

#[test]
fn the_service_saying_none_and_a_disconnect_both_clear_the_reading() {
    let mut state = State::default();
    state.received(Some(reading(NOON)));
    state.received(None);
    assert_eq!(state.reading, None, "the service has nothing current");

    state.received(Some(reading(NOON)));
    state.disconnected();
    assert_eq!(
        state.reading, None,
        "a restarted service starts empty (FR-014)"
    );
}

#[test]
fn the_view_is_the_headline_label_and_every_window_in_the_details() {
    let mut state = State::default();
    state.received(Some(reading(NOON - 120)));
    let view = state
        .view(NOON, utc())
        .expect("a current reading is shown (FR-004)");
    assert_eq!(view.label, "42% · 15:30", "US1 s1");
    assert_eq!(
        view.details,
        vec![
            "5-hour: 42%, resets 15:30".to_string(),
            "Weekly: 18%, resets Thu 12:00".to_string(),
            "Updated 11:58".to_string(),
        ],
        "US1 s2 (FR-006)"
    );
}

#[test]
fn reset_times_follow_the_given_offset() {
    let mut state = State::default();
    state.received(Some(reading(NOON)));
    let cest = FixedOffset::east_opt(2 * HOUR as i32).unwrap();
    assert_eq!(
        state.view(NOON, cest).unwrap().label,
        "42% · 17:30",
        "FR-008"
    );
}

#[test]
fn a_passed_window_drops_out_and_none_left_hides_the_indicator() {
    let mut state = State::default();
    state.received(Some(reading(NOON)));
    let after_short_reset = NOON + 4 * HOUR;
    let view = state.view(after_short_reset, utc()).unwrap();
    assert!(view.label.starts_with("18% · "), "US1 s4: {}", view.label);
    assert!(state.view(NOON + 97 * HOUR, utc()).is_none(), "US3 s2");
}

#[test]
fn with_the_switch_off_nothing_is_shown_or_kept() {
    let mut state = State::default();
    state.received(Some(reading(NOON)));
    state.settings(false, WarnPercent::default());
    assert!(state.view(NOON, utc()).is_none(), "US3 s5");
    assert_eq!(state.reading, None, "data-model: switch off → None");

    state.received(Some(reading(NOON)));
    assert!(
        state.view(NOON, utc()).is_none(),
        "a late push while off is not shown"
    );
    state.settings(true, WarnPercent::default());
    assert!(
        state.view(NOON, utc()).is_some(),
        "a push that overtook the switch turning on is shown once it is on"
    );
}

#[test]
fn the_clock_ticks_only_while_a_reading_is_held() {
    let mut state = State::default();
    assert!(!state.ticks(NOON), "R7: no reading, no tick");
    state.received(Some(reading(NOON)));
    assert!(state.ticks(NOON));
    assert!(
        !state.ticks(NOON + 97 * HOUR),
        "every window has reset: nothing left to drop"
    );
    state.disconnected();
    assert!(!state.ticks(NOON));
}

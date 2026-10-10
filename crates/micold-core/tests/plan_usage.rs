//! The plan-usage model (feature 490, data-model *UsageReading*, *LimitWindow*, *CurrentUsage*;
//! research R7, R11, R12).

use chrono::FixedOffset;
use micold_core::plan_usage::{format_reset, LimitWindow, UsageReading, WarnPercent};

/// 2026-10-10 14:00:00 UTC, a Saturday.
const NOW: i64 = 1_791_640_800;
const HOUR: i64 = 3600;

fn window(key: &str, used_tenths: u32, resets_at: Option<i64>) -> LimitWindow {
    LimitWindow {
        key: key.to_owned(),
        used_tenths,
        resets_at,
    }
}

fn reading(windows: Vec<LimitWindow>) -> UsageReading {
    UsageReading {
        windows,
        obtained_at: NOW - 120,
    }
}

fn utc() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

fn threshold(percent: u8) -> WarnPercent {
    WarnPercent::clamped(u64::from(percent))
}

#[test]
fn the_two_documented_windows_have_plain_names() {
    assert_eq!(window("five_hour", 0, None).name(), "5-hour");
    assert_eq!(window("seven_day", 0, None).name(), "Weekly");
}

#[test]
fn an_unknown_window_is_named_after_its_key() {
    assert_eq!(
        window("seven_day_opus", 0, None).name(),
        "Seven day opus",
        "R12"
    );
}

#[test]
fn the_percent_label_rounds_tenths_half_up() {
    assert_eq!(window("five_hour", 425, None).percent_label(), "43%");
    assert_eq!(window("five_hour", 424, None).percent_label(), "42%");
    assert_eq!(window("five_hour", 0, None).percent_label(), "0%");
    assert_eq!(window("five_hour", 1050, None).percent_label(), "105%");
}

#[test]
fn a_window_is_at_or_above_the_threshold_from_its_tenths() {
    assert!(window("five_hour", 800, None).at_or_above(threshold(80)));
    assert!(!window("five_hour", 799, None).at_or_above(threshold(80)));
    assert!(
        window("five_hour", 1001, None).at_or_above(threshold(100)),
        "above 100% counts as above any threshold"
    );
}

#[test]
fn current_drops_windows_whose_reset_has_passed() {
    let usage = reading(vec![
        window("five_hour", 900, Some(NOW)),
        window("seven_day", 180, Some(NOW + HOUR)),
    ])
    .current(NOW)
    .expect("one window is still current");
    assert_eq!(
        usage.windows,
        vec![window("seven_day", 180, Some(NOW + HOUR))],
        "FR-009"
    );
    assert_eq!(
        usage.headline().key,
        "seven_day",
        "the highest still-current window"
    );
}

#[test]
fn a_window_with_no_reset_stays_current() {
    let usage = reading(vec![window("five_hour", 420, None)]).current(NOW + 365 * 24 * HOUR);
    assert!(
        usage.is_some(),
        "FR-009: current until a newer reading replaces it"
    );
}

#[test]
fn current_is_none_when_every_window_has_passed() {
    let passed = reading(vec![
        window("five_hour", 420, Some(NOW - 1)),
        window("seven_day", 180, Some(NOW)),
    ]);
    assert_eq!(passed.current(NOW), None);
}

#[test]
fn the_headline_is_the_highest_window() {
    let usage = reading(vec![
        window("five_hour", 420, Some(NOW + HOUR)),
        window("seven_day", 610, Some(NOW + 48 * HOUR)),
    ])
    .current(NOW)
    .unwrap();
    assert_eq!(usage.headline().key, "seven_day");
}

#[test]
fn a_tie_goes_to_the_earliest_reset_then_the_first() {
    let earliest = reading(vec![
        window("seven_day", 500, Some(NOW + 48 * HOUR)),
        window("five_hour", 500, Some(NOW + HOUR)),
    ])
    .current(NOW)
    .unwrap();
    assert_eq!(
        earliest.headline().key,
        "five_hour",
        "earliest reset wins a tie"
    );

    let absent_last = reading(vec![
        window("other", 500, None),
        window("seven_day", 500, Some(NOW + 48 * HOUR)),
    ])
    .current(NOW)
    .unwrap();
    assert_eq!(
        absent_last.headline().key,
        "seven_day",
        "no reset sorts last"
    );

    let first = reading(vec![window("a_one", 500, None), window("b_two", 500, None)])
        .current(NOW)
        .unwrap();
    assert_eq!(first.headline().key, "a_one", "then the first");
}

#[test]
fn same_windows_ignores_when_the_reading_was_obtained() {
    let one = reading(vec![window("five_hour", 420, Some(NOW + HOUR))]);
    let later = UsageReading {
        obtained_at: NOW + 30,
        ..one.clone()
    };
    assert!(one.same_windows(&later));
    let moved = reading(vec![window("five_hour", 430, Some(NOW + HOUR))]);
    assert!(!one.same_windows(&moved));
}

#[test]
fn the_label_is_the_headline_percent_and_its_reset() {
    let usage = reading(vec![
        window("five_hour", 420, Some(NOW + 90 * 60)),
        window("seven_day", 180, Some(NOW + 48 * HOUR)),
    ])
    .current(NOW)
    .unwrap();
    assert_eq!(usage.label(NOW, utc()), "42% · 15:30");
}

#[test]
fn the_label_has_no_time_when_the_headline_has_no_reset() {
    let usage = reading(vec![window("five_hour", 420, None)])
        .current(NOW)
        .unwrap();
    assert_eq!(usage.label(NOW, utc()), "42%");
}

#[test]
fn the_details_list_every_window_the_warning_and_when_it_was_obtained() {
    let usage = reading(vec![
        window("five_hour", 850, Some(NOW + 90 * 60)),
        window("seven_day", 180, Some(NOW + 48 * HOUR)),
        window("seven_day_opus", 50, None),
    ])
    .current(NOW)
    .unwrap();
    assert_eq!(
        usage.details(threshold(80), NOW, utc()),
        vec![
            "5-hour: 85%, resets 15:30".to_owned(),
            "Weekly: 18%, resets Mon 14:00".to_owned(),
            "Seven day opus: 5%".to_owned(),
            "At or above 80%: 5-hour".to_owned(),
            "Updated 13:58".to_owned(),
        ]
    );
}

#[test]
fn the_details_have_no_warning_line_below_the_threshold() {
    let usage = reading(vec![window("five_hour", 420, Some(NOW + HOUR))])
        .current(NOW)
        .unwrap();
    assert_eq!(
        usage.details(threshold(80), NOW, utc()),
        vec![
            "5-hour: 42%, resets 15:00".to_owned(),
            "Updated 13:58".to_owned()
        ]
    );
}

#[test]
fn warning_lists_the_windows_at_or_above_the_threshold() {
    let usage = reading(vec![
        window("five_hour", 800, Some(NOW + HOUR)),
        window("seven_day", 790, Some(NOW + 48 * HOUR)),
    ])
    .current(NOW)
    .unwrap();
    let keys: Vec<&str> = usage
        .warning(threshold(80))
        .iter()
        .map(|w| w.key.as_str())
        .collect();
    assert_eq!(keys, vec!["five_hour"]);
    assert!(usage.warning(threshold(90)).is_empty());
}

#[test]
fn a_reset_within_a_day_is_a_time_and_beyond_it_has_its_day() {
    assert_eq!(format_reset(NOW + 90 * 60, NOW, utc()), "15:30", "FR-008");
    assert_eq!(format_reset(NOW + 24 * HOUR, NOW, utc()), "14:00");
    assert_eq!(format_reset(NOW + 48 * HOUR, NOW, utc()), "Mon 14:00");
}

#[test]
fn reset_times_are_shown_in_the_given_offset() {
    let cest = FixedOffset::east_opt(2 * 3600).unwrap();
    assert_eq!(format_reset(NOW + 90 * 60, NOW, cest), "17:30", "FR-008");
    let new_york = FixedOffset::west_opt(4 * 3600).unwrap();
    assert_eq!(format_reset(NOW + 11 * HOUR, NOW, new_york), "21:00");
    assert_eq!(format_reset(NOW + 48 * HOUR, NOW, new_york), "Mon 10:00");
}

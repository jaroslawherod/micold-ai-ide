//! Feature 944, US4 (T016): a structured session's plan-limit event as a plan-usage source
//! (contracts/plan-usage-source.md *Parse*; FR-010, FR-012).

use micold_core::plan_usage::rate_limit_event::parse;
use micold_core::plan_usage::{limit_window, LimitWindow, UsageReading};
use serde_json::{json, Value};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{lines_of, RECORDED};

/// Before every reset time the recorded events carry (2026-10-18 05:06 UTC).
const OBTAINED: i64 = 1_792_300_000;

fn window(key: &str, used_tenths: u32, resets_at: Option<i64>) -> LimitWindow {
    LimitWindow {
        key: key.to_owned(),
        used_tenths,
        resets_at,
    }
}

/// A `rate_limit_event` line whose `unifiedWindows` is `windows`.
fn event(windows: Value) -> String {
    json!({
        "type": "rate_limit_event",
        "rate_limit_info": {"status": "allowed", "unifiedWindows": windows},
        "session_id": "<SESSION_ID>"
    })
    .to_string()
}

fn rate_limit_lines(name: &str) -> Vec<String> {
    lines_of(name)
        .into_iter()
        .filter(|line| line.contains(r#""type":"rate_limit_event""#))
        .collect()
}

#[test]
fn every_recorded_event_parses_to_its_two_windows() {
    for name in RECORDED {
        let lines = rate_limit_lines(name);
        assert!(!lines.is_empty(), "{name} records a plan-limit event");
        for line in lines {
            let raw: Value = serde_json::from_str(&line).unwrap();
            let windows = &raw["rate_limit_info"]["unifiedWindows"];
            let expected: Vec<LimitWindow> = ["five_hour", "seven_day"]
                .iter()
                .map(|key| {
                    let w = &windows[key];
                    window(
                        key,
                        (w["utilization"].as_f64().unwrap() * 1000.0).round() as u32,
                        w["resetsAt"].as_i64(),
                    )
                })
                .collect();
            let reading = parse(&line, OBTAINED).unwrap_or_else(|| panic!("{name}: {line}"));
            assert_eq!(
                reading,
                UsageReading {
                    windows: expected,
                    obtained_at: OBTAINED,
                },
                "{name}"
            );
            let names: Vec<String> = reading.windows.iter().map(LimitWindow::name).collect();
            assert_eq!(names, ["5-hour", "Weekly"], "{name}");
        }
    }
}

#[test]
fn text_turn_reads_zero_and_ten_percent() {
    let line = &rate_limit_lines("text-turn")[0];
    assert_eq!(
        parse(line, OBTAINED).unwrap().windows,
        [
            window("five_hour", 0, Some(1_792_363_200)),
            window("seven_day", 100, Some(1_792_904_400)),
        ]
    );
}

#[test]
fn a_line_that_is_not_a_readable_plan_limit_event_is_none() {
    for line in [
        "not json",
        "",
        r#"{"type":"result","rate_limit_info":{"unifiedWindows":{"five_hour":{"utilization":0.1}}}}"#,
        r#"{"type":"rate_limit_event"}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":"none"}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":[]}}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{}}}"#,
    ] {
        assert_eq!(parse(line, OBTAINED), None, "{line}");
    }
}

#[test]
fn an_unusable_window_is_dropped_and_the_rest_kept() {
    let line = event(json!({
        "five_hour": {"utilization": "0.5", "resetsAt": OBTAINED + 3600},
        "seven_day": {"utilization": 0.3, "resetsAt": OBTAINED + 7200},
        "a_negative": {"utilization": -0.1},
        "a_huge": {"utilization": 1e10},
        "a_past": {"utilization": 0.2, "resetsAt": OBTAINED - 1},
        "a_bare_number": -3,
        "a_string_reset": {"utilization": 0.2, "resetsAt": "soon"},
    }));
    assert_eq!(
        parse(&line, OBTAINED).unwrap().windows,
        [window("seven_day", 300, Some(OBTAINED + 7200))]
    );
}

#[test]
fn a_window_without_a_reset_time_is_kept_without_one() {
    let line = event(
        json!({"five_hour": {"utilization": 0.425}, "seven_day": {"utilization": 1.2, "resetsAt": null}}),
    );
    assert_eq!(
        parse(&line, OBTAINED).unwrap().windows,
        [
            window("five_hour", 425, None),
            window("seven_day", 1200, None)
        ]
    );
}

#[test]
fn an_event_with_no_usable_window_is_none() {
    let line = event(json!({
        "five_hour": {"utilization": "0.5"},
        "seven_day": {"utilization": 0.5, "resetsAt": OBTAINED},
    }));
    assert_eq!(parse(&line, OBTAINED), None);
}

#[test]
fn the_top_level_utilization_and_reset_are_not_read() {
    let line = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {"status": "allowed", "utilization": 0.9, "resetsAt": OBTAINED + 60},
    })
    .to_string();
    assert_eq!(parse(&line, OBTAINED), None);
    let line = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "utilization": 0.9,
            "resetsAt": OBTAINED + 60,
            "unifiedWindows": {"five_hour": {"utilization": 0.2}},
        },
    })
    .to_string();
    assert_eq!(
        parse(&line, OBTAINED).unwrap().windows,
        [window("five_hour", 200, None)]
    );
}

#[test]
fn the_tolerance_fixtures_events_give_none_without_a_panic() {
    let lines = rate_limit_lines("insights-tolerance");
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert_eq!(parse(&line, OBTAINED), None, "{line}");
    }
}

/// The shared per-window rule (research R4) drops a percentage whose tenths do not fit a `u32`,
/// for both sources, rather than saturating it.
#[test]
fn the_shared_rule_drops_a_percentage_beyond_u32_tenths() {
    let max = f64::from(u32::MAX) / 10.0;
    assert_eq!(
        limit_window("five_hour", max, None, OBTAINED),
        Some(window("five_hour", u32::MAX, None))
    );
    assert_eq!(limit_window("five_hour", max + 1.0, None, OBTAINED), None);
    assert_eq!(limit_window("five_hour", f64::NAN, None, OBTAINED), None);
    assert_eq!(
        limit_window("five_hour", f64::INFINITY, None, OBTAINED),
        None
    );
}

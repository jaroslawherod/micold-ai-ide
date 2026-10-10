//! Reading plan usage out of Claude Code's status-line input (feature 490, data-model
//! *Status-line extraction*; spec Edge Cases).

use micold_core::plan_usage::status_line::{extract, parse};
use micold_core::plan_usage::LimitWindow;
use serde_json::json;

const OBTAINED: i64 = 1_791_640_800;

fn window(key: &str, used_tenths: u32, resets_at: Option<i64>) -> LimitWindow {
    LimitWindow {
        key: key.to_owned(),
        used_tenths,
        resets_at,
    }
}

#[test]
fn extract_returns_only_the_rate_limits_object() {
    let input = json!({
        "session_id": "abc",
        "transcript_path": "/home/me/.claude/projects/x.jsonl",
        "cwd": "/home/me/project",
        "model": {"id": "claude", "display_name": "Claude"},
        "rate_limits": {
            "five_hour": {"used_percentage": 42.0, "resets_at": OBTAINED + 3600}
        }
    });
    let stdin = serde_json::to_vec(&input).unwrap();
    assert_eq!(
        extract(&stdin),
        Some(json!({"five_hour": {"used_percentage": 42.0, "resets_at": OBTAINED + 3600}}))
    );
}

#[test]
fn extract_is_none_without_json_or_rate_limits() {
    assert_eq!(extract(b"not json"), None);
    assert_eq!(extract(b""), None);
    assert_eq!(extract(br#"{"session_id":"abc"}"#), None);
    assert_eq!(extract(br#"{"rate_limits":"soon"}"#), None, "not an object");
}

#[test]
fn parse_turns_the_documented_windows_into_a_reading() {
    let body = json!({
        "five_hour": {"used_percentage": 42.04, "resets_at": OBTAINED + 3600},
        "seven_day": {"used_percentage": 18, "resets_at": OBTAINED + 86_400}
    });
    let reading = parse(&body, OBTAINED).expect("two usable windows");
    assert_eq!(reading.obtained_at, OBTAINED);
    assert_eq!(
        reading.windows,
        vec![
            window("five_hour", 420, Some(OBTAINED + 3600)),
            window("seven_day", 180, Some(OBTAINED + 86_400)),
        ]
    );
}

#[test]
fn parse_drops_unusable_windows() {
    let body = json!({
        "five_hour": {"used_percentage": -1, "resets_at": OBTAINED + 3600},
        "seven_day": {"used_percentage": "18", "resets_at": OBTAINED + 3600},
        "seven_day_opus": {"resets_at": OBTAINED + 3600},
        "seven_day_sonnet": {"used_percentage": 12, "resets_at": OBTAINED},
        "spend_limit": {"used_percentage": 10, "resets_at": OBTAINED + 3600},
        "other": 5,
        "ok_window": {"used_percentage": 5}
    });
    let reading = parse(&body, OBTAINED).expect("one usable window");
    assert_eq!(reading.windows, vec![window("ok_window", 50, None)]);
}

#[test]
fn parse_keeps_a_percentage_above_100_as_reported() {
    let body = json!({"five_hour": {"used_percentage": 105, "resets_at": OBTAINED + 60}});
    assert_eq!(
        parse(&body, OBTAINED).unwrap().windows,
        vec![window("five_hour", 1050, Some(OBTAINED + 60))]
    );
}

#[test]
fn parse_keeps_an_unknown_window() {
    let body = json!({"seven_day_opus": {"used_percentage": 7.5, "resets_at": OBTAINED + 60}});
    assert_eq!(
        parse(&body, OBTAINED).unwrap().windows,
        vec![window("seven_day_opus", 75, Some(OBTAINED + 60))],
        "R12: a plan that gains a window shows it"
    );
}

#[test]
fn parse_is_none_when_no_window_survives() {
    assert_eq!(parse(&json!({}), OBTAINED), None);
    assert_eq!(
        parse(&json!({"spend_limit": {"used_percentage": 1}}), OBTAINED),
        None
    );
    assert_eq!(parse(&json!([1, 2]), OBTAINED), None);
    assert_eq!(parse(&json!("five_hour"), OBTAINED), None);
}

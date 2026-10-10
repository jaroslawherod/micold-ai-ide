//! Source A (research R1): Claude Code's documented status-line input, whose `rate_limits`
//! object carries each plan window's `used_percentage` and `resets_at`
//! (<https://code.claude.com/docs/en/statusline>). The relay posts only that object; the daemon
//! turns it into a [`UsageReading`].

use serde_json::Value;

use super::{LimitWindow, UsageReading};

/// A gateway spend limit, not plan usage (spec Out of Scope).
const SPEND_LIMIT: &str = "spend_limit";

/// The `rate_limits` object of a status-line input, and nothing else from it; `None` when the
/// input is not JSON or has none.
pub fn extract(stdin: &[u8]) -> Option<Value> {
    let mut input: Value = serde_json::from_slice(stdin).ok()?;
    match input.get_mut("rate_limits")?.take() {
        limits @ Value::Object(_) => Some(limits),
        _ => None,
    }
}

/// The reading in a `rate_limits` body, stamped `obtained_at`; `None` when no window is usable
/// (spec Edge Cases). A window is usable when its value is an object with a non-negative numeric
/// `used_percentage` and a `resets_at`, if any, after `obtained_at`.
pub fn parse(body: &Value, obtained_at: i64) -> Option<UsageReading> {
    let windows: Vec<LimitWindow> = body
        .as_object()?
        .iter()
        .filter(|(key, _)| !key.is_empty() && key.as_str() != SPEND_LIMIT)
        .filter_map(|(key, value)| window(key, value, obtained_at))
        .collect();
    (!windows.is_empty()).then_some(UsageReading {
        windows,
        obtained_at,
    })
}

fn window(key: &str, value: &Value, obtained_at: i64) -> Option<LimitWindow> {
    let percent = value.get("used_percentage")?.as_f64()?;
    if !(percent >= 0.0 && percent.is_finite()) {
        return None;
    }
    let resets_at = value
        .get("resets_at")
        .and_then(Value::as_f64)
        .map(|at| at as i64);
    if resets_at.is_some_and(|at| at <= obtained_at) {
        return None;
    }
    Some(LimitWindow {
        key: key.to_owned(),
        // Non-negative and finite; `as` saturates a value beyond `u32`.
        used_tenths: (percent * 10.0).round() as u32,
        resets_at,
    })
}

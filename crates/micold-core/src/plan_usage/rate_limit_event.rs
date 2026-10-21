//! Source B (feature 944, research R4; contracts/plan-usage-source.md): the plan-limit event a
//! structured session's CLI writes (`SDKRateLimitEvent`), whose `rate_limit_info.unifiedWindows`
//! carries each plan window's used share (`utilization`, `0.01` = 1%) and `resetsAt`. The CLI
//! writes `unifiedWindows` without documenting it, so it is read tolerantly: a window of another
//! shape is left out, and an event with none left is no reading (FR-012).

use serde_json::Value;

use super::{limit_window, UsageReading};

/// The reading in one stdout line of a structured session, stamped `obtained_at`; `None` when the
/// line is not a `rate_limit_event` with a usable window. Windows come in key order; the top-level
/// `utilization` and `resetsAt` name no window and are not read.
pub fn parse(raw: &str, obtained_at: i64) -> Option<UsageReading> {
    let event: Value = serde_json::from_str(raw).ok()?;
    if event.get("type")?.as_str()? != "rate_limit_event" {
        return None;
    }
    let windows: Vec<_> = event
        .get("rate_limit_info")?
        .get("unifiedWindows")?
        .as_object()?
        .iter()
        .filter(|(key, _)| !key.is_empty())
        .filter_map(|(key, value)| {
            let share = value.get("utilization")?.as_f64()?;
            limit_window(key, share * 100.0, value.get("resetsAt"), obtained_at)
        })
        .collect();
    (!windows.is_empty()).then_some(UsageReading {
        windows,
        obtained_at,
    })
}

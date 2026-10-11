//! The Claude plan's usage and its next limit reset (feature 490).
//!
//! Source-neutral (research R1): every source produces a [`UsageReading`], and everything past it
//! — the daemon's one stored reading, the wire, the indicator — reads only that. The one
//! source-specific part is [`status_line`], which reads Claude Code's documented status-line
//! `rate_limits`.
//!
//! Render-free and I/O-free: times are Unix seconds, and the caller passes `now` and the local
//! offset, so a test can pin both. The one exception is [`user_status_line`], which reads the
//! `statusLine` key of Claude Code's settings files so the relay can keep the user's own line.

pub mod status_line;
pub mod user_status_line;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// Whether **Show Claude plan usage** is on in a fresh install (FR-001, Decision 2). The one
/// definition of the default.
pub const DEFAULT_PLAN_USAGE_ENABLED: bool = true;

/// The account's plan usage at one moment (data-model *UsageReading*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageReading {
    /// Non-empty, in the source's key order.
    pub windows: Vec<LimitWindow>,
    /// When the daemon accepted the reading, in Unix seconds (research R5).
    pub obtained_at: i64,
}

/// One limit window of a reading: the 5-hour window, the weekly one, or one a plan gains later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitWindow {
    /// The source's key (`five_hour`, `seven_day`, …). Non-empty.
    pub key: String,
    /// The used percentage times ten, rounded; above 1000 when the source reports over 100%.
    pub used_tenths: u32,
    /// When the window resets, in Unix seconds; `None` when the source gave no time.
    pub resets_at: Option<i64>,
}

impl LimitWindow {
    /// The window's name as the user reads it (research R12).
    pub fn name(&self) -> String {
        match self.key.as_str() {
            "five_hour" => "5-hour".to_owned(),
            "seven_day" => "Weekly".to_owned(),
            key => {
                let spaced = key.replace('_', " ");
                let mut chars = spaced.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => spaced,
                }
            }
        }
    }

    /// The whole percent, rounded half up (`"42%"`).
    pub fn percent_label(&self) -> String {
        format!("{}%", (u64::from(self.used_tenths) + 5) / 10)
    }

    /// Whether the window has reached `threshold` (FR-010).
    pub fn at_or_above(&self, threshold: WarnPercent) -> bool {
        u64::from(self.used_tenths) >= u64::from(threshold.get()) * 10
    }

    fn is_current(&self, now: i64) -> bool {
        self.resets_at.is_none_or(|at| at > now)
    }
}

impl UsageReading {
    /// The windows still current at `now`, with the headline picked among them; `None` once none
    /// remains (FR-009, research R7). A window with no reset time stays current.
    pub fn current(&self, now: i64) -> Option<CurrentUsage> {
        let windows: Vec<LimitWindow> = self
            .windows
            .iter()
            .filter(|w| w.is_current(now))
            .cloned()
            .collect();
        let headline = headline(&windows)?;
        Some(CurrentUsage {
            windows,
            headline,
            obtained_at: self.obtained_at,
        })
    }

    /// Whether `other` holds the same windows, whenever each was obtained (research R6).
    pub fn same_windows(&self, other: &UsageReading) -> bool {
        self.windows == other.windows
    }
}

/// The highest window; a tie goes to the earliest reset (none sorts last), then the first.
fn headline(windows: &[LimitWindow]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, w) in windows.iter().enumerate() {
        let better = match best.map(|b| &windows[b]) {
            None => true,
            Some(b) => {
                w.used_tenths > b.used_tenths
                    || (w.used_tenths == b.used_tenths
                        && w.resets_at.unwrap_or(i64::MAX) < b.resets_at.unwrap_or(i64::MAX))
            }
        };
        if better {
            best = Some(i);
        }
    }
    best
}

/// What a view draws: the current windows of a reading (data-model *CurrentUsage*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUsage {
    /// Current windows only (FR-009).
    pub windows: Vec<LimitWindow>,
    /// Index into `windows` of the window the indicator shows.
    pub headline: usize,
    /// When the reading was obtained, in Unix seconds.
    pub obtained_at: i64,
}

impl CurrentUsage {
    /// The window the indicator shows.
    pub fn headline(&self) -> &LimitWindow {
        &self.windows[self.headline]
    }

    /// The windows at or above `threshold` (FR-010).
    pub fn warning(&self, threshold: WarnPercent) -> Vec<&LimitWindow> {
        self.windows
            .iter()
            .filter(|w| w.at_or_above(threshold))
            .collect()
    }

    /// The indicator's label: `"42% · 15:30"`, or `"42%"` when the headline has no reset.
    pub fn label(&self, now: i64, offset: FixedOffset) -> String {
        let headline = self.headline();
        match headline.resets_at {
            Some(at) => format!(
                "{} · {}",
                headline.percent_label(),
                format_reset(at, now, offset)
            ),
            None => headline.percent_label(),
        }
    }

    /// The details: one line per window, the windows at or above `threshold` when any, and when
    /// the reading was obtained (FR-006, FR-009, FR-010).
    pub fn details(&self, threshold: WarnPercent, now: i64, offset: FixedOffset) -> Vec<String> {
        let mut lines: Vec<String> = self
            .windows
            .iter()
            .map(|w| match w.resets_at {
                Some(at) => format!(
                    "{}: {}, resets {}",
                    w.name(),
                    w.percent_label(),
                    format_reset(at, now, offset)
                ),
                None => format!("{}: {}", w.name(), w.percent_label()),
            })
            .collect();
        let warned = self.warning(threshold);
        if !warned.is_empty() {
            let names: Vec<String> = warned.iter().map(|w| w.name()).collect();
            lines.push(format!(
                "At or above {}%: {}",
                threshold.get(),
                names.join(", ")
            ));
        }
        lines.push(format!(
            "Updated {}",
            local(self.obtained_at, offset).format("%H:%M")
        ));
        lines
    }
}

/// A reset time in the user's time zone: `HH:MM` within 24 hours of `now`, else `Mon HH:MM`
/// (FR-008, research R11).
pub fn format_reset(resets_at: i64, now: i64, offset: FixedOffset) -> String {
    let at = local(resets_at, offset);
    if resets_at - now <= 24 * 3600 {
        at.format("%H:%M").to_string()
    } else {
        at.format("%a %H:%M").to_string()
    }
}

fn local(secs: i64, offset: FixedOffset) -> DateTime<FixedOffset> {
    DateTime::from_timestamp(secs, 0)
        .unwrap_or_default()
        .with_timezone(&offset)
}

/// The warning threshold, a whole percent from 50 to 100 (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WarnPercent(u8);

impl WarnPercent {
    /// The lowest accepted threshold.
    pub const MIN: u8 = 50;
    /// The highest accepted threshold.
    pub const MAX: u8 = 100;
    /// The threshold when none is stored.
    pub const DEFAULT: u8 = 80;

    /// A stored or sent value, clamped into the range (clamp on read, FR-005).
    pub fn clamped(value: u64) -> Self {
        // In range after the clamp, so the narrowing cannot lose anything.
        Self(value.clamp(u64::from(Self::MIN), u64::from(Self::MAX)) as u8)
    }

    /// A typed value, refused out of range with a message naming the range (refuse on save,
    /// FR-005).
    pub fn parse(text: &str) -> Result<Self, String> {
        text.trim()
            .parse::<u8>()
            .ok()
            .filter(|v| (Self::MIN..=Self::MAX).contains(v))
            .map(Self)
            .ok_or_else(|| format!("Enter a whole number from {} to {}", Self::MIN, Self::MAX))
    }

    /// The percent.
    pub fn get(self) -> u8 {
        self.0
    }
}

impl Default for WarnPercent {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

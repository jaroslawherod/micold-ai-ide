//! What this window holds of the Claude plan's usage, and what the app bar shows of it (feature
//! 490, data-model *Client state*).
//!
//! The service keeps the one reading and pushes it (`Welcome.plan_usage`, `PlanUsageChanged`);
//! this holds the last one pushed, forgets it on a disconnect (a restarted service starts empty)
//! and with the switch off. Whether a window is still current is decided here, when drawn
//! (research R7), so the caller passes `now` and the local offset.
//!
//! Render-free: the app bar maps [`View`] to a `UsageIndicator`.

use micold_core::local_time::UtcOffset;
use micold_core::plan_usage::{UsageReading, WarnPercent, DEFAULT_PLAN_USAGE_ENABLED};

/// The window's plan-usage state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// **Show Claude plan usage**, as the service has it.
    pub enabled: bool,
    /// The warning threshold, as the service has it.
    pub warn: WarnPercent,
    /// The last reading the service pushed, if any.
    pub reading: Option<UsageReading>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_PLAN_USAGE_ENABLED,
            warn: WarnPercent::default(),
            reading: None,
        }
    }
}

/// What the indicator draws: the headline label and the tooltip's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    /// `"42% · 15:30"` (FR-004, FR-008).
    pub label: String,
    /// One line per current window, the threshold line when one is reached, and when the reading
    /// was obtained (FR-006, FR-009).
    pub details: Vec<String>,
    /// A current window is at or above the threshold: the warning look (FR-010). Only the look
    /// changes; nothing is raised (FR-011).
    pub warning: bool,
}

impl State {
    /// The service pushed `reading`: the newest wins, and `None` means it has nothing current.
    /// Kept whatever the switch says, so a push that overtakes the switch turning on is not lost;
    /// [`State::view`] shows nothing while the switch is off.
    pub fn received(&mut self, reading: Option<UsageReading>) {
        self.reading = reading;
    }

    /// The connection to the service was lost: a restarted service holds no reading (FR-014).
    pub fn disconnected(&mut self) {
        self.reading = None;
    }

    /// The service's settings arrived. Turning the switch off forgets the reading (FR-002).
    pub fn settings(&mut self, enabled: bool, warn: WarnPercent) {
        self.enabled = enabled;
        self.warn = warn;
        if !enabled {
            self.reading = None;
        }
    }

    /// What to draw at `now` in the zone `offset`, or `None` when nothing is shown: the switch
    /// is off, there is no reading, or none of its windows is still current (FR-009, FR-015).
    pub fn view(&self, now: i64, offset: UtcOffset) -> Option<View> {
        if !self.enabled {
            return None;
        }
        let current = self.reading.as_ref()?.current(now)?;
        Some(View {
            label: current.label(now, offset),
            details: current.details(self.warn, now, offset),
            warning: !current.warning(self.warn).is_empty(),
        })
    }

    /// Whether the window needs the clock at `now`: only while something is shown, to drop a
    /// window whose reset passes (research R7). Once every window has reset there is nothing left
    /// to drop, so an idle window is not woken for it.
    pub fn ticks(&self, now: i64) -> bool {
        self.view(now, UtcOffset::UTC).is_some()
    }
}

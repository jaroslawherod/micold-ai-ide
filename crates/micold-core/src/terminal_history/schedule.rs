//! When a running terminal's history is due for a save (data-model §4, research R6).

use std::time::{Duration, Instant};

/// The least time between two saves of one running terminal (FR-003).
pub const SAVE_SPACING: Duration = Duration::from_secs(30);

/// How often the saver asks each schedule.
pub const SAVER_TICK: Duration = Duration::from_secs(5);

/// The save schedule of one covered running terminal. Pure: the time is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSchedule {
    /// When the last save was tried, successful or not.
    last_attempt: Option<Instant>,
    /// The terminal's output count at the last successful save; `None` means it must be saved.
    saved_count: Option<u64>,
}

impl SaveSchedule {
    /// The schedule of a terminal that just started with `output_count`: nothing to save until
    /// the count moves (FR-004).
    pub fn new(output_count: u64) -> SaveSchedule {
        SaveSchedule {
            last_attempt: None,
            saved_count: Some(output_count),
        }
    }

    /// Whether the terminal is to be saved at `now`, its output count being `output_count`: the
    /// count is not the one last saved, and the last attempt is [`SAVE_SPACING`] ago or more.
    pub fn due(&self, now: Instant, output_count: u64) -> bool {
        self.saved_count != Some(output_count)
            && self
                .last_attempt
                .is_none_or(|last| now.saturating_duration_since(last) >= SAVE_SPACING)
    }

    /// A save at `now` wrote the history as of `output_count`.
    pub fn saved(&mut self, now: Instant, output_count: u64) {
        self.last_attempt = Some(now);
        self.saved_count = Some(output_count);
    }

    /// Whether output has come since the last save, however recent that was. The orderly stop
    /// saves what changed and ignores the spacing (FR-004).
    pub fn needs_save(&self, output_count: u64) -> bool {
        self.saved_count != Some(output_count)
    }

    /// A save was tried at `now` and failed: it is due again [`SAVE_SPACING`] later (FR-007).
    pub fn failed(&mut self, now: Instant) {
        self.last_attempt = Some(now);
    }

    /// The next save is due whatever the output count, still spaced from the last attempt.
    pub fn mark_due(&mut self) {
        self.saved_count = None;
    }
}

//! When a running terminal's history is due for a save (data-model §4, research R6).

use std::time::{Duration, Instant};

/// The least time between two saves of one running terminal (FR-003).
pub const SAVE_SPACING: Duration = Duration::from_secs(30);

/// How often the saver asks each schedule.
pub const SAVER_TICK: Duration = Duration::from_secs(5);

/// The save schedule of one covered running terminal. Pure: the time is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveSchedule {}

impl SaveSchedule {
    /// The schedule of a terminal that just started with `output_count`.
    pub fn new(_output_count: u64) -> SaveSchedule {
        SaveSchedule {}
    }

    /// Whether the terminal is to be saved at `now`, its output count being `output_count`.
    pub fn due(&self, _now: Instant, _output_count: u64) -> bool {
        false
    }

    /// A save at `now` wrote the history as of `output_count`.
    pub fn saved(&mut self, _now: Instant, _output_count: u64) {}

    /// A save was tried at `now` and failed.
    pub fn failed(&mut self, _now: Instant) {}

    /// The next save is due whatever the output count.
    pub fn mark_due(&mut self) {}
}

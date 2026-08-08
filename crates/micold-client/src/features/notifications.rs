//! Notification severity, and the translation into the core's queue (feature 021, T020).
//!
//! T020 asked for `NoticeLevel` and `Notification` to move here, "reconciling against the existing
//! `micold_core::notify` queue rather than duplicating it". The reconciliation turned out to be a
//! deletion: `app::Notification` was never constructed anywhere. Every real notification is a
//! `micold_core::notify::Notification` pushed onto `notify::Queue`, and the snackbar renders that
//! one. The struct in `app.rs` was a leftover from before the queue existed — two names for one
//! concept, which is what the task warned against, already present rather than about to be
//! introduced. It is gone.
//!
//! `NoticeLevel` is not a duplicate and stays. It is the *banner's* severity: it picks the surface
//! fill and nothing else. `notify::Level` additionally decides how long a message lingers, which is
//! a queue concern the banner has no business knowing (FR-032c keeps the two components separate,
//! and `tests/banner_is_not_a_snackbar.rs` holds that line).

use micold_core::notify;

/// How prominently a notification is presented.
///
/// The banner's vocabulary, deliberately narrower than [`notify::Level`]: it chooses a fill, not a
/// duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    /// Something happened that the user should know about, but nothing failed.
    Info,
    /// An action the user asked for could not be completed.
    Error,
}

impl NoticeLevel {
    /// The queue level this severity corresponds to.
    ///
    /// The one place the banner's vocabulary meets the core's. Written as a method on the source
    /// type so a new variant cannot be added without the compiler pointing here — the previous
    /// inline `match` in the reducer would have been just as exhaustive, but far less findable.
    pub fn to_queue_level(self) -> notify::Level {
        match self {
            NoticeLevel::Info => notify::Level::Info,
            NoticeLevel::Error => notify::Level::Error,
        }
    }
}

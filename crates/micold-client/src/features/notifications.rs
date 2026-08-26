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

use std::time::Duration;

use micold_core::notify;

use crate::app::State;

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
    /// Every level, so a gate that reads the set cannot silently miss one added later.
    pub const ALL: [NoticeLevel; 2] = [NoticeLevel::Info, NoticeLevel::Error];

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

/// A failure worth telling the user about, as an outcome (T067a, group E).
///
/// The translation from the banner's `NoticeLevel` to the queue's own level stays here, where
/// `to_queue_level` already lives — a feature emitting this should not have to know the queue has
/// a different vocabulary. `State::notify_error` remains for code that is not a feature reducer.
pub fn error(message: impl Into<String>) -> crate::features::Outcome {
    raised(NoticeLevel::Error, message.into())
}

/// Something the user should know about that is not a failure (T067a, group E).
pub fn info(message: impl Into<String>) -> crate::features::Outcome {
    raised(NoticeLevel::Info, message.into())
}

fn raised(level: NoticeLevel, message: String) -> crate::features::Outcome {
    crate::features::Outcome::NotificationRaised(notify::Notification::new(
        level.to_queue_level(),
        message,
    ))
}

/// What can happen to the notification currently on screen (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// `Message::NotificationDismissed` is `Msg::Dismissed` here and `NotificationsAdvanced` is
/// `Msg::Advanced` — the type says which surface, so the variants do not have to (contract M1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Dismiss the visible notification, promoting the next immediately (FR-032b).
    ///
    /// No index: exactly one is visible, so there is nothing to identify. The index this used to
    /// carry was a position in a stack that no longer exists.
    Dismissed,
    /// Time passed while a notification was on screen, in milliseconds.
    ///
    /// Subscribed to only while the queue is active (`Queue::is_active`), so nothing ticks at rest
    /// (SC-017).
    Advanced(u32),
}

/// This feature's whole reducer surface: one entry point, shape A (contract M2).
///
/// Both arms drive the queue this module owns, so nothing comes back. They were the root's last
/// two inline bodies — written straight against `state.notify` in `app.rs` rather than through a
/// function here — which is why the module had no reducer to route to before now.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Dismissed => dismissed(state),
        Msg::Advanced(elapsed_ms) => advanced(state, elapsed_ms),
    }
    Vec::new()
}

/// The visible notification was dismissed; the next one, if any, takes its place at once.
pub fn dismissed(state: &mut State) {
    state.notify.dismiss();
}

/// A tick of the snackbar clock, in milliseconds, which may retire the visible notification.
pub fn advanced(state: &mut State, elapsed_ms: u32) {
    state
        .notify
        .advance(Duration::from_millis(u64::from(elapsed_ms)));
}

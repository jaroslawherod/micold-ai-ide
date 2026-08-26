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
//!
//! # The vocabulary this feature declares
//!
//! Two transitions in [`Msg`] — `Dismissed` and `Advanced`, the snackbar's two ways of leaving —
//! routed by [`update`], which is pure (data-model.md §1.1 shape A). Advancing the queue is a
//! decision about the queue, so nothing here is matched again in the binary.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! One field in [`State`], reached as `state.notifications`: `queue`, the snackbar's pending
//! notifications in arrival order. It was the root's `notify` and is `queue` here, because
//! `notifications.notify` would say the same thing twice (T028).
//!
//! The banner is not in here. It is derived on every view from what the queue's head says, which is
//! why a wholesale replacement of the queue cannot leave a banner showing something that is no
//! longer in it.

use std::time::Duration;

use micold_core::notify;

/// What this feature remembers.
///
/// One member, and it is not a field this module invented: the queue is `micold_core::notify`'s,
/// because which notification is visible, how long it stays and what is behind it are decisions
/// with no pixels in them. What moved here is *whose* queue it is.
///
/// Spelled `crate::app::State` in the signatures below rather than imported, now that `State` in
/// this module means this struct. The fully-qualified form is deliberate: the scans in
/// `tests/feature_write_isolation.rs` resolve a parameter type by its last `::` segment, so
/// `&mut crate::app::State` is still a root-state operation to them, where `as AppState` would
/// have made every reducer here invisible to the guard that checks what it writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Global messages, newest last: one visible, the rest waiting (FR-032a). Never persisted.
    ///
    /// A message stays until the user dismisses it or it is evicted by newer ones. Nothing clears
    /// these implicitly: a report that vanishes on unrelated activity — a background worktree
    /// re-scan, say — is how these failures became invisible in the first place.
    pub queue: notify::Queue,
}

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
/// two inline bodies — written straight against `state.notifications.queue` in `app.rs` rather than through a
/// function here — which is why the module had no reducer to route to before now.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Dismissed => dismissed(state),
        Msg::Advanced(elapsed_ms) => advanced(state, elapsed_ms),
    }
    Vec::new()
}

/// The visible notification was dismissed; the next one, if any, takes its place at once.
pub fn dismissed(state: &mut crate::app::State) {
    state.notifications.queue.dismiss();
}

/// A tick of the snackbar clock, in milliseconds, which may retire the visible notification.
pub fn advanced(state: &mut crate::app::State, elapsed_ms: u32) {
    state
        .notifications
        .queue
        .advance(Duration::from_millis(u64::from(elapsed_ms)));
}

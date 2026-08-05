//! The notification queue: one visible at a time, the rest waiting (feature 018, T051 — FR-032a,
//! FR-032b; contract §7.8).
//!
//! The **single sanctioned behaviour change** in a feature whose rule is otherwise appearance-only
//! (FR-036a). What ships today stacks up to three banners, all visible at once; Material's snackbar
//! shows exactly one and queues the rest, and an error stays long enough not to be missed.
//!
//! # Why this is in the render-free core
//!
//! Which notification is visible, how long it stays and what is behind it are decisions with no
//! pixels in them. Held here they are testable without a renderer — `tests/notify_queue.rs` drives
//! the whole of it in milliseconds — and the snackbar component stays what a component should be:
//! something that draws what it is handed. A queue inside a widget would only be reachable through
//! one, which is how the old per-modal error fields became untestable.
//!
//! # Time is a parameter
//!
//! Nothing here reads a clock. [`Queue::advance`] is told how much time passed, so the caller owns
//! the clock and the tests own it too. That is the same shape as `cdk::motion`'s `Progress`, and
//! for the same reason: a type that reads the clock itself can only be tested by waiting.

use std::collections::VecDeque;
use std::time::Duration;

/// How prominently a notification is presented, and — because the two are the same decision — how
/// long it stays on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Something happened that the user should know about, but nothing failed.
    Info,
    /// An action the user asked for could not be completed.
    Error,
}

impl Level {
    /// How long a notification at this level stays visible (contract §7.8).
    ///
    /// Material's short and long durations. The *relationship* is the requirement rather than the
    /// two numbers: an error the user missed is a failure they will never learn about, so it must
    /// not be as easy to miss as a success message.
    pub fn duration(self) -> Duration {
        match self {
            Level::Info => Duration::from_secs(4),
            Level::Error => Duration::from_secs(10),
        }
    }
}

/// A transient, user-visible message not owned by any modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub level: Level,
    pub message: String,
}

impl Notification {
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
}

/// One visible notification and an ordered queue of those waiting.
///
/// Construct with [`Default`], push with [`Queue::push`], and drive with [`Queue::advance`] or
/// [`Queue::dismiss`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Queue {
    /// What is on screen, and how long it has been there.
    ///
    /// The elapsed time lives beside the notification rather than in a field of its own so the two
    /// cannot get out of step — promoting the next one replaces both together, which is what makes
    /// "each notification is timed by its own severity" true by construction.
    visible: Option<(Notification, Duration)>,
    pending: VecDeque<Notification>,
}

impl Queue {
    /// The most notifications kept waiting. Older ones are dropped rather than growing without
    /// bound — carried over from the behaviour this replaces, now applied to the queue.
    pub const CAP: usize = 3;

    /// What is on screen, if anything.
    pub fn visible(&self) -> Option<&Notification> {
        self.visible.as_ref().map(|(n, _)| n)
    }

    /// How many are waiting behind it.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Whether the queue needs the clock.
    ///
    /// The application subscribes to time only while this is true, so nothing animates at rest
    /// (FR-039e, SC-017). A queue that always wanted the clock would hold the render loop awake for
    /// the life of the process.
    pub fn is_active(&self) -> bool {
        self.visible.is_some()
    }

    /// Raise a notification: shown at once if nothing is, queued otherwise.
    ///
    /// Duplicates are dropped — of the visible one and of anything already waiting. Repeating an
    /// action that keeps failing should not queue the same sentence behind itself. A message may be
    /// raised again once its twin has cleared, or an action that fails, is dismissed and fails
    /// again would report nothing the second time.
    pub fn push(&mut self, notification: Notification) {
        if self.visible().is_some_and(|v| *v == notification)
            || self.pending.contains(&notification)
        {
            return;
        }
        if self.visible.is_none() {
            self.visible = Some((notification, Duration::ZERO));
            return;
        }
        self.pending.push_back(notification);
        // Over the cap, the *oldest waiting* one goes. Never the visible one: evicting that would
        // take a message off the screen mid-read because something unrelated arrived, which is
        // worse than losing one that was never shown.
        while self.pending.len() > Self::CAP {
            self.pending.pop_front();
        }
    }

    /// Clear the visible notification and promote the next one immediately (FR-032b).
    ///
    /// Immediately rather than on the next tick: a user who dismisses a snackbar to get it out of
    /// the way should see the queue move at once, not watch a gap where the next one should be.
    ///
    /// A no-op when nothing is visible — reachable, because a dismissal can arrive just after the
    /// timeout cleared the same notification.
    pub fn dismiss(&mut self) {
        self.visible = self.pending.pop_front().map(|n| (n, Duration::ZERO));
    }

    /// Account for `elapsed` time, promoting when the visible notification has had its due.
    ///
    /// The promoted one starts from zero rather than inheriting the overshoot: a long frame, or a
    /// caller that advanced by more than was left, must not eat into the next notification's time.
    pub fn advance(&mut self, elapsed: Duration) {
        let Some((notification, shown_for)) = &mut self.visible else {
            return;
        };
        *shown_for += elapsed;
        if *shown_for >= notification.level.duration() {
            self.dismiss();
        }
    }
}

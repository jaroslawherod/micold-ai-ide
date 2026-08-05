//! The notification queue shows one thing at a time (feature 018, T041 — FR-032a, FR-032b).
//!
//! This is the **single sanctioned behaviour change** in a feature whose rule is otherwise
//! appearance-only (FR-036a). Today every notification is visible at once, up to three stacked
//! banners; Material's snackbar shows exactly one and queues the rest. That is a real change to
//! what the user sees happen, so it is specified and tested rather than absorbed into a styling
//! commit.
//!
//! # Why the queue is here and not in the widget
//!
//! Which notification is visible, how long it stays and what is waiting behind it are decisions
//! with no pixels in them. Held in the render-free core they are testable without a renderer, and
//! the snackbar component becomes what it should be: something that draws whatever it is handed.
//! A queue living inside a widget would only be reachable through one.
//!
//! # Time is a parameter
//!
//! Durations are asked for, never slept through. Every test here drives the clock, so the suite has
//! no timing dependence at all — a test that waited four seconds for an info notification to expire
//! would be both slow and flaky, and would still not prove an error waits ten.

use std::time::Duration;

use micold_core::notify::{Level, Notification, Queue};

fn info(message: &str) -> Notification {
    Notification::new(Level::Info, message)
}

fn error(message: &str) -> Notification {
    Notification::new(Level::Error, message)
}

// ---------------------------------------------------------------------------------------------
// One at a time — the requirement this exists for
// ---------------------------------------------------------------------------------------------

/// A fresh queue shows nothing.
#[test]
fn an_empty_queue_has_nothing_visible() {
    let q = Queue::default();
    assert!(q.visible().is_none());
    assert_eq!(q.pending(), 0);
}

/// The first notification becomes visible immediately — a queue that made the first one wait would
/// delay every message in the application by a tick.
#[test]
fn the_first_notification_is_shown_at_once() {
    let mut q = Queue::default();
    q.push(info("worktree created"));
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("worktree created")
    );
    assert_eq!(q.pending(), 0);
}

/// Never more than one visible, however many arrive (FR-032a).
#[test]
fn only_one_notification_is_ever_visible() {
    let mut q = Queue::default();
    q.push(info("first"));
    q.push(info("second"));
    q.push(error("third"));

    assert_eq!(q.visible().map(|n| n.message.as_str()), Some("first"));
    assert_eq!(
        q.pending(),
        2,
        "the other two must be waiting, not shown and not dropped"
    );
}

/// The queue is ordered: what arrived first is shown first.
#[test]
fn pending_notifications_are_shown_in_the_order_they_arrived() {
    let mut q = Queue::default();
    q.push(info("first"));
    q.push(info("second"));
    q.push(info("third"));

    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(q.visible().expect("something is visible").message.clone());
        q.dismiss();
    }
    assert_eq!(seen, ["first", "second", "third"]);
}

// ---------------------------------------------------------------------------------------------
// Dismissal
// ---------------------------------------------------------------------------------------------

/// Manual dismissal promotes the next one **immediately** (FR-032b).
///
/// Not on the next timer tick: a user who dismisses a snackbar to get it out of the way should see
/// the queue move at once, and an implementation that waited for the elapsed-time path would leave
/// a gap with nothing visible while messages were still waiting.
#[test]
fn dismissing_promotes_the_next_one_immediately() {
    let mut q = Queue::default();
    q.push(info("first"));
    q.push(info("second"));

    q.dismiss();
    assert_eq!(q.visible().map(|n| n.message.as_str()), Some("second"));
    assert_eq!(q.pending(), 0);
}

/// Dismissing the last one leaves nothing visible, rather than leaving it on screen forever.
#[test]
fn dismissing_the_last_one_empties_the_queue() {
    let mut q = Queue::default();
    q.push(info("only"));
    q.dismiss();
    assert!(q.visible().is_none());
    assert_eq!(q.pending(), 0);
}

/// Dismissing an empty queue is a no-op rather than a panic. Reachable: the dismiss message can
/// arrive after the timeout already cleared the same notification.
#[test]
fn dismissing_an_empty_queue_does_nothing() {
    let mut q = Queue::default();
    q.dismiss();
    assert!(q.visible().is_none());
}

// ---------------------------------------------------------------------------------------------
// Duration — severity decides how long it stays
// ---------------------------------------------------------------------------------------------

/// An error stays strictly longer than an info (FR-032a).
///
/// The *relationship* is the requirement, not the two numbers: an error the user missed is a
/// failure they will never learn about, so it must not be as easy to miss as a success message.
#[test]
fn an_error_stays_longer_than_an_info() {
    assert!(
        Level::Error.duration() > Level::Info.duration(),
        "an error ({:?}) must outlast an info ({:?}), or a failure is as easy to miss as a success",
        Level::Error.duration(),
        Level::Info.duration()
    );
}

/// Material's short and long durations (contract §7.8).
#[test]
fn the_durations_are_materials() {
    assert_eq!(Level::Info.duration(), Duration::from_secs(4));
    assert_eq!(Level::Error.duration(), Duration::from_secs(10));
}

/// Time advances the queue: once the visible one has had its duration, the next is promoted.
#[test]
fn elapsed_time_promotes_the_next_notification() {
    let mut q = Queue::default();
    q.push(info("first"));
    q.push(info("second"));

    q.advance(Duration::from_secs(1));
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("first"),
        "one second is not four; the first one is still owed its time"
    );

    q.advance(Level::Info.duration());
    assert_eq!(q.visible().map(|n| n.message.as_str()), Some("second"));
}

/// Each notification gets its *own* duration, not the first one's.
///
/// The bug this rules out: a queue that starts one timer and reuses it would give an error the
/// info's four seconds whenever an info happened to be shown first.
#[test]
fn each_notification_is_timed_by_its_own_severity() {
    let mut q = Queue::default();
    q.push(info("info first"));
    q.push(error("error second"));

    q.advance(Level::Info.duration());
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("error second")
    );

    // Four seconds is enough for an info and not for an error.
    q.advance(Level::Info.duration());
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("error second"),
        "the error was cleared after the info's duration — it is being timed by the wrong severity"
    );

    q.advance(Level::Error.duration());
    assert!(q.visible().is_none());
}

/// Time passing with nothing shown is harmless.
#[test]
fn advancing_an_empty_queue_does_nothing() {
    let mut q = Queue::default();
    q.advance(Duration::from_secs(60));
    assert!(q.visible().is_none());
}

/// A promoted notification starts its own duration from the moment it becomes visible, rather than
/// inheriting whatever was left over from the one before it.
#[test]
fn a_promoted_notification_starts_its_duration_fresh() {
    let mut q = Queue::default();
    q.push(info("first"));
    q.push(info("second"));

    // Overshoot: far more than the first one needed. The surplus must not be charged to the second.
    q.advance(Level::Info.duration() * 3);
    assert_eq!(q.visible().map(|n| n.message.as_str()), Some("second"));

    q.advance(Level::Info.duration() - Duration::from_millis(1));
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("second"),
        "the second one was cut short — it inherited the overshoot from the first"
    );
}

// ---------------------------------------------------------------------------------------------
// Deduplication — preserved from today's behaviour, now applied to the queue
// ---------------------------------------------------------------------------------------------

/// A duplicate of the **visible** notification is not enqueued (FR-032a).
///
/// Repeating an action that keeps failing should not queue the same sentence behind itself, which
/// is what today's `contains` check prevents for the stack and what must survive the move.
#[test]
fn a_duplicate_of_the_visible_notification_is_not_enqueued() {
    let mut q = Queue::default();
    q.push(error("could not create the worktree"));
    q.push(error("could not create the worktree"));

    assert_eq!(
        q.pending(),
        0,
        "the repeat queued itself behind its own twin"
    );
}

/// …and a duplicate of one already **waiting** is not enqueued either.
#[test]
fn a_duplicate_of_a_pending_notification_is_not_enqueued() {
    let mut q = Queue::default();
    q.push(info("visible"));
    q.push(error("waiting"));
    q.push(error("waiting"));

    assert_eq!(q.pending(), 1);
}

/// Same text at a different severity is a different notification — an operation that warns and
/// then fails with the same sentence has two things to say.
#[test]
fn the_same_message_at_a_different_level_is_not_a_duplicate() {
    let mut q = Queue::default();
    q.push(info("the session ended"));
    q.push(error("the session ended"));

    assert_eq!(q.pending(), 1);
}

/// A message may repeat once its twin has gone. Otherwise an action that fails, is dismissed and
/// fails again would report nothing the second time.
#[test]
fn a_message_may_be_raised_again_once_it_has_cleared() {
    let mut q = Queue::default();
    q.push(error("could not reach the daemon"));
    q.dismiss();
    q.push(error("could not reach the daemon"));

    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("could not reach the daemon"),
        "a repeat after dismissal was swallowed, so the second failure reported nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// The cap — bounded memory, and it never costs the user what they are reading
// ---------------------------------------------------------------------------------------------

/// The cap drops the **oldest pending** one, never the visible one (FR-032a).
///
/// Dropping the visible one would take a message off screen mid-read because something unrelated
/// arrived, which is worse than losing a message that was never shown.
#[test]
fn the_cap_drops_the_oldest_pending_and_never_the_visible_one() {
    let mut q = Queue::default();
    q.push(info("visible"));
    for i in 0..Queue::CAP + 5 {
        q.push(info(&format!("pending {i}")));
    }

    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some("visible"),
        "the visible notification was evicted by the cap"
    );
    assert_eq!(q.pending(), Queue::CAP);

    // The survivors are the newest ones: the oldest pending were dropped, not the newest arrivals.
    q.dismiss();
    assert_eq!(
        q.visible().map(|n| n.message.as_str()),
        Some(format!("pending {}", 5).as_str()),
        "the cap dropped the wrong end of the queue"
    );
}

/// The cap is a real bound, not a large number that happens not to be reached.
#[test]
fn the_queue_is_bounded() {
    let mut q = Queue::default();
    for i in 0..500 {
        q.push(info(&format!("message {i}")));
    }
    assert!(
        q.pending() <= Queue::CAP,
        "{} pending notifications — the queue grows without bound",
        q.pending()
    );
}

// ---------------------------------------------------------------------------------------------
// What the widget needs to know
// ---------------------------------------------------------------------------------------------

/// The queue reports whether it needs the clock, so the application animates only while something
/// is on screen (FR-039e, SC-017). A snackbar that asked for frames at rest would hold the render
/// loop awake for the life of the process.
#[test]
fn the_queue_only_needs_the_clock_while_something_is_visible() {
    let mut q = Queue::default();
    assert!(!q.is_active(), "an empty queue must not ask for the clock");

    q.push(info("something"));
    assert!(q.is_active());

    q.dismiss();
    assert!(
        !q.is_active(),
        "the queue still wants the clock with nothing to show"
    );
}

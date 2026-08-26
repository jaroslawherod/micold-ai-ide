//! Notifications reach the queue and can be cleared (feature 018, T053 — FR-032a, FR-032b).
//!
//! # What this file used to assert
//!
//! Up to three banners visible at once, dismissed by index, oldest dropped at the cap. That was the
//! behaviour before feature 018 and it is the **single sanctioned behaviour change** in a feature
//! whose rule is otherwise appearance-only (FR-036a): Material's snackbar shows exactly one and
//! queues the rest. The old assertions were not wrong, they were about a design that no longer
//! exists, so they are replaced rather than adjusted.
//!
//! # What it asserts now
//!
//! The *client side* of the queue: that the application's two entry points reach it, that the
//! dismissal message clears the visible one, and that dedup survives at this boundary. The queue's
//! own discipline — ordering, timing by severity, the cap, promotion on dismissal — is
//! `micold-core/tests/notify_queue.rs`, where it needs no renderer and no `State`.

use micold_client::app::{Message, State};
use micold_client::features::notifications::Msg as NotificationsMsg;
use micold_core::notify::Level;

/// Nothing is shown until something is reported.
#[test]
fn a_fresh_application_shows_nothing() {
    let state = State::default();
    assert!(state.notify.visible().is_none());
    assert_eq!(state.notify.pending(), 0);
}

/// Both entry points reach the queue, at their own severities.
#[test]
fn both_entry_points_reach_the_queue() {
    let mut error = State::default();
    error.notify_error("could not create the worktree");
    assert_eq!(error.notify.visible().map(|n| n.level), Some(Level::Error));

    let mut info = State::default();
    info.notify_info("a background session was restarted");
    assert_eq!(info.notify.visible().map(|n| n.level), Some(Level::Info));
}

/// One at a time (FR-032a). The rest wait rather than stacking up the screen.
#[test]
fn only_one_is_visible_and_the_rest_wait() {
    let mut st = State::default();
    st.notify_error("first");
    st.notify_error("second");
    st.notify_info("third");

    assert_eq!(
        st.notify.visible().map(|n| n.message.as_str()),
        Some("first")
    );
    assert_eq!(st.notify.pending(), 2);
}

/// Dismissal clears the visible one and promotes the next immediately (FR-032b).
///
/// No index any more: exactly one is visible, so there is nothing to identify. The index this
/// message used to carry was a position in a stack that no longer exists.
#[test]
fn dismissing_promotes_the_next_one() {
    let mut st = State::default();
    st.notify_error("first");
    st.notify_error("second");

    st.update(Message::Notifications(NotificationsMsg::Dismissed));

    assert_eq!(
        st.notify.visible().map(|n| n.message.as_str()),
        Some("second"),
        "dismissing left a gap instead of promoting what was waiting"
    );
    assert_eq!(st.notify.pending(), 0);
}

/// Dismissing the last one leaves nothing, and dismissing nothing is harmless — the message can
/// arrive just after a timeout cleared the same notification.
#[test]
fn dismissing_the_last_one_is_safe_and_so_is_dismissing_none() {
    let mut st = State::default();
    st.notify_info("only");
    st.update(Message::Notifications(NotificationsMsg::Dismissed));
    assert!(st.notify.visible().is_none());

    st.update(Message::Notifications(NotificationsMsg::Dismissed));
    assert!(st.notify.visible().is_none());
}

/// Dedup survives the move (FR-032a). Repeating an action that keeps failing must not queue the
/// same sentence behind itself.
#[test]
fn a_repeated_failure_does_not_queue_behind_itself() {
    let mut st = State::default();
    st.notify_error("could not reach the daemon");
    st.notify_error("could not reach the daemon");

    assert_eq!(st.notify.pending(), 0);
}

/// Time clears the visible one, and the application drives that clock explicitly — nothing here
/// sleeps.
#[test]
fn elapsed_time_clears_the_visible_notification() {
    let mut st = State::default();
    st.notify_info("a background session was restarted");

    let ms = Level::Info.duration().as_millis() as u32;
    st.update(Message::Notifications(NotificationsMsg::Advanced(ms)));

    assert!(
        st.notify.visible().is_none(),
        "an info notice outlived its own duration"
    );
}

/// An error outlasts an info, so a failure is not as easy to miss as a success (FR-032b).
#[test]
fn an_error_survives_an_info_s_duration() {
    let mut st = State::default();
    st.notify_error("could not create the worktree");

    st.update(Message::Notifications(NotificationsMsg::Advanced(
        Level::Info.duration().as_millis() as u32,
    )));
    assert!(
        st.notify.visible().is_some(),
        "the error was cleared after the info duration — it is being timed by the wrong severity"
    );
}

/// The clock is only wanted while something is on screen (SC-017). A timer running at rest would
/// hold the render loop awake for the life of the process.
#[test]
fn the_queue_wants_the_clock_only_while_it_has_something_to_show() {
    let mut st = State::default();
    assert!(!st.notify.is_active());

    st.notify_error("something");
    assert!(st.notify.is_active());

    st.update(Message::Notifications(NotificationsMsg::Dismissed));
    assert!(!st.notify.is_active());
}

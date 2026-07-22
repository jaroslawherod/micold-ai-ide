//! The global notification surface: the shared replacement for the per-feature error fields
//! whose single modal-specific render sites became unreachable as the UI grew. Feature 005
//! FR-017 (a session that fails to start must tell the user) is the first consumer.

use micold_client::app::{Message, NoticeLevel, Notification, State};

#[test]
fn a_new_state_has_nothing_to_report() {
    assert!(State::default().notifications.is_empty());
}

/// FR-017: a failed session start must be surfaced, not swallowed. The failure is reported by
/// the binary at the PTY boundary; this covers the state it produces.
#[test]
fn fr_017_failed_session_start_surfaces_an_error() {
    let mut st = State::default();
    st.notify_error("Could not start session: No such file or directory (os error 2)");

    assert_eq!(
        st.notifications,
        vec![Notification {
            level: NoticeLevel::Error,
            message: "Could not start session: No such file or directory (os error 2)".to_string(),
        }]
    );
}

#[test]
fn info_and_error_are_distinguishable() {
    let mut st = State::default();
    st.notify_info("Session restarted while you were away.");
    st.notify_error("Could not start session: boom");

    let levels: Vec<_> = st.notifications.iter().map(|n| n.level).collect();
    assert_eq!(levels, vec![NoticeLevel::Info, NoticeLevel::Error]);
}

/// Clicking Dismiss removes exactly that banner and leaves the others.
#[test]
fn dismissing_removes_only_the_chosen_notification() {
    let mut st = State::default();
    st.notify_error("first");
    st.notify_error("second");
    st.notify_error("third");

    st.update(Message::NotificationDismissed(1));

    let messages: Vec<_> = st.notifications.iter().map(|n| &n.message).collect();
    assert_eq!(messages, vec!["first", "third"]);
}

/// A click delivered after the list already shrank must not panic or remove a bystander.
#[test]
fn dismissing_an_out_of_range_index_is_a_no_op() {
    let mut st = State::default();
    st.notify_error("only");

    st.update(Message::NotificationDismissed(7));

    assert_eq!(st.notifications.len(), 1);
}

/// Retrying an action that keeps failing must not stack identical banners.
#[test]
fn repeating_the_same_failure_does_not_duplicate_it() {
    let mut st = State::default();
    st.notify_error("Could not start session: boom");
    st.notify_error("Could not start session: boom");

    assert_eq!(st.notifications.len(), 1);
}

/// Unrelated failures in a row must not grow an unbounded banner stack.
#[test]
fn the_notification_stack_is_bounded_keeping_the_newest() {
    let mut st = State::default();
    for i in 0..6 {
        st.notify_error(format!("failure {i}"));
    }

    let messages: Vec<_> = st.notifications.iter().map(|n| n.message.clone()).collect();
    assert_eq!(messages, vec!["failure 3", "failure 4", "failure 5"]);
}

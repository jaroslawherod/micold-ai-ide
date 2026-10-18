//! Feature 489 (FR-007, FR-008, FR-019; M3, T040): the reconnect delta of the per-daemon state
//! machine in `contracts/remote-daemon.md`.

use micold_core::daemons::{
    DaemonEvent, DaemonId, DaemonState, DaemonStates, ATTEMPTS_STOPPED, UNREACHABLE_AFTER_FAILURES,
};

fn id(n: u32) -> DaemonId {
    DaemonId(n)
}

fn two() -> DaemonStates {
    let mut states = DaemonStates::default();
    states.insert(id(1), DaemonState::Starting);
    states.insert(id(2), DaemonState::Starting);
    states
}

fn reconnecting(reason: &str) -> DaemonState {
    DaemonState::Reconnecting {
        reason: reason.into(),
    }
}

/// A daemon whose connection was established and then lost.
fn dropped() -> DaemonStates {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    states.apply(id(1), DaemonEvent::Lost("eof".into()));
    states
}

#[test]
fn an_established_link_lost_is_reconnecting_with_the_reason() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    let change = states
        .apply(id(1), DaemonEvent::Lost("eof".into()))
        .unwrap();
    assert_eq!(change.to, reconnecting("eof"));
    assert!(states.should_dial(id(1)), "attempts are in progress");
}

#[test]
fn a_failed_attempt_updates_the_reason_and_not_the_kind() {
    let mut states = dropped();
    let change = states
        .apply(id(1), DaemonEvent::DialFailed("no route".into()))
        .unwrap();
    assert_eq!(change.to, reconnecting("no route"));
    // The loop reports the end of an attempt as a loss too; it must not erase the reason.
    assert_eq!(states.apply(id(1), DaemonEvent::Lost("gone".into())), None);
    assert_eq!(states.state(id(1)), Some(&reconnecting("no route")));
}

#[test]
fn an_attempt_that_succeeds_connects() {
    let mut states = dropped();
    let change = states.apply(id(1), DaemonEvent::Connected).unwrap();
    assert_eq!(change.to, DaemonState::Connected);
}

#[test]
fn giving_up_is_unreachable_and_stops_the_attempts() {
    let mut states = dropped();
    let change = states.apply(id(1), DaemonEvent::GiveUp).unwrap();
    assert_eq!(
        change.to,
        DaemonState::Unreachable {
            reason: ATTEMPTS_STOPPED.into()
        }
    );
    assert!(!states.should_dial(id(1)));
    assert!(states.established(id(1)), "it was connected before");
    // Nothing a late report says restarts them.
    assert_eq!(
        states.apply(id(1), DaemonEvent::DialFailed("x".into())),
        None
    );
    assert!(!states.should_dial(id(1)));
}

#[test]
fn giving_up_applies_only_while_reconnecting() {
    let mut states = two();
    assert_eq!(states.apply(id(1), DaemonEvent::GiveUp), None);
    states.apply(id(1), DaemonEvent::Connected);
    assert_eq!(states.apply(id(1), DaemonEvent::GiveUp), None);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Connected));
}

#[test]
fn an_explicit_reconnect_after_giving_up_starts_again() {
    let mut states = dropped();
    states.apply(id(1), DaemonEvent::GiveUp);
    let before = states.epoch(id(1));
    let change = states.apply(id(1), DaemonEvent::Reconnect).unwrap();
    assert_eq!(change.to, DaemonState::Starting);
    assert_ne!(
        states.epoch(id(1)),
        before,
        "a new epoch restarts the actor (T087)"
    );
    assert!(states.should_dial(id(1)));
}

#[test]
fn an_explicit_reconnect_while_reconnecting_restarts_the_attempts_without_a_change() {
    let mut states = dropped();
    let before = states.epoch(id(1));
    assert_eq!(states.apply(id(1), DaemonEvent::Reconnect), None);
    assert_eq!(states.state(id(1)), Some(&reconnecting("eof")));
    assert_ne!(
        states.epoch(id(1)),
        before,
        "a new epoch restarts the actor"
    );
}

#[test]
fn a_reconnect_of_a_connected_or_stopped_daemon_does_nothing() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    let epoch = states.epoch(id(1));
    assert_eq!(states.apply(id(1), DaemonEvent::Reconnect), None);
    assert_eq!(states.epoch(id(1)), epoch);
    states.apply(id(1), DaemonEvent::Stop);
    assert_eq!(states.apply(id(1), DaemonEvent::Reconnect), None);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Stopped));
}

#[test]
fn a_start_forgets_that_a_connection_was_established() {
    let mut states = dropped();
    assert!(states.established(id(1)));
    states.apply(id(1), DaemonEvent::Stop);
    states.apply(id(1), DaemonEvent::Start);
    assert!(!states.established(id(1)));
}

#[test]
fn a_daemon_that_never_connected_keeps_the_491_debounce_and_retries() {
    let mut states = two();
    for _ in 1..UNREACHABLE_AFTER_FAILURES {
        assert_eq!(
            states.apply(id(1), DaemonEvent::DialFailed("no".into())),
            None
        );
    }
    let change = states
        .apply(id(1), DaemonEvent::DialFailed("no".into()))
        .unwrap();
    assert!(matches!(change.to, DaemonState::Unreachable { .. }));
    assert!(
        states.should_dial(id(1)),
        "local runtimes keep their silent retries after a failed start"
    );
}

#[test]
fn a_mismatch_still_outranks_a_reconnect_attempt() {
    let mut states = dropped();
    let refused = DaemonEvent::Refused {
        client: "2".into(),
        daemon: "1".into(),
    };
    let change = states.apply(id(1), refused).unwrap();
    assert!(matches!(change.to, DaemonState::VersionMismatch { .. }));
}

#[test]
fn one_daemons_transition_touches_no_other() {
    let mut states = two();
    states.apply(id(2), DaemonEvent::Connected);
    states.apply(id(1), DaemonEvent::Connected);
    states.apply(id(1), DaemonEvent::Lost("eof".into()));
    states.apply(id(1), DaemonEvent::GiveUp);
    assert_eq!(states.state(id(2)), Some(&DaemonState::Connected));
    assert!(states.should_dial(id(2)));
}

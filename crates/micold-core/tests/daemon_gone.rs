//! Feature 686 (FR-011; M8, T057): a daemon whose pod is found gone becomes `Stopped` with the
//! reason, and Start from it clears the reason.

use micold_core::daemons::{DaemonEvent, DaemonId, DaemonState, DaemonStates};

const REASON: &str = "stopped: the pod was deleted";

fn id(n: u32) -> DaemonId {
    DaemonId(n)
}

fn two(first: DaemonEvent) -> DaemonStates {
    let mut states = DaemonStates::default();
    for n in [1, 2] {
        states.insert(id(n), DaemonState::Starting);
        states.apply(id(n), DaemonEvent::Connected);
    }
    states.apply(id(1), first);
    states
}

#[test]
fn a_reconnecting_daemon_whose_pod_is_gone_stops_with_the_reason() {
    let mut states = two(DaemonEvent::Lost("eof".into()));
    assert!(matches!(
        states.state(id(1)),
        Some(DaemonState::Reconnecting { .. })
    ));
    let change = states
        .apply(id(1), DaemonEvent::Gone(REASON.into()))
        .unwrap();
    assert_eq!(change.to, DaemonState::Stopped);
    assert_eq!(states.stopped_reason(id(1)), Some(REASON));
    assert!(!states.should_dial(id(1)));
}

#[test]
fn an_unreachable_daemon_whose_pod_is_gone_stops_with_the_reason() {
    let mut states = two(DaemonEvent::Lost("eof".into()));
    states.apply(id(1), DaemonEvent::GiveUp);
    assert!(matches!(
        states.state(id(1)),
        Some(DaemonState::Unreachable { .. })
    ));
    states.apply(id(1), DaemonEvent::Gone(REASON.into()));
    assert_eq!(states.state(id(1)), Some(&DaemonState::Stopped));
    assert_eq!(states.stopped_reason(id(1)), Some(REASON));
}

#[test]
fn start_from_stopped_clears_the_reason_and_another_daemon_is_untouched() {
    let mut states = two(DaemonEvent::Lost("eof".into()));
    states.apply(id(1), DaemonEvent::Gone(REASON.into()));
    states.apply(id(1), DaemonEvent::Start);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Starting));
    assert_eq!(states.stopped_reason(id(1)), None);
    assert_eq!(states.state(id(2)), Some(&DaemonState::Connected));
    assert_eq!(states.stopped_reason(id(2)), None);
}

#[test]
fn a_stopped_daemon_ignores_a_late_gone_and_a_user_stop_has_no_reason() {
    let mut states = two(DaemonEvent::Stop);
    assert_eq!(states.apply(id(1), DaemonEvent::Gone(REASON.into())), None);
    assert_eq!(states.stopped_reason(id(1)), None);
}

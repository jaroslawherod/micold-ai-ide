//! Feature 491 (FR-007, FR-008, FR-009, SC-002): the per-daemon state machine of
//! `contracts/client-daemon-routing.md`.

use micold_core::daemons::{
    DaemonEntry, DaemonEvent, DaemonId, DaemonName, DaemonRegistry, DaemonRuntime, DaemonState,
    DaemonStates, UNREACHABLE_AFTER_FAILURES,
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

fn lost(reason: &str) -> DaemonEvent {
    DaemonEvent::Lost(reason.into())
}

fn refused() -> DaemonEvent {
    DaemonEvent::Refused {
        client: "2".into(),
        daemon: "1".into(),
    }
}

#[test]
fn a_handshake_connects_a_starting_daemon() {
    let mut states = two();
    let change = states.apply(id(1), DaemonEvent::Connected).unwrap();
    assert_eq!(change.id, id(1));
    assert_eq!(change.to, DaemonState::Connected);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Connected));
}

#[test]
fn a_failing_dial_is_debounced_and_then_unreachable() {
    let mut states = two();
    for _ in 1..UNREACHABLE_AFTER_FAILURES {
        assert_eq!(
            states.apply(id(1), DaemonEvent::DialFailed("no".into())),
            None
        );
        assert_eq!(states.state(id(1)), Some(&DaemonState::Starting));
    }
    let change = states
        .apply(id(1), DaemonEvent::DialFailed("no".into()))
        .unwrap();
    assert_eq!(
        change.to,
        DaemonState::Unreachable {
            reason: "no".into()
        }
    );
}

#[test]
fn a_success_resets_the_debounce() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::DialFailed("no".into()));
    states.apply(id(1), DaemonEvent::Connected);
    states.apply(id(1), lost("eof"));
    states.apply(id(1), DaemonEvent::Start);
    for _ in 1..UNREACHABLE_AFTER_FAILURES {
        assert_eq!(
            states.apply(id(1), DaemonEvent::DialFailed("no".into())),
            None
        );
    }
}

#[test]
fn an_established_connection_lost_is_unreachable_at_once() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    let change = states.apply(id(1), lost("eof")).unwrap();
    assert_eq!(
        change.to,
        DaemonState::Unreachable {
            reason: "eof".into()
        }
    );
}

#[test]
fn a_refusal_marks_version_mismatch_with_both_versions() {
    let mut states = two();
    let change = states.apply(id(1), refused()).unwrap();
    assert_eq!(
        change.to,
        DaemonState::VersionMismatch {
            client: "2".into(),
            daemon: "1".into()
        }
    );
    // Retries keep it a mismatch, and no new change is reported.
    assert_eq!(states.apply(id(1), refused()), None);
    assert_eq!(
        states.apply(id(1), DaemonEvent::DialFailed("x".into())),
        None
    );
}

#[test]
fn unreachable_comes_back_connected_without_a_start() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    states.apply(id(1), lost("eof"));
    let change = states.apply(id(1), DaemonEvent::Connected).unwrap();
    assert_eq!(change.to, DaemonState::Connected);
}

#[test]
fn stop_wins_from_any_state_and_stopped_is_not_reconnected() {
    let mut states = two();
    states.apply(id(1), DaemonEvent::Connected);
    let change = states.apply(id(1), DaemonEvent::Stop).unwrap();
    assert_eq!(change.to, DaemonState::Stopped);
    assert!(!states.should_dial(id(1)));
    // A late handshake or loss report does not resurrect it.
    assert_eq!(states.apply(id(1), DaemonEvent::Connected), None);
    assert_eq!(states.apply(id(1), lost("eof")), None);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Stopped));
    // Start is the only way out.
    let change = states.apply(id(1), DaemonEvent::Start).unwrap();
    assert_eq!(change.to, DaemonState::Starting);
    assert!(states.should_dial(id(1)));
}

#[test]
fn a_transition_on_one_daemon_leaves_every_other_untouched() {
    let mut states = two();
    states.apply(id(2), DaemonEvent::Connected);
    let before = states.state(id(2)).cloned();
    for event in [lost("x"), refused(), DaemonEvent::Stop, DaemonEvent::Start] {
        if let Some(change) = states.apply(id(1), event) {
            assert_eq!(change.id, id(1));
        }
        assert_eq!(states.state(id(2)).cloned(), before);
    }
}

#[test]
fn an_unknown_daemon_has_no_state_and_no_change() {
    let mut states = two();
    assert_eq!(states.apply(id(9), DaemonEvent::Connected), None);
    assert_eq!(states.state(id(9)), None);
    assert!(!states.should_dial(id(9)));
}

#[test]
fn launch_state_follows_auto_start_and_support() {
    let entry = |n: u32, name: &str, runtime, auto_start| DaemonEntry {
        id: DaemonId(n),
        name: DaemonName::new(name).unwrap(),
        runtime,
        auto_start,
    };
    let registry = DaemonRegistry::new(
        vec![
            entry(1, "Host", DaemonRuntime::Host, true),
            entry(2, "Off", DaemonRuntime::Host, false),
            entry(
                3,
                "Odd",
                DaemonRuntime::Unsupported {
                    kind: "ssh".into(),
                    raw: serde_json::json!({"kind": "ssh"}),
                },
                true,
            ),
        ],
        4,
    );
    let states = DaemonStates::at_launch(&registry);
    assert_eq!(states.state(id(1)), Some(&DaemonState::Starting));
    assert_eq!(states.state(id(2)), Some(&DaemonState::Stopped));
    assert_eq!(states.state(id(3)), Some(&DaemonState::Stopped));
    assert!(states.should_dial(id(1)));
    assert!(!states.should_dial(id(2)));
    assert!(!states.should_dial(id(3)));
}

#[test]
fn removing_a_daemon_forgets_only_its_state() {
    let mut states = two();
    states.remove(id(1));
    assert_eq!(states.state(id(1)), None);
    assert_eq!(states.state(id(2)), Some(&DaemonState::Starting));
}

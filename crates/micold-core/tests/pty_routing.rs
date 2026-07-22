//! T041 — per-session output routing/isolation, no cross-talk (FR-019, SC-005).
//! Routing only — VT grid rendering is gui-side (analyze finding C1).

use micold_core::session::SessionId;
use micold_core::terminal::SessionRouter;

#[test]
fn output_is_routed_to_the_owning_session_only() {
    let a = SessionId::new();
    let b = SessionId::new();
    let mut router = SessionRouter::new();
    router.register(a);
    router.register(b);

    router.route(a, b"hello ");
    router.route(b, b"world");
    router.route(a, b"A");

    assert_eq!(router.buffer(a), b"hello A");
    assert_eq!(router.buffer(b), b"world");
}

#[test]
fn output_for_unregistered_session_is_dropped() {
    let a = SessionId::new();
    let ghost = SessionId::new();
    let mut router = SessionRouter::new();
    router.register(a);

    router.route(ghost, b"leak");
    // Nothing leaks into a registered session.
    assert!(router.buffer(a).is_empty());
    assert!(router.buffer(ghost).is_empty());
}

#[test]
fn removed_session_no_longer_receives_output() {
    let a = SessionId::new();
    let mut router = SessionRouter::new();
    router.register(a);
    router.route(a, b"x");
    router.remove(a);
    router.route(a, b"y");
    assert!(router.buffer(a).is_empty());
}

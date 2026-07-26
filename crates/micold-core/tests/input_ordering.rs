//! T039 [US2] — `SessionInput.serial` is monotonic and input is never coalesced, dropped, or
//! reordered across a detach/reattach boundary (data-model G2; protocol.md §7; spec Edge:
//! clock/ordering).
//!
//! These tests pin the *contract*, not an implementation: the client's [`InputSeq`] and the daemon's
//! [`InputReceiver`] are the two ends of it, and they must agree with no hidden coupling — a serial
//! stamped by one is classified by the other exactly as the protocol says.

use micold_core::input::{InputOutcome, InputReceiver, InputSeq};

#[test]
fn the_client_serial_is_dense_and_monotonic() {
    let mut seq = InputSeq::new();
    let serials: Vec<u64> = (0..5).map(|_| seq.stamp()).collect();
    // Dense (0,1,2,…) so a receiver can detect a gap as loss, and strictly increasing.
    assert_eq!(serials, vec![0, 1, 2, 3, 4]);
    assert_eq!(
        seq.peek(),
        5,
        "peek reflects the next serial without consuming it"
    );
}

#[test]
fn every_in_order_serial_is_applied_exactly_once() {
    let mut seq = InputSeq::new();
    let mut rx = InputReceiver::new();
    for _ in 0..100 {
        let serial = seq.stamp();
        assert_eq!(
            rx.accept(serial),
            InputOutcome::Apply,
            "a contiguous stream must apply every serial"
        );
    }
    assert_eq!(
        rx.expected(),
        100,
        "the log advanced once per applied serial"
    );
}

#[test]
fn a_replayed_or_reordered_serial_is_stale_and_never_reapplied() {
    let mut rx = InputReceiver::new();
    assert_eq!(rx.accept(0), InputOutcome::Apply);
    assert_eq!(rx.accept(1), InputOutcome::Apply);
    // A duplicate of an already-applied serial: dropped, not re-applied (no coalescing/duplication).
    assert_eq!(rx.accept(1), InputOutcome::Stale);
    assert_eq!(rx.accept(0), InputOutcome::Stale);
    // The high-water mark is unmoved by stale serials, so the real next one still applies.
    assert_eq!(rx.accept(2), InputOutcome::Apply);
    assert_eq!(rx.expected(), 3);
}

#[test]
fn a_gap_is_surfaced_as_loss_never_silently_swallowed() {
    let mut rx = InputReceiver::new();
    assert_eq!(rx.accept(0), InputOutcome::Apply);
    // Serial 1 and 2 never arrived; 3 shows up. The receiver reports the exact loss loudly.
    assert_eq!(rx.accept(3), InputOutcome::Lost { missing: 2 });
    // It resyncs past the arrived serial rather than re-reporting the same gap forever.
    assert_eq!(rx.accept(4), InputOutcome::Apply);
    assert_eq!(rx.expected(), 5);
}

#[test]
fn input_survives_a_detach_reattach_boundary_without_loss_or_reordering() {
    // The whole point of the serial: the client counter is NOT reset on detach, and the daemon's
    // expectation is per-session (not per-connection), so continuity is provable across a reconnect.
    let mut seq = InputSeq::new();
    let mut rx = InputReceiver::new();

    // --- first attachment: type a few keystrokes ---
    for _ in 0..3 {
        assert_eq!(rx.accept(seq.stamp()), InputOutcome::Apply);
    }

    // --- the client detaches and reattaches over a new connection ---
    // The reconnect touches only the transport: `seq` is the SAME counter (never re-created via
    // `InputSeq::new()`, never reset), and `rx` is per-session so it outlives the connection too.

    // --- second attachment: keep typing; serials continue seamlessly ---
    for _ in 0..3 {
        assert_eq!(
            rx.accept(seq.stamp()),
            InputOutcome::Apply,
            "post-reattach input must apply with no gap and no reorder"
        );
    }

    assert_eq!(seq.peek(), 6, "the counter never reset across the boundary");
    assert_eq!(
        rx.expected(),
        6,
        "the daemon applied all six in order, none lost"
    );
}

#[test]
fn input_lost_in_flight_at_a_detach_is_detected_on_reattach() {
    // Edge (spec): input typed immediately before a detach must not be *silently* lost. If the drop
    // actually severed a keystroke mid-flight, the resumed stream's first serial is ahead of the
    // daemon's expectation — and that is reported as loss, not papered over as success.
    let mut seq = InputSeq::new();
    let mut rx = InputReceiver::new();

    assert_eq!(rx.accept(seq.stamp()), InputOutcome::Apply); // 0 applied
    let _severed = seq.stamp(); // 1 stamped by the client but lost when the connection dropped

    // On reattach the client keeps counting from 2 — the daemon sees the gap and surfaces it.
    assert_eq!(rx.accept(seq.stamp()), InputOutcome::Lost { missing: 1 });
}

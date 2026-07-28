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
    //
    // NOTE (BUG-006): reusing one counter object is the *reconnect* case, and only that. It is not
    // the binding case for this feature — the daemon is designed to outlive the UI, so the ordinary
    // event is a new client *process*, which re-creates the counter. That case is covered by
    // `a_restarted_client_seeded_from_the_daemon_drives_a_session_it_did_not_start` below; this test
    // deliberately keeps the same-counter premise, so do not read it as covering a client restart.

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

// --- BUG-006: the client *process* restart boundary (T112, FR-028a) ------------------------------
//
// The two tests below pin the regression from both sides: what goes wrong when a fresh counter meets
// a surviving receiver, and that seeding from the daemon's authoritative position fixes it. The
// second is the contract; the first exists so a future refactor that quietly drops the seeding fails
// here with the actual symptom named, rather than only failing an integration test far away.

#[test]
fn an_unseeded_restarted_client_has_every_keystroke_discarded() {
    // This is the bug as observed: install a new `.deb` (or just quit and reopen), and every session
    // that predates the restart stops accepting input while still rendering normally.
    let mut rx = InputReceiver::new();

    // --- client generation 1 drives the session for a while ---
    let mut gen1 = InputSeq::new();
    for _ in 0..40 {
        assert_eq!(rx.accept(gen1.stamp()), InputOutcome::Apply);
    }
    assert_eq!(rx.expected(), 40);

    // --- the UI restarts; the daemon and the session do not ---
    // A new process means a new stamper, and `InputSeq::new()` starts at 0. Nothing resets `rx`:
    // a detach deliberately leaves the session running with no client attached (G4/FR-002).
    let mut gen2 = InputSeq::new();

    // Every keystroke the user types is below the high-water mark, so every one is discarded.
    for _ in 0..40 {
        assert_eq!(
            rx.accept(gen2.stamp()),
            InputOutcome::Stale,
            "an unseeded fresh counter is behind the receiver, so its input is dropped"
        );
    }
    assert_eq!(
        rx.expected(),
        40,
        "40 keystroke batches reached the daemon and not one advanced the log"
    );

    // The session is not permanently dead — it comes back only once the client has burned through
    // the previous generation's serial count into the void. That is the user-visible "read-only
    // until I restart the daemon" symptom.
    assert_eq!(rx.accept(gen2.stamp()), InputOutcome::Apply);
}

#[test]
fn a_restarted_client_seeded_from_the_daemon_drives_a_session_it_did_not_start() {
    // The fix (FR-028a): the daemon's expectation is authoritative and travels in the catalog
    // snapshot as `SessionSummary::input_serial`; a client with no counter of its own for a session
    // adopts it instead of starting at 0.
    let mut rx = InputReceiver::new();

    let mut gen1 = InputSeq::new();
    for _ in 0..40 {
        assert_eq!(rx.accept(gen1.stamp()), InputOutcome::Apply);
    }

    // --- the UI restarts and resyncs from the snapshot ---
    let published = rx.expected(); // what `overlay_live_summaries` puts on the wire
    let mut gen2 = InputSeq::resume_from(published);

    // The very first keystroke of the new process lands. No warm-up, no discarded input.
    assert_eq!(
        rx.accept(gen2.stamp()),
        InputOutcome::Apply,
        "a seeded client must drive a session it did not start, from its first keystroke"
    );
    for _ in 0..10 {
        assert_eq!(rx.accept(gen2.stamp()), InputOutcome::Apply);
    }
    assert_eq!(rx.expected(), 51, "all 51 batches applied, none lost");
}

#[test]
fn seeding_preserves_loss_detection_within_the_new_clients_lifetime() {
    // Seeding must not weaken the contract it is patching: once resumed, the counter is an ordinary
    // dense monotonic stream, so a keystroke severed in flight is still caught as loss rather than
    // being papered over.
    let mut rx = InputReceiver::new();
    let mut gen1 = InputSeq::new();
    for _ in 0..7 {
        assert_eq!(rx.accept(gen1.stamp()), InputOutcome::Apply);
    }

    let mut gen2 = InputSeq::resume_from(rx.expected());
    assert_eq!(
        gen2.peek(),
        7,
        "the resumed counter starts where the daemon is"
    );
    assert_eq!(rx.accept(gen2.stamp()), InputOutcome::Apply); // 7
    let _severed = gen2.stamp(); // 8, lost in flight
    assert_eq!(rx.accept(gen2.stamp()), InputOutcome::Lost { missing: 1 });
}

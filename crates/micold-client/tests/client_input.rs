//! T044 [US2] — the client stamps keystrokes into ordered `SessionInput` messages (protocol.md §7,
//! data-model G2, FR-019). Proves the client end of the drive loop: a key press becomes exactly the
//! wire message the daemon expects, with a per-session monotonic serial that never resets across a
//! (simulated) detach/reattach.

use micold_client::input::SessionInputStamper;
use micold_client::keymap::{encode, Key, KeyInput, KeyOutput, Mods, TermMode};
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::SessionId;

fn input(serial_msg: &ClientMsg) -> (SessionId, u64, &[u8]) {
    match serial_msg {
        ClientMsg::SessionInput {
            session,
            serial,
            bytes,
        } => (*session, *serial, bytes),
        other => panic!("expected SessionInput, got {other:?}"),
    }
}

#[test]
fn serials_are_dense_and_monotonic_per_session() {
    let mut stamper = SessionInputStamper::new();
    let s = SessionId::new();
    for expected in 0..5u64 {
        let msg = stamper.stamp(s, vec![b'x']);
        let (got_session, serial, bytes) = input(&msg);
        assert_eq!(got_session, s);
        assert_eq!(serial, expected, "serials must be dense and monotonic");
        assert_eq!(bytes, b"x");
    }
}

#[test]
fn each_session_has_an_independent_serial_stream() {
    let mut stamper = SessionInputStamper::new();
    let (a, b) = (SessionId::new(), SessionId::new());

    assert_eq!(input(&stamper.stamp(a, vec![1])).1, 0);
    assert_eq!(input(&stamper.stamp(a, vec![2])).1, 1);
    // b's stream starts at 0, unaffected by a's.
    assert_eq!(input(&stamper.stamp(b, vec![3])).1, 0);
    assert_eq!(input(&stamper.stamp(a, vec![4])).1, 2);
    assert_eq!(input(&stamper.stamp(b, vec![5])).1, 1);
}

#[test]
fn a_sessions_serial_is_not_reset_by_a_detach_reattach() {
    // The stamper lives in the client's long-lived state, so a reconnect (which replaces only the
    // transport) leaves the counter untouched — the daemon can prove no keystroke was lost.
    let mut stamper = SessionInputStamper::new();
    let s = SessionId::new();
    assert_eq!(input(&stamper.stamp(s, vec![b'a'])).1, 0);
    assert_eq!(input(&stamper.stamp(s, vec![b'b'])).1, 1);

    // ...detach + reattach happen here; the same stamper keeps counting...
    assert_eq!(
        input(&stamper.stamp(s, vec![b'c'])).1,
        2,
        "the serial must continue across a reconnect, never restart at 0"
    );
}

#[test]
fn forget_clears_a_sessions_counter_for_reuse_hygiene() {
    let mut stamper = SessionInputStamper::new();
    let s = SessionId::new();
    assert_eq!(input(&stamper.stamp(s, vec![b'a'])).1, 0);
    assert_eq!(input(&stamper.stamp(s, vec![b'b'])).1, 1);
    stamper.forget(s);
    // After an explicit end-of-session forget, a fresh counter starts over.
    assert_eq!(input(&stamper.stamp(s, vec![b'c'])).1, 0);
}

#[test]
fn keymap_encode_to_stamp_is_the_client_drive_pipeline() {
    // The real client path: a key press → keymap::encode → VT bytes → stamped SessionInput.
    let mut stamper = SessionInputStamper::new();
    let s = SessionId::new();

    let press = KeyInput {
        key: Key::Char('a'),
        mods: Mods::NONE,
        text: Some("a".to_string()),
    };
    let KeyOutput::Bytes(bytes) = encode(&press, TermMode::default()) else {
        panic!("a plain character must encode to bytes");
    };
    assert_eq!(bytes, b"a", "the printable char encodes to its byte");

    let msg = stamper.stamp(s, bytes);
    let (_, serial, wire_bytes) = input(&msg);
    assert_eq!(serial, 0);
    assert_eq!(
        wire_bytes, b"a",
        "the stamped message carries the encoded bytes verbatim"
    );
}

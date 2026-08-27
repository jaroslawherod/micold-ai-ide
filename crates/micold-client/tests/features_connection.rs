//! The daemon-connection status, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module. It builds no `State`, no `App`, references no other
//! feature's types, and needs no application shell — which is the point of T022 having moved the
//! precedence out of `main.rs`: before, deciding which banner wins required constructing the whole
//! shell struct.
//!
//! `main.rs` keeps its own test of this ordering through `App`, and that test is untouched
//! (FR-027). This one asserts the same rule one layer down, where it is cheap.

use micold_client::features::connection::{connection_status, ConnectionStatus, Hold, HoldCause};

fn taken_over() -> Hold {
    Hold::taken_over("other window")
}

fn already_open() -> Hold {
    Hold::already_open("other window")
}

fn version() -> (u32, u32, String) {
    (3, 4, "daemon-build".into())
}

fn build() -> (String, String) {
    ("client-build".into(), "daemon-build".into())
}

#[test]
fn a_quiet_connection_reports_connected() {
    assert_eq!(
        connection_status(None, None, None, false),
        ConnectionStatus::Connected,
        "nothing wrong, so the banner renders nothing"
    );
}

#[test]
fn a_version_mismatch_outranks_everything_else_true_at_the_same_time() {
    let status = connection_status(Some(&version()), Some(&build()), Some(&taken_over()), true);

    assert!(
        matches!(status, ConnectionStatus::VersionMismatch { .. }),
        "when the daemon speaks a different protocol nothing else will work, so telling the user \
         to reconnect a socket or take back a project would send them after the wrong problem: \
         got {status:?}"
    );
}

#[test]
fn a_build_mismatch_outranks_displacement_and_disconnection() {
    let status = connection_status(None, Some(&build()), Some(&taken_over()), true);

    assert!(
        matches!(status, ConnectionStatus::BuildMismatch { .. }),
        "got {status:?}"
    );
}

#[test]
fn displacement_outranks_a_plain_disconnect() {
    let status = connection_status(None, None, Some(&taken_over()), true);

    assert_eq!(
        status,
        ConnectionStatus::Displaced {
            by: "other window".into()
        },
        "read-only because another window holds the project is a different situation from \
         read-only because the socket dropped, and only one of them is fixed by waiting"
    );
}

#[test]
fn a_dropped_socket_alone_reports_disconnected() {
    assert_eq!(
        connection_status(None, None, None, true),
        ConnectionStatus::Disconnected
    );
}

#[test]
fn no_displacement_is_reported_when_no_window_holds_the_project() {
    // The shell resolves the active project to a displacement and passes None when there is
    // none — the seam where a `HashMap::get` miss used to fall through an `if let` in main.rs.
    assert_eq!(
        connection_status(None, None, None, false),
        ConnectionStatus::Connected,
        "an absent entry means nobody took the project, not that somebody took it anonymously"
    );
}

/// `010` BUG-023: the two ways a project can be read-only are not the same event, and the banner
/// that describes them is chosen from this variant.
///
/// A displacement is something that happened *to* this window — it held the project and lost it. A
/// refusal is something this window asked for and did not get; nobody took anything. They share the
/// read-only state and the take-over action, which is why the refusal was mapped onto the
/// displacement in the first place; sharing the *sentence* is what made the banner state the
/// opposite of what occurred.
#[test]
fn a_refusal_and_a_takeover_are_not_the_same_status() {
    let refused = connection_status(None, None, Some(&already_open()), false);
    let taken = connection_status(None, None, Some(&taken_over()), false);

    assert_eq!(
        refused,
        ConnectionStatus::ProjectBusy {
            holder: "other window".into()
        },
        "a refused attach must reach the banner as a refusal; got {refused:?}"
    );
    assert_ne!(
        refused, taken,
        "one variant for both is how the refusal came to borrow the displacement's sentence — \
         the banner cannot say something different about a difference it cannot see"
    );
}

/// Both are read-only, and both outrank a plain disconnect. The distinction BUG-023 asks for is in
/// what the user is *told*, not in what they may do: the take-over is the right action for either.
#[test]
fn a_refusal_outranks_a_plain_disconnect_exactly_as_a_takeover_does() {
    let status = connection_status(None, None, Some(&already_open()), true);

    assert!(
        matches!(status, ConnectionStatus::ProjectBusy { .. }),
        "waiting fixes a dropped socket and does nothing about a project another window holds, \
         which is as true of a refusal as of a takeover; got {status:?}"
    );
}

/// The cause travels with the holder rather than being re-derived, because by the time the banner
/// is drawn the event is over: nothing in the app state afterwards distinguishes \"we were pushed
/// out\" from \"we were turned away\".
#[test]
fn the_cause_is_carried_not_inferred() {
    assert_eq!(Hold::taken_over("w").cause, HoldCause::TakenOver);
    assert_eq!(Hold::already_open("w").cause, HoldCause::AlreadyOpen);
    assert_eq!(Hold::already_open("w").holder, "w");
}

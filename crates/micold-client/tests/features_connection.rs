//! The daemon-connection status, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module. It builds no `State`, no `App`, references no other
//! feature's types, and needs no application shell — which is the point of T022 having moved the
//! precedence out of `main.rs`: before, deciding which banner wins required constructing the whole
//! shell struct.
//!
//! `main.rs` keeps its own test of this ordering through `App`, and that test is untouched
//! (FR-027). This one asserts the same rule one layer down, where it is cheap.

use micold_client::features::connection::{connection_status, ConnectionStatus};

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
    let status = connection_status(Some(&version()), Some(&build()), Some("other window"), true);

    assert!(
        matches!(status, ConnectionStatus::VersionMismatch { .. }),
        "when the daemon speaks a different protocol nothing else will work, so telling the user \
         to reconnect a socket or take back a project would send them after the wrong problem: \
         got {status:?}"
    );
}

#[test]
fn a_build_mismatch_outranks_displacement_and_disconnection() {
    let status = connection_status(None, Some(&build()), Some("other window"), true);

    assert!(
        matches!(status, ConnectionStatus::BuildMismatch { .. }),
        "got {status:?}"
    );
}

#[test]
fn displacement_outranks_a_plain_disconnect() {
    let status = connection_status(None, None, Some("other window"), true);

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

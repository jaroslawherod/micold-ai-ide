//! The daemon connection, as it concerns the active project (feature 021, T022).
//!
//! This module exists because the feature had nowhere to live. research.md §6's Tier 1 table lists
//! seven migration steps for eight features, and the daemon connection is the one it misses — its
//! type sat in `ui/mod.rs` beside the banner that draws it, and its decision sat in `main.rs`
//! beside the runtime that feeds it. Neither placement is wrong by accident: the connection *is*
//! binary-owned runtime state. But the question "which status wins when three of them are true at
//! once" is a decision, not a runtime, and Principle I wants decisions testable.
//!
//! So [`ConnectionStatus`] moves here whole, and [`connection_status`] takes the four facts it
//! needs rather than the shell's `App`. What is left in `main.rs` is the lookup that turns the
//! active project into a displacement, which is plumbing.

/// The daemon-connection state, as it concerns the *active* project (US5, FR-024/027). Computed by
/// the binary (the connection is binary-owned runtime state) and passed to `ui::view` so the shell
/// can show a persistent status banner. `Connected` renders nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// The service is reachable and this window holds the active project.
    Connected,
    /// The service connection is down; displayed content may be stale and is auto-reconnecting.
    Disconnected,
    /// Another window took over the active project (`by` = its build/identity). This window is
    /// read-only until the user takes it back (FR-024), or until an attach of its own is accepted
    /// (FR-024a) — the service decides who holds a project, so its acceptance ends this state
    /// whether or not the user pressed the takeover button.
    Displaced {
        /// The taking-over window's identity string, **as of the takeover or refusal that raised
        /// this state**. It is a point-in-time label, not a live one: it is never re-derived, so a
        /// window that has since exited can still be named here. That is tolerable because the
        /// label's job is to explain *why* this window is read-only, and the reason is historical
        /// by nature — but it must not be read as a claim that the named window is running now
        /// (BUG-007, T118).
        by: String,
    },
    /// The running service speaks a different contract version (US6, FR-021/022). Names both versions
    /// and offers a one-click restart of the service.
    VersionMismatch {
        /// This client's protocol version.
        client: u32,
        /// The running daemon's protocol version.
        daemon: u32,
        /// The running daemon's build string.
        daemon_build: String,
    },
    /// The running service is a same-contract different build — most releases don't touch the wire
    /// protocol, so this is the common shape a `.deb` upgrade takes (US6, FR-022a, BUG-002). Offers
    /// the same one-click restart as [`ConnectionStatus::VersionMismatch`], but without implying any
    /// risk to live sessions: the contract still matches, so nothing is actually incompatible.
    BuildMismatch {
        /// This client's build string.
        client_build: String,
        /// The running daemon's build string.
        daemon_build: String,
    },
}

/// Which status the banner shows when several are true at once (BUG-002, convergence finding F1).
///
/// The precedence is the whole content of this function, and it is ordered by how much the user
/// can do about it: a version mismatch means nothing will work until the service is restarted, so
/// it outranks a build mismatch, which outranks displacement, which outranks a plain disconnect.
/// Reporting the mildest true state first would leave the user reconnecting a socket when the
/// actual problem is that the daemon speaks a different protocol.
///
/// Takes the four facts rather than the shell's `App` so the ordering is testable without a
/// running window (Principle I). Resolving the active project to a displacement stays in the
/// shell — that is a lookup, not a decision.
pub fn connection_status(
    version_mismatch: Option<&(u32, u32, String)>,
    build_mismatch: Option<&(String, String)>,
    displaced_by: Option<&str>,
    disconnected: bool,
) -> ConnectionStatus {
    if let Some((client, daemon, daemon_build)) = version_mismatch {
        return ConnectionStatus::VersionMismatch {
            client: *client,
            daemon: *daemon,
            daemon_build: daemon_build.clone(),
        };
    }
    if let Some((client_build, daemon_build)) = build_mismatch {
        return ConnectionStatus::BuildMismatch {
            client_build: client_build.clone(),
            daemon_build: daemon_build.clone(),
        };
    }
    if let Some(by) = displaced_by {
        return ConnectionStatus::Displaced { by: by.to_string() };
    }
    if disconnected {
        ConnectionStatus::Disconnected
    } else {
        ConnectionStatus::Connected
    }
}

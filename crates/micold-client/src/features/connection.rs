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

/// Everything the daemon connection reports or is asked to do (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// Seven began with `Daemon` and two with `Connection`; neither prefix survives, because the type
/// now says which connection (contract M1). The result reads the way [`ConnectionStatus`] beside it
/// already did — `Connected`, `Disconnected`, `VersionMismatch` — which is the same vocabulary
/// about the same thing, and the duplication of names between the two is the point: a status is
/// what a message leaves behind. `DiagnosticsRequested` and the two `LogoutSurvival` variants
/// carried no prefix to drop.
///
/// # Every arm of this one is an effect
///
/// This is the feature data-model §1.1 calls shape B: the reducer entry is
/// `shell/connection.rs`'s `update`, and there is no pure half. The connection is binary-owned
/// runtime — an outbox handle, a socket that dropped, a service to restart — so `State::update`
/// has never done anything with these but decline them, and it still declines them, now in one arm
/// instead of twelve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The daemon connection is up: the binary stores the `Outbox` to drive sessions and adopts
    /// the welcome catalog/settings.
    Connected {
        /// Handle for sending `ClientMsg`s to the daemon.
        outbox: crate::daemon::Outbox,
        /// The catalog as of the handshake.
        catalog: micold_core::protocol::messages::CatalogSnapshot,
        /// The service-owned settings.
        settings: micold_core::protocol::messages::DaemonSettings,
    },
    /// A control message pushed by the daemon (catalog/settings changes, operation results, …).
    Event(micold_core::protocol::messages::DaemonMsg),
    /// A grid frame for the viewed session (full snapshot or delta), applied into the per-session
    /// grid cache.
    GridFrame(micold_core::protocol::grid::GridFrame),
    /// The daemon connection dropped; the binary clears its outbox until it reconnects.
    Disconnected,
    /// Connecting to (or spawning) the daemon failed, with a human-facing reason.
    ConnectFailed(String),
    /// The user asked to take the active project back after being displaced (US5, FR-024):
    /// re-attach with `force`.
    TakeoverRequested,
    /// The daemon refused the handshake on a contract mismatch (US6, FR-021): carries both protocol
    /// versions and the daemon build so the client can render an actionable diagnostic.
    VersionMismatch {
        /// This client's protocol version.
        client: u32,
        /// The running daemon's protocol version.
        daemon: u32,
        /// The running daemon's human-facing build string.
        daemon_build: String,
    },
    /// The daemon refused the handshake on a same-contract package-version difference (US6,
    /// FR-022a, BUG-002): the wire contract matches, but a `.deb` upgrade installed a newer build
    /// than the one still running. Carries both build strings so the client can render a distinct,
    /// lower-severity diagnostic than [`Msg::VersionMismatch`].
    BuildMismatch {
        /// This client's human-facing build string.
        client_build: String,
        /// The running daemon's human-facing build string.
        daemon_build: String,
    },
    /// The user chose "restart service" after a version or build mismatch (US6, FR-022/022a): stop
    /// the mismatched daemon so the auto-reconnect spawns a matching one.
    RestartServiceRequested,
    /// The user asked to see where the session service logs and its recent errors (Phase 10,
    /// FR-046): the binary requests both from the daemon and shows the answers as notices.
    DiagnosticsRequested,
    /// The user asked to make sessions survive logout (US7, FR-038; Linux only). The binary runs
    /// the enable flow off-thread. Never triggered by install — a deliberate choice.
    LogoutSurvivalRequested,
    /// The logout-survival enable flow finished; carries a ready-to-show message (info or error).
    LogoutSurvivalOutcome(String),
}

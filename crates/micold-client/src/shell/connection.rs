//! The daemon connection's reducer entry: shape B, with no pure half (feature 028, contract M2).
//!
//! `features/connection.rs` holds the vocabulary and the one decision the connection *is* asked to
//! make — which status wins when several are true at once. Everything else about a connection is
//! runtime: an outbox handle that only the binary can hold, a socket that dropped, a service to
//! stop and let respawn. So all twelve arms return a `Task`, and by M2's rule — an arm belongs in
//! the shell when it must return a `Task`, and in the feature otherwise — all twelve are here.
//!
//! That makes this the case data-model §1.1 lists third and `settings` does not exercise: a
//! feature whose entry point is *only* effectful. `State::update` still declines the whole
//! vocabulary, as it declined each of the twelve variants before, and says so in one arm.
//!
//! The bodies did not move. Ten of them are `daemon_sync`'s and `service_control`'s, called from
//! here rather than from a per-variant arm in `main.rs`. The two mismatch arms had no function to
//! call — they wrote `app.version_mismatch` and `app.build_mismatch` inline in `main.rs` — and
//! they write the same fields here, next to the rest of the connection's routing rather than
//! interleaved with the overlay and session arms that surrounded them.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::connection::Msg;

use crate::shell::{daemon_sync, service_control};
use crate::App;

/// This feature's entry point: one arm in `main.rs` routes here (contract M2).
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::Connected {
            outbox,
            catalog,
            settings,
        } => daemon_sync::on_connected(app, outbox, catalog, settings),
        Msg::Event(event) => daemon_sync::on_daemon_event(app, event),
        Msg::GridFrame(frame) => daemon_sync::on_grid_frame(app, frame),
        Msg::Disconnected => daemon_sync::on_disconnected(app),
        Msg::ConnectFailed(reason) => daemon_sync::on_connect_failed(app, reason),
        Msg::TakeoverRequested => daemon_sync::on_takeover_requested(app),
        // The daemon refused us on a contract mismatch (US6, FR-021): record it so the banner can
        // name both versions and offer the restart action. The connection subscription keeps
        // retrying in the background; each retry re-sets this identically until the user acts.
        Msg::VersionMismatch {
            client,
            daemon,
            daemon_build,
        } => {
            app.version_mismatch = Some((client, daemon, daemon_build));
            Task::none()
        }
        // Same contract, different package version (US6, FR-022a, BUG-002): record it so the
        // banner can name both builds and offer the restart action, distinct from a contract
        // mismatch.
        Msg::BuildMismatch {
            client_build,
            daemon_build,
        } => {
            app.build_mismatch = Some((client_build, daemon_build));
            Task::none()
        }
        Msg::RestartServiceRequested => service_control::on_restart_service_requested(app),
        Msg::LogoutSurvivalOutcome(message) => {
            service_control::on_logout_survival_outcome(app, message)
        }
        Msg::DiagnosticsRequested => daemon_sync::on_diagnostics_requested(app),
    }
}

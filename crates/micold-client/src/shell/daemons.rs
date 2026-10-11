//! The Daemons feature's entry point in the shell (feature 491, M3).
//!
//! The reducer ([`micold_client::features::daemons`]) changes the registry, the links and the
//! bindings in place and reports what has to leave the process as outcomes. This performs them:
//! it writes the registry and a binding through [`App::apply_daemon_outcome`], stops the sessions
//! of an edited or removed daemon on that daemon's own outbox, and registers a newly bound
//! project. Starting and stopping need nothing more: the connection subscription reads the links
//! ([`micold_client::links::DaemonLinks::should_dial`]), so a started daemon is dialled and a
//! stopped one is neither dialled nor reconnected.

use std::collections::BTreeMap;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::daemons::Msg;
use micold_client::features::Outcome;
use micold_core::daemons::DaemonId;
use micold_core::protocol::messages::ClientMsg;

use crate::App;

/// Apply a Daemons message and carry out what it asks for.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    // Taken before the reducer: stopping or removing a daemon drops its outbox, and its sessions
    // are stopped on it.
    let outboxes: BTreeMap<DaemonId, _> = app
        .core
        .settings
        .daemons
        .entries()
        .iter()
        .filter_map(|e| app.outbox_for(e.id).map(|o| (e.id, o)))
        .collect();
    let outcomes = app.core.update_daemons(msg);
    let mut rest = Vec::new();
    let mut bound = Vec::new();
    for outcome in outcomes {
        match &outcome {
            Outcome::DaemonRegistryChanged(_) => app.apply_daemon_outcome(&outcome),
            Outcome::DaemonBound { daemon, .. } => {
                app.apply_daemon_outcome(&outcome);
                bound.push(*daemon);
            }
            Outcome::DaemonSessionsEnded { daemon, sessions } => {
                if let Some(outbox) = outboxes.get(daemon) {
                    for session in sessions {
                        outbox.send(ClientMsg::SessionStop { session: *session });
                    }
                }
                rest.push(outcome);
            }
            _ => rest.push(outcome),
        }
    }
    // A daemon added since launch is tracked, a removed one forgotten with its catalog.
    app.sync_links();
    // A daemon stopped (or removed, or restarted by an edit) no longer has a link: its catalog,
    // pending requests and "connected" standing go the way a lost connection's do, so its
    // sessions are not shown as live (`Stopped` ignores the `Lost` this folds).
    let gone: Vec<DaemonId> = app
        .daemon_catalogs
        .keys()
        .copied()
        .filter(|id| {
            !matches!(
                app.core.settings.links.state(*id),
                Some(micold_core::daemons::DaemonState::Connected)
                    | Some(micold_core::daemons::DaemonState::Starting)
            )
        })
        .collect();
    for id in gone {
        let _ = crate::shell::daemon_sync::on_disconnected(app, id);
    }
    // A project first bound to a connected daemon is registered there before a session starts.
    for daemon in bound {
        crate::shell::daemon_sync::register_projects(app, daemon);
    }
    micold_client::app::drain(rest, |outcome| {
        micold_client::app::interpret(&mut app.core, outcome)
    });
    Task::none()
}

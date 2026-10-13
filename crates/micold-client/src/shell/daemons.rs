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
use std::path::{Path, PathBuf};

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

/// The `ssh` executable on Windows: the one on `PATH`, then the OpenSSH that ships with Windows
/// (`%SystemRoot%\\System32\\OpenSSH\\ssh.exe`). `None` leaves the lookup to the runner, which
/// reports a missing `ssh` with the hint to install it. `is_file` is injected so the order is
/// testable on any platform.
pub fn locate_windows_ssh(
    path_var: &str,
    system_root: Option<&Path>,
    is_file: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let on_path = std::env::split_paths(path_var)
        .map(|dir| dir.join("ssh.exe"))
        .find(|p| is_file(p));
    on_path.or_else(|| {
        system_root
            .map(|root| root.join("System32").join("OpenSSH").join("ssh.exe"))
            .filter(|p| is_file(p))
    })
}

/// The one real `ssh` runner of this process (feature 489, FR-017): this build's own daemon is the
/// artefact copied to a host of the same platform, and the executable is looked up by name, except
/// on Windows where OpenSSH is not always on `PATH`.
pub fn system_ssh() -> micold_core::ssh::SystemSsh {
    let ssh_path = if cfg!(windows) {
        locate_windows_ssh(
            &std::env::var("PATH").unwrap_or_default(),
            std::env::var_os("SystemRoot").map(PathBuf::from).as_deref(),
            &|p| p.is_file(),
        )
    } else {
        None
    };
    micold_core::ssh::SystemSsh {
        own_daemon: PathBuf::from(micold_core::spawn::daemon_binary()),
        // Builds for other platforms ship beside the client when packaged (M8).
        bundled_dir: std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|d| d.join("remote"))),
        ssh_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_ssh_is_taken_from_path_before_the_system_openssh() {
        let tools = PathBuf::from("tools");
        let root = PathBuf::from("win");
        let path_var = std::env::join_paths([&tools])
            .unwrap()
            .into_string()
            .unwrap();
        let on_path = tools.join("ssh.exe");
        let system = root.join("System32").join("OpenSSH").join("ssh.exe");

        let both = |p: &Path| p == on_path || p == system;
        assert_eq!(
            locate_windows_ssh(&path_var, Some(&root), &both),
            Some(on_path.clone())
        );
        let system_only = |p: &Path| p == system;
        assert_eq!(
            locate_windows_ssh(&path_var, Some(&root), &system_only),
            Some(system.clone())
        );
        assert_eq!(locate_windows_ssh(&path_var, None, &system_only), None);
    }
}

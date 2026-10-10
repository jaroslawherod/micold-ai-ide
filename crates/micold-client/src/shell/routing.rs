//! Where an op goes (feature 491, M2, contract `client-daemon-routing.md`).
//!
//! The binary holds one [`DaemonLinks`] and asks the pure [`links::route`] where each op belongs.
//! Everything here is a lookup on `App`: the binding of a worktree, of the active location, of a
//! session, and the outbox of the daemon an op is bound to. None of them falls back to another
//! daemon: an op bound to a daemon that is down gets no outbox, and the caller says why.

use std::path::Path;

use micold_client::daemon::Outbox;
use micold_client::links::{self, Route};
use micold_core::daemons::{Binding, DaemonId};
use micold_core::session::{Session, SessionLocation};

use crate::App;

/// The binding key of a session location: a worktree's `dir_name`, `""` for the Default location.
fn location_key(location: &SessionLocation) -> &str {
    match location {
        SessionLocation::Worktree(dir) => dir.as_str(),
        SessionLocation::Default => "",
    }
}

impl App {
    /// The daemon `key` of `project` is bound to, else the legacy default.
    pub(crate) fn binding_of(&self, project: &Path, key: &str) -> Binding {
        let stored = self
            .core
            .workspace
            .bindings
            .get(project)
            .cloned()
            .unwrap_or_default();
        Binding::resolve(
            &stored,
            key,
            &self.core.settings.daemons,
            self.core.settings.legacy_default_daemon,
        )
    }

    /// The binding key of the location in view: the displayed session's worktree, else `""`.
    fn active_key(&self) -> String {
        self.core
            .session
            .active
            .and_then(|id| self.core.active_sessions().iter().find(|s| s.id == id))
            .map(|s| location_key(&s.location).to_string())
            .unwrap_or_default()
    }

    /// The binding of the active project's active location. `NoDaemon` with no active project.
    pub(crate) fn active_binding(&self) -> Binding {
        // With no project open there is no stored binding to read, and a location resolves to the
        // legacy default, as every unbound one does.
        match &self.core.workspace.active {
            Some(project) => self.binding_of(project, &self.active_key()),
            None => self.binding_of(Path::new(""), ""),
        }
    }

    /// The daemon the active project's active location is bound to, when it names one.
    pub(crate) fn active_daemon(&self) -> Option<DaemonId> {
        match self.active_binding() {
            Binding::Bound(id) => Some(id),
            Binding::NoDaemon => None,
        }
    }

    /// The binding of a session of `project`, from where it is, via [`session_daemon`].
    ///
    /// [`session_daemon`]: micold_client::catalog_sync::session_daemon
    pub(crate) fn session_binding(&self, project: &Path, session: &Session) -> Binding {
        let id = micold_client::catalog_sync::session_daemon(&self.core, project, session)
            .or(self.core.settings.legacy_default_daemon);
        match id {
            Some(id) if self.core.settings.daemons.get(id).is_some() => Binding::Bound(id),
            _ => Binding::NoDaemon,
        }
    }

    /// Where an op bound by `binding` goes.
    pub(crate) fn route(&self, binding: Binding) -> Route {
        links::route(&self.links, &self.core.settings.daemons, binding)
    }

    /// The outbox of the daemon `binding` names, only while it is connected.
    pub(crate) fn outbox_of(&self, binding: Binding) -> Option<Outbox> {
        match self.route(binding) {
            Route::To(id) => self.links.outbox(id).cloned(),
            _ => None,
        }
    }

    /// The outbox of the daemon the active project's active location is bound to. `None` while
    /// that daemon is down, never another daemon's.
    pub(crate) fn active_outbox(&self) -> Option<Outbox> {
        self.outbox_of(self.active_binding())
    }

    /// The outbox of `id`, while connected.
    pub(crate) fn outbox_for(&self, id: DaemonId) -> Option<Outbox> {
        self.links.outbox(id).cloned()
    }

    /// The binding of a session by its id, in whichever project holds it. `NoDaemon` for a
    /// session no project knows, never the active location's daemon.
    pub(crate) fn session_binding_of(&self, session: micold_core::session::SessionId) -> Binding {
        self.core
            .workspace
            .sessions
            .iter()
            .find_map(|(project, sessions)| {
                sessions
                    .iter()
                    .find(|s| s.id == session)
                    .map(|s| self.session_binding(project, s))
            })
            .unwrap_or(Binding::NoDaemon)
    }

    /// The daemon a session runs on, by its id.
    pub(crate) fn daemon_of_session(
        &self,
        session: micold_core::session::SessionId,
    ) -> Option<DaemonId> {
        match self.session_binding_of(session) {
            Binding::Bound(id) => Some(id),
            Binding::NoDaemon => None,
        }
    }

    /// The distinct bindings of `project`'s locations: its Default and every stored one.
    pub(crate) fn project_bindings(&self, project: &Path) -> Vec<Binding> {
        let mut all = vec![self.binding_of(project, "")];
        if let Some(keys) = self.core.workspace.bindings.get(project) {
            for key in keys.keys() {
                let binding = self.binding_of(project, key);
                if !all.contains(&binding) {
                    all.push(binding);
                }
            }
        }
        all
    }

    /// Bring `links` in line with the registry: a daemon added since launch is tracked the way
    /// launch tracks one, and a removed one is forgotten with its outbox and catalog.
    pub(crate) fn sync_links(&mut self) {
        let registry = &self.core.settings.daemons;
        let launch = micold_core::daemons::DaemonStates::at_launch(registry);
        for entry in registry.entries() {
            if self.links.state(entry.id).is_none() {
                if let Some(state) = launch.state(entry.id) {
                    self.links.insert(entry.id, state.clone());
                }
            }
        }
        let stale: Vec<DaemonId> = self
            .links
            .states()
            .ids()
            .into_iter()
            .filter(|id| registry.get(*id).is_none())
            .collect();
        for id in stale {
            self.links.remove(id);
            self.daemon_catalogs.remove(&id);
        }
    }

    /// Whether any location of `project` is bound to `daemon`: the project is one it serves.
    pub(crate) fn project_uses(&self, project: &Path, daemon: DaemonId) -> bool {
        self.binding_of(project, "") == Binding::Bound(daemon)
            || self
                .core
                .workspace
                .bindings
                .get(project)
                .is_some_and(|keys| keys.values().any(|d| *d == daemon))
    }

    /// Every `(project, binding)` of every location the window knows: each project's Default
    /// location and each location with a stored binding.
    pub(crate) fn all_bindings(&self) -> Vec<(std::path::PathBuf, Binding)> {
        let mut all = Vec::new();
        for project in &self.core.workspace.projects {
            all.push((project.path.clone(), self.binding_of(&project.path, "")));
            if let Some(keys) = self.core.workspace.bindings.get(&project.path) {
                for key in keys.keys() {
                    all.push((project.path.clone(), self.binding_of(&project.path, key)));
                }
            }
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_key_is_the_dir_or_empty() {
        assert_eq!(location_key(&SessionLocation::Default), "");
        assert_eq!(
            location_key(&SessionLocation::Worktree("w".to_string())),
            "w"
        );
    }
}

#[cfg(test)]
impl App {
    /// Test seam: make daemon 1 the registry's one host daemon and the legacy default, when the
    /// test app has none (a bare `State` has an empty registry).
    pub(crate) fn ensure_test_registry(&mut self) -> DaemonId {
        let id = DaemonId(1);
        if self.core.settings.daemons.get(id).is_none() {
            let entry = micold_core::daemons::migrated_entry(
                &micold_core::settings::DaemonConfig::default(),
            );
            self.core.settings.daemons = micold_core::daemons::DaemonRegistry::new(vec![entry], 2);
        }
        self.core.settings.legacy_default_daemon.get_or_insert(id);
        if self.links.state(id).is_none() {
            self.links
                .insert(id, micold_core::daemons::DaemonState::Starting);
        }
        id
    }

    /// Test seam: the legacy default daemon is connected over `outbox`.
    pub(crate) fn connect_test_daemon(&mut self, outbox: Outbox) {
        let id = self.ensure_test_registry();
        self.links.connected(id, outbox);
    }

    /// Test seam: the legacy default daemon's connection is gone.
    pub(crate) fn disconnect_test_daemon(&mut self) {
        let id = self.ensure_test_registry();
        let _ = self.links.lost(id, "test");
    }
}

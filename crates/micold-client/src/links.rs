//! What the client knows of each daemon's connection, and the decisions made from it (feature 491,
//! M2: US1 routing, US2 per-daemon state).
//!
//! Everything here is pure: [`DaemonLinks`] holds one [`DaemonStates`] and one [`Outbox`] per
//! daemon, and the functions beside it answer questions about them: where an op goes ([`route`]),
//! which projects a daemon still has to be told about ([`registrations`]), what a worktree row says
//! when its daemon is not connected ([`unavailable_label`]), and whether launch should reconnect,
//! spawn or leave a daemon alone ([`bring_up`]). Nothing here opens a socket; the actors in
//! [`crate::daemon`] do, and the shell feeds what they report in through [`DaemonLinks::apply`].
//!
//! The one invariant every function protects is the contract's rule P-2: nothing falls back to a
//! different daemon. An op for a daemon that is not connected is refused with that daemon's name,
//! never sent somewhere that happens to be up.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use micold_core::daemons::{
    Binding, DaemonEntry, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime, DaemonState,
    DaemonStates, StateChange,
};
use micold_core::protocol::messages::RefusalReason;
use micold_core::session::SessionLifecycle;

use crate::daemon::Outbox;

/// The text of the refusal for an op bound to no daemon (contract strings table).
pub const NO_DAEMON_MESSAGE: &str = "no daemon";
/// Shown when a container daemon cannot start because no container runtime is installed.
pub const RUNTIME_NOT_FOUND: &str = "container runtime not found";
/// Shown for a container daemon whose paths are mapped (a Windows host, R7).
pub const PATHS_MAPPED: &str = "paths are mapped";

/// The connection of every daemon: its state fold and, while connected, the handle that reaches it.
///
/// Each daemon's entry is independent: [`apply`](Self::apply), [`connected`](Self::connected) and
/// [`lost`](Self::lost) take an id and touch only that id's state and outbox, so one transport
/// failing changes no other daemon's status or outbox (FR-007).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonLinks {
    states: DaemonStates,
    outboxes: BTreeMap<DaemonId, Outbox>,
}

impl DaemonLinks {
    /// Links at launch: each daemon `Starting` or `Stopped` as [`DaemonStates::at_launch`] says.
    pub fn at_launch(registry: &DaemonRegistry) -> Self {
        Self {
            states: DaemonStates::at_launch(registry),
            outboxes: BTreeMap::new(),
        }
    }

    /// Track `id` in `state`, with no outbox.
    pub fn insert(&mut self, id: DaemonId, state: DaemonState) {
        self.states.insert(id, state);
    }

    /// The handshake with `id` succeeded and `outbox` reaches it. Returns whether the link
    /// accepted it: a stopped (or untracked) daemon ignores `Connected`, and then the outbox is not
    /// stored, so a refused connection never looks usable.
    pub fn connected(&mut self, id: DaemonId, outbox: Outbox) -> bool {
        self.states.apply(id, DaemonEvent::Connected);
        let accepted = self.states.state(id) == Some(&DaemonState::Connected);
        if accepted {
            self.outboxes.insert(id, outbox);
        }
        accepted
    }

    /// The established transport to `id` ended.
    pub fn lost(&mut self, id: DaemonId, reason: &str) -> Option<StateChange> {
        self.apply(id, DaemonEvent::Lost(reason.to_string()))
    }

    /// Fold `event` into `id`'s state. The outbox is kept only while the daemon is connected, so a
    /// refusal, a stop or a loss can never leave a handle that still looks usable.
    pub fn apply(&mut self, id: DaemonId, event: DaemonEvent) -> Option<StateChange> {
        let change = self.states.apply(id, event);
        if self.states.state(id) != Some(&DaemonState::Connected) {
            self.outboxes.remove(&id);
        }
        change
    }

    /// The outbox that reaches `id`, only while it is connected.
    pub fn outbox(&self, id: DaemonId) -> Option<&Outbox> {
        match self.states.state(id) {
            Some(DaemonState::Connected) => self.outboxes.get(&id),
            _ => None,
        }
    }

    /// Every daemon's state, as the connection subscription reads it.
    pub fn states(&self) -> &DaemonStates {
        &self.states
    }

    /// `id`'s state, when tracked.
    pub fn state(&self, id: DaemonId) -> Option<&DaemonState> {
        self.states.state(id)
    }

    /// Whether `id` is connected.
    pub fn is_connected(&self, id: DaemonId) -> bool {
        self.state(id) == Some(&DaemonState::Connected)
    }

    /// Whether the client should be dialling `id` (tracked and not stopped).
    pub fn should_dial(&self, id: DaemonId) -> bool {
        self.states.should_dial(id)
    }

    /// Forget `id` and its outbox (the daemon was removed).
    pub fn remove(&mut self, id: DaemonId) {
        self.states.remove(id);
        self.outboxes.remove(&id);
    }
}

/// Where an op goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Send it on this daemon's outbox.
    To(DaemonId),
    /// The bound daemon is not connected: refuse with `message`, and send nowhere.
    Unavailable {
        /// "daemon <name> unavailable".
        message: String,
    },
    /// The worktree has no daemon: refuse with `message`, and send nowhere.
    NoDaemon {
        /// "no daemon".
        message: String,
    },
}

/// Decide where an op bound by `binding` goes. Never falls back to another daemon (rule P-2).
pub fn route(links: &DaemonLinks, registry: &DaemonRegistry, binding: Binding) -> Route {
    let Binding::Bound(id) = binding else {
        return no_daemon();
    };
    let Some(entry) = registry.get(id) else {
        return no_daemon();
    };
    if links.outbox(id).is_some() {
        Route::To(id)
    } else {
        Route::Unavailable {
            message: format!("daemon {} unavailable", entry.name.as_str()),
        }
    }
}

fn no_daemon() -> Route {
    Route::NoDaemon {
        message: NO_DAEMON_MESSAGE.to_string(),
    }
}

/// The `ProjectAdd`s to send now: `(daemon, project)` for each project that has a worktree bound
/// to a **connected** daemon and is absent from that daemon's catalog snapshot.
///
/// `bindings` is one `(project path, binding)` per worktree. A bind to a daemon that is not
/// connected yields nothing now and is kept by the caller; calling this again once the daemon
/// connects (its fresh catalog in `catalog_projects`) yields it, which is the whole of "register on
/// connect". Sessions never start on a daemon before this has been sent.
pub fn registrations(
    links: &DaemonLinks,
    registry: &DaemonRegistry,
    bindings: &[(PathBuf, Binding)],
    catalog_projects: &BTreeMap<DaemonId, BTreeSet<PathBuf>>,
) -> Vec<(DaemonId, PathBuf)> {
    let mut wanted: BTreeSet<(DaemonId, PathBuf)> = BTreeSet::new();
    for (project, binding) in bindings {
        let Binding::Bound(id) = binding else {
            continue;
        };
        if registry.get(*id).is_none() || !links.is_connected(*id) {
            continue;
        }
        let known = catalog_projects
            .get(id)
            .is_some_and(|projects| projects.contains(project));
        if !known {
            wanted.insert((*id, project.clone()));
        }
    }
    wanted.into_iter().collect()
}

/// The container a terminal for a worktree bound to `id` execs into: the bound daemon's own.
pub fn terminal_container(registry: &DaemonRegistry, id: DaemonId) -> Option<&str> {
    registry.get(id).and_then(DaemonEntry::container_name)
}

/// The event a daemon's refusal is, when it is one about versions: the daemon and the client
/// disagree, whichever is newer. Other refusals (a busy project, a bad token) are not.
pub fn refusal_event(reason: &RefusalReason) -> Option<DaemonEvent> {
    match reason {
        RefusalReason::VersionMismatch { client, daemon, .. } => {
            Some(refused(client.to_string(), daemon.to_string()))
        }
        RefusalReason::BuildMismatch {
            client_build,
            daemon_build,
        } => Some(refused(client_build.clone(), daemon_build.clone())),
        _ => None,
    }
}

/// The version-mismatch event for the two version strings.
pub fn refused(client: String, daemon: String) -> DaemonEvent {
    DaemonEvent::Refused { client, daemon }
}

/// What the user sees for a daemon in `VersionMismatch`: both versions and the way out, which
/// depends on which side is newer (restart the daemon when the client is newer; update the client
/// when the daemon is).
pub fn mismatch_message(name: &str, client: &str, daemon: &str) -> String {
    let way_out = match version_order(client, daemon) {
        Some(std::cmp::Ordering::Greater) => {
            format!("restart daemon {name} so it runs the client's version")
        }
        Some(std::cmp::Ordering::Less) => "update this client to the daemon's version".to_string(),
        _ => format!("restart daemon {name} or update this client so the versions match"),
    };
    format!("daemon {name} version mismatch: client {client}, daemon {daemon} ({way_out})")
}

/// Order two version strings by their dotted numeric parts, when both have them.
fn version_order(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parts = |v: &str| -> Option<Vec<u64>> {
        v.trim_start_matches(|c: char| !c.is_ascii_digit())
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .ok()
            })
            .collect()
    };
    Some(parts(a)?.cmp(&parts(b)?))
}

/// Facts about a worktree row's daemon that its state alone does not carry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LabelFacts {
    /// The container runtime is not installed.
    pub runtime_missing: bool,
    /// The daemon is a container whose paths are mapped from the host (Windows, R7).
    pub paths_mapped: bool,
    /// The row has sessions that were live when the daemon dropped.
    pub lost_sessions: bool,
}

/// What a worktree row says in place of its daemon chip, or `None` when the daemon is connected
/// and there is nothing to add. Computed from `links` on every call, never cached, so a state
/// change reaches the row within the reducer step that applied it (SC-002).
///
/// A daemon that is not connected outranks the informational "paths are mapped", which shows only
/// for a connected one.
pub fn unavailable_label(
    registry: &DaemonRegistry,
    links: &DaemonLinks,
    binding: Binding,
    facts: LabelFacts,
) -> Option<String> {
    let Binding::Bound(id) = binding else {
        return Some(NO_DAEMON_MESSAGE.to_string());
    };
    let Some(entry) = registry.get(id) else {
        return Some(NO_DAEMON_MESSAGE.to_string());
    };
    let name = entry.name.as_str();
    match links.state(id) {
        Some(DaemonState::Connected) => facts.paths_mapped.then(|| PATHS_MAPPED.to_string()),
        Some(DaemonState::VersionMismatch { client, daemon }) => {
            Some(mismatch_message(name, client, daemon))
        }
        _ if facts.runtime_missing && matches!(entry.runtime, DaemonRuntime::Container(_)) => {
            Some(RUNTIME_NOT_FOUND.to_string())
        }
        Some(DaemonState::Unreachable { .. }) if facts.lost_sessions => {
            Some(format!("lost to daemon {name}"))
        }
        _ => Some(format!("daemon {name} unavailable")),
    }
}

/// What a session row says when its daemon dropped: it is "lost to its daemon", and its lifecycle
/// is left as the daemon last reported it, never `finished`.
pub fn session_lost_label(
    registry: &DaemonRegistry,
    links: &DaemonLinks,
    daemon: DaemonId,
    lifecycle: &SessionLifecycle,
) -> Option<String> {
    let live = matches!(
        lifecycle,
        SessionLifecycle::Running
            | SessionLifecycle::Starting
            | SessionLifecycle::Restarting { .. }
    );
    let dropped = matches!(
        links.state(daemon),
        Some(DaemonState::Unreachable { .. } | DaemonState::Stopped)
    );
    let entry = registry.get(daemon)?;
    (live && dropped).then(|| format!("lost to daemon {}", entry.name.as_str()))
}

/// What launch does with one daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BringUp {
    /// It is already running: reconnect to it. Never respawn or recreate it.
    Adopt,
    /// It is not running and is to start: bring it up.
    Spawn,
    /// Leave it alone: stopped, not `auto_start`, or a runtime this build cannot start.
    Skip,
}

/// Decide launch's action for `entry` in `state`, given whether its process or container is
/// `already_running` (so a daemon that outlived the previous client is adopted, not recreated).
pub fn bring_up(entry: &DaemonEntry, state: &DaemonState, already_running: bool) -> BringUp {
    if matches!(entry.runtime, DaemonRuntime::Unsupported { .. }) || *state == DaemonState::Stopped
    {
        return BringUp::Skip;
    }
    if already_running {
        BringUp::Adopt
    } else if entry.auto_start {
        BringUp::Spawn
    } else {
        BringUp::Skip
    }
}

//! The Daemons list, its add/edit form and the removal flow (feature 491, M3: US3, FR-010 to
//! FR-015).
//!
//! The reducer works on [`State`], what the feature remembers (a draft, a pending confirmation,
//! per-daemon details), and on a [`Model`], the facts and the connection it acts on: the registry,
//! the links and the bindings. Registry and binding changes reach the rest of the app only as
//! [`Outcome`]s (`DaemonRegistryChanged`, `DaemonSessionsEnded`, `DaemonBound`), which the shell
//! persists and acts on, so this module writes no other feature's data.
//!
//! Runtime is data: a row has a plain label whatever the daemon runs in (FR-015).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use micold_core::daemons::{
    Binding, ConnectionError, DaemonDiagnostics, DaemonEvent, DaemonId, DaemonName, DaemonRegistry,
    DaemonRuntime, DaemonState, DaemonView, RegistryError, StateChange, WorktreeRef,
};
use micold_core::sandbox::SandboxProfile;
use micold_core::session::SessionId;

use super::Outcome;
use crate::links::{route, DaemonLinks, Route};

/// What the reducer acts on: the registry, the connection of each daemon, the bindings and what
/// runs where. Borrowed from the app's settings and workspace for the length of one step.
pub struct Model<'a> {
    /// The configured daemons.
    pub registry: &'a mut DaemonRegistry,
    /// The connection of each.
    pub links: &'a mut DaemonLinks,
    /// Explicit bindings: project path to worktree `dir_name` (`""` Default) to daemon.
    pub bindings: &'a mut BTreeMap<PathBuf, BTreeMap<String, DaemonId>>,
    /// The daemon an unbound location resolves to, written once by the migration.
    pub legacy_default: Option<DaemonId>,
    /// Every location the window knows, bound explicitly or not.
    pub locations: &'a [WorktreeRef],
    /// The live sessions of each daemon.
    pub sessions: &'a BTreeMap<DaemonId, Vec<SessionId>>,
}

/// Which runtime the form is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DraftRuntime {
    /// A detached process on this machine.
    Host,
    /// A container on this machine.
    #[default]
    Container,
}

/// The add/edit form's fields, as typed (the port is text so a bad one can be reported).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The daemon being edited; `None` for an add.
    pub editing: Option<DaemonId>,
    /// Display name.
    pub name: String,
    /// Runtime kind.
    pub runtime: DraftRuntime,
    /// Container name (container runtime only).
    pub container_name: String,
    /// Control-channel port (container runtime only).
    pub port: String,
    /// Image, budget and the rest of a container's profile.
    pub profile: SandboxProfile,
    /// Connect at launch.
    pub auto_start: bool,
}

/// A refused save: the field at fault and what to tell the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormError {
    /// `name`, `runtime`, `container_name`, `port` or `image`.
    pub field: &'static str,
    /// The sentence shown beside the field.
    pub message: String,
}

/// The confirmation shown before a removal, or before a running daemon's runtime edit is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalPrompt {
    /// The daemon concerned.
    pub daemon: DaemonId,
    /// Its name.
    pub name: String,
    /// Worktrees bound to it.
    pub worktrees_affected: usize,
    /// Sessions running on it.
    pub running_sessions: usize,
    /// The whole sentence, stating both counts.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Remove(DaemonId),
    Edit {
        id: DaemonId,
        name: String,
        runtime: DaemonRuntime,
        auto_start: bool,
    },
}

/// What one daemon's details view shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonDetails {
    /// Recent connection errors, oldest first.
    pub errors: Vec<ConnectionError>,
    /// The log tail, oldest first.
    pub log: Vec<String>,
    /// Why the container runtime is unusable, when it is.
    pub runtime_missing: Option<String>,
}

/// The list as the view draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    /// No daemon is configured: show the empty state and the add action.
    pub empty: bool,
    /// One row per daemon, in registry order.
    pub rows: Vec<DaemonView>,
    /// The daemons a worktree can be bound to (`Bind to daemon...`); empty with none configured.
    pub bind_choices: Vec<(DaemonId, String)>,
}

/// What the Daemons feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The open add/edit form.
    pub draft: Option<Draft>,
    pending: Option<(Pending, RemovalPrompt)>,
    diagnostics: BTreeMap<DaemonId, DaemonDiagnostics>,
    versions: BTreeMap<DaemonId, String>,
    runtime_missing: BTreeMap<DaemonId, String>,
    ended: Vec<SessionId>,
}

impl State {
    /// The confirmation waiting for an answer.
    pub fn prompt(&self) -> Option<&RemovalPrompt> {
        self.pending.as_ref().map(|(_, prompt)| prompt)
    }

    /// Sessions that ended because their daemon was removed.
    pub fn ended_by_removal(&self) -> &[SessionId] {
        &self.ended
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Every location bound to `id`: explicit bindings and the known locations that resolve to it.
fn bound_to(model: &Model<'_>, id: DaemonId) -> Vec<WorktreeRef> {
    let mut all: BTreeSet<WorktreeRef> = model.locations.iter().cloned().collect();
    for (project, keys) in model.bindings.iter() {
        for key in keys.keys() {
            all.insert(WorktreeRef {
                project: project.clone(),
                worktree: key.clone(),
            });
        }
    }
    all.into_iter()
        .filter(|loc| resolve(model, loc) == Binding::Bound(id))
        .collect()
}

fn resolve(model: &Model<'_>, loc: &WorktreeRef) -> Binding {
    let empty = BTreeMap::new();
    let stored = model.bindings.get(&loc.project).unwrap_or(&empty);
    Binding::resolve(stored, &loc.worktree, model.registry, model.legacy_default)
}

/// The list, its empty state and the daemons a worktree can bind to.
pub fn views(state: &State, model: &Model<'_>) -> ViewModel {
    let rows = model
        .registry
        .entries()
        .iter()
        .map(|entry| DaemonView {
            id: entry.id,
            name: entry.name.as_str().to_string(),
            runtime_label: entry.runtime.label().to_string(),
            endpoint: match &entry.runtime {
                DaemonRuntime::Container(c) => format!("127.0.0.1:{}", c.port),
                DaemonRuntime::Host => "local".to_string(),
                DaemonRuntime::Unsupported { kind, .. } => format!("unsupported ({kind})"),
                _ => String::new(),
            },
            version: state.versions.get(&entry.id).cloned(),
            state: model
                .links
                .state(entry.id)
                .cloned()
                .unwrap_or(DaemonState::Stopped),
            bound: bound_to(model, entry.id),
        })
        .collect::<Vec<_>>();
    ViewModel {
        empty: rows.is_empty(),
        bind_choices: rows.iter().map(|r| (r.id, r.name.clone())).collect(),
        rows,
    }
}

/// One daemon's recent errors, log tail and, for a missing runtime, the reason.
pub fn details(state: &State, id: DaemonId) -> DaemonDetails {
    let diagnostics = state.diagnostics.get(&id);
    DaemonDetails {
        errors: diagnostics
            .map(DaemonDiagnostics::errors)
            .unwrap_or_default(),
        log: diagnostics.map(DaemonDiagnostics::log).unwrap_or_default(),
        runtime_missing: state.runtime_missing.get(&id).cloned(),
    }
}

/// Fold a daemon's event into its state (only that daemon's), keeping what a failure said.
pub fn daemon_event(
    state: &mut State,
    links: &mut DaemonLinks,
    id: DaemonId,
    event: DaemonEvent,
) -> Option<StateChange> {
    let diagnostics = state.diagnostics.entry(id).or_default();
    match &event {
        DaemonEvent::DialFailed(reason) | DaemonEvent::Lost(reason) => {
            diagnostics.record_error(reason.clone());
        }
        DaemonEvent::Refused { client, daemon } => {
            diagnostics.record_error(format!(
                "version mismatch: client {client}, daemon {daemon}"
            ));
        }
        _ => {}
    }
    let change = links.apply(id, event);
    if let Some(change) = &change {
        diagnostics.push_log(format!("state: {:?}", change.to));
    }
    change
}

/// The daemon reported its version.
pub fn version_reported(state: &mut State, id: DaemonId, version: &str) {
    state.versions.insert(id, version.to_string());
}

/// A line of the daemon's log arrived.
pub fn log_line(state: &mut State, id: DaemonId, line: impl Into<String>) {
    state.diagnostics.entry(id).or_default().push_log(line);
}

/// The container runtime cannot be used, and why.
pub fn runtime_missing(state: &mut State, id: DaemonId, reason: &str) {
    state.runtime_missing.insert(id, reason.to_string());
    state
        .diagnostics
        .entry(id)
        .or_default()
        .record_error(reason);
}

/// Start `id`, and only `id`. Refused for a runtime this build cannot run.
pub fn start(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    if !model.registry.get(id)?.runtime.is_startable() {
        return None;
    }
    state.runtime_missing.remove(&id);
    daemon_event(state, model.links, id, DaemonEvent::Start)
}

/// Reconnect `id` now rather than at the next backoff.
pub fn reconnect(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    start(state, model, id)
}

/// Stop `id`, and only `id`: it is neither dialled nor respawned until started.
pub fn stop(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    daemon_event(state, model.links, id, DaemonEvent::Stop)
}

/// Open the form for a new daemon.
pub fn begin_add(state: &mut State) {
    state.draft = Some(Draft {
        editing: None,
        name: String::new(),
        runtime: DraftRuntime::default(),
        container_name: String::new(),
        port: String::new(),
        profile: SandboxProfile::default(),
        auto_start: true,
    });
}

/// Open the form for `id`, filled from its entry.
pub fn begin_edit<'s>(state: &'s mut State, model: &Model<'_>, id: DaemonId) -> Option<&'s Draft> {
    let entry = model.registry.get(id)?;
    let (runtime, container_name, port, profile) = match &entry.runtime {
        DaemonRuntime::Container(c) => (
            DraftRuntime::Container,
            c.container_name.clone(),
            c.port.to_string(),
            c.profile.clone(),
        ),
        _ => (
            DraftRuntime::Host,
            String::new(),
            String::new(),
            SandboxProfile::default(),
        ),
    };
    state.draft = Some(Draft {
        editing: Some(id),
        name: entry.name.as_str().to_string(),
        runtime,
        container_name,
        port,
        profile,
        auto_start: entry.auto_start,
    });
    state.draft.as_ref()
}

/// Close the form without saving.
pub fn discard_draft(state: &mut State) {
    state.draft = None;
}

fn form_error(registry: &DaemonRegistry, draft: &Draft, err: RegistryError) -> FormError {
    let named = |id: DaemonId| {
        registry
            .get(id)
            .map(|e| e.name.as_str().to_string())
            .unwrap_or_default()
    };
    match err {
        RegistryError::BlankName => FormError {
            field: "name",
            message: "A daemon needs a name.".into(),
        },
        RegistryError::DuplicateName => FormError {
            field: "name",
            message: format!("Another daemon is already named {}.", draft.name.trim()),
        },
        RegistryError::HostExists => FormError {
            field: "runtime",
            message: "A host daemon already exists; only one can run on this machine.".into(),
        },
        RegistryError::ContainerInUse { other } => {
            let same_name = matches!(
                registry.get(other).and_then(|e| e.container_name()),
                Some(c) if c == draft.container_name
            );
            let (field, what) = if same_name {
                ("container_name", "container")
            } else {
                ("port", "port")
            };
            FormError {
                field,
                message: format!("That {what} is already used by daemon {}.", named(other)),
            }
        }
        RegistryError::InvalidField { field } => FormError {
            field,
            message: match field {
                "port" => "Enter a port between 1 and 65535.".to_string(),
                "image" => "The image reference is not valid.".to_string(),
                "container_name" => "Enter a container name.".to_string(),
                other => format!("{other} is not valid."),
            },
        },
        RegistryError::UnknownDaemon => FormError {
            field: "name",
            message: "That daemon no longer exists.".into(),
        },
    }
}

fn runtime_of(draft: &Draft) -> Result<DaemonRuntime, FormError> {
    match draft.runtime {
        DraftRuntime::Host => Ok(DaemonRuntime::Host),
        DraftRuntime::Container => {
            let port = draft.port.trim().parse::<u16>().map_err(|_| FormError {
                field: "port",
                message: "Enter a port between 1 and 65535.".into(),
            })?;
            Ok(DaemonRuntime::Container(
                micold_core::daemons::ContainerSettings {
                    profile: draft.profile.clone(),
                    container_name: draft.container_name.trim().to_string(),
                    port,
                },
            ))
        }
    }
}

fn prompt_for(model: &Model<'_>, id: DaemonId, action: &str, effect: &str) -> RemovalPrompt {
    let name = model
        .registry
        .get(id)
        .map(|e| e.name.as_str().to_string())
        .unwrap_or_default();
    let worktrees_affected = bound_to(model, id).len();
    let running_sessions = model.sessions.get(&id).map_or(0, Vec::len);
    let text = format!(
        "{action} daemon {name}? {} {effect}, and {} will stop. The worktrees are kept; \
         nothing on disk is deleted, and the container is not stopped or deleted.",
        plural(worktrees_affected, "worktree", "worktrees"),
        plural(running_sessions, "running session", "running sessions"),
    );
    RemovalPrompt {
        daemon: id,
        name,
        worktrees_affected,
        running_sessions,
        text,
    }
}

fn started_state(entry_auto_start: bool, runtime: &DaemonRuntime) -> DaemonState {
    if entry_auto_start && runtime.is_startable() {
        DaemonState::Starting
    } else {
        DaemonState::Stopped
    }
}

/// Save the open form: an add, an edit, or (for a running daemon's runtime settings) a request for
/// the counted confirmation. A refusal names the field and changes nothing.
pub fn save(state: &mut State, model: &mut Model<'_>) -> Result<Vec<Outcome>, FormError> {
    let Some(draft) = state.draft.clone() else {
        return Ok(Vec::new());
    };
    let name = DaemonName::new(&draft.name)
        .map_err(|e| form_error(model.registry, &draft, e))?
        .as_str()
        .to_string();
    let runtime = runtime_of(&draft)?;
    // Validate on a copy so a refusal leaves the registry exactly as it was.
    let mut probe = model.registry.clone();
    let probed = match draft.editing {
        Some(id) => probe
            .edit(id, &name, runtime.clone(), draft.auto_start)
            .map(|()| id),
        None => probe.add(&name, runtime.clone(), draft.auto_start),
    };
    let id = probed.map_err(|e| form_error(model.registry, &draft, e))?;

    match draft.editing {
        None => {
            *model.registry = probe;
            model
                .links
                .insert(id, started_state(draft.auto_start, &runtime));
            state.draft = None;
            Ok(vec![Outcome::DaemonRegistryChanged(model.registry.clone())])
        }
        Some(id) => {
            let runtime_changed = model.registry.get(id).map(|e| &e.runtime) != Some(&runtime);
            if runtime_changed && model.links.should_dial(id) {
                let prompt = prompt_for(
                    model,
                    id,
                    "Apply new runtime settings to",
                    "use it and will be reconnected",
                );
                state.pending = Some((
                    Pending::Edit {
                        id,
                        name,
                        runtime,
                        auto_start: draft.auto_start,
                    },
                    prompt,
                ));
                return Ok(Vec::new());
            }
            *model.registry = probe;
            state.draft = None;
            Ok(vec![Outcome::DaemonRegistryChanged(model.registry.clone())])
        }
    }
}

/// Ask to remove `id`: the confirmation, with both counts.
pub fn request_removal<'s>(
    state: &'s mut State,
    model: &Model<'_>,
    id: DaemonId,
) -> Option<&'s RemovalPrompt> {
    model.registry.get(id)?;
    let prompt = prompt_for(
        model,
        id,
        "Remove",
        "will show \"no daemon\" until bound again",
    );
    state.pending = Some((Pending::Remove(id), prompt));
    state.prompt()
}

/// Answer the confirmation with no: nothing changes.
pub fn cancel(state: &mut State) {
    state.pending = None;
}

/// Answer the confirmation with yes. Only the daemon concerned is touched: its sessions end, its
/// actor stops (and, for an edit, starts again), and for a removal its entry goes. Bindings are
/// left alone and resolve to no daemon; nothing on disk and no container is touched.
pub fn confirm(state: &mut State, model: &mut Model<'_>) -> Vec<Outcome> {
    let Some((pending, _)) = state.pending.take() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match pending {
        Pending::Remove(id) => {
            let sessions = model.sessions.get(&id).cloned().unwrap_or_default();
            model.links.apply(id, DaemonEvent::Stop);
            model.links.remove(id);
            model.registry.remove(id);
            state.diagnostics.remove(&id);
            state.versions.remove(&id);
            state.runtime_missing.remove(&id);
            state.ended.extend(sessions.iter().copied());
            if !sessions.is_empty() {
                out.push(Outcome::DaemonSessionsEnded {
                    daemon: id,
                    sessions,
                });
            }
        }
        Pending::Edit {
            id,
            name,
            runtime,
            auto_start,
        } => {
            let sessions = model.sessions.get(&id).cloned().unwrap_or_default();
            if model
                .registry
                .edit(id, &name, runtime.clone(), auto_start)
                .is_err()
            {
                return out;
            }
            model.links.apply(id, DaemonEvent::Stop);
            if runtime.is_startable() {
                model.links.apply(id, DaemonEvent::Start);
            }
            state.runtime_missing.remove(&id);
            state.draft = None;
            if !sessions.is_empty() {
                out.push(Outcome::DaemonSessionsEnded {
                    daemon: id,
                    sessions,
                });
            }
        }
    }
    out.push(Outcome::DaemonRegistryChanged(model.registry.clone()));
    out
}

/// Where an op for `location` goes. A location with no daemon is refused: no session or terminal
/// starts there until it is bound by an explicit action (FR-014).
pub fn route_for(model: &Model<'_>, location: &WorktreeRef) -> Route {
    route(model.links, model.registry, resolve(model, location))
}

/// `Bind to daemon...`: write the explicit binding of `location`. Refused for an unknown daemon.
pub fn bind_worktree(
    model: &mut Model<'_>,
    location: &WorktreeRef,
    daemon: DaemonId,
) -> Result<Vec<Outcome>, RegistryError> {
    if model.registry.get(daemon).is_none() {
        return Err(RegistryError::UnknownDaemon);
    }
    model
        .bindings
        .entry(location.project.clone())
        .or_default()
        .insert(location.worktree.clone(), daemon);
    Ok(vec![Outcome::DaemonBound {
        project: location.project.clone(),
        worktree: location.worktree.clone(),
        daemon,
    }])
}

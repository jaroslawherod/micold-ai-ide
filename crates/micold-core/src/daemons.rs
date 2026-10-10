//! The registry of session daemons and the worktree-to-daemon binding (feature 491).
//!
//! The client owns the list; each daemon stays an independent process. Everything here is a pure
//! rule over plain data, so it is testable without a runtime (Constitution Principle I).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sandbox::SandboxProfile;

/// Stable key of a daemon. Allocated from the registry's counter and never reused, so removing a
/// daemon and adding another of the same name cannot inherit its bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DaemonId(pub u32);

/// A daemon's display name: trimmed, non-empty, unique case-insensitively within a registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DaemonName(String);

/// The container runtime's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSettings {
    /// What the container may reach and run.
    #[serde(default)]
    pub profile: SandboxProfile,
    /// The container's name, which is also how an existing one is found and adopted.
    pub container_name: String,
    /// The loopback port its control channel is published on.
    pub port: u16,
}

/// What a daemon runs in. Runtime is data, so later runtimes add a variant, not a surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DaemonRuntime {
    /// A detached process on this machine.
    Host,
    /// A container on this machine.
    Container(ContainerSettings),
    /// A runtime this build does not know: kept verbatim on save, never started.
    Unsupported {
        /// The `kind` the file named.
        kind: String,
        /// The whole stored value, written back unchanged.
        raw: serde_json::Value,
    },
}

/// One configured daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonEntry {
    /// Stable identity.
    pub id: DaemonId,
    /// Display name.
    pub name: DaemonName,
    /// What it runs in.
    pub runtime: DaemonRuntime,
    /// Connect this daemon when the app launches.
    #[serde(default)]
    pub auto_start: bool,
}

/// Why a registry change was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Another daemon has this name, ignoring case.
    DuplicateName,
    /// The name is empty after trimming.
    BlankName,
    /// Another daemon already uses this container name or port.
    ContainerInUse {
        /// The daemon that holds it.
        other: DaemonId,
    },
    /// A field is missing or out of range.
    InvalidField {
        /// Which field.
        field: &'static str,
    },
    /// A host daemon exists already.
    HostExists,
    /// No daemon has this id.
    UnknownDaemon,
}

/// The ordered list of daemons and the counter ids are allocated from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DaemonRegistry {
    entries: Vec<DaemonEntry>,
    next_id: u32,
}

/// Where a worktree runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// On this daemon.
    Bound(DaemonId),
    /// Its daemon was removed, or it never had one a legacy default can supply.
    NoDaemon,
}

impl DaemonName {
    /// Trim and check a name (uniqueness is the registry's rule).
    pub fn new(raw: &str) -> Result<Self, RegistryError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(RegistryError::BlankName);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// The name as shown.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DaemonRuntime {
    /// The plain value shown in lists.
    pub fn label(&self) -> &'static str {
        match self {
            DaemonRuntime::Host => "Host",
            DaemonRuntime::Container(_) => "Container",
            DaemonRuntime::Unsupported { .. } => "Unsupported",
        }
    }

    /// Whether this build can start a daemon of this runtime.
    pub fn is_startable(&self) -> bool {
        !matches!(self, DaemonRuntime::Unsupported { .. })
    }
}

impl DaemonRegistry {
    /// A registry over stored entries and the stored counter.
    pub fn new(entries: Vec<DaemonEntry>, next_id: u32) -> Self {
        Self { entries, next_id }
    }

    /// The entries in order.
    pub fn entries(&self) -> &[DaemonEntry] {
        &self.entries
    }

    /// The id the next added daemon receives.
    pub fn next_id(&self) -> u32 {
        self.next_id
    }

    /// The entry with this id.
    pub fn get(&self, id: DaemonId) -> Option<&DaemonEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Add a daemon; returns its new id.
    pub fn add(
        &mut self,
        name: &str,
        runtime: DaemonRuntime,
        auto_start: bool,
    ) -> Result<DaemonId, RegistryError> {
        let name = DaemonName::new(name)?;
        self.check(None, &name, &runtime)?;
        let id = DaemonId(self.next_id.max(1));
        self.next_id = id.0 + 1;
        self.entries.push(DaemonEntry {
            id,
            name,
            runtime,
            auto_start,
        });
        Ok(id)
    }

    /// Replace a daemon's name, runtime and auto-start; its id never changes.
    pub fn edit(
        &mut self,
        id: DaemonId,
        name: &str,
        runtime: DaemonRuntime,
        auto_start: bool,
    ) -> Result<(), RegistryError> {
        let name = DaemonName::new(name)?;
        if self.get(id).is_none() {
            return Err(RegistryError::UnknownDaemon);
        }
        self.check(Some(id), &name, &runtime)?;
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.name = name;
            entry.runtime = runtime;
            entry.auto_start = auto_start;
        }
        Ok(())
    }

    /// The rules every change must satisfy, ignoring the entry being edited.
    fn check(
        &self,
        editing: Option<DaemonId>,
        name: &DaemonName,
        runtime: &DaemonRuntime,
    ) -> Result<(), RegistryError> {
        let others = || self.entries.iter().filter(|e| Some(e.id) != editing);
        if others().any(|e| e.name.0.to_lowercase() == name.0.to_lowercase()) {
            return Err(RegistryError::DuplicateName);
        }
        match runtime {
            DaemonRuntime::Host => {
                if others().any(|e| e.runtime == DaemonRuntime::Host) {
                    return Err(RegistryError::HostExists);
                }
            }
            DaemonRuntime::Container(c) => {
                if c.container_name.trim().is_empty() {
                    return Err(RegistryError::InvalidField {
                        field: "container_name",
                    });
                }
                if c.port == 0 {
                    return Err(RegistryError::InvalidField { field: "port" });
                }
                if c.profile.image.validate().iter().any(|p| p.is_fatal()) {
                    return Err(RegistryError::InvalidField { field: "image" });
                }
                let holder = others().find_map(|e| match &e.runtime {
                    DaemonRuntime::Container(o)
                        if o.container_name == c.container_name || o.port == c.port =>
                    {
                        Some(e.id)
                    }
                    _ => None,
                });
                if let Some(other) = holder {
                    return Err(RegistryError::ContainerInUse { other });
                }
            }
            // A runtime this build cannot run is only ever loaded, never added.
            DaemonRuntime::Unsupported { .. } => {
                return Err(RegistryError::InvalidField { field: "runtime" });
            }
        }
        Ok(())
    }

    /// Remove a daemon. Nothing else changes: its bindings simply resolve to no daemon.
    pub fn remove(&mut self, id: DaemonId) -> Option<DaemonEntry> {
        let at = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(at))
    }
}

/// Consecutive failed dials before a daemon that never connected is shown unreachable. With the
/// client's one-second reconnect backoff this stays inside the 5 s budget of SC-002.
pub const UNREACHABLE_AFTER_FAILURES: u32 = 3;

/// Where one daemon's connection stands (FR-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonState {
    /// Being brought up or dialled; not yet connected.
    Starting,
    /// Handshake done; sessions can run.
    Connected,
    /// Not reachable now; the client keeps retrying.
    Unreachable {
        /// What the last attempt reported.
        reason: String,
    },
    /// The daemon and the client disagree on version; retrying cannot help.
    VersionMismatch {
        /// The client's version.
        client: String,
        /// The daemon's version.
        daemon: String,
    },
    /// Neither spawned nor reconnected until started.
    Stopped,
}

/// Something that happened to one daemon's connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonEvent {
    /// The user, or the launch, asked it to run.
    Start,
    /// The handshake succeeded.
    Connected,
    /// A dial or bring-up attempt failed.
    DialFailed(String),
    /// An established connection ended.
    Lost(String),
    /// The daemon refused the handshake on a version difference.
    Refused {
        /// The client's version.
        client: String,
        /// The daemon's version.
        daemon: String,
    },
    /// The user stopped it, or it was removed.
    Stop,
}

/// A state change of exactly one daemon: the only thing a transition reports (FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChange {
    /// The daemon that changed.
    pub id: DaemonId,
    /// What it is now.
    pub to: DaemonState,
}

/// The state of every daemon, each folded independently of the others.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonStates {
    states: BTreeMap<DaemonId, (DaemonState, u32)>,
}

impl DaemonStates {
    /// States at app launch: `Starting` for an `auto_start` daemon this build can run, `Stopped`
    /// otherwise, so a stopped daemon is neither spawned nor reconnected.
    pub fn at_launch(registry: &DaemonRegistry) -> Self {
        let mut states = Self::default();
        for entry in registry.entries() {
            let state = if entry.auto_start && entry.runtime.is_startable() {
                DaemonState::Starting
            } else {
                DaemonState::Stopped
            };
            states.insert(entry.id, state);
        }
        states
    }

    /// Track `id` in `state`.
    pub fn insert(&mut self, id: DaemonId, state: DaemonState) {
        self.states.insert(id, (state, 0));
    }

    /// Forget `id`.
    pub fn remove(&mut self, id: DaemonId) {
        self.states.remove(&id);
    }

    /// The daemon's state, when tracked.
    pub fn state(&self, id: DaemonId) -> Option<&DaemonState> {
        self.states.get(&id).map(|(state, _)| state)
    }

    /// Whether the client should be dialling this daemon: tracked and not stopped.
    pub fn should_dial(&self, id: DaemonId) -> bool {
        matches!(self.state(id), Some(s) if *s != DaemonState::Stopped)
    }

    /// Fold `event` into `id`'s state, touching no other daemon. `None` when nothing changed.
    pub fn apply(&mut self, id: DaemonId, event: DaemonEvent) -> Option<StateChange> {
        let (state, failures) = self.states.get_mut(&id)?;
        let next = match (&*state, event) {
            (DaemonState::Stopped, DaemonEvent::Start) => Some(DaemonState::Starting),
            (DaemonState::Stopped, _) => None,
            (_, DaemonEvent::Stop) => Some(DaemonState::Stopped),
            (DaemonState::Connected, DaemonEvent::Start) => None,
            (_, DaemonEvent::Start) => {
                *failures = 0;
                Some(DaemonState::Starting)
            }
            (_, DaemonEvent::Connected) => {
                *failures = 0;
                Some(DaemonState::Connected)
            }
            (_, DaemonEvent::Refused { client, daemon }) => {
                Some(DaemonState::VersionMismatch { client, daemon })
            }
            // A mismatch is not cured by a dial that fails again.
            (DaemonState::VersionMismatch { .. }, _) => None,
            (
                DaemonState::Connected,
                DaemonEvent::Lost(reason) | DaemonEvent::DialFailed(reason),
            ) => Some(DaemonState::Unreachable { reason }),
            (_, DaemonEvent::Lost(reason) | DaemonEvent::DialFailed(reason)) => {
                *failures += 1;
                match state {
                    DaemonState::Starting if *failures < UNREACHABLE_AFTER_FAILURES => None,
                    _ => Some(DaemonState::Unreachable { reason }),
                }
            }
        };
        let next = next.filter(|n| n != state)?;
        *state = next.clone();
        Some(StateChange { id, to: next })
    }
}

/// The single daemon a pre-feature `daemon` block stands for (R5): id 1, named by its runtime,
/// carrying the legacy container name and port so a running container is adopted, not recreated.
pub fn migrated_entry(config: &crate::settings::DaemonConfig) -> DaemonEntry {
    use crate::sandbox::placement::PlacementKind;
    let (name, runtime) = match config.placement {
        PlacementKind::HostProcess => ("Host", DaemonRuntime::Host),
        PlacementKind::LocalSandbox => (
            "Container",
            DaemonRuntime::Container(ContainerSettings {
                profile: config.sandbox.clone(),
                container_name: crate::sandbox::CONTAINER_NAME.to_string(),
                port: crate::endpoint::DEFAULT_SANDBOX_PORT,
            }),
        ),
    };
    DaemonEntry {
        id: DaemonId(1),
        name: DaemonName(name.to_string()),
        runtime,
        // Today's daemon is always brought up at launch.
        auto_start: true,
    }
}

impl DaemonEntry {
    /// The container's name, when this daemon runs in one.
    pub fn container_name(&self) -> Option<&str> {
        match &self.runtime {
            DaemonRuntime::Container(c) => Some(&c.container_name),
            _ => None,
        }
    }

    /// The daemon's own state directory under `base` (the per-user state directory): the legacy
    /// default daemon keeps `base` itself so its running container is adopted, every other
    /// container daemon gets `base/daemons/<slug>`.
    pub fn state_dir(
        &self,
        base: &std::path::Path,
        legacy_default: Option<DaemonId>,
    ) -> Option<std::path::PathBuf> {
        let DaemonRuntime::Container(c) = &self.runtime else {
            return None;
        };
        if legacy_default == Some(self.id) {
            return Some(base.to_path_buf());
        }
        let slug: String = c
            .container_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        Some(base.join("daemons").join(slug))
    }

    /// Where this daemon's token lives on the host.
    pub fn token_path(
        &self,
        base: &std::path::Path,
        legacy_default: Option<DaemonId>,
    ) -> Option<std::path::PathBuf> {
        self.state_dir(base, legacy_default)
            .map(|dir| crate::protocol::auth::host_token_path(&dir))
    }
}

impl Binding {
    /// Resolve a worktree's binding from the project's stored map, keyed by worktree `dir_name`
    /// with `""` for the Default location.
    pub fn resolve(
        bindings: &BTreeMap<String, DaemonId>,
        key: &str,
        registry: &DaemonRegistry,
        legacy_default: Option<DaemonId>,
    ) -> Binding {
        let id = bindings.get(key).copied().or(legacy_default);
        match id {
            Some(id) if registry.get(id).is_some() => Binding::Bound(id),
            _ => Binding::NoDaemon,
        }
    }
}

impl Serialize for DaemonRuntime {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        let value = match self {
            DaemonRuntime::Host => serde_json::json!({ "kind": "host" }),
            DaemonRuntime::Container(c) => {
                let mut value = serde_json::to_value(c).map_err(S::Error::custom)?;
                value["kind"] = "container".into();
                value
            }
            DaemonRuntime::Unsupported { raw, .. } => raw.clone(),
        };
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DaemonRuntime {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let kind = raw
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok(match kind.as_str() {
            "host" => DaemonRuntime::Host,
            "container" => match serde_json::from_value::<ContainerSettings>(raw.clone()) {
                Ok(c) => DaemonRuntime::Container(c),
                // A container entry this build cannot read is kept, not repaired.
                Err(_) => DaemonRuntime::Unsupported { kind, raw },
            },
            _ => DaemonRuntime::Unsupported { kind, raw },
        })
    }
}

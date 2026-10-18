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
use micold_core::kube::lifecycle::leftovers;
use micold_core::kube::names::ResourceNames;
use micold_core::kube::settings::KubeSettings;
use micold_core::provision::{ProvisionEvent, Step};
use micold_core::required::{RequiredMarks, Requirement};
use micold_core::sandbox::image::{ImageSourceKind, ImageSourceProblem};
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::{CredentialShare, NetworkPosture, SandboxProfile};
use micold_core::session::AiCli;
use micold_core::session::SessionId;

pub use super::daemons_kube::{AllowedKind, ContextList, Editor, KubeDraft, VOLUME_NOTICE};
use super::settings::{parse_budget, LimitField, ProfileTexts};
use super::window::FieldId;
use super::Outcome;
use crate::floating_surface;
use crate::kube::live::ContextEntry;
use crate::links::{route, DaemonLinks, Route};
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;

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
    /// Container ports other windows' registries hold, which a picked port must also avoid.
    pub taken_elsewhere: Vec<u16>,
    /// Where a free port comes from when a container daemon is saved with none.
    pub next_port: &'a dyn Fn() -> std::io::Result<u16>,
}

/// Which runtime the form is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DraftRuntime {
    /// A detached process on this machine.
    Host,
    /// A container on this machine.
    #[default]
    Container,
    /// A host reached through the user's own `ssh` (feature 489).
    Ssh,
    /// A pod in a Kubernetes cluster (feature 686).
    Kubernetes,
}

/// Whether this build has #806's shared SSH image mechanism, without which a Kubernetes daemon
/// cannot start (FR-024). While it is `false` the Add form does not offer Kubernetes; a Kubernetes
/// daemon already in the settings still opens in Edit. #806 flips it.
pub const SHARED_SSH_IMAGE_AVAILABLE: bool = false;

/// The runtimes the form's "Runs in" choice offers for `draft`: Kubernetes only once the shared
/// mechanism exists, or while editing a Kubernetes daemon (FR-024).
pub fn runtime_choices(draft: &Draft) -> Vec<DraftRuntime> {
    let mut choices = vec![
        DraftRuntime::Container,
        DraftRuntime::Host,
        DraftRuntime::Ssh,
    ];
    if SHARED_SSH_IMAGE_AVAILABLE || draft.editing.is_some() && draft.kube_edited {
        choices.push(DraftRuntime::Kubernetes);
    }
    choices
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
    /// The image archive path as typed (container runtime only; empty means none).
    pub image_path: String,
    /// The processor limit in cores as typed; empty means unset (rule RB-2).
    pub cpus: String,
    /// The memory limit in MiB as typed; empty means unset.
    pub memory_mib: String,
    /// The process-count limit as typed; empty means unset.
    pub pids: String,
    /// The writable-storage limit in MiB as typed; empty means unset.
    pub storage_mib: String,
    /// Connect at launch.
    pub auto_start: bool,
    /// The host or `Host` alias (SSH runtime only, feature 489).
    pub ssh_host: String,
    /// The remote user; blank takes it from the SSH configuration (SSH runtime only).
    pub ssh_user: String,
    /// The remote port as typed; blank takes it from the SSH configuration (SSH runtime only).
    pub ssh_port: String,
    /// The private key file; blank takes it from the SSH configuration (SSH runtime only).
    pub ssh_key_file: String,
    /// The Kubernetes fields (feature 686); used while `runtime` is Kubernetes.
    pub kube: KubeDraft,
    /// Whether the daemon being edited is a Kubernetes one, so its runtime stays on offer.
    pub kube_edited: bool,
    /// Which fields the user has left and whether a save was attempted, for the required errors
    /// (feature 937). Fresh when the form opens, dropped with it.
    pub required: RequiredMarks<FieldId>,
}

impl Draft {
    /// The note under the volume size once the daemon exists (FR-005), so an edit is not taken to
    /// resize a volume that is already there.
    pub fn volume_notice(&self) -> Option<&'static str> {
        (self.editing.is_some() && self.runtime == DraftRuntime::Kubernetes)
            .then_some(VOLUME_NOTICE)
    }

    /// The fields the form refuses empty for the runtime chosen now, in display order, with
    /// whether each is empty (feature 937, FR-002). Mirrors [`runtime_of`] and the registry's
    /// validation: the SSH host only over SSH, the container name and image reference for a
    /// container, the image file only for an imported-file image source. Port, SSH user and key
    /// file are never required: each has a default.
    pub fn required_fields(&self) -> Vec<(FieldId, Requirement)> {
        let mut fields = vec![(FieldId::DaemonName, Requirement::text(true, &self.name))];
        match self.runtime {
            DraftRuntime::Host => {}
            DraftRuntime::Ssh => fields.push((
                FieldId::DaemonSshHost,
                Requirement::text(true, &self.ssh_host),
            )),
            DraftRuntime::Kubernetes => {
                fields.push((
                    FieldId::DaemonKubeContext,
                    Requirement::text(true, &self.kube.context),
                ));
                fields.push((
                    FieldId::DaemonKubeNamespace,
                    Requirement::text(true, &self.kube.namespace),
                ));
            }
            DraftRuntime::Container => {
                fields.push((
                    FieldId::DaemonContainerName,
                    Requirement::text(true, &self.container_name),
                ));
                fields.push((
                    FieldId::DaemonImage,
                    Requirement::text(true, &self.profile.image.reference),
                ));
                if self.profile.image.kind == ImageSourceKind::ImportedFile {
                    fields.push((
                        FieldId::DaemonImagePath,
                        Requirement::text(true, &self.image_path),
                    ));
                }
            }
        }
        fields
    }
}

/// The five titled sections a container daemon's settings are grouped in (feature 804 M4, FR-012).
/// View state only: which are open is never saved (FR-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    /// Container runtime and image.
    Container,
    /// What the container may use of the user's credentials.
    Credentials,
    /// The network posture.
    Network,
    /// Processor, memory, process and storage limits.
    Limits,
    /// Survive-logout.
    Sessions,
}

impl Section {
    /// Every section, in the order the form lists them.
    pub const ALL: [Section; 5] = [
        Section::Container,
        Section::Credentials,
        Section::Network,
        Section::Limits,
        Section::Sessions,
    ];

    /// The header text.
    pub fn title(self) -> &'static str {
        match self {
            Section::Container => "Container",
            Section::Credentials => "Credentials",
            Section::Network => "Network",
            Section::Limits => "Limits",
            Section::Sessions => "Sessions",
        }
    }

    /// Whether `field` (a [`FormError::field`]) is one of this section's.
    fn holds(self, field: &str) -> bool {
        match self {
            Section::Container => matches!(field, "image" | "image_path"),
            Section::Limits => matches!(field, "cpus" | "memory" | "pids" | "storage"),
            _ => false,
        }
    }
}

/// A refused save: the field at fault and what to tell the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormError {
    /// `name`, `runtime`, `container_name`, `port`, `image`, `image_path`, `cpus`, `memory`, `pids`
    /// or `storage`.
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
    /// What the confirm button says: the action it carries out (`Stop`, `Remove`, ...).
    pub confirm_label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Remove(DaemonId),
    /// Stop a Kubernetes daemon's pod.
    StopKube(DaemonId),
    /// Delete a Kubernetes daemon's cluster objects, then forget it.
    RemoveKube(DaemonId),
    /// Forget a Kubernetes daemon on this computer only.
    ForgetLocally(DaemonId),
    Edit {
        id: DaemonId,
        name: String,
        runtime: Box<DaemonRuntime>,
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
    /// A Kubernetes daemon was edited while running: the note says the change waits for the next
    /// start (FR-005).
    pub applies_on_next_start: bool,
    /// The pod's log tail as last fetched, or why it could not be (FR-018).
    pub pod_log: Option<Result<String, String>>,
}

/// What the details say of an edit saved while the pod runs (FR-005).
pub const APPLIES_ON_NEXT_START: &str = "Applies on next start";

/// How many lines of the pod's log Logs fetches.
pub const LOG_TAIL_LINES: u32 = 200;

/// The list as the view draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    /// No daemon is configured: show the empty state and the add action.
    pub empty: bool,
    /// One row per daemon, in registry order.
    pub rows: Vec<DaemonView>,
    /// The daemons a worktree can be bound to (`Bind to daemon...`); empty with none configured.
    pub bind_choices: Vec<(DaemonId, String)>,
    /// The daemon an unbound location resolves to: the one whose container the machine facts
    /// (probed limits, an unmounted sign-in) describe.
    pub legacy_default: Option<DaemonId>,
    /// Whether any container daemon shares a host credential, counting the open form's unsaved
    /// choice in place of its daemon's stored one (feature 804, FR-007). The settings rail's
    /// *Sharing* badge on the Daemons row reads it, so "am I sharing anything?" is answerable from
    /// any section.
    pub shares_credentials: bool,
}

/// Where one daemon's start is: the step it is in and, while it copies the build, how far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartProgress {
    step: Option<Step>,
    percent: Option<u8>,
}

impl StartProgress {
    fn apply(&mut self, event: ProvisionEvent) {
        match event {
            ProvisionEvent::Entered(step) => {
                self.step = Some(step);
                self.percent = None;
            }
            ProvisionEvent::CopyPercent(p) => self.percent = Some(p),
            ProvisionEvent::InstallLine(_) | ProvisionEvent::InstallDone { .. } => {}
        }
    }

    /// The row's line: the step's label, then the percent while there is one.
    fn text(&self) -> Option<String> {
        let label = self.step?.label();
        Some(match self.percent {
            Some(p) => format!("{label} ({p}%)"),
            None => label.to_string(),
        })
    }
}

/// What the Daemons feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The open add/edit form.
    pub draft: Option<Draft>,
    /// The sentence beside the form field the last refused save was about.
    pub form_error: Option<FormError>,
    /// The sections the user has flipped from how the form opens them (Container open, the rest
    /// closed). View state: never persisted, forgotten when the form closes (FR-015).
    flipped_sections: BTreeSet<Section>,
    /// The daemons whose details (errors and log tail) are open.
    pub details_shown: BTreeSet<DaemonId>,
    /// The location the `Bind to daemon...` picker is open for.
    pub bind_target: Option<WorktreeRef>,
    pending: Option<(Pending, RemovalPrompt)>,
    diagnostics: BTreeMap<DaemonId, DaemonDiagnostics>,
    versions: BTreeMap<DaemonId, String>,
    runtime_missing: BTreeMap<DaemonId, String>,
    /// The start step an SSH daemon is in and, while it copies its build, the percent (features
    /// 489 and 806). Per daemon: one daemon's start never touches another's (FR-022).
    provisioning: BTreeMap<DaemonId, StartProgress>,
    /// The repository path typed under each remote daemon (feature 489, FR-012).
    pub repositories: BTreeMap<DaemonId, String>,
    /// The sentence under the repository field the last refused add was about.
    pub repository_error: Option<(DaemonId, String)>,
    /// Which repository fields the user has left, for the required error (feature 937). Keyed by
    /// [`FieldId::DaemonRepository`], so each remote daemon's step is its own field.
    pub repository_marks: RequiredMarks<FieldId>,
    ended: Vec<SessionId>,
    /// Kubernetes daemons edited while running, until they are started again.
    applies_on_next_start: BTreeSet<DaemonId>,
    pod_logs: BTreeMap<DaemonId, Result<String, String>>,
    /// The kubeconfig contexts the Kubernetes form offers (feature 686, FR-002).
    pub contexts: ContextList,
    /// The AI CLIs a new Kubernetes daemon installs: the providers enabled in Settings (FR-018).
    pub default_providers: Vec<AiCli>,
}

impl State {
    /// Whether `section` is shown open: opened by the user, or holding the field the last refused
    /// save was about, so an error is never hidden (FR-012, FR-014).
    pub fn section_open(&self, section: Section) -> bool {
        let by_user = (section == Section::Container) != self.flipped_sections.contains(&section);
        by_user
            || self
                .form_error
                .as_ref()
                .is_some_and(|e| section.holds(e.field))
    }

    /// The repository step of remote daemon `id`: required, and empty exactly when the path is
    /// (feature 937; `RemotePathError::Empty`).
    pub fn repository_field(&self, id: DaemonId) -> (FieldId, Requirement) {
        let typed = self.repositories.get(&id).map_or("", String::as_str);
        (
            FieldId::DaemonRepository(id),
            Requirement::text(true, typed),
        )
    }

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
    Binding::resolve_for(
        &loc.project,
        stored,
        &loc.worktree,
        model.registry,
        model.legacy_default,
    )
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
                DaemonRuntime::Ssh(s) => s.target.describe(),
                DaemonRuntime::Kubernetes(k) => k.endpoint(entry.name.as_str()),
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
            activity: match (&entry.runtime, state.provisioning.get(&entry.id)) {
                (DaemonRuntime::Ssh(_), Some(progress))
                    if model.links.state(entry.id) == Some(&DaemonState::Starting) =>
                {
                    progress.text()
                }
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let form = state.draft.as_ref();
    let stored_shares = model.registry.entries().iter().any(|e| {
        let edited = form.is_some_and(|d| d.editing == Some(e.id));
        !edited
            && matches!(&e.runtime, DaemonRuntime::Container(c) if !c.profile.credentials.is_empty())
    });
    let form_shares = form
        .is_some_and(|d| d.runtime == DraftRuntime::Container && !d.profile.credentials.is_empty());
    ViewModel {
        shares_credentials: stored_shares || form_shares,
        empty: rows.is_empty(),
        bind_choices: rows.iter().map(|r| (r.id, r.name.clone())).collect(),
        // The daemon whose container the machine facts describe: the legacy default when it is a
        // container, else the first container entry (as `BootPlan::sandbox_daemon`).
        legacy_default: {
            let is_container = |e: &&micold_core::daemons::DaemonEntry| {
                matches!(e.runtime, DaemonRuntime::Container(_))
            };
            model
                .legacy_default
                .and_then(|id| model.registry.get(id))
                .filter(is_container)
                .or_else(|| model.registry.entries().iter().find(is_container))
                .map(|e| e.id)
        },
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
        applies_on_next_start: state.applies_on_next_start.contains(&id),
        pod_log: state.pod_logs.get(&id).cloned(),
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
        DaemonEvent::DialFailed(reason)
        | DaemonEvent::Lost(reason)
        | DaemonEvent::StartFailed(reason) => {
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
    if change
        .as_ref()
        .is_some_and(|c| c.to != DaemonState::Starting)
    {
        state.provisioning.remove(&id);
    }
    let diagnostics = state.diagnostics.entry(id).or_default();
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
    state.applies_on_next_start.remove(&id);
    daemon_event(state, model.links, id, DaemonEvent::Start)
}

/// Reconnect `id` now rather than at the next backoff: the attempts start again with the first
/// delay, and a daemon whose attempts were stopped is dialled again (feature 489, FR-007).
pub fn reconnect(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    if !model.registry.get(id)?.runtime.is_startable() {
        return None;
    }
    state.runtime_missing.remove(&id);
    daemon_event(state, model.links, id, DaemonEvent::Reconnect)
}

/// Stop trying to reconnect `id`: it shows unreachable until the user reconnects it.
pub fn give_up(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    daemon_event(state, model.links, id, DaemonEvent::GiveUp)
}

/// Stop `id`, and only `id`: it is neither dialled nor respawned until started.
pub fn stop(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Option<StateChange> {
    daemon_event(state, model.links, id, DaemonEvent::Stop)
}

/// Open the form for a new daemon.
pub fn begin_add(state: &mut State) {
    let profile = SandboxProfile::default();
    let texts = ProfileTexts::of(&profile);
    state.flipped_sections.clear();
    state.draft = Some(Draft {
        editing: None,
        name: String::new(),
        runtime: DraftRuntime::default(),
        container_name: String::new(),
        port: String::new(),
        profile,
        image_path: texts.image_path,
        cpus: texts.cpus,
        memory_mib: texts.memory_mib,
        pids: texts.pids,
        storage_mib: texts.storage_mib,
        auto_start: true,
        kube: KubeDraft::new(&state.default_providers),
        kube_edited: false,
        ssh_host: String::new(),
        ssh_user: String::new(),
        ssh_port: String::new(),
        ssh_key_file: String::new(),
        required: Default::default(),
    });
}

/// Open the form for `id`, filled from its entry.
pub fn begin_edit<'s>(state: &'s mut State, model: &Model<'_>, id: DaemonId) -> Option<&'s Draft> {
    let entry = model.registry.get(id)?;
    // A runtime this build does not know is kept opaque: editing it as a host would replace it.
    if !matches!(
        entry.runtime,
        DaemonRuntime::Container(_)
            | DaemonRuntime::Host
            | DaemonRuntime::Ssh(_)
            | DaemonRuntime::Kubernetes(_)
    ) {
        return None;
    }
    let mut ssh = (String::new(), String::new(), String::new(), String::new());
    if let DaemonRuntime::Ssh(s) = &entry.runtime {
        ssh = (
            s.target.host.clone(),
            s.target.user.clone().unwrap_or_default(),
            s.target.port.map(|p| p.to_string()).unwrap_or_default(),
            s.target
                .key_file
                .as_ref()
                .map(|k| k.display().to_string())
                .unwrap_or_default(),
        );
    }
    let (runtime, container_name, port, profile) = match &entry.runtime {
        DaemonRuntime::Container(c) => (
            DraftRuntime::Container,
            c.container_name.clone(),
            c.port.to_string(),
            c.profile.clone(),
        ),
        DaemonRuntime::Ssh(_) => (
            DraftRuntime::Ssh,
            String::new(),
            String::new(),
            SandboxProfile::default(),
        ),
        DaemonRuntime::Kubernetes(_) => (
            DraftRuntime::Kubernetes,
            String::new(),
            String::new(),
            SandboxProfile::default(),
        ),
        _ => (
            DraftRuntime::Host,
            String::new(),
            String::new(),
            SandboxProfile::default(),
        ),
    };
    let texts = ProfileTexts::of(&profile);
    let kube = match &entry.runtime {
        DaemonRuntime::Kubernetes(k) => KubeDraft::of(k),
        _ => KubeDraft::new(&state.default_providers),
    };
    state.flipped_sections.clear();
    state.draft = Some(Draft {
        editing: Some(id),
        kube,
        kube_edited: runtime == DraftRuntime::Kubernetes,
        name: entry.name.as_str().to_string(),
        runtime,
        container_name,
        port,
        profile,
        image_path: texts.image_path,
        cpus: texts.cpus,
        memory_mib: texts.memory_mib,
        pids: texts.pids,
        storage_mib: texts.storage_mib,
        auto_start: entry.auto_start,
        ssh_host: ssh.0,
        ssh_user: ssh.1,
        ssh_port: ssh.2,
        ssh_key_file: ssh.3,
        required: Default::default(),
    });
    state.draft.as_ref()
}

/// Close the form without saving.
pub fn discard_draft(state: &mut State) {
    state.flipped_sections.clear();
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
        RegistryError::InvalidField { field } => invalid_field(field),
        RegistryError::UnknownDaemon => FormError {
            field: "name",
            message: "That daemon no longer exists.".into(),
        },
    }
}

/// The refused field and the sentence beside it.
pub(crate) fn invalid_field(field: &'static str) -> FormError {
    let message = match field {
        "port" => "Enter a port between 1 and 65535.".to_string(),
        "image" => "The image reference is not valid.".to_string(),
        "container_name" => "Enter a container name.".to_string(),
        "host" => "Enter a host name, or a Host alias from your SSH configuration.".to_string(),
        "user" => "Enter a user name without an @.".to_string(),
        "key_file" => "That key file does not exist.".to_string(),
        "context" => "Choose a kube context.".to_string(),
        "namespace" => {
            "Enter a namespace: lowercase letters, digits and dashes, up to 63.".to_string()
        }
        "node_selector" => "Enter node selector labels as key=value.".to_string(),
        "tolerations" => "A toleration is key[=value]:Effect or key:Exists:Effect.".to_string(),
        "volume_size" => "The volume must be at least 1 GiB.".to_string(),
        "storage_class" => "Enter a storage class name.".to_string(),
        "allowed" => "Allow at most one SSH key and one registry file.".to_string(),
        "ssh_key_file" => "Enter the SSH key file to allow.".to_string(),
        "pull_file" => "Enter the registry credentials file to allow.".to_string(),
        "cpu_request" => "The CPU request must not exceed the CPU limit.".to_string(),
        "memory_request" => "The memory request must not exceed the memory limit.".to_string(),
        other => format!("{other} is not valid."),
    };
    FormError { field, message }
}

fn runtime_of(
    draft: &Draft,
    registry: &DaemonRegistry,
    taken_elsewhere: &[u16],
    next_port: &dyn Fn() -> std::io::Result<u16>,
) -> Result<DaemonRuntime, FormError> {
    match draft.runtime {
        DraftRuntime::Host => Ok(DaemonRuntime::Host),
        DraftRuntime::Kubernetes => draft.kube.settings().map(DaemonRuntime::Kubernetes),
        DraftRuntime::Ssh => {
            fn blank(v: &str) -> Option<&str> {
                Some(v.trim()).filter(|v| !v.is_empty())
            }
            let port = match blank(&draft.ssh_port) {
                Some(p) => Some(p.parse::<u16>().map_err(|_| invalid_field("port"))?),
                None => None,
            };
            let target = micold_core::ssh::SshTarget::new(
                &draft.ssh_host,
                blank(&draft.ssh_user),
                port,
                blank(&draft.ssh_key_file).map(PathBuf::from),
            )
            .map_err(|e| match e {
                RegistryError::InvalidField { field } => invalid_field(field),
                _ => invalid_field("host"),
            })?;
            Ok(DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }))
        }
        DraftRuntime::Container => {
            let port = container_port(draft, registry, taken_elsewhere, next_port)?;
            let mut profile = draft.profile.clone();
            let path = draft.image_path.trim();
            profile.image.path = (!path.is_empty()).then(|| PathBuf::from(path));
            profile.budget = parse_budget(
                &draft.cpus,
                &draft.memory_mib,
                &draft.pids,
                &draft.storage_mib,
            )
            .map_err(|(field, message)| FormError {
                field: match field {
                    LimitField::Cpus => "cpus",
                    LimitField::Memory => "memory",
                    LimitField::Pids => "pids",
                    LimitField::Storage => "storage",
                },
                message,
            })?;
            if let Some(problem) = profile.image.validate().into_iter().find(|p| {
                matches!(
                    p,
                    ImageSourceProblem::MissingArchivePath | ImageSourceProblem::UnusedArchivePath
                )
            }) {
                return Err(FormError {
                    field: "image_path",
                    message: problem.message(),
                });
            }
            Ok(DaemonRuntime::Container(
                micold_core::daemons::ContainerSettings {
                    profile,
                    container_name: draft.container_name.trim().to_string(),
                    port,
                },
            ))
        }
    }
}

/// The container port: as typed, else the stored one while editing a container daemon (FR-006),
/// else a free one picked now (FR-001).
fn container_port(
    draft: &Draft,
    registry: &DaemonRegistry,
    taken_elsewhere: &[u16],
    next_port: &dyn Fn() -> std::io::Result<u16>,
) -> Result<u16, FormError> {
    let text = draft.port.trim();
    if !text.is_empty() {
        return text
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or(FormError {
                field: "port",
                message: "Enter a port between 1 and 65535.".into(),
            });
    }
    let stored = draft
        .editing
        .and_then(|id| registry.get(id))
        .and_then(|e| match &e.runtime {
            DaemonRuntime::Container(c) => Some(c.port),
            _ => None,
        });
    if let Some(port) = stored {
        return Ok(port);
    }
    let mut taken = registry.container_ports();
    taken.extend_from_slice(taken_elsewhere);
    micold_core::daemons::pick_free_port(&taken, &mut || next_port()).map_err(|_| FormError {
        field: "port",
        message: "No free port could be found; enter one yourself.".into(),
    })
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
        confirm_label: "Continue",
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
    let runtime = runtime_of(
        &draft,
        model.registry,
        &model.taken_elsewhere,
        model.next_port,
    )?;
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
            let kube = matches!(runtime, DaemonRuntime::Kubernetes(_));
            if kube && runtime_changed && model.links.should_dial(id) {
                // The pod is not changed silently: the new settings wait for the next start.
                state.applies_on_next_start.insert(id);
            } else if runtime_changed && model.links.should_dial(id) {
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
                        runtime: Box::new(runtime),
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
        Pending::StopKube(id) => return stop_kube(state, model, id),
        Pending::RemoveKube(id) => return remove_kube(model, id),
        Pending::ForgetLocally(id) => {
            out = end_sessions(model, id);
            out.extend(forget(state, model, id));
            return out;
        }
        Pending::Remove(id) => {
            let sessions = model.sessions.get(&id).cloned().unwrap_or_default();
            model.links.apply(id, DaemonEvent::Stop);
            model.links.remove(id);
            model.registry.remove(id);
            if state.draft.as_ref().and_then(|d| d.editing) == Some(id) {
                state.draft = None;
            }
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
                .edit(id, &name, (*runtime).clone(), auto_start)
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

/// A Kubernetes daemon's name and settings.
fn kube_of(model: &Model<'_>, id: DaemonId) -> Option<(String, KubeSettings)> {
    let entry = model.registry.get(id)?;
    match &entry.runtime {
        DaemonRuntime::Kubernetes(k) => Some((entry.name.as_str().to_string(), k.clone())),
        _ => None,
    }
}

/// The names of a Kubernetes daemon's cluster objects (as `core/kube/start.rs` builds them).
fn kube_names(name: &str, settings: &KubeSettings) -> ResourceNames {
    ResourceNames::new(&settings.context.0, &settings.namespace.0, name)
}

/// A confirmation with `text`, counted as 491's prompts are.
fn kube_prompt(
    model: &Model<'_>,
    id: DaemonId,
    name: String,
    text: String,
    confirm_label: &'static str,
) -> RemovalPrompt {
    RemovalPrompt {
        daemon: id,
        name,
        worktrees_affected: bound_to(model, id).len(),
        running_sessions: model.sessions.get(&id).map_or(0, Vec::len),
        text,
        confirm_label,
    }
}

/// Stop `id`'s sessions and its link, returning the outcome that ends the sessions.
fn end_sessions(model: &mut Model<'_>, id: DaemonId) -> Vec<Outcome> {
    let sessions = model.sessions.get(&id).cloned().unwrap_or_default();
    model.links.apply(id, DaemonEvent::Stop);
    if sessions.is_empty() {
        Vec::new()
    } else {
        vec![Outcome::DaemonSessionsEnded {
            daemon: id,
            sessions,
        }]
    }
}

/// Stop of a Kubernetes daemon (FR-014): confirmed first when sessions run, else at once.
fn request_kube_stop(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Vec<Outcome> {
    let sessions = model.sessions.get(&id).map_or(0, Vec::len);
    if sessions == 0 {
        return stop_kube(state, model, id);
    }
    let Some((name, _)) = kube_of(model, id) else {
        return Vec::new();
    };
    let text = format!(
        "Stop {name}? {} will end. Worktrees on its volume are kept.",
        plural(sessions, "running session", "running sessions"),
    );
    let prompt = kube_prompt(model, id, name, text, "Stop");
    state.pending = Some((Pending::StopKube(id), prompt));
    Vec::new()
}

fn stop_kube(_state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Vec<Outcome> {
    let Some((name, settings)) = kube_of(model, id) else {
        return Vec::new();
    };
    let mut out = end_sessions(model, id);
    out.push(Outcome::KubeStopRequested {
        daemon: id,
        settings,
        name,
    });
    out
}

/// Remove of a Kubernetes daemon (FR-015): the confirmation names what is deleted; nothing is
/// deleted before it.
fn request_kube_removal(state: &mut State, model: &Model<'_>, id: DaemonId) {
    let Some((name, settings)) = kube_of(model, id) else {
        return;
    };
    let names = kube_names(&name, &settings);
    let left = leftovers(&settings.allowed, &names);
    let (wt, sessions) = (
        bound_to(model, id).len(),
        model.sessions.get(&id).map_or(0, Vec::len),
    );
    let text = format!(
        "Remove {name}? This deletes from {}/{}: pod {}, volume {} with its worktrees, secrets {}. \
         This computer's SSH key for {name} is deleted too. {} will show \"no daemon\" until bound \
         again, and {} will stop.",
        settings.context.0,
        settings.namespace.0,
        left.pod,
        left.pvc,
        left.secrets.join(", "),
        plural(wt, "worktree", "worktrees"),
        plural(sessions, "running session", "running sessions"),
    );
    let prompt = kube_prompt(model, id, name, text, "Remove");
    state.pending = Some((Pending::RemoveKube(id), prompt));
}

/// The confirmed removal: the sessions end and the shell deletes the cluster objects; the entry
/// stays until it reports.
fn remove_kube(model: &mut Model<'_>, id: DaemonId) -> Vec<Outcome> {
    let Some((name, settings)) = kube_of(model, id) else {
        return Vec::new();
    };
    let mut out = end_sessions(model, id);
    out.push(Outcome::KubeRemoveRequested {
        daemon: id,
        settings,
        name,
    });
    out
}

/// The prompt to forget `id` on this computer only, after `unreachable` (the cluster error) or on
/// request.
fn request_forget_locally(
    state: &mut State,
    model: &Model<'_>,
    id: DaemonId,
    unreachable: Option<&str>,
) {
    let Some((name, settings)) = kube_of(model, id) else {
        return;
    };
    let names = kube_names(&name, &settings);
    let left = leftovers(&settings.allowed, &names);
    let why = unreachable
        .map(|e| format!("The cluster could not be reached ({e}). "))
        .unwrap_or_default();
    let text = format!(
        "{why}Forget {name} on this computer only? These stay in {}/{}: {}, {}, {}; this \
         computer's key stays authorized in {} until they are deleted.",
        settings.context.0,
        settings.namespace.0,
        left.pod,
        left.pvc,
        left.secrets.join(", "),
        names.access_secret(),
    );
    let prompt = kube_prompt(model, id, name, text, "Forget locally");
    state.pending = Some((Pending::ForgetLocally(id), prompt));
}

/// The shell's answer to [`Outcome::KubeRemoveRequested`].
fn kube_removed(
    state: &mut State,
    model: &mut Model<'_>,
    id: DaemonId,
    result: Result<(), String>,
) -> Vec<Outcome> {
    match result {
        Ok(()) if model.registry.get(id).is_some() => forget(state, model, id),
        Ok(()) => Vec::new(),
        Err(why) => {
            request_forget_locally(state, model, id, Some(&why));
            Vec::new()
        }
    }
}

/// Forget a Kubernetes daemon here: its entry, its state, and this client's key files. Bindings
/// are left as for 491; the cluster is not touched.
fn forget(state: &mut State, model: &mut Model<'_>, id: DaemonId) -> Vec<Outcome> {
    model.links.apply(id, DaemonEvent::Stop);
    model.links.remove(id);
    model.registry.remove(id);
    if state.draft.as_ref().and_then(|d| d.editing) == Some(id) {
        state.draft = None;
    }
    state.diagnostics.remove(&id);
    state.versions.remove(&id);
    state.runtime_missing.remove(&id);
    state.applies_on_next_start.remove(&id);
    state.pod_logs.remove(&id);
    state.details_shown.remove(&id);
    vec![
        Outcome::DaemonRegistryChanged(model.registry.clone()),
        Outcome::KubeKeysDelete { daemon: id },
    ]
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

/// Everything the user can do in the Daemons section, the removal confirmation and the
/// `Bind to daemon...` picker (feature 491, M3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// Open the form for a new daemon.
    AddOpened,
    /// Open the form for this daemon.
    EditOpened(DaemonId),
    /// Save the open form.
    Saved,
    /// Close the form without saving.
    DraftDiscarded,
    /// A section header was pressed: that section opens or closes and nothing else changes.
    SectionToggled(Section),
    /// The name field changed.
    NameEdited(String),
    /// The runtime choice changed.
    RuntimeChosen(DraftRuntime),
    /// The container name field changed.
    ContainerNameEdited(String),
    /// The port field changed.
    PortEdited(String),
    /// The image reference field changed.
    ImageEdited(String),
    /// The container runtime (Docker or Podman) changed.
    ContainerRuntimeChosen(RuntimeKind),
    /// The image source changed.
    ImageKindChosen(ImageSourceKind),
    /// The image archive path field changed.
    ImagePathEdited(String),
    /// A credential share was ticked or unticked.
    CredentialToggled(CredentialShare, bool),
    /// The network posture changed.
    NetworkChosen(NetworkPosture),
    /// The processor limit field changed.
    CpuLimitEdited(String),
    /// The memory limit field changed.
    MemoryLimitEdited(String),
    /// The process-count limit field changed.
    PidLimitEdited(String),
    /// The storage limit field changed.
    StorageLimitEdited(String),
    /// The keep-running-after-sign-out box changed.
    SurviveLogoutToggled(bool),
    /// The SSH host field changed.
    SshHostEdited(String),
    /// The SSH user field changed.
    SshUserEdited(String),
    /// The SSH port field changed.
    SshPortEdited(String),
    /// The SSH key file field changed.
    SshKeyFileEdited(String),
    /// The repository path under a remote daemon was edited (feature 489).
    RepositoryEdited(DaemonId, String),
    /// Add the typed repository path of this remote daemon as a project (feature 489).
    RepositoryAdded(DaemonId),
    /// A start step began, or the copy advanced, for this daemon (feature 806, FR-018).
    Provisioning(DaemonId, ProvisionEvent),
    /// The kubeconfig's contexts were read (feature 686): the list, or why there is none.
    ContextsListed(Result<Vec<ContextEntry>, String>),
    /// A kube context was chosen.
    ContextChosen(String),
    /// The Kubernetes namespace field changed.
    NamespaceEdited(String),
    /// The Kubernetes image field changed.
    KubeImageEdited(String),
    /// An AI CLI to install was ticked or unticked.
    ProviderToggled(AiCli, bool),
    /// The Kubernetes CPU request field changed.
    CpuRequestEdited(String),
    /// The Kubernetes CPU limit field changed.
    KubeCpuLimitEdited(String),
    /// The Kubernetes memory request field changed.
    MemoryRequestEdited(String),
    /// The Kubernetes memory limit field changed.
    KubeMemoryLimitEdited(String),
    /// An allowed credential was ticked or unticked.
    AllowedToggled(AllowedKind, bool),
    /// The allowed SSH key file field changed.
    KubeSshKeyEdited(String),
    /// The allowed registry auth file field changed.
    PullFileEdited(String),
    /// The Kubernetes node selector text changed (one `key=value` row per line).
    NodeSelectorEdited(String),
    /// The Kubernetes tolerations text changed (one row per line).
    TolerationsEdited(String),
    /// The node selector editor was acted on (typing, cursor, selection).
    NodeSelectorAction(crate::app::EditorAction),
    /// The tolerations editor was acted on.
    TolerationsAction(crate::app::EditorAction),
    /// The Kubernetes volume size text changed (`20Gi`).
    VolumeSizeEdited(String),
    /// The Kubernetes storage class field changed.
    StorageClassEdited(String),
    /// The connect-at-launch box changed.
    AutoStartToggled(bool),
    /// Start this daemon, and only it.
    Started(DaemonId),
    /// Stop this daemon, and only it.
    Stopped(DaemonId),
    /// Reconnect this daemon now.
    Reconnected(DaemonId),
    /// The user stopped the reconnect attempts of a daemon.
    GaveUp(DaemonId),
    /// Ask to remove this daemon: opens the counted confirmation.
    RemovalRequested(DaemonId),
    /// Confirm the removal (or the counted edit, a Kubernetes stop, or forgetting locally).
    RemovalConfirmed,
    /// The shell finished deleting a Kubernetes daemon's cluster objects (`Ok`), or could not
    /// (`Err` with why); the daemon is forgotten only on `Ok`, else the forget-locally prompt opens.
    KubeRemoved(DaemonId, Result<(), String>),
    /// Ask to forget this Kubernetes daemon on this computer only: opens its confirmation.
    ForgetLocallyRequested(DaemonId),
    /// Show the last lines of this Kubernetes daemon's pod log.
    LogsRequested(DaemonId),
    /// The pod log the shell fetched, or why it could not.
    LogsFetched(DaemonId, Result<String, String>),
    /// Decline it: nothing changes.
    RemovalCancelled,
    /// Show or hide this daemon's errors and log tail.
    DetailsToggled(DaemonId),
    /// What one daemon's connection reported (its state moves, and what it said is kept for the
    /// details). Sent by the shell as the connection actor's messages arrive.
    EventReported(DaemonId, DaemonEvent),
    /// The version a connected daemon reported.
    VersionReported(DaemonId, String),
    /// A line of a daemon's log.
    LogLine(DaemonId, String),
    /// The container runtime of this daemon cannot be used, and why.
    RuntimeMissing(DaemonId, String),
    /// Open the `Bind to daemon...` picker for a location.
    BindRequested(WorktreeRef),
    /// The picker's choice.
    BindChosen(DaemonId),
    /// Close the picker without binding.
    BindCancelled,
    /// Bind a location to a daemon, with no picker.
    BindWorktree {
        /// The location.
        location: WorktreeRef,
        /// The daemon.
        daemon: DaemonId,
    },
}

/// The repository step of a remote daemon (feature 489, FR-012): the typed path, made the identity
/// of a project bound to this daemon alone. Refused, with the sentence under the field, for a path
/// that is not absolute and for a daemon that is not connected: the open-project gate is asked of
/// the host, so nothing is guessed while it cannot answer.
pub fn add_repository(state: &mut State, model: &Model<'_>, id: DaemonId) -> Vec<Outcome> {
    let typed = state.repositories.get(&id).cloned().unwrap_or_default();
    let located = micold_core::ssh::RemoteLocation::new(id, &typed);
    let outcome = match located {
        Err(e) => Err(e.to_string()),
        Ok(_) if !model.links.is_connected(id) => {
            let name = model
                .registry
                .get(id)
                .map(|e| e.name.as_str().to_string())
                .unwrap_or_default();
            Err(format!("Daemon {name} is not connected yet."))
        }
        Ok(location) => Ok(location),
    };
    match outcome {
        Ok(location) => {
            state.repository_error = None;
            state.repositories.remove(&id);
            vec![Outcome::RemoteRepositoryRequested {
                path: location.identity(),
            }]
        }
        Err(message) => {
            state.repository_error = Some((id, message));
            Vec::new()
        }
    }
}

/// Apply one message. The registry, the links and the bindings change through `model`; what the
/// shell must persist or stop comes back as [`Outcome`]s.
pub fn update(state: &mut State, model: &mut Model<'_>, msg: Msg) -> Vec<Outcome> {
    let edit = |state: &mut State, f: &dyn Fn(&mut Draft)| {
        if let Some(draft) = state.draft.as_mut() {
            f(draft);
        }
        state.form_error = None;
    };
    match msg {
        Msg::AddOpened => {
            begin_add(state);
            state.form_error = None;
        }
        Msg::EditOpened(id) => {
            state.form_error = None;
            let kube = begin_edit(state, model, id).is_some_and(|d| d.kube_edited);
            if kube {
                return vec![Outcome::KubeContextsRequested];
            }
        }
        Msg::Saved => match save(state, model) {
            Ok(out) => {
                state.form_error = None;
                return out;
            }
            Err(error) => state.form_error = Some(error),
        },
        Msg::DraftDiscarded => {
            discard_draft(state);
            state.form_error = None;
        }
        Msg::SectionToggled(section) => {
            if !state.flipped_sections.remove(&section) {
                state.flipped_sections.insert(section);
            }
        }
        Msg::NameEdited(v) => edit(state, &|d| d.name = v.clone()),
        Msg::RuntimeChosen(v) => {
            edit(state, &|d| d.runtime = v);
            if v == DraftRuntime::Kubernetes {
                return vec![Outcome::KubeContextsRequested];
            }
        }
        Msg::ContextsListed(Ok(list)) => state.contexts = ContextList::Found(list),
        Msg::ContextsListed(Err(why)) => state.contexts = ContextList::Failed(why),
        Msg::ContextChosen(name) => {
            let contexts = state.contexts.clone();
            edit(state, &|d| d.kube.choose_context(&name, &contexts));
        }
        Msg::NamespaceEdited(v) => edit(state, &|d| d.kube.namespace = v.clone()),
        Msg::KubeImageEdited(v) => edit(state, &|d| d.kube.image = v.clone()),
        Msg::ProviderToggled(cli, on) => edit(state, &|d| {
            if on {
                d.kube.providers.insert(cli);
            } else {
                d.kube.providers.remove(&cli);
            }
        }),
        Msg::CpuRequestEdited(v) => edit(state, &|d| d.kube.cpu_request = v.clone()),
        Msg::KubeCpuLimitEdited(v) => edit(state, &|d| d.kube.cpu_limit = v.clone()),
        Msg::MemoryRequestEdited(v) => edit(state, &|d| d.kube.memory_request = v.clone()),
        Msg::KubeMemoryLimitEdited(v) => edit(state, &|d| d.kube.memory_limit = v.clone()),
        Msg::AllowedToggled(kind, on) => edit(state, &|d| {
            if on {
                d.kube.allowed.insert(kind);
            } else {
                d.kube.allowed.remove(&kind);
            }
        }),
        Msg::KubeSshKeyEdited(v) => edit(state, &|d| d.kube.ssh_key_file = v.clone()),
        Msg::PullFileEdited(v) => edit(state, &|d| d.kube.pull_file = v.clone()),
        Msg::ContainerNameEdited(v) => edit(state, &|d| d.container_name = v.clone()),
        Msg::PortEdited(v) => edit(state, &|d| d.port = v.clone()),
        Msg::ImageEdited(v) => edit(state, &|d| d.profile.image.reference = v.clone()),
        Msg::ContainerRuntimeChosen(v) => edit(state, &|d| d.profile.runtime = v),
        Msg::ImageKindChosen(v) => edit(state, &|d| d.profile.image.kind = v),
        Msg::ImagePathEdited(v) => edit(state, &|d| d.image_path = v.clone()),
        Msg::CredentialToggled(share, on) => edit(state, &|d| {
            if on {
                d.profile.credentials.insert(share);
            } else {
                d.profile.credentials.remove(&share);
            }
        }),
        Msg::NetworkChosen(v) => edit(state, &|d| d.profile.network = v),
        Msg::CpuLimitEdited(v) => edit(state, &|d| d.cpus = v.clone()),
        Msg::MemoryLimitEdited(v) => edit(state, &|d| d.memory_mib = v.clone()),
        Msg::PidLimitEdited(v) => edit(state, &|d| d.pids = v.clone()),
        Msg::StorageLimitEdited(v) => edit(state, &|d| d.storage_mib = v.clone()),
        Msg::SurviveLogoutToggled(v) => edit(state, &|d| d.profile.survive_logout = v),
        Msg::SshHostEdited(v) => edit(state, &|d| d.ssh_host = v.clone()),
        Msg::SshUserEdited(v) => edit(state, &|d| d.ssh_user = v.clone()),
        Msg::SshPortEdited(v) => edit(state, &|d| d.ssh_port = v.clone()),
        Msg::SshKeyFileEdited(v) => edit(state, &|d| d.ssh_key_file = v.clone()),
        Msg::RepositoryEdited(id, v) => {
            state.repositories.insert(id, v);
            state.repository_error = None;
        }
        Msg::RepositoryAdded(id) => return add_repository(state, model, id),
        // A late event of a start that already ended must not outlive it.
        Msg::Provisioning(id, event) => {
            if model.links.state(id) == Some(&DaemonState::Starting) {
                state.provisioning.entry(id).or_default().apply(event);
            }
        }
        Msg::NodeSelectorEdited(v) => edit(state, &|d| d.kube.set_node_selector(v.clone())),
        Msg::TolerationsEdited(v) => edit(state, &|d| d.kube.set_tolerations(v.clone())),
        Msg::NodeSelectorAction(action) => edit(state, &|d| {
            d.kube.node_selector = d.kube.node_selector_editor.perform(action.0.clone());
        }),
        Msg::TolerationsAction(action) => edit(state, &|d| {
            d.kube.tolerations = d.kube.tolerations_editor.perform(action.0.clone());
        }),
        Msg::VolumeSizeEdited(v) => edit(state, &|d| d.kube.volume_size = v.clone()),
        Msg::StorageClassEdited(v) => edit(state, &|d| d.kube.storage_class = v.clone()),
        Msg::AutoStartToggled(v) => edit(state, &|d| d.auto_start = v),
        Msg::Started(id) => {
            start(state, model, id);
        }
        Msg::Stopped(id) => {
            if kube_of(model, id).is_some() {
                return request_kube_stop(state, model, id);
            }
            stop(state, model, id);
        }
        Msg::Reconnected(id) => {
            reconnect(state, model, id);
        }
        Msg::GaveUp(id) => {
            give_up(state, model, id);
        }
        Msg::RemovalRequested(id) => {
            if kube_of(model, id).is_some() {
                request_kube_removal(state, model, id);
            } else {
                request_removal(state, model, id);
            }
        }
        Msg::ForgetLocallyRequested(id) => request_forget_locally(state, model, id, None),
        Msg::KubeRemoved(id, result) => return kube_removed(state, model, id, result),
        Msg::LogsRequested(id) => {
            if let Some((_, settings)) = kube_of(model, id) {
                return vec![Outcome::KubeLogsRequested {
                    daemon: id,
                    settings,
                    tail: LOG_TAIL_LINES,
                }];
            }
        }
        Msg::LogsFetched(id, result) => {
            state.pod_logs.insert(id, result);
        }
        Msg::RemovalConfirmed => {
            let out = confirm(state, model);
            state
                .details_shown
                .retain(|id| model.registry.get(*id).is_some());
            return out;
        }
        Msg::RemovalCancelled => cancel(state),
        Msg::DetailsToggled(id) => {
            if !state.details_shown.remove(&id) {
                state.details_shown.insert(id);
            }
        }
        Msg::EventReported(id, event) => {
            daemon_event(state, model.links, id, event);
        }
        Msg::VersionReported(id, version) => version_reported(state, id, &version),
        Msg::LogLine(id, line) => log_line(state, id, line),
        Msg::RuntimeMissing(id, reason) => runtime_missing(state, id, &reason),
        Msg::BindRequested(location) => state.bind_target = Some(location),
        Msg::BindChosen(daemon) => {
            if let Some(location) = state.bind_target.take() {
                return bind_worktree(model, &location, daemon).unwrap_or_default();
            }
        }
        Msg::BindCancelled => state.bind_target = None,
        Msg::BindWorktree { location, daemon } => {
            return bind_worktree(model, &location, daemon).unwrap_or_default();
        }
    }
    Vec::new()
}

/// The removal confirmation, as a floating surface (feature 491, T035). Declining is what Escape
/// and the scrim do, so a removal is never half applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmDaemonRemovalDialog;

floating_surface!(ConfirmDaemonRemovalDialog, id: SurfaceId::new("confirm_daemon_removal"), layer: Dialog, cancelled_by: crate::app::Message::Daemons(Msg::RemovalCancelled));

impl Registered for ConfirmDaemonRemovalDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .daemon_list
            .prompt()
            .map(|_| ConfirmDaemonRemovalDialog)
    }
}

/// The `Bind to daemon...` picker, as a floating surface (feature 491, T036).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindDaemonDialog;

floating_surface!(BindDaemonDialog, id: SurfaceId::new("bind_daemon"), layer: Dialog, cancelled_by: crate::app::Message::Daemons(Msg::BindCancelled));

impl Registered for BindDaemonDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .daemon_list
            .bind_target
            .as_ref()
            .map(|_| BindDaemonDialog)
    }
}

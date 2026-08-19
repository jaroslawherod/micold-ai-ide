//! The sandbox placement: running the session daemon inside a container (feature 027).
//!
//! Everything that *decides* anything about the sandbox lives here, in the render-free core, so it
//! is unit-testable without a container runtime installed (Constitution Principle I). The split
//! mirrors `git.rs`'s treatment of git porcelain (research R7):
//!
//! | Module | Purity | Responsibility |
//! |---|---|---|
//! | [`argv`] | **pure** | spec + capabilities → the runtime's argument vector |
//! | [`parse`] | **pure** | `--format '{{json .}}'` output → typed facts |
//! | [`dialect`] | **pure** | the per-runtime differences: flag names, defaults, quirks |
//! | [`exec`] | impure | the single process-spawn shim everything above is composed over |
//!
//! Only [`exec`] touches the world. That is what lets the whole adapter layer be exercised on
//! Linux, macOS and Windows against an injected fake runner, with nothing installed (Principle VI)
//! — see `specs/027-sandboxed-daemon-runtime/contracts/container-runtime.md`.
//!
//! This module itself holds the user's *intent*: the profile, its budget, its network posture and
//! its credential opt-ins. Intent is deliberately separate from what a given runtime can actually
//! enforce ([`runtime::RuntimeCapabilities`]), because the two fail differently — an out-of-range
//! budget is the user's mistake, an unenforceable one is the environment's limitation.

pub mod argv;
pub mod dialect;
pub mod exec;
pub mod image;
pub mod parse;
pub mod pathmap;
pub mod placement;
pub mod runtime;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use image::ImageSource;
use runtime::RuntimeKind;

/// Processor share, in thousandths of a core. `1000` is one core.
///
/// A newtype rather than a bare integer so a megabyte can never be passed where a millicpu is
/// expected — Principle V's "a type-level fact, not a runtime string comparison".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilliCpus(pub u32);

/// A byte count, for the memory and writable-storage limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bytes(pub u64);

impl Bytes {
    /// Mebibytes as bytes, for readable constants and tests.
    pub const fn from_mib(mib: u64) -> Self {
        Bytes(mib * 1024 * 1024)
    }

    /// This count in whole mebibytes, rounded down — the unit both runtimes' flags accept.
    pub fn as_mib(self) -> u64 {
        self.0 / (1024 * 1024)
    }
}

/// Below this the daemon cannot function: it has to hold the session catalogue, the scrollback of
/// every open session, and whatever the AI CLI needs to load at all (FR-016).
pub const MIN_MEMORY: Bytes = Bytes::from_mib(512);
/// Below this the daemon is starved rather than limited — a session cannot make progress.
pub const MIN_MILLI_CPUS: MilliCpus = MilliCpus(250);
/// Below this a shell plus the AI CLI plus one build cannot coexist.
pub const MIN_PIDS: u32 = 64;
/// Below this the daemon's own state does not fit, before a session writes anything.
pub const MIN_STORAGE: Bytes = Bytes::from_mib(1024);

/// The default budget: generous enough that a user who never opens the section is not surprised by
/// a limit, bounded enough that a runaway session cannot take the desktop down (FR-013).
pub const DEFAULT_MILLI_CPUS: MilliCpus = MilliCpus(2000);
/// See [`DEFAULT_MILLI_CPUS`].
pub const DEFAULT_MEMORY: Bytes = Bytes::from_mib(4096);
/// See [`DEFAULT_MILLI_CPUS`].
pub const DEFAULT_PIDS: u32 = 512;

/// What the sandbox may consume (FR-012 … FR-016).
///
/// Every limit is [`Option`] because *unset* and *set to the maximum* are different user intents
/// and must round-trip differently: `None` means "the runtime's own default", which the view
/// renders as *unlimited* rather than as a blank field (rule RB-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// Processor share. `None` leaves the runtime's default.
    #[serde(default = "default_cpus")]
    pub cpus_milli: Option<MilliCpus>,
    /// Memory ceiling. `None` leaves the runtime's default.
    #[serde(default = "default_memory")]
    pub memory_bytes: Option<Bytes>,
    /// Process-count ceiling. `None` leaves the runtime's default.
    #[serde(default = "default_pids")]
    pub pids: Option<u32>,
    /// Writable-storage ceiling. `None` by default because most runtime/driver combinations
    /// cannot enforce it at all (research R5) — the capability probe decides, not this field.
    #[serde(default)]
    pub storage_bytes: Option<Bytes>,
}

fn default_cpus() -> Option<MilliCpus> {
    Some(DEFAULT_MILLI_CPUS)
}
fn default_memory() -> Option<Bytes> {
    Some(DEFAULT_MEMORY)
}
fn default_pids() -> Option<u32> {
    Some(DEFAULT_PIDS)
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            cpus_milli: default_cpus(),
            memory_bytes: default_memory(),
            pids: default_pids(),
            storage_bytes: None,
        }
    }
}

/// A limit that was set below what the daemon needs to run, with the range that would be accepted.
///
/// Carried as a value rather than a formatted string so the message is testable and so the view can
/// point at the field that caused it (US4 scenario 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetViolation {
    /// The setting's user-facing name, e.g. `"memory"`.
    pub field: &'static str,
    /// What the user asked for, in the field's own unit.
    pub requested: u64,
    /// The smallest value that would be accepted, in the same unit.
    pub minimum: u64,
    /// The unit both numbers are in, e.g. `"MiB"`.
    pub unit: &'static str,
}

impl BudgetViolation {
    /// The message the view shows: it names the accepted range, which is what FR-016 requires.
    pub fn message(&self) -> String {
        format!(
            "{} is set to {} {unit}, below the {} {unit} the daemon needs to run",
            self.field,
            self.requested,
            self.minimum,
            unit = self.unit
        )
    }
}

impl ResourceBudget {
    /// Clamp every set limit up to its documented minimum, reporting what moved.
    ///
    /// Used on **read**, following `settings.rs`'s existing `clamp_scrollback` /
    /// `clamp_env_include_timeout` idiom (rule S-7): a hand-edited file opens the app with a
    /// corrected value and a note, not an error dialog.
    pub fn clamp(&mut self) -> Vec<BudgetViolation> {
        let mut moved = self.violations();
        for v in &mut moved {
            match v.field {
                "processor" => self.cpus_milli = Some(MIN_MILLI_CPUS),
                "memory" => self.memory_bytes = Some(MIN_MEMORY),
                "processes" => self.pids = Some(MIN_PIDS),
                "storage" => self.storage_bytes = Some(MIN_STORAGE),
                other => unreachable!("unknown budget field {other}"),
            }
        }
        moved
    }

    /// Every limit set below its documented minimum.
    ///
    /// Used on **save**, where the answer is a refusal naming the accepted range rather than a
    /// silent correction (US4 scenario 5).
    pub fn violations(&self) -> Vec<BudgetViolation> {
        let mut out = Vec::new();
        if let Some(c) = self.cpus_milli.filter(|c| *c < MIN_MILLI_CPUS) {
            out.push(BudgetViolation {
                field: "processor",
                requested: u64::from(c.0),
                minimum: u64::from(MIN_MILLI_CPUS.0),
                unit: "millicpus",
            });
        }
        if let Some(m) = self.memory_bytes.filter(|m| *m < MIN_MEMORY) {
            out.push(BudgetViolation {
                field: "memory",
                requested: m.as_mib(),
                minimum: MIN_MEMORY.as_mib(),
                unit: "MiB",
            });
        }
        if let Some(p) = self.pids.filter(|p| *p < MIN_PIDS) {
            out.push(BudgetViolation {
                field: "processes",
                requested: u64::from(p),
                minimum: u64::from(MIN_PIDS),
                unit: "processes",
            });
        }
        if let Some(s) = self.storage_bytes.filter(|s| *s < MIN_STORAGE) {
            out.push(BudgetViolation {
                field: "storage",
                requested: s.as_mib(),
                minimum: MIN_STORAGE.as_mib(),
                unit: "MiB",
            });
        }
        out
    }
}

/// Whether the sandbox may open outbound connections (FR-017, FR-018).
///
/// Note what [`NetworkPosture::NoOutbound`] does **not** claim. It is implemented as a user-defined
/// bridge with IP masquerade disabled (research R4), which blocks outbound connections while
/// leaving the published control port working. DNS *lookups* still resolve, because the runtime's
/// embedded resolver forwards them from the host side — names resolve, connections to them do not.
/// That is a small metadata channel, and it is documented rather than claimed away: the posture is
/// "no outbound connections", not "no outbound traffic of any kind".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPosture {
    /// The default. Outbound connections fail; the control channel is unaffected.
    #[default]
    NoOutbound,
    /// Full egress, for sessions that need to fetch dependencies — and for the AI CLI to reach its
    /// provider at all, which is why changing away from this warns at the point of the change.
    Outbound,
}

/// One host credential the user has explicitly chosen to share with the sandbox (FR-004a/b).
///
/// An enumeration, never a free-text path list: a user cannot mount an arbitrary host directory by
/// typing it into a credentials field (rule N-1). Each variant names one thing, so the view can
/// render exactly what is shared while it is shared (FR-004c).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialShare {
    /// `~/.gitconfig` — the user's commit identity, read-only.
    GitConfig,
    /// The authentication agent's socket. The socket, never the keys themselves.
    SshAgent,
    /// The git credential helper's store.
    GitCredentials,
    /// The AI CLI's own authentication material.
    AiCliAuth,
}

impl CredentialShare {
    /// Every share, for rendering the opt-in list.
    pub const ALL: [CredentialShare; 4] = [
        CredentialShare::GitConfig,
        CredentialShare::SshAgent,
        CredentialShare::GitCredentials,
        CredentialShare::AiCliAuth,
    ];

    /// The user-facing label.
    pub fn label(self) -> &'static str {
        match self {
            CredentialShare::GitConfig => "Git configuration",
            CredentialShare::SshAgent => "SSH agent",
            CredentialShare::GitCredentials => "Git credentials",
            CredentialShare::AiCliAuth => "AI CLI sign-in",
        }
    }
}

/// The user's sandbox configuration (FR-005 … FR-019).
///
/// A profile is *valid in isolation* (ranges, a well-formed image reference) and separately
/// *satisfiable against a runtime* ([`runtime::reconcile`]). The two are different questions with
/// different answers, and conflating them is how a user ends up being told their settings are
/// wrong when in fact their Docker installation cannot do what they asked (rule SP-3).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SandboxProfile {
    /// Which container runtime drives the sandbox. Docker at release (FR-021).
    #[serde(default)]
    pub runtime: RuntimeKind,
    /// Where the image comes from.
    #[serde(default)]
    pub image: ImageSource,
    /// What the sandbox may consume.
    #[serde(default)]
    pub budget: ResourceBudget,
    /// Whether the sandbox may open outbound connections.
    #[serde(default)]
    pub network: NetworkPosture,
    /// Host credentials the user has explicitly shared. **Empty by default** (rule SP-1): a user
    /// who never opened this section shares nothing, and an upgrade never opts them in.
    #[serde(default)]
    pub credentials: BTreeSet<CredentialShare>,
    /// Mirrors the existing session-survival opt-in. In the sandboxed placement it is honoured on
    /// all three platforms via the runtime's restart policy, where the host-process mechanism
    /// manages it only on Linux (FR-014a/b, research R6).
    #[serde(default)]
    pub survive_logout: bool,
}

/// One project directory shared with the sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMount {
    /// Where it lives on the host.
    pub host: PathBuf,
    /// Where it appears inside the container. Equal to [`Self::host`] on Linux and macOS
    /// (research R2); mapped on Windows, see [`pathmap`].
    pub container: PathBuf,
    /// Whether the session may write to it. Projects are writable; that is the point.
    pub writable: bool,
}

impl ProjectMount {
    /// A writable mount for a registered project, at this platform's container path.
    pub fn project(host: impl Into<PathBuf>) -> Self {
        let host = host.into();
        let container = pathmap::map(&host);
        Self {
            host,
            container,
            writable: true,
        }
    }
}

/// The daemon's state, in a runtime-managed named volume.
///
/// A volume rather than a bind mount (rule M-3, FR-011) so recreating the container keeps
/// `projects.json`, per-project state and logs — which is what makes "the sandbox was recreated"
/// a non-event rather than a data loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedVolume {
    pub name: String,
    pub container: PathBuf,
}

/// The client's authentication token, mounted read-only (research R1).
///
/// The token reaches the container through the filesystem rather than through the command line, so
/// it does not appear in `inspect` output or in the host's process list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMount {
    /// The `0600` file in the per-user state directory.
    pub host: PathBuf,
    /// Where the daemon reads it from.
    pub container: PathBuf,
}

/// Everything the sandbox can see (FR-006 … FR-011).
///
/// The load-bearing rule of this feature is rule M-1: **only** what is listed here is mounted. The
/// user's home, the runtime's own control socket, and anything not registered are absent. A
/// sandbox's guarantee is what it cannot reach, so a convenience mount added here is not a
/// convenience — it is the feature failing quietly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSet {
    /// One per registered project.
    pub projects: Vec<ProjectMount>,
    /// Daemon state; survives container recreation.
    pub state: NamedVolume,
    /// The authentication token.
    pub secret: SecretMount,
    /// Credential mounts, one per active opt-in. Empty unless the user opted in (rule N-1).
    pub credentials: Vec<CredentialMount>,
}

/// A host credential path shared because the user opted into it.
///
/// Derived from a [`CredentialShare`] and the host's own layout — never from user-entered text,
/// which is what stops the credentials section becoming an arbitrary-path mounter (rule N-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialMount {
    /// Which opt-in produced this mount, so the view can name what is shared (FR-004c).
    pub share: CredentialShare,
    pub host: PathBuf,
    pub container: PathBuf,
}

impl MountSet {
    /// Every host path this sandbox can reach. Used by the denylist assertion, which checks that
    /// generated argv mounts nothing outside this set (obligation C-3, conformance check K-4).
    pub fn host_paths(&self) -> Vec<&Path> {
        self.projects
            .iter()
            .map(|m| m.host.as_path())
            .chain(std::iter::once(self.secret.host.as_path()))
            .chain(self.credentials.iter().map(|c| c.host.as_path()))
            .collect()
    }
}

/// A profile that has been validated and joined with everything the runtime needs to start it.
///
/// Implementations of [`runtime::ContainerRuntime`] never re-validate and never silently drop a
/// field: if a spec reaches `create`, every part of it is enforceable. That is the invariant which
/// lets `argv` be a pure total function rather than one that reports problems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    /// The container's name, so a leftover from a previous run is recognisable (US6 scenario 5).
    pub name: String,
    /// The user's configuration.
    pub profile: SandboxProfile,
    /// What the sandbox can see.
    pub mounts: MountSet,
    /// The host uid/gid the container process runs as (research R3). Read at start time rather
    /// than baked into the image, so one image serves every user.
    pub uid: u32,
    /// See [`Self::uid`].
    pub gid: u32,
    /// The loopback port the daemon's control channel is published on.
    pub control_port: u16,
    /// Extra ports the user asked to expose (US2 scenario 8).
    pub published_ports: Vec<u16>,
    /// The user-defined network the sandbox joins. With [`NetworkPosture::NoOutbound`] this network
    /// is created with IP masquerade disabled (research R4).
    pub network_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_profile_shares_no_credentials() {
        // Rule SP-1. The one default in this feature that is a security property rather than a
        // convenience: upgrading the app must never opt a user into sharing anything.
        assert!(SandboxProfile::default().credentials.is_empty());
    }

    #[test]
    fn the_default_network_posture_blocks_outbound_connections() {
        assert_eq!(NetworkPosture::default(), NetworkPosture::NoOutbound);
    }

    #[test]
    fn unset_and_maximum_are_different_intents() {
        // Rule RB-2: `None` is "the runtime's default", not "zero" and not "the largest value".
        let unset = ResourceBudget {
            memory_bytes: None,
            ..ResourceBudget::default()
        };
        let huge = ResourceBudget {
            memory_bytes: Some(Bytes(u64::MAX)),
            ..ResourceBudget::default()
        };
        assert_ne!(unset, huge);
        // And neither is a violation — an unset limit cannot be below a minimum.
        assert!(unset.violations().is_empty());
        assert!(huge.violations().is_empty());
    }

    #[test]
    fn a_limit_below_the_workable_minimum_is_reported_with_its_range() {
        let budget = ResourceBudget {
            memory_bytes: Some(Bytes::from_mib(64)),
            ..ResourceBudget::default()
        };
        let violations = budget.violations();
        assert_eq!(violations.len(), 1);
        let v = &violations[0];
        assert_eq!(v.field, "memory");
        assert_eq!(v.requested, 64);
        assert_eq!(v.minimum, MIN_MEMORY.as_mib());
        // FR-016: the message names the accepted range, so the user knows what to type instead.
        assert!(v.message().contains("512"), "message was {:?}", v.message());
    }

    #[test]
    fn clamping_moves_the_value_and_reports_the_move() {
        let mut budget = ResourceBudget {
            memory_bytes: Some(Bytes::from_mib(64)),
            pids: Some(1),
            ..ResourceBudget::default()
        };
        let moved = budget.clamp();
        assert_eq!(moved.len(), 2);
        assert_eq!(budget.memory_bytes, Some(MIN_MEMORY));
        assert_eq!(budget.pids, Some(MIN_PIDS));
        // Clamping is idempotent: a corrected budget has nothing left to correct.
        assert!(budget.clamp().is_empty());
    }

    #[test]
    fn every_credential_share_has_a_label_and_a_stable_serialisation() {
        for share in CredentialShare::ALL {
            assert!(!share.label().is_empty());
            let json = serde_json::to_string(&share).unwrap();
            let back: CredentialShare = serde_json::from_str(&json).unwrap();
            assert_eq!(share, back);
        }
    }
}

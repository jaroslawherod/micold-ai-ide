//! The [`ContainerRuntime`] trait, its capability probe, and the closed error enumeration.
//!
//! This is the seam FR-020 requires: everything above it describes *what the sandbox should be*,
//! everything below it knows how one particular runtime spells that. Docker ships at release
//! (FR-021); podman is written alongside it, because an abstraction with one implementation is a
//! guess.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::exec::CommandOutput;
use super::image::ImageSource;
use super::parse::{ContainerFacts, ImageFacts};
use super::{ResourceBudget, SandboxProfile, SandboxSpec};

/// Which container runtime drives the sandbox (FR-020, FR-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// The runtime supported at release (FR-021), and the default.
    #[default]
    Docker,
    /// Rootless by default, and the reason the abstraction is not a Docker shim.
    Podman,
}

impl RuntimeKind {
    /// Every runtime, for rendering the selector.
    pub const ALL: [RuntimeKind; 2] = [RuntimeKind::Docker, RuntimeKind::Podman];

    /// The executable this runtime is driven through.
    pub fn program(self) -> &'static str {
        match self {
            RuntimeKind::Docker => "docker",
            RuntimeKind::Podman => "podman",
        }
    }

    /// The user-facing name.
    pub fn label(self) -> &'static str {
        match self {
            RuntimeKind::Docker => "Docker",
            RuntimeKind::Podman => "Podman",
        }
    }
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A runtime's version, as reported by the runtime itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVersion {
    pub kind: RuntimeKind,
    /// The server version string. Also the cache key for [`RuntimeCapabilities`]: the probe
    /// re-runs exactly when the runtime changes underneath us (research R10).
    pub version: String,
}

/// How a runtime maps the host user's identity into the container (research R3).
///
/// Anything else leaves root-owned files in the user's project after a session writes, which the
/// user then cannot edit without elevation — a worse outcome than not sandboxing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMapping {
    /// Docker: `--user <uid>:<gid>`, read from the host at start time rather than baked in.
    ExplicitUidGid,
    /// Podman rootless: `--userns=keep-id` maps the invoking user to the same uid inside.
    KeepId,
}

/// Whether a runtime can enforce one particular limit here.
///
/// The unsupported case carries its reason because the view shows it: a limit the runtime cannot
/// enforce is rendered **disabled with the reason**, not hidden and not silently accepted
/// (FR-015, SC-009, obligation C-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitSupport {
    Supported,
    Unsupported { reason: String },
}

impl LimitSupport {
    /// Convenience for the common construction.
    pub fn unsupported(reason: impl Into<String>) -> Self {
        LimitSupport::Unsupported {
            reason: reason.into(),
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, LimitSupport::Supported)
    }
}

/// What a runtime can actually do, here, on this machine (research R10).
///
/// Probed rather than tabulated. A static table of runtime versions and their behaviours goes stale
/// the first time either project ships a release, and it lies confidently in the meantime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub kind: RuntimeKind,
    /// The version this probe describes. Re-probe when it changes.
    pub version: String,
    pub cpus: LimitSupport,
    pub memory: LimitSupport,
    pub pids: LimitSupport,
    /// The one research R5 showed is not portable: `--storage-opt size=` works on some
    /// driver/filesystem combinations and is rejected outright on others.
    pub storage: LimitSupport,
    pub identity_mapping: IdentityMapping,
}

/// A limit the user asked for that the selected runtime cannot enforce.
///
/// Reported so the view can show *why*, and so the argv builder can omit the flag — the same fact
/// consumed twice, which is what stops the UI drifting from the behaviour (rule RC-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsatisfiableLimit {
    /// The setting's user-facing name, e.g. `"storage"`.
    pub field: &'static str,
    /// Why this runtime cannot enforce it.
    pub reason: String,
}

/// The probe's answer, kept for exactly as long as it is still true (rule RC-1).
///
/// # Why this is not just a `once_cell`
///
/// [`ContainerRuntime::probe`] costs a `docker info`, and it is consulted by the settings view, the
/// argv builder and the reconciliation report — one of which is a view function that runs on every
/// frame. So it has to be cached. But a capability set cached forever is worse than none: the user
/// upgrades Docker, the limit that was unavailable becomes available, and the application goes on
/// refusing to let them set it until they restart it. The runtime's *version* is what can
/// invalidate the answer, so it is what the answer is kept against.
///
/// [`ContainerRuntime::detect`] runs on every call and [`ContainerRuntime::probe`] only when the
/// version it reports has moved. That is the trade this type exists to make: the cheap command
/// every time, so the expensive one almost never.
#[derive(Debug, Default, Clone)]
pub struct CapabilityCache {
    cached: Option<RuntimeCapabilities>,
}

impl CapabilityCache {
    /// An empty cache. The first call probes.
    pub fn new() -> Self {
        Self::default()
    }

    /// What `runtime` can enforce, probing only if the cache does not already describe the version
    /// it currently reports.
    ///
    /// A failure is reported rather than papered over with the stale answer: a runtime that has
    /// stopped responding is a fact the caller needs (US6). The previous answer is kept, so the
    /// next successful call does not have to pay for the probe again unless the version moved.
    pub fn capabilities(
        &mut self,
        runtime: &dyn ContainerRuntime,
    ) -> Result<RuntimeCapabilities, RuntimeError> {
        let now = runtime.detect()?;
        if let Some(cached) = &self.cached {
            if cached.kind == now.kind && cached.version == now.version {
                return Ok(cached.clone());
            }
        }
        let probed = runtime.probe()?;
        self.cached = Some(probed.clone());
        Ok(probed)
    }

    /// What was last probed, without consulting the runtime at all.
    ///
    /// For a caller that already holds the answer and only needs to draw with it — a view redrawing
    /// mid-frame must not spawn a process, however cheap.
    pub fn cached(&self) -> Option<&RuntimeCapabilities> {
        self.cached.as_ref()
    }
}

/// Every limit the profile sets that `caps` cannot enforce.
///
/// Pure and total, and it never mutates the profile: the user's stored intent survives a move to a
/// runtime that cannot honour it, and takes effect again on one that can (rule RC-3).
pub fn reconcile(profile: &SandboxProfile, caps: &RuntimeCapabilities) -> Vec<UnsatisfiableLimit> {
    let ResourceBudget {
        cpus_milli,
        memory_bytes,
        pids,
        storage_bytes,
    } = profile.budget;

    let mut out = Vec::new();
    let mut check = |set: bool, support: &LimitSupport, field: &'static str| {
        if let (true, LimitSupport::Unsupported { reason }) = (set, support) {
            out.push(UnsatisfiableLimit {
                field,
                reason: reason.clone(),
            });
        }
    };
    check(cpus_milli.is_some(), &caps.cpus, "processor");
    check(memory_bytes.is_some(), &caps.memory, "memory");
    check(pids.is_some(), &caps.pids, "processes");
    check(storage_bytes.is_some(), &caps.storage, "storage");
    out
}

/// A step in acquiring an image, reported often enough for a progress indicator to move.
///
/// SC-004 gives first-time enable five minutes; five silent minutes reads as a hang, which is why
/// this is an obligation (C-8) rather than a nicety.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// What is happening, e.g. `"Downloading"`.
    pub stage: String,
    /// The layer or item, when the runtime names one.
    pub detail: Option<String>,
    /// Completion in the range 0..=100, when the runtime reports enough to compute it.
    pub percent: Option<u8>,
}

/// An identifier for a created container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerId(pub String);

/// Drives one container runtime.
///
/// The seam FR-020 requires. Implementations are **argument dialects over the runtime's own CLI**
/// (research R7), not API clients: `argv` builds the arguments, `parse` reads the output, and
/// `exec` is the single place a process is spawned. That layering is what lets the whole surface
/// below be tested with nothing installed.
///
/// Implementations never re-validate a [`SandboxSpec`] and never silently drop a field: if a spec
/// reaches [`Self::create`], every part of it is enforceable (obligation C-2). And they mount
/// exactly the spec's `MountSet` — no implicit home, no runtime control socket (obligation C-3).
pub trait ContainerRuntime {
    /// Is this runtime installed and usable? Cheap; no container is created.
    fn detect(&self) -> Result<RuntimeVersion, RuntimeError>;

    /// What can it enforce here? Runs once per version and is cached against it (research R10).
    fn probe(&self) -> Result<RuntimeCapabilities, RuntimeError>;

    /// Is the image present locally, and what fingerprint does it carry (FR-024, FR-024d)?
    fn inspect_image(&self, reference: &str) -> Result<Option<ImageFacts>, RuntimeError>;

    /// Make the image available: pull, import from a file, or build (FR-024a–c).
    fn acquire_image(
        &self,
        source: &ImageSource,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ImageFacts, RuntimeError>;

    /// Create the container. The spec is already validated and reconciled.
    fn create(&self, spec: &SandboxSpec) -> Result<ContainerId, RuntimeError>;

    /// Start a created container. Succeeds if it is already running (obligation C-7).
    fn start(&self, id: &ContainerId) -> Result<(), RuntimeError>;

    /// Stop a running container. Succeeds if it is already stopped (obligation C-7).
    fn stop(&self, id: &ContainerId) -> Result<(), RuntimeError>;

    /// Remove a container. Succeeds if it is already absent (obligation C-7).
    ///
    /// The idempotence is not tidiness: the client's recovery paths call these without checking
    /// first, and a race with the user's own `docker stop` must not produce an error dialog.
    fn remove(&self, id: &ContainerId) -> Result<(), RuntimeError>;

    /// What the container is doing now.
    fn inspect(&self, id: &ContainerId) -> Result<ContainerFacts, RuntimeError>;

    /// The container carrying `name`, if there is one.
    ///
    /// Distinct from [`Self::inspect`] because "is one already there?" has a negative answer, and
    /// a sandbox outliving the application means that question is asked on almost every start.
    fn find(&self, name: &str) -> Result<Option<ContainerFacts>, RuntimeError>;

    /// The daemon's own diagnostics from inside the sandbox (US6 scenario 6).
    fn logs(&self, id: &ContainerId, lines: usize) -> Result<Vec<String>, RuntimeError>;
}

/// Why a runtime operation failed.
///
/// A closed enumeration, never raw text (obligation C-6). Each variant carries the reason and the
/// remedy FR-034 requires, so a failure is a testable value rather than a formatted string built at
/// the call site. [`RuntimeError::Unknown`] is the only variant permitted to surface unclassified
/// text, and it retains stderr for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// The runtime's executable is not on `PATH`.
    NotInstalled { kind: RuntimeKind },
    /// Installed, but its service is not running.
    NotRunning { kind: RuntimeKind },
    /// Installed and running, but this user may not use it — not in the required group, or a
    /// rootless setup that was never initialised.
    PermissionDenied { kind: RuntimeKind },
    /// Installed, but older than this feature needs.
    VersionTooOld { found: String, needed: String },
    /// The image is not present and could not be found where it was looked for.
    ImageNotFound { reference: String },
    /// The image exists but could not be fetched — rate limit, authentication, or a proxy. The
    /// offline import path (FR-024a) is the way forward, and the remedy says so.
    ImagePullFailed { reference: String, detail: String },
    /// The control port is taken.
    PortUnavailable { port: u16 },
    /// A project path the runtime will not share — a network mount, an excluded path, or a form
    /// the runtime does not accept.
    MountRejected { path: String, detail: String },
    /// The runtime refused a limit the capability probe said it supported. A probe/reality
    /// disagreement, which is worth reporting distinctly rather than folding into `Unknown`.
    LimitRejected { field: String, detail: String },
    /// The runtime did not answer in time.
    Timeout { operation: String },
    /// Unclassified. Retains stderr so the log has the whole story.
    Unknown { stderr: String },
}

impl RuntimeError {
    /// What went wrong, in one sentence.
    pub fn reason(&self) -> String {
        match self {
            RuntimeError::NotInstalled { kind } => format!("{kind} is not installed."),
            RuntimeError::NotRunning { kind } => format!("{kind} is installed but not running."),
            RuntimeError::PermissionDenied { kind } => {
                format!("{kind} is running, but this user is not permitted to use it.")
            }
            RuntimeError::VersionTooOld { found, needed } => {
                format!("This runtime is version {found}; the sandbox needs {needed} or newer.")
            }
            RuntimeError::ImageNotFound { reference } => {
                format!("The image `{reference}` was not found.")
            }
            RuntimeError::ImagePullFailed { reference, detail } => {
                format!("`{reference}` could not be fetched: {detail}")
            }
            RuntimeError::PortUnavailable { port } => {
                format!("Port {port} is already in use, so the sandbox has no control channel.")
            }
            RuntimeError::MountRejected { path, detail } => {
                format!("`{path}` cannot be shared with the sandbox: {detail}")
            }
            RuntimeError::LimitRejected { field, detail } => {
                format!("The runtime refused the {field} limit: {detail}")
            }
            RuntimeError::Timeout { operation } => format!("{operation} did not finish in time."),
            RuntimeError::Unknown { stderr } => {
                let line = stderr.lines().next().unwrap_or("no detail").trim();
                format!("The runtime reported: {line}")
            }
        }
    }

    /// What the user can do about it. Every failure has one — that is what FR-034 requires, and
    /// what stops this feature turning a security improvement into a support burden.
    pub fn remedy(&self) -> String {
        match self {
            RuntimeError::NotInstalled { kind } => {
                format!("Install {kind}, or choose another runtime in Settings → Daemon.")
            }
            RuntimeError::NotRunning { kind } => {
                format!("Start {kind}, then retry.")
            }
            RuntimeError::PermissionDenied { kind } => match kind {
                RuntimeKind::Docker => {
                    "Add your user to the `docker` group and log in again.".to_string()
                }
                RuntimeKind::Podman => {
                    "Initialise rootless Podman (`podman system migrate`), then retry.".to_string()
                }
            },
            RuntimeError::VersionTooOld { needed, .. } => {
                format!("Upgrade the runtime to {needed} or newer.")
            }
            RuntimeError::ImageNotFound { .. } => {
                "Check the image reference in Settings → Daemon, or import an image archive."
                    .to_string()
            }
            RuntimeError::ImagePullFailed { .. } => {
                "Import the image from a file instead — the sandbox does not need a registry."
                    .to_string()
            }
            RuntimeError::PortUnavailable { .. } => {
                "Stop whatever holds the port, or choose another in Settings → Daemon.".to_string()
            }
            RuntimeError::MountRejected { .. } => {
                "Move the project to a path the runtime can share, or unregister it.".to_string()
            }
            RuntimeError::LimitRejected { field, .. } => {
                format!("Clear the {field} limit in Settings → Daemon, then retry.")
            }
            RuntimeError::Timeout { .. } => {
                "Retry; if it persists, restart the runtime.".to_string()
            }
            RuntimeError::Unknown { .. } => {
                "Retry. The runtime's own output is in the diagnostics.".to_string()
            }
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.reason(), self.remedy())
    }
}

impl std::error::Error for RuntimeError {}

/// Classify a failed invocation into the closed enumeration (obligation C-6).
///
/// Pure: it reads the exit code and the text the runtime printed, and nothing else. Matching on
/// output text is unlovely, but the alternative — an API client — moves this boundary inside a
/// dependency and makes the same tests require a live daemon (research R7).
pub fn classify(kind: RuntimeKind, out: &CommandOutput) -> RuntimeError {
    let text = format!("{} {}", out.stderr, out.stdout).to_ascii_lowercase();
    let has = |needle: &str| text.contains(needle);

    if has("command not found") || has("executable file not found") || has("not recognized") {
        return RuntimeError::NotInstalled { kind };
    }
    if has("permission denied") {
        return RuntimeError::PermissionDenied { kind };
    }
    if has("cannot connect to the docker daemon") || has("is the docker daemon running") {
        return RuntimeError::NotRunning { kind };
    }
    if has("pull rate limit") || has("toomanyrequests") || has("unauthorized") {
        return RuntimeError::ImagePullFailed {
            reference: String::new(),
            detail: first_line(&out.stderr),
        };
    }
    if has("pull access denied") || has("manifest unknown") || has("repository does not exist") {
        return RuntimeError::ImageNotFound {
            reference: String::new(),
        };
    }
    if has("address already in use") || has("bind for") {
        return RuntimeError::PortUnavailable { port: 0 };
    }
    if has("mount source path") || has("invalid mount") || has("read-only file system") {
        return RuntimeError::MountRejected {
            path: String::new(),
            detail: first_line(&out.stderr),
        };
    }
    if has("--storage-opt") || has("pquota") {
        return RuntimeError::LimitRejected {
            field: "storage".to_string(),
            detail: first_line(&out.stderr),
        };
    }
    RuntimeError::Unknown {
        stderr: out.stderr.clone(),
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::{Bytes, ResourceBudget};

    fn caps(storage: LimitSupport) -> RuntimeCapabilities {
        RuntimeCapabilities {
            kind: RuntimeKind::Docker,
            version: "29.5.1".to_string(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage,
            identity_mapping: IdentityMapping::ExplicitUidGid,
        }
    }

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/runtime")
                .join(name),
        )
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
    }

    #[test]
    fn an_unset_limit_is_never_unsatisfiable() {
        // Rule RC-2: reconciliation reports what the user *asked for* and cannot get. Asking for
        // nothing cannot fail, however incapable the runtime is.
        let profile = SandboxProfile {
            budget: ResourceBudget {
                storage_bytes: None,
                ..ResourceBudget::default()
            },
            ..SandboxProfile::default()
        };
        let caps = caps(LimitSupport::unsupported("overlayfs without pquota"));
        assert!(reconcile(&profile, &caps).is_empty());
    }

    #[test]
    fn a_set_limit_the_runtime_cannot_enforce_is_reported_with_its_reason() {
        let profile = SandboxProfile {
            budget: ResourceBudget {
                storage_bytes: Some(Bytes::from_mib(4096)),
                ..ResourceBudget::default()
            },
            ..SandboxProfile::default()
        };
        let caps = caps(LimitSupport::unsupported("overlayfs without pquota"));
        let out = reconcile(&profile, &caps);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].field, "storage");
        // The reason travels with it, because the view shows it (SC-009).
        assert!(out[0].reason.contains("pquota"));
    }

    #[test]
    fn reconciliation_does_not_mutate_the_profile() {
        // Rule RC-3. The user's intent survives a move to a runtime that cannot honour it, and
        // takes effect again on one that can.
        let profile = SandboxProfile {
            budget: ResourceBudget {
                storage_bytes: Some(Bytes::from_mib(4096)),
                ..ResourceBudget::default()
            },
            ..SandboxProfile::default()
        };
        let before = profile.clone();
        let _ = reconcile(&profile, &caps(LimitSupport::unsupported("no")));
        assert_eq!(profile, before);
    }

    #[test]
    fn every_runtime_error_carries_a_reason_and_a_remedy() {
        // FR-034: no failure in this feature may be a dead end. Enumerated explicitly so a variant
        // added later without a remedy fails here rather than in front of a user.
        let all = [
            RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
            RuntimeError::NotRunning {
                kind: RuntimeKind::Docker,
            },
            RuntimeError::PermissionDenied {
                kind: RuntimeKind::Podman,
            },
            RuntimeError::VersionTooOld {
                found: "1.0".into(),
                needed: "20.10".into(),
            },
            RuntimeError::ImageNotFound {
                reference: "x".into(),
            },
            RuntimeError::ImagePullFailed {
                reference: "x".into(),
                detail: "d".into(),
            },
            RuntimeError::PortUnavailable { port: 7727 },
            RuntimeError::MountRejected {
                path: "/x".into(),
                detail: "d".into(),
            },
            RuntimeError::LimitRejected {
                field: "storage".into(),
                detail: "d".into(),
            },
            RuntimeError::Timeout {
                operation: "start".into(),
            },
            RuntimeError::Unknown {
                stderr: "boom".into(),
            },
        ];
        for e in all {
            assert!(!e.reason().trim().is_empty(), "{e:?} has no reason");
            assert!(!e.remedy().trim().is_empty(), "{e:?} has no remedy");
        }
    }

    #[test]
    fn the_permission_remedy_differs_by_runtime() {
        // "Add yourself to the docker group" is useless advice to a podman user. A remedy that is
        // generic enough to be always-correct is not a remedy.
        let docker = RuntimeError::PermissionDenied {
            kind: RuntimeKind::Docker,
        }
        .remedy();
        let podman = RuntimeError::PermissionDenied {
            kind: RuntimeKind::Podman,
        }
        .remedy();
        assert_ne!(docker, podman);
    }

    #[test]
    fn canned_failures_map_to_distinct_variants() {
        // Conformance check K-8, against output the runtimes actually produce.
        type Matcher = fn(&RuntimeError) -> bool;
        let cases: [(&str, Matcher); 7] = [
            ("err_not_installed.txt", |e| {
                matches!(e, RuntimeError::NotInstalled { .. })
            }),
            ("err_daemon_down.txt", |e| {
                matches!(e, RuntimeError::NotRunning { .. })
            }),
            ("err_permission_denied.txt", |e| {
                matches!(e, RuntimeError::PermissionDenied { .. })
            }),
            ("err_image_not_found.txt", |e| {
                matches!(e, RuntimeError::ImageNotFound { .. })
            }),
            ("err_pull_failed.txt", |e| {
                matches!(e, RuntimeError::ImagePullFailed { .. })
            }),
            ("err_port_unavailable.txt", |e| {
                matches!(e, RuntimeError::PortUnavailable { .. })
            }),
            ("err_mount_rejected.txt", |e| {
                matches!(e, RuntimeError::MountRejected { .. })
            }),
        ];
        for (fixture_name, matches_variant) in cases {
            let out = CommandOutput::err(125, fixture(fixture_name));
            let err = classify(RuntimeKind::Docker, &out);
            assert!(
                matches_variant(&err),
                "{fixture_name} classified as {err:?}"
            );
        }
    }

    #[test]
    fn unclassifiable_output_becomes_unknown_and_keeps_stderr() {
        // The only variant allowed to surface unclassified text — and it keeps the whole thing, so
        // the diagnostics have the story even when the classifier does not.
        let out = CommandOutput::err(1, "something nobody has seen before");
        match classify(RuntimeKind::Docker, &out) {
            RuntimeError::Unknown { stderr } => assert!(stderr.contains("nobody has seen")),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }
}

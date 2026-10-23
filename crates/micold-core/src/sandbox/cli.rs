//! [`ContainerRuntime`] over a runtime's own CLI (research R7).
//!
//! The only implementation there is, and the only one there should be: `argv` builds the arguments,
//! `parse` reads the output, `dialect` holds the per-runtime differences, and `exec` spawns. What
//! is left here is sequencing — which command runs when, and what its failure means.
//!
//! Every method is generic over [`CommandRunner`], so the whole surface is driven by
//! [`RecordingRunner`](super::exec::RecordingRunner) in tests with nothing installed. That is not a
//! testing convenience bolted on afterwards; it is why the layering exists.

use std::ffi::{OsStr, OsString};
use std::time::Duration;

use super::argv;
use super::dialect::Dialect;
use super::exec::{CommandOutput, CommandRunner, RunLimits, StreamEvent};
use super::image::{ImageChoice, OwnImage};
use super::parse::{self, ContainerFacts, ImageFacts, RuntimeInfo};
use super::runtime::{
    classify, ContainerId, ContainerRuntime, IdentityMapping, LimitSupport, NoOutboundSupport,
    Progress, RuntimeCapabilities, RuntimeError, RuntimeKind, RuntimeVersion,
};
use super::SandboxSpec;

/// Drives one container runtime through its command-line interface.
pub struct CliRuntime<R: CommandRunner> {
    kind: RuntimeKind,
    runner: R,
    /// What may end an image acquisition early (feature 806, FR-019).
    acquire: RunLimits,
    /// What may end every other runtime command early (FR-019).
    commands: RunLimits,
}

impl<R: CommandRunner> CliRuntime<R> {
    /// A runtime of `kind`, driven through `runner`.
    pub fn new(kind: RuntimeKind, runner: R) -> Self {
        Self {
            kind,
            runner,
            acquire: RunLimits::default(),
            commands: RunLimits::default(),
        }
    }

    /// End any runtime command other than the image pull after `bound`, or when `cancel` is
    /// cancelled, so a hung `create` or `start` cannot hang the start (FR-019).
    #[must_use]
    pub fn limiting_commands(
        mut self,
        bound: Duration,
        cancel: crate::provision::CancelToken,
    ) -> Self {
        self.commands = RunLimits {
            deadline: Some(bound),
            cancel: Some(cancel),
        };
        self
    }

    /// End an image acquisition after `bound`, or when `cancel` is cancelled: the runtime process
    /// is killed and the acquisition fails (FR-019, FR-023).
    #[must_use]
    pub fn limiting_acquire(
        mut self,
        bound: Duration,
        cancel: crate::provision::CancelToken,
    ) -> Self {
        self.acquire = RunLimits {
            deadline: Some(bound),
            cancel: Some(cancel),
        };
        self
    }

    /// A timeout while getting the image, naming the image and the bound.
    fn bounded_acquire(&self, error: RuntimeError, reference: &str) -> RuntimeError {
        match (error, self.acquire.deadline) {
            (RuntimeError::Timeout { .. }, Some(bound)) => RuntimeError::Timeout {
                operation: format!(
                    "Getting the image `{reference}` (limit {} minutes)",
                    bound.as_secs().div_ceil(60)
                ),
            },
            (other, _) => other,
        }
    }

    fn dialect(&self) -> Dialect {
        Dialect::for_kind(self.kind)
    }

    fn program(&self) -> OsString {
        OsString::from(self.dialect().program)
    }

    /// Run and classify. An io error reaching the program at all is `NotInstalled`: the runtime is
    /// looked up on `PATH`, so "no such file" is not an unknown failure, it is the answer.
    fn run(&self, args: &[&str]) -> Result<CommandOutput, RuntimeError> {
        let owned: Vec<OsString> = args.iter().map(OsString::from).collect();
        if self.commands.deadline.is_none() && self.commands.cancel.is_none() {
            return self.finish(self.runner.run(&self.program(), &owned));
        }
        self.finish(
            self.runner
                .run_limited(&self.program(), &owned, &self.commands, &mut |_| {}),
        )
    }

    fn finish(
        &self,
        result: std::io::Result<CommandOutput>,
    ) -> Result<CommandOutput, RuntimeError> {
        match result {
            Ok(out) if out.success() => Ok(out),
            Ok(out) => Err(classify(self.kind, &out)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(RuntimeError::NotInstalled { kind: self.kind })
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Err(RuntimeError::Cancelled),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(RuntimeError::Timeout {
                operation: format!("The {} command", self.kind),
            }),
            Err(e) => Err(RuntimeError::Unknown {
                stderr: e.to_string(),
            }),
        }
    }

    /// Run, treating output matching `already` as success (obligation C-7).
    ///
    /// Idempotence is not tidiness here: the client's recovery paths call stop/remove/start without
    /// checking first, and a race with the user's own `docker stop` must not surface as an error
    /// dialog about a state the user just asked for.
    fn run_idempotent(&self, args: &[&str], already: &[&str]) -> Result<(), RuntimeError> {
        match self.run(args) {
            Ok(_) => Ok(()),
            Err(e) => {
                let text = match &e {
                    RuntimeError::Unknown { stderr } => stderr.to_ascii_lowercase(),
                    _ => String::new(),
                };
                if already.iter().any(|needle| text.contains(needle)) {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// A pull failure the classifier left without the image's name gets it (US3 #3: the error names
/// the image).
fn name_the_image(error: RuntimeError, reference: &str) -> RuntimeError {
    match error {
        RuntimeError::ImageNotFound { reference: r } if r.is_empty() => {
            RuntimeError::ImageNotFound {
                reference: reference.to_string(),
            }
        }
        RuntimeError::ImagePullFailed {
            reference: r,
            detail,
        } if r.is_empty() => RuntimeError::ImagePullFailed {
            reference: reference.to_string(),
            detail,
        },
        other => other,
    }
}

/// Which storage drivers can enforce `--storage-opt size=` (research R5).
///
/// This *is* the probe: the driver is read from the runtime rather than inferred from its version,
/// which is what R10 asked for. Actually attempting the limit would be a truer probe still, but it
/// costs a container start on every capability refresh, and the driver determines the answer.
///
/// # `overlayfs` is unsupported, and the exit code that said otherwise was the wrong measurement
///
/// This originally classified Docker 29's `overlayfs` as supported, on the strength of
/// `docker run --storage-opt size=1G alpine true` → exit 0 (research R5). That measured whether the
/// flag is **accepted**. It is; it is also **ignored**. Measured again, on the same runtime:
///
/// ```text
/// $ docker run --rm --storage-opt size=512m … sh -c 'dd if=/dev/zero of=/big bs=1M count=700'
/// 734003200 bytes (734 MB, 700 MiB) copied, 0.302584 s, 2.4 GB/s
/// ```
///
/// 700 MiB written into a 512 MiB cap, no error anywhere. Both overlay drivers enforce a size limit
/// only over a filesystem with project quotas — xfs with `pquota`, or ext4 with `prjquota` — which
/// cannot be read from `info`, so both are reported unsupported **with the reason** rather than
/// guessed at. That is the whole point of C-2: a limit the user believes is enforced and is not is
/// worse than one they were told they cannot have, and an exit code of 0 is not enforcement.
///
/// `crates/micold-core/tests/sandbox_real_storage.rs` is what now holds this honest — it writes past
/// the cap against the real runtime and requires the classification to match what happened.
fn storage_support(driver: &str) -> LimitSupport {
    match driver {
        "btrfs" | "zfs" => LimitSupport::Supported,
        "overlayfs" | "overlay2" => LimitSupport::unsupported(
            "the overlay storage drivers enforce a size limit only over a filesystem with project \
             quotas (xfs with `pquota`, ext4 with `prjquota`), which cannot be detected here — \
             without one the runtime accepts the limit and does not apply it",
        ),
        "" => LimitSupport::unsupported("the runtime did not report a storage driver"),
        other => LimitSupport::unsupported(format!(
            "the `{other}` storage driver does not support a per-container size limit"
        )),
    }
}

/// How this runtime setup blocks outbound connections and keeps the published port (contracts
/// C-5's table, research R4's amendment, BUG-1285).
///
/// Read from the same `info` as [`storage_support`], because the answer is a fact of the setup, not
/// of the runtime: rootless Podman forwards published ports through `rootlessport`, so an
/// `--internal` network keeps them; rootful Podman forwards them through netavark's DNAT, which an
/// internal network does not get, and only netavark 1.7's `no_default_route` blocks outbound there.
/// Before 1.7 netavark accepts that option and ignores it, which would leave outbound open without
/// a word — so an older or unknown backend is `Unsupported`, never a guess.
pub fn no_outbound_support(kind: RuntimeKind, info: &RuntimeInfo) -> NoOutboundSupport {
    const WAYS: &str = "rootless Podman blocks them with an internal network, and rootful Podman \
                        needs netavark 1.7 or newer";
    match kind {
        RuntimeKind::Docker => NoOutboundSupport::docker(),
        RuntimeKind::Podman if info.rootless => NoOutboundSupport::podman_rootless(),
        RuntimeKind::Podman => {
            let reason = match (
                info.network_backend.as_deref(),
                info.network_backend_version.as_deref(),
            ) {
                (Some("netavark") | None, Some(v)) if netavark_has_no_default_route(v) => {
                    return NoOutboundSupport::podman_no_default_route();
                }
                (Some("netavark") | None, Some(v)) => format!(
                    "rootful Podman on {v} cannot block outbound connections without cutting off \
                     the published port; {WAYS}"
                ),
                (Some("netavark") | None, None) => format!(
                    "rootful Podman did not report its netavark version, so it is not known to \
                     block outbound connections; {WAYS}"
                ),
                (Some(other), _) => format!(
                    "rootful Podman with the `{other}` network backend cannot block outbound \
                     connections without cutting off the published port; {WAYS}"
                ),
            };
            NoOutboundSupport::Unsupported { reason }
        }
    }
}

/// Whether a netavark version line (`netavark 1.10.3`) is 1.7 or newer, where `no_default_route`
/// takes effect. A line this cannot read is not 1.7.
fn netavark_has_no_default_route(version_line: &str) -> bool {
    let Some(version) = version_line
        .split_whitespace()
        .find(|t| t.starts_with(|c: char| c.is_ascii_digit()))
    else {
        return false;
    };
    let mut parts = version.split('.').map(|p| {
        let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().ok()
    });
    match (parts.next().flatten(), parts.next().flatten()) {
        (Some(major), Some(minor)) => (major, minor) >= (1, 7),
        _ => false,
    }
}

impl<R: CommandRunner> ContainerRuntime for CliRuntime<R> {
    fn detect(&self) -> Result<RuntimeVersion, RuntimeError> {
        let out = self.run(&["version", "--format", "{{json .}}"])?;
        parse::version(self.kind, &out.stdout)
    }

    fn probe(&self) -> Result<RuntimeCapabilities, RuntimeError> {
        let version = self.detect()?;
        let out = self.run(&["info", "--format", "{{json .}}"])?;
        let info = parse::info(&out.stdout)?;
        Ok(RuntimeCapabilities {
            kind: self.kind,
            version: version.version,
            // Both runtimes take these on every platform this feature supports, including through
            // Docker Desktop's Linux VM. Storage is the one that varies, which is an observation
            // rather than something the code assumes — hence the same `LimitSupport` for all four.
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage: storage_support(&info.storage_driver),
            identity_mapping: self.dialect().identity,
            no_outbound: no_outbound_support(self.kind, &info),
        })
    }

    fn inspect_image(&self, reference: &str) -> Result<Option<ImageFacts>, RuntimeError> {
        match self.run(&["image", "inspect", reference, "--format", "{{json .}}"]) {
            Ok(out) => parse::image(&out.stdout).map(Some),
            // Absent is a normal answer to "is it here?", not a failure.
            Err(RuntimeError::ImageNotFound { .. }) => Ok(None),
            Err(RuntimeError::Unknown { stderr })
                if stderr.to_ascii_lowercase().contains("no such image") =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn acquire_image(
        &self,
        source: &ImageChoice,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ImageFacts, RuntimeError> {
        let reference = source.reference();
        // Already here: nothing to acquire. Checked first for every reference, so a machine that is
        // offline with the image present never touches the network (Principle IV), and a local
        // `micold-basic:dev` is used as it is. An archive is always loaded: the user may have
        // replaced the file, and the name it is tagged with says nothing about which file it was.
        if !matches!(source, ImageChoice::Own(OwnImage::Archive(_))) {
            if let Some(facts) = self.inspect_image(reference)? {
                progress(Progress {
                    stage: "Ready".into(),
                    detail: Some(reference.to_string()),
                    percent: Some(100),
                });
                return Ok(facts);
            }
        }

        match source {
            ImageChoice::Basic | ImageChoice::Own(OwnImage::Reference(_)) => {
                let args: Vec<OsString> = ["pull", reference].iter().map(OsString::from).collect();
                let mut on_line = |event: StreamEvent<'_>| {
                    progress(match event {
                        StreamEvent::Line(line) => pull_progress(line),
                        StreamEvent::Quiet { elapsed } => still_working("Downloading", elapsed),
                    })
                };
                self.finish(self.runner.run_limited(
                    &self.program(),
                    &args,
                    &self.acquire,
                    &mut on_line,
                ))
                .map_err(|e| self.bounded_acquire(name_the_image(e, reference), reference))?;
            }
            ImageChoice::Own(OwnImage::Archive(path)) => {
                let args: Vec<OsString> = vec![
                    OsString::from("load"),
                    OsString::from("-i"),
                    path.as_os_str().to_os_string(),
                ];
                // `load` without a terminal prints nothing until the import is over, so the quiet
                // ticks are the only progress a large archive gets (#790).
                let mut loaded = String::new();
                let mut on_line = |event: StreamEvent<'_>| {
                    progress(match event {
                        StreamEvent::Line(line) => {
                            if let Some(name) = line.trim().strip_prefix("Loaded image") {
                                let name = name.trim_start_matches(" ID").trim_start_matches(':');
                                loaded = name.trim().to_string();
                            }
                            Progress {
                                stage: "Importing".into(),
                                detail: Some(line.to_string()),
                                percent: None,
                            }
                        }
                        StreamEvent::Quiet { elapsed } => still_working("Importing", elapsed),
                    })
                };
                self.finish(self.runner.run_limited(
                    &self.program(),
                    &args,
                    &self.acquire,
                    &mut on_line,
                ))
                .map_err(|e| self.bounded_acquire(e, reference))?;
                // The archive names no reference the container could be created from, so what it
                // loaded is tagged with the fixed one `ImageChoice::reference` returns.
                if loaded.is_empty() {
                    return Err(RuntimeError::ImageNotFound {
                        reference: path.display().to_string(),
                    });
                }
                self.run(&["tag", loaded.as_str(), reference])?;
            }
        }

        self.inspect_image(reference)?
            .ok_or_else(|| RuntimeError::ImageNotFound {
                reference: reference.to_string(),
            })
    }

    fn create(&self, spec: &SandboxSpec) -> Result<ContainerId, RuntimeError> {
        // The probe first: it decides the network's arguments, and a setup that cannot block
        // outbound is refused here, before any network exists (FR-017a).
        let caps = self.probe()?;
        // Then the network. "Already exists" is the normal case on every start after the first.
        let net_args = argv::network_create(spec, &caps)?;
        let net: Vec<&str> = net_args.iter().filter_map(|a| a.to_str()).collect();
        self.run_idempotent(&net, &["already exists"])?;

        let args = argv::create(spec, &caps);
        let borrowed: Vec<&str> = args.iter().filter_map(|a| a.to_str()).collect();
        // The mount list is known here and nowhere below, so a rejected bind is named here or not
        // at all — see `RuntimeError::naming_mount`.
        let out = self
            .run(&borrowed)
            .map_err(|e| e.naming_mount(&spec.mounts.host_paths()))?;
        Ok(ContainerId(out.stdout.trim().to_string()))
    }

    fn start(&self, id: &ContainerId) -> Result<(), RuntimeError> {
        self.run_idempotent(
            &["start", &id.0],
            &["already started", "is already running"],
        )
    }

    fn stop(&self, id: &ContainerId) -> Result<(), RuntimeError> {
        self.run_idempotent(
            &["stop", &id.0],
            &["is not running", "no such container", "already stopped"],
        )
    }

    fn remove(&self, id: &ContainerId) -> Result<(), RuntimeError> {
        self.run_idempotent(&["rm", "-f", &id.0], &["no such container"])
    }

    fn inspect(&self, id: &ContainerId) -> Result<ContainerFacts, RuntimeError> {
        let out = self.run(&["inspect", &id.0, "--format", "{{json .}}"])?;
        parse::container(&out.stdout)
    }

    fn find(&self, name: &str) -> Result<Option<ContainerFacts>, RuntimeError> {
        match self.run(&["inspect", name, "--format", "{{json .}}"]) {
            Ok(out) => parse::container(&out.stdout).map(Some),
            // "No such container" is the answer, not a failure.
            Err(RuntimeError::Unknown { stderr })
                if stderr.to_ascii_lowercase().contains("no such") =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn logs(&self, id: &ContainerId, lines: usize) -> Result<Vec<String>, RuntimeError> {
        let n = lines.to_string();
        let out = self.run(&["logs", "--tail", &n, &id.0])?;
        Ok(out
            .stdout
            .lines()
            .chain(out.stderr.lines())
            .map(str::to_string)
            .collect())
    }
}

/// Turn one line of `docker pull` output into a progress report.
///
/// Best-effort by design. The format is not a contract, so a line this does not recognise still
/// produces a report — an unrecognised line means the indicator moves without a percentage, which
/// is the honest rendering of "something is happening and we cannot say how much is left".
fn pull_progress(line: &str) -> Progress {
    let stage = line
        .split_once(':')
        .map(|(_, rest)| rest.trim())
        .unwrap_or(line)
        .split_whitespace()
        .next()
        .unwrap_or("Downloading")
        .to_string();
    Progress {
        stage: if stage.is_empty() {
            "Downloading".to_string()
        } else {
            stage
        },
        detail: Some(line.to_string()),
        percent: None,
    }
}

/// The report for a runtime that is still working but has said nothing for a while.
///
/// No percentage: the runtime gave nothing to compute one from. The elapsed time is what moves.
fn still_working(stage: &str, elapsed: Duration) -> Progress {
    Progress {
        stage: stage.to_string(),
        detail: Some(format!("still working, {}s", elapsed.as_secs())),
        percent: None,
    }
}

/// A helper for the one place a `&OsStr` program name is needed outside this module.
pub fn program_of(kind: RuntimeKind) -> &'static OsStr {
    OsStr::new(kind.program())
}

/// Only ever true when a dialect's identity mapping is the explicit one — kept as a named function
/// so the meaning is visible at the call site rather than as a bare match on an enum.
pub fn needs_explicit_uid(mapping: IdentityMapping) -> bool {
    matches!(mapping, IdentityMapping::ExplicitUidGid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provision::CancelToken;
    use crate::sandbox::runtime::RuntimeKind;
    use std::io;

    /// Answers `image inspect` with "no such image", then ends every limited run as `ended`.
    struct EndsEarly {
        ended: io::ErrorKind,
        inspect_stderr: &'static str,
    }

    impl CommandRunner for EndsEarly {
        fn run(&self, _: &OsStr, _: &[OsString]) -> io::Result<CommandOutput> {
            Ok(CommandOutput::err(1, self.inspect_stderr))
        }

        fn run_limited(
            &self,
            _: &OsStr,
            _: &[OsString],
            limits: &RunLimits,
            _: &mut dyn FnMut(StreamEvent<'_>),
        ) -> io::Result<CommandOutput> {
            assert_eq!(limits.deadline, Some(Duration::from_secs(15 * 60)));
            Err(io::Error::new(self.ended, "ended early"))
        }
    }

    fn acquire(ended: io::ErrorKind) -> RuntimeError {
        acquire_after(ended, "Error: No such image: x")
    }

    fn acquire_after(ended: io::ErrorKind, inspect_stderr: &'static str) -> RuntimeError {
        CliRuntime::new(
            RuntimeKind::Podman,
            EndsEarly {
                ended,
                inspect_stderr,
            },
        )
        .limiting_acquire(Duration::from_secs(15 * 60), CancelToken::new())
        .acquire_image(&ImageChoice::Basic, &mut |_| {})
        .expect_err("the pull ended early")
    }

    fn podman_info(rootless: bool, backend: Option<&str>, version: Option<&str>) -> RuntimeInfo {
        RuntimeInfo {
            storage_driver: "overlay".into(),
            rootless,
            network_backend: backend.map(str::to_string),
            network_backend_version: version.map(str::to_string),
        }
    }

    /// U57 (BUG-1285): contracts C-5's table, setup by setup.
    #[test]
    fn the_no_outbound_network_follows_the_probed_setup() {
        let docker = RuntimeInfo {
            storage_driver: "overlayfs".into(),
            rootless: false,
            network_backend: None,
            network_backend_version: None,
        };
        assert_eq!(
            no_outbound_support(RuntimeKind::Docker, &docker),
            NoOutboundSupport::docker()
        );
        for (rootless, backend, version) in [
            (true, Some("netavark"), Some("netavark 1.4.0")),
            (true, Some("cni"), None),
            (true, None, None),
        ] {
            assert_eq!(
                no_outbound_support(
                    RuntimeKind::Podman,
                    &podman_info(rootless, backend, version)
                ),
                NoOutboundSupport::podman_rootless(),
                "rootless {backend:?} {version:?}"
            );
        }
        for version in ["netavark 1.7.0", "netavark 1.10.3", "netavark 2.0.0-dev"] {
            assert_eq!(
                no_outbound_support(
                    RuntimeKind::Podman,
                    &podman_info(false, Some("netavark"), Some(version))
                ),
                NoOutboundSupport::podman_no_default_route(),
                "{version}"
            );
        }
    }

    #[test]
    fn a_rootful_podman_that_cannot_block_egress_is_unsupported_with_the_ways_forward() {
        for (backend, version, names) in [
            (Some("netavark"), Some("netavark 1.4.0"), "netavark 1.4.0"),
            (Some("netavark"), Some("netavark 1.6.9"), "netavark 1.6.9"),
            (Some("netavark"), Some("netavark"), "netavark"),
            (Some("netavark"), None, "did not report"),
            (Some("cni"), None, "`cni`"),
        ] {
            match no_outbound_support(RuntimeKind::Podman, &podman_info(false, backend, version)) {
                NoOutboundSupport::Unsupported { reason } => {
                    for needle in [names, "netavark 1.7", "rootless Podman"] {
                        assert!(reason.contains(needle), "{version:?}: {needle} in {reason}");
                    }
                }
                other => panic!("{backend:?} {version:?}: expected Unsupported, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_pull_past_its_bound_fails_naming_the_image_and_the_limit() {
        let error = acquire(io::ErrorKind::TimedOut);
        let reason = error.reason();
        assert!(matches!(error, RuntimeError::Timeout { .. }), "{error:?}");
        assert!(reason.contains("15 minutes"), "{reason}");
        assert!(reason.contains(ImageChoice::Basic.reference()), "{reason}");
    }

    #[test]
    fn podmans_image_not_known_is_absent_so_the_image_is_pulled() {
        // Podman 5.x words an absent image "image not known"; the pull must still be attempted,
        // which the stub observes by ending it early.
        let error = acquire_after(
            io::ErrorKind::Interrupted,
            "Error: ghcr.io/x/micold-basic@sha256:7551: image not known",
        );
        assert_eq!(error, RuntimeError::Cancelled);
    }

    #[test]
    fn a_cancelled_pull_fails_cancelled() {
        assert_eq!(acquire(io::ErrorKind::Interrupted), RuntimeError::Cancelled);
    }

    #[test]
    fn a_hung_command_other_than_the_pull_is_ended_by_the_command_limit() {
        let runtime = CliRuntime::new(
            RuntimeKind::Docker,
            EndsEarly {
                ended: io::ErrorKind::TimedOut,
                inspect_stderr: "",
            },
        )
        .limiting_commands(Duration::from_secs(15 * 60), CancelToken::new());
        let error = runtime
            .stop(&ContainerId("c1".into()))
            .expect_err("the stop timed out");
        assert!(matches!(error, RuntimeError::Timeout { .. }), "{error:?}");
    }
}

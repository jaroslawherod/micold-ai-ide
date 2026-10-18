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
use super::exec::{CommandOutput, CommandRunner, StreamEvent};
use super::image::{ImageChoice, OwnImage};
use super::parse::{self, ContainerFacts, ImageFacts};
use super::runtime::{
    classify, ContainerId, ContainerRuntime, IdentityMapping, LimitSupport, Progress,
    RuntimeCapabilities, RuntimeError, RuntimeKind, RuntimeVersion,
};
use super::SandboxSpec;

/// Drives one container runtime through its command-line interface.
pub struct CliRuntime<R: CommandRunner> {
    kind: RuntimeKind,
    runner: R,
}

impl<R: CommandRunner> CliRuntime<R> {
    /// A runtime of `kind`, driven through `runner`.
    pub fn new(kind: RuntimeKind, runner: R) -> Self {
        Self { kind, runner }
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
        self.finish(self.runner.run(&self.program(), &owned))
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
                self.finish(
                    self.runner
                        .run_streaming(&self.program(), &args, &mut on_line),
                )
                .map_err(|e| name_the_image(e, reference))?;
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
                self.finish(
                    self.runner
                        .run_streaming(&self.program(), &args, &mut on_line),
                )?;
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
        // The network first. "Already exists" is the normal case on every start after the first.
        let dialect = self.dialect();
        let net_args = argv::network_create(spec, &dialect);
        let net: Vec<&str> = net_args.iter().filter_map(|a| a.to_str()).collect();
        self.run_idempotent(&net, &["already exists"])?;

        let caps = self.probe()?;
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

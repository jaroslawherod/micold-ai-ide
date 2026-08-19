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

use super::argv;
use super::dialect::Dialect;
use super::exec::{CommandOutput, CommandRunner};
use super::image::{ImageSource, ImageSourceKind};
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

/// Which storage drivers can enforce `--storage-opt size=` (research R5).
///
/// This *is* the probe: the driver is read from the runtime rather than inferred from its version,
/// which is what R10 asked for. Actually attempting the limit would be a truer probe still, but it
/// costs a container start on every capability refresh, and the driver determines the answer.
///
/// Measured: Docker 29.5.1 on `overlayfs` accepts it (`docker run --storage-opt size=1G alpine
/// true` → exit 0). `overlay2` accepts it only over xfs with `pquota`, which cannot be read from
/// `info`, so it is reported unsupported with the reason rather than guessed at — a limit the user
/// believes is enforced and is not would be worse than one they were told they cannot have.
fn storage_support(driver: &str) -> LimitSupport {
    match driver {
        "overlayfs" | "btrfs" | "zfs" => LimitSupport::Supported,
        "overlay2" => LimitSupport::unsupported(
            "the overlay2 storage driver enforces a size limit only over xfs with the `pquota` \
             mount option, which cannot be detected here",
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
        source: &ImageSource,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<ImageFacts, RuntimeError> {
        // Already here: nothing to acquire. Checked first for every source, so a machine that is
        // offline with the image present never touches the network (Principle IV).
        if let Some(facts) = self.inspect_image(&source.reference)? {
            progress(Progress {
                stage: "Ready".into(),
                detail: Some(source.reference.clone()),
                percent: Some(100),
            });
            return Ok(facts);
        }

        match source.kind {
            ImageSourceKind::Registry => {
                let args: Vec<OsString> = ["pull", source.reference.as_str()]
                    .iter()
                    .map(OsString::from)
                    .collect();
                let mut on_line = |line: &str| progress(pull_progress(line));
                let out = self.finish(self.runner.run_streaming(
                    &self.program(),
                    &args,
                    &mut on_line,
                ))?;
                let _ = out;
            }
            ImageSourceKind::ImportedFile => {
                let path = source.path.as_ref().ok_or_else(|| RuntimeError::Unknown {
                    stderr: "importing from a file needs the archive's path".into(),
                })?;
                let args: Vec<OsString> = vec![
                    OsString::from("load"),
                    OsString::from("-i"),
                    path.as_os_str().to_os_string(),
                ];
                let mut on_line = |line: &str| {
                    progress(Progress {
                        stage: "Importing".into(),
                        detail: Some(line.to_string()),
                        percent: None,
                    })
                };
                self.finish(
                    self.runner
                        .run_streaming(&self.program(), &args, &mut on_line),
                )?;
            }
            ImageSourceKind::LocalBuild => {
                // Deliberately not built from here. `mise run image` builds it, because building
                // needs a Linux daemon binary cross-compiled and staged beside the Containerfile —
                // a build-system job, not a runtime-adapter one. What the app does is say so.
                return Err(RuntimeError::ImageNotFound {
                    reference: source.reference.clone(),
                });
            }
        }

        self.inspect_image(&source.reference)?
            .ok_or_else(|| RuntimeError::ImageNotFound {
                reference: source.reference.clone(),
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
        let out = self.run(&borrowed)?;
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

/// A helper for the one place a `&OsStr` program name is needed outside this module.
pub fn program_of(kind: RuntimeKind) -> &'static OsStr {
    OsStr::new(kind.program())
}

/// Only ever true when a dialect's identity mapping is the explicit one — kept as a named function
/// so the meaning is visible at the call site rather than as a bare match on an enum.
pub fn needs_explicit_uid(mapping: IdentityMapping) -> bool {
    matches!(mapping, IdentityMapping::ExplicitUidGid)
}

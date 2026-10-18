//! The one start sequence for a daemon behind SSH (feature 806, contracts/provisioning.md): reach
//! the server, probe, copy the build only when absent, install the AI CLIs, start, check the
//! version. Containers, 489 remote hosts and (686) pods hand it a [`ProvisionTarget`]; none of
//! them has a step of its own inside it. Nothing here names a container runtime, so a runtime
//! that only knows how to forward a port can use it.

pub mod ai_cli;
pub mod attempt;
pub mod failure;
pub mod keys;
pub mod login;
pub mod relay;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

pub use ai_cli::AiCliTable;
pub use attempt::CancelToken;
pub use failure::{FailureCause, ProvisionFailure, ProvisionWarning, Step};
pub use keys::DaemonKeys;

use crate::connect::{handshake_with, Connected, Credentials, Transport};
use crate::sandbox::AiCliSelection;
use crate::ssh::argv::{self, Reach};
use crate::ssh::classify::classify;
use crate::ssh::provision::{
    io_failure, parse_probe, select_artefact, ArtefactChoice, Probe, ProvisionOptions, SshHost,
    SshOutput,
};
use crate::ssh::SshFailure;

/// How long `Reach` waits between tries while the SSH server is not up yet.
const REACH_RETRY: Duration = Duration::from_millis(500);

/// A host and a port, for the install relay's allow list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPort {
    /// The host name.
    pub host: String,
    /// The port.
    pub port: u16,
}

/// How an app-owned target's installers reach the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Egress {
    /// The target connects out itself.
    Direct,
    /// Through a client-loopback proxy that allows only these `CONNECT` destinations.
    Relay {
        /// The destinations the relay allows.
        allow: Vec<HostPort>,
    },
}

/// Who owns the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ownership {
    /// The app made the target (a container, a pod): it may install into it and record the build.
    AppOwned {
        /// The AI CLIs to have installed.
        ai_clis: AiCliSelection,
        /// Their pinned packages.
        table: AiCliTable,
        /// The target's state directory.
        state_dir: PathBuf,
        /// How installers reach the network.
        egress: Egress,
    },
    /// A host the user owns: nothing is installed on it, and nothing is written but the build.
    UserHost,
}

/// The longest each step may take. `None` is unbounded (cancel still applies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepBounds {
    /// Getting the image.
    pub acquire: Duration,
    /// Starting the container.
    pub create_container: Duration,
    /// Reaching the SSH server.
    pub reach: Duration,
    /// Probing.
    pub probe: Duration,
    /// Copying the build; `None` for a transfer that reports progress instead.
    pub copy: Option<Duration>,
    /// Installing the AI CLIs, in total.
    pub install: Duration,
    /// Starting the daemon and the handshake.
    pub start: Duration,
    /// Checking the version.
    pub version: Duration,
}

impl StepBounds {
    /// A container's bounds: 15 minutes to get the image, 120 s per step, 600 s of installs.
    pub fn container() -> Self {
        let step = Duration::from_secs(120);
        Self {
            acquire: Duration::from_secs(15 * 60),
            create_container: step,
            reach: step,
            probe: step,
            copy: Some(step),
            install: Duration::from_secs(600),
            start: step,
            version: step,
        }
    }

    /// A user's host: the connect deadline bounds every step, the copy has none (489 R3).
    pub fn user_host(connect_deadline: Duration) -> Self {
        Self {
            acquire: connect_deadline,
            create_container: connect_deadline,
            reach: connect_deadline,
            probe: connect_deadline,
            copy: None,
            install: connect_deadline,
            start: connect_deadline,
            version: connect_deadline,
        }
    }
}

/// What a runtime hands the sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionTarget {
    /// How to reach its SSH server.
    pub reach: Reach,
    /// Who owns it.
    pub owned: Ownership,
    /// Each step's bound.
    pub bounds: StepBounds,
    /// The daemon's environment, quoted into the remote bridge command (empty for a user's host).
    pub env: Vec<(String, String)>,
}

impl ProvisionTarget {
    /// A host the user owns (489): their SSH configuration, nothing installed, and the connect
    /// deadline bounding every step but the copy.
    pub fn user_host(target: &crate::ssh::SshTarget, connect_deadline: Duration) -> Self {
        Self {
            reach: Reach::UserConfig(target.clone()),
            owned: Ownership::UserHost,
            bounds: StepBounds::user_host(connect_deadline),
            env: Vec::new(),
        }
    }
}

/// What the sequence reports as it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionEvent {
    /// A step began.
    Entered(Step),
    /// The copy has written this percentage.
    CopyPercent(u8),
    /// One line of installer output.
    InstallLine(String),
    /// One AI CLI's install finished.
    InstallDone {
        /// The CLI.
        cli: crate::session::AiCli,
        /// Whether it installed.
        ok: bool,
    },
    /// One AI CLI's install failed: the start goes on, and the row keeps this (FR-012).
    Warned(ProvisionWarning),
}

/// Receives the sequence's events.
pub type EventSink = Arc<dyn Fn(ProvisionEvent) + Send + Sync>;

/// A start that got as far as the handshake.
pub struct Provisioned {
    /// The handshake's outcome. `Refused` is for the caller to explain (489 FR-011).
    pub connected: Connected,
    /// CLIs whose install failed (the start still succeeded).
    pub warnings: Vec<ProvisionWarning>,
    /// Whether the build was copied by this start.
    pub copied: bool,
}

fn fail(step: Step, cause: FailureCause) -> ProvisionFailure {
    ProvisionFailure::new(step, cause)
}

/// Run `fut` as `step`: stopped by `cancel`, and by `bound` when there is one.
async fn guard<T>(
    step: Step,
    bound: Option<Duration>,
    cancel: &CancelToken,
    fut: impl std::future::Future<Output = Result<T, ProvisionFailure>>,
) -> Result<T, ProvisionFailure> {
    if cancel.is_cancelled() {
        return Err(fail(step, FailureCause::Cancelled));
    }
    let bounded = async {
        match bound {
            Some(b) => tokio::time::timeout(b, fut)
                .await
                .unwrap_or_else(|_| Err(fail(step, FailureCause::TimedOut { bound: b }))),
            None => fut.await,
        }
    };
    tokio::select! {
        r = bounded => r,
        () = cancel.cancelled() => Err(fail(step, FailureCause::Cancelled)),
    }
}

/// A finished `ssh` run as a failure of `step`, or its output when it succeeded.
fn checked(
    step: Step,
    out: SshOutput,
    bound: Option<Duration>,
    app_owned: bool,
) -> Result<SshOutput, ProvisionFailure> {
    if out.ok() {
        return Ok(out);
    }
    if out.timed_out {
        if let Some(bound) = bound {
            return Err(fail(step, FailureCause::TimedOut { bound }));
        }
    }
    Err(ssh_failure(
        step,
        classify(false, out.timed_out, false, &out.stderr),
        app_owned,
    ))
}

fn ssh_failure(step: Step, class: SshFailure, app_owned: bool) -> ProvisionFailure {
    if app_owned && class == SshFailure::AuthRefused {
        fail(step, FailureCause::AuthRefused)
    } else {
        fail(step, FailureCause::Ssh(class))
    }
}

fn io_fail(step: Step, e: std::io::Error) -> ProvisionFailure {
    if e.kind() == std::io::ErrorKind::Interrupted {
        return fail(step, FailureCause::Cancelled);
    }
    fail(step, FailureCause::Ssh(io_failure(e)))
}

/// Whether a failed `ssh true` means the server is not up yet.
fn not_up_yet(stderr: &str) -> bool {
    [
        "Connection refused",
        "Connection reset",
        "kex_exchange_identification",
    ]
    .iter()
    .any(|n| stderr.contains(n))
}

async fn reach_step(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    app_owned: bool,
) -> Result<(), ProvisionFailure> {
    // A user's host has no server to wait for: the probe is its first connection.
    if !app_owned {
        return Ok(());
    }
    let bound = target.bounds.reach;
    let started = Instant::now();
    let saw_not_up = std::sync::atomic::AtomicBool::new(false);
    let result = guard(Step::Reach, Some(bound), cancel, async {
        loop {
            let left = bound.saturating_sub(started.elapsed());
            let cmd = argv::reach_check(&target.reach, opts.platform, opts.timeout_secs);
            let out = host
                .run(cmd, Vec::new(), Some(left), None)
                .await
                .map_err(|e| io_fail(Step::Reach, e))?;
            if out.ok() {
                return Ok(());
            }
            if out.timed_out || !not_up_yet(&out.stderr) {
                return Err(checked(Step::Reach, out, Some(bound), app_owned).unwrap_err());
            }
            saw_not_up.store(true, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(REACH_RETRY).await;
        }
    })
    .await;
    match result {
        Err(ProvisionFailure {
            cause: FailureCause::TimedOut { .. },
            ..
        }) if saw_not_up.load(std::sync::atomic::Ordering::SeqCst) => {
            Err(fail(Step::Reach, FailureCause::NoSshServer))
        }
        other => other,
    }
}

async fn probe_step(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    app_owned: bool,
) -> Result<Probe, ProvisionFailure> {
    let bound = target.bounds.probe;
    guard(Step::Probe, Some(bound), cancel, async {
        let cmd = argv::probe(&target.reach, opts.platform, opts.timeout_secs);
        let script = argv::probe_script(&opts.build_id).into_bytes();
        let out = host
            .run(cmd, script, Some(bound), None)
            .await
            .map_err(|e| io_fail(Step::Probe, e))?;
        let out = checked(Step::Probe, out, Some(bound), app_owned)?;
        parse_probe(&String::from_utf8_lossy(&out.stdout)).map_err(|raw| match raw {
            Some((os, arch)) => fail(
                Step::Probe,
                FailureCause::Ssh(SshFailure::RemoteUnsupported { os, arch }),
            ),
            None => ssh_failure(
                Step::Probe,
                classify(false, false, true, &out.stderr),
                app_owned,
            ),
        })
    })
    .await
}

/// Reach, probe and copy (when absent): everything before the AI CLIs and the daemon. Returns the
/// probe that confirmed the build and whether this call copied it.
pub async fn ensure_build(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    sink: &EventSink,
) -> Result<(Probe, bool), ProvisionFailure> {
    let app_owned = matches!(target.owned, Ownership::AppOwned { .. });
    sink(ProvisionEvent::Entered(Step::Reach));
    reach_step(host, target, opts, cancel, app_owned).await?;

    sink(ProvisionEvent::Entered(Step::Probe));
    let probe = probe_step(host, target, opts, cancel, app_owned).await?;
    if !probe.home_writable {
        return Err(fail(
            Step::Probe,
            FailureCause::Ssh(SshFailure::RemoteNotWritable),
        ));
    }
    if probe.have {
        return Ok((probe, false));
    }

    let bytes = match select_artefact(opts.platform, &opts.client_arch, &probe.os, &probe.arch) {
        ArtefactChoice::Own => host.own().map_err(|e| io_fail(Step::Probe, e))?,
        ArtefactChoice::Bundled => host.bundled(&probe.os, &probe.arch).ok_or_else(|| {
            fail(
                Step::Probe,
                FailureCause::NoMatchingBuild {
                    os: probe.os.clone(),
                    arch: probe.arch.clone(),
                },
            )
        })?,
    };

    sink(ProvisionEvent::Entered(Step::Copy));
    let bound = target.bounds.copy;
    guard(Step::Copy, bound, cancel, async {
        let cmd = argv::copy(
            &target.reach,
            opts.platform,
            opts.timeout_secs,
            &opts.build_id,
        );
        let progress_sink = sink.clone();
        let progress: crate::ssh::ProvisionProgress =
            Arc::new(move |p| progress_sink(ProvisionEvent::CopyPercent(p)));
        let out = host
            .run(cmd, bytes, bound, Some(progress))
            .await
            .map_err(|e| io_fail(Step::Copy, e))?;
        if out.ok() {
            return Ok(());
        }
        if let (true, Some(bound)) = (out.timed_out, bound) {
            return Err(fail(Step::Copy, FailureCause::TimedOut { bound }));
        }
        let cause = match classify(false, out.timed_out, false, &out.stderr) {
            SshFailure::Other { .. } => FailureCause::CopyFailed,
            known => FailureCause::Ssh(known),
        };
        Err(fail(Step::Copy, cause).with_output(out.stderr))
    })
    .await?;

    let again = probe_step(host, target, opts, cancel, app_owned).await?;
    if !again.have {
        return Err(fail(Step::Copy, FailureCause::CopyFailed));
    }
    Ok((again, true))
}

async fn start_step(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
) -> Result<Connected, ProvisionFailure> {
    let app_owned = matches!(target.owned, Ownership::AppOwned { .. });
    let bound = target.bounds.start;
    guard(Step::Start, Some(bound), cancel, async {
        if app_owned {
            let cmd = argv::record_last(
                &target.reach,
                opts.platform,
                opts.timeout_secs,
                &opts.build_id,
            );
            let out = host
                .run(cmd, Vec::new(), Some(bound), None)
                .await
                .map_err(|e| io_fail(Step::Start, e))?;
            if !out.ok() {
                // The image's entrypoint cannot start the daemon unattended without this record.
                return Err(fail(Step::Start, FailureCause::DaemonStart).with_output(out.stderr));
            }
        }
        let cmd = argv::bridge_with_env(
            &target.reach,
            opts.platform,
            opts.timeout_secs,
            &opts.build_id,
            &target.env,
        );
        let stdio = host.open(cmd).map_err(|e| io_fail(Step::Start, e))?;
        let tail = stdio.stderr_tail();
        let credentials = Credentials::default();
        match handshake_with(Transport::Ssh(stdio), &opts.client_build, &credentials).await {
            Ok(connected) => Ok(connected),
            Err(e) => {
                // Give the child a moment to write why it exited before reading its stderr.
                tokio::time::sleep(Duration::from_millis(50)).await;
                let text = tail.text();
                let text = if text.trim().is_empty() {
                    e.to_string()
                } else {
                    text
                };
                Err(if app_owned {
                    fail(Step::Start, FailureCause::DaemonStart).with_output(text)
                } else {
                    fail(
                        Step::Start,
                        FailureCause::Ssh(classify(false, false, true, &text)),
                    )
                })
            }
        }
    })
    .await
}

/// The CLIs of `selected` the table has a row for, in the table's order.
fn selected_rows<'a>(
    table: &'a AiCliTable,
    selected: &AiCliSelection,
) -> Vec<&'a ai_cli::AiCliRow> {
    table
        .rows()
        .iter()
        .filter(|r| selected.0.contains(&r.cli))
        .collect()
}

/// Step 4: install the selected AI CLIs the target lacks, under one bound for the check and the
/// installs together. Entered only when something is missing; a CLI that fails to install is a
/// warning, not a failure (FR-012).
#[allow(clippy::too_many_arguments)]
async fn install_step(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    sink: &EventSink,
    selected: &AiCliSelection,
    table: &AiCliTable,
    state_dir: &std::path::Path,
    egress: &Egress,
) -> Result<Vec<ProvisionWarning>, ProvisionFailure> {
    install_step_with(
        host,
        target,
        opts,
        cancel,
        sink,
        (selected, table, state_dir, egress),
        relay::Relay::start,
    )
    .await
}

/// [`install_step`] with the relay's starter given, so a test can make it fail to bind.
async fn install_step_with<F, Fut>(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    sink: &EventSink,
    (selected, table, state_dir, egress): (&AiCliSelection, &AiCliTable, &std::path::Path, &Egress),
    start_relay: F,
) -> Result<Vec<ProvisionWarning>, ProvisionFailure>
where
    F: FnOnce(Vec<HostPort>) -> Fut,
    Fut: std::future::Future<Output = std::io::Result<relay::Relay>>,
{
    let step = Step::InstallAiClis;
    // An empty selection still runs the check: it rewrites `offered.json` to offer nothing, so a
    // daemon whose selection was emptied after an install stops offering the old file's CLIs.
    let rows = selected_rows(table, selected);
    let bound = target.bounds.install;
    guard(step, Some(bound), cancel, async {
        // What is already there? Nothing missing: no step, no download (invariant 1).
        let cmd = argv::install(&target.reach, opts.platform, opts.timeout_secs, None);
        let script = ai_cli::check_script(&rows, state_dir).into_bytes();
        let out = host
            .run(cmd, script, Some(bound), None)
            .await
            .map_err(|e| io_fail(step, e))?;
        let out = checked(step, out, Some(bound), true)?;
        let have = ai_cli::parse_installed(&String::from_utf8_lossy(&out.stdout));
        let missing: Vec<&ai_cli::AiCliRow> = rows
            .iter()
            .copied()
            .filter(|r| !have.contains(&r.cli))
            .collect();
        if missing.is_empty() {
            return Ok(Vec::new());
        }
        sink(ProvisionEvent::Entered(step));

        // Held until the command ends: dropping it stops the relay.
        let relay = match egress {
            Egress::Direct => None,
            Egress::Relay { allow } => Some(
                start_relay(allow.clone())
                    .await
                    .map_err(|_| fail(step, FailureCause::PostureBlocksDownload))?,
            ),
        };
        let cmd = argv::install(
            &target.reach,
            opts.platform,
            opts.timeout_secs,
            relay.as_ref().map(relay::Relay::port),
        );
        let script = ai_cli::install_script(&rows, &missing, state_dir, relay.is_some());
        let out = host
            .run(cmd, script.into_bytes(), Some(bound), None)
            .await
            .map_err(|e| io_fail(step, e))?;
        drop(relay);
        let out = checked(step, out, Some(bound), true)?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        for line in stdout.lines().filter(|l| !l.starts_with("MICOLD_INSTALL ")) {
            sink(ProvisionEvent::InstallLine(line.to_string()));
        }
        let results = ai_cli::parse_install_output(&stdout);
        let mut warnings = Vec::new();
        for row in &missing {
            let found = results.iter().find(|r| r.cli == row.cli);
            let ok = found.is_some_and(|r| r.ok);
            sink(ProvisionEvent::InstallDone { cli: row.cli, ok });
            if !ok {
                let warning = ProvisionWarning {
                    cli: row.cli,
                    output: found.map_or_else(
                        || "the installer reported no result".to_string(),
                        |r| r.output.clone(),
                    ),
                };
                sink(ProvisionEvent::Warned(warning.clone()));
                warnings.push(warning);
            }
        }
        Ok(warnings)
    })
    .await
}

/// The whole sequence: [`ensure_build`], then the daemon. A refused handshake is returned as
/// `Ok` with [`Connected::Refused`] (after `Entered(VersionCheck)`): explaining it, and replacing
/// the daemon only when no session runs, is the caller's (489 FR-011).
pub async fn run(
    host: &dyn SshHost,
    target: &ProvisionTarget,
    opts: &ProvisionOptions,
    cancel: &CancelToken,
    sink: &EventSink,
) -> Result<Provisioned, ProvisionFailure> {
    let (_, copied) = ensure_build(host, target, opts, cancel, sink).await?;
    // The AI CLIs are installed on app-owned targets only: a user's host never enters the step.
    let warnings = match &target.owned {
        Ownership::AppOwned {
            ai_clis,
            table,
            state_dir,
            egress,
        } => {
            install_step(
                host, target, opts, cancel, sink, ai_clis, table, state_dir, egress,
            )
            .await?
        }
        Ownership::UserHost => Vec::new(),
    };
    sink(ProvisionEvent::Entered(Step::Start));
    let connected = start_step(host, target, opts, cancel).await?;
    if matches!(connected, Connected::Refused(_)) {
        sink(ProvisionEvent::Entered(Step::VersionCheck));
    }
    Ok(Provisioned {
        connected,
        warnings,
        copied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codec::{DaemonCodec, Frame};
    use crate::protocol::messages::{DaemonMsg, RefusalReason};
    use crate::ssh::argv::ClientPlatform;
    use crate::ssh::{Artefacts, SshCommand, SshRunner, SshStdio, SshTarget};
    use futures_util::future::BoxFuture;
    use futures_util::{SinkExt, StreamExt};
    use std::collections::{HashMap, VecDeque};
    use std::io;
    use std::sync::Mutex;
    use tokio_util::codec::Framed;

    const PROBE_HAVE: &str =
        "MICOLD_PROBE os=Linux arch=x86_64 home_writable=1 have=1 dir=/h/.micold/remote/b\n";
    const PROBE_ABSENT: &str =
        "MICOLD_PROBE os=Linux arch=x86_64 home_writable=1 have=0 dir=/h/.micold/remote/b\n";

    /// Which kind of command this is, from what it runs and what it is fed.
    fn kind(cmd: &SshCommand, stdin: &[u8]) -> &'static str {
        let last = cmd.args.last().map(String::as_str).unwrap_or_default();
        if last == "true" {
            "reach"
        } else if last.contains("cat >") {
            "copy"
        } else if last.contains("printf") {
            "last"
        } else if last.contains("remote bridge") {
            "bridge"
        } else if String::from_utf8_lossy(stdin).contains("MICOLD_PROBE") {
            "probe"
        } else if String::from_utf8_lossy(stdin).contains("MICOLD_HAVE_CLI") {
            "have_clis"
        } else {
            "install"
        }
    }

    /// A host that answers each kind of command from its own script and records the order.
    #[derive(Default)]
    struct Fake {
        script: Mutex<HashMap<&'static str, VecDeque<io::Result<SshOutput>>>>,
        seen: Mutex<Vec<&'static str>>,
        /// Every `run`, with its command line and stdin.
        runs: Mutex<Vec<(&'static str, SshCommand, Vec<u8>)>>,
        bridge: Mutex<Option<SshStdio>>,
        percent: Vec<u8>,
        hang: Option<&'static str>,
    }

    impl Fake {
        fn say(self, kind: &'static str, out: io::Result<SshOutput>) -> Self {
            self.script
                .lock()
                .unwrap()
                .entry(kind)
                .or_default()
                .push_back(out);
            self
        }
        fn kinds(&self) -> Vec<&'static str> {
            self.seen.lock().unwrap().clone()
        }
        /// The command line and stdin of the first `run` of `kind`.
        fn first(&self, kind: &str) -> (SshCommand, String) {
            let runs = self.runs.lock().unwrap();
            let (_, cmd, stdin) = runs.iter().find(|r| r.0 == kind).expect("ran");
            (cmd.clone(), String::from_utf8_lossy(stdin).into_owned())
        }
        fn count(&self, kind: &str) -> usize {
            self.kinds().iter().filter(|k| **k == kind).count()
        }
    }

    impl SshRunner for Fake {
        fn run(
            &self,
            cmd: SshCommand,
            stdin: Vec<u8>,
            _deadline: Option<Duration>,
            progress: Option<crate::ssh::ProvisionProgress>,
        ) -> BoxFuture<'_, io::Result<SshOutput>> {
            let k = kind(&cmd, &stdin);
            self.seen.lock().unwrap().push(k);
            self.runs
                .lock()
                .unwrap()
                .push((k, cmd.clone(), stdin.clone()));
            if k == "copy" {
                for p in &self.percent {
                    if let Some(f) = &progress {
                        f(*p);
                    }
                }
            }
            let hang = self.hang == Some(k);
            let next = self
                .script
                .lock()
                .unwrap()
                .get_mut(k)
                .and_then(VecDeque::pop_front);
            Box::pin(async move {
                if hang {
                    std::future::pending::<()>().await;
                }
                next.unwrap_or_else(|| panic!("unscripted {k}"))
            })
        }
        fn open(&self, cmd: SshCommand) -> io::Result<SshStdio> {
            self.seen.lock().unwrap().push(kind(&cmd, b""));
            self.bridge
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| io::Error::other("no bridge"))
        }
    }

    impl Artefacts for Fake {
        fn own(&self) -> io::Result<Vec<u8>> {
            Ok(vec![7; 10])
        }
        fn bundled(&self, _: &str, _: &str) -> Option<Vec<u8>> {
            None
        }
    }

    fn ok(stdout: &str) -> io::Result<SshOutput> {
        Ok(SshOutput {
            status: Some(0),
            stdout: stdout.as_bytes().to_vec(),
            ..SshOutput::default()
        })
    }

    fn bad(stderr: &str) -> io::Result<SshOutput> {
        Ok(SshOutput {
            status: Some(255),
            stderr: stderr.into(),
            ..SshOutput::default()
        })
    }

    fn opts() -> ProvisionOptions {
        ProvisionOptions {
            platform: ClientPlatform::Linux,
            client_arch: "x86_64".into(),
            timeout_secs: 15,
            build_id: "1.0.0-abcdef123456".into(),
            client_build: "test".into(),
        }
    }

    fn user_target(bounds: StepBounds) -> ProvisionTarget {
        ProvisionTarget {
            reach: Reach::UserConfig(SshTarget::new("host", None, None, None).unwrap()),
            owned: Ownership::UserHost,
            bounds,
            env: Vec::new(),
        }
    }

    /// An app-owned target with nothing selected: the install step installs nothing.
    fn owned_target(bounds: StepBounds) -> ProvisionTarget {
        owned_with(bounds, AiCliSelection(Default::default()), Egress::Direct)
    }

    fn owned_with(bounds: StepBounds, ai_clis: AiCliSelection, egress: Egress) -> ProvisionTarget {
        ProvisionTarget {
            reach: Reach::Pinned(argv::PinnedReach {
                host: "127.0.0.1".into(),
                port: 40222,
                user: "dev".into(),
                key: "/k/client".into(),
                known_hosts: "/k/known_hosts".into(),
            }),
            owned: Ownership::AppOwned {
                ai_clis,
                table: AiCliTable::pinned(),
                state_dir: "/var/lib/micold-ai-ide".into(),
                egress,
            },
            bounds,
            env: Vec::new(),
        }
    }

    fn recorder() -> (EventSink, Arc<Mutex<Vec<ProvisionEvent>>>) {
        let log: Arc<Mutex<Vec<ProvisionEvent>>> = Arc::default();
        let l = log.clone();
        (Arc::new(move |e| l.lock().unwrap().push(e)), log)
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// A bridge whose far end refuses the handshake as a version mismatch would.
    fn refusing_bridge() -> SshStdio {
        let (near, far) = tokio::io::duplex(4096);
        tokio::spawn(async move {
            let mut daemon = Framed::new(far, DaemonCodec::new());
            let _hello = daemon.next().await;
            let reason = RefusalReason::NotPermitted {
                detail: "no".into(),
            };
            let _ = daemon
                .send(Frame::Control(DaemonMsg::Refused { reason }))
                .await;
            let _ = daemon.next().await;
        });
        let (r, w) = tokio::io::split(near);
        SshStdio::from_parts(r, w)
    }

    async fn run_with(
        fake: &Fake,
        target: &ProvisionTarget,
        cancel: &CancelToken,
    ) -> (Result<Provisioned, ProvisionFailure>, Vec<ProvisionEvent>) {
        let (sink, log) = recorder();
        let result = run(fake, target, &opts(), cancel, &sink).await;
        let events = log.lock().unwrap().clone();
        (result, events)
    }

    fn entered(events: &[ProvisionEvent]) -> Vec<Step> {
        events
            .iter()
            .filter_map(|e| match e {
                ProvisionEvent::Entered(s) => Some(*s),
                _ => None,
            })
            .collect()
    }

    fn failure(r: Result<Provisioned, ProvisionFailure>) -> ProvisionFailure {
        match r {
            Ok(_) => panic!("expected a failure"),
            Err(f) => f,
        }
    }

    #[tokio::test]
    async fn steps_are_entered_in_contract_order_and_a_present_build_is_not_copied() {
        let mut fake = Fake::default()
            .say("reach", ok(""))
            .say("probe", ok(PROBE_HAVE))
            .say("install", ok(""))
            .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let (result, events) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        let done = result.unwrap();
        assert!(!done.copied && done.warnings.is_empty());
        assert!(matches!(done.connected, Connected::Refused(_)));
        assert_eq!(
            entered(&events),
            [Step::Reach, Step::Probe, Step::Start, Step::VersionCheck]
        );
        assert_eq!(fake.count("copy"), 0, "invariant 1");
        assert_eq!(
            fake.kinds(),
            ["reach", "probe", "install", "last", "bridge"],
            "an empty selection still runs the offered.json check"
        );
    }

    #[tokio::test]
    async fn an_absent_build_is_copied_once_with_progress_then_confirmed() {
        let mut fake = Fake {
            percent: vec![10, 100],
            ..Fake::default()
        }
        .say("reach", ok(""))
        .say("probe", ok(PROBE_ABSENT))
        .say("copy", ok(""))
        .say("probe", ok(PROBE_HAVE))
        .say("install", ok(""))
        .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let (result, events) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        assert!(result.unwrap().copied);
        assert_eq!(
            entered(&events)[..3],
            [Step::Reach, Step::Probe, Step::Copy]
        );
        let pct: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProvisionEvent::CopyPercent(_)))
            .collect();
        assert_eq!(
            pct,
            [
                &ProvisionEvent::CopyPercent(10),
                &ProvisionEvent::CopyPercent(100)
            ]
        );
    }

    #[tokio::test]
    async fn a_user_host_never_installs_nor_records_the_build_nor_runs_the_reach_command() {
        let mut fake = Fake::default().say("probe", ok(PROBE_HAVE));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let (result, events) = run_with(
            &fake,
            &user_target(StepBounds::user_host(secs(15))),
            &CancelToken::new(),
        )
        .await;
        assert!(result.is_ok());
        assert!(!entered(&events).contains(&Step::InstallAiClis));
        assert_eq!(
            fake.kinds(),
            ["probe", "bridge"],
            "no install, no last, no true"
        );
        assert_eq!(entered(&events)[..2], [Step::Reach, Step::Probe]);
    }

    #[tokio::test]
    async fn reach_retries_only_while_the_server_is_not_up_then_gives_no_ssh_server() {
        let fake = Fake::default()
            .say(
                "reach",
                bad("ssh: connect to host 127.0.0.1 port 40222: Connection refused"),
            )
            .say(
                "reach",
                bad("kex_exchange_identification: read: Connection reset by peer"),
            )
            .say("reach", ok(""))
            .say("probe", ok(PROBE_HAVE))
            .say("install", ok(""))
            .say("last", ok(""));
        // Not up twice, then up: the start gets as far as the bridge, which is not scripted.
        let (result, _) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        assert_eq!(failure(result).step, Step::Start);
        assert_eq!(fake.count("reach"), 3);

        let mut bounds = StepBounds::container();
        bounds.reach = secs(2);
        let mut fake = Fake::default();
        for _ in 0..10 {
            fake = fake.say(
                "reach",
                bad("ssh: connect to host 127.0.0.1 port 1: Connection refused"),
            );
        }
        let (result, _) = run_with(&fake, &owned_target(bounds), &CancelToken::new()).await;
        let f = failure(result);
        assert_eq!((f.step, f.cause), (Step::Reach, FailureCause::NoSshServer));
        assert!(fake.count("reach") >= 2);
    }

    #[tokio::test]
    async fn any_other_reach_failure_is_final_and_a_refused_key_is_auth_refused() {
        let fake = Fake::default().say(
            "reach",
            bad("dev@127.0.0.1: Permission denied (publickey)."),
        );
        let (result, _) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        let f = failure(result);
        assert_eq!((f.step, f.cause), (Step::Reach, FailureCause::AuthRefused));
        assert_eq!(fake.count("reach"), 1);

        let fake = Fake::default().say("probe", bad("Permission denied (publickey)."));
        let (result, _) = run_with(
            &fake,
            &user_target(StepBounds::user_host(secs(15))),
            &CancelToken::new(),
        )
        .await;
        assert_eq!(
            failure(result).cause,
            FailureCause::Ssh(SshFailure::AuthRefused)
        );
    }

    #[tokio::test]
    async fn a_step_that_outlives_its_bound_is_timed_out_naming_it() {
        let mut bounds = StepBounds::container();
        bounds.probe = secs(1);
        let fake = Fake {
            hang: Some("probe"),
            ..Fake::default()
        }
        .say("reach", ok(""));
        let (result, _) = run_with(&fake, &owned_target(bounds), &CancelToken::new()).await;
        let f = failure(result);
        assert_eq!(
            (f.step, &f.cause),
            (Step::Probe, &FailureCause::TimedOut { bound: secs(1) })
        );
        assert!(f
            .reason(&crate::ssh::ReasonContext {
                host: "h".into(),
                port: 1,
                timeout_secs: 1,
                platform: ClientPlatform::Linux
            })
            .ends_with("did not finish within 1 second"));
    }

    #[tokio::test]
    async fn a_cancelled_token_stops_before_or_during_a_step() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let fake = Fake::default();
        let (result, events) =
            run_with(&fake, &owned_target(StepBounds::container()), &cancel).await;
        assert_eq!(failure(result).cause, FailureCause::Cancelled);
        assert!(fake.kinds().is_empty());
        assert_eq!(entered(&events), [Step::Reach]);

        let fake = Fake {
            hang: Some("probe"),
            ..Fake::default()
        }
        .say("reach", ok(""));
        let cancel = CancelToken::new();
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(secs(1)).await;
            c.cancel();
        });
        let (result, _) = run_with(&fake, &owned_target(StepBounds::container()), &cancel).await;
        let f = failure(result);
        assert_eq!((f.step, f.cause), (Step::Probe, FailureCause::Cancelled));
    }

    #[tokio::test]
    async fn no_matching_build_a_dead_bridge_and_a_vanished_copy_each_have_their_cause() {
        let fake = Fake::default().say("reach", ok("")).say(
            "probe",
            ok("MICOLD_PROBE os=Darwin arch=arm64 home_writable=1 have=0 dir=/x\n"),
        );
        let (result, _) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        let f = failure(result);
        assert_eq!(
            (f.step, f.cause),
            (
                Step::Probe,
                FailureCause::NoMatchingBuild {
                    os: "macos".into(),
                    arch: "aarch64".into()
                }
            )
        );

        let mut fake = Fake::default()
            .say("reach", ok(""))
            .say("probe", ok(PROBE_HAVE))
            .say("install", ok(""))
            .say("last", ok(""));
        let (near, far) = tokio::io::duplex(64);
        drop(far);
        let (r, w) = tokio::io::split(near);
        *fake.bridge.get_mut().unwrap() = Some(SshStdio::from_parts(r, w));
        let (result, _) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        let f = failure(result);
        assert_eq!((f.step, f.cause), (Step::Start, FailureCause::DaemonStart));

        let fake = Fake::default()
            .say("reach", ok(""))
            .say("probe", ok(PROBE_ABSENT))
            .say("copy", ok(""))
            .say("probe", ok(PROBE_ABSENT));
        let (result, _) = run_with(
            &fake,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        assert_eq!(failure(result).cause, FailureCause::CopyFailed);
    }

    /// US6 #1, invariant 3: one failing copy, two kinds of target, the same story.
    #[tokio::test]
    async fn a_failed_copy_reads_the_same_for_a_user_host_and_an_app_owned_target() {
        let script = |f: Fake| {
            f.say("probe", ok(PROBE_ABSENT)).say(
                "copy",
                bad("cat: /h/.micold/remote/b/.part: No space left on device"),
            )
        };
        let user = script(Fake {
            percent: vec![50],
            ..Fake::default()
        });
        let owned = script(Fake {
            percent: vec![50],
            ..Fake::default()
        })
        .say("reach", ok(""));
        let (u, ue) = run_with(
            &user,
            &user_target(StepBounds::user_host(secs(15))),
            &CancelToken::new(),
        )
        .await;
        let (o, oe) = run_with(
            &owned,
            &owned_target(StepBounds::container()),
            &CancelToken::new(),
        )
        .await;
        assert_eq!(ue, oe, "same steps and CopyPercent events");
        assert_eq!(entered(&ue), [Step::Reach, Step::Probe, Step::Copy]);
        let ctx = crate::ssh::ReasonContext {
            host: "h".into(),
            port: 22,
            timeout_secs: 15,
            platform: ClientPlatform::Linux,
        };
        let (u, o) = (failure(u), failure(o));
        assert_eq!(u.cause, FailureCause::CopyFailed);
        assert_eq!(u.reason(&ctx), o.reason(&ctx));
        assert!(u.reason(&ctx).starts_with(
            "copying the daemon failed: the daemon build could not be written in the target"
        ));
    }

    const HAVE_CLAUDE: &str = "MICOLD_HAVE_CLI ClaudeCode\n";

    fn claude_only() -> AiCliSelection {
        AiCliSelection::new_daemon()
    }

    /// A target and host that get as far as the install step with the build present.
    fn through_copy(fake: Fake) -> Fake {
        fake.say("reach", ok("")).say("probe", ok(PROBE_HAVE))
    }

    #[tokio::test]
    async fn missing_cli_are_installed_after_the_copy_and_before_the_start_with_progress() {
        let mut fake = Fake {
            percent: vec![100],
            ..Fake::default()
        }
        .say("reach", ok(""))
        .say("probe", ok(PROBE_ABSENT))
        .say("copy", ok(""))
        .say("probe", ok(PROBE_HAVE))
        .say("have_clis", ok(""))
        .say(
            "install",
            ok("added 3 packages\nMICOLD_INSTALL ClaudeCode ok\n"),
        )
        .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let target = owned_with(StepBounds::container(), claude_only(), Egress::Direct);
        let (result, events) = run_with(&fake, &target, &CancelToken::new()).await;
        assert!(result.unwrap().warnings.is_empty());
        assert_eq!(
            entered(&events),
            [
                Step::Reach,
                Step::Probe,
                Step::Copy,
                Step::InstallAiClis,
                Step::Start,
                Step::VersionCheck
            ]
        );
        assert!(events.contains(&ProvisionEvent::InstallLine("added 3 packages".into())));
        assert!(events.contains(&ProvisionEvent::InstallDone {
            cli: crate::session::AiCli::ClaudeCode,
            ok: true
        }));
        let (cmd, _) = fake.first("install");
        assert!(
            cmd.args.contains(&"ClearAllForwardings=yes".to_string()),
            "Direct egress forwards nothing"
        );
    }

    /// FR-012: a CLI that fails is a warning naming it, and the start still succeeds.
    #[tokio::test]
    async fn a_failed_install_is_a_warning_and_the_start_still_succeeds() {
        let mut fake = through_copy(Fake::default())
            .say("have_clis", ok(""))
            .say(
                "install",
                ok("npm ERR! 404 Not Found\nMICOLD_INSTALL ClaudeCode failed\n"),
            )
            .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let target = owned_with(StepBounds::container(), claude_only(), Egress::Direct);
        let (result, events) = run_with(&fake, &target, &CancelToken::new()).await;
        let done = result.unwrap();
        assert_eq!(
            done.warnings,
            [ProvisionWarning {
                cli: crate::session::AiCli::ClaudeCode,
                output: "npm ERR! 404 Not Found".into()
            }]
        );
        assert!(events.contains(&ProvisionEvent::InstallDone {
            cli: crate::session::AiCli::ClaudeCode,
            ok: false
        }));
        assert!(entered(&events).contains(&Step::Start));
    }

    /// Invariant 1: a second start with everything installed downloads nothing and never enters
    /// the step.
    #[tokio::test]
    async fn nothing_missing_skips_the_step_and_runs_no_install() {
        let mut fake = through_copy(Fake::default())
            .say("have_clis", ok(HAVE_CLAUDE))
            .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let target = owned_with(
            StepBounds::container(),
            claude_only(),
            Egress::Relay {
                allow: AiCliTable::pinned().allow(),
            },
        );
        let (result, events) = run_with(&fake, &target, &CancelToken::new()).await;
        assert!(result.is_ok());
        assert!(!entered(&events).contains(&Step::InstallAiClis));
        assert_eq!(fake.count("install"), 0);
        assert_eq!(
            fake.kinds(),
            ["reach", "probe", "have_clis", "last", "bridge"]
        );
    }

    /// A selection emptied after an install must not keep the old `offered.json`: the check still
    /// runs and rewrites it to offer nothing, with no step entered and no download.
    #[tokio::test]
    async fn an_empty_selection_rewrites_offered_json_to_offer_nothing() {
        let mut fake = through_copy(Fake::default())
            .say("install", ok(""))
            .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let target = owned_target(StepBounds::container());
        let (result, events) = run_with(&fake, &target, &CancelToken::new()).await;
        assert!(result.unwrap().warnings.is_empty());
        assert!(!entered(&events).contains(&Step::InstallAiClis));
        let (_, script) = fake.first("install");
        assert!(script.contains("{\"version\":1,\"clis\":["), "{script}");
        assert!(script.contains("offered.json"), "{script}");
        assert!(!script.contains("flock"), "no install ran: {script}");
    }

    #[tokio::test]
    async fn an_install_that_outlives_its_bound_is_timed_out_on_the_install_step() {
        let mut bounds = StepBounds::container();
        bounds.install = secs(1);
        let fake = through_copy(Fake {
            hang: Some("install"),
            ..Fake::default()
        })
        .say("have_clis", ok(""));
        let target = owned_with(bounds, claude_only(), Egress::Direct);
        let (result, _) = run_with(&fake, &target, &CancelToken::new()).await;
        let f = failure(result);
        assert_eq!(
            (f.step, f.cause),
            (
                Step::InstallAiClis,
                FailureCause::TimedOut { bound: secs(1) }
            )
        );
    }

    #[tokio::test]
    async fn a_relay_install_forwards_the_relay_port_and_exports_the_proxy() {
        let mut fake = through_copy(Fake::default())
            .say("have_clis", ok(""))
            .say("install", ok("MICOLD_INSTALL ClaudeCode ok\n"))
            .say("last", ok(""));
        *fake.bridge.get_mut().unwrap() = Some(refusing_bridge());
        let target = owned_with(
            StepBounds::container(),
            claude_only(),
            Egress::Relay {
                allow: AiCliTable::pinned().allow(),
            },
        );
        let (result, _) = run_with(&fake, &target, &CancelToken::new()).await;
        assert!(result.is_ok());
        let (cmd, stdin) = fake.first("install");
        let at = cmd.args.iter().position(|a| a == "-R").expect("a forward");
        let forward = &cmd.args[at + 1];
        assert!(
            forward.starts_with("127.0.0.1:17728:127.0.0.1:"),
            "{forward}"
        );
        assert!(!cmd.args.contains(&"ClearAllForwardings=yes".to_string()));
        assert!(cmd.args.contains(&"ExitOnForwardFailure=yes".to_string()));
        assert!(stdin.contains("export HTTPS_PROXY=http://127.0.0.1:17728"));
        // The check never carries a forward.
        let (check, _) = fake.first("have_clis");
        assert!(!check.args.contains(&"-R".to_string()));
    }

    #[tokio::test]
    async fn a_relay_that_cannot_bind_blocks_the_download() {
        let fake = Fake::default().say("have_clis", ok(""));
        let target = owned_with(
            StepBounds::container(),
            claude_only(),
            Egress::Relay { allow: Vec::new() },
        );
        let Ownership::AppOwned {
            ai_clis,
            table,
            state_dir,
            egress,
        } = &target.owned
        else {
            unreachable!()
        };
        let (sink, _) = recorder();
        let err = install_step_with(
            &fake,
            &target,
            &opts(),
            &CancelToken::new(),
            &sink,
            (ai_clis, table, state_dir, egress),
            |_| async { Err(io::Error::from(io::ErrorKind::AddrInUse)) },
        )
        .await
        .unwrap_err();
        assert_eq!(
            (err.step, err.cause),
            (Step::InstallAiClis, FailureCause::PostureBlocksDownload)
        );
        assert_eq!(fake.count("install"), 0, "nothing runs without the relay");
    }

    #[test]
    fn bounds_and_the_default_selection_are_the_contracts() {
        let c = StepBounds::container();
        assert_eq!(c.acquire, secs(900));
        assert_eq!(
            [c.create_container, c.reach, c.probe, c.start, c.version],
            [secs(120); 5]
        );
        assert_eq!((c.copy, c.install), (Some(secs(120)), secs(600)));
        let u = StepBounds::user_host(secs(15));
        assert_eq!(
            (u.reach, u.probe, u.start, u.copy),
            (secs(15), secs(15), secs(15), None)
        );
        assert_eq!(
            AiCliSelection::new_daemon()
                .0
                .into_iter()
                .collect::<Vec<_>>(),
            [crate::session::AiCli::ClaudeCode]
        );
        assert_eq!(
            AiCliSelection::all().0.into_iter().collect::<Vec<_>>(),
            crate::session::AiCli::ALL
        );
    }
}

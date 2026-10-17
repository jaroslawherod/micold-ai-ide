//! The provisioning sequence over an injected [`SshRunner`]: probe, copy only when absent, re-probe,
//! open the bridge, handshake (contracts/ssh-transport.md, "Attempt sequence").
//!
//! The real runner (which spawns `ssh`) lives in the client's shell; everything here is testable
//! with a stand-in runner and no process (Constitution Principle I).

use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::BoxFuture;

use super::argv::{ClientPlatform, SshCommand};
use super::classify::SshFailure;
use super::stdio::SshStdio;
use super::target::SshTarget;
use crate::connect::Connected;
use crate::provision::{
    ensure_build, run, CancelToken, EventSink, FailureCause, ProvisionEvent, ProvisionFailure,
    ProvisionTarget,
};

/// Reports the copy's progress as a percentage, 0..=100.
pub type ProvisionProgress = Arc<dyn Fn(u8) + Send + Sync>;

/// What a finished one-shot `ssh` run produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshOutput {
    /// The exit code; `None` when the child was killed (the deadline) or had none.
    pub status: Option<i32>,
    /// Everything written to stdout.
    pub stdout: Vec<u8>,
    /// Everything written to stderr (lossy UTF-8).
    pub stderr: String,
    /// The watchdog killed the child at the deadline.
    pub timed_out: bool,
}

impl SshOutput {
    pub(crate) fn ok(&self) -> bool {
        self.status == Some(0) && !self.timed_out
    }
}

/// Runs `ssh` processes. The one real implementation spawns the user's `ssh` executable.
pub trait SshRunner: Send + Sync {
    /// Run to completion with `stdin` written, killing the child at `deadline`. `progress` is told
    /// how much of `stdin` has been written. An `io::ErrorKind::NotFound` error means `ssh` itself
    /// is missing.
    fn run(
        &self,
        cmd: SshCommand,
        stdin: Vec<u8>,
        deadline: Option<Duration>,
        progress: Option<ProvisionProgress>,
    ) -> BoxFuture<'_, io::Result<SshOutput>>;

    /// Start a long-lived child and hand back its stdio (the bridge). The child dies with it.
    fn open(&self, cmd: SshCommand) -> io::Result<SshStdio>;
}

/// The daemon builds this app can put on a remote host.
pub trait Artefacts: Send + Sync {
    /// This build's own daemon executable.
    fn own(&self) -> io::Result<Vec<u8>>;
    /// A bundled daemon for another platform, by `os` (`linux`, `macos`) and `arch`.
    fn bundled(&self, os: &str, arch: &str) -> Option<Vec<u8>>;
}

/// What the client needs to reach a host: a runner and the artefacts.
pub trait SshHost: SshRunner + Artefacts {}
impl<T: SshRunner + Artefacts> SshHost for T {}

/// What the remote probe reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// `linux` or `macos`.
    pub os: String,
    /// `x86_64` or `aarch64`.
    pub arch: String,
    /// Whether `~/.micold/remote` is writable.
    pub home_writable: bool,
    /// Whether this build's daemon is already there.
    pub have: bool,
    /// The build directory as the remote shell expanded it.
    pub dir: String,
}

/// Parse the probe's stdout. `Err` carries the raw `uname` values for an unsupported platform, or
/// `None` when the output was not a probe line at all.
pub fn parse_probe(stdout: &str) -> Result<Probe, Option<(String, String)>> {
    let line = stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("MICOLD_PROBE "))
        .ok_or(None)?;
    let field = |key: &str| -> Option<&str> {
        line.split_whitespace()
            .find_map(|t| t.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
    };
    let raw_os = field("os").unwrap_or("").to_string();
    let raw_arch = field("arch").unwrap_or("").to_string();
    let os = match raw_os.as_str() {
        "Linux" => "linux",
        "Darwin" => "macos",
        _ => return Err(Some((raw_os, raw_arch))),
    };
    let arch = match normalize_arch(&raw_arch) {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(Some((raw_os, raw_arch))),
    };
    let dir = line
        .split_once(" dir=")
        .map(|(_, d)| d.trim().to_string())
        .or_else(|| line.strip_prefix("dir=").map(str::to_string))
        .unwrap_or_default();
    Ok(Probe {
        os: os.to_string(),
        arch: arch.to_string(),
        home_writable: field("home_writable") == Some("1"),
        have: field("have") == Some("1"),
        dir,
    })
}

fn normalize_arch(arch: &str) -> &str {
    match arch {
        "arm64" => "aarch64",
        other => other,
    }
}

/// Which daemon build goes to a remote host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtefactChoice {
    /// This build's own executable: client and remote are the same platform.
    Own,
    /// A bundled build for the remote's platform.
    Bundled,
}

/// The own build when the platforms are equal, else a bundled one (resolved by the caller), so a
/// remote host is never sent a build it cannot run.
pub fn select_artefact(
    client: ClientPlatform,
    client_arch: &str,
    remote_os: &str,
    remote_arch: &str,
) -> ArtefactChoice {
    if client.os() == remote_os && normalize_arch(client_arch) == normalize_arch(remote_arch) {
        ArtefactChoice::Own
    } else {
        ArtefactChoice::Bundled
    }
}

/// What one attempt needs besides the host and the target.
#[derive(Debug, Clone)]
pub struct ProvisionOptions {
    /// The client's platform (command lines, hints).
    pub platform: ClientPlatform,
    /// The client's CPU architecture (`std::env::consts::ARCH`).
    pub client_arch: String,
    /// The connect deadline in seconds (FR-009).
    pub timeout_secs: u32,
    /// The remote build directory name ([`argv::current_build_id`]).
    pub build_id: String,
    /// The client's build string presented in `Hello`.
    pub client_build: String,
}

pub(crate) fn io_failure(e: io::Error) -> SshFailure {
    if e.kind() == io::ErrorKind::NotFound {
        SshFailure::SshMissing
    } else {
        SshFailure::Other {
            tail: e.to_string(),
        }
    }
}

/// The shared sequence's target for a user's host: the 489 bounds (the connect deadline for every
/// step, the copy unbounded).
fn user_host(target: &SshTarget, opts: &ProvisionOptions) -> ProvisionTarget {
    ProvisionTarget::user_host(target, Duration::from_secs(u64::from(opts.timeout_secs)))
}

/// A sink that forwards the copy's percentage and drops every other event.
fn progress_sink(progress: Option<ProvisionProgress>) -> EventSink {
    Arc::new(move |event| {
        if let (ProvisionEvent::CopyPercent(p), Some(progress)) = (event, &progress) {
            progress(p);
        }
    })
}

/// A shared-sequence failure as 489's class, so the callers keep their reason table.
fn ssh_failure_of(failure: ProvisionFailure) -> SshFailure {
    match failure.cause {
        FailureCause::Ssh(f) => f,
        FailureCause::TimedOut { .. } => SshFailure::Timeout,
        FailureCause::NoMatchingBuild { os, arch } => SshFailure::NoMatchingBuild { os, arch },
        FailureCause::CopyFailed => SshFailure::RemoteNotWritable,
        FailureCause::AuthRefused => SshFailure::AuthRefused,
        _ => SshFailure::HelperFailed {
            tail: failure
                .output
                .lines()
                .last()
                .unwrap_or_default()
                .to_string(),
        },
    }
}

/// Make sure this build's daemon is on the host, copying it only when absent. Returns the probe
/// that confirmed it. The shared sequence with a [`Ownership::UserHost`] target.
pub async fn provision(
    host: &dyn SshHost,
    target: &SshTarget,
    opts: &ProvisionOptions,
    progress: Option<ProvisionProgress>,
) -> Result<Probe, SshFailure> {
    let sink = progress_sink(progress);
    ensure_build(
        host,
        &user_host(target, opts),
        opts,
        &CancelToken::new(),
        &sink,
    )
    .await
    .map(|(probe, _)| probe)
    .map_err(ssh_failure_of)
}

/// Provision, open the bridge and handshake. `Connected::Refused` is returned as is: a version
/// mismatch is the caller's to explain, not a failure of the transport.
pub async fn connect(
    host: &dyn SshHost,
    target: &SshTarget,
    opts: &ProvisionOptions,
    progress: Option<ProvisionProgress>,
) -> Result<Connected, SshFailure> {
    let sink = progress_sink(progress);
    run(
        host,
        &user_host(target, opts),
        opts,
        &CancelToken::new(),
        &sink,
    )
    .await
    .map(|p| p.connected)
    .map_err(ssh_failure_of)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    fn probe_line(os: &str, arch: &str, writable: u8, have: u8) -> String {
        format!("MICOLD_PROBE os={os} arch={arch} home_writable={writable} have={have} dir=/home/u/.micold/remote/b\n")
    }

    #[test]
    fn probe_line_maps_uname_names_to_platform_names() {
        let p = parse_probe(&probe_line("Darwin", "arm64", 1, 0)).unwrap();
        assert_eq!((p.os.as_str(), p.arch.as_str()), ("macos", "aarch64"));
        assert!(p.home_writable && !p.have);
        assert_eq!(p.dir, "/home/u/.micold/remote/b");
        let l = parse_probe(&probe_line("Linux", "x86_64", 1, 1)).unwrap();
        assert_eq!((l.os.as_str(), l.arch.as_str()), ("linux", "x86_64"));
        assert!(l.have);
    }

    #[test]
    fn unsupported_platform_and_garbage_are_told_apart() {
        assert_eq!(
            parse_probe(&probe_line("FreeBSD", "amd64", 1, 0)),
            Err(Some(("FreeBSD".into(), "amd64".into())))
        );
        assert_eq!(parse_probe("hello\n"), Err(None));
    }

    #[test]
    fn artefact_table_over_client_and_remote_pairs() {
        use ArtefactChoice::{Bundled, Own};
        let cases = [
            (ClientPlatform::Linux, "x86_64", "linux", "x86_64", Own),
            (ClientPlatform::Linux, "x86_64", "linux", "aarch64", Bundled),
            (ClientPlatform::MacOs, "aarch64", "macos", "aarch64", Own),
            (ClientPlatform::MacOs, "aarch64", "linux", "x86_64", Bundled),
            (
                ClientPlatform::Windows,
                "x86_64",
                "linux",
                "x86_64",
                Bundled,
            ),
            (
                ClientPlatform::Windows,
                "x86_64",
                "macos",
                "aarch64",
                Bundled,
            ),
        ];
        for (c, ca, ro, ra, want) in cases {
            assert_eq!(
                select_artefact(c, ca, ro, ra),
                want,
                "{c:?} {ca} -> {ro} {ra}"
            );
        }
    }

    /// A runner that answers `run` from a script and records what it was asked.
    struct Fake {
        runs: Mutex<VecDeque<io::Result<SshOutput>>>,
        seen: Mutex<Vec<(Vec<String>, usize)>>,
        bridge: Mutex<Option<SshStdio>>,
        bundled: Option<Vec<u8>>,
    }

    impl Fake {
        fn new(runs: Vec<io::Result<SshOutput>>) -> Self {
            Self {
                runs: Mutex::new(runs.into()),
                seen: Mutex::default(),
                bridge: Mutex::default(),
                bundled: None,
            }
        }
        fn commands(&self) -> Vec<String> {
            self.seen
                .lock()
                .unwrap()
                .iter()
                .map(|(a, _)| a.last().cloned().unwrap_or_default())
                .collect()
        }
    }

    impl SshRunner for Fake {
        fn run(
            &self,
            cmd: SshCommand,
            stdin: Vec<u8>,
            _deadline: Option<Duration>,
            _progress: Option<ProvisionProgress>,
        ) -> BoxFuture<'_, io::Result<SshOutput>> {
            self.seen.lock().unwrap().push((cmd.args, stdin.len()));
            let next = self.runs.lock().unwrap().pop_front();
            Box::pin(async move { next.expect("unscripted run") })
        }
        fn open(&self, cmd: SshCommand) -> io::Result<SshStdio> {
            self.seen.lock().unwrap().push((cmd.args, 0));
            self.bridge
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| io::Error::other("no bridge scripted"))
        }
    }

    impl Artefacts for Fake {
        fn own(&self) -> io::Result<Vec<u8>> {
            Ok(vec![7; 10])
        }
        fn bundled(&self, _os: &str, _arch: &str) -> Option<Vec<u8>> {
            self.bundled.clone()
        }
    }

    fn out(stdout: String) -> io::Result<SshOutput> {
        Ok(SshOutput {
            status: Some(0),
            stdout: stdout.into_bytes(),
            ..SshOutput::default()
        })
    }

    fn failed(stderr: &str) -> io::Result<SshOutput> {
        Ok(SshOutput {
            status: Some(255),
            stderr: stderr.to_string(),
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

    fn target() -> SshTarget {
        SshTarget::new("build-host", None, None, None).unwrap()
    }

    #[test]
    fn the_wrappers_run_the_shared_sequence_for_a_user_host_with_489_bounds() {
        let t = user_host(&target(), &opts());
        assert_eq!(t.owned, crate::provision::Ownership::UserHost);
        let b = t.bounds;
        let d = Duration::from_secs(15);
        assert_eq!((b.reach, b.probe, b.start, b.copy), (d, d, d, None));
    }

    #[tokio::test]
    async fn present_daemon_is_not_copied() {
        let fake = Fake::new(vec![out(probe_line("Linux", "x86_64", 1, 1))]);
        provision(&fake, &target(), &opts(), None).await.unwrap();
        assert_eq!(fake.commands(), vec!["sh -s"]);
    }

    #[tokio::test]
    async fn absent_daemon_is_copied_once_then_confirmed() {
        let fake = Fake::new(vec![
            out(probe_line("Linux", "x86_64", 1, 0)),
            out(String::new()),
            out(probe_line("Linux", "x86_64", 1, 1)),
        ]);
        provision(&fake, &target(), &opts(), None).await.unwrap();
        let seen = fake.seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[1].1, 10, "the own build is what is copied");
        assert!(seen[1].0.last().unwrap().contains("cat >"));
    }

    #[tokio::test]
    async fn a_copy_that_leaves_nothing_is_not_writable() {
        let fake = Fake::new(vec![
            out(probe_line("Linux", "x86_64", 1, 0)),
            out(String::new()),
            out(probe_line("Linux", "x86_64", 1, 0)),
        ]);
        let err = provision(&fake, &target(), &opts(), None)
            .await
            .unwrap_err();
        assert_eq!(err, SshFailure::RemoteNotWritable);
    }

    #[tokio::test]
    async fn permanent_failures_stop_before_any_copy() {
        let fake = Fake::new(vec![failed(
            "@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@",
        )]);
        let err = provision(&fake, &target(), &opts(), None)
            .await
            .unwrap_err();
        assert_eq!(err, SshFailure::HostKeyChanged);
        assert!(err.is_permanent());
        assert_eq!(fake.commands().len(), 1);
    }

    #[tokio::test]
    async fn no_matching_build_is_named_with_the_remote_platform() {
        let fake = Fake::new(vec![out(probe_line("Darwin", "arm64", 1, 0))]);
        let err = provision(&fake, &target(), &opts(), None)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            SshFailure::NoMatchingBuild {
                os: "macos".into(),
                arch: "aarch64".into()
            }
        );
    }

    #[tokio::test]
    async fn bundled_build_is_copied_when_platforms_differ() {
        let mut fake = Fake::new(vec![
            out(probe_line("Linux", "aarch64", 1, 0)),
            out(String::new()),
            out(probe_line("Linux", "aarch64", 1, 1)),
        ]);
        fake.bundled = Some(vec![1; 3]);
        provision(&fake, &target(), &opts(), None).await.unwrap();
        assert_eq!(fake.seen.lock().unwrap()[1].1, 3);
    }

    #[tokio::test]
    async fn missing_ssh_is_named() {
        let fake = Fake::new(vec![Err(io::Error::from(io::ErrorKind::NotFound))]);
        let err = provision(&fake, &target(), &opts(), None)
            .await
            .unwrap_err();
        assert_eq!(err, SshFailure::SshMissing);
    }

    #[tokio::test]
    async fn read_only_home_is_named_before_copying() {
        let fake = Fake::new(vec![out(probe_line("Linux", "x86_64", 0, 0))]);
        let err = provision(&fake, &target(), &opts(), None)
            .await
            .unwrap_err();
        assert_eq!(err, SshFailure::RemoteNotWritable);
        assert_eq!(fake.commands().len(), 1);
    }

    #[tokio::test]
    async fn a_bridge_that_dies_before_the_handshake_is_a_helper_failure() {
        let fake = Fake::new(vec![out(probe_line("Linux", "x86_64", 1, 1))]);
        let (near, far) = tokio::io::duplex(64);
        drop(far);
        let (r, w) = tokio::io::split(near);
        *fake.bridge.lock().unwrap() = Some(SshStdio::from_parts(r, w));
        let result = connect(&fake, &target(), &opts(), None).await;
        assert!(matches!(result, Err(SshFailure::HelperFailed { .. })));
    }
}

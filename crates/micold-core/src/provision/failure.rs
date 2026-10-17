//! Steps, failure causes and their reason texts (contracts/provisioning.md, *Failure causes and
//! reason texts*; FR-018). One text per cause, the same for every target (US6 #1).

use std::time::Duration;

use crate::session::AiCli;
use crate::ssh::{ReasonContext, SshFailure};

/// How many trailing lines of a tool's output a reason carries.
pub const OUTPUT_LINES: usize = 40;

/// One start step. `Acquire` and `CreateContainer` are a container runtime's own, reported in the
/// same vocabulary so the row shows one list; the shared sequence starts at `Reach`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// Pull or load the image.
    Acquire,
    /// Create and start the container.
    CreateContainer,
    /// Reach the SSH server.
    Reach,
    /// Probe the target for the daemon build.
    Probe,
    /// Copy the daemon build.
    Copy,
    /// Install the selected AI CLIs.
    InstallAiClis,
    /// Start the daemon and handshake.
    Start,
    /// Check the daemon's version.
    VersionCheck,
}

impl Step {
    /// The label the row shows.
    pub fn label(self) -> &'static str {
        match self {
            Step::Acquire => "getting the image",
            Step::CreateContainer => "starting the container",
            Step::Reach => "connecting to the SSH server",
            Step::Probe => "checking the daemon build",
            Step::Copy => "copying the daemon",
            Step::InstallAiClis => "installing the AI CLIs",
            Step::Start => "starting the daemon",
            Step::VersionCheck => "checking the daemon version",
        }
    }
}

/// Why a step failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureCause {
    /// The container runtime could not pull or load the image.
    PullOrLoad {
        /// The image reference.
        image: String,
    },
    /// The container runtime could not start the container.
    ContainerStart,
    /// No SSH server came up within the bound.
    NoSshServer,
    /// The SSH server refused the daemon key.
    AuthRefused,
    /// The daemon build could not be written in the target.
    CopyFailed,
    /// This app has no daemon build for the target's platform.
    NoMatchingBuild {
        /// The target's OS.
        os: String,
        /// The target's architecture.
        arch: String,
    },
    /// The daemon did not start.
    DaemonStart,
    /// The daemon runs another version than the client (489 FR-011).
    VersionMismatch {
        /// What the daemon reports.
        found: String,
        /// What this client expects.
        expected: String,
    },
    /// The step outlived its bound.
    TimedOut {
        /// The bound it outlived.
        bound: Duration,
    },
    /// The user stopped the start.
    Cancelled,
    /// The daemon's network setting blocks downloads and no relay could be opened.
    PostureBlocksDownload,
    /// An SSH failure, with 489's reason text unchanged.
    Ssh(SshFailure),
}

impl FailureCause {
    /// The cause text. `ctx` names the host for an [`FailureCause::Ssh`] text.
    pub fn text(&self, ctx: &ReasonContext) -> String {
        match self {
            FailureCause::PullOrLoad { image } => {
                format!("the container runtime could not get image {image}")
            }
            FailureCause::ContainerStart => {
                "the container runtime could not start the container".into()
            }
            FailureCause::NoSshServer => "the image has no reachable SSH server; an image of your own must run one (see the user guide)".into(),
            FailureCause::AuthRefused => "the container's SSH server refused the daemon key".into(),
            FailureCause::CopyFailed => "the daemon build could not be written in the target".into(),
            FailureCause::NoMatchingBuild { os, arch } => {
                format!("no matching daemon build for {os}-{arch}")
            }
            FailureCause::DaemonStart => "the daemon did not start".into(),
            FailureCause::VersionMismatch { found, expected } => format!(
                "the daemon runs version {found} but this client expects {expected}; restart it with no sessions running to replace it"
            ),
            FailureCause::TimedOut { bound } => format!("did not finish within {}", bound_text(*bound)),
            FailureCause::Cancelled => "stopped by you".into(),
            FailureCause::PostureBlocksDownload => "the daemon's network setting blocks downloads and the install relay could not be opened".into(),
            FailureCause::Ssh(failure) => failure.reason(ctx),
        }
    }

    /// Whether the text is followed by the tool's output.
    fn has_output(&self) -> bool {
        !matches!(
            self,
            FailureCause::Cancelled
                | FailureCause::TimedOut { .. }
                | FailureCause::NoMatchingBuild { .. }
        )
    }
}

/// `15 minutes`, `120 seconds`: whole minutes when it is a whole number of them.
fn bound_text(bound: Duration) -> String {
    let secs = bound.as_secs();
    if secs >= 60 && secs % 60 == 0 {
        let minutes = secs / 60;
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{secs} second{}", if secs == 1 { "" } else { "s" })
    }
}

/// A failed start: the step, the cause and the output of the tool that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionFailure {
    /// The step that failed.
    pub step: Step,
    /// Why.
    pub cause: FailureCause,
    /// The failing tool's output (container runtime, SSH, installer); may be empty.
    pub output: String,
}

impl ProvisionFailure {
    /// A failure with no tool output.
    pub fn new(step: Step, cause: FailureCause) -> Self {
        Self {
            step,
            cause,
            output: String::new(),
        }
    }

    /// The same failure carrying `output`.
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = output.into();
        self
    }

    /// `"<step label> failed: <cause text>"`, then the last [`OUTPUT_LINES`] lines of the output
    /// where the cause has one.
    pub fn reason(&self, ctx: &ReasonContext) -> String {
        let head = format!("{} failed: {}", self.step.label(), self.cause.text(ctx));
        let lines: Vec<&str> = self.output.lines().collect();
        if !self.cause.has_output() || lines.iter().all(|l| l.trim().is_empty()) {
            return head;
        }
        let tail = &lines[lines.len().saturating_sub(OUTPUT_LINES)..];
        format!("{head}\n{}", tail.join("\n"))
    }
}

/// An AI CLI whose install failed. The start still succeeds (FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionWarning {
    /// The CLI.
    pub cli: AiCli,
    /// The installer's output.
    pub output: String,
}

impl ProvisionWarning {
    /// `"<CLI name> was not installed:"` plus the installer output.
    pub fn text(&self) -> String {
        let head = format!("{} was not installed:", self.cli.provider().display_name());
        if self.output.trim().is_empty() {
            head
        } else {
            format!("{head}\n{}", self.output.trim_end())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::ClientPlatform;

    fn ctx() -> ReasonContext {
        ReasonContext {
            host: "127.0.0.1".into(),
            port: 2222,
            timeout_secs: 15,
            platform: ClientPlatform::Linux,
        }
    }

    #[test]
    fn step_labels_are_the_contracts_text() {
        let want = [
            (Step::Acquire, "getting the image"),
            (Step::CreateContainer, "starting the container"),
            (Step::Reach, "connecting to the SSH server"),
            (Step::Probe, "checking the daemon build"),
            (Step::Copy, "copying the daemon"),
            (Step::InstallAiClis, "installing the AI CLIs"),
            (Step::Start, "starting the daemon"),
            (Step::VersionCheck, "checking the daemon version"),
        ];
        for (step, label) in want {
            assert_eq!(step.label(), label);
        }
    }

    #[test]
    fn every_cause_has_exactly_its_contract_text() {
        let want = [
            (
                FailureCause::PullOrLoad { image: "img:1".into() },
                "the container runtime could not get image img:1",
            ),
            (
                FailureCause::ContainerStart,
                "the container runtime could not start the container",
            ),
            (
                FailureCause::NoSshServer,
                "the image has no reachable SSH server; an image of your own must run one (see the user guide)",
            ),
            (
                FailureCause::AuthRefused,
                "the container's SSH server refused the daemon key",
            ),
            (
                FailureCause::CopyFailed,
                "the daemon build could not be written in the target",
            ),
            (
                FailureCause::NoMatchingBuild { os: "linux".into(), arch: "riscv".into() },
                "no matching daemon build for linux-riscv",
            ),
            (FailureCause::DaemonStart, "the daemon did not start"),
            (
                FailureCause::TimedOut { bound: Duration::from_secs(120) },
                "did not finish within 2 minutes",
            ),
            (
                FailureCause::TimedOut { bound: Duration::from_secs(90) },
                "did not finish within 90 seconds",
            ),
            (
                FailureCause::TimedOut { bound: Duration::from_secs(15 * 60) },
                "did not finish within 15 minutes",
            ),
            (FailureCause::Cancelled, "stopped by you"),
            (
                FailureCause::PostureBlocksDownload,
                "the daemon's network setting blocks downloads and the install relay could not be opened",
            ),
        ];
        for (cause, text) in want {
            assert_eq!(cause.text(&ctx()), text);
        }
        let mismatch = FailureCause::VersionMismatch {
            found: "1.0".into(),
            expected: "2.0".into(),
        };
        let text = mismatch.text(&ctx());
        assert!(text.contains("1.0") && text.contains("2.0"), "{text}");
    }

    #[test]
    fn an_ssh_cause_reuses_the_489_text_unchanged() {
        let failure = SshFailure::HostUnreachable;
        assert_eq!(
            FailureCause::Ssh(failure.clone()).text(&ctx()),
            failure.reason(&ctx())
        );
    }

    #[test]
    fn reason_is_step_failed_cause_then_the_last_forty_lines() {
        let output: String = (1..=50).map(|n| format!("line {n}\n")).collect();
        let f = ProvisionFailure::new(Step::Copy, FailureCause::CopyFailed).with_output(output);
        let reason = f.reason(&ctx());
        let mut lines = reason.lines();
        assert_eq!(
            lines.next(),
            Some("copying the daemon failed: the daemon build could not be written in the target")
        );
        let rest: Vec<&str> = lines.collect();
        assert_eq!(rest.len(), 40);
        assert_eq!(rest[0], "line 11");
        assert_eq!(rest[39], "line 50");
    }

    #[test]
    fn a_cause_without_output_is_one_line() {
        let f = ProvisionFailure::new(Step::Probe, FailureCause::Cancelled).with_output("noise");
        assert_eq!(
            f.reason(&ctx()),
            "checking the daemon build failed: stopped by you"
        );
    }

    #[test]
    fn a_warning_names_the_cli_and_carries_the_installer_output() {
        let w = ProvisionWarning {
            cli: AiCli::Codex,
            output: "npm ERR! 404\n".into(),
        };
        assert_eq!(
            w.text(),
            format!(
                "{} was not installed:\nnpm ERR! 404",
                AiCli::Codex.provider().display_name()
            )
        );
    }
}

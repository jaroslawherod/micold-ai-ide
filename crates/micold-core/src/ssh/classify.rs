//! Failure classification from an `ssh` child's exit and stderr (research R8, contracts table).
//!
//! The patterns are data in one table; OpenSSH exits 255 for every ssh-level error, so the stderr
//! text is what names the cause. `LC_ALL=C` makes those messages stable.

use super::argv::ClientPlatform;

/// Why an SSH attempt failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshFailure {
    /// No `ssh` executable could be started.
    SshMissing,
    /// The host name did not resolve.
    DnsFailure,
    /// The host actively refused the connection.
    ConnectionRefused,
    /// No route to the host or network.
    HostUnreachable,
    /// No answer within the connect deadline.
    Timeout,
    /// Authentication was refused (no usable key, agent not loaded).
    AuthRefused,
    /// The host key is not in `known_hosts` and strict checking refuses it.
    HostKeyUnknown,
    /// The host key differs from the recorded one.
    HostKeyChanged,
    /// The probe reported an OS or architecture this feature does not support.
    RemoteUnsupported {
        /// The remote's `uname -s`.
        os: String,
        /// The remote's `uname -m`.
        arch: String,
    },
    /// The remote's `~/.micold` could not be written.
    RemoteNotWritable,
    /// No daemon build for the remote's platform is available in this app.
    NoMatchingBuild {
        /// The remote OS.
        os: String,
        /// The remote architecture.
        arch: String,
    },
    /// The helper exited before the handshake with no ssh-level error.
    HelperFailed {
        /// The last stderr line.
        tail: String,
    },
    /// Anything else, with the last stderr line.
    Other {
        /// The last stderr line.
        tail: String,
    },
}

/// What the user-facing sentence names besides the class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonContext {
    /// The host as entered.
    pub host: String,
    /// The port, resolved or default 22.
    pub port: u16,
    /// The connect deadline in seconds.
    pub timeout_secs: u32,
    /// The client platform, for the install hint.
    pub platform: ClientPlatform,
}

/// `(stderr needle, class)` in match order: the first present needle wins.
const PATTERNS: &[(&str, fn() -> SshFailure)] = &[
    ("REMOTE HOST IDENTIFICATION HAS CHANGED", || {
        SshFailure::HostKeyChanged
    }),
    ("Host key verification failed", || {
        SshFailure::HostKeyUnknown
    }),
    ("host key is known", || SshFailure::HostKeyUnknown),
    ("Could not resolve hostname", || SshFailure::DnsFailure),
    ("Connection refused", || SshFailure::ConnectionRefused),
    ("No route to host", || SshFailure::HostUnreachable),
    ("Network is unreachable", || SshFailure::HostUnreachable),
    ("Connection timed out", || SshFailure::Timeout),
    ("Operation timed out", || SshFailure::Timeout),
    ("Permission denied", || SshFailure::AuthRefused),
];

/// Classify an attempt. `spawn_not_found`: the `ssh` executable could not be started;
/// `timed_out`: the watchdog killed the child at the deadline.
pub fn classify(spawn_not_found: bool, timed_out: bool, exit_ok: bool, stderr: &str) -> SshFailure {
    if spawn_not_found {
        return SshFailure::SshMissing;
    }
    if let Some((_, class)) = PATTERNS.iter().find(|(needle, _)| stderr.contains(needle)) {
        return class();
    }
    if timed_out {
        return SshFailure::Timeout;
    }
    let tail = last_line(stderr);
    if exit_ok {
        SshFailure::HelperFailed { tail }
    } else {
        SshFailure::Other { tail }
    }
}

fn last_line(text: &str) -> String {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

impl SshFailure {
    /// A permanent class cannot be cured by retrying: the user has to act.
    pub fn is_permanent(&self) -> bool {
        matches!(
            self,
            SshFailure::HostKeyChanged
                | SshFailure::HostKeyUnknown
                | SshFailure::RemoteUnsupported { .. }
                | SshFailure::NoMatchingBuild { .. }
        )
    }

    /// The sentence shown to the user (contracts/ssh-transport.md, failure table).
    pub fn reason(&self, ctx: &ReasonContext) -> String {
        let h = &ctx.host;
        match self {
            SshFailure::SshMissing => format!(
                "ssh was not found on this computer; install an OpenSSH client ({})",
                install_hint(ctx.platform)
            ),
            SshFailure::DnsFailure => format!("host {h} could not be resolved"),
            SshFailure::ConnectionRefused => {
                format!("{h} refused the connection on port {}", ctx.port)
            }
            SshFailure::HostUnreachable => format!("{h} is unreachable"),
            SshFailure::Timeout => format!("no answer from {h} within {} s", ctx.timeout_secs),
            SshFailure::AuthRefused => format!(
                "{h} refused authentication; load your key into the SSH agent (ssh-add) and retry"
            ),
            SshFailure::HostKeyUnknown => format!(
                "{h} has an unknown host key; connect once with ssh in a terminal to verify and trust it"
            ),
            SshFailure::HostKeyChanged => format!(
                "the host key of {h} has changed; verify it, then fix known_hosts yourself"
            ),
            SshFailure::RemoteUnsupported { os, arch } => {
                format!("{h} runs {os}/{arch}, which is not supported")
            }
            SshFailure::NoMatchingBuild { os, arch } => {
                format!("no daemon build for {os}/{arch} is available in this app")
            }
            SshFailure::RemoteNotWritable => format!(
                "cannot write the daemon to ~/.micold on {h} (read-only or no space)"
            ),
            SshFailure::HelperFailed { tail } => {
                format!("the daemon helper on {h} failed: {tail}")
            }
            SshFailure::Other { tail } => format!("ssh to {h} failed: {tail}"),
        }
    }
}

fn install_hint(platform: ClientPlatform) -> &'static str {
    match platform {
        ClientPlatform::Windows => "Settings > Optional features > OpenSSH Client",
        ClientPlatform::MacOs => "it ships with macOS; check your PATH",
        ClientPlatform::Linux => {
            "for example `apt install openssh-client` or `dnf install openssh-clients`"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ReasonContext {
        ReasonContext {
            host: "build".into(),
            port: 22,
            timeout_secs: 15,
            platform: ClientPlatform::Linux,
        }
    }

    /// Real OpenSSH messages, one per class.
    fn fixtures() -> Vec<(&'static str, SshFailure)> {
        vec![
            (
                "ssh: Could not resolve hostname nope: Name or service not known",
                SshFailure::DnsFailure,
            ),
            (
                "ssh: connect to host build port 22: Connection refused",
                SshFailure::ConnectionRefused,
            ),
            (
                "ssh: connect to host build port 22: No route to host",
                SshFailure::HostUnreachable,
            ),
            (
                "ssh: connect to host build port 22: Network is unreachable",
                SshFailure::HostUnreachable,
            ),
            (
                "ssh: connect to host build port 22: Connection timed out",
                SshFailure::Timeout,
            ),
            (
                "me@build: Permission denied (publickey,password).",
                SshFailure::AuthRefused,
            ),
            (
                "No ED25519 host key is known for build and you have requested strict checking.\nHost key verification failed.",
                SshFailure::HostKeyUnknown,
            ),
            (
                "@@@@@@@@\n@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n@@@@@@@@\nHost key verification failed.",
                SshFailure::HostKeyChanged,
            ),
        ]
    }

    #[test]
    fn every_stderr_fixture_classifies_as_its_class() {
        for (stderr, expected) in fixtures() {
            assert_eq!(
                classify(false, false, false, stderr),
                expected,
                "stderr: {stderr}"
            );
        }
    }

    #[test]
    fn a_changed_key_wins_over_the_generic_verification_failure_line() {
        // The changed-key banner is followed by `Host key verification failed.`, which alone
        // would read as an unknown key: the more specific class must win.
        let stderr =
            "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!\nHost key verification failed.";
        assert_eq!(
            classify(false, false, false, stderr),
            SshFailure::HostKeyChanged
        );
    }

    #[test]
    fn a_missing_executable_is_ssh_missing_whatever_stderr_says() {
        assert_eq!(classify(true, false, false, ""), SshFailure::SshMissing);
    }

    #[test]
    fn the_watchdog_deadline_is_a_timeout_when_stderr_names_nothing() {
        assert_eq!(classify(false, true, false, ""), SshFailure::Timeout);
    }

    #[test]
    fn an_unrecognised_failure_keeps_the_last_stderr_line() {
        assert_eq!(
            classify(false, false, false, "first\nsecond line\n\n"),
            SshFailure::Other {
                tail: "second line".into()
            }
        );
    }

    #[test]
    fn a_clean_exit_before_the_handshake_is_a_helper_failure() {
        assert_eq!(
            classify(false, false, true, "micold-daemon: bad things"),
            SshFailure::HelperFailed {
                tail: "micold-daemon: bad things".into()
            }
        );
    }

    #[test]
    fn only_host_key_and_build_availability_classes_are_permanent() {
        let permanent = [
            SshFailure::HostKeyChanged,
            SshFailure::HostKeyUnknown,
            SshFailure::RemoteUnsupported {
                os: "FreeBSD".into(),
                arch: "amd64".into(),
            },
            SshFailure::NoMatchingBuild {
                os: "linux".into(),
                arch: "aarch64".into(),
            },
        ];
        for f in &permanent {
            assert!(f.is_permanent(), "{f:?}");
        }
        for f in [
            SshFailure::SshMissing,
            SshFailure::DnsFailure,
            SshFailure::ConnectionRefused,
            SshFailure::HostUnreachable,
            SshFailure::Timeout,
            SshFailure::AuthRefused,
            SshFailure::RemoteNotWritable,
            SshFailure::HelperFailed { tail: "x".into() },
            SshFailure::Other { tail: "x".into() },
        ] {
            assert!(!f.is_permanent(), "{f:?}");
        }
    }

    #[test]
    fn reasons_name_the_cause_and_the_host_as_the_contract_table_says() {
        let c = ctx();
        assert_eq!(
            SshFailure::DnsFailure.reason(&c),
            "host build could not be resolved"
        );
        assert_eq!(
            SshFailure::ConnectionRefused.reason(&c),
            "build refused the connection on port 22"
        );
        assert_eq!(
            SshFailure::HostUnreachable.reason(&c),
            "build is unreachable"
        );
        assert_eq!(
            SshFailure::Timeout.reason(&c),
            "no answer from build within 15 s"
        );
        assert_eq!(
            SshFailure::AuthRefused.reason(&c),
            "build refused authentication; load your key into the SSH agent (ssh-add) and retry"
        );
        assert_eq!(
            SshFailure::HostKeyUnknown.reason(&c),
            "build has an unknown host key; connect once with ssh in a terminal to verify and trust it"
        );
        assert_eq!(
            SshFailure::HostKeyChanged.reason(&c),
            "the host key of build has changed; verify it, then fix known_hosts yourself"
        );
        assert_eq!(
            SshFailure::RemoteUnsupported {
                os: "FreeBSD".into(),
                arch: "amd64".into()
            }
            .reason(&c),
            "build runs FreeBSD/amd64, which is not supported"
        );
        assert_eq!(
            SshFailure::NoMatchingBuild {
                os: "linux".into(),
                arch: "aarch64".into()
            }
            .reason(&c),
            "no daemon build for linux/aarch64 is available in this app"
        );
        assert_eq!(
            SshFailure::RemoteNotWritable.reason(&c),
            "cannot write the daemon to ~/.micold on build (read-only or no space)"
        );
        assert_eq!(
            SshFailure::HelperFailed {
                tail: "boom".into()
            }
            .reason(&c),
            "the daemon helper on build failed: boom"
        );
    }

    #[test]
    fn the_missing_ssh_hint_is_per_platform() {
        let mut c = ctx();
        assert!(SshFailure::SshMissing.reason(&c).contains("apt"));
        c.platform = ClientPlatform::Windows;
        assert!(SshFailure::SshMissing.reason(&c).contains("OpenSSH Client"));
        c.platform = ClientPlatform::MacOs;
        assert!(SshFailure::SshMissing.reason(&c).contains("macOS"));
    }
}

//! Logout survival: systemd user linger on Linux, the runtime's restart policy in a sandbox
//! (US7, FR-038; feature 027 FR-014a/b).
//!
//! Surviving a full logout — not just closing the window — is **Linux-only** and MUST NEVER be
//! enabled silently (FR-038). This is the in-session enable path the GUI offers on request. It needs
//! the user's own login session: a running `systemd --user` and polkit's self-linger policy — exactly
//! what a root `postinst` lacks (research R5.1), which is why it lives here and the client triggers it
//! rather than the installer.
//!
//! The order is load-bearing (research R3.5). Enabling linger starts the user manager immediately but
//! does **not** migrate already-running processes: a daemon the client self-spawned into the login
//! session's scope stays there and still dies at logout. So the sequence is: enable linger → stop the
//! session-scoped daemon → enable+start the socket unit, which re-activates a fresh daemon inside the
//! now-lingering user manager. Failure is *detected*, never assumed — hardened deployments can refuse
//! self-linger via polkit (research R3.5).
//!
//! # The sandbox raises the bar (feature 027, FR-014b)
//!
//! "Linux-only" is a property of the *host-process* mechanism, not of the promise. A container
//! runtime's restart policy is implemented by a service the platform already keeps running across
//! logout and reboot — on all three platforms — so the sandboxed placement can offer on macOS and
//! Windows what a detached host process cannot. The setting keeps one name and one meaning; only
//! the mechanism behind it differs by placement, which is the shape this module already had.
//!
//! Nothing is *done* here for a sandbox: the policy is applied when the container is created
//! (`--restart unless-stopped`, see `sandbox::argv`). What [`enable_for`] does is report the truth
//! about a sandbox that already exists — which, when the user has only just enabled the setting,
//! is that it takes effect on the sandbox's next start.

use crate::endpoint::Endpoint;
use crate::sandbox::placement::Placement;

/// The result of an [`enable`] attempt, with a user-facing explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurvivalOutcome {
    /// Linger is on and the daemon now runs inside the lingering user manager; sessions will outlive
    /// logout.
    Enabled,
    /// This platform does not support surviving logout (macOS/Windows) — FR-038 scopes it to Linux.
    Unsupported,
    /// A required step failed (linger refused by policy, no `systemd --user`, …). Carries the detail.
    Failed(String),
    /// The sandbox will honour it, from its next start (feature 027, FR-014a/b).
    ///
    /// Its own variant rather than a [`Self::Failed`] with an explanatory string: nothing failed,
    /// and telling the user something failed when the answer is "restart the sandbox" sends them
    /// looking for a problem that is not there.
    PendingSandboxRestart,
}

impl SurvivalOutcome {
    /// A single user-facing sentence describing the outcome.
    pub fn user_message(&self) -> String {
        match self {
            SurvivalOutcome::Enabled => "Sessions will now survive logout on this machine — the \
                 session service runs under your lingering user manager."
                .to_string(),
            SurvivalOutcome::Unsupported => "Surviving logout isn't supported for a service \
                 running directly on this platform — that's Linux-only. Sessions still survive \
                 closing the window, and running the service in a container supports it here."
                .to_string(),
            SurvivalOutcome::PendingSandboxRestart => "Sessions will survive logout once the \
                 sandbox restarts — its restart policy is set when the container is created."
                .to_string(),
            SurvivalOutcome::Failed(detail) => {
                format!("Couldn't enable logout survival: {detail}")
            }
        }
    }
}

/// Enable logout survival for the current user (Linux). Idempotent — safe to run when already
/// enabled. **Blocking** (spawns `loginctl`/`systemctl`), so the caller runs it off any async
/// runtime / the UI thread. On non-Linux it is a pure [`SurvivalOutcome::Unsupported`].
#[cfg(target_os = "linux")]
pub fn enable(endpoint: &Endpoint) -> SurvivalOutcome {
    // 1. Enable linger for *ourselves* (no privilege needed under the default self-linger policy;
    //    detect failure rather than assume, per research R3.5).
    if let Err(detail) = run("loginctl", &["enable-linger"]) {
        return SurvivalOutcome::Failed(format!("enabling linger failed ({detail})"));
    }

    // 2. Stop any daemon the client self-spawned into the login-session scope: enabling linger does
    //    not migrate it, so it would still die at logout. Stopping it frees the socket for the unit
    //    and lets the survivor be a fresh, manager-hosted daemon. Best-effort — no daemon is fine.
    let _ = crate::spawn::stop_running_daemon(endpoint);

    // 3. Enable + start the socket unit inside the user manager. Socket activation then spawns the
    //    service (the daemon) on the next client connection, now under the lingering manager.
    if let Err(detail) = run(
        "systemctl",
        &["--user", "enable", "--now", "micold-daemon.socket"],
    ) {
        return SurvivalOutcome::Failed(format!(
            "enabling the systemd user socket failed ({detail}) — is a user systemd manager running?"
        ));
    }

    SurvivalOutcome::Enabled
}

/// Non-Linux: logout survival is unsupported (FR-038); this is a no-op that says so.
#[cfg(not(target_os = "linux"))]
pub fn enable(_endpoint: &Endpoint) -> SurvivalOutcome {
    SurvivalOutcome::Unsupported
}

/// Enable logout survival for a resolved placement (feature 027, FR-014a/b).
///
/// Pure for the sandbox, and blocking for the host process — which is the asymmetry the two
/// mechanisms actually have. A sandbox's survival is a property of the container that already
/// exists; there is nothing to run, only something to report.
pub fn enable_for(placement: &Placement, endpoint: &Endpoint) -> SurvivalOutcome {
    match placement {
        Placement::HostProcess => enable(endpoint),
        Placement::LocalSandbox(profile) => {
            if profile.survive_logout {
                // The container was created with `--restart unless-stopped`, by a runtime service
                // the platform keeps running across logout and reboot. On all three platforms.
                SurvivalOutcome::Enabled
            } else {
                SurvivalOutcome::PendingSandboxRestart
            }
        }
        // A remote daemon's survival is the remote host's business, and this release cannot build
        // that placement anyway (FR-003a).
        Placement::Remote(_) => SurvivalOutcome::Unsupported,
    }
}

/// Run `program args...`, mapping a non-zero exit or a spawn error to `Err(detail)` with the
/// program's own stderr preserved where there is one.
#[cfg(target_os = "linux")]
fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        Err(if detail.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            detail.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_outcome_has_a_clear_user_message() {
        assert!(SurvivalOutcome::Enabled
            .user_message()
            .contains("survive logout"));
        // The unsupported message must name the limitation plainly (FR-038, acceptance scenario 3).
        let unsupported = SurvivalOutcome::Unsupported.user_message();
        assert!(unsupported.contains("Linux-only"));
        assert!(
            unsupported.to_lowercase().contains("not supported")
                || unsupported.to_lowercase().contains("isn't supported")
        );
        // A failure surfaces its detail verbatim so the user can act on it.
        assert!(SurvivalOutcome::Failed("polkit denied".into())
            .user_message()
            .contains("polkit denied"));
    }

    fn endpoint() -> Endpoint {
        Endpoint {
            socket_path: std::path::PathBuf::from("/tmp/x.sock"),
            lock_path: std::path::PathBuf::from("/tmp/x.lock"),
        }
    }

    /// FR-014b, the bar the spec raises deliberately: the sandboxed placement offers survival on
    /// **every** platform, where the host-process mechanism manages it only on Linux.
    #[test]
    fn a_sandbox_with_survival_on_is_enabled_on_every_platform() {
        use crate::sandbox::placement::PlacementKind;
        use crate::sandbox::SandboxProfile;

        let profile = SandboxProfile {
            survive_logout: true,
            ..SandboxProfile::default()
        };
        let placement = Placement::resolve(PlacementKind::LocalSandbox, &profile);
        // No `cfg` on this assertion, on purpose: it must hold on Linux, macOS and Windows alike,
        // and a platform-gated version of it would let the promise quietly become Linux-only again.
        assert_eq!(
            enable_for(&placement, &endpoint()),
            SurvivalOutcome::Enabled
        );
    }

    #[test]
    fn a_sandbox_with_survival_off_reports_a_pending_restart_not_a_failure() {
        use crate::sandbox::placement::PlacementKind;
        use crate::sandbox::SandboxProfile;

        let placement = Placement::resolve(PlacementKind::LocalSandbox, &SandboxProfile::default());
        let outcome = enable_for(&placement, &endpoint());
        assert_eq!(outcome, SurvivalOutcome::PendingSandboxRestart);
        // Nothing failed. Saying it did sends the user looking for a problem that is not there.
        assert!(!matches!(outcome, SurvivalOutcome::Failed(_)));
        assert!(outcome.user_message().contains("restarts"));
    }

    #[test]
    fn the_unsupported_message_points_at_the_placement_that_does_support_it() {
        // The old message said "it's a Linux-only feature", which stopped being true when the
        // sandbox arrived. It is the *host-process mechanism* that is Linux-only.
        let message = SurvivalOutcome::Unsupported.user_message();
        assert!(message.contains("directly on this platform"));
        assert!(message.contains("container"));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_is_always_unsupported() {
        let endpoint = crate::endpoint::Endpoint {
            socket_path: std::path::PathBuf::from("/tmp/x.sock"),
            lock_path: std::path::PathBuf::from("/tmp/x.lock"),
        };
        assert_eq!(enable(&endpoint), SurvivalOutcome::Unsupported);
    }
}

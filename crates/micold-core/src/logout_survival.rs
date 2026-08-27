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
    /// Survival is off again: the session service no longer starts under the lingering user
    /// manager, so sessions end with the login session (feature 027, FR-014d).
    ///
    /// The opt-in has to be reversible by the same control that set it, and reversible *audibly*:
    /// a checkbox that turned something on and then did nothing when unticked would leave the user
    /// believing they had withdrawn something they had not.
    Disabled,
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
            SurvivalOutcome::Disabled => "Sessions will no longer survive logout — the session \
                 service is back to running inside your login session."
                .to_string(),
            // Neither "enable" nor "turn off": this one variant reports both directions, and a
            // failure to *withdraw* the opt-in phrased as a failure to enable it would tell the
            // user the opposite of what happened.
            SurvivalOutcome::Failed(detail) => {
                format!("Couldn't change whether sessions survive logout: {detail}")
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

/// Turn logout survival back off for the current user (Linux). Idempotent. **Blocking**, for the
/// same reason [`enable`] is.
///
/// It disables and stops the socket unit and stops the daemon the unit was activating; it does
/// **not** run `loginctl disable-linger`. Linger is a per-user switch that other services may be
/// relying on, and this application did not create it exclusively — turning it off here would
/// reach outside the promise the checkbox makes. With the unit disabled the lingering manager has
/// nothing of ours to start, so the sessions are back inside the login session either way, which
/// is the whole of what was undone.
#[cfg(target_os = "linux")]
pub fn disable(endpoint: &Endpoint) -> SurvivalOutcome {
    if let Err(detail) = run(
        "systemctl",
        &["--user", "disable", "--now", "micold-daemon.socket"],
    ) {
        return SurvivalOutcome::Failed(format!(
            "disabling the systemd user socket failed ({detail})"
        ));
    }
    // The unit being stopped does not stop a daemon it already activated: that one is its own
    // process and would keep the socket, and keep surviving. Best-effort — none running is fine.
    let _ = crate::spawn::stop_running_daemon(endpoint);
    SurvivalOutcome::Disabled
}

/// Non-Linux: there was nothing to disable, because there was nothing to enable (FR-038).
#[cfg(not(target_os = "linux"))]
pub fn disable(_endpoint: &Endpoint) -> SurvivalOutcome {
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

/// Withdraw logout survival for a resolved placement (feature 027, FR-014d).
///
/// The mirror of [`enable_for`], and it has to exist for the same reason the one control does: the
/// opt-in is a checkbox now, not a menu command, and a checkbox that only works in one direction is
/// the "silently ineffective" FR-014d names.
pub fn disable_for(placement: &Placement, endpoint: &Endpoint) -> SurvivalOutcome {
    match placement {
        Placement::HostProcess => disable(endpoint),
        // Nothing to run: the restart policy is an argument to `podman create`, so withdrawing it
        // is something the *next* start does. Same answer as enabling, and true for the same
        // reason — see [`enable_for`].
        Placement::LocalSandbox(_) => SurvivalOutcome::PendingSandboxRestart,
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
        // Withdrawing says so in the same voice, rather than leaving the user to infer it from
        // silence (FR-014d).
        let disabled = SurvivalOutcome::Disabled.user_message();
        assert!(disabled.contains("no longer survive logout"));
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

    /// Withdrawing under the sandbox is the same "next start" answer as granting it, and for the
    /// same reason: `--restart` is an argument to container creation, not a knob on a running one.
    /// It must not report a *failure* — nothing failed, and there is nothing for the user to fix.
    #[test]
    fn withdrawing_under_the_sandbox_reports_a_pending_restart() {
        use crate::sandbox::placement::PlacementKind;
        use crate::sandbox::SandboxProfile;

        let placement = Placement::resolve(PlacementKind::LocalSandbox, &SandboxProfile::default());
        let outcome = disable_for(&placement, &endpoint());
        assert_eq!(outcome, SurvivalOutcome::PendingSandboxRestart);
        assert!(!matches!(outcome, SurvivalOutcome::Failed(_)));
    }

    /// FR-014d's "rather than being absent or silently ineffective": a placement that cannot do
    /// this must *say so*, in both directions, and the message is what says it.
    #[test]
    fn a_placement_that_cannot_do_it_says_so_in_both_directions() {
        let remote = Placement::Remote(crate::sandbox::placement::RemotePlacement {
            host: "elsewhere".to_string(),
        });
        for outcome in [
            enable_for(&remote, &endpoint()),
            disable_for(&remote, &endpoint()),
        ] {
            assert_eq!(outcome, SurvivalOutcome::Unsupported);
            assert!(!outcome.user_message().is_empty());
        }
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

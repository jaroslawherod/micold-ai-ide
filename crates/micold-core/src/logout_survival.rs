//! Logout survival: the container runtime's restart policy, for a sandboxed daemon (feature 027
//! FR-014a/b/d; narrowed by feature 028 FR-005).
//!
//! Surviving a full logout — not just closing the window — belongs to **the sandboxed placement,
//! and only to it**. A container runtime's restart policy is implemented by a service the platform
//! already keeps running across logout and reboot, on all three platforms, so the promise holds on
//! macOS and Windows as well as Linux.
//!
//! # What feature 028 removed, and why
//!
//! There used to be a host-process mechanism here too: `loginctl enable-linger`, then
//! `systemctl --user enable --now micold-daemon.socket`, run in the user's own session — with a
//! `disable` mirroring it. It worked by registering the daemon with the user's service manager,
//! and that registration is exactly what feature 028 removes, because the application is now the
//! only thing that ever starts a session service (lifecycle contract §1.1, packaging contract
//! §4.11). A promise resting on the registration could not outlive it, so
//! [`Placement::HostProcess`] reports [`SurvivalOutcome::Unsupported`] in **both** directions, on
//! every platform, Linux included.
//!
//! # Nothing is *done* here
//!
//! Every arm is pure now, which is what removing the host mechanism leaves behind. For the sandbox
//! that was always true: the policy is an argument to container creation (`--restart
//! unless-stopped`, see `sandbox::argv`), so what [`enable_for`] and [`disable_for`] do is report
//! the truth about a sandbox that already exists — which, when the user has only just moved the
//! setting, is that it takes effect on the sandbox's next start.

use crate::sandbox::placement::Placement;

/// What a placement's survive-logout support amounts to, with a user-facing explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurvivalOutcome {
    /// The sandbox carries a restart policy the runtime honours across logout and reboot; sessions
    /// will outlive a logout.
    Enabled,
    /// This placement does not support surviving logout. Since feature 028 that includes a service
    /// running directly on the computer, on every platform (FR-005).
    Unsupported,
    /// A required step failed, with the detail. No arm produces this since feature 028 removed the
    /// only mechanism that *ran* anything; it stays because [`Self::user_message`] is the one place
    /// that phrases a survival failure for a user, and the remote placement (FR-003a) will need it.
    Failed(String),
    /// The sandbox will honour the setting from its next start (feature 027, FR-014a/b/d).
    ///
    /// Its own variant rather than a [`Self::Failed`] with an explanatory string: nothing failed,
    /// and telling the user something failed when the answer is "restart the sandbox" sends them
    /// looking for a problem that is not there.
    ///
    /// It answers **both** directions, so its message is written to be true both ways — see
    /// [`Self::user_message`].
    PendingSandboxRestart,
}

impl SurvivalOutcome {
    /// A single user-facing sentence describing the outcome.
    pub fn user_message(&self) -> String {
        match self {
            SurvivalOutcome::Enabled => "Sessions will survive logout on this machine — the \
                 sandbox is kept running by the container runtime, across logout and reboot."
                .to_string(),
            SurvivalOutcome::Unsupported => "Surviving logout isn't supported for a service \
                 running directly on this computer. Sessions still survive closing the window, and \
                 come back resumable after a logout; running the service in a container keeps them \
                 running through one."
                .to_string(),
            // Deliberately direction-neutral. Since feature 028 the sandbox is the only placement
            // that answers this at all, so this one sentence has to serve both ticking and
            // unticking the setting — and "sessions will survive logout once the sandbox restarts"
            // said the opposite of what happened when the user had just withdrawn the opt-in
            // (FR-014d asks for an *audible* withdrawal, not a misleading one).
            SurvivalOutcome::PendingSandboxRestart => "Whether sessions survive logout is fixed \
                 when the sandbox is created, so this takes effect the next time it restarts."
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

/// Report whether a resolved placement survives logout (feature 027, FR-014a/b; feature 028
/// FR-005).
///
/// Pure, on every arm. It once took an [`crate::endpoint::Endpoint`] because the host-process arm
/// had commands to run against it; that arm now reports rather than acts, so there is nothing left
/// for an endpoint to address.
pub fn enable_for(placement: &Placement) -> SurvivalOutcome {
    match placement {
        // Feature 028: a service running directly on this computer does not survive logout, on any
        // platform. The mechanism that used to make it survive on Linux registered the daemon with
        // the user's service manager, and that registration is what this release removes.
        Placement::HostProcess => SurvivalOutcome::Unsupported,
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
/// opt-in is a checkbox, not a menu command, and a checkbox that only works in one direction is the
/// "silently ineffective" FR-014d names.
pub fn disable_for(placement: &Placement) -> SurvivalOutcome {
    match placement {
        // Nothing to withdraw, because feature 028 removed what there was to grant. Reporting
        // `Unsupported` in this direction too is the honest answer, and it is the same answer
        // `enable_for` gives — the pair has to agree or the checkbox tells two stories.
        Placement::HostProcess => SurvivalOutcome::Unsupported,
        // Nothing to run: the restart policy is an argument to `podman create`, so withdrawing it
        // is something the *next* start does. Same answer as enabling, and true for the same
        // reason — see [`enable_for`].
        Placement::LocalSandbox(_) => SurvivalOutcome::PendingSandboxRestart,
        Placement::Remote(_) => SurvivalOutcome::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::placement::PlacementKind;
    use crate::sandbox::SandboxProfile;

    fn sandbox(survive_logout: bool) -> Placement {
        let profile = SandboxProfile {
            survive_logout,
            ..SandboxProfile::default()
        };
        Placement::resolve(PlacementKind::LocalSandbox, &profile)
    }

    #[test]
    fn each_outcome_has_a_clear_user_message() {
        assert!(SurvivalOutcome::Enabled
            .user_message()
            .contains("survive logout"));
        // The unsupported message must name the limitation plainly (acceptance scenario 3).
        let unsupported = SurvivalOutcome::Unsupported.user_message();
        assert!(
            unsupported.to_lowercase().contains("not supported")
                || unsupported.to_lowercase().contains("isn't supported")
        );
        // A failure surfaces its detail verbatim so the user can act on it.
        assert!(SurvivalOutcome::Failed("polkit denied".into())
            .user_message()
            .contains("polkit denied"));
    }

    /// FR-014d, in the direction that is easy to get wrong: the sandbox's one answer is given for
    /// both ticking and unticking, so it must not claim survival was just switched on.
    #[test]
    fn the_pending_message_reads_correctly_in_both_directions() {
        let message = SurvivalOutcome::PendingSandboxRestart.user_message();
        assert!(message.contains("restarts"));
        assert!(
            !message.contains("will survive logout"),
            "unticking the setting must not be reported as enabling it: {message}"
        );
    }

    /// FR-014b, the bar the spec raises deliberately: the sandboxed placement offers survival on
    /// **every** platform.
    #[test]
    fn a_sandbox_with_survival_on_is_enabled_on_every_platform() {
        // No `cfg` on this assertion, on purpose: it must hold on Linux, macOS and Windows alike,
        // and a platform-gated version of it would let the promise quietly become Linux-only again.
        assert_eq!(enable_for(&sandbox(true)), SurvivalOutcome::Enabled);
    }

    #[test]
    fn a_sandbox_with_survival_off_reports_a_pending_restart_not_a_failure() {
        let outcome = enable_for(&sandbox(false));
        assert_eq!(outcome, SurvivalOutcome::PendingSandboxRestart);
        // Nothing failed. Saying it did sends the user looking for a problem that is not there.
        assert!(!matches!(outcome, SurvivalOutcome::Failed(_)));
    }

    /// Withdrawing under the sandbox is the same "next start" answer as granting it, and for the
    /// same reason: `--restart` is an argument to container creation, not a knob on a running one.
    /// It must not report a *failure* — nothing failed, and there is nothing for the user to fix.
    #[test]
    fn withdrawing_under_the_sandbox_reports_a_pending_restart() {
        let outcome = disable_for(&sandbox(true));
        assert_eq!(outcome, SurvivalOutcome::PendingSandboxRestart);
        assert!(!matches!(outcome, SurvivalOutcome::Failed(_)));
    }

    /// Feature 028, FR-005: the host process does not survive logout — and now that is true on
    /// **Linux too**, not only on macOS and Windows, and in both directions.
    ///
    /// Deliberately un-`cfg`-ed. The assertion this replaced was the Linux arm returning `Enabled`,
    /// and a platform-gated replacement would let the old promise creep back on the one platform
    /// where the mechanism used to exist, which is the only platform it could creep back on.
    #[test]
    fn the_host_process_no_longer_survives_logout_on_any_platform() {
        for outcome in [
            enable_for(&Placement::HostProcess),
            disable_for(&Placement::HostProcess),
        ] {
            assert_eq!(outcome, SurvivalOutcome::Unsupported);
        }
    }

    /// FR-014d's "rather than being absent or silently ineffective": a placement that cannot do
    /// this must *say so*, in both directions, and the message is what says it.
    #[test]
    fn a_placement_that_cannot_do_it_says_so_in_both_directions() {
        let remote = Placement::Remote(crate::sandbox::placement::RemotePlacement {
            host: "elsewhere".to_string(),
        });
        for outcome in [enable_for(&remote), disable_for(&remote)] {
            assert_eq!(outcome, SurvivalOutcome::Unsupported);
            assert!(!outcome.user_message().is_empty());
        }
    }

    #[test]
    fn the_unsupported_message_points_at_the_placement_that_does_support_it() {
        // The message must not send a Linux user looking for a setting that no longer exists, and
        // must name the placement that does keep sessions through a logout.
        let message = SurvivalOutcome::Unsupported.user_message();
        assert!(message.contains("directly on this computer"));
        assert!(message.contains("container"));
        assert!(
            !message.contains("Linux"),
            "the limitation is no longer about the platform: {message}"
        );
    }
}

//! Logout survival: the container runtime's restart policy, for a sandboxed daemon (feature 027
//! FR-014a/b; narrowed by feature 028 FR-005).
//!
//! Surviving a full logout — not just closing the window — belongs to **the sandboxed placement,
//! and only to it**. A container runtime's restart policy is implemented by a service the platform
//! already keeps running across logout and reboot, on all three platforms, so the promise holds on
//! macOS and Windows as well as Linux.
//!
//! # What feature 028 removed, and why
//!
//! There used to be a host-process mechanism here too: `loginctl enable-linger`, then
//! `systemctl --user enable --now micold-daemon.socket`, run in the user's own session on request.
//! It worked by registering the daemon with the user's service manager — and that registration is
//! exactly what feature 028 removes, because the application is now the only thing that ever starts
//! a session service (lifecycle contract §1.1, packaging contract §4.11). A promise resting on the
//! registration could not outlive it, so [`Placement::HostProcess`] reports
//! [`SurvivalOutcome::Unsupported`] on every platform, Linux included.
//!
//! # Nothing is *done* here
//!
//! Even for the sandbox: the policy is applied when the container is created (`--restart
//! unless-stopped`, see `sandbox::argv`). What [`enable_for`] does is report the truth about a
//! sandbox that already exists — which, when the user has only just enabled the setting, is that it
//! takes effect on the sandbox's next start.

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
            SurvivalOutcome::Enabled => "Sessions will survive logout on this machine — the \
                 sandbox is kept running by the container runtime, across logout and reboot."
                .to_string(),
            SurvivalOutcome::Unsupported => "Surviving logout isn't supported for a service \
                 running directly on this computer. Sessions still survive closing the window, and \
                 come back resumable after a logout; running the service in a container keeps them \
                 running through one."
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
        assert!(outcome.user_message().contains("restarts"));
    }

    /// Feature 028, FR-005: the host process does not survive logout — and now that is true on
    /// **Linux too**, not only on macOS and Windows.
    ///
    /// Deliberately un-`cfg`-ed. The assertion this replaced was the Linux arm returning `Enabled`,
    /// and a platform-gated replacement would let the old promise creep back on the one platform
    /// where the mechanism used to exist, which is the only platform it could creep back on.
    #[test]
    fn the_host_process_no_longer_survives_logout_on_any_platform() {
        assert_eq!(
            enable_for(&Placement::HostProcess),
            SurvivalOutcome::Unsupported
        );
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

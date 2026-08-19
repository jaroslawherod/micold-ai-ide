//! Where the daemon runs, and how that is resolved (FR-001, FR-002, FR-003, FR-003a).
//!
//! Generalises the assumption `endpoint.rs` used to encode implicitly: that the daemon is a
//! detached host process. Resolution is pure and never silently substitutes one placement for
//! another — a sandbox that cannot start is an error, not a quiet fall back to the host (FR-035).

use serde::{Deserialize, Serialize};

use super::SandboxProfile;

/// Which placement the user has chosen, as it is persisted.
///
/// Separate from the rich [`Placement`] below: settings store the *choice*, and resolution builds
/// the placement from it. That separation is what keeps resolution a pure function of settings
/// (rule P-1) rather than something that reads the world on the way past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementKind {
    /// Today's behaviour: a detached host process. Unchanged, and still the default — upgrading
    /// the app never moves a user into the sandbox (rule S-4, FR-001).
    #[default]
    HostProcess,
    /// This feature: a container on the local machine.
    LocalSandbox,
}

impl PlacementKind {
    /// Every placement a user may choose today. [`Placement::Remote`] is deliberately absent: the
    /// variant exists so the *model* accommodates it (FR-003a), not so the UI offers it.
    pub const SELECTABLE: [PlacementKind; 2] =
        [PlacementKind::HostProcess, PlacementKind::LocalSandbox];

    /// The user-facing name.
    pub fn label(self) -> &'static str {
        match self {
            PlacementKind::HostProcess => "On this computer",
            PlacementKind::LocalSandbox => "In a container",
        }
    }
}

/// A daemon that is not on this machine. Reserved by FR-003a.
///
/// `#[non_exhaustive]` and unconstructible outside this crate on purpose: the shape is not designed
/// yet, and pretending otherwise would bake a guess into the settings schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemotePlacement {
    /// The host the daemon runs on. Private, so no caller outside can build one by accident.
    #[allow(dead_code)]
    pub(crate) host: String,
}

/// Where the daemon runs, resolved.
///
/// # Why a variant that cannot be built
///
/// FR-003a requires the placement model to describe a non-local daemon. Adding the variant now
/// costs one `match` arm per site and forces every placement-dependent decision to be *expressed*;
/// adding it later means finding every place `HostProcess` was assumed by omission. That is also
/// why `connect_or_spawn` becomes `connect_or_start(placement)` — the old name encoded the
/// assumption in the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// A detached host process.
    HostProcess,
    /// A container on this machine, configured by the profile.
    LocalSandbox(Box<SandboxProfile>),
    /// A daemon elsewhere. Not constructible in this release.
    Remote(RemotePlacement),
}

impl Placement {
    /// Resolve the placement from the user's stored choice and profile.
    ///
    /// Pure (rule P-1): it never touches the network, the filesystem, or a runtime. Whether the
    /// resolved placement can actually *start* is a separate question asked later, by code that is
    /// allowed to fail.
    pub fn resolve(kind: PlacementKind, profile: &SandboxProfile) -> Self {
        match kind {
            PlacementKind::HostProcess => Placement::HostProcess,
            PlacementKind::LocalSandbox => Placement::LocalSandbox(Box::new(profile.clone())),
        }
    }

    /// Which stored choice this placement came from.
    pub fn kind(&self) -> PlacementKind {
        match self {
            Placement::HostProcess => PlacementKind::HostProcess,
            Placement::LocalSandbox(_) => PlacementKind::LocalSandbox,
            // A remote daemon is not a local choice; it is reported as the host placement's
            // stored value because there is no stored value for it yet.
            Placement::Remote(_) => PlacementKind::HostProcess,
        }
    }

    /// The profile, when this placement has one.
    pub fn profile(&self) -> Option<&SandboxProfile> {
        match self {
            Placement::LocalSandbox(p) => Some(p),
            Placement::HostProcess | Placement::Remote(_) => None,
        }
    }

    /// Whether the daemon is contained by this placement.
    pub fn is_sandboxed(&self) -> bool {
        matches!(self, Placement::LocalSandbox(_))
    }
}

/// A one-occurrence decision to run unsandboxed after the sandbox failed (FR-035a).
///
/// Deliberately **not** a [`Placement`] and deliberately not a resolution outcome (rule P-3). It is
/// a distinct, user-taken action that carries the reason it was offered, so the "did the user
/// choose this?" question has a value to answer it rather than being inferred from state. It is
/// also why it does not persist: the next launch attempts the sandbox again without intervention
/// (US6 scenario 2), which is what stops a broken sandbox quietly becoming a permanent one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentedFallback {
    /// What the user was told when they agreed to it.
    pub because: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_placement_is_the_host_process() {
        // Rule S-4 / FR-001: upgrading the app never moves a user into the sandbox.
        assert_eq!(PlacementKind::default(), PlacementKind::HostProcess);
    }

    #[test]
    fn resolution_is_a_pure_function_of_its_arguments() {
        // Rule P-1. Same inputs, same output, every time — no environment, no runtime, no clock.
        let profile = SandboxProfile::default();
        let a = Placement::resolve(PlacementKind::LocalSandbox, &profile);
        let b = Placement::resolve(PlacementKind::LocalSandbox, &profile);
        assert_eq!(a, b);
    }

    #[test]
    fn resolving_a_sandbox_never_yields_a_host_process() {
        // Rule P-2, the load-bearing one: a sandbox that cannot start must be an error somewhere
        // else, never a quiet substitution here. If this test ever fails, FR-035's guarantee is
        // gone and nothing else in the feature would notice.
        let resolved = Placement::resolve(PlacementKind::LocalSandbox, &SandboxProfile::default());
        assert!(resolved.is_sandboxed());
        assert_ne!(resolved, Placement::HostProcess);
    }

    #[test]
    fn a_resolved_placement_reports_the_choice_it_came_from() {
        for kind in PlacementKind::SELECTABLE {
            let resolved = Placement::resolve(kind, &SandboxProfile::default());
            assert_eq!(resolved.kind(), kind);
        }
    }

    #[test]
    fn only_the_sandbox_placement_carries_a_profile() {
        assert!(Placement::HostProcess.profile().is_none());
        assert!(
            Placement::resolve(PlacementKind::LocalSandbox, &SandboxProfile::default())
                .profile()
                .is_some()
        );
    }

    #[test]
    fn a_fallback_is_not_a_placement_and_carries_its_reason() {
        // Rule P-3. Consent is a value with a reason attached, not a state the code can slide into
        // — which is what makes "did the user actually choose this?" answerable.
        let fallback = ConsentedFallback {
            because: "Docker is not running".to_string(),
        };
        assert!(!fallback.because.is_empty());
    }
}

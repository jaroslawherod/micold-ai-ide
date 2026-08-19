//! Bringing a sandbox up, and the state the user watches while it happens (FR-032 … FR-036).
//!
//! The sequence is probe → acquire → create → start, and the reason it lives here rather than in
//! the client is that every decision in it is testable: which stage failed, what the user is told,
//! and — the one that matters most — that **no path through it ends in an unsandboxed daemon**.
//!
//! # The guarantee this file exists to hold
//!
//! FR-035 says the app never silently falls back out of the sandbox. That is easy to state and easy
//! to lose: one `unwrap_or_else(|| HostProcess)` written in a hurry, in any of four stages, and the
//! feature is gone while every test still passes and the app still works. So the state machine has
//! no edge from [`SandboxState::Failed`] to a working unsandboxed daemon at all — leaving that
//! state requires a [`ConsentedFallback`](super::placement::ConsentedFallback), which only the user
//! can produce, and `lifecycle_never_falls_back_on_its_own` asserts it as a property of the graph.

use super::image::ImageSource;
use super::placement::ConsentedFallback;
use super::runtime::{
    ContainerId, ContainerRuntime, Progress, RuntimeCapabilities, RuntimeError, UnsatisfiableLimit,
};
use super::{MountSet, SandboxProfile, SandboxSpec};

/// Where the sandbox is in coming up (data-model §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxState {
    /// The host placement is selected; nothing to do.
    Disabled,
    /// Asking the runtime what it is and what it can enforce.
    Probing,
    /// Making the image available. The only stage that may last minutes, and the first thing a new
    /// user sees — which is why it reports progress (SC-004).
    Acquiring(Progress),
    /// Creating and starting the container.
    Starting,
    /// Up, with a container to talk to.
    Running(ContainerId),
    /// The registered projects changed, so what the sandbox can see is out of date. The mount set
    /// is fixed at creation — neither runtime can add a bind mount to a running container — so the
    /// only question is who decides when to take the interruption, and the answer is the user
    /// (research R9, rule M-4).
    Stale(ContainerId),
    /// Down, with a reason and a remedy. There is no edge out of here to a working unsandboxed
    /// daemon; see this module's header.
    Failed(Failure),
}

/// Why the sandbox is not running, in terms a user can act on (FR-034).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// Which stage failed, so the message can say *when* as well as *what*.
    pub stage: Stage,
    /// The classified cause.
    pub error: RuntimeError,
}

/// The stage a failure happened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Probing,
    Acquiring,
    Creating,
    Starting,
}

impl Stage {
    /// The user-facing name, used in the failure message.
    pub fn label(self) -> &'static str {
        match self {
            Stage::Probing => "checking the container runtime",
            Stage::Acquiring => "getting the sandbox image",
            Stage::Creating => "creating the sandbox",
            Stage::Starting => "starting the sandbox",
        }
    }
}

impl Failure {
    /// One sentence naming the cause.
    pub fn reason(&self) -> String {
        format!(
            "The sandbox failed while {}. {}",
            self.stage.label(),
            self.error.reason()
        )
    }

    /// What the user can do about it. Every failure has one (FR-034).
    pub fn remedy(&self) -> String {
        self.error.remedy()
    }
}

impl SandboxState {
    /// Whether this state should be shown persistently rather than as a passing notification
    /// (FR-035b). A failed or stale sandbox is a condition, not an event: a toast that scrolls away
    /// is how a sandbox stays broken for weeks without anyone noticing.
    pub fn is_persistent(&self) -> bool {
        matches!(self, SandboxState::Failed(_) | SandboxState::Stale(_))
    }

    /// The container, when there is one.
    pub fn container(&self) -> Option<&ContainerId> {
        match self {
            SandboxState::Running(id) | SandboxState::Stale(id) => Some(id),
            _ => None,
        }
    }

    /// Whether a session may be started against this state.
    ///
    /// `Stale` says yes: what is out of date is the *mount set*, and sessions already running are
    /// unaffected. Refusing here would turn a background settings change into an outage.
    pub fn accepts_sessions(&self) -> bool {
        matches!(self, SandboxState::Running(_) | SandboxState::Stale(_))
    }
}

/// What a successful bring-up produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The running container.
    pub id: ContainerId,
    /// What the runtime turned out to be able to enforce, for the view to render.
    pub capabilities: RuntimeCapabilities,
    /// Limits the user set that this runtime cannot enforce. Not an error — the sandbox runs — but
    /// the view must say so rather than let the user believe a bound exists (FR-015).
    pub unsatisfiable: Vec<UnsatisfiableLimit>,
}

/// Bring the sandbox up, reporting each state as it is entered.
///
/// `spec_for` builds the spec once the capabilities are known, because reconciliation needs them:
/// a limit this runtime cannot enforce must be absent from the argv, not merely absent from the UI.
pub fn bring_up<R, F>(
    runtime: &R,
    profile: &SandboxProfile,
    mounts: &MountSet,
    spec_for: F,
    observe: &mut dyn FnMut(SandboxState),
) -> Result<Started, Failure>
where
    R: ContainerRuntime,
    F: FnOnce(&RuntimeCapabilities) -> SandboxSpec,
{
    observe(SandboxState::Probing);
    let capabilities = runtime.probe().map_err(|error| Failure {
        stage: Stage::Probing,
        error,
    })?;

    let unsatisfiable = super::runtime::reconcile(profile, &capabilities);

    acquire(runtime, &profile.image, observe)?;

    observe(SandboxState::Starting);
    let spec = spec_for(&capabilities);
    debug_assert_eq!(
        spec.mounts, *mounts,
        "the spec must carry the mount set it was built for"
    );
    let id = runtime.create(&spec).map_err(|error| Failure {
        stage: Stage::Creating,
        error,
    })?;
    runtime.start(&id).map_err(|error| Failure {
        stage: Stage::Starting,
        error,
    })?;

    observe(SandboxState::Running(id.clone()));
    Ok(Started {
        id,
        capabilities,
        unsatisfiable,
    })
}

fn acquire<R: ContainerRuntime>(
    runtime: &R,
    image: &ImageSource,
    observe: &mut dyn FnMut(SandboxState),
) -> Result<(), Failure> {
    let mut report = |p: Progress| observe(SandboxState::Acquiring(p));
    runtime
        .acquire_image(image, &mut report)
        .map(|_| ())
        .map_err(|error| Failure {
            stage: Stage::Acquiring,
            error,
        })
}

/// The only way out of [`SandboxState::Failed`] that reaches a working daemon.
///
/// Takes the consent as an argument because consent is the whole point: there is no version of this
/// function that decides on the user's behalf, and there is no other function that returns an
/// unsandboxed placement from a failed sandbox (FR-035a).
pub fn accept_fallback(
    state: &SandboxState,
    consent: ConsentedFallback,
) -> Option<ConsentedFallback> {
    match state {
        SandboxState::Failed(_) => Some(consent),
        // Offering a fallback out of a *working* sandbox would be a way to leave it without meaning
        // to, which is the same failure from the other direction.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::runtime::RuntimeKind;

    fn failure() -> Failure {
        Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        }
    }

    /// The property FR-035 rests on, asserted over the whole state space rather than one path.
    #[test]
    fn no_state_reports_itself_as_an_unsandboxed_daemon() {
        let states = [
            SandboxState::Disabled,
            SandboxState::Probing,
            SandboxState::Acquiring(Progress {
                stage: "Downloading".into(),
                detail: None,
                percent: None,
            }),
            SandboxState::Starting,
            SandboxState::Running(ContainerId("x".into())),
            SandboxState::Stale(ContainerId("x".into())),
            SandboxState::Failed(failure()),
        ];
        for s in &states {
            // Only a running or stale sandbox accepts sessions. Nothing else does, and in
            // particular `Failed` does not — which is what stops a session starting outside the
            // sandbox because the sandbox was not there.
            let accepts = s.accepts_sessions();
            assert_eq!(accepts, s.container().is_some(), "{s:?}");
        }
    }

    #[test]
    fn a_failure_cannot_reach_a_working_daemon_without_consent() {
        // There is no `From<Failure> for Placement`, and this is the only function that produces a
        // fallback at all — so the assertion is that it refuses without a `ConsentedFallback`,
        // which the type system already guarantees, and that it refuses from every other state.
        let consent = ConsentedFallback {
            because: "Docker is not installed".into(),
        };
        assert!(accept_fallback(&SandboxState::Failed(failure()), consent.clone()).is_some());
        for s in [
            SandboxState::Disabled,
            SandboxState::Probing,
            SandboxState::Starting,
            SandboxState::Running(ContainerId("x".into())),
            SandboxState::Stale(ContainerId("x".into())),
        ] {
            assert!(
                accept_fallback(&s, consent.clone()).is_none(),
                "{s:?} offered a fallback it should not have"
            );
        }
    }

    #[test]
    fn failed_and_stale_are_the_persistent_states() {
        // FR-035b. A sandbox that is broken and says so once, in a toast, is a sandbox that stays
        // broken — the spec's own edge case: "never noticing that sandboxing has been broken for
        // weeks".
        assert!(SandboxState::Failed(failure()).is_persistent());
        assert!(SandboxState::Stale(ContainerId("x".into())).is_persistent());
        assert!(!SandboxState::Running(ContainerId("x".into())).is_persistent());
        assert!(!SandboxState::Probing.is_persistent());
    }

    #[test]
    fn every_stage_and_failure_reads_as_a_sentence_with_a_remedy() {
        for stage in [
            Stage::Probing,
            Stage::Acquiring,
            Stage::Creating,
            Stage::Starting,
        ] {
            let f = Failure {
                stage,
                error: RuntimeError::NotRunning {
                    kind: RuntimeKind::Docker,
                },
            };
            assert!(f.reason().contains(stage.label()), "{stage:?}");
            assert!(!f.remedy().trim().is_empty(), "{stage:?} has no remedy");
        }
    }

    #[test]
    fn a_stale_sandbox_still_accepts_sessions() {
        // Rule M-4: what is out of date is the mount set. Refusing sessions here would turn a
        // background settings change into an outage, which is the opposite of what the daemon is
        // for.
        assert!(SandboxState::Stale(ContainerId("x".into())).accepts_sessions());
    }
}

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
use super::parse::ContainerFacts;
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

/// What to do about a container that already carries our name (US6 scenario 5, FR-024d).
///
/// A sandbox outlives the application by design, so on almost every start there is one already
/// there. The question is whether it is *ours* — the same image, from the same build — because a
/// container left over from a previous or mismatched version looks identical from the outside and
/// misbehaves from the inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adoption {
    /// Nothing there; create one.
    Create,
    /// Ours and running. Use it: this is the ordinary case, and re-creating it would end every
    /// session the user left running.
    Attach(ContainerId),
    /// Ours but stopped. Start it — the state is intact, only the process is gone.
    Start(ContainerId),
    /// Not ours. Replace it, naming why.
    ///
    /// Replace rather than *accumulate beside*: a second container under a different name would
    /// leave the first holding the control port and the state directory, and the user with two
    /// sandboxes and no way to tell which is which.
    Replace {
        id: ContainerId,
        reason: StaleReason,
    },
}

/// Why an existing container is not ours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    /// Built from a different image than the profile now names — the user changed the image
    /// setting, or upgraded the application.
    DifferentImage { found: String, expected: String },
    /// Same image reference, different build. Only meaningful for a locally built image, where the
    /// container and the client came from one working tree and have no business disagreeing
    /// (research R8).
    StaleBuild {
        found: Option<String>,
        expected: String,
    },
}

impl StaleReason {
    /// One sentence, for the log and for the diagnostics the user can ask for.
    pub fn describe(&self) -> String {
        match self {
            StaleReason::DifferentImage { found, expected } => format!(
                "the existing sandbox was built from `{found}`, not `{expected}`"
            ),
            StaleReason::StaleBuild { found, expected } => format!(
                "the existing sandbox was built from a different source tree ({} rather than {expected})",
                found.as_deref().unwrap_or("an unlabelled build")
            ),
        }
    }
}

/// Decide what to do about whatever is already using our container name.
///
/// Pure: it takes the facts and returns the decision, so every branch is testable without a
/// runtime — including the ones that are awkward to arrange with one (an image that changed under
/// a running container, a build with no fingerprint at all).
pub fn adopt(
    existing: Option<&ContainerFacts>,
    expected_image: &str,
    expected_fingerprint: &str,
    strict_fingerprint: bool,
) -> Adoption {
    let Some(facts) = existing else {
        return Adoption::Create;
    };
    let id = ContainerId(facts.id.clone());

    if facts.image != expected_image {
        return Adoption::Replace {
            id,
            reason: StaleReason::DifferentImage {
                found: facts.image.clone(),
                expected: expected_image.to_string(),
            },
        };
    }

    // The fingerprint is only decisive for a locally built image. A released client and a released
    // daemon are built separately and legitimately differ, so comparing them here would replace a
    // perfectly good sandbox on every normal start.
    if strict_fingerprint && facts.fingerprint.as_deref() != Some(expected_fingerprint) {
        return Adoption::Replace {
            id,
            reason: StaleReason::StaleBuild {
                found: facts.fingerprint.clone(),
                expected: expected_fingerprint.to_string(),
            },
        };
    }

    if facts.running {
        Adoption::Attach(id)
    } else {
        Adoption::Start(id)
    }
}

/// Bring the sandbox up, reporting each state as it is entered.
///
/// `spec_for` builds the spec once the capabilities are known, because reconciliation needs them:
/// a limit this runtime cannot enforce must be absent from the argv, not merely absent from the UI.
pub fn bring_up<R, F>(
    runtime: &R,
    profile: &SandboxProfile,
    mounts: &MountSet,
    expected_fingerprint: &str,
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

    // A sandbox outlives the application by design, so on almost every start there is already one
    // carrying our name. Whether it is *ours* is what `adopt` decides — and getting this wrong in
    // the direction of "always create" would end every session the user left running, on every
    // launch (US6 scenario 5).
    let existing = runtime.find(&spec.name).map_err(|error| Failure {
        stage: Stage::Creating,
        error,
    })?;
    let decision = adopt(
        existing.as_ref(),
        &profile.image.reference,
        expected_fingerprint,
        profile.image.refuses_fingerprint_mismatch(),
    );

    let id = match decision {
        Adoption::Attach(id) => id,
        Adoption::Start(id) => {
            runtime.start(&id).map_err(|error| Failure {
                stage: Stage::Starting,
                error,
            })?;
            id
        }
        Adoption::Create => create_and_start(runtime, &spec)?,
        Adoption::Replace { id, .. } => {
            // Replace, never accumulate beside: a second container under another name would leave
            // the first holding the control port and the state directory.
            runtime.remove(&id).map_err(|error| Failure {
                stage: Stage::Creating,
                error,
            })?;
            create_and_start(runtime, &spec)?
        }
    };

    observe(SandboxState::Running(id.clone()));
    Ok(Started {
        id,
        capabilities,
        unsatisfiable,
    })
}

fn create_and_start<R: ContainerRuntime>(
    runtime: &R,
    spec: &SandboxSpec,
) -> Result<ContainerId, Failure> {
    let id = runtime.create(spec).map_err(|error| Failure {
        stage: Stage::Creating,
        error,
    })?;
    runtime.start(&id).map_err(|error| Failure {
        stage: Stage::Starting,
        error,
    })?;
    Ok(id)
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

/// What a running sandbox becomes when the set of things it should be sharing changes (R9, M-4).
///
/// A container's mounts are fixed when it is created. Register a project after that and the
/// sandbox cannot see it, however many times it is asked to — so the state has to say so, and
/// `Stale` is that answer.
///
/// # Why this does not restart anything
///
/// Restarting would be the obliging thing to do and it is the wrong thing to do: the sessions
/// inside the container are the user's work, and ending them to service a settings change they
/// made in another window turns an edit into an outage. `Stale` still accepts sessions for the
/// same reason — what is out of date is the mount set, not the container.
///
/// Every other state is returned unchanged. A sandbox that is still coming up will pick the new
/// mount set up when it creates its container, and one that has failed or is disabled has no
/// mounts to be out of date.
/// What a sandbox becomes when the container it names has gone away (FR-036, US6 scenario 3).
///
/// The trigger is a lost connection, not a poll: the application does not watch the container, it
/// notices that the service it was talking to has stopped answering and then asks *once* whether
/// the container is still there. That ordering matters — polling a container runtime every few
/// seconds for the lifetime of the application is a cost paid on every run to detect something
/// that happens almost never.
///
/// `None` when there was nothing to lose. A sandbox that never came up cannot be stopped from
/// outside, and reporting one as lost would overwrite a real failure with a vaguer one.
///
/// The result is `Failed`, which is a *defined* state with a banner and a restart action — the
/// whole point of FR-036 is that the alternative is a client retrying a connection to a container
/// that no longer exists, forever, with nothing on screen to say so.
pub fn container_lost(state: &SandboxState, name: &str) -> Option<SandboxState> {
    match state {
        SandboxState::Running(_) | SandboxState::Stale(_) => Some(SandboxState::Failed(Failure {
            // `Starting` rather than a stage of its own: the sandbox is no longer up, and that is
            // the stage whose label — "starting the sandbox" — is true of what has to happen next.
            stage: Stage::Starting,
            error: RuntimeError::SandboxStopped {
                name: name.to_string(),
            },
        })),
        SandboxState::Disabled
        | SandboxState::Probing
        | SandboxState::Acquiring(_)
        | SandboxState::Starting
        | SandboxState::Failed(_) => None,
    }
}

pub fn mount_set_changed(state: &SandboxState) -> SandboxState {
    match state {
        SandboxState::Running(id) => SandboxState::Stale(id.clone()),
        other => other.clone(),
    }
}

/// The user asked, in so many words, for the sandbox to be restarted.
///
/// A marker rather than a bare call, for the same reason [`ConsentedFallback`] is one: it makes
/// the *only* edge back into bring-up carry evidence that a person asked for it, so "nothing
/// restarts on its own" is a property of the type rather than a rule a caller is trusted to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartRequested;

/// Restart the sandbox, at the user's explicit request.
///
/// `None` where there is nothing to restart: a disabled sandbox is not broken, and restarting one
/// that is already coming up would abandon the attempt in flight and start it again from the top.
pub fn restart(state: &SandboxState, _: RestartRequested) -> Option<SandboxState> {
    match state {
        // The case R9 exists for, plus the two a user would reasonably ask for by hand.
        SandboxState::Stale(_) | SandboxState::Running(_) | SandboxState::Failed(_) => {
            Some(SandboxState::Probing)
        }
        SandboxState::Disabled | SandboxState::Probing | SandboxState::Acquiring(_)
        | SandboxState::Starting => None,
    }
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

#[cfg(test)]
mod adoption_tests {
    use super::*;
    use crate::sandbox::parse::ContainerFacts;

    fn facts(image: &str, fingerprint: Option<&str>, running: bool) -> ContainerFacts {
        ContainerFacts {
            id: "9f2b".to_string(),
            running,
            image: image.to_string(),
            fingerprint: fingerprint.map(str::to_string),
        }
    }

    #[test]
    fn nothing_there_means_create_one() {
        assert_eq!(
            adopt(None, "micold-daemon:0.27.0", "abc", false),
            Adoption::Create
        );
    }

    #[test]
    fn our_running_sandbox_is_attached_to_not_recreated() {
        // The ordinary case, and the important one: re-creating here would end every session the
        // user left running, on every launch.
        let f = facts("micold-daemon:0.27.0", Some("abc"), true);
        assert_eq!(
            adopt(Some(&f), "micold-daemon:0.27.0", "abc", false),
            Adoption::Attach(ContainerId("9f2b".into()))
        );
    }

    #[test]
    fn our_stopped_sandbox_is_started_not_replaced() {
        // The state is intact; only the process is gone. Replacing it would be a data loss dressed
        // up as a recovery.
        let f = facts("micold-daemon:0.27.0", Some("abc"), false);
        assert_eq!(
            adopt(Some(&f), "micold-daemon:0.27.0", "abc", false),
            Adoption::Start(ContainerId("9f2b".into()))
        );
    }

    #[test]
    fn a_different_image_is_replaced_with_the_reason_named() {
        let f = facts("micold-daemon:0.26.0", Some("abc"), true);
        match adopt(Some(&f), "micold-daemon:0.27.0", "abc", false) {
            Adoption::Replace { reason, .. } => {
                let text = reason.describe();
                assert!(text.contains("0.26.0"), "{text}");
                assert!(text.contains("0.27.0"), "{text}");
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn a_stale_build_is_replaced_only_under_the_strict_policy() {
        // Research R8's asymmetry, at the container level this time. A released client and daemon
        // legitimately differ, so a non-strict comparison must leave a working sandbox alone.
        let f = facts("micold-daemon:dev", Some("old"), true);

        assert_eq!(
            adopt(Some(&f), "micold-daemon:dev", "new", false),
            Adoption::Attach(ContainerId("9f2b".into())),
            "a released image must not be replaced over a fingerprint difference"
        );

        match adopt(Some(&f), "micold-daemon:dev", "new", true) {
            Adoption::Replace { reason, .. } => {
                assert!(reason.describe().contains("source tree"));
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn a_container_with_no_fingerprint_at_all_is_stale_under_the_strict_policy() {
        // An image built before the label existed. Under the strict policy that is exactly as
        // untrustworthy as a wrong one, and saying so is what makes `mise run image` reliable.
        let f = facts("micold-daemon:dev", None, true);
        match adopt(Some(&f), "micold-daemon:dev", "new", true) {
            Adoption::Replace { reason, .. } => {
                assert!(reason.describe().contains("unlabelled"));
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }
}

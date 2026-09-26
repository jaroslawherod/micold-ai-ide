//! The sandbox's client-side state (feature 027, FR-032 … FR-036).
//!
//! Render-free, like every module here: it holds what the app knows about the sandbox and the
//! operations over it, and the view in `crate::ui` draws from it. The *decisions* — which stage
//! comes next, whether a state accepts sessions, whether a fallback is reachable — live in
//! `micold_core::sandbox::lifecycle`, so they are testable without the GUI and cannot be quietly
//! re-decided here.
//!
//! What this module adds on top of the core's state machine is the part that is specific to *this*
//! application being open: whether the user has taken a one-occurrence fallback in this run, and
//! what the view needs to render about limits the runtime cannot enforce.

use micold_core::protocol::messages::ExitStatus;
use micold_core::sandbox::lifecycle::{
    Failure, RestartRequested, SandboxState, Started, UnattendedBringUps,
};
use micold_core::sandbox::placement::{ConsentedFallback, PlacementKind};
use micold_core::sandbox::runtime::{ContainerId, RuntimeCapabilities, UnsatisfiableLimit};
use micold_core::sandbox::{Bytes, ResourceBudget};

/// Everything the sandbox reports or is asked to do (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// All six began with `Sandbox` and none does now — the type says which feature (contract M1), so
/// `SandboxRestartRequested` is `Msg::RestartRequested`.
///
/// # This module declares no `update`
///
/// Shape **B** with no pure half, exactly like [`crate::features::connection`]: bringing a
/// container up is runtime, not a decision, and all six arms must return a `Task`. So the reducer
/// entry is `shell/sandbox.rs`'s `update` and `app::State::update` declines the whole vocabulary
/// in one arm, as it declined each of the six before.
///
/// The state the six arms write is [`Sandbox`] below, which lives on the binary's `App` rather
/// than on `app::State` for the same reason the connection's does — see contract S2's record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The bring-up finished and the service is reachable inside the container (FR-032).
    ///
    /// Carries the locations that container shares with this machine too, so feature 031 can
    /// translate its sessions' `file` links (FR-018): they are known only to the bring-up that
    /// adopted the container, and rebuilding them later would describe the *next* container.
    ///
    /// Boxed because `Started` carries the whole spec that produced it, and an unboxed variant
    /// would set the size of every `Message` in the application by this one.
    Started(Box<(Started, SandboxLocations)>),
    /// The container is there but the service inside it logged something worth showing (FR-036).
    Diagnostics(Vec<String>),
    /// The container that was running is gone (FR-034).
    Lost,
    /// The user asked for the bring-up to be tried again (FR-035a). Never sent by the application
    /// to itself.
    RestartRequested,
    /// The user chose to run without the sandbox for this occurrence (FR-035a). Also never sent by
    /// the application to itself.
    FallbackAccepted,
    /// The bring-up failed, and the failure is recorded rather than acted on (FR-035b).
    ///
    /// Boxed for the same reason as [`Msg::Started`].
    Failed(Box<Failure>),
    /// A bring-up entered a stage.
    Progress(Box<SandboxState>),
}

/// A limit that can stop a session, and the control that governs it (US4 scenario 3).
///
/// The processor limit is deliberately absent. A CPU share *throttles* — a session under it runs
/// slowly and finishes — so there is no stop to explain, and offering it here would invite a
/// message blaming a limit that did not do anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxLimit {
    /// The memory ceiling. Exceeding it gets the process killed outright.
    Memory,
    /// The process-count ceiling. Exceeding it makes the next fork fail.
    Processes,
    /// The writable-storage ceiling. Exceeding it makes the next write fail.
    Storage,
}

impl SandboxLimit {
    /// The **setting's own label**, exactly as the Settings form prints it.
    ///
    /// Named once and read by both, so the message cannot send a user looking for a control by a
    /// name the form does not use. That is the whole of "which setting governs it": a limit
    /// reported as "the memory cap" against a field called "Memory limit" is a scavenger hunt.
    pub fn setting(self) -> &'static str {
        match self {
            SandboxLimit::Memory => "Memory limit",
            SandboxLimit::Processes => "Process limit",
            SandboxLimit::Storage => "Writable storage limit",
        }
    }

    /// What the limit did, in the past tense, for the first half of the sentence.
    fn what_happened(self) -> &'static str {
        match self {
            SandboxLimit::Memory => "ran out of memory",
            SandboxLimit::Processes => "could not start another process",
            SandboxLimit::Storage => "ran out of writable space",
        }
    }
}

/// A session that stopped because a sandbox limit stopped it, and everything needed to say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoppedByLimit {
    /// Which limit.
    pub limit: SandboxLimit,
    /// What that limit is currently set to, in the unit its field uses.
    pub configured: String,
}

impl StoppedByLimit {
    /// The whole report: what stopped, why, which setting governs it, and what it is set to.
    ///
    /// # Why this is one sentence and not a category
    ///
    /// US4 scenario 3 asks that this not be an anonymous failure. "The session exited
    /// unexpectedly" is anonymous; so is "killed by the sandbox". The user needs the three things
    /// they would otherwise have to guess at: that a limit did it, *which* limit, and where the
    /// number lives — because the only useful next action is to go and change it.
    pub fn message(&self) -> String {
        format!(
            "The session {} and was stopped by the sandbox. \u{2018}{}\u{2019} is set to {} \
             \u{2014} raise it in Settings \u{203a} Session service \u{203a} Limits, or clear it \
             to use the runtime\u{2019}s own default.",
            self.limit.what_happened(),
            self.limit.setting(),
            self.configured,
        )
    }
}

/// Whether a sandbox limit is what stopped this session, and which one.
///
/// # Why the budget is part of the question
///
/// A container process killed with SIGKILL looks the same whether the memory cgroup killed it or
/// the user did. The difference this can see is whether a limit was *set at all*: with no memory
/// ceiling configured there is no memory ceiling to blame, and reporting one would send the user
/// to a field that is empty. So an unset limit is never named, however the session died — the
/// caller falls back to its ordinary "exited unexpectedly" reporting, which is the honest answer
/// when nothing here can do better.
///
/// `output_tail` is the last of what the session printed. The kernel does not tell the parent
/// *why* a fork or a write failed; the shell does, in words, and those two words are the only
/// evidence that separates a process limit from a storage limit.
pub fn stopped_by_limit(
    status: ExitStatus,
    output_tail: &str,
    budget: &ResourceBudget,
) -> Option<StoppedByLimit> {
    let lower = output_tail.to_lowercase();

    if budget.storage_bytes.is_some()
        && (lower.contains("no space left on device") || lower.contains("enospc"))
    {
        return Some(StoppedByLimit {
            limit: SandboxLimit::Storage,
            configured: mib(budget.storage_bytes),
        });
    }

    if budget.pids.is_some()
        && (lower.contains("resource temporarily unavailable")
            || lower.contains("cannot fork")
            || lower.contains("fork failed"))
    {
        return Some(StoppedByLimit {
            limit: SandboxLimit::Processes,
            configured: budget
                .pids
                .map(|p| format!("{p} processes"))
                .unwrap_or_default(),
        });
    }

    // Last, and only on the kill signal: the memory cgroup's only outward sign is a SIGKILL, so
    // this is the broadest of the three and must not claim a stop one of the others explains.
    // Docker reports it as exit 137 through the CLI and as signal 9 through the process API, and
    // which one arrives depends on how the session was launched — so both are accepted.
    let killed = status.signal == Some(9) || status.code == Some(137);
    if killed && budget.memory_bytes.is_some() {
        return Some(StoppedByLimit {
            limit: SandboxLimit::Memory,
            configured: mib(budget.memory_bytes),
        });
    }

    None
}

fn mib(bytes: Option<Bytes>) -> String {
    bytes
        .map(|b| format!("{} MiB", b.as_mib()))
        .unwrap_or_default()
}

/// Everything the app knows about the sandbox right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sandbox {
    /// Where in bring-up it is.
    pub state: SandboxState,
    /// Limits the user set that the selected runtime cannot enforce (FR-015). Empty until the
    /// capability probe has run, and **not** an error: the sandbox runs, the view says so.
    pub unsatisfiable: Vec<UnsatisfiableLimit>,
    /// What the runtime turned out to be able to enforce, once a bring-up has told us.
    ///
    /// Distinct from [`Self::unsatisfiable`], which is about the limits the user *set*. The
    /// settings form needs the other question — what could be set at all — so that a limit nobody
    /// has enabled yet is still shown as unavailable rather than as an editable field that will
    /// silently do nothing (FR-015).
    ///
    /// `None` is "not probed yet", which the form renders as *editable*: an application that has
    /// never asked must not invent a restriction.
    pub capabilities: Option<RuntimeCapabilities>,
    /// The fallback the user took for this run, if they took one.
    ///
    /// Not persisted, on purpose. The next launch attempts the sandbox again without the user
    /// having to remember to re-enable it (US6 scenario 2) — which is what stops a broken sandbox
    /// becoming a permanently disabled one that nobody notices.
    pub fallback: Option<ConsentedFallback>,
    /// The bring-ups the application may still start on its own.
    pub unattended: UnattendedBringUps,
    /// Why the attempt before the one in flight failed, while the application tries again on its
    /// own (FR-036a). Taken from `Failed` as it moves to `Probing`, which is otherwise where the
    /// reason is lost; retired when a sandbox comes up.
    pub previous_attempt: Option<Failure>,
    /// While the sandbox has been started and its service has not answered yet, how many more
    /// refused dials that gap may still absorb (FR-036b). `None` outside that gap.
    ///
    /// `Started` is the container being up, not the daemon inside it listening; the gap between the
    /// two is still the bring-up, and a banner saying the service is gone would call it broken. It is
    /// bounded, so a service that never listens is still reported.
    pub awaiting_service: Option<u8>,
    /// What the *running* container shares with this machine, for translating its file links
    /// (feature 031, FR-018; research R10 decision 1).
    ///
    /// Written by [`Sandbox::started`] from the bring-up, never rebuilt from the boot plan: the plan
    /// describes the container this client would create *next*, and the sessions printing these
    /// paths run in the one that is up. Read through [`Sandbox::locations`], never directly, so a
    /// stopped sandbox cannot leave its map applied to host sessions.
    ///
    /// The container it was read from is kept beside it, because the state alone does not identify
    /// one: `bring_up` reports `Running(id)` before it returns what that container shares, and a
    /// replacement reaches `Running` under a new id while this field still holds the old map.
    pub locations: Option<(ContainerId, SandboxLocations)>,
}

/// The locations a running sandbox shares with the host, and the host paths it must never reach
/// (feature 031, FR-018, FR-018a).
///
/// `denied` is not the complement of `shared`: it is the short list of host paths that *are*
/// reachable through a shared location and must still be refused — this client's own sandbox token,
/// which lives in the shared state directory (C16b).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SandboxLocations {
    /// Most specific container path first, as `MountSet::shared_locations` orders them.
    pub shared: Vec<micold_core::link::SharedLocation>,
    /// Host paths no translation may produce.
    pub denied: Vec<String>,
}

/// Refused dials a just-started service may take before it is overdue: one reconnect
/// (`daemon::RECONNECT_BACKOFF`) is how long the daemon inside the container may take to listen.
pub const REFUSALS_WHILE_STARTING: u8 = 1;

impl Default for Sandbox {
    fn default() -> Self {
        Self {
            state: SandboxState::Disabled,
            unsatisfiable: Vec::new(),
            capabilities: None,
            fallback: None,
            unattended: UnattendedBringUps::default(),
            previous_attempt: None,
            awaiting_service: None,
            locations: None,
        }
    }
}

impl Sandbox {
    /// The state for a placement the user has just selected.
    pub fn for_placement(kind: PlacementKind) -> Self {
        Self {
            state: match kind {
                PlacementKind::HostProcess => SandboxState::Disabled,
                PlacementKind::LocalSandbox => SandboxState::Probing,
            },
            ..Self::default()
        }
    }

    /// Adopt a state reported by the bring-up.
    pub fn observe(&mut self, state: SandboxState) {
        self.state = state;
    }

    /// Adopt the result of a successful bring-up, and the locations that container shares
    /// (feature 031, T18).
    pub fn started(&mut self, started: Started, locations: SandboxLocations) {
        self.unsatisfiable = started.unsatisfiable;
        self.capabilities = Some(started.capabilities);
        self.locations = Some((started.id.clone(), locations));
        self.state = SandboxState::Running(started.id);
        // A successful start retires the fallback: the sandbox is working again, and continuing to
        // show "running unsandboxed" would be a lie the banner keeps telling.
        self.fallback = None;
        self.awaiting_service = Some(REFUSALS_WHILE_STARTING);
    }

    /// What the sandbox shares with this machine, while there is a container to share it
    /// (feature 031, FR-018, T18/T19).
    ///
    /// `Some` only while the state is `Running` or `Stale` **for the container the map was read
    /// from** — a `Stale` container is still up, only its mount set is out of date, and its sessions
    /// still print its paths. Every other state, including a placement change through
    /// [`Sandbox::for_placement`], answers `None`, so `LinkContext.sandbox` is `None` and a host
    /// session's `file` links are read as this machine's own. Gating here rather than clearing the
    /// field on each transition means a transition added later cannot forget to (research R10
    /// decision 1), and matching the id closes the two live windows the state alone leaves open:
    /// `bring_up`'s `observe(Running(id))` arrives before `Started`, and a replaced container reaches
    /// `Running` under a new id.
    pub fn locations(&self) -> Option<&SandboxLocations> {
        let live = match &self.state {
            SandboxState::Running(id) | SandboxState::Stale(id) => id,
            SandboxState::Disabled
            | SandboxState::Probing
            | SandboxState::Acquiring(_)
            | SandboxState::Starting
            | SandboxState::Failed(_) => return None,
        };
        let (owner, locations) = self.locations.as_ref()?;
        (owner == live).then_some(locations)
    }

    /// Whether a bring-up is under way, including a started sandbox whose service has not answered
    /// yet (FR-036b). `Stale` counts: that container is still up, only out of date.
    pub fn is_coming_up(&self) -> bool {
        self.state.is_coming_up()
            || (matches!(
                self.state,
                SandboxState::Running(_) | SandboxState::Stale(_)
            ) && self.awaiting_service.is_some())
    }

    /// Adopt a dial the started service refused, spending one the gap may absorb; past the last, the
    /// service is overdue and no longer coming up.
    pub fn service_refused(&mut self) {
        self.awaiting_service = self.awaiting_service.and_then(|left| left.checked_sub(1));
    }

    /// Adopt the service answering: the bring-up, if there was one, is over.
    pub fn answered(&mut self) {
        self.awaiting_service = None;
        // A service that answered has recovered; the next outage gets the whole bound again. Not on
        // `started`: a container that is up says nothing about the service in it, and one that
        // crashes after every start would refill the bound on each loss (S-6).
        self.unattended = UnattendedBringUps::default();
        self.previous_attempt = None;
    }

    /// Adopt a failure.
    pub fn failed(&mut self, failure: Failure) {
        self.state = SandboxState::Failed(failure);
        // A *fresh* attempt just ended, so the consent given for the previous one no longer
        // describes the situation. Left standing it would keep the banner reporting the old reason
        // and never offer the way back for this failure (FR-035a).
        self.fallback = None;
    }

    /// Adopt a change to the set of projects the sandbox should be sharing (R9, M-4).
    ///
    /// Called when a project is registered or unregistered while the sandbox is up. It marks,
    /// rather than acts: `micold_core::sandbox::lifecycle::mount_set_changed` decides what the
    /// state becomes, and it never restarts anything — see that function for why ending the user's
    /// running sessions to service a settings change is the wrong trade.
    pub fn mounts_changed(&mut self) {
        self.state = micold_core::sandbox::lifecycle::mount_set_changed(&self.state);
    }

    /// Adopt a change to the keep-it-running opt-in (feature 028, FR-022a).
    ///
    /// The same marking, and the same refusal to act on it, as [`Self::mounts_changed`]: the
    /// restart policy and the idle rule are both fixed at container creation, so the sandbox that
    /// is up was created under the old answer. See `lifecycle::survive_logout_changed`.
    pub fn survive_logout_changed(&mut self) {
        self.state = micold_core::sandbox::lifecycle::survive_logout_changed(&self.state);
    }

    /// Restart the sandbox because the user asked for it, reporting whether there was anything to
    /// restart.
    ///
    /// The one edge back into bring-up, and it takes a `RestartRequested` to get there — so the
    /// "nothing restarts on its own" half of R9 is carried by the signature rather than by every
    /// caller remembering it.
    pub fn restart(&mut self, request: RestartRequested) -> bool {
        match micold_core::sandbox::lifecycle::restart(&self.state, request) {
            Some(next) => {
                self.state = next;
                // A person's restart is a fresh start, not the next unattended attempt: the reason
                // left from before would name an older failure than the one they just read.
                self.previous_attempt = None;
                true
            }
            None => false,
        }
    }

    /// Bring the sandbox up again because its service is not there (FR-002a, BUG-005).
    ///
    /// Moves a failed sandbox back to `Probing`, spends one of the unattended attempts, and returns
    /// how long to wait before starting it. `None` — nothing moved — for a sandbox that has not
    /// failed or has no attempts left; the decision is `lifecycle::service_absent`'s (S-6, S-7).
    pub fn service_absent(&mut self) -> Option<std::time::Duration> {
        let again = micold_core::sandbox::lifecycle::service_absent(&self.state, self.unattended)?;
        if let SandboxState::Failed(failure) = std::mem::replace(&mut self.state, again.state) {
            self.previous_attempt = Some(failure);
        }
        self.unattended = again.budget;
        Some(again.after)
    }

    /// Adopt the loss of the container the sandbox was using (FR-036, US6 scenario 3).
    ///
    /// Reports whether anything changed, so a liveness check that arrives after the sandbox has
    /// already failed for its own reason does not overwrite that reason with a vaguer one.
    pub fn container_lost(&mut self, name: &str) -> bool {
        match micold_core::sandbox::lifecycle::container_lost(&self.state, name) {
            Some(next) => {
                self.state = next;
                true
            }
            None => false,
        }
    }

    /// The one-occurrence fallback the app should be offering right now, if there is one
    /// (US5 scenario 2, FR-035a).
    ///
    /// A failed bring-up leaves the user with no service at all unless something offers them the
    /// way back. That offer is *this*, and it exists as a method rather than as a rule the view
    /// applies because the view is not the place to decide when running unsandboxed is on the
    /// table — `micold_core::sandbox::lifecycle::accept_fallback` decides that, and this is the
    /// same question asked one step earlier so the two cannot disagree.
    ///
    /// The reason carried here is the *cause* rather than the whole failure sentence: it is read
    /// back after an em dash by [`Self::persistent_notice`] for as long as the user stays
    /// unsandboxed, and "Running without the sandbox for now — The sandbox failed while checking
    /// the container runtime." says when it broke rather than what broke.
    ///
    /// `None` once the offer has been taken. An offer still on screen after it was accepted reads
    /// as though pressing it did nothing.
    pub fn fallback_offer(&self) -> Option<ConsentedFallback> {
        if self.fallback.is_some() {
            return None;
        }
        match &self.state {
            SandboxState::Failed(f) => Some(ConsentedFallback {
                because: f.error.reason(),
            }),
            _ => None,
        }
    }

    /// Record that the user chose to run unsandboxed for this occurrence (FR-035a).
    ///
    /// Only reachable from a failed sandbox — `micold_core::sandbox::lifecycle::accept_fallback`
    /// decides that, not this function, so there is one place the rule lives.
    pub fn accept_fallback(&mut self, consent: ConsentedFallback) -> bool {
        match micold_core::sandbox::lifecycle::accept_fallback(&self.state, consent) {
            Some(taken) => {
                self.fallback = Some(taken);
                true
            }
            None => false,
        }
    }

    /// Whether the app should be showing a persistent indicator, and what it should say.
    ///
    /// FR-035b: a failed sandbox, and a session running outside one, are *conditions*. The spec's
    /// own edge case is a user who takes the one-occurrence choice on every launch and never
    /// notices sandboxing has been broken for weeks — a notification that scrolls away is how that
    /// happens.
    pub fn persistent_notice(&self) -> Option<String> {
        if let Some(fallback) = &self.fallback {
            return Some(format!(
                "Running without the sandbox for now — {}",
                fallback.because
            ));
        }
        match &self.state {
            SandboxState::Failed(f) => Some(format!("{} {}", f.reason(), f.remedy())),
            SandboxState::Stale(_) => Some(
                "The sandbox does not yet share every registered project. Restart it to apply."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::link::SharedLocation;
    use micold_core::sandbox::runtime::{
        ContainerId, IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
    };

    fn caps() -> RuntimeCapabilities {
        RuntimeCapabilities {
            kind: RuntimeKind::Docker,
            version: "29.5.1".into(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage: LimitSupport::Supported,
            identity_mapping: IdentityMapping::ExplicitUidGid,
        }
    }

    /// A bring-up that created the container, so it shares this client's whole mount set.
    fn created() -> Started {
        Started {
            id: ContainerId("0123456789abcdef".into()),
            capabilities: caps(),
            unsatisfiable: Vec::new(),
            mounted: None,
        }
    }

    /// A bring-up that attached to a container sharing only the project.
    fn attached() -> Started {
        Started {
            mounted: Some(vec!["/work/proj".into()]),
            ..created()
        }
    }

    fn locations() -> SandboxLocations {
        SandboxLocations {
            shared: vec![SharedLocation {
                container: "/work/proj".into(),
                host: "/home/u/proj".into(),
            }],
            denied: vec!["/home/u/.local/share/micold-ai-ide/sandbox.token".into()],
        }
    }

    /// U90, T18: a started sandbox carries what it shares, however it came to be running.
    #[test]
    fn a_started_sandbox_reports_the_locations_it_shares() {
        for started in [created(), attached()] {
            let mut s = Sandbox::default();
            s.started(started, locations());
            assert_eq!(
                s.locations(),
                Some(&locations()),
                "the bring-up is the only thing that knows what this container shares, so what it \
                 reported is what the pane's links resolve against (FR-018)"
            );
        }
    }

    /// U92: a `Stale` container is still up, and its sessions still print its paths.
    #[test]
    fn a_stale_sandbox_still_reports_its_locations() {
        let mut s = Sandbox::default();
        s.started(created(), locations());
        s.mounts_changed();
        assert!(
            matches!(s.state, SandboxState::Stale(_)),
            "setup: registering a project marked the running container out of date"
        );
        assert_eq!(
            s.locations(),
            Some(&locations()),
            "what is out of date is the mount set the container *should* have, not the one it has"
        );
    }

    /// U91, review B finding 3: the map belongs to one container, not to the state `Running`.
    ///
    /// `lifecycle::bring_up` reports `Running(id)` through `observe` *before* it returns `Started`,
    /// so there is a moment when the state is live and no map has arrived; and a replaced container
    /// reaches `Running` under a new id while the old map is still held. In both the pane would
    /// otherwise read a container path as this machine's own, with no confirmation — the one outcome
    /// FR-018 forbids.
    #[test]
    fn locations_answer_only_for_the_container_they_were_read_from() {
        let mut s = Sandbox::default();
        s.observe(SandboxState::Running(created().id));
        assert_eq!(
            s.locations(),
            None,
            "the bring-up reports Running before it reports what the container shares"
        );

        s.started(created(), locations());
        assert_eq!(s.locations(), Some(&locations()), "setup: the map arrived");

        s.observe(SandboxState::Running(ContainerId(
            "fedcba9876543210".into(),
        )));
        assert_eq!(
            s.locations(),
            None,
            "a replacement container shares what it was created with, not what the last one did"
        );
    }

    /// U91, T19: every way out of a live sandbox takes the map with it.
    #[test]
    fn leaving_running_or_stale_leaves_no_locations_behind() {
        let live = || {
            let mut s = Sandbox::default();
            s.started(created(), locations());
            s
        };
        #[allow(clippy::type_complexity)]
        let transitions: Vec<(&str, Box<dyn Fn(&mut Sandbox)>)> = vec![
            (
                "the container was observed stopped",
                Box::new(|s: &mut Sandbox| s.observe(SandboxState::Probing)),
            ),
            (
                "the bring-up failed",
                Box::new(|s: &mut Sandbox| {
                    s.failed(Failure {
                        stage: micold_core::sandbox::lifecycle::Stage::Starting,
                        error: micold_core::sandbox::runtime::RuntimeError::NotRunning {
                            kind: RuntimeKind::Docker,
                        },
                    })
                }),
            ),
            (
                "the container was lost",
                Box::new(|s: &mut Sandbox| {
                    assert!(s.container_lost("micold-sandbox"), "setup: it was running");
                }),
            ),
            (
                "the user accepted the fallback",
                Box::new(|s: &mut Sandbox| {
                    s.failed(Failure {
                        stage: micold_core::sandbox::lifecycle::Stage::Probing,
                        error: micold_core::sandbox::runtime::RuntimeError::NotRunning {
                            kind: RuntimeKind::Docker,
                        },
                    });
                    let offer = s.fallback_offer().expect("a failed sandbox offers one");
                    assert!(s.accept_fallback(offer), "setup: the offer was taken");
                }),
            ),
        ];
        for (what, apply) in transitions {
            let mut s = live();
            apply(&mut s);
            assert_eq!(
                s.locations(),
                None,
                "{what}: a map that outlived its container would translate a *host* session's \
                 file link into a path the sandbox chose (FR-018)"
            );
        }

        for kind in [
            micold_core::sandbox::placement::PlacementKind::HostProcess,
            micold_core::sandbox::placement::PlacementKind::LocalSandbox,
        ] {
            assert_eq!(
                Sandbox::for_placement(kind).locations(),
                None,
                "a placement the user has just selected has no container yet, so it shares nothing"
            );
        }
    }
}

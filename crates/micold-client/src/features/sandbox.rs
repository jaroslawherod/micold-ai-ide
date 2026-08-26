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
use micold_core::sandbox::lifecycle::{Failure, RestartRequested, SandboxState, Started};
use micold_core::sandbox::placement::{ConsentedFallback, PlacementKind};
use micold_core::sandbox::runtime::{RuntimeCapabilities, UnsatisfiableLimit};
use micold_core::sandbox::{Bytes, ResourceBudget};

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
}

impl Default for Sandbox {
    fn default() -> Self {
        Self {
            state: SandboxState::Disabled,
            unsatisfiable: Vec::new(),
            capabilities: None,
            fallback: None,
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

    /// Adopt the result of a successful bring-up.
    pub fn started(&mut self, started: Started) {
        self.unsatisfiable = started.unsatisfiable;
        self.capabilities = Some(started.capabilities);
        self.state = SandboxState::Running(started.id);
        // A successful start retires the fallback: the sandbox is working again, and continuing to
        // show "running unsandboxed" would be a lie the banner keeps telling.
        self.fallback = None;
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
                true
            }
            None => false,
        }
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
    use micold_core::sandbox::lifecycle::Stage;
    use micold_core::sandbox::runtime::{ContainerId, RuntimeError, RuntimeKind};

    fn failure() -> Failure {
        Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        }
    }

    #[test]
    fn the_host_placement_leaves_the_sandbox_disabled() {
        assert_eq!(
            Sandbox::for_placement(PlacementKind::HostProcess).state,
            SandboxState::Disabled
        );
    }

    #[test]
    fn a_fallback_is_only_reachable_from_a_failure() {
        let consent = ConsentedFallback {
            because: "Docker is not installed".into(),
        };

        let mut running = Sandbox {
            state: SandboxState::Running(ContainerId("x".into())),
            ..Sandbox::default()
        };
        assert!(!running.accept_fallback(consent.clone()));
        assert!(running.fallback.is_none());

        let mut failed = Sandbox {
            state: SandboxState::Failed(failure()),
            ..Sandbox::default()
        };
        assert!(failed.accept_fallback(consent));
        assert!(failed.fallback.is_some());
    }

    #[test]
    fn an_accepted_fallback_stays_visible_for_as_long_as_it_lasts() {
        // FR-035b. The spec's edge case is a user who takes this on every launch and never notices
        // sandboxing has been broken for weeks.
        let mut s = Sandbox {
            state: SandboxState::Failed(failure()),
            ..Sandbox::default()
        };
        s.accept_fallback(ConsentedFallback {
            because: "Docker is not installed".into(),
        });
        let notice = s.persistent_notice().expect("a notice while unsandboxed");
        assert!(notice.contains("without the sandbox"));
        assert!(notice.contains("Docker is not installed"));
    }

    #[test]
    fn a_successful_start_retires_the_fallback() {
        let mut s = Sandbox {
            state: SandboxState::Failed(failure()),
            ..Sandbox::default()
        };
        s.accept_fallback(ConsentedFallback {
            because: "was not installed".into(),
        });

        s.started(Started {
            id: ContainerId("x".into()),
            capabilities: caps(),
            unsatisfiable: Vec::new(),
        });
        assert!(
            s.fallback.is_none(),
            "the banner must stop claiming we are unsandboxed"
        );
        assert!(s.persistent_notice().is_none());
    }

    #[test]
    fn a_failure_notice_names_the_cause_and_the_remedy() {
        let mut s = Sandbox::default();
        s.failed(failure());
        let notice = s.persistent_notice().unwrap();
        assert!(notice.contains("not installed"));
        // FR-034: never a dead end.
        assert!(notice.contains("Settings") || notice.contains("Install"));
    }

    #[test]
    fn unenforceable_limits_survive_a_successful_start() {
        // FR-015: the sandbox runs, and the view still has to say which limit is not being applied.
        let mut s = Sandbox::default();
        s.started(Started {
            id: ContainerId("x".into()),
            capabilities: caps(),
            unsatisfiable: vec![UnsatisfiableLimit {
                field: "storage",
                reason: "overlay2 without pquota".into(),
            }],
        });
        assert_eq!(s.unsatisfiable.len(), 1);
        // ...but it is not a failure, so nothing persistent is shown for it. The daemon section
        // renders it beside the field it belongs to.
        assert!(s.persistent_notice().is_none());
    }

    // -----------------------------------------------------------------------------------------
    // T096 — a runtime that cannot be used says which way it cannot, and leaves a way forward
    // (US5 scenario 2)
    // -----------------------------------------------------------------------------------------

    /// The three answers a probe can give, for whichever runtime the user selected.
    fn detect_failures(kind: RuntimeKind) -> [Failure; 3] {
        [
            RuntimeError::NotInstalled { kind },
            RuntimeError::NotRunning { kind },
            RuntimeError::PermissionDenied { kind },
        ]
        .map(|error| Failure {
            stage: Stage::Probing,
            error,
        })
    }

    /// Each of the three reads differently, and each ends somewhere the user can go.
    ///
    /// The three are not interchangeable: "install it", "start it" and "you are not allowed to use
    /// it" send a user to three different places, and a probe that collapses them into "the
    /// sandbox did not start" costs them the afternoon. Asserted for both runtimes, because the
    /// notice names the one the user actually selected — telling a podman user to start Docker is
    /// the same dead end wearing a different word.
    #[test]
    fn each_way_a_runtime_can_be_unusable_reads_differently_and_names_the_runtime() {
        for kind in RuntimeKind::ALL {
            let mut notices = Vec::new();
            for failure in detect_failures(kind) {
                let mut s = Sandbox::for_placement(PlacementKind::LocalSandbox);
                s.failed(failure);

                let notice = s
                    .persistent_notice()
                    .unwrap_or_else(|| panic!("{kind}: a failed probe must be visible"));
                assert!(
                    notice.contains(kind.label()),
                    "{kind}: the notice must name the runtime the user selected: {notice}"
                );
                assert!(
                    notice.len() > "The sandbox failed while checking the container runtime.".len(),
                    "{kind}: a cause with no next step is a dead end: {notice}"
                );
                notices.push(notice);
            }
            notices.sort();
            notices.dedup();
            assert_eq!(
                notices.len(),
                3,
                "{kind}: the three answers must not collapse into one"
            );
        }
    }

    /// A failed probe is never the end of the road.
    ///
    /// FR-035 forbids falling back on the user's behalf, which leaves exactly one requirement in
    /// its place: that the way back is *offered*. Without this the app has no working service path
    /// at all after a failed detect — the sandbox refuses sessions, and nothing else is on the
    /// table.
    #[test]
    fn a_failed_probe_offers_the_way_back_and_taking_it_leaves_a_working_service() {
        for kind in RuntimeKind::ALL {
            for failure in detect_failures(kind) {
                let mut s = Sandbox::for_placement(PlacementKind::LocalSandbox);
                s.failed(failure);

                assert!(
                    !s.state.accepts_sessions(),
                    "{kind}: a failed sandbox must not quietly serve sessions"
                );
                let offer = s
                    .fallback_offer()
                    .unwrap_or_else(|| panic!("{kind}: a failure with no offer is a dead end"));
                assert!(
                    offer.because.contains(kind.label()),
                    "{kind}: the user consents to something named: {}",
                    offer.because
                );

                assert!(s.accept_fallback(offer.clone()), "{kind}");
                let notice = s
                    .persistent_notice()
                    .unwrap_or_else(|| panic!("{kind}: running unsandboxed must stay visible"));
                assert!(notice.contains(&offer.because), "{kind}: {notice}");
                assert!(
                    s.fallback_offer().is_none(),
                    "{kind}: an offer already taken must stop being offered"
                );
            }
        }
    }

    /// Nothing else offers it — in particular, a sandbox that is working does not.
    /// A retry that fails again is a *new* failure, and the banner has to say so.
    ///
    /// The sequence is the ordinary one: the sandbox fails, the user runs without it, presses
    /// "Try the sandbox again", and it fails for a different reason. If the accepted fallback
    /// survives that, the banner keeps naming the *first* reason and never offers the way back for
    /// the second — the user would be looking at a stale explanation of a problem that has changed
    /// underneath them (FR-035a, FR-035b).
    #[test]
    fn a_retry_that_fails_again_reports_the_new_reason_and_offers_again() {
        let mut s = Sandbox::for_placement(PlacementKind::LocalSandbox);
        s.failed(Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        });
        let offer = s
            .fallback_offer()
            .expect("a failed sandbox offers the way back");
        assert!(s.accept_fallback(offer));

        assert!(
            s.restart(micold_core::sandbox::lifecycle::RestartRequested),
            "a failed sandbox can be restarted"
        );
        s.failed(Failure {
            stage: Stage::Acquiring,
            error: RuntimeError::ImageNotFound {
                reference: "ghcr.io/micold/sandbox:0.8.0".to_string(),
            },
        });

        let notice = s
            .persistent_notice()
            .expect("a failed sandbox is a standing condition");
        assert!(
            notice.contains("getting the sandbox image"),
            "the banner should name the failure that just happened, not the one before it: {notice}"
        );
        assert!(
            s.fallback_offer().is_some(),
            "the way back has to be offered again for the new failure"
        );
    }

    #[test]
    fn no_offer_is_made_from_a_state_that_has_not_failed() {
        for state in [
            SandboxState::Disabled,
            SandboxState::Probing,
            SandboxState::Starting,
            SandboxState::Running(ContainerId("x".into())),
            SandboxState::Stale(ContainerId("x".into())),
        ] {
            let s = Sandbox {
                state: state.clone(),
                ..Sandbox::default()
            };
            assert!(
                s.fallback_offer().is_none(),
                "{state:?} is not a failure and must not offer a way out of the sandbox"
            );
        }
    }

    // -----------------------------------------------------------------------------------------
    // T088 — a session stopped by a limit is not an anonymous failure (US4 scenario 3)
    // -----------------------------------------------------------------------------------------

    fn budget() -> ResourceBudget {
        ResourceBudget {
            cpus_milli: Some(micold_core::sandbox::MilliCpus(2000)),
            memory_bytes: Some(Bytes::from_mib(2048)),
            pids: Some(256),
            storage_bytes: Some(Bytes::from_mib(4096)),
        }
    }

    fn killed() -> ExitStatus {
        ExitStatus {
            code: None,
            signal: Some(9),
        }
    }

    /// The report names the limit, the setting, and what it is set to.
    #[test]
    fn an_out_of_memory_stop_names_the_setting_that_governs_it() {
        let stop =
            stopped_by_limit(killed(), "", &budget()).expect("a kill under a memory ceiling");

        assert_eq!(stop.limit, SandboxLimit::Memory);
        let message = stop.message();
        assert!(
            message.contains("Memory limit"),
            "the report must name the control by the name the form gives it: {message}"
        );
        assert!(
            message.contains("2048 MiB"),
            "and say what it is currently set to, so the user knows what they are raising: \
             {message}"
        );
        assert!(message.contains("Settings"), "and where to go: {message}");
    }

    /// Docker's CLI reports the same kill as exit 137.
    #[test]
    fn the_cli_form_of_the_same_kill_is_recognised() {
        let status = ExitStatus {
            code: Some(137),
            signal: None,
        };
        assert_eq!(
            stopped_by_limit(status, "", &budget()).map(|s| s.limit),
            Some(SandboxLimit::Memory)
        );
    }

    /// With no memory ceiling set there is nothing to blame, and blaming it anyway would send the
    /// user to an empty field.
    #[test]
    fn a_kill_with_no_memory_limit_set_is_not_attributed_to_one() {
        let budget = ResourceBudget {
            memory_bytes: None,
            ..budget()
        };
        assert_eq!(stopped_by_limit(killed(), "", &budget), None);
    }

    /// The two limits the kernel does not signal are told apart by what the session printed.
    #[test]
    fn a_full_disk_and_an_exhausted_process_table_are_distinguished() {
        let disk = stopped_by_limit(
            ExitStatus {
                code: Some(1),
                signal: None,
            },
            "cp: error writing 'out.bin': No space left on device",
            &budget(),
        )
        .expect("a full sandbox is a stop with a cause");
        assert_eq!(disk.limit, SandboxLimit::Storage);
        assert!(disk.message().contains("4096 MiB"));

        let forks = stopped_by_limit(
            ExitStatus {
                code: Some(1),
                signal: None,
            },
            "bash: fork: retry: Resource temporarily unavailable",
            &budget(),
        )
        .expect("an exhausted process table is a stop with a cause");
        assert_eq!(forks.limit, SandboxLimit::Processes);
        assert!(forks.message().contains("256 processes"));
    }

    /// A specific cause outranks the general one.
    ///
    /// A session that filled the disk and was then killed would otherwise be reported as an
    /// out-of-memory stop, sending the user to raise a ceiling that had nothing to do with it.
    #[test]
    fn a_named_cause_wins_over_the_bare_kill_signal() {
        let stop = stopped_by_limit(killed(), "No space left on device", &budget())
            .expect("still a stop with a cause");
        assert_eq!(stop.limit, SandboxLimit::Storage);
    }

    /// An ordinary exit is not a limit, and must not be dressed as one.
    #[test]
    fn an_ordinary_exit_reports_nothing() {
        let status = ExitStatus {
            code: Some(0),
            signal: None,
        };
        assert_eq!(stopped_by_limit(status, "goodbye", &budget()), None);
    }

    /// The processor limit never appears. It throttles; it does not stop anything.
    ///
    /// Stated as a test rather than left to the enum's shape, because "add the fourth limit for
    /// symmetry" is exactly the change someone would make without noticing that a throttled
    /// session has not stopped, and that blaming the CPU share would be blaming nothing.
    #[test]
    fn the_processor_limit_is_never_blamed() {
        for output in [
            "",
            "No space left on device",
            "Resource temporarily unavailable",
        ] {
            let named = stopped_by_limit(killed(), output, &budget()).map(|s| s.limit.setting());
            assert_ne!(
                named,
                Some("Processor limit"),
                "a CPU share slows a session down; it never stops one"
            );
        }
    }

    fn caps() -> micold_core::sandbox::runtime::RuntimeCapabilities {
        use micold_core::sandbox::runtime::{IdentityMapping, LimitSupport, RuntimeCapabilities};
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
}

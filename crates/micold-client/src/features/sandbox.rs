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

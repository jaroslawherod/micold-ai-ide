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

use micold_core::sandbox::lifecycle::{Failure, SandboxState, Started};
use micold_core::sandbox::placement::{ConsentedFallback, PlacementKind};
use micold_core::sandbox::runtime::UnsatisfiableLimit;

/// Everything the app knows about the sandbox right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sandbox {
    /// Where in bring-up it is.
    pub state: SandboxState,
    /// Limits the user set that the selected runtime cannot enforce (FR-015). Empty until the
    /// capability probe has run, and **not** an error: the sandbox runs, the view says so.
    pub unsatisfiable: Vec<UnsatisfiableLimit>,
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
        self.state = SandboxState::Running(started.id);
        // A successful start retires the fallback: the sandbox is working again, and continuing to
        // show "running unsandboxed" would be a lie the banner keeps telling.
        self.fallback = None;
    }

    /// Adopt a failure.
    pub fn failed(&mut self, failure: Failure) {
        self.state = SandboxState::Failed(failure);
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

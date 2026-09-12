//! The client-side sandbox state, in isolation (feature 027; feature 021 SC-004).
//!
//! Moved out of `features/sandbox.rs` when this branch met main's one-file-per-feature convention:
//! the module was written before it, and an inline `mod tests` is the one shape the convention
//! does not allow. Nothing about the coverage changed — the same assertions, against the same
//! feature's own types, from outside the crate.
//!
//! It names no other feature. The decisions all live in `micold_core::sandbox::lifecycle`, so what
//! is checked here is the part that is specific to this application being open: which state a
//! placement starts in, when the one-occurrence fallback is on the table, and what the persistent
//! notice says while it lasts.

use micold_client::features::sandbox::{stopped_by_limit, Sandbox, SandboxLimit};
use micold_core::protocol::messages::ExitStatus;
use micold_core::sandbox::lifecycle::{Failure, SandboxState, Stage, Started};
use micold_core::sandbox::placement::{ConsentedFallback, PlacementKind};
use micold_core::sandbox::runtime::{
    ContainerId, IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeError, RuntimeKind,
    UnsatisfiableLimit,
};
use micold_core::sandbox::{Bytes, ResourceBudget};

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
            reference: "ghcr.io/example/sandbox:0.8.0".to_string(),
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
    let stop = stopped_by_limit(killed(), "", &budget()).expect("a kill under a memory ceiling");

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

//! Bringing the sandbox up, end to end against the injected fake (feature 027, FR-032 … FR-036).
//!
//! `sandbox_runtime.rs` covers one runtime call at a time. This covers the *sequence*: that each
//! stage is entered in order, that progress reaches the observer while the image is being acquired,
//! that a failure names the stage it happened in — and, the one that matters, that no path through
//! it produces a working unsandboxed daemon.

use std::path::{Path, PathBuf};

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
use micold_core::sandbox::image::{ImageSource, ImageSourceKind};
use micold_core::sandbox::lifecycle::{
    bring_up, container_lost, mount_set_changed, restart, service_absent, survive_logout_changed,
    RestartRequested, SandboxState, Stage, UnattendedBringUps, UNATTENDED_BRING_UP_DELAYS,
};
use micold_core::sandbox::runtime::{ContainerId, Progress, RuntimeError, RuntimeKind};
use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

/// The fingerprint the client claims. Every `find` below answers "no such container", so the
/// adoption branch under test is always `Create` — the adoption decisions themselves are covered
/// by `lifecycle`'s own tests, which drive them directly rather than through eight canned
/// responses.
const FINGERPRINT: &str = "b7f3a1c9";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/runtime")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

fn mounts() -> MountSet {
    MountSet::build(
        &[PathBuf::from("/home/u/p")],
        &SandboxProfile::default(),
        &CredentialLayout::default(),
        PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        SecretMount {
            host: PathBuf::from("/run/user/1000/micold/sandbox.token"),
            container: PathBuf::from("/run/micold/token"),
        },
    )
}

fn spec(profile: &SandboxProfile) -> SandboxSpec {
    SandboxSpec {
        name: "micold-sandbox".into(),
        profile: profile.clone(),
        mounts: mounts(),
        uid: 1000,
        gid: 1000,
        control_port: 7727,
        published_ports: Vec::new(),
        network_name: "micold-net".into(),
        home: PathBuf::from("/home/u"),
    }
}

/// The happy path, with the fake standing in for every runtime call.
///
/// Responses in order: probe's version, probe's info, image inspect (present), network create,
/// create's probe version, create's probe info, create, start.
fn happy_runner() -> RecordingRunner {
    let r = RecordingRunner::new();
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push_ok(fixture("docker_inspect_image.json"));
    // No sandbox is already there, so the adoption decision is `Create`.
    r.push(Ok(CommandOutput::err(
        1,
        "Error: No such container: micold-sandbox",
    )));
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push_ok("9f2b1c4d7e8a");
    r.push_ok("");
    r
}

#[test]
fn a_successful_bring_up_walks_the_stages_in_order() {
    let profile = SandboxProfile::default();
    let rt = CliRuntime::new(RuntimeKind::Docker, happy_runner());

    let mut seen = Vec::new();
    let started = bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |s| seen.push(s),
    )
    .expect("bring up");

    assert_eq!(started.id.0, "9f2b1c4d7e8a");

    let names: Vec<&str> = seen
        .iter()
        .map(|s| match s {
            SandboxState::Probing => "probing",
            SandboxState::Acquiring(_) => "acquiring",
            SandboxState::Starting => "starting",
            SandboxState::Running(_) => "running",
            other => panic!("unexpected state {other:?}"),
        })
        .collect();
    assert_eq!(names.first(), Some(&"probing"));
    assert_eq!(names.last(), Some(&"running"));
    assert!(names.contains(&"acquiring"));
    assert!(names.contains(&"starting"));
}

/// SC-004: the acquiring stage reports progress rather than going quiet. The image is the only part
/// of this that can take minutes, and it is the first thing a new user sees.
#[test]
fn acquisition_progress_reaches_the_observer() {
    let profile = SandboxProfile {
        image: ImageSource {
            kind: ImageSourceKind::Registry,
            reference: "micold-daemon:0.27.0".into(),
            path: None,
        },
        ..SandboxProfile::default()
    };

    let r = RecordingRunner::new();
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push(Ok(CommandOutput::err(1, "Error: No such image")));
    r.push_ok("layer a: Downloading\nlayer b: Downloading\nStatus: Downloaded");
    r.push_ok(fixture("docker_inspect_image.json"));
    r.push(Ok(CommandOutput::err(
        1,
        "Error: No such container: micold-sandbox",
    )));
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push_ok("9f2b");
    r.push_ok("");
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let mut progress = Vec::new();
    bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |s| {
            if let SandboxState::Acquiring(p) = s {
                progress.push(p);
            }
        },
    )
    .expect("bring up");

    assert!(
        progress.len() >= 2,
        "five silent minutes reads as a hang; progress was {progress:?}"
    );
}

/// A failure names the stage it happened in. "The sandbox failed" is not actionable; "failed while
/// getting the sandbox image" tells the user which setting to look at (FR-034).
#[test]
fn a_failure_names_the_stage_and_carries_a_remedy() {
    let profile = SandboxProfile::default();

    // Probe fails: the runtime is not running.
    let r = RecordingRunner::new();
    r.push(Ok(CommandOutput::err(125, fixture("err_daemon_down.txt"))));
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let failure = bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |_| {},
    )
    .expect_err("a downed runtime cannot bring a sandbox up");
    assert_eq!(failure.stage, Stage::Probing);
    assert!(matches!(failure.error, RuntimeError::NotRunning { .. }));
    assert!(failure.reason().contains("checking the container runtime"));
    assert!(!failure.remedy().trim().is_empty());
}

/// The stage is the one that actually failed, not the first one. A failure attributed to the wrong
/// stage sends the user to the wrong setting.
#[test]
fn a_late_failure_is_attributed_to_its_own_stage() {
    let profile = SandboxProfile::default();

    let r = RecordingRunner::new();
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push_ok(fixture("docker_inspect_image.json"));
    r.push(Ok(CommandOutput::err(
        1,
        "Error: No such container: micold-sandbox",
    )));
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    // `create` fails: the port is taken.
    r.push(Ok(CommandOutput::err(
        125,
        fixture("err_port_unavailable.txt"),
    )));
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let failure = bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |_| {},
    )
    .expect_err("a taken port cannot be created over");
    assert_eq!(failure.stage, Stage::Creating);
    assert!(matches!(
        failure.error,
        RuntimeError::PortUnavailable { .. }
    ));
}

/// FR-035: a failed bring-up yields a failure, never a placement. Stated as a type-level fact —
/// `bring_up` returns `Result<Started, Failure>` and `Started` carries a `ContainerId` — but
/// asserted anyway, because the way this guarantee is usually lost is a caller that maps the error
/// away, and a test that names it is what makes that visible in review.
#[test]
fn a_failed_bring_up_yields_no_container_at_all() {
    let profile = SandboxProfile::default();
    let r = RecordingRunner::new();
    r.push(Ok(CommandOutput::err(127, "docker: command not found")));
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let result = bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |_| {},
    );
    assert!(result.is_err());
    // There is no `unwrap_or_default`, no fallback container, and no second attempt against a
    // different placement. The caller is handed the failure and has to decide.
    match result {
        Err(f) => assert!(matches!(f.error, RuntimeError::NotInstalled { .. })),
        Ok(started) => panic!("a missing runtime produced a container: {started:?}"),
    }
}

/// FR-015: limits this runtime cannot enforce come back as a list, so the view can say so. The
/// sandbox still runs — an unenforceable limit is not an error — but the user is not left believing
/// a bound exists.
#[test]
fn unenforceable_limits_are_reported_without_failing_the_bring_up() {
    use micold_core::sandbox::{Bytes, ResourceBudget};

    let profile = SandboxProfile {
        budget: ResourceBudget {
            storage_bytes: Some(Bytes::from_mib(8192)),
            ..ResourceBudget::default()
        },
        ..SandboxProfile::default()
    };

    let r = RecordingRunner::new();
    r.push_ok(fixture("docker_version.json"));
    // overlay2: cannot enforce a size limit without xfs + pquota.
    r.push_ok(r#"{"Driver":"overlay2"}"#);
    r.push_ok(fixture("docker_inspect_image.json"));
    r.push(Ok(CommandOutput::err(
        1,
        "Error: No such container: micold-sandbox",
    )));
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(r#"{"Driver":"overlay2"}"#);
    r.push_ok("9f2b");
    r.push_ok("");
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let started = bring_up(
        &rt,
        &profile,
        &mounts(),
        FINGERPRINT,
        |_| spec(&profile),
        &mut |_| {},
    )
    .expect("an unenforceable limit is not a failure");
    assert_eq!(started.unsatisfiable.len(), 1);
    assert_eq!(started.unsatisfiable[0].field, "storage");
    assert!(started.unsatisfiable[0].reason.contains("pquota"));
}

// ---------------------------------------------------------------------------------------------
// T101 — rule M-4: registering a project marks the sandbox stale, and nothing restarts on its own
// (research R9)
// ---------------------------------------------------------------------------------------------

/// Every state the machine has, so the properties below are asserted over the whole space rather
/// than over the one path that happened to be written first.
fn every_state() -> Vec<SandboxState> {
    vec![
        SandboxState::Disabled,
        SandboxState::Probing,
        SandboxState::Acquiring(Progress {
            stage: "Downloading".into(),
            detail: None,
            percent: Some(40),
        }),
        SandboxState::Starting,
        SandboxState::Running(ContainerId("9f2b".into())),
        SandboxState::Stale(ContainerId("9f2b".into())),
        SandboxState::Failed(micold_core::sandbox::lifecycle::Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        }),
    ]
}

/// The transition itself. A container's mounts are fixed when it is created, so a project
/// registered afterwards is one the sandbox genuinely cannot see.
#[test]
fn registering_a_project_marks_a_running_sandbox_stale() {
    let id = ContainerId("9f2b".into());
    assert_eq!(
        mount_set_changed(&SandboxState::Running(id.clone())),
        SandboxState::Stale(id)
    );
}

/// M-4's other half, and the one that is easy to lose: the obliging behaviour is the wrong one.
///
/// Restarting to pick the new project up would end every session inside the container — the user's
/// actual work — to service a settings change they made in another window. So no state may move
/// *towards* bring-up here, and no container may be dropped on the floor.
#[test]
fn a_change_to_the_mount_set_never_restarts_anything() {
    for before in every_state() {
        let after = mount_set_changed(&before);

        let restarting = |s: &SandboxState| {
            matches!(
                s,
                SandboxState::Probing | SandboxState::Acquiring(_) | SandboxState::Starting
            )
        };
        assert!(
            !restarting(&after) || restarting(&before),
            "{before:?} began restarting itself and became {after:?}"
        );
        assert_eq!(
            after.container(),
            before.container(),
            "{before:?} lost or invented a container"
        );
    }
}

/// T046, FR-022a, research R2a.
///
/// Both halves of the keep-it-running opt-in — the restart policy and the idle rule — are fixed
/// when the container is created, so a container already running was made under the old answer and
/// cannot be talked into the new one. Without this the user toggles the control, the settings say
/// one thing, the container does the other, and nothing on screen admits it.
#[test]
fn toggling_the_keep_running_opt_in_marks_a_running_sandbox_stale() {
    let id = ContainerId("9f2b".into());
    assert_eq!(
        survive_logout_changed(&SandboxState::Running(id.clone())),
        SandboxState::Stale(id)
    );
}

/// And it is the same trade `mount_set_changed` makes, for the same reason: restarting to apply the
/// new answer would end every session inside the container to service a settings change.
#[test]
fn a_change_to_the_keep_running_opt_in_never_restarts_anything() {
    for before in every_state() {
        let after = survive_logout_changed(&before);

        let restarting = |s: &SandboxState| {
            matches!(
                s,
                SandboxState::Probing | SandboxState::Acquiring(_) | SandboxState::Starting
            )
        };
        assert!(
            !restarting(&after) || restarting(&before),
            "{before:?} began restarting itself and became {after:?}"
        );
        assert_eq!(
            after.container(),
            before.container(),
            "{before:?} lost or invented a container"
        );
    }
}

/// A stale sandbox is out of date, not out of order.
///
/// Refusing sessions here would make registering a project an outage for every session already
/// running in the container, which is the same failure as restarting, arrived at more slowly.
#[test]
fn a_stale_sandbox_still_serves_the_sessions_already_in_it() {
    let stale = mount_set_changed(&SandboxState::Running(ContainerId("9f2b".into())));
    assert!(stale.accepts_sessions());
    assert!(
        stale.is_persistent(),
        "and it has to keep saying so — a notice that scrolls away leaves the user wondering \
         why a project they registered is missing"
    );
}

/// The only edge back into bring-up from a running sandbox, and it takes an explicit request to
/// cross. The unattended edge out of `Failed` is `service_absent`'s, below (S-6, S-7).
#[test]
fn a_running_sandbox_restarts_only_on_an_explicit_request() {
    for before in every_state() {
        let after = restart(&before, RestartRequested);
        match &before {
            // The case R9 exists for, plus the two a user would reasonably ask for by hand.
            SandboxState::Stale(_) | SandboxState::Running(_) | SandboxState::Failed(_) => {
                assert_eq!(
                    after,
                    Some(SandboxState::Probing),
                    "{before:?} refused a restart the user asked for"
                );
            }
            // Nothing to restart: a disabled sandbox is not broken, and one already coming up
            // would be abandoned mid-attempt and started again from the top.
            _ => assert_eq!(
                after, None,
                "{before:?} restarted from a state with no sandbox"
            ),
        }
    }
}

/// A container stopped from outside leaves the app in a state it can name and act on (FR-036).
///
/// The failure this replaces is not a wrong message, it is *no* message: a client whose container
/// vanished keeps reconnecting to an address nothing is listening on, forever, with the last frame
/// still on screen. US6 scenario 3 asks for "recovers to a defined state rather than hanging", and
/// `Failed` is that state — it draws a banner and it carries a restart.
#[test]
fn a_container_stopped_from_outside_becomes_a_state_the_user_can_act_on() {
    let running = SandboxState::Running(ContainerId("abc123".into()));

    let after = container_lost(&running, "micold-sandbox").expect("a running sandbox can be lost");

    let SandboxState::Failed(failure) = &after else {
        panic!("expected a defined failure, got {after:?}");
    };
    assert!(
        failure.reason().contains("micold-sandbox"),
        "the message should name the container that went away: {}",
        failure.reason()
    );
    assert!(
        !failure.remedy().is_empty(),
        "FR-034: every failure carries a next step"
    );
    assert!(
        restart(&after, RestartRequested).is_some(),
        "the state it recovers to has to be one a restart can leave"
    );
}

/// A stale sandbox can be lost too — it is still serving the sessions inside it (M-4, FR-036).
#[test]
fn a_stale_sandbox_can_also_be_lost() {
    let stale = SandboxState::Stale(ContainerId("abc123".into()));
    assert!(container_lost(&stale, "micold-sandbox").is_some());
}

/// Nothing that never came up can be reported as lost.
///
/// The check runs on a dropped connection, and a connection can drop for reasons that have nothing
/// to do with the container. If a sandbox that already failed for a nameable reason were then
/// re-reported as "no longer running", the user would watch a precise message — a missing subuid
/// range, a rejected mount — be replaced by a vague one (FR-034).
#[test]
fn a_sandbox_that_never_ran_cannot_be_lost() {
    for before in every_state() {
        if before.accepts_sessions() {
            continue;
        }
        assert_eq!(
            container_lost(&before, "micold-sandbox"),
            None,
            "{before:?} was reported as a lost container"
        );
    }
}

// --- BUG-005 (T167, S-6/S-7, FR-002a, FR-036a): a sandbox found absent is brought up again -------

fn failed() -> SandboxState {
    SandboxState::Failed(micold_core::sandbox::lifecycle::Failure {
        stage: Stage::Starting,
        error: RuntimeError::SandboxStopped {
            name: "micold-sandbox".into(),
        },
    })
}

/// FR-002a. The host placement has always started its service when it found none there; this is the
/// same promise for the container, and it carries no `RestartRequested` because nobody pressed
/// anything — which is exactly the situation BUG-005 left without a way out.
#[test]
fn a_failed_sandbox_found_absent_is_brought_up_again_without_anyone_asking() {
    let budget = UnattendedBringUps::default();

    let again = service_absent(&failed(), budget)
        .expect("a sandbox that is not running, with attempts left, has to be brought up again");

    assert_eq!(again.state, SandboxState::Probing);
    assert_eq!(
        again.budget.remaining() + 1,
        budget.remaining(),
        "the attempt has to be paid for, or the bound below means nothing"
    );
}

/// S-6's guard and S-7's scope, over the whole state space. A bring-up already in flight is never
/// started a second time — the connection keeps failing at 1 Hz while one runs, and each failure
/// is not a new reason to begin again. A running or stale sandbox is R9's to protect, and an
/// absent *service* in front of a present container is `container_lost`'s question, not this one.
#[test]
fn absence_only_brings_up_a_sandbox_that_has_failed() {
    for before in every_state() {
        if matches!(before, SandboxState::Failed(_)) {
            continue;
        }
        assert_eq!(
            service_absent(&before, UnattendedBringUps::default()),
            None,
            "{before:?} was brought up again on a failed connection"
        );
    }
}

/// FR-036b's "not listening *yet*", over the whole state space. These are exactly the states a
/// bring-up passes through; a refused dial in any other one is not the bring-up still running, and
/// one missing from here is a working bring-up the client reports as a broken connection.
#[test]
fn only_a_bring_up_in_flight_is_coming_up() {
    let coming_up: Vec<SandboxState> = every_state()
        .into_iter()
        .filter(SandboxState::is_coming_up)
        .collect();

    assert!(
        matches!(
            coming_up.as_slice(),
            [
                SandboxState::Probing,
                SandboxState::Acquiring(_),
                SandboxState::Starting
            ]
        ),
        "a bring-up in flight is Probing, Acquiring and Starting, and nothing else: {coming_up:?}"
    );
}

/// More unattended bring-ups than any bound this application could mean. A loop that passes it has
/// no bound, and stops here instead of hanging the suite.
const NO_BOUND: usize = 10;

/// FR-036a, S-2. A runtime that is not installed fails in milliseconds; unbounded, this would be
/// a bring-up loop for as long as the application stayed open. Bounded, the failure ends up
/// standing with its reason and remedy on screen, which is where FR-034's manual path takes over.
#[test]
fn unattended_bring_ups_are_bounded_and_then_the_failure_stands() {
    let mut budget = UnattendedBringUps::default();
    let mut attempts = 0;
    while let Some(again) = service_absent(&failed(), budget) {
        attempts += 1;
        budget = again.budget;
        assert!(attempts <= NO_BOUND, "the bound never arrived");
    }

    assert!(
        attempts >= 1,
        "a bound of zero is the bug, not a fix for it"
    );
    assert_eq!(
        attempts,
        UNATTENDED_BRING_UP_DELAYS.len(),
        "the bound is one attempt per delay in the table"
    );
    assert_eq!(budget.remaining(), 0);
    assert_eq!(
        service_absent(&failed(), budget),
        None,
        "a spent budget has to stay spent"
    );
}

/// S-6: "spaced by their own backoff". The first attempt may go at once — the service is missing
/// *now* — and none after it waits less than the one before. How many there are is the bound test's
/// to say, and that the waits outlast the connection's own retry is checked where that retry is
/// defined, against the real value: `micold-client/src/daemon.rs`.
#[test]
fn unattended_bring_ups_never_wait_less_than_the_one_before() {
    let mut budget = UnattendedBringUps::default();
    let mut waits = Vec::new();
    while let Some(again) = service_absent(&failed(), budget) {
        waits.push(again.after);
        budget = again.budget;
        assert!(waits.len() <= NO_BOUND, "the bound never arrived");
    }

    assert!(
        waits.len() >= 2,
        "spacing needs at least two attempts to be between: {waits:?}"
    );
    for (i, wait) in waits.iter().enumerate().skip(1) {
        assert!(
            *wait >= waits[i - 1],
            "attempt {} waits less than the one before it: {waits:?}",
            i + 1
        );
    }
}

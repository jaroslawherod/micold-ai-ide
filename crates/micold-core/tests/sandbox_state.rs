//! Bringing the sandbox up, end to end against the injected fake (feature 027, FR-032 … FR-036).
//!
//! `sandbox_runtime.rs` covers one runtime call at a time. This covers the *sequence*: that each
//! stage is entered in order, that progress reaches the observer while the image is being acquired,
//! that a failure names the stage it happened in — and, the one that matters, that no path through
//! it produces a working unsandboxed daemon.

use std::path::PathBuf;

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
use micold_core::sandbox::image::{ImageSource, ImageSourceKind};
use micold_core::sandbox::lifecycle::{bring_up, SandboxState, Stage};
use micold_core::sandbox::runtime::{RuntimeError, RuntimeKind};
use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

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
    let started = bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |s| {
        seen.push(s)
    })
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
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    r.push_ok("9f2b");
    r.push_ok("");
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let mut progress = Vec::new();
    bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |s| {
        if let SandboxState::Acquiring(p) = s {
            progress.push(p);
        }
    })
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

    let failure = bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |_| {})
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
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(fixture("docker_info.json"));
    // `create` fails: the port is taken.
    r.push(Ok(CommandOutput::err(
        125,
        fixture("err_port_unavailable.txt"),
    )));
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let failure = bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |_| {})
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

    let result = bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |_| {});
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
    r.push_ok("micold-net");
    r.push_ok(fixture("docker_version.json"));
    r.push_ok(r#"{"Driver":"overlay2"}"#);
    r.push_ok("9f2b");
    r.push_ok("");
    let rt = CliRuntime::new(RuntimeKind::Docker, r);

    let started = bring_up(&rt, &profile, &mounts(), |_| spec(&profile), &mut |_| {})
        .expect("an unenforceable limit is not a failure");
    assert_eq!(started.unsatisfiable.len(), 1);
    assert_eq!(started.unsatisfiable[0].field, "storage");
    assert!(started.unsatisfiable[0].reason.contains("pquota"));
}

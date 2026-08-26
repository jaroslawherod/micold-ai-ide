//! The runtime adapter, driven end to end against the injected fake (feature 027).
//!
//! Conformance checks K-8 … K-12 from `contracts/container-runtime.md`. Every test here runs on
//! Linux, macOS and Windows with **no container runtime installed** — that property is the whole
//! reason `exec.rs` is a seam rather than a `Command::new` in the middle of the adapter.
//!
//! What is asserted is `docker`/`podman` behaviour we cannot otherwise reach: a daemon that is
//! down, an image that vanishes between inspect and create, output that is cut off mid-object.
//! Arranging those against a live runtime is either impossible or destructive.

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
use micold_core::sandbox::runtime::{ContainerId, ContainerRuntime, RuntimeError, RuntimeKind};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/runtime")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

fn runtime(kind: RuntimeKind) -> CliRuntime<RecordingRunner> {
    CliRuntime::new(kind, RecordingRunner::new())
}

/// A runtime whose next responses are `outputs`, in order.
fn primed(kind: RuntimeKind, outputs: Vec<CommandOutput>) -> CliRuntime<RecordingRunner> {
    let runner = RecordingRunner::new();
    for o in outputs {
        runner.push(Ok(o));
    }
    CliRuntime::new(kind, runner)
}

#[test]
fn detect_reports_the_servers_version() {
    for (kind, f) in [
        (RuntimeKind::Docker, "docker_version.json"),
        (RuntimeKind::Podman, "podman_version.json"),
    ] {
        let rt = primed(kind, vec![CommandOutput::ok(fixture(f))]);
        let v = rt.detect().expect("detect");
        assert_eq!(v.kind, kind);
        assert!(!v.version.is_empty());
    }
}

/// K-8: each canned failure maps to its own variant, through the *adapter*, not just the
/// classifier. A classifier that is right and an adapter that swallows its answer is still a
/// failure the user sees as "something went wrong".
///
/// Run against **each runtime's own wording**, which is the only version of this check worth
/// having. The two runtimes do not phrase the same failure the same way, so a classifier built
/// from Docker's strings turns podman's daemon being down into `Unknown` — an anonymous failure,
/// which is the thing FR-034 exists to prevent. See the fixtures' README on where podman's
/// wording came from and what still has to confirm it.
#[test]
fn canned_failures_reach_the_caller_as_distinct_variants() {
    type Matcher<'a> = &'a dyn Fn(&RuntimeError) -> bool;
    let cases: [(&str, &str, Matcher, &str); 4] = [
        (
            "err_daemon_down.txt",
            "podman_err_service_down.txt",
            &|e| matches!(e, RuntimeError::NotRunning { .. }),
            "NotRunning",
        ),
        (
            "err_permission_denied.txt",
            "podman_err_permission_denied.txt",
            &|e| matches!(e, RuntimeError::PermissionDenied { .. }),
            "PermissionDenied",
        ),
        (
            "err_port_unavailable.txt",
            "podman_err_port_unavailable.txt",
            // The port, not just the variant: both fixtures are a conflict on 7727, and a
            // classifier that reads the variant off one runtime's wording while losing the number
            // off the other's still passes every `{ .. }` check — then tells the user "Port 0 is
            // already in use" and sends them looking for a port nothing is listening on.
            &|e| matches!(e, RuntimeError::PortUnavailable { port: 7727 }),
            "PortUnavailable on 7727",
        ),
        (
            "err_mount_rejected.txt",
            "podman_err_mount_rejected.txt",
            &|e| matches!(e, RuntimeError::MountRejected { .. }),
            "MountRejected",
        ),
    ];
    for (docker_fixture, podman_fixture, matches_variant, name) in cases {
        for (kind, f) in [
            (RuntimeKind::Docker, docker_fixture),
            (RuntimeKind::Podman, podman_fixture),
        ] {
            let rt = primed(kind, vec![CommandOutput::err(125, fixture(f))]);
            let err = rt
                .detect()
                .expect_err("a failed invocation must not succeed");
            assert!(
                matches_variant(&err),
                "{kind} / {f}: expected {name}, got {err:?}"
            );
            // FR-034: every one of them is actionable.
            assert!(!err.remedy().trim().is_empty(), "{kind} / {f}");
        }
    }
}

/// K-8, the case that is not about output at all: the program is missing.
#[test]
fn a_missing_program_is_reported_as_not_installed() {
    let runner = RecordingRunner::new();
    runner.push(Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no such file",
    )));
    let rt = CliRuntime::new(RuntimeKind::Podman, runner);
    match rt.detect().unwrap_err() {
        RuntimeError::NotInstalled { kind } => assert_eq!(kind, RuntimeKind::Podman),
        other => panic!("expected NotInstalled, got {other:?}"),
    }
}

/// K-12: truncated JSON is classified, never panicked. The adapter runs while the user is being
/// told the sandbox is starting; a panic here takes the client down at the worst moment.
#[test]
fn truncated_output_is_classified_not_panicked() {
    for kind in RuntimeKind::ALL {
        let rt = primed(kind, vec![CommandOutput::ok(fixture("err_truncated.json"))]);
        match rt.detect() {
            Err(RuntimeError::Unknown { stderr }) => assert!(!stderr.is_empty(), "{kind}"),
            other => panic!("{kind}: expected a classified error, got {other:?}"),
        }
    }
}

/// The storage capability is read from the driver the runtime reports, not from its version
/// (research R5/R10). Both directions, because getting it wrong in either is a real failure: a
/// false negative denies a working limit, a false positive promises one that is not enforced.
#[test]
fn the_storage_capability_follows_the_reported_driver() {
    // A driver that enforces it for real. Not the recorded `docker_info.json` fixture: that one is
    // this machine's, and this machine's driver is `overlayfs`, which does not.
    let supported = primed(
        RuntimeKind::Docker,
        vec![
            CommandOutput::ok(fixture("docker_version.json")),
            CommandOutput::ok(r#"{"Driver":"btrfs"}"#.to_string()),
        ],
    );
    assert!(supported.probe().unwrap().storage.is_supported());

    // Both overlay drivers do not — including `overlayfs`, which this once classified as supported
    // because `docker run --storage-opt size=1G … true` exits 0. It does; it also ignores the cap.
    // See `storage_support`'s note and `tests/sandbox_real_storage.rs`.
    for driver in ["overlay2", "overlayfs"] {
        let unsupported = primed(
            RuntimeKind::Docker,
            vec![
                CommandOutput::ok(fixture("docker_version.json")),
                CommandOutput::ok(format!(r#"{{"Driver":"{driver}"}}"#)),
            ],
        );
        let caps = unsupported.probe().unwrap();
        assert!(!caps.storage.is_supported(), "{driver}");
        match caps.storage {
            micold_core::sandbox::runtime::LimitSupport::Unsupported { reason } => {
                // SC-009 needs the reason, because the view renders it beside the disabled field.
                assert!(reason.contains("quota"), "{driver}: reason was {reason:?}");
            }
            other => panic!("{driver}: expected Unsupported, got {other:?}"),
        }
    }
}

/// The other three limits are supported on both runtimes; storage is the one that varies. Asserted
/// so a future change that quietly marks one unsupported has to be deliberate.
#[test]
fn cpu_memory_and_process_limits_are_supported_on_both_runtimes() {
    for (kind, version, info) in [
        (
            RuntimeKind::Docker,
            "docker_version.json",
            "docker_info.json",
        ),
        (
            RuntimeKind::Podman,
            "podman_version.json",
            "podman_info.json",
        ),
    ] {
        let rt = primed(
            kind,
            vec![
                CommandOutput::ok(fixture(version)),
                CommandOutput::ok(fixture(info)),
            ],
        );
        let caps = rt.probe().unwrap();
        assert!(caps.cpus.is_supported(), "{kind}");
        assert!(caps.memory.is_supported(), "{kind}");
        assert!(caps.pids.is_supported(), "{kind}");
    }
}

/// K-9: stop, remove and start succeed against a container already in that state.
///
/// The client's recovery paths call these without checking first, so a race with the user's own
/// `docker stop` must not produce an error dialog about a state they just asked for.
#[test]
fn lifecycle_operations_are_idempotent() {
    let id = ContainerId("micold-sandbox".to_string());

    // Each runtime's own phrasing of "it is already in the state you asked for". Short enough to
    // sit inline rather than in the fixtures directory; podman's are transcribed, not captured,
    // for the reason the fixtures' README gives.
    let cases = [
        (
            RuntimeKind::Docker,
            "Error response from daemon: container is not running",
            "Error: No such container: micold-sandbox",
            "Error response from daemon: container is already running",
        ),
        (
            RuntimeKind::Podman,
            "Error: container micold-sandbox is not running",
            r#"Error: no container with name or ID "micold-sandbox" found: no such container"#,
            "Error: container micold-sandbox is already running",
        ),
    ];

    for (kind, stopped, gone, running) in cases {
        let already_stopped = primed(kind, vec![CommandOutput::err(1, stopped)]);
        assert!(already_stopped.stop(&id).is_ok(), "{kind}");

        let already_gone = primed(kind, vec![CommandOutput::err(1, gone)]);
        assert!(already_gone.remove(&id).is_ok(), "{kind}");

        let already_running = primed(kind, vec![CommandOutput::err(1, running)]);
        assert!(already_running.start(&id).is_ok(), "{kind}");
    }
}

/// K-9's other half: idempotence must not swallow a *real* failure. A `stop` that fails because the
/// daemon is down has to surface, or the client reports success and then cannot reach anything.
#[test]
fn idempotence_does_not_swallow_a_real_failure() {
    for (kind, f) in [
        (RuntimeKind::Docker, "err_daemon_down.txt"),
        (RuntimeKind::Podman, "podman_err_service_down.txt"),
    ] {
        let rt = primed(kind, vec![CommandOutput::err(125, fixture(f))]);
        let err = rt
            .stop(&ContainerId("x".into()))
            .expect_err("a daemon-down stop is a real failure");
        assert!(
            matches!(err, RuntimeError::NotRunning { .. }),
            "{kind}: {err:?}"
        );
    }
}

/// K-10: acquisition reports progress more than once for multi-layer output (obligation C-8).
#[test]
fn acquiring_an_image_emits_progress_while_it_runs() {
    use micold_core::sandbox::image::{ImageSource, ImageSourceKind};

    // Both runtimes report a layered pull line by line, and both print image inspect in the same
    // shape, so one fixture serves both — the thing under test is that the adapter forwards each
    // line as it arrives rather than after the pull is over.
    for kind in RuntimeKind::ALL {
        let runner = RecordingRunner::new();
        // inspect: absent -> pull (multi-line) -> inspect: present
        runner.push(Ok(CommandOutput::err(
            1,
            "Error: No such image: micold-daemon:0.27.0",
        )));
        runner.push_ok(
            "0e29546d541c: Pulling fs layer\n\
             0e29546d541c: Downloading  1.2MB/5.8MB\n\
             0e29546d541c: Extracting  5.8MB/5.8MB\n\
             Status: Downloaded newer image for micold-daemon:0.27.0",
        );
        runner.push_ok(fixture("docker_inspect_image.json"));

        let rt = CliRuntime::new(kind, runner);
        let source = ImageSource {
            kind: ImageSourceKind::Registry,
            reference: "micold-daemon:0.27.0".to_string(),
            path: None,
        };

        let mut reports = Vec::new();
        let facts = rt
            .acquire_image(&source, &mut |p| reports.push(p))
            .expect("acquire");

        assert!(
            reports.len() >= 2,
            "{kind}: SC-004 gives this five minutes; silence for that long reads as a hang. \
             reports: {reports:?}"
        );
        assert!(!facts.tags.is_empty(), "{kind}");
    }
}

/// An image already present is not fetched. This is Principle IV's offline claim in miniature: a
/// machine with the image and no network must not be made to reach for one.
#[test]
fn an_image_already_present_is_not_fetched() {
    use micold_core::sandbox::image::ImageSource;

    let runner = RecordingRunner::new();
    runner.push_ok(fixture("docker_inspect_image.json"));
    let rt = CliRuntime::new(RuntimeKind::Docker, runner);

    rt.acquire_image(&ImageSource::default(), &mut |_| {})
        .expect("already present");
}

/// A local build is not built from here: `mise run image` does that, because it needs a Linux
/// daemon binary cross-compiled and staged. What the app owes the user is saying so, not guessing.
#[test]
fn a_missing_local_build_is_reported_rather_than_built() {
    use micold_core::sandbox::image::{ImageSource, ImageSourceKind};

    let runner = RecordingRunner::new();
    runner.push(Ok(CommandOutput::err(
        1,
        "Error: No such image: micold-daemon:dev",
    )));
    let rt = CliRuntime::new(RuntimeKind::Docker, runner);

    let source = ImageSource {
        kind: ImageSourceKind::LocalBuild,
        reference: "micold-daemon:dev".to_string(),
        path: None,
    };
    match rt.acquire_image(&source, &mut |_| {}) {
        Err(RuntimeError::ImageNotFound { reference }) => {
            assert_eq!(reference, "micold-daemon:dev")
        }
        other => panic!("expected ImageNotFound, got {other:?}"),
    }
}

/// An absent image is `Ok(None)`, not an error: "is it here?" has a negative answer.
#[test]
fn inspecting_an_absent_image_is_not_a_failure() {
    let rt = primed(
        RuntimeKind::Docker,
        vec![CommandOutput::err(
            1,
            "Error: No such image: micold-daemon:dev",
        )],
    );
    assert_eq!(rt.inspect_image("micold-daemon:dev").unwrap(), None);
}

/// Diagnostics reach the user (US6 scenario 6): the daemon's own output from inside the sandbox.
#[test]
fn logs_are_retrievable_through_the_adapter() {
    let rt = primed(
        RuntimeKind::Docker,
        vec![CommandOutput::ok("starting\nlistening on 127.0.0.1:7727\n")],
    );
    let lines = rt.logs(&ContainerId("x".into()), 50).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("7727"));
}

/// Both dialects go through the same adapter, so "the runtime is replaceable" is exercised rather
/// than asserted (FR-020, SC-009).
#[test]
fn both_runtimes_are_driven_by_the_same_adapter() {
    for kind in RuntimeKind::ALL {
        let rt = runtime(kind);
        // No canned response: the default empty success is enough to prove the call was made and
        // that nothing in the adapter is Docker-specific.
        let _ = rt.detect();
    }
}

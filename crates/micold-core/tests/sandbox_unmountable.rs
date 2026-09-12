//! A project the runtime will not share fails by name (feature 027, T103, Edge Cases).
//!
//! The edge case is ordinary: a user keeps one repository on a team NFS share, or on a path their
//! container runtime excludes, and enables the sandbox. What the runtime says about that is
//! `invalid mount config for type "bind"`, which is true and useless — it names a category of
//! configuration, not the project, and there is nothing in it to act on.
//!
//! What this file pins is that the failure reaching the user names **the path** and **the reason**,
//! and that it is a classified `MountRejected` rather than an `Unknown` carrying the runtime's
//! stderr. The distinction is not cosmetic: `Unknown` is the variant FR-034 exists to keep rare,
//! and a failure mode this predictable landing there means every user who hits it gets the search
//! engine instead of the answer.

use std::path::{Path, PathBuf};

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
use micold_core::sandbox::lifecycle::bring_up;
use micold_core::sandbox::runtime::{RuntimeError, RuntimeKind};
use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

const FINGERPRINT: &str = "b7f3a1c9";

/// The project that lives somewhere the runtime will not go.
const SHARED: &str = "/mnt/team-share/webapp";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/runtime")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

/// Two projects, so the message has to pick the right one rather than being right by having no
/// alternative.
fn mounts() -> MountSet {
    MountSet::build(
        &[PathBuf::from("/home/u/p"), PathBuf::from(SHARED)],
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

/// Everything up to `create`, which then fails with `refusal`.
fn runner_rejecting_the_mount(kind: RuntimeKind, refusal: &str) -> RecordingRunner {
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
    r.push(Ok(CommandOutput::err(125, refusal)));
    let _ = kind;
    r
}

/// The refusal each runtime actually prints for a bind it will not make.
fn refusal(kind: RuntimeKind) -> String {
    match kind {
        RuntimeKind::Docker => format!(
            "docker: Error response from daemon: invalid mount config for type \"bind\": \
             bind source path does not exist: {SHARED}."
        ),
        RuntimeKind::Podman => {
            format!("Error: statfs {SHARED}: no such file or directory")
        }
    }
}

/// The whole of the requirement, for both runtimes: classified, named, and with a way forward.
#[test]
fn a_project_the_runtime_will_not_share_fails_by_name() {
    for kind in RuntimeKind::ALL {
        let profile = SandboxProfile {
            runtime: kind,
            ..SandboxProfile::default()
        };
        let rt = CliRuntime::new(kind, runner_rejecting_the_mount(kind, &refusal(kind)));

        let failure = bring_up(
            &rt,
            &profile,
            &mounts(),
            FINGERPRINT,
            |_| spec(&profile),
            &mut |_| {},
        )
        .expect_err("a mount the runtime refuses is a failed bring-up");

        match &failure.error {
            RuntimeError::MountRejected { path, detail } => {
                assert_eq!(
                    path, SHARED,
                    "{kind}: the wrong project was blamed, or none was"
                );
                assert!(
                    !detail.is_empty(),
                    "{kind}: the runtime's own words are what say *why*, and they are gone"
                );
            }
            other => panic!(
                "{kind}: a refused bind is a predictable failure and must not land in the \
                 unclassified bucket: {other:?}"
            ),
        }

        let reason = failure.reason();
        assert!(
            reason.contains(SHARED),
            "{kind}: the user cannot act on a path they are not told: {reason}"
        );
        assert!(
            !failure.remedy().is_empty(),
            "{kind}: FR-034 — every failure carries a next step"
        );
    }
}

/// The nearer path wins, so a project inside a rejected parent is named as itself.
///
/// Written because the obvious implementation — "the first candidate the message contains" — gets
/// this backwards whenever a mount's parent is also mounted, and blames a directory the user did
/// not register.
#[test]
fn the_project_is_named_rather_than_the_directory_that_contains_it() {
    let parent = std::path::Path::new("/mnt/team-share");
    let project = std::path::Path::new(SHARED);

    let error = RuntimeError::MountRejected {
        path: String::new(),
        detail: format!("bind source path does not exist: {SHARED}"),
    }
    .naming_mount(&[parent, project]);

    assert_eq!(
        error,
        RuntimeError::MountRejected {
            path: SHARED.to_string(),
            detail: format!("bind source path does not exist: {SHARED}"),
        }
    );
}

/// An unattributable rejection still reads as a sentence.
///
/// Empty backticks where a path should be look like a bug in this application rather than a
/// problem with a path, and send the user to the wrong place entirely.
#[test]
fn a_rejection_that_matches_no_mount_still_says_something_useful() {
    let error = RuntimeError::MountRejected {
        path: String::new(),
        detail: "operation not permitted".into(),
    };
    let reason = error.reason();
    assert!(!reason.contains("``"), "{reason}");
    assert!(reason.contains("operation not permitted"), "{reason}");
    assert!(!error.remedy().is_empty());
}

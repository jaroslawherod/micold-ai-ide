//! Which runtime is there, and if it is not usable, *why* (feature 027, US5 scenario 2 and 3).
//!
//! Conformance obligation C-6 says the answer is a classified value, never raw text. This file is
//! about the three answers that are easy to collapse into one and must not be, because the user's
//! next action differs completely between them:
//!
//! | Answer | What the user does |
//! |---|---|
//! | not installed | installs the runtime |
//! | not running | starts the service they already have |
//! | not permitted | fixes group membership or rootless setup |
//!
//! A single "cannot use Docker" covers all three and helps with none. So each is asserted to
//! reach the caller as its own variant, with its own remedy, for **both** runtimes — a classifier
//! written against Docker's wording answers `Unknown` for podman, and `Unknown` is the anonymous
//! failure FR-034 exists to prevent.
//!
//! Runs on Linux, macOS and Windows with no container runtime installed: everything here is above
//! the `exec.rs` seam.

use std::ffi::OsString;

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
use micold_core::sandbox::runtime::{ContainerRuntime, RuntimeError, RuntimeKind};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/runtime")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

/// The runtime is not on `PATH` at all: the spawn itself fails.
fn not_installed(kind: RuntimeKind) -> RuntimeError {
    let runner = RecordingRunner::new();
    runner.push(Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no such file or directory",
    )));
    CliRuntime::new(kind, runner).detect().unwrap_err()
}

/// The runtime ran and printed `fixture_name`.
fn spoke(kind: RuntimeKind, fixture_name: &str) -> RuntimeError {
    let runner = RecordingRunner::new();
    runner.push(Ok(CommandOutput::err(125, fixture(fixture_name))));
    CliRuntime::new(kind, runner).detect().unwrap_err()
}

fn wording(kind: RuntimeKind, docker: &'static str, podman: &'static str) -> &'static str {
    match kind {
        RuntimeKind::Docker => docker,
        RuntimeKind::Podman => podman,
    }
}

#[test]
fn a_runtime_that_is_not_installed_says_so_and_names_itself() {
    for kind in RuntimeKind::ALL {
        match not_installed(kind) {
            RuntimeError::NotInstalled { kind: named } => assert_eq!(named, kind),
            other => panic!("{kind}: expected NotInstalled, got {other:?}"),
        }
    }
}

#[test]
fn a_runtime_whose_service_is_down_is_not_reported_as_missing() {
    // The distinction the user feels most: told "install Docker" when Docker is installed and
    // stopped, they go and download something they already have.
    for kind in RuntimeKind::ALL {
        let f = wording(kind, "err_daemon_down.txt", "podman_err_service_down.txt");
        match spoke(kind, f) {
            RuntimeError::NotRunning { kind: named } => assert_eq!(named, kind),
            other => panic!("{kind} / {f}: expected NotRunning, got {other:?}"),
        }
    }
}

#[test]
fn a_runtime_this_user_may_not_drive_is_its_own_answer() {
    for kind in RuntimeKind::ALL {
        let f = wording(
            kind,
            "err_permission_denied.txt",
            "podman_err_permission_denied.txt",
        );
        match spoke(kind, f) {
            RuntimeError::PermissionDenied { kind: named } => assert_eq!(named, kind),
            other => panic!("{kind} / {f}: expected PermissionDenied, got {other:?}"),
        }
    }
}

#[test]
fn rootless_podman_without_a_subuid_range_is_a_permission_problem_not_an_unknown_one() {
    // Podman's characteristic first-run failure, and the one most likely to be misread: it is
    // installed, there is no daemon to start, and the fix is `usermod --add-subuids`. Classified
    // as anything else, the user is sent to install or restart something instead.
    match spoke(RuntimeKind::Podman, "podman_err_no_subuid.txt") {
        RuntimeError::PermissionDenied { kind } => assert_eq!(kind, RuntimeKind::Podman),
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
}

#[test]
fn a_subuid_range_too_small_to_use_is_the_same_answer_as_none_at_all() {
    // The same problem one step along: the user has a range, it is too short, and podman 5.8.4
    // says so in completely different words — mid-unpack, without "subuid" or "rootless" anywhere
    // in the sentence. Both fixtures were captured from podman 5.8.4 (T098); the first spelling
    // was the one the classifier knew, and this one was reaching the user as `Unknown` while the
    // remedy for both is the same `usermod`.
    match spoke(RuntimeKind::Podman, "podman_err_subuid_range_too_small.txt") {
        RuntimeError::PermissionDenied { kind } => assert_eq!(kind, RuntimeKind::Podman),
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
}

#[test]
fn the_three_answers_are_distinct_and_each_carries_its_own_next_step() {
    // The point of the classification, asserted as the property rather than as three separate
    // string checks: a remedy shared between two of them would send half the users somewhere
    // useless while every individual test still passed.
    for kind in RuntimeKind::ALL {
        let three = [
            not_installed(kind),
            spoke(
                kind,
                wording(kind, "err_daemon_down.txt", "podman_err_service_down.txt"),
            ),
            spoke(
                kind,
                wording(
                    kind,
                    "err_permission_denied.txt",
                    "podman_err_permission_denied.txt",
                ),
            ),
        ];

        for e in &three {
            assert!(!e.reason().trim().is_empty(), "{kind}: {e:?} has no reason");
            assert!(!e.remedy().trim().is_empty(), "{kind}: {e:?} has no remedy");
        }

        let reasons: Vec<String> = three.iter().map(|e| e.reason()).collect();
        let remedies: Vec<String> = three.iter().map(|e| e.remedy()).collect();
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            assert_ne!(reasons[i], reasons[j], "{kind}: two answers read the same");
            assert_ne!(
                remedies[i], remedies[j],
                "{kind}: two answers ask for the same fix"
            );
        }
    }
}

#[test]
fn the_runtime_the_user_did_not_select_is_never_invoked() {
    // US5 scenario 3. The seam is only worth having if selecting one runtime means the other is
    // not touched at all — not consulted for a version, not asked whether it is there. A "helpful"
    // fallback to the other runtime would be a sandbox the user did not ask for, running under a
    // different identity model than the one their settings describe.
    for selected in RuntimeKind::ALL {
        let log = RecordingRunner::new();
        let rt = CliRuntime::new(selected, &log);

        // A full pass over the surface rather than one call: a leak is likeliest in the paths that
        // are reached rarely, not in `detect`.
        let _ = rt.detect();
        let _ = rt.probe();
        let _ = rt.inspect_image("micold-daemon:dev");
        let _ = rt.find("micold-sandbox");

        let calls = log.calls();
        assert!(!calls.is_empty(), "{selected}: nothing was invoked at all");

        let other = match selected {
            RuntimeKind::Docker => RuntimeKind::Podman,
            RuntimeKind::Podman => RuntimeKind::Docker,
        };
        let unselected = OsString::from(other.program());
        let leaked: Vec<_> = calls
            .iter()
            .filter(|c| c.program == unselected)
            .map(|c| c.args_lossy())
            .collect();
        assert!(
            leaked.is_empty(),
            "{selected} was selected and {other} was invoked: {leaked:?}"
        );
        assert!(
            calls.iter().all(|c| c.program == *selected.program()),
            "{selected}: something other than the selected runtime was invoked: {calls:?}"
        );
    }
}

//! T115 — the default suite reaches no container runtime, on any platform.
//!
//! Principle VI's cross-platform coverage of the sandbox adapter rests on one property: with
//! nothing installed, `cargo test -p micold-core --all-targets` is green on Linux, macOS and
//! Windows. The CI matrix runs exactly that on all three, so the matrix *is* the verification —
//! provided the property holds. This file is what keeps it holding.
//!
//! It is a source-text gate rather than a behavioural one because the failure it prevents is not
//! observable here: a test that spawns `docker` passes on this machine, and on the Linux runner,
//! and fails only on the two runners without Docker Desktop — where the message is a `NotFound`
//! from deep inside an adapter, several layers from the `Command::new` that caused it. Catching it
//! by reading the source costs one file walk and names the offending path directly.

use std::path::Path;

/// Anything that spawns a process is one of these. `SystemRunner` is the real `CommandRunner`;
/// the two `Command::new` forms are what an adapter test would reach for if it stopped using the
/// seam. `exec.rs` exists so that this list can be short.
const REACHES_THE_HOST: &[&str] = &[
    "SystemRunner",
    "Command::new(\"docker\")",
    "Command::new(\"podman\")",
];

/// The prefix that marks a test as needing the real thing, and the feature that gates it.
const REAL_PREFIX: &str = "sandbox_real_";
const REAL_FEATURE: &str = "#![cfg(feature = \"sandbox-real-runtime\")]";

fn tests_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn integration_tests() -> Vec<(String, String)> {
    let dir = tests_dir().join("tests");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("the tests directory is readable") {
        let path = entry.expect("a readable entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path
                .file_stem()
                .expect("a named file")
                .to_string_lossy()
                .into_owned();
            let body = std::fs::read_to_string(&path).expect("a readable test file");
            out.push((name, body));
        }
    }
    assert!(out.len() > 5, "the walk found the tests: {}", out.len());
    out
}

/// Only a `sandbox_real_*` target may reach a container runtime.
///
/// The name is load-bearing twice over: it is how this gate recognises a test that needs the real
/// thing, and it is how the `sandbox-runtime` CI job *selects* one — `cargo test … sandbox_real_`
/// filters on the test's own path, not the file's. A target that reaches the host without the
/// prefix therefore breaks two platforms and is invisible to the job meant to cover it.
#[test]
fn only_the_real_runtime_targets_reach_a_container_runtime() {
    for (name, body) in integration_tests() {
        // This file names the offending strings in order to look for them.
        if name == "sandbox_needs_no_runtime" {
            continue;
        }
        for needle in REACHES_THE_HOST {
            if body.contains(needle) {
                assert!(
                    name.starts_with(REAL_PREFIX),
                    "tests/{name}.rs reaches the host with `{needle}`, but the default suite must \
                     run with no container runtime installed. Either drive `CliRuntime` through \
                     `exec::RecordingRunner`, or rename this target to `{REAL_PREFIX}*` and gate \
                     it on the feature."
                );
            }
        }
    }
}

/// And a `sandbox_real_*` target must actually be gated, not merely named.
///
/// Without the inner attribute the file compiles and runs in the default suite regardless of what
/// it is called, so the naming convention above would be decoration.
#[test]
fn every_real_runtime_target_is_gated_on_the_feature() {
    let mut checked = 0;
    for (name, body) in integration_tests() {
        if !name.starts_with(REAL_PREFIX) {
            continue;
        }
        assert!(
            body.contains(REAL_FEATURE),
            "tests/{name}.rs is named for the real runtime but carries no `{REAL_FEATURE}`, so it \
             runs in the default suite on every platform"
        );
        checked += 1;
    }
    assert!(
        checked >= 2,
        "the real-runtime targets were found: {checked}"
    );
}

/// The seam itself: the adapter reaches the process boundary in exactly one module.
///
/// If a second one appears, the property above becomes unenforceable — a test can then be
/// runtime-free by this file's reading and still spawn something.
#[test]
fn the_process_boundary_stays_in_one_module() {
    let src = tests_dir().join("src/sandbox");
    let mut spawners = Vec::new();
    for entry in std::fs::read_dir(&src).expect("the sandbox module is readable") {
        let path = entry.expect("a readable entry").path();
        if path.extension().is_some_and(|e| e == "rs")
            && std::fs::read_to_string(&path)
                .expect("a readable source file")
                .contains("std::process::Command")
        {
            spawners.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert_eq!(
        spawners,
        vec!["exec.rs".to_string()],
        "the sandbox layer must spawn processes only through `exec.rs`'s `CommandRunner` seam"
    );
}

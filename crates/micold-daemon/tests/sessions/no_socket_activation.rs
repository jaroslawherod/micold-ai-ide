//! Socket activation is gone, and stays gone (feature 028, T013 — FR-004, lifecycle contract §1.3).
//!
//! §1.3 says a service MUST NOT be startable by socket activation: the application is the only
//! thing that starts one. That is a statement about what *cannot happen*, and the only honest way
//! to check it from a test is to check that the machinery is absent — a daemon that cannot adopt a
//! listener on fd 3 cannot be activated onto one.
//!
//! Two absences, because either alone is reversible by accident. The source must not reach for an
//! inherited listener, and the manifest must not carry the crate that makes reaching for one a
//! two-line change. A `listenfd` dependency left declared is an unused dependency that the next
//! person reads as an invitation.
//!
//! Source-guard style follows `crates/micold-core/tests/documentation_is_not_read.rs`: the rule is
//! a function over its inputs, and synthetic inputs below prove it really does fail — otherwise
//! this is a gate nobody has ever seen go red.

use std::fs;
use std::path::{Path, PathBuf};

/// The two spellings of "adopt a listener somebody else opened".
///
/// `LISTEN_FDS` is the protocol (systemd's own variable); `listenfd` is the crate that reads it.
/// Neither is a term this codebase uses for anything else, so a plain substring is precise here.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "LISTEN_FDS",
        "the systemd socket-activation protocol variable — a daemon that reads it is a daemon a \
         service manager can start",
    ),
    (
        "listenfd",
        "the crate that adopts an inherited listener; `singleton::acquire` is the only bind path \
         this contract allows",
    ),
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under a directory, recursively.
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// One violation, phrased so the failure says what to do about it.
#[derive(Debug, PartialEq)]
struct Violation(String);

/// The rule over one file's text. Takes the text so its failure behaviour is demonstrable.
fn violations_in(label: &str, text: &str) -> Vec<Violation> {
    FORBIDDEN
        .iter()
        .filter(|(needle, _)| text.contains(needle))
        .map(|(needle, why)| {
            Violation(format!(
                "{label} names `{needle}` — {why} (lifecycle contract §1.3, FR-004)"
            ))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The real crate
// ---------------------------------------------------------------------------------------------

#[test]
fn no_daemon_source_adopts_an_inherited_listener() {
    let src = crate_dir().join("src");
    let sources = rust_sources(&src);
    assert!(
        !sources.is_empty(),
        "found no sources under {} — if the layout moved, this guard must move with it, or \
         §1.3 goes unchecked",
        src.display()
    );

    let mut found = Vec::new();
    for path in &sources {
        let text = fs::read_to_string(path).expect("read a daemon source");
        let label = path
            .strip_prefix(crate_dir())
            .unwrap_or(path)
            .display()
            .to_string();
        found.extend(violations_in(&label, &text));
    }

    assert!(
        found.is_empty(),
        "the daemon can still be socket-activated:\n{}",
        found
            .iter()
            .map(|v| format!("  {}", v.0))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_manifest_declares_no_listenfd_dependency() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).expect("read the manifest");
    assert!(
        !manifest.contains("listenfd"),
        "crates/micold-daemon/Cargo.toml still declares `listenfd`. Removing the adoption code \
         while leaving the dependency makes restoring it a two-line change, which is the opposite \
         of what lifecycle contract §1.3 asks for."
    );
}

// ---------------------------------------------------------------------------------------------
// The synthetic Red: the rule really does fail
// ---------------------------------------------------------------------------------------------

#[test]
fn a_clean_source_passes() {
    assert_eq!(
        violations_in(
            "src/server.rs",
            "let bound = singleton::acquire(&endpoint).await?;"
        ),
        vec![]
    );
}

#[test]
fn a_source_that_reads_the_environment_variable_fails() {
    let found = violations_in(
        "src/server.rs",
        "if std::env::var(\"LISTEN_FDS\").is_ok() { adopt() }",
    );
    assert_eq!(found.len(), 1, "expected exactly one violation: {found:?}");
    assert!(found[0].0.contains("LISTEN_FDS"), "{}", found[0].0);
}

#[test]
fn a_source_that_uses_the_crate_fails() {
    let found = violations_in(
        "src/server.rs",
        "let mut fds = listenfd::ListenFd::from_env();",
    );
    assert_eq!(found.len(), 1, "expected exactly one violation: {found:?}");
    assert!(found[0].0.contains("singleton::acquire"), "{}", found[0].0);
}

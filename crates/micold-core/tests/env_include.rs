//! Env-include resolution, decided and observed without a shell (feature 021, T043 — FR-019,
//! SC-005).
//!
//! # What this file is for, next to the two that already exist
//!
//! `env_include_resolve.rs` spawns **real** disposable subprocesses, deliberately: FR-005 says the
//! engine must actually source the script rather than parse it, and only a real shell can show
//! that. `env_include_diff.rs` covers the pure diffing. Neither can say anything about the
//! *decision* — whether to source at all — because reaching that question through the engine means
//! paying for a subprocess to observe something that has nothing to do with one.
//!
//! That decision is [`snapshot_for`], and this file is its behaviour, exercised through
//! [`FakeEnvIncludeResolver`] with zero real filesystem, repository or process access (SC-005).
//!
//! # Why the fake records calls rather than only returning answers
//!
//! The short-circuit's claim is not "the outcome is `Disabled`". It is "**no subprocess was
//! spawned**". Those come apart: an implementation that sourced the script and then threw the
//! result away would satisfy the first and violate the second, and a test reading only the outcome
//! would pass while the user waited on a shell nobody wanted. So the assertions here are about the
//! call log as much as the value.

use std::path::{Path, PathBuf};
use std::time::Duration;

use micold_core::env_include::{
    snapshot_for, EnvIncludeOutcome, EnvIncludeResolver, FakeEnvIncludeResolver,
};

const TIMEOUT: Duration = Duration::from_secs(5);

fn cwd() -> PathBuf {
    PathBuf::from("/projects/example")
}

/// A resolver that would succeed, if anything asked it to.
fn willing() -> FakeEnvIncludeResolver {
    FakeEnvIncludeResolver::answering(
        vec![("PATH".to_string(), "/opt/bin".to_string())],
        EnvIncludeOutcome::Success,
    )
}

#[test]
fn a_disabled_feature_spawns_nothing_and_reports_disabled() {
    let resolver = willing();

    let snapshot = snapshot_for(&resolver, false, "~/env.sh", TIMEOUT, &cwd());

    assert_eq!(snapshot.outcome, EnvIncludeOutcome::Disabled);
    assert!(
        snapshot.vars.is_empty(),
        "a disabled feature contributes no variables"
    );
    assert!(
        resolver.calls().is_empty(),
        "the feature is off, so nothing should have been sourced — the resolver was asked {:?}",
        resolver.calls()
    );
}

#[test]
fn a_blank_path_spawns_nothing_even_when_enabled() {
    // The spec's Edge Cases treat an empty *and* a whitespace-only path as "not configured". The
    // second is the one worth an assertion: `""` is obviously blank, `"   "` is what a settings
    // field actually contains after someone clears it.
    for path in ["", "   ", "\t\n"] {
        let resolver = willing();

        let snapshot = snapshot_for(&resolver, true, path, TIMEOUT, &cwd());

        assert_eq!(
            snapshot.outcome,
            EnvIncludeOutcome::Disabled,
            "a path of {path:?} is not a configured script"
        );
        assert!(
            resolver.calls().is_empty(),
            "a path of {path:?} should not have been sourced"
        );
    }
}

#[test]
fn an_enabled_script_is_sourced_exactly_once_with_what_it_was_given() {
    // The other half: when it *should* run, the port receives precisely the path, directory and
    // timeout the caller supplied. A resolver invoked with the wrong cwd resolves a different
    // environment — BUG-002 was exactly that — and no outcome assertion would notice.
    let resolver = willing();

    let snapshot = snapshot_for(&resolver, true, "/home/u/env.sh", TIMEOUT, &cwd());

    assert_eq!(snapshot.outcome, EnvIncludeOutcome::Success);
    assert_eq!(
        snapshot.vars,
        vec![("PATH".to_string(), "/opt/bin".to_string())],
        "the resolver's variables reach the caller unaltered"
    );
    assert_eq!(
        resolver.calls(),
        vec![(PathBuf::from("/home/u/env.sh"), cwd(), TIMEOUT)],
        "sourced once, with the path, directory and timeout it was given"
    );
}

#[test]
fn a_path_is_not_trimmed_before_it_is_sourced() {
    // Blankness is judged on the trimmed path; what gets sourced is the path as configured. Those
    // are different decisions, and collapsing them would silently "fix" a path with a trailing
    // space into one that resolves — a repair the user never asked for and cannot see.
    let resolver = willing();

    snapshot_for(&resolver, true, " /home/u/env.sh ", TIMEOUT, &cwd());

    assert_eq!(
        resolver.calls().first().map(|(path, ..)| path.clone()),
        Some(PathBuf::from(" /home/u/env.sh ")),
        "the configured path is sourced as configured"
    );
}

#[test]
fn every_failure_the_engine_can_report_reaches_the_caller_unchanged() {
    // A snapshot that flattened a failure into "no variables" would look identical to a success
    // that contributed none, and the Settings screen reports the outcome to the user verbatim.
    let failures = [
        EnvIncludeOutcome::MissingScript,
        EnvIncludeOutcome::NonZeroExit {
            code: 3,
            diagnostic: "line 4: nope".to_string(),
        },
        EnvIncludeOutcome::TimedOut {
            diagnostic: "still running".to_string(),
        },
    ];

    for outcome in failures {
        let resolver = FakeEnvIncludeResolver::answering(Vec::new(), outcome.clone());

        let snapshot = snapshot_for(&resolver, true, "/home/u/env.sh", TIMEOUT, &cwd());

        assert_eq!(
            snapshot.outcome, outcome,
            "the outcome must reach the caller as the engine reported it"
        );
        assert_eq!(
            resolver.calls().len(),
            1,
            "{outcome:?} was reached by sourcing"
        );
    }
}

#[test]
fn the_fake_is_a_usable_stand_in_for_the_port() {
    // FR-019's own claim, stated rather than assumed: the fake satisfies the capability, so a
    // consumer written against the trait takes it without knowing which it has.
    fn source_through(resolver: &dyn EnvIncludeResolver, path: &Path) -> EnvIncludeOutcome {
        resolver.resolve(path, Path::new("/tmp"), TIMEOUT).1
    }

    let resolver = willing();
    assert_eq!(
        source_through(&resolver, Path::new("/home/u/env.sh")),
        EnvIncludeOutcome::Success
    );
    assert_eq!(resolver.calls().len(), 1);
}

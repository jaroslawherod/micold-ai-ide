//! The OS theme probe, answered without an operating system (feature 021, T044 — FR-019, SC-005).
//!
//! # What this adds to `theme.rs`, which already tests the fallback
//!
//! `tests/theme.rs` feeds `observe_system_scheme` hand-written `Result` values and asserts what it
//! returns. That covers the decision perfectly and cannot cover two things:
//!
//! 1. **That the probe is substitutable at all.** FR-019's claim is that every I/O-dependent
//!    behaviour is reachable through a fake. A test that hand-writes the `Result` never touches the
//!    capability, so it says nothing about whether one exists or works.
//! 2. **Sequences.** BUG-001 is not a statement about one reading; it is about a *timeline* — the
//!    OS says dark, the next poll times out, and the screen must not flash to light. Hand-feeding
//!    one value at a time cannot express "and then", so the composed path — probe, observe, repeat
//!    — has never been exercised end to end.
//!
//! Everything here runs with zero operating-system access (SC-005).
//!
//! # Why the fake counts how often it is asked
//!
//! A consumer that caches when it should poll, or polls when it should not, still produces a
//! plausible scheme. The count is the difference between "the right answer" and "the right answer
//! for the right reason".

use micold_core::os_theme::{FakeOsThemeProbe, OsThemeProbe};
use micold_core::theme::{observe_system_scheme, SystemScheme};

/// Poll `probe` once per tick, folding each reading through the real fallback rule, and report the
/// scheme after each tick — the timeline a user would actually see.
fn observed_over(
    probe: &dyn OsThemeProbe,
    ticks: usize,
    initial: SystemScheme,
) -> Vec<SystemScheme> {
    let mut last = initial;
    (0..ticks)
        .map(|_| {
            last = observe_system_scheme(probe.detect(), last);
            last
        })
        .collect()
}

#[test]
fn a_working_probe_reports_what_the_os_says() {
    for scheme in [
        SystemScheme::Dark,
        SystemScheme::Light,
        SystemScheme::Unspecified,
    ] {
        let probe = FakeOsThemeProbe::always(scheme);

        assert_eq!(probe.detect(), Ok(scheme));
        assert_eq!(probe.times_asked(), 1, "asked exactly once for one reading");
    }
}

#[test]
fn a_failure_to_reach_the_os_is_not_an_answer_from_it() {
    // The distinction BUG-001 turned on. `Err(())` means the query failed; `Unspecified` means the
    // OS answered and has no preference. A capability that collapsed them would make the fallback
    // rule unable to tell "could not ask" from "no preference", which is how a dark desktop
    // flashed to light in the first place.
    let unreachable = FakeOsThemeProbe::failing();
    let no_preference = FakeOsThemeProbe::always(SystemScheme::Unspecified);

    assert_eq!(unreachable.detect(), Err(()));
    assert_eq!(no_preference.detect(), Ok(SystemScheme::Unspecified));
    assert_ne!(
        unreachable.detect(),
        no_preference.detect(),
        "a failed query and an expressed lack of preference must not be the same value"
    );
}

#[test]
fn a_transient_failure_between_two_readings_never_shows_through() {
    // The composed path, which is the point of this file: dark, then a timeout, then dark again.
    // `theme.rs` proves the rule; this proves the rule is what a polling consumer actually gets.
    let probe = FakeOsThemeProbe::scripted(vec![
        Ok(SystemScheme::Dark),
        Err(()),
        Ok(SystemScheme::Dark),
    ]);

    let seen = observed_over(&probe, 3, SystemScheme::Light);

    assert_eq!(
        seen,
        vec![SystemScheme::Dark, SystemScheme::Dark, SystemScheme::Dark],
        "the middle tick failed to reach the OS, and the screen must not move"
    );
    assert_eq!(probe.times_asked(), 3, "one query per tick");
}

#[test]
fn a_run_of_failures_holds_the_last_known_scheme_indefinitely() {
    // Not a one-tick property. Under sustained CPU load every poll can fail, and each one must
    // hold rather than the run eventually decaying to a default.
    let probe = FakeOsThemeProbe::scripted(vec![Ok(SystemScheme::Dark), Err(())]);

    let seen = observed_over(&probe, 6, SystemScheme::Light);

    assert!(
        seen.iter().all(|s| *s == SystemScheme::Dark),
        "five consecutive failures after one good reading must all hold dark, saw {seen:?}"
    );
}

#[test]
fn a_real_change_is_followed_immediately() {
    // The other half of the fallback: holding on failure must not become holding in general. A
    // probe that starts answering `Light` moves the scheme on the very next tick.
    let probe = FakeOsThemeProbe::scripted(vec![
        Ok(SystemScheme::Dark),
        Err(()),
        Ok(SystemScheme::Light),
    ]);

    let seen = observed_over(&probe, 3, SystemScheme::Unspecified);

    assert_eq!(
        seen,
        vec![SystemScheme::Dark, SystemScheme::Dark, SystemScheme::Light],
        "the third reading succeeded, so the scheme follows it"
    );
}

#[test]
fn a_failure_before_any_reading_leaves_the_starting_scheme_alone() {
    // Boot under load: the first poll fails, and there is no last-known reading to fall back to
    // beyond whatever the app started with. It must not invent one.
    let probe = FakeOsThemeProbe::failing();

    let seen = observed_over(&probe, 2, SystemScheme::Light);

    assert_eq!(seen, vec![SystemScheme::Light, SystemScheme::Light]);
}

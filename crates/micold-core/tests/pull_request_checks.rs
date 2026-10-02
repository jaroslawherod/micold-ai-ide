//! Reducing a pull request's check counts to one status (feature 040,
//! contracts/pull-request-source.md §4).
//!
//! Failed beats not finished beats finished; no checks at all is its own answer; a state name this
//! version does not know counts as not finished (FR-008).

use micold_core::pull_request::{reduce_checks, CheckCounts, CheckStatus};

#[derive(Clone, Copy, Debug)]
enum List {
    Runs,
    Contexts,
}

fn counts(runs: &[(&str, u64)], contexts: &[(&str, u64)]) -> CheckCounts {
    let own = |v: &[(&str, u64)]| v.iter().map(|(n, c)| (n.to_string(), *c)).collect();
    CheckCounts {
        check_runs: own(runs),
        status_contexts: own(contexts),
    }
}

fn reduce(runs: &[(&str, u64)], contexts: &[(&str, u64)]) -> CheckStatus {
    reduce_checks(Some(&counts(runs, contexts)))
}

use CheckStatus::{Failing, Passing, Pending};
use List::{Contexts, Runs};

const NAMES: [(List, &str, CheckStatus); 19] = [
    (Runs, "FAILURE", Failing),
    (Runs, "CANCELLED", Failing),
    (Runs, "TIMED_OUT", Failing),
    (Runs, "ACTION_REQUIRED", Failing),
    (Runs, "STARTUP_FAILURE", Failing),
    (Contexts, "FAILURE", Failing),
    (Contexts, "ERROR", Failing),
    (Runs, "QUEUED", Pending),
    (Runs, "IN_PROGRESS", Pending),
    (Runs, "PENDING", Pending),
    (Runs, "WAITING", Pending),
    (Runs, "STALE", Pending),
    (Contexts, "PENDING", Pending),
    (Contexts, "EXPECTED", Pending),
    (Runs, "SUCCESS", Passing),
    (Runs, "NEUTRAL", Passing),
    (Runs, "SKIPPED", Passing),
    (Runs, "COMPLETED", Passing),
    (Contexts, "SUCCESS", Passing),
];

// U23, U24, U25
#[test]
fn each_state_name_alone_reduces_to_its_group() {
    for (list, name, expected) in NAMES {
        let got = match list {
            Runs => reduce(&[(name, 1)], &[]),
            Contexts => reduce(&[], &[(name, 1)]),
        };
        assert_eq!(
            got, expected,
            "{name} in {list:?} alone must be {expected:?} (FR-008)"
        );
    }
}

// U26
#[test]
fn a_failed_count_beside_pending_and_passing_is_failing() {
    assert_eq!(
        reduce(&[("FAILURE", 1), ("IN_PROGRESS", 2), ("SUCCESS", 5)], &[]),
        Failing,
        "a failed check run beside pending and passing is failing (FR-008)"
    );
    assert_eq!(
        reduce(&[("IN_PROGRESS", 2), ("SUCCESS", 5)], &[("ERROR", 1)]),
        Failing,
        "a failed status context beside pending check runs is failing (FR-008)"
    );
}

// U27
#[test]
fn a_pending_count_beside_passing_is_pending() {
    assert_eq!(
        reduce(&[("QUEUED", 1), ("SUCCESS", 4)], &[("SUCCESS", 2)]),
        Pending,
        "pending beside passing, nothing failed, is pending (FR-008)"
    );
}

// U28
#[test]
fn only_skipped_and_neutral_is_passing() {
    assert_eq!(
        reduce(&[("SKIPPED", 2), ("NEUTRAL", 1)], &[]),
        Passing,
        "skipped and neutral checks finished without a failure (FR-008)"
    );
}

// U29
#[test]
fn an_unknown_state_name_alone_is_pending() {
    assert_eq!(
        reduce(&[("SOMETHING_NEW", 1)], &[]),
        Pending,
        "an unknown check-run state counts as not finished (FR-008)"
    );
    assert_eq!(
        reduce(&[], &[("SOMETHING_NEW", 1)]),
        Pending,
        "an unknown status-context state counts as not finished (FR-008)"
    );
}

// U30
#[test]
fn no_rollup_is_no_checks() {
    assert_eq!(
        reduce_checks(None),
        CheckStatus::None,
        "a null rollup means the pull request has no checks (FR-008)"
    );
}

// U30
#[test]
fn counts_that_are_all_zero_are_no_checks() {
    assert_eq!(
        reduce(&[("FAILURE", 0), ("SUCCESS", 0)], &[("PENDING", 0)]),
        CheckStatus::None,
        "names present with count 0 are no checks (FR-008)"
    );
    assert_eq!(
        reduce(&[], &[]),
        CheckStatus::None,
        "empty lists are no checks (FR-008)"
    );
}

// A zero count does not count.
#[test]
fn a_name_with_count_zero_does_not_count() {
    assert_eq!(
        reduce(&[("FAILURE", 0), ("SUCCESS", 3)], &[]),
        Passing,
        "FAILURE 0 beside SUCCESS 3 is passing (FR-008)"
    );
}

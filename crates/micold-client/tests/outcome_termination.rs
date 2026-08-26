//! Outcome interpretation terminates, and does not depend on composition order (feature 021,
//! T060 — FR-024, contract O4/O5).
//!
//! # Why this drives a fake interpretation rather than the real one
//!
//! Both properties are properties of the **loop**, not of any variant. Interpreting one outcome may
//! emit another (the spec's Edge Cases name the case), so the queue has no natural end — but no
//! real interpretation can currently produce a cycle, which means the bound could never be observed
//! being reached by driving real variants through it. [`micold_client::app::drain`] is therefore
//! generic over the apply step, and these tests hand it cascades that real code cannot make. A
//! guard that can only be exercised by behaviour that does not exist yet is a guard nobody has run.
//!
//! # `ClipboardWrite` is a stand-in here, and carries no meaning
//!
//! It was the only variant the enum had when this was written (T045 added it for FR-015a); T065
//! added the other three, and this file deliberately did **not** switch to them. These tests never
//! interpret anything — the payload strings are labels for identifying which outcome came back in
//! which order, and nothing reads them as clipboard requests. A test of the *loop* wants the
//! cheapest possible payload, not a realistic one: `SessionsClosed` would need session ids that
//! mean nothing here, and would invite the reader to look for a meaning that is not there.

use micold_client::app::{drain, Drained, OUTCOME_BUDGET};
use micold_client::features::Outcome;

/// An outcome labelled `tag`, for following one through the queue.
fn tagged(tag: &str) -> Outcome {
    Outcome::ClipboardWrite(tag.to_string())
}

fn tag_of(outcome: &Outcome) -> String {
    // Irrefutable until T065 gave the enum three more variants. The `unreachable!` is honest
    // rather than defensive: `tagged` above is the only constructor these tests use.
    match outcome {
        Outcome::ClipboardWrite(tag) => tag.clone(),
        other => unreachable!("these tests construct only ClipboardWrite; got {other:?}"),
    }
}

/// O4: a cycle stops at the bound instead of hanging.
///
/// `#[should_panic]` is the assertion. The contract asks for a loud failure under test and a
/// bounded no-op in release, and this is the loud half — the test binary is a debug build, so the
/// `debug_assert` inside `drain` is live. The release half is
/// [`the_bound_is_reported_rather_than_panicked_in_release`], which cannot run here for the same
/// reason and says so.
#[test]
#[should_panic(expected = "outcome cycle")]
fn a_cycle_stops_at_the_bound_rather_than_hanging() {
    // Every application emits one more, forever. Nothing in the loop can shrink this queue.
    drain([tagged("seed")], |_| vec![tagged("again")]);
}

/// The same cycle, seen from the release side of the contract.
///
/// Skipped rather than silently passing when assertions are on: a test that asserts nothing when
/// it cannot run is how a release-only path goes unchecked for a year.
#[test]
fn the_bound_is_reported_rather_than_panicked_in_release() {
    if cfg!(debug_assertions) {
        // The debug half is asserted by the test above; this configuration cannot observe the
        // return value because `drain` panics before producing it.
        return;
    }
    let result = drain([tagged("seed")], |_| vec![tagged("again")]);
    assert_eq!(
        result,
        Drained {
            applied: OUTCOME_BUDGET,
            overflowed: true
        },
        "a release build must stop at the bound and report it, not panic at a user"
    );
}

/// O5, first half: outcomes are applied in the order they were emitted.
#[test]
fn outcomes_are_applied_in_emission_order() {
    let mut applied = Vec::new();
    let result = drain(
        [tagged("first"), tagged("second"), tagged("third")],
        |outcome| {
            applied.push(tag_of(&outcome));
            Vec::new()
        },
    );
    assert_eq!(applied, ["first", "second", "third"]);
    assert_eq!(
        result,
        Drained {
            applied: 3,
            overflowed: false
        }
    );
}

/// O5, the half that is actually load-bearing: one feature's cascade must not run ahead of an
/// outcome another feature has already emitted.
///
/// **This is the test that distinguishes a queue from a stack**, and it is the whole of "does not
/// depend on the order feature modules are composed in". With a stack, `alpha`'s two-step cascade
/// would be applied before `beta` — so whether `beta` ran second or fourth would depend on which
/// feature the composition happened to put first. With a queue it is second either way.
#[test]
fn a_cascade_does_not_preempt_another_features_pending_outcome() {
    let mut applied = Vec::new();
    drain([tagged("alpha"), tagged("beta")], |outcome| {
        let tag = tag_of(&outcome);
        applied.push(tag.clone());
        match tag.as_str() {
            "alpha" => vec![tagged("alpha-then")],
            "alpha-then" => vec![tagged("alpha-last")],
            _ => Vec::new(),
        }
    });
    assert_eq!(
        applied,
        ["alpha", "beta", "alpha-then", "alpha-last"],
        "`beta` was emitted before any of alpha's cascade existed, so it must be applied before \
         them — a stack would run alpha's chain to the end first (contract O5)"
    );
}

/// The set of outcomes a feature emits does not depend on where it sits in the composition (O5).
///
/// Composing the two features in either order applies each one's own outcomes in its own emission
/// order; only the interleaving between features moves, which is what "composition order" is
/// allowed to change.
#[test]
fn each_features_own_order_survives_either_composition() {
    let run = |initial: Vec<Outcome>| {
        let mut applied = Vec::new();
        drain(initial, |outcome| {
            applied.push(tag_of(&outcome));
            Vec::new()
        });
        applied
    };
    let alpha = || vec![tagged("a1"), tagged("a2")];
    let beta = || vec![tagged("b1"), tagged("b2")];

    let alpha_first = run([alpha(), beta()].concat());
    let beta_first = run([beta(), alpha()].concat());

    let only = |applied: &[String], prefix: char| -> Vec<String> {
        applied
            .iter()
            .filter(|t| t.starts_with(prefix))
            .cloned()
            .collect()
    };
    assert_eq!(only(&alpha_first, 'a'), only(&beta_first, 'a'));
    assert_eq!(only(&beta_first, 'b'), only(&alpha_first, 'b'));
}

// The bound must leave room for ordinary work and still bite. A `OUTCOME_BUDGET` of 0 or 1 would
// make every drain in this file trip the assert; one in the millions would turn the cycle test into
// a hang with extra steps. Both are silent failures of the same guard.
//
// Asserted at compile time rather than in a test because that is what the property is — a fact
// about a constant, decidable without running anything. clippy says as much: an `assert!` over a
// constant is `assertions_on_constants`, and the fix it points at is this one.
const _: () = assert!(
    OUTCOME_BUDGET >= 8,
    "the outcome bound would trip on ordinary cascades"
);
const _: () = assert!(
    OUTCOME_BUDGET <= 1024,
    "the outcome bound is a hang with extra steps, not a guard"
);

/// An ordinary drain finishes well inside the bound and says so.
///
/// The runtime half of the two `const` assertions above: they fix the bound's range, this shows a
/// realistic queue completing inside it rather than being cut short.
#[test]
fn ordinary_work_finishes_inside_the_bound() {
    let deep = (0..8).map(|i| tagged(&format!("n{i}"))).collect::<Vec<_>>();
    let result = drain(deep, |_| Vec::new());
    assert_eq!(
        result,
        Drained {
            applied: 8,
            overflowed: false
        }
    );
}

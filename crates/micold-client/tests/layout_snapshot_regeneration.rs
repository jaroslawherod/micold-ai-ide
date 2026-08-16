//! A normal run never writes the fixture (feature 019, T026 — FR-013).
//!
//! The negative case is the whole point. A gate that rewrites its own baseline does not fail — it
//! records the new reality and reports success, and every run after it is green for a layout nobody
//! agreed to. That failure mode is silent by construction, so it needs a test that is explicitly
//! about *not* writing rather than about writing correctly.
//!
//! Three moments matter, and the last two are the ones a careless implementation gets wrong:
//!
//! - **On success** — nothing to write, and nothing written.
//! - **On failure** — this is the dangerous one. "Write what we observed so the diff is easy to
//!   read" is a tempting convenience and it destroys the baseline the failure was measured against.
//! - **When the fixture is missing** — equally tempting ("just create it"), and equally wrong: a
//!   deleted fixture would be silently recreated from whatever the code currently does, which is
//!   how a snapshot gate stops being evidence of anything.
//!
//! These drive `support::layout::compare_or_regenerate`, which is the function
//! `layout_snapshot.rs` itself calls — not a restatement of its logic. A test that reimplements the
//! branch it checks agrees with itself no matter what the gate does.
//!
//! Every case runs against a temporary file. The committed fixture is never touched, including by a
//! test that fails partway: a check about not clobbering a baseline would be a poor thing to
//! implement by clobbering the baseline.

mod support;

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use support::layout::{self as lay, Outcome};

/// A scratch fixture path unique to the calling test, and its containing directory.
///
/// `CARGO_TARGET_TMPDIR` is cargo's own per-suite scratch space, so nothing here lands in the
/// source tree or in `/tmp`.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("layout_snapshot_regeneration");
    fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(format!("{name}.txt"));
    let _ = fs::remove_file(&path);
    path
}

/// Stands in for the gate's real differ. Its content is irrelevant here — what matters is that a
/// mismatch panics rather than writing.
fn describe(recorded: &str, observed: &str) -> String {
    format!("recorded {recorded:?}, observed {observed:?}")
}

/// The fixture matches: nothing is written, and the file is untouched.
///
/// The weakest of the three, and included because "untouched" has to be established somewhere with
/// nothing else going on.
#[test]
fn a_passing_run_does_not_write_the_fixture() {
    let path = scratch("passing");
    fs::write(&path, "same\n").expect("seed the fixture");
    let before = fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("mtime");

    let outcome = lay::compare_or_regenerate(&path, "same\n", false, describe);

    assert_eq!(outcome, Outcome::Matched);
    assert_eq!(fs::read_to_string(&path).expect("read back"), "same\n");
    assert_eq!(
        fs::metadata(&path)
            .and_then(|m| m.modified())
            .expect("mtime"),
        before,
        "the fixture was rewritten on a passing run, even though its content did not change. \
         Identical bytes make this invisible in review and in git, and the same code path would \
         overwrite a fixture that *had* changed."
    );
}

/// The fixture does not match: the gate fails, and the committed bytes survive.
///
/// This is the case worth having. Rewriting on failure turns "the layout changed, here is what
/// moved" into "the layout is whatever it is now", and the evidence is gone before anyone reads it.
#[test]
fn a_failing_run_does_not_write_the_fixture() {
    let path = scratch("failing");
    fs::write(&path, "recorded\n").expect("seed the fixture");

    let result = catch_unwind(AssertUnwindSafe(|| {
        lay::compare_or_regenerate(&path, "observed\n", false, describe)
    }));

    assert!(
        result.is_err(),
        "a fixture mismatch did not fail the gate at all"
    );
    assert_eq!(
        fs::read_to_string(&path).expect("read back"),
        "recorded\n",
        "the fixture was overwritten by the run that failed against it, so the baseline the \
         failure was measured against no longer exists and the next run is green"
    );
}

/// The fixture is absent: the gate fails loudly and does not create it.
///
/// A missing fixture is not a fresh start. Recreating it from whatever the code does today records
/// the current behaviour as intended without anyone having looked at it — which is exactly what
/// happens when someone deletes the file to "reset" a failing gate.
#[test]
fn a_missing_fixture_is_not_created_by_a_normal_run() {
    let path = scratch("missing");
    assert!(!path.exists(), "the scratch fixture must start absent");

    let result = catch_unwind(AssertUnwindSafe(|| {
        lay::compare_or_regenerate(&path, "generated\n", false, describe)
    }));

    assert!(
        result.is_err(),
        "a missing fixture did not fail the gate, so the snapshot can silently cover nothing"
    );
    assert!(
        !path.exists(),
        "a normal run created the fixture from scratch, recording whatever the code currently \
         does as the expected value"
    );

    let message = result
        .err()
        .and_then(|e| e.downcast_ref::<String>().cloned())
        .unwrap_or_default();
    assert!(
        message.contains("UPDATE_LAYOUT_SNAPSHOT"),
        "the failure must say how to regenerate deliberately, or the obvious next move is to \
         delete something else; got: {message}"
    );
}

/// The other half of the ratchet: regeneration must actually work when it is asked for.
///
/// Without this the three tests above are satisfied by a function that never writes under any
/// circumstances, and the fixture could never be updated at all. Held both ways, like the coverage
/// registry scan.
#[test]
fn an_explicit_regeneration_does_write_the_fixture() {
    let path = scratch("regenerating");
    fs::write(&path, "stale\n").expect("seed the fixture");

    let outcome = lay::compare_or_regenerate(&path, "fresh\n", true, describe);

    assert_eq!(outcome, Outcome::Regenerated);
    assert_eq!(
        fs::read_to_string(&path).expect("read back"),
        "fresh\n",
        "regeneration was requested and the fixture was not updated, so an intended layout change \
         can never be accepted"
    );
}

/// Regeneration is reached only through the environment variable, and only through that name.
///
/// The tests above take `regenerate` as an argument, which deliberately says nothing about what
/// sets it. This reads the gate's own source: the call must pass `UPDATE_LAYOUT_SNAPSHOT`, so a
/// later edit cannot quietly widen the trigger — to a debug build, a CI variable, or a bare
/// `cfg!(test)` — while every test above stays green.
#[test]
fn only_the_documented_variable_triggers_regeneration() {
    let gate =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/layout_snapshot.rs"))
            .expect("read the gate's source");

    let call = gate
        .split_once("compare_or_regenerate(")
        .map(|(_, rest)| rest.split_once(");").map_or(rest, |(args, _)| args))
        .expect("layout_snapshot.rs must call compare_or_regenerate");

    assert!(
        call.contains("UPDATE_LAYOUT_SNAPSHOT"),
        "the gate no longer decides regeneration from UPDATE_LAYOUT_SNAPSHOT. Whatever it reads \
         now is undocumented and untested, and the tests in this file cannot see it because they \
         pass the flag in directly. Call site:\n{call}"
    );
    assert!(
        !call.contains("cfg!("),
        "regeneration is gated on a compile-time condition rather than on a deliberate request, so \
         some builds rewrite the baseline as a matter of course. Call site:\n{call}"
    );
}

/// Every printed regeneration hint must actually regenerate.
///
/// The hint said `cargo test -p micold-client layout_snapshot`. That trailing word is a **test-name
/// filter**, not a target selector, and no test in `layout_snapshot.rs` is called
/// `layout_snapshot` — so the command matched nothing, every binary reported `0 passed; N filtered
/// out`, and it **exited 0 having regenerated nothing**. Someone following it after a real layout
/// change would read the success and believe they had accepted a baseline they had not; the next
/// run fails again with the same instruction, which is the shape of an advice loop.
///
/// That is the mirror image of the failure the rest of this file exists to prevent. Those tests
/// stop the gate from rewriting its baseline and *reporting success*; this one stops the gate from
/// telling you to rewrite the baseline in a way that reports success without doing it. Both are
/// silent, and both end with a fixture nobody actually agreed to.
///
/// Checked by reading the strings themselves, in every place one is printed or written — a
/// convention that lives only in review decays between reviews, and this one did.
#[test]
fn the_regenerate_hint_selects_this_target() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources = [
        (
            "tests/layout_snapshot.rs",
            "the failure message and module doc",
        ),
        (
            "tests/support/layout.rs",
            "the fixture header and the missing-fixture panic",
        ),
    ];

    for (rel, what) in sources {
        let src = fs::read_to_string(dir.join(rel)).expect("read source");
        for (i, line) in src.lines().enumerate() {
            if !line.contains("UPDATE_LAYOUT_SNAPSHOT=1 cargo test") {
                continue;
            }
            assert!(
                line.contains("--test layout_snapshot"),
                "{rel}:{} ({what}) prints a regeneration command without `--test`. A bare \
                 `layout_snapshot` is a test-name filter matching nothing, so the command exits 0 \
                 and regenerates nothing — the reader is told it worked. Use `--test \
                 layout_snapshot`, which selects the target.\n  {}",
                i + 1,
                line.trim(),
            );
        }
    }
}

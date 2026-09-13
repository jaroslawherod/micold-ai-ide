//! Search keeps up with typing (feature 021, contract `match-ranking.md` §5 — SC-002).
//!
//! The whole design rests on recomputing every match on every keystroke: there is no cache, and
//! there is deliberately no debounce, because FR-005 requires the visible results to correspond to
//! the text currently in the field and a debounce is precisely a window in which they do not. That
//! is a defensible choice only while the recompute fits in a frame, so this measures rather than
//! assumes.
//!
//! **Sixteen milliseconds is one frame at 60fps** — the budget the whole recompute has to fit
//! inside for typing to feel immediate, in the release build every user runs. So the budget tests
//! run only in a release build (`cargo test --release -p micold-core --test typeahead_budget`, a
//! CI step of its own) and refuse a debug one, the way 027's start-time measurement does.
//!
//! This used to say a debug pass was the pessimistic case and therefore a real pass. That argument
//! gave the debug build a margin nobody stated, and the margin was zero the day the worst case read
//! 16.37 ms on a loaded machine (BUG-003). A debug build is 10–22× slower than release here, and by
//! a ratio that moves with load, so its number is not SC-002's number at any threshold.

use micold_core::typeahead::{rank, Query};
use std::time::Instant;

/// One frame at 60 frames per second.
const BUDGET_MS: f64 = 16.0;

/// A repository far larger than most: 500 branches, with realistic shapes and lengths.
fn corpus() -> Vec<String> {
    let types = [
        "feat", "fix", "chore", "docs", "refactor", "test", "perf", "ci",
    ];
    let words = [
        "login",
        "logout",
        "reporting",
        "checkout-flow",
        "dependency-bump",
        "api-surface",
        "terminal-emulator",
        "worktree-lifecycle",
        "session-persistence",
        "material-tokens",
    ];
    (0..500)
        .map(|i| {
            format!(
                "{}/JIRA-{}-{}-v{}",
                types[i % types.len()],
                1000 + i,
                words[(i / 7) % words.len()],
                i % 5
            )
        })
        .collect()
}

/// How long `rank` takes over the whole corpus for `query`, in milliseconds, taking the best of
/// several runs — the machine's other work is noise, and the best run is the closest reading of
/// what the code costs.
///
/// Refuses a debug build, so `cargo test -- --ignored` cannot bring back the comparison BUG-003
/// removed.
fn millis(corpus: &[String], query: &str) -> f64 {
    // Bound to a local first: `assert!(!cfg!(..))` folds to a constant, and clippy rejects a
    // constant assertion. Here the constant *is* the check.
    let release_build = !cfg!(debug_assertions);
    assert!(
        release_build,
        "run this with --release: SC-002's 16 ms is the release build's budget, and a debug build's \
         time says nothing about it (BUG-003)"
    );
    let q = Query::new(query);
    let mut best = f64::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        let out = rank(corpus, |s| s.as_str(), &q);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        // Consume the result so the work cannot be optimised away.
        assert!(out.len() <= corpus.len());
        best = best.min(elapsed);
    }
    eprintln!(
        "rank({query:?}) over {} names: best of 5 {best:.3}ms",
        corpus.len()
    );
    best
}

/// The ordinary case: a short query most branches match, so ranking does the most work it will do
/// on the literal tier.
#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build budget (BUG-003)")]
fn ranking_a_common_query_over_500_branches_fits_in_a_frame() {
    let corpus = corpus();
    let took = millis(&corpus, "feat");

    assert!(
        took < BUDGET_MS,
        "ranking 500 branches for \"feat\" took {took:.2}ms, over the {BUDGET_MS}ms frame budget"
    );
}

/// A query nothing matches literally is the expensive one: every candidate falls through every
/// tier before being rejected. This is the case SC-002 actually promises.
#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build budget (BUG-003)")]
fn ranking_a_query_that_matches_nothing_fits_in_a_frame() {
    let corpus = corpus();
    let took = millis(&corpus, "zzqxwv");

    assert!(
        took < BUDGET_MS,
        "ranking 500 branches for a non-matching query took {took:.2}ms, over the {BUDGET_MS}ms \
         frame budget — this is the worst case, because every candidate is tried in every tier"
    );
}

/// A long query costs more per candidate than a short one; the budget covers it too.
#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build budget (BUG-003)")]
fn ranking_a_long_query_fits_in_a_frame() {
    let corpus = corpus();
    let took = millis(&corpus, "feat/JIRA-1234-worktree-lifecycle");

    assert!(
        took < BUDGET_MS,
        "ranking 500 branches for a 33-character query took {took:.2}ms, over the {BUDGET_MS}ms \
         frame budget"
    );
}

/// A corpus that is not what it claims to be would make every measurement above meaningless.
#[test]
fn the_corpus_is_the_size_and_shape_it_claims() {
    let corpus = corpus();
    assert_eq!(corpus.len(), 500);
    assert!(
        corpus.iter().all(|n| n.len() >= 20),
        "names should be realistically long"
    );
    assert!(
        corpus
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == 500,
        "duplicate names would make the ranking work smaller than it looks"
    );
}

/// The genuinely worst case, and the only one that exercises **all three tiers** on every
/// candidate: a query long enough to clear the single-edit floor, matching nothing literally, whose
/// characters *do* all occur across every name so the subsequence scan runs to completion rather
/// than bailing at the first missing character.
///
/// The two cheaper cases above stop early — a short query is answered on the literal tier, and
/// `zzqxwv` dies on its first `z`. This one pays for everything, which is what SC-002 promises to
/// cover (contract §5).
#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build budget (BUG-003)")]
fn ranking_with_all_three_tiers_active_fits_in_a_frame() {
    let corpus = corpus();
    // Every character below appears in every corpus name, but never in this order and never
    // contiguously — so the literal tier misses, the single-edit tier scans every window, and the
    // subsequence tier walks the whole name.
    let took = millis(&corpus, "rtaeiov");

    assert!(
        took < BUDGET_MS,
        "ranking 500 branches with all three tiers active took {took:.2}ms, over the {BUDGET_MS}ms \
         frame budget — this is the worst case the budget has to cover"
    );
}

/// The case above is only the worst case while it really does reach every tier. If a future change
/// let it match literally, or bail out of the subsequence scan early, the measurement would quietly
/// become a cheap one that still passes — so the shape of the work is asserted, not assumed.
#[test]
fn the_worst_case_query_really_does_reach_every_tier() {
    use micold_core::typeahead::{match_one, MatchKind};

    let corpus = corpus();
    let query = Query::new("rtaeiov");
    assert!(
        query.char_len() >= 5,
        "below the single-edit floor, one tier never runs at all"
    );

    let kinds: Vec<MatchKind> = corpus
        .iter()
        .filter_map(|name| match_one(name, &query))
        .map(|m| m.kind)
        .collect();

    assert!(
        !kinds.contains(&MatchKind::Literal),
        "a literal hit would answer on the first tier and skip the expensive two"
    );
    assert!(
        kinds.contains(&MatchKind::Subsequence),
        "no candidate reached the subsequence tier, so the scan is bailing out early and the \
         measurement is of something cheaper than the worst case"
    );
}

// ---------------------------------------------------------------------------------------------
// Which build the budget is measured in (BUG-003)
// ---------------------------------------------------------------------------------------------

/// This file's own source, for the two checks below that hold its shape rather than its numbers.
const SOURCE: &str = include_str!("typeahead_budget.rs");

/// The attribute every frame-budget measurement carries, and the command that runs them.
const RELEASE_ONLY: &str = "#[cfg_attr(debug_assertions, ignore";
const RELEASE_COMMAND: &str = "cargo test --release -p micold-core --test typeahead_budget";

/// The names of the `#[test]` functions in this file whose body checks against [`BUDGET_MS`], each
/// with the attribute lines written above it.
fn budget_tests() -> Vec<(String, String)> {
    let lines: Vec<&str> = SOURCE.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(name) = line
            .strip_prefix("fn ")
            .and_then(|rest| rest.split_once('('))
            .map(|(name, _)| name)
        else {
            continue;
        };
        let body: String = lines[i..]
            .iter()
            .take_while(|l| **l != "}")
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        let attributes: Vec<&str> = lines[..i]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("#["))
            .copied()
            .collect();
        let is_test = attributes.contains(&"#[test]");
        if is_test && body.contains("took < BUDGET_MS") {
            out.push((name.to_string(), attributes.join("\n")));
        }
    }
    out
}

/// SC-002 is a promise about the build a user runs, and the 16 ms in it is that build's number. A
/// debug build measured against it has a margin nobody stated — and the margin was zero the day
/// this was found, when the worst case read 16.37 ms on a loaded machine. So a budget test does not
/// run in a debug build at all, the way 027's start-time measurement refuses one.
#[test]
fn every_frame_budget_measurement_is_release_only() {
    let tests = budget_tests();
    assert!(
        tests.len() >= 4,
        "the scan found {} budget tests, fewer than the four this file has — the check has stopped \
         seeing them",
        tests.len()
    );
    for (name, attributes) in tests {
        assert!(
            attributes.contains(RELEASE_ONLY),
            "`{name}` checks a measurement against the release frame budget, so it must be skipped \
             in a debug build (`{RELEASE_ONLY} = \"…\")]`) — a debug build's time is not SC-002's \
             number (BUG-003)"
        );
    }
}

/// A release-only test that nothing runs in release measures nothing. The workspace suite is a
/// debug build, so CI has to run this file with `--release` in a step of its own.
#[test]
fn ci_runs_the_frame_budget_in_a_release_build() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/ci.yml");
    let workflow =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        workflow
            .lines()
            .any(|l| l.trim_start().trim_start_matches("run: ") == RELEASE_COMMAND),
        "`.github/workflows/ci.yml` must run `{RELEASE_COMMAND}`, or SC-002 is measured in no \
         build at all (BUG-003)"
    );
}

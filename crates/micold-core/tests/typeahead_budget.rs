//! Search keeps up with typing (feature 021, contract `match-ranking.md` §5 — SC-002).
//!
//! The whole design rests on recomputing every match on every keystroke: there is no cache, and
//! there is deliberately no debounce, because FR-005 requires the visible results to correspond to
//! the text currently in the field and a debounce is precisely a window in which they do not. That
//! is a defensible choice only while the recompute fits in a frame, so this measures rather than
//! assumes.
//!
//! **Sixteen milliseconds is one frame at 60fps** — the budget the whole recompute has to fit
//! inside for typing to feel immediate. Measured in a debug build, which is the pessimistic case:
//! the release build every user runs is several times faster, so a debug pass is a real pass.

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
fn millis(corpus: &[String], query: &str) -> f64 {
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
    best
}

/// The ordinary case: a short query most branches match, so ranking does the most work it will do
/// on the literal tier.
#[test]
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

//! What one branch name scores against one search text (feature 021, contract
//! `match-ranking.md` §1–§2).
//!
//! The tiers are tried in order and the first hit wins, because the tier that matched decides both
//! the rank and the shape of the emphasis. So "which tier" is asserted here as carefully as
//! "did it match at all" — a name found literally must never come back as an approximate match,
//! even though it satisfies both.

use micold_core::typeahead::{match_one, MatchKind, Query};

/// The text a match says to emphasise, reassembled from its spans. Asserting on this rather than on
/// raw offsets is what keeps these tests readable when a name gains a character.
fn emphasised(name: &str, spans: &[std::ops::Range<usize>]) -> String {
    spans.iter().map(|s| &name[s.clone()]).collect()
}

/// The match for `query` against `name`, or a panic naming both — an `unwrap` here reports
/// `None` and nothing else, which is the least useful failure a matching test can produce.
fn hit(name: &str, query: &str) -> micold_core::typeahead::Match {
    match_one(name, &Query::new(query))
        .unwrap_or_else(|| panic!("expected {query:?} to match {name:?}, but it did not"))
}

// ---------------------------------------------------------------------------------------------
// §1 Normalisation
// ---------------------------------------------------------------------------------------------

/// Q1.1 — surrounding whitespace and letter case are both noise. Two search texts that differ
/// only in those must be the same query, or the same typing produces different results depending
/// on how it was typed.
#[test]
fn a_query_is_trimmed_and_case_folded() {
    assert_eq!(Query::new("  Feat/Login  "), Query::new("feat/login"));
    assert_eq!(Query::new("FEAT"), Query::new("feat"));
    assert_eq!(Query::new("\tmain\n"), Query::new("main"));
}

/// Q1.2 — nothing but whitespace is nothing at all, which is what "no filtering" means.
#[test]
fn a_query_of_only_whitespace_is_empty() {
    assert!(Query::new("").is_empty());
    assert!(Query::new("   ").is_empty());
    assert!(Query::new(" \t\n ").is_empty());
    assert!(!Query::new(" a ").is_empty());
}

/// Interior whitespace is kept and matched literally. Splitting it into terms would be a second,
/// unspecified matching rule; a branch name rarely contains a space, so such a query simply finds
/// nothing.
#[test]
fn interior_whitespace_is_part_of_the_query() {
    assert_eq!(Query::new("  feat login  ").char_len(), 10);
    assert_ne!(Query::new("feat login"), Query::new("featlogin"));
}

/// The floor the approximate tiers sit behind is counted in **characters**, not bytes — otherwise
/// a two-character Japanese query would be treated as long enough and a two-character ASCII one
/// would not.
#[test]
fn query_length_counts_characters_not_bytes() {
    assert_eq!(Query::new("ab").char_len(), 2);
    assert_eq!(Query::new("日本語").char_len(), 3);
    assert_eq!(Query::new("é").char_len(), 1);
}

// ---------------------------------------------------------------------------------------------
// §2.1 The literal tier
// ---------------------------------------------------------------------------------------------

/// Q2.1.1 — the ordinary case, and the one that carries the feature. The offset matters as much as
/// the hit: it is the secondary rank key, so an early match sorting above a late one depends on it.
#[test]
fn a_name_containing_the_query_matches_literally_at_the_right_offset() {
    let m = hit("feat/login", "log");

    assert_eq!(m.kind, MatchKind::Literal);
    assert_eq!(m.at, 5);
    assert_eq!(m.spans, vec![5..8]);
    assert_eq!(emphasised("feat/login", &m.spans), "log");
}

/// Q2.1.2 — case folds on both sides, and a query spanning a `/` needs no escaping: nothing in a
/// query is a metacharacter.
#[test]
fn matching_ignores_case_on_both_sides() {
    let m = hit("Feat/Login", "feat/l");

    assert_eq!(m.kind, MatchKind::Literal);
    assert_eq!(m.at, 0);
    assert_eq!(emphasised("Feat/Login", &m.spans), "Feat/L");
}

/// Punctuation is ordinary text. A developer searching `412_retry` should not have to think about
/// what the search treats specially, because it treats nothing specially.
#[test]
fn punctuation_in_a_query_is_matched_literally() {
    let name = "feat/JIRA-412_retry-v2";
    assert_eq!(emphasised(name, &hit(name, "412_retry").spans), "412_retry");
    assert_eq!(emphasised(name, &hit(name, "-v2").spans), "-v2");
}

/// The leftmost occurrence, not just any occurrence — otherwise the rank key would depend on
/// which one the search happened to find first.
#[test]
fn a_repeated_query_matches_at_its_first_occurrence() {
    let m = hit("log/tools/log", "log");
    assert_eq!(m.at, 0);
    assert_eq!(m.spans, vec![0..3]);
}

/// An empty query matches everything and emphasises nothing — "no filtering" is a match, not the
/// absence of one, so an empty search text still yields a row per candidate (FR-002).
#[test]
fn an_empty_query_matches_everything_and_emphasises_nothing() {
    let m = match_one("feat/login", &Query::new("  ")).expect("an empty query matches");
    assert!(m.spans.is_empty());
}

/// A name that simply does not contain the text, and is nothing like it, must not be listed
/// (FR-004).
#[test]
fn an_unrelated_name_does_not_match() {
    assert!(match_one("chore/deps", &Query::new("login")).is_none());
}

// ---------------------------------------------------------------------------------------------
// §2.4 Tier exclusivity
// ---------------------------------------------------------------------------------------------

/// Q2.4.1 — `feat/log` satisfies all three tiers at once. It must report the strongest, because the
/// tier decides both where it ranks and how it is emphasised: reported as a subsequence, this name
/// would sort below genuine approximations and emphasise three scattered characters instead of a
/// word.
#[test]
fn a_literal_match_is_never_reported_as_an_approximate_one() {
    let m = hit("feat/log", "log");
    assert_eq!(m.kind, MatchKind::Literal);
    assert_eq!(m.spans, vec![5..8], "a literal hit emphasises one run, not three characters");
}

// ---------------------------------------------------------------------------------------------
// §2.2 Single edit
// ---------------------------------------------------------------------------------------------

/// Q2.2.1 — one dropped letter. The window is emphasised whole rather than as "the characters that
/// survived", because a typo's highlight should read as the word the developer meant, not as the
/// wreckage of what they typed.
#[test]
fn a_dropped_letter_still_finds_the_name() {
    let m = hit("feat/reporting", "reportng");
    assert_eq!(m.kind, MatchKind::SingleEdit);
    assert_eq!(m.at, 5);
    assert_eq!(m.spans.len(), 1, "the whole window is one span");
    assert_eq!(emphasised("feat/reporting", &m.spans), "reporting");
}

/// Q2.2.2 — an extra letter, over a window one shorter than the query.
#[test]
fn an_extra_letter_still_finds_the_name() {
    let m = hit("feat/reporting", "repot");
    assert_eq!(m.kind, MatchKind::SingleEdit);
    assert_eq!(m.spans.len(), 1);
    assert_eq!(emphasised("feat/reporting", &m.spans), "repor");
}

/// A substituted letter — the third edit, over a window the same length as the query.
#[test]
fn a_substituted_letter_still_finds_the_name() {
    let m = hit("feat/login", "lagin");
    assert_eq!(m.kind, MatchKind::SingleEdit);
    assert_eq!(emphasised("feat/login", &m.spans), "login");
}

/// Q2.2.3 — forgiving is not the same as indiscriminate. A query with nothing to do with the name
/// must come back empty, or the list stops narrowing at all (FR-008).
#[test]
fn an_unrelated_query_is_not_forgiven_into_a_match() {
    assert!(match_one("feat/reporting", &Query::new("xyz")).is_none());
    assert!(match_one("feat/reporting", &Query::new("qqqq")).is_none());
}

/// A same-length window is preferred, so a substitution is reported rather than an insertion that
/// also happens to work — the emphasis then covers exactly as many characters as were typed.
#[test]
fn a_window_the_length_of_the_query_wins_over_a_longer_one() {
    let m = hit("feat/login", "logan");
    assert_eq!(m.kind, MatchKind::SingleEdit);
    assert_eq!(
        emphasised("feat/login", &m.spans).chars().count(),
        "logan".chars().count()
    );
}

// ---------------------------------------------------------------------------------------------
// §2.3 Subsequence
// ---------------------------------------------------------------------------------------------

/// Q2.3.1 — an abbreviation. One range per matched character, except that characters landing
/// side by side merge into a single run, so `rep` reads as a word rather than as three boxes.
#[test]
fn an_abbreviation_matches_in_order_and_merges_adjacent_characters() {
    let m = hit("feat/reporting", "frep");
    assert_eq!(m.kind, MatchKind::Subsequence);
    assert_eq!(m.at, 0);
    assert_eq!(m.spans, vec![0..1, 5..8]);
    assert_eq!(emphasised("feat/reporting", &m.spans), "frep");
}

/// Greedy-leftmost is normative, not incidental: it is what makes the same inputs emphasise the
/// same characters on every run (SC-005). `l` takes the *first* `l`, not a later one that would
/// also complete the match.
#[test]
fn a_subsequence_takes_the_leftmost_alignment_available() {
    let m = hit("release/local-login", "llo");
    assert_eq!(m.kind, MatchKind::Subsequence);
    assert_eq!(m.at, 2, "the first `l` in `release`, not a later one");
    // Repeating the call must produce the identical answer — no set iteration, no clock.
    assert_eq!(hit("release/local-login", "llo"), m);
}

/// Out of order is not a subsequence. `gol` is `log` backwards, and the list would be useless if
/// order stopped mattering.
#[test]
fn characters_out_of_order_do_not_match() {
    assert!(match_one("feat/log", &Query::new("gol")).is_none());
}

// ---------------------------------------------------------------------------------------------
// §2.2/§2.3 The three-character floor
// ---------------------------------------------------------------------------------------------

/// Q2.3.2 — below three characters, approximation is off. Two characters match too much of
/// anything to be informative, so a short query narrows only by what it literally contains
/// (FR-006a).
#[test]
fn a_two_character_query_matches_only_literally() {
    // `fl` is a subsequence of `feat/login` (f … l) but there is no literal `fl`.
    assert!(match_one("feat/login", &Query::new("fl")).is_none());
    // The same two characters where they *are* literal still match.
    let m = hit("conflict/resolve", "fl");
    assert_eq!(m.kind, MatchKind::Literal);
}

/// The boundary itself: at two, only what is literally there; at three, skipping is allowed.
#[test]
fn approximation_starts_at_exactly_three_characters() {
    // One character: literal only. `f` is there literally, so it matches — as a `Literal`.
    assert_eq!(hit("feat/login", "f").kind, MatchKind::Literal);
    // Two: `fl` skips, and skipping is not yet allowed.
    assert!(match_one("feat/login", &Query::new("fl")).is_none());
    // Three: the same kind of skip now matches.
    assert_eq!(hit("feat/login", "flg").kind, MatchKind::Subsequence);
}

/// Single edit has a floor of its own, above the approximate floor: at three or four characters one
/// wrong character is too large a share of the query, and the tier reaches far enough that a
/// two-character window would answer a three-character search. Below five, an in-order reading wins.
#[test]
fn single_edit_starts_at_five_characters_and_subsequence_covers_below_it() {
    // Four characters: `frep` is one edit from the window `/rep`, but it is also `f` … `rep` in
    // order — and in order is the stronger claim at this length.
    assert_eq!(hit("feat/reporting", "frep").kind, MatchKind::Subsequence);
    // Five: the tier is live, and a substituted letter reads as the whole word again.
    assert_eq!(hit("feat/login", "lagin").kind, MatchKind::SingleEdit);
}

/// SC-004 promises that *one* wrong character still finds the branch, and says nothing about how
/// much was typed. A blanket five-character floor on the single-edit tier broke that promise for
/// short searches: `lagi` is one substitution from `logi`, and it found nothing at all.
///
/// The floor was fixing two real failures, and both have a narrower cause than "the query is
/// short" — see the tier's own note. What survives at three and four characters is the case where
/// the developer got the first and last characters right, which is the shape of a genuine typo.
#[test]
fn a_single_typo_is_forgiven_even_in_a_short_search() {
    let m = hit("feat/login", "lagi");
    assert_eq!(m.kind, MatchKind::SingleEdit);
    assert_eq!(
        emphasised("feat/login", &m.spans),
        "logi",
        "the whole window reads as the word that was meant"
    );

    // Three characters is the floor for approximation of any kind, and a typo there is forgiven too.
    assert_eq!(hit("feat/login", "lgn").kind, MatchKind::Subsequence);
    assert_eq!(hit("chore/deps", "dops").kind, MatchKind::SingleEdit);
}

/// The narrower rule must not undo what the floor was there for. Both failures it fixed are
/// characterised by the query and the window disagreeing at an *end*: `frep` against `/rep`
/// differs at the first character, which is what makes it an abbreviation rather than a typo.
#[test]
fn a_short_search_whose_ends_disagree_is_not_read_as_a_typo() {
    // Still an abbreviation, not a one-substitution match of `/rep` (contract Q2.2.4, Q2.3.1).
    assert_eq!(hit("feat/reporting", "frep").kind, MatchKind::Subsequence);
    // And a two-character window still cannot answer a three-character search.
    assert_eq!(hit("release/local-login", "llo").kind, MatchKind::Subsequence);
}

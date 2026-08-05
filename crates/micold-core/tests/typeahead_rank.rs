//! The order results come back in (feature 021, contract `match-ranking.md` §3).
//!
//! Ordering is the half of search a developer notices only when it is wrong. Three rules, in
//! priority order: the tier that matched, then how early in the name it matched, then the order the
//! caller supplied. The third is what makes the same search text produce the same list twice, which
//! SC-005 needs and which an unstable sort would quietly take away.

use micold_core::typeahead::{rank, MatchKind, Query};

/// Ranks `names` against `query`, returning just the names in the order they came back.
fn ranked<'a>(names: &[&'a str], query: &str) -> Vec<&'a str> {
    rank(names, |n| *n, &Query::new(query))
        .into_iter()
        .map(|(i, _)| names[i])
        .collect()
}

/// Q3.4 — an empty query filters nothing and reorders nothing. This is the state the picker opens
/// in, so "the branches, in the order the picker already offers them" is the baseline the whole
/// feature is measured against (FR-002).
#[test]
fn an_empty_query_returns_every_item_in_input_order() {
    let names = ["feat/login", "chore/deps", "docs/api"];
    assert_eq!(ranked(&names, ""), vec!["feat/login", "chore/deps", "docs/api"]);
    assert_eq!(ranked(&names, "   "), vec!["feat/login", "chore/deps", "docs/api"]);
}

/// Non-matching items are absent, not merely last (FR-004).
#[test]
fn items_that_do_not_match_are_left_out() {
    let names = ["feat/login", "chore/deps", "feat/logout"];
    assert_eq!(ranked(&names, "log"), vec!["feat/login", "feat/logout"]);
}

/// Q3.1 — among equals, the earlier match wins. `feat/log` matches at 5 and
/// `feat/dialog-cleanup` at 8, and a developer typing `log` almost certainly means the former.
#[test]
fn an_earlier_match_position_ranks_higher() {
    let names = ["feat/dialog-cleanup", "feat/log"];
    assert_eq!(ranked(&names, "log"), vec!["feat/log", "feat/dialog-cleanup"]);
}

/// Q3.2 — the rule that keeps approximate matching from being a nuisance: a name containing the
/// text verbatim always precedes one that merely resembles it, however early the resemblance
/// starts. `chore/repot` matches `repot` at offset 6; `feat/reporting` matches it only by a single
/// edit, at offset 5. Position would put the approximation first; the tier must not let it.
#[test]
fn a_literal_match_precedes_an_approximate_one_regardless_of_position() {
    let names = ["feat/reporting", "chore/repot"];
    let out = rank(&names, |n| *n, &Query::new("repot"));

    assert_eq!(out.len(), 2, "both should match: one literally, one by one edit");
    assert_eq!(names[out[0].0], "chore/repot");
    assert_eq!(out[0].1.kind, MatchKind::Literal);
    assert_eq!(names[out[1].0], "feat/reporting");
    assert_eq!(out[1].1.kind, MatchKind::SingleEdit);
}

/// Q3.3 — the tie-break, and the reason the sort must be stable. Two names matching in the same
/// tier at the same offset have nothing left to separate them but the order they arrived in; a
/// sort that reshuffled them would make the list flicker between identical searches.
#[test]
fn ties_keep_the_order_they_were_given() {
    let forwards = ["a/log", "b/log", "c/log"];
    let backwards = ["c/log", "b/log", "a/log"];

    assert_eq!(ranked(&forwards, "log"), vec!["a/log", "b/log", "c/log"]);
    assert_eq!(ranked(&backwards, "log"), vec!["c/log", "b/log", "a/log"]);
}

/// The same inputs, the same output, every time — the property Q3.3 exists to protect, stated
/// directly rather than inferred from one pair.
#[test]
fn ranking_is_deterministic() {
    let names = [
        "feat/login",
        "feat/logout",
        "chore/dialog-cleanup",
        "docs/logging",
    ];
    let first = ranked(&names, "log");
    for _ in 0..8 {
        assert_eq!(ranked(&names, "log"), first);
    }
}

/// `rank` returns indices into the caller's slice rather than clones, which is what lets it order
/// things it knows nothing about. The indices must actually be usable as such.
#[test]
fn results_are_indices_into_the_callers_own_slice() {
    #[derive(Debug, PartialEq)]
    struct Row {
        label: String,
        payload: u32,
    }
    let rows = vec![
        Row { label: "chore/deps".into(), payload: 7 },
        Row { label: "feat/login".into(), payload: 42 },
    ];

    let out = rank(&rows, |r| r.label.as_str(), &Query::new("login"));

    assert_eq!(out.len(), 1);
    assert_eq!(rows[out[0].0].payload, 42, "the index must address the caller's own item");
}

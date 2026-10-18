//! The quick-switcher's rules (feature 968): which entries a query keeps, in which order, which
//! one is highlighted, and where Up/Down moves the highlight.
//!
//! Render-free, over plain names and keys, so the rules are tested here once: in-order character
//! matching ranked prefix, then run, then scattered (FR-005); the highlight held by key and
//! re-seated on every list change (FR-013); Up/Down skipping unselectable entries and stopping at
//! the ends (FR-007). The matching tiers are the type-ahead's own (`typeahead.rs`), shared rather
//! than copied; this module adds the prefix rank and drops the type-ahead's typo tier, which
//! FR-005 does not allow.

use std::ops::Range;
use std::path::PathBuf;

use crate::navigation::Dir;
use crate::session::{SessionId, SessionLocation};
use crate::typeahead::{self, Folded, Query};

/// What an entry stands for. The highlight is stored as a key, never as an index, so it stays on
/// the same entry when the list changes (FR-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKey {
    /// A location of the active project.
    Location(SessionLocation),
    /// An open session of one of those locations.
    Session(SessionId),
    /// A known project other than the active one, by its root path.
    Project(PathBuf),
}

/// One entry as the rules see it. Input order is FR-004's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Unique in a list.
    pub key: EntryKey,
    /// The displayed name, the only text matched.
    pub name: String,
    /// A session's location candidate index; `None` for locations and projects. `parent` always
    /// points earlier in the list.
    pub parent: Option<usize>,
    /// `selectable` `false` only for an unavailable project (FR-008).
    pub selectable: bool,
}

/// How well a name matched. The derived Ord is the rank order (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    /// The name starts with the query, ignoring case.
    Prefix,
    /// The name contains the query as one run of characters.
    Run,
    /// The query's characters occur in the name in order, not adjacent.
    Scattered,
}

/// One kept candidate: its index in the input, its rank, and the byte ranges of its name to
/// emphasise (empty for an empty query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    /// The candidate's index in the input list.
    pub index: usize,
    /// How it matched.
    pub rank: Rank,
    /// Byte ranges of the candidate's original name to emphasise.
    pub spans: Vec<Range<usize>>,
}

/// The candidates `query` keeps, best rank first and in input order within a rank (FR-005).
///
/// An empty query keeps every candidate in order, rank `Prefix`, nothing emphasised (FR-004).
/// Otherwise a candidate is kept exactly when its name contains the query's characters in order,
/// ignoring case; there is no typo tier.
pub fn filter(candidates: &[Candidate], query: &Query) -> Vec<Ranked> {
    if query.is_empty() {
        return (0..candidates.len())
            .map(|index| Ranked {
                index,
                rank: Rank::Prefix,
                spans: Vec::new(),
            })
            .collect();
    }
    let q = query.chars();
    let mut kept: Vec<Ranked> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let (rank, spans) = rank_one(&Folded::new(&candidate.name), q)?;
            Some(Ranked { index, rank, spans })
        })
        .collect();
    // `sort_by_key` is stable: within a rank the input (FR-004) order is kept.
    kept.sort_by_key(|ranked| ranked.rank);
    kept
}

/// One name's rank and emphasis against the folded query, from the type-ahead's tiers.
fn rank_one(name: &Folded, query: &[char]) -> Option<(Rank, Vec<Range<usize>>)> {
    if let Some(found) = typeahead::literal(name, query) {
        let rank = if found.at == 0 {
            Rank::Prefix
        } else {
            Rank::Run
        };
        return Some((rank, found.spans));
    }
    typeahead::subsequence(name, query).map(|found| (Rank::Scattered, found.spans))
}

/// The position in `ranked` to highlight: `key`'s while it is listed and selectable, else the
/// first selectable one, `None` when none is (FR-005, FR-013).
pub fn reseat(
    candidates: &[Candidate],
    ranked: &[Ranked],
    key: Option<&EntryKey>,
) -> Option<usize> {
    let selectable = |r: &Ranked| candidates[r.index].selectable;
    key.and_then(|key| {
        ranked
            .iter()
            .position(|r| selectable(r) && candidates[r.index].key == *key)
    })
    .or_else(|| ranked.iter().position(selectable))
}

/// Where Up (`Previous`) or Down (`Next`) moves the highlight from position `at` of `ranked`: the
/// next selectable entry that way, or `at` itself at the first/last selectable one (FR-007).
pub fn step(candidates: &[Candidate], ranked: &[Ranked], at: usize, dir: Dir) -> usize {
    let selectable = |&i: &usize| candidates[ranked[i].index].selectable;
    let found = match dir {
        Dir::Next => (at + 1..ranked.len()).find(selectable),
        Dir::Previous => (0..at.min(ranked.len())).rev().find(selectable),
    };
    found.unwrap_or(at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn location(name: &str) -> Candidate {
        Candidate {
            key: EntryKey::Location(SessionLocation::Worktree(name.to_owned())),
            name: name.to_owned(),
            parent: None,
            selectable: true,
        }
    }

    fn project(name: &str, selectable: bool) -> Candidate {
        Candidate {
            key: EntryKey::Project(PathBuf::from(format!("/repos/{name}"))),
            name: name.to_owned(),
            parent: None,
            selectable,
        }
    }

    fn names<'a>(candidates: &'a [Candidate], ranked: &[Ranked]) -> Vec<&'a str> {
        ranked
            .iter()
            .map(|r| candidates[r.index].name.as_str())
            .collect()
    }

    fn list(names: &[&str]) -> Vec<Candidate> {
        names.iter().map(|n| location(n)).collect()
    }

    #[test]
    fn an_empty_query_keeps_every_candidate_in_order_unemphasised() {
        let candidates = list(&["Default", "login-page", "billing"]);
        let ranked = filter(&candidates, &Query::new(""));
        assert_eq!(
            ranked,
            (0..3)
                .map(|index| Ranked {
                    index,
                    rank: Rank::Prefix,
                    spans: vec![]
                })
                .collect::<Vec<_>>(),
            "an empty query lists everything in FR-004 order with nothing emphasised"
        );
    }

    #[test]
    fn letters_in_order_keep_a_name_and_others_are_dropped() {
        let candidates = list(&["Login page", "billing", "logs", "plog"]);
        let ranked = filter(&candidates, &Query::new("logpg"));
        assert_eq!(
            names(&candidates, &ranked),
            vec!["Login page"],
            "only names containing l, o, g, p, g in that order are kept"
        );
    }

    #[test]
    fn prefix_ranks_before_run_before_scattered_and_input_order_holds_within_a_rank() {
        let candidates = list(&["a-log", "l-o-g", "logout", "b-log", "login", "lxoxg"]);
        let ranked = filter(&candidates, &Query::new("log"));
        assert_eq!(
            names(&candidates, &ranked),
            vec!["logout", "login", "a-log", "b-log", "l-o-g", "lxoxg"],
            "prefix, then run, then scattered; input order within each rank"
        );
        let ranks: Vec<Rank> = ranked.iter().map(|r| r.rank).collect();
        assert_eq!(
            ranks,
            vec![
                Rank::Prefix,
                Rank::Prefix,
                Rank::Run,
                Rank::Run,
                Rank::Scattered,
                Rank::Scattered
            ],
            "each kept name carries its rank"
        );
    }

    #[test]
    fn case_is_ignored() {
        let candidates = list(&["login"]);
        let ranked = filter(&candidates, &Query::new("LOG"));
        assert_eq!(ranked.len(), 1, "LOG matches login: case is ignored");
        assert_eq!(ranked[0].rank, Rank::Prefix, "and it is a prefix match");
    }

    #[test]
    fn accents_are_not_folded() {
        let candidates = list(&["café"]);
        assert!(
            filter(&candidates, &Query::new("cafe")).is_empty(),
            "e does not match é: no accent folding"
        );
    }

    #[test]
    fn spans_are_byte_ranges_of_the_original_name_past_a_non_ascii_character() {
        let candidates = list(&["été-login"]);
        let ranked = filter(&candidates, &Query::new("log"));
        let name = &candidates[0].name;
        assert_eq!(ranked.len(), 1, "the run after the accents is found");
        let emphasised: String = ranked[0].spans.iter().map(|s| &name[s.clone()]).collect();
        assert_eq!(
            emphasised, "log",
            "the span indexes the original, non-ASCII name"
        );
    }

    #[test]
    fn a_typo_does_not_match() {
        let candidates = list(&["login"]);
        assert!(
            filter(&candidates, &Query::new("lgoin")).is_empty(),
            "no single-edit tier: a transposition is not an in-order match"
        );
    }

    #[test]
    fn reseat_keeps_a_listed_selectable_key() {
        let candidates = list(&["a", "b", "c"]);
        let ranked = filter(&candidates, &Query::new(""));
        assert_eq!(
            reseat(&candidates, &ranked, Some(&candidates[2].key)),
            Some(2),
            "the highlight stays on the same entry while it is listed"
        );
    }

    #[test]
    fn reseat_falls_back_to_the_first_selectable_entry() {
        let candidates = vec![project("gone", false), location("a"), location("b")];
        let ranked = filter(&candidates, &Query::new(""));
        let absent = EntryKey::Location(SessionLocation::Worktree("zzz".into()));
        assert_eq!(
            reseat(&candidates, &ranked, Some(&absent)),
            Some(1),
            "a key that left the list moves the highlight to the first selectable entry"
        );
        assert_eq!(
            reseat(&candidates, &ranked, Some(&candidates[0].key)),
            Some(1),
            "an unselectable key is never highlighted"
        );
        assert_eq!(
            reseat(&candidates, &ranked, None),
            Some(1),
            "no key: the first selectable entry"
        );
    }

    #[test]
    fn reseat_is_none_when_nothing_is_selectable() {
        let candidates = vec![project("gone", false)];
        let ranked = filter(&candidates, &Query::new(""));
        assert_eq!(
            reseat(&candidates, &ranked, None),
            None,
            "nothing selectable: no highlight"
        );
        assert_eq!(reseat(&[], &[], None), None, "an empty list: no highlight");
    }

    #[test]
    fn step_moves_to_the_next_selectable_and_stops_at_the_ends() {
        let candidates = vec![
            location("a"),
            project("gone", false),
            location("b"),
            project("x", false),
        ];
        let ranked = filter(&candidates, &Query::new(""));
        assert_eq!(
            step(&candidates, &ranked, 0, Dir::Next),
            2,
            "Down skips an unselectable entry"
        );
        assert_eq!(
            step(&candidates, &ranked, 2, Dir::Previous),
            0,
            "Up skips an unselectable entry"
        );
        assert_eq!(
            step(&candidates, &ranked, 2, Dir::Next),
            2,
            "Down stays at the last selectable entry"
        );
        assert_eq!(
            step(&candidates, &ranked, 0, Dir::Previous),
            0,
            "Up stays at the first selectable entry"
        );
    }

    #[test]
    fn five_hundred_candidates_filter_within_100_ms_per_query() {
        let candidates: Vec<Candidate> = (0..500)
            .map(|i| location(&format!("feat/PROJ-{i}-login-page-{}", i * 7)))
            .collect();
        let queries = [
            "l", "lo", "log", "logp", "logpg", "p", "pr", "proj", "proj-4", "x",
        ];
        for text in queries {
            let query = Query::new(text);
            let started = Instant::now();
            let ranked = filter(&candidates, &query);
            let took = started.elapsed();
            assert!(
                took < Duration::from_millis(100),
                "SC-003: filtering 500 entries for {text:?} took {took:?} ({} kept)",
                ranked.len()
            );
        }
    }
}

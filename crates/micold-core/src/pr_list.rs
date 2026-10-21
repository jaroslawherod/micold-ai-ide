//! The pull requests of one branch as an Overview card lists them (feature 1123, data-model §1–§3).
//!
//! Pure: no I/O and no logging. [`BranchPrList`] holds the own pull requests one status reading
//! found for a branch in the card's order (FR-003), gives the counts line (FR-006) and the rows a
//! collapsed or an expanded card draws (FR-002, FR-005), each with its one status phrase (FR-004).
//! Everything here is held in memory only (FR-014): nothing derives `Serialize`, and
//! [`ListedPr`]'s `Debug` leaves out the address; [`PrRow`] carries none.

use std::cmp::Ordering;
use std::fmt;

use crate::overview::PrCounts;
use crate::pull_request::{newest_first, BranchPrState, CheckStatus, ReviewState};

/// How many rows a collapsed card shows (FR-002).
const COLLAPSED_ROWS: usize = 2;

/// One own (non-fork) pull request of a branch, from one reading (data-model §1).
#[derive(Clone, PartialEq, Eq)]
pub struct ListedPr {
    /// The pull request's number in its repository.
    pub number: u64,
    /// Its address on GitHub. Never logged, never serialised.
    pub url: String,
    /// Open, draft, merged or closed.
    pub state: BranchPrState,
    /// The combined result of its last commit's checks; none for a pull request only the merged
    /// page returned (it asks no commits).
    pub checks: CheckStatus,
    /// GitHub's review decision; none for a merged-page-only pull request.
    pub review: ReviewState,
    /// GitHub reports a merge conflict; false for a merged-page-only pull request.
    pub conflict: bool,
    /// `createdAt`: ISO-8601 in UTC, compares as text.
    pub created_at: String,
}

/// Prints the number and the state, and nothing else (FR-014).
impl fmt::Debug for ListedPr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListedPr")
            .field("number", &self.number)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl ListedPr {
    /// Open and draft first (0), then merged and closed (1).
    fn group(&self) -> u8 {
        u8::from(matches!(
            self.state,
            BranchPrState::Merged | BranchPrState::Closed
        ))
    }

    /// The card's order (FR-003): open and draft first, then merged and closed; each group newest
    /// first, then the higher number first.
    fn card_order(&self, other: &Self) -> Ordering {
        self.group().cmp(&other.group()).then_with(|| {
            newest_first(
                (&self.created_at, self.number),
                (&other.created_at, other.number),
            )
        })
    }
}

/// The pull requests one reading found for a branch, in the card's order (data-model §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchPrList {
    prs: Vec<ListedPr>,
    open_full: bool,
    merged_full: bool,
}

impl BranchPrList {
    /// The list of `prs`, one per number (the first given of a number is kept), in the card's order
    /// (FR-003). `open_full` and `merged_full` say the open and the merged page came back full: more
    /// of that group may exist (FR-006).
    pub fn new(prs: Vec<ListedPr>, open_full: bool, merged_full: bool) -> Self {
        let mut seen = std::collections::BTreeSet::new();
        let mut prs: Vec<ListedPr> = prs
            .into_iter()
            .filter(|pr| seen.insert(pr.number))
            .collect();
        prs.sort_by(ListedPr::card_order);
        Self {
            prs,
            open_full,
            merged_full,
        }
    }

    /// The listed pull requests, in the card's order.
    pub fn prs(&self) -> &[ListedPr] {
        &self.prs
    }

    /// The counts line's numbers (FR-006): open and draft as open, merged as merged, each capped
    /// when its page came back full.
    pub fn counts(&self) -> PrCounts {
        let count = |wanted: &[BranchPrState]| {
            let n = self
                .prs
                .iter()
                .filter(|pr| wanted.contains(&pr.state))
                .count();
            u8::try_from(n).unwrap_or(u8::MAX)
        };
        PrCounts {
            open: count(&[BranchPrState::Open, BranchPrState::Draft]),
            merged: count(&[BranchPrState::Merged]),
            open_capped: self.open_full,
            merged_capped: self.merged_full,
        }
    }

    /// The rows a card draws and how many are not drawn (FR-002, FR-005): the first two collapsed,
    /// all of them expanded (and then none left out).
    pub fn rows(&self, expanded: bool) -> (Vec<PrRow>, usize) {
        let shown = if expanded {
            self.prs.len()
        } else {
            self.prs.len().min(COLLAPSED_ROWS)
        };
        let rows = self.prs[..shown].iter().map(PrRow::of).collect();
        (rows, self.prs.len() - shown)
    }
}

/// The one status phrase an open or draft pull request's row shows, in precedence order (FR-004):
/// the first three in feature 813's attention order, the check words feature 1003's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusPhrase {
    /// At least one check failed.
    ChecksFailing,
    /// A reviewer asked for changes.
    ChangesRequested,
    /// GitHub reports a merge conflict.
    MergeConflict,
    /// No check failed and at least one is still running.
    ChecksPending,
    /// Approved.
    Approved,
    /// Every check passed.
    ChecksPassing,
}

impl StatusPhrase {
    /// The first phrase that applies to `pr`; `None` for a merged or closed one, or when none
    /// applies (no checks, no review decision or *review required*, no conflict).
    pub fn of(pr: &ListedPr) -> Option<Self> {
        if matches!(pr.state, BranchPrState::Merged | BranchPrState::Closed) {
            return None;
        }
        let candidates = [
            (pr.checks == CheckStatus::Failing, Self::ChecksFailing),
            (
                pr.review == ReviewState::ChangesRequested,
                Self::ChangesRequested,
            ),
            (pr.conflict, Self::MergeConflict),
            (pr.checks == CheckStatus::Pending, Self::ChecksPending),
            (pr.review == ReviewState::Approved, Self::Approved),
            (pr.checks == CheckStatus::Passing, Self::ChecksPassing),
        ];
        candidates
            .into_iter()
            .find_map(|(applies, phrase)| applies.then_some(phrase))
    }

    /// The phrase in words; the first three are the card's attention reasons' own words, so a
    /// card's header and its rows name the same pull request alike (feature 813).
    pub fn label(self) -> &'static str {
        use crate::overview::CardReason;
        match self {
            Self::ChecksFailing => CardReason::ChecksFailing.label(),
            Self::ChangesRequested => CardReason::ChangesRequested.label(),
            Self::MergeConflict => CardReason::MergeConflict.label(),
            Self::ChecksPending => "checks pending",
            Self::Approved => "approved",
            Self::ChecksPassing => "checks passing",
        }
    }
}

/// One row of a card (data-model §3). No address: a press looks it up by number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRow {
    /// The pull request's number.
    pub number: u64,
    /// The number as shown: "#N".
    pub label: String,
    /// Its state, drawn as a pill.
    pub pill: BranchPrState,
    /// Its status phrase, when one applies.
    pub phrase: Option<StatusPhrase>,
}

impl PrRow {
    fn of(pr: &ListedPr) -> Self {
        Self {
            number: pr.number,
            label: format!("#{}", pr.number),
            pill: pr.state,
            phrase: StatusPhrase::of(pr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(number: u64, state: BranchPrState, created_at: &str) -> ListedPr {
        ListedPr {
            number,
            url: format!("https://github.com/o/r/pull/{number}"),
            state,
            checks: CheckStatus::None,
            review: ReviewState::None,
            conflict: false,
            created_at: created_at.to_string(),
        }
    }

    fn numbers(list: &BranchPrList) -> Vec<u64> {
        list.prs().iter().map(|pr| pr.number).collect()
    }

    /// K1: open and draft before merged and closed, newest first, ties by the higher number, one per
    /// number.
    #[test]
    fn the_list_orders_open_first_then_newest_and_dedupes() {
        use BranchPrState::*;
        let list = BranchPrList::new(
            vec![
                pr(1, Merged, "2026-01-05T00:00:00Z"),
                pr(2, Open, "2026-01-01T00:00:00Z"),
                pr(3, Closed, "2026-01-06T00:00:00Z"),
                pr(4, Draft, "2026-01-02T00:00:00Z"),
                pr(5, Open, "2026-01-02T00:00:00Z"),
                pr(2, Open, "2026-01-01T00:00:00Z"),
            ],
            false,
            false,
        );
        assert_eq!(numbers(&list), vec![5, 4, 2, 3, 1]);
    }

    /// K1: the first given of a number is kept.
    #[test]
    fn a_repeated_number_keeps_the_first_given() {
        let mut first = pr(7, BranchPrState::Open, "2026-01-01T00:00:00Z");
        first.checks = CheckStatus::Failing;
        let second = pr(7, BranchPrState::Open, "2026-01-01T00:00:00Z");
        let list = BranchPrList::new(vec![first.clone(), second], false, false);
        assert_eq!(list.prs(), &[first]);
    }

    fn open_with(checks: CheckStatus, review: ReviewState, conflict: bool) -> ListedPr {
        let mut open = pr(1, BranchPrState::Open, "2026-01-01T00:00:00Z");
        open.checks = checks;
        open.review = review;
        open.conflict = conflict;
        open
    }

    /// K2: each adjacent precedence pair.
    #[test]
    fn each_phrase_beats_the_next() {
        use CheckStatus as C;
        use ReviewState as R;
        use StatusPhrase as S;
        let of = |c, r, x| StatusPhrase::of(&open_with(c, r, x));
        assert_eq!(
            of(C::Failing, R::ChangesRequested, false),
            Some(S::ChecksFailing)
        );
        assert_eq!(
            of(C::None, R::ChangesRequested, true),
            Some(S::ChangesRequested)
        );
        assert_eq!(of(C::Pending, R::None, true), Some(S::MergeConflict));
        assert_eq!(of(C::Pending, R::Approved, false), Some(S::ChecksPending));
        assert_eq!(of(C::Passing, R::Approved, false), Some(S::Approved));
        assert_eq!(of(C::Passing, R::None, false), Some(S::ChecksPassing));
    }

    /// K2: merged and closed carry no phrase; a draft reads like an open one; review required alone
    /// and nothing at all give none.
    #[test]
    fn only_open_and_draft_carry_a_phrase() {
        for state in [BranchPrState::Merged, BranchPrState::Closed] {
            let mut done = open_with(CheckStatus::Failing, ReviewState::ChangesRequested, true);
            done.state = state;
            assert_eq!(StatusPhrase::of(&done), None, "{state:?}");
        }
        let mut draft = open_with(CheckStatus::Pending, ReviewState::None, false);
        draft.state = BranchPrState::Draft;
        assert_eq!(StatusPhrase::of(&draft), Some(StatusPhrase::ChecksPending));
        let required = open_with(CheckStatus::None, ReviewState::ReviewRequired, false);
        assert_eq!(StatusPhrase::of(&required), None);
        let nothing = open_with(CheckStatus::None, ReviewState::None, false);
        assert_eq!(StatusPhrase::of(&nothing), None);
    }

    /// K2: the words.
    #[test]
    fn each_phrase_has_its_words() {
        use StatusPhrase::*;
        let words: Vec<_> = [
            ChecksFailing,
            ChangesRequested,
            MergeConflict,
            ChecksPending,
            Approved,
            ChecksPassing,
        ]
        .map(StatusPhrase::label)
        .to_vec();
        assert_eq!(
            words,
            [
                "checks failing",
                "changes requested",
                "merge conflict",
                "checks pending",
                "approved",
                "checks passing"
            ]
        );
    }

    fn four() -> BranchPrList {
        use BranchPrState::*;
        BranchPrList::new(
            vec![
                pr(9, Merged, "2026-01-03T00:00:00Z"),
                pr(8, Closed, "2026-01-02T00:00:00Z"),
                pr(12, Open, "2026-01-04T00:00:00Z"),
                pr(11, Draft, "2026-01-01T00:00:00Z"),
            ],
            false,
            false,
        )
    }

    /// K4: two rows collapsed and the rest counted; all rows expanded.
    #[test]
    fn a_collapsed_card_shows_two_rows_and_the_rest_as_more() {
        let list = four();
        let (rows, more) = list.rows(false);
        assert_eq!(rows.iter().map(|r| r.number).collect::<Vec<_>>(), [12, 11]);
        assert_eq!(more, 2);
        let (rows, more) = list.rows(true);
        assert_eq!(
            rows.iter().map(|r| r.number).collect::<Vec<_>>(),
            [12, 11, 9, 8]
        );
        assert_eq!(more, 0);
        assert_eq!(
            rows[0],
            PrRow {
                number: 12,
                label: "#12".into(),
                pill: BranchPrState::Open,
                phrase: None,
            }
        );
    }

    /// K4: one or two listed: the same rows both ways, none more.
    #[test]
    fn a_short_list_is_the_same_both_ways() {
        use BranchPrState::*;
        for prs in [
            vec![pr(1, Open, "2026-01-01T00:00:00Z")],
            vec![
                pr(1, Open, "2026-01-01T00:00:00Z"),
                pr(2, Merged, "2026-01-02T00:00:00Z"),
            ],
        ] {
            let list = BranchPrList::new(prs, false, false);
            assert_eq!(list.rows(false), list.rows(true));
            assert_eq!(list.rows(false).1, 0);
        }
    }

    /// FR-006: the counts come from the list; the flags given are the caps.
    #[test]
    fn the_counts_come_from_the_list() {
        assert_eq!(
            four().counts(),
            PrCounts {
                open: 2,
                merged: 1,
                open_capped: false,
                merged_capped: false,
            }
        );
        let capped = BranchPrList::new(four().prs().to_vec(), true, true).counts();
        assert!(capped.open_capped && capped.merged_capped);
    }

    /// An empty list: no rows, zero counts.
    #[test]
    fn an_empty_list_has_no_rows() {
        let list = BranchPrList::new(Vec::new(), false, false);
        assert_eq!(list.rows(false), (Vec::new(), 0));
        assert_eq!(list.rows(true), (Vec::new(), 0));
        assert_eq!(list.counts(), PrCounts::default());
    }

    /// K6: `Debug` holds the number and the state, never the address.
    #[test]
    fn debug_leaves_out_the_address() {
        let listed = pr(42, BranchPrState::Open, "2026-01-01T00:00:00Z");
        let text = format!("{listed:?}");
        assert!(text.contains("42") && text.contains("Open"), "{text}");
        assert!(!text.contains("github.com"), "{text}");
        let text = format!("{:?}", BranchPrList::new(vec![listed], false, false));
        assert!(!text.contains("github.com"), "{text}");
    }
}

//! The review's source control (feature 1272, data-model §1–§7, contract §1): which rows the open
//! list holds and in what order, which of them are available and chosen, where the keyboard stops,
//! and what the closed control reads.
//!
//! Pure: no I/O, no clock (the stale flag arrives in [`PrRows::List`]), no iced. Every function is
//! total: any [`MenuInput`] yields rows, stops and a label, and none panics or reports an error
//! (FR-005, FR-010).

use super::base::Toggles;
use crate::pull_request::BranchPullRequest;

/// The option that returns the view to the entry's own changes (FR-005).
pub const WORKING_TREE: &str = "Working tree";
/// The heading above the worktree's pull requests.
pub const PULL_REQUESTS: &str = "Pull requests";
/// The option that shows the whole tree instead of the changed files.
pub const FILES: &str = "Files";
/// The caption under the pull requests' heading once a failed refresh left a list older than the
/// PR status stale threshold (feature 775, FR-012).
pub const POSSIBLY_STALE: &str = "Possibly stale";
/// The pull requests' line while the first read is pending (feature 1003, FR-010).
pub const PRS_LOADING: &str = "Reading pull requests…";
/// The pull requests' muted line when no list can be read (FR-011).
pub const PRS_UNAVAILABLE: &str = "Pull requests can't be shown for this worktree";
/// The pull requests' line for a worktree with no pull request (FR-010).
pub const PRS_EMPTY: &str = "No pull requests from this worktree yet";
/// The note beside the Default entry's unavailable Committed chip (L1, US1 s9).
pub const DEFAULT_ENTRY_NOTE: &str =
    "The project root has no branch of its own to compare; only uncommitted changes are listed";
/// The closed label's kinds suffix when only committed changes are listed.
pub const KINDS_COMMITTED: &str = "committed";
/// The closed label's kinds suffix when only uncommitted changes are listed.
pub const KINDS_UNCOMMITTED: &str = "uncommitted";
/// The closed label's kinds suffix when no kind that can be listed is on.
pub const KINDS_NONE: &str = "nothing shown";

/// Between the closed label's parts.
const SEP: &str = " · ";

/// What the view shows (data-model §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown<'a> {
    /// The entry's own changes.
    WorkingTree,
    /// One pull request's changes.
    PullRequest {
        /// Its number.
        number: u64,
        /// The listed pull request's title; `None` while the list does not hold the number.
        title: Option<&'a str>,
    },
}

/// The pull-request list state (data-model §2, research R2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrRows<'a> {
    /// Nothing: the setting is off, or the entry has no branch of its own.
    Absent,
    /// The first read is pending.
    Loading,
    /// No list was read and none can be: one muted line.
    Unavailable,
    /// A list was read and holds no pull request.
    Empty,
    /// The list, marked possibly stale once past the threshold.
    List {
        /// The pull requests, newest first.
        prs: &'a [BranchPullRequest],
        /// The list is older than the stale threshold.
        stale: bool,
    },
}

/// Everything the control is decided from (data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuInput<'a> {
    /// What the view shows.
    pub shown: Shown<'a>,
    /// The Files pane is on.
    pub files_on: bool,
    /// The working tree's kinds.
    pub toggles: Toggles,
    /// Committed changes can be listed (not on the Default entry).
    pub committed_available: bool,
    /// The pull-request list state.
    pub prs: PrRows<'a>,
}

/// One kind chip inside the Working tree option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chip {
    /// The kind is listed.
    pub on: bool,
    /// The chip can be changed.
    pub available: bool,
}

/// One line of the open list, in order (data-model §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row<'a> {
    /// The working tree option, with its kind chips.
    WorkingTree {
        /// The view shows the working tree.
        chosen: bool,
        /// The Committed chip.
        committed: Chip,
        /// The Uncommitted chip.
        uncommitted: Chip,
        /// Why Committed is unavailable, when it is.
        note: Option<&'static str>,
    },
    /// The pull requests' heading.
    Heading,
    /// A line in place of the pull requests.
    Note(&'static str),
    /// The list is possibly stale.
    Stale,
    /// One pull request.
    PullRequest {
        /// The listed pull request.
        pr: &'a BranchPullRequest,
        /// The view shows it.
        chosen: bool,
    },
    /// The Files option.
    Files {
        /// The Files pane is on.
        on: bool,
    },
}

/// A kind of working-tree change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Committed on the branch since the base.
    Committed,
    /// Staged, unstaged and untracked.
    Uncommitted,
}

/// Where the keyboard stops in the open list (data-model §5, research R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stop {
    /// The Working tree option.
    WorkingTree,
    /// One kind chip.
    Kind(Kind),
    /// One pull request, by number.
    PullRequest(u64),
    /// The Files option.
    Files,
}

impl Stop {
    /// Taking this stop leaves the list open: only a chip does, so the other can be changed too
    /// (US2 s2).
    pub fn keeps_open(self) -> bool {
        matches!(self, Self::Kind(_))
    }
}

/// The closed control's text (data-model §6, research R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label<'a> {
    /// The part that always stays visible.
    pub fixed: String,
    /// The pull request's title: the only part that may give way.
    pub title: Option<&'a str>,
}

impl Label<'_> {
    /// The untruncated text.
    pub fn full(&self) -> String {
        let mut full = self.fixed.clone();
        full.push_str(self.title.unwrap_or_default());
        full
    }
}

/// The open list's rows, in order (data-model §4).
pub fn rows<'a>(input: &MenuInput<'a>) -> Vec<Row<'a>> {
    let shown_number = match input.shown {
        Shown::WorkingTree => None,
        Shown::PullRequest { number, .. } => Some(number),
    };
    let committed = Chip {
        on: input.toggles.committed && input.committed_available,
        available: input.committed_available,
    };
    let mut rows = vec![Row::WorkingTree {
        chosen: shown_number.is_none(),
        committed,
        uncommitted: Chip {
            on: input.toggles.uncommitted,
            available: true,
        },
        note: (!input.committed_available).then_some(DEFAULT_ENTRY_NOTE),
    }];
    let note = match &input.prs {
        PrRows::Absent => None,
        PrRows::Loading => Some(PRS_LOADING),
        PrRows::Unavailable => Some(PRS_UNAVAILABLE),
        // A list that holds nothing reads as `Empty`, so a heading is never left alone.
        PrRows::Empty | PrRows::List { prs: [], .. } => Some(PRS_EMPTY),
        PrRows::List { prs, stale } => {
            rows.push(Row::Heading);
            if *stale {
                rows.push(Row::Stale);
            }
            rows.extend(prs.iter().map(|pr| Row::PullRequest {
                pr,
                chosen: shown_number == Some(pr.number),
            }));
            None
        }
    };
    if let Some(note) = note {
        rows.extend([Row::Heading, Row::Note(note)]);
    }
    rows.push(Row::Files { on: input.files_on });
    rows
}

/// What the keyboard reaches, in row order, each with whether taking it does something
/// (data-model §5). A disabled stop stays in the list, so the highlight can land on it.
pub fn stops(rows: &[Row<'_>]) -> Vec<(Stop, bool)> {
    let mut stops = Vec::with_capacity(rows.len() + 2);
    for row in rows {
        match row {
            Row::WorkingTree {
                committed,
                uncommitted,
                ..
            } => stops.extend([
                (Stop::WorkingTree, true),
                (Stop::Kind(Kind::Committed), committed.available),
                (Stop::Kind(Kind::Uncommitted), uncommitted.available),
            ]),
            Row::PullRequest { pr, .. } => stops.push((Stop::PullRequest(pr.number), true)),
            Row::Files { .. } => stops.push((Stop::Files, true)),
            Row::Heading | Row::Note(_) | Row::Stale => {}
        }
    }
    stops
}

/// The closed control's label (data-model §6).
pub fn label<'a>(input: &MenuInput<'a>) -> Label<'a> {
    let files = || {
        if input.files_on {
            format!("{FILES}{SEP}")
        } else {
            String::new()
        }
    };
    match input.shown {
        Shown::WorkingTree if input.files_on => Label {
            fixed: format!("{}{WORKING_TREE}", files()),
            title: None,
        },
        Shown::WorkingTree => {
            let committed = input.toggles.committed && input.committed_available;
            let kinds = match (committed, input.toggles.uncommitted) {
                (true, true) => None,
                (true, false) => Some(KINDS_COMMITTED),
                (false, true) if input.committed_available => Some(KINDS_UNCOMMITTED),
                // The Default entry lists uncommitted changes only: all it can list is on.
                (false, true) => None,
                (false, false) => Some(KINDS_NONE),
            };
            Label {
                fixed: match kinds {
                    Some(kinds) => format!("{WORKING_TREE}{SEP}{kinds}"),
                    None => WORKING_TREE.to_owned(),
                },
                title: None,
            }
        }
        Shown::PullRequest {
            number,
            title: Some(title),
        } => Label {
            fixed: format!("{}#{number}{SEP}", files()),
            title: Some(title),
        },
        Shown::PullRequest {
            number,
            title: None,
        } => Label {
            fixed: format!("{}#{number}", files()),
            title: None,
        },
    }
}

/// The control's tooltip: the full label, then the base line when there is one (FR-009).
pub fn tooltip(base_line: Option<&str>, label: &Label<'_>) -> String {
    let full = label.full();
    match base_line {
        Some(base) => format!("{full}\n{base}"),
        None => full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pull_request::{BranchPrState, CheckStatus};

    fn pr(number: u64, title: &str) -> BranchPullRequest {
        BranchPullRequest {
            number,
            title: title.to_owned(),
            url: format!("https://github.com/o/r/pull/{number}"),
            state: BranchPrState::Open,
            created_at: String::new(),
            base: "main".to_owned(),
            head: "abc".to_owned(),
            head_branch: "topic".to_owned(),
            checks: CheckStatus::None,
        }
    }

    fn input<'a>(shown: Shown<'a>, prs: PrRows<'a>) -> MenuInput<'a> {
        MenuInput {
            shown,
            files_on: false,
            toggles: Toggles::default(),
            committed_available: true,
            prs,
        }
    }

    fn toggles(committed: bool, uncommitted: bool) -> Toggles {
        Toggles {
            committed,
            uncommitted,
        }
    }

    fn shown_pr(number: u64, title: Option<&str>) -> Shown<'_> {
        Shown::PullRequest { number, title }
    }

    /// Every list state with every `shown`, `files_on`, toggle and availability combination.
    fn every_input(prs: &[BranchPullRequest]) -> Vec<MenuInput<'_>> {
        let states = [
            PrRows::List {
                prs: &[],
                stale: false,
            },
            PrRows::List {
                prs: &[],
                stale: true,
            },
            PrRows::Absent,
            PrRows::Loading,
            PrRows::Unavailable,
            PrRows::Empty,
            PrRows::List { prs, stale: false },
            PrRows::List { prs, stale: true },
        ];
        let shown = [
            Shown::WorkingTree,
            shown_pr(7, Some("seven")),
            shown_pr(5, Some("five")),
            shown_pr(99, None),
        ];
        let mut all = Vec::new();
        for state in &states {
            for shown in shown {
                for files_on in [false, true] {
                    for committed_available in [false, true] {
                        for (c, u) in [(true, true), (true, false), (false, true), (false, false)] {
                            all.push(MenuInput {
                                shown,
                                files_on,
                                toggles: toggles(c, u),
                                committed_available,
                                prs: state.clone(),
                            });
                        }
                    }
                }
            }
        }
        all
    }

    fn chosen(row: &Row<'_>) -> bool {
        matches!(
            row,
            Row::WorkingTree { chosen: true, .. } | Row::PullRequest { chosen: true, .. }
        )
    }

    fn working_tree(chosen: bool) -> Row<'static> {
        Row::WorkingTree {
            chosen,
            committed: Chip {
                on: true,
                available: true,
            },
            uncommitted: Chip {
                on: true,
                available: true,
            },
            note: None,
        }
    }

    // ---- rows (T002) ----

    #[test]
    fn absent_has_no_heading_and_no_pull_requests() {
        let rows = rows(&input(Shown::WorkingTree, PrRows::Absent));
        assert_eq!(rows, vec![working_tree(true), Row::Files { on: false }]);
    }

    #[test]
    fn loading_unavailable_and_empty_give_heading_then_their_note() {
        for (prs, note) in [
            (PrRows::Loading, PRS_LOADING),
            (PrRows::Unavailable, PRS_UNAVAILABLE),
            (PrRows::Empty, PRS_EMPTY),
        ] {
            let rows = rows(&input(Shown::WorkingTree, prs));
            assert_eq!(
                rows,
                vec![
                    working_tree(true),
                    Row::Heading,
                    Row::Note(note),
                    Row::Files { on: false },
                ]
            );
        }
    }

    #[test]
    fn stale_list_gives_heading_stale_then_the_rows() {
        let prs = [pr(7, "seven"), pr(5, "five")];
        let rows = rows(&input(
            shown_pr(5, Some("five")),
            PrRows::List {
                prs: &prs,
                stale: true,
            },
        ));
        assert_eq!(
            rows,
            vec![
                working_tree(false),
                Row::Heading,
                Row::Stale,
                Row::PullRequest {
                    pr: &prs[0],
                    chosen: false,
                },
                Row::PullRequest {
                    pr: &prs[1],
                    chosen: true,
                },
                Row::Files { on: false },
            ]
        );
    }

    #[test]
    fn list_rows_keep_input_order() {
        let prs = [pr(3, "c"), pr(9, "i"), pr(1, "a")];
        let rows = rows(&input(
            Shown::WorkingTree,
            PrRows::List {
                prs: &prs,
                stale: false,
            },
        ));
        let numbers: Vec<u64> = rows
            .iter()
            .filter_map(|row| match row {
                Row::PullRequest { pr, .. } => Some(pr.number),
                _ => None,
            })
            .collect();
        assert_eq!(numbers, [3, 9, 1]);
        assert_eq!(rows[1], Row::Heading, "a fresh list has no stale row");
    }

    #[test]
    fn row_invariants_hold_for_every_input() {
        let prs = [pr(7, "seven"), pr(5, "five")];
        for input in every_input(&prs) {
            let rows = rows(&input);
            let context = format!("{input:?}");
            assert!(
                matches!(rows.first(), Some(Row::WorkingTree { .. })),
                "{context}"
            );
            assert_eq!(
                rows.last(),
                Some(&Row::Files { on: input.files_on }),
                "{context}"
            );
            let working = rows
                .iter()
                .filter(|r| matches!(r, Row::WorkingTree { .. }))
                .count();
            let files = rows
                .iter()
                .filter(|r| matches!(r, Row::Files { .. }))
                .count();
            assert_eq!((working, files), (1, 1), "{context}");
            let marked = rows.iter().filter(|r| chosen(r)).count();
            let listed = match (input.shown, &input.prs) {
                (Shown::WorkingTree, _) => true,
                (Shown::PullRequest { number, .. }, PrRows::List { prs, .. }) => {
                    prs.iter().any(|p| p.number == number)
                }
                (Shown::PullRequest { .. }, _) => false,
            };
            assert_eq!(marked, usize::from(listed), "{context}");
            for (i, row) in rows.iter().enumerate() {
                if *row == Row::Heading {
                    let next = rows[i + 1..]
                        .iter()
                        .find(|r| **r != Row::Stale)
                        .expect("a row after the heading");
                    assert!(
                        matches!(next, Row::Note(_) | Row::PullRequest { .. }),
                        "{context}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_list_that_holds_nothing_reads_as_empty() {
        for stale in [false, true] {
            let rows = rows(&input(Shown::WorkingTree, PrRows::List { prs: &[], stale }));
            assert_eq!(
                rows,
                vec![
                    working_tree(true),
                    Row::Heading,
                    Row::Note(PRS_EMPTY),
                    Row::Files { on: false },
                ]
            );
        }
    }

    #[test]
    fn an_unlisted_number_marks_nothing() {
        let prs = [pr(7, "seven")];
        let rows = rows(&input(
            shown_pr(99, None),
            PrRows::List {
                prs: &prs,
                stale: false,
            },
        ));
        assert!(!rows.iter().any(chosen));
    }

    #[test]
    fn chips_follow_toggles_and_availability() {
        let prs = [pr(7, "seven")];
        for input in every_input(&prs) {
            let Some(Row::WorkingTree {
                committed,
                uncommitted,
                note,
                ..
            }) = rows(&input).first().cloned()
            else {
                panic!("no working tree row");
            };
            assert_eq!(
                committed,
                Chip {
                    on: input.toggles.committed && input.committed_available,
                    available: input.committed_available,
                }
            );
            assert_eq!(
                uncommitted,
                Chip {
                    on: input.toggles.uncommitted,
                    available: true,
                }
            );
            assert_eq!(note == Some(DEFAULT_ENTRY_NOTE), !input.committed_available);
        }
    }

    // ---- stops (T003) ----

    #[test]
    fn stops_run_working_tree_kinds_pull_requests_files() {
        let prs = [pr(7, "seven"), pr(5, "five")];
        let mut menu = input(
            Shown::WorkingTree,
            PrRows::List {
                prs: &prs,
                stale: false,
            },
        );
        let expected = |committed: bool| {
            vec![
                (Stop::WorkingTree, true),
                (Stop::Kind(Kind::Committed), committed),
                (Stop::Kind(Kind::Uncommitted), true),
                (Stop::PullRequest(7), true),
                (Stop::PullRequest(5), true),
                (Stop::Files, true),
            ]
        };
        assert_eq!(stops(&rows(&menu)), expected(true));
        menu.committed_available = false;
        assert_eq!(
            stops(&rows(&menu)),
            expected(false),
            "disabled, still listed"
        );
    }

    #[test]
    fn heading_note_and_stale_rows_are_no_stops() {
        let prs = [pr(7, "seven")];
        for state in [
            PrRows::Absent,
            PrRows::Loading,
            PrRows::Unavailable,
            PrRows::Empty,
        ] {
            assert_eq!(
                stops(&rows(&input(Shown::WorkingTree, state))),
                vec![
                    (Stop::WorkingTree, true),
                    (Stop::Kind(Kind::Committed), true),
                    (Stop::Kind(Kind::Uncommitted), true),
                    (Stop::Files, true),
                ]
            );
        }
        let fresh = input(
            Shown::WorkingTree,
            PrRows::List {
                prs: &prs,
                stale: false,
            },
        );
        let stale = input(
            Shown::WorkingTree,
            PrRows::List {
                prs: &prs,
                stale: true,
            },
        );
        assert_eq!(stops(&rows(&fresh)), stops(&rows(&stale)));
    }

    #[test]
    fn only_a_kind_keeps_the_list_open() {
        assert!(Stop::Kind(Kind::Committed).keeps_open());
        assert!(Stop::Kind(Kind::Uncommitted).keeps_open());
        assert!(!Stop::WorkingTree.keeps_open());
        assert!(!Stop::PullRequest(7).keeps_open());
        assert!(!Stop::Files.keeps_open());
    }

    // ---- label and tooltip (T004) ----

    fn label_of(shown: Shown<'_>, files_on: bool, t: Toggles, available: bool) -> Label<'_> {
        label(&MenuInput {
            shown,
            files_on,
            toggles: t,
            committed_available: available,
            prs: PrRows::Absent,
        })
    }

    fn plain(fixed: &str) -> Label<'static> {
        Label {
            fixed: fixed.to_owned(),
            title: None,
        }
    }

    #[test]
    fn working_tree_label_names_the_listed_kinds() {
        let wt = Shown::WorkingTree;
        assert_eq!(
            label_of(wt, false, toggles(true, true), true),
            plain("Working tree")
        );
        assert_eq!(
            label_of(wt, false, toggles(true, false), true),
            plain("Working tree · committed")
        );
        assert_eq!(
            label_of(wt, false, toggles(false, true), true),
            plain("Working tree · uncommitted")
        );
        assert_eq!(
            label_of(wt, false, toggles(false, false), true),
            plain("Working tree · nothing shown")
        );
    }

    #[test]
    fn default_entry_label_counts_only_what_it_can_list() {
        let wt = Shown::WorkingTree;
        for committed in [true, false] {
            assert_eq!(
                label_of(wt, false, toggles(committed, true), false),
                plain("Working tree")
            );
            assert_eq!(
                label_of(wt, false, toggles(committed, false), false),
                plain("Working tree · nothing shown")
            );
        }
    }

    #[test]
    fn files_on_working_tree_has_no_kinds_suffix() {
        for (c, u) in [(true, true), (true, false), (false, true), (false, false)] {
            assert_eq!(
                label_of(Shown::WorkingTree, true, toggles(c, u), true),
                plain("Files · Working tree")
            );
        }
    }

    #[test]
    fn pull_request_label_keeps_the_number_and_lets_the_title_give_way() {
        let t = Toggles::default();
        let listed = shown_pr(42, Some("Fix the thing"));
        assert_eq!(
            label_of(listed, false, t, true),
            Label {
                fixed: "#42 · ".to_owned(),
                title: Some("Fix the thing"),
            }
        );
        assert_eq!(
            label_of(listed, true, t, true),
            Label {
                fixed: "Files · #42 · ".to_owned(),
                title: Some("Fix the thing"),
            }
        );
        assert_eq!(label_of(shown_pr(42, None), false, t, true), plain("#42"));
        assert_eq!(
            label_of(shown_pr(42, None), true, t, true),
            plain("Files · #42")
        );
    }

    #[test]
    fn full_joins_fixed_and_title() {
        let listed = Label {
            fixed: "#42 · ".to_owned(),
            title: Some("Fix the thing"),
        };
        assert_eq!(listed.full(), "#42 · Fix the thing");
        assert_eq!(plain("Working tree").full(), "Working tree");
    }

    #[test]
    fn tooltip_is_the_full_label_then_the_base_line() {
        let listed = Label {
            fixed: "#42 · ".to_owned(),
            title: Some("Fix the thing"),
        };
        assert_eq!(
            tooltip(Some("topic → main"), &listed),
            "#42 · Fix the thing\ntopic → main"
        );
        assert_eq!(tooltip(None, &listed), "#42 · Fix the thing");
    }
}

//! What the session area's context header says (feature 1126, research R1): the shown location's
//! title and its tooltip, its state word, its short branch and its current pull request, derived
//! from the location's Overview [`CardModel`] so the header cannot disagree with its card
//! (SC-001). The header widget only places these words; every rule with a branch in it lives here.
//!
//! Render-free: no I/O and no logging.

use crate::overview::{CardModel, CardState};
use crate::pr_list::StatusPhrase;
use crate::pull_request::{PrState, ReviewState};
use crate::review::changes::ChangeList;

/// What a pull request chip's tooltip adds when the reading it comes from is stale (FR-005).
pub const STALE_NOTE: &str = "as of the last reading";

/// The shown location's current pull request, as the sidebar's indicator reads it (data-model
/// *PrChipInput*, research R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrChipInput {
    /// The pull request's number.
    pub number: u64,
    /// Open, draft, merged or closed; an open or draft one carries its checks.
    pub state: PrState,
    /// GitHub's review decision.
    pub review: ReviewState,
    /// GitHub reports a merge conflict.
    pub conflict: bool,
    /// The reading is older than the sidebar's stale limit.
    pub stale: bool,
}

/// One chip: the words it shows and its tooltip, which holds them in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    /// The chip's words.
    pub text: String,
    /// The tooltip: the full text, and what the chip leaves out.
    pub tooltip: String,
}

/// The shown location's changes as the Changes view's list counts them (data-model
/// *ChangeSummary*, contract H6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChangeSummary {
    /// Changed files: the list's rows.
    pub files: u32,
    /// Lines added over every row (0 for a binary file, as its row).
    pub added: u32,
    /// Lines removed over every row.
    pub removed: u32,
}

impl ChangeSummary {
    /// The summary of `list`; the sums saturate.
    pub fn of(list: &ChangeList) -> Self {
        list.files.iter().fold(
            Self {
                files: u32::try_from(list.files.len()).unwrap_or(u32::MAX),
                ..Self::default()
            },
            |sum, file| Self {
                added: sum.added.saturating_add(file.added),
                removed: sum.removed.saturating_add(file.removed),
                ..sum
            },
        )
    }
}

/// What the header knows of the counts (data-model *CountsReading*): nothing yet, a read under
/// way or a read that failed (`Unknown`), or the last answer (`Known`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountsReading {
    /// No figure to show: never a stale or made-up one (US3 scenario 4).
    Unknown,
    /// The counts of the latest read.
    Known(ChangeSummary),
}

/// What the header says (data-model *HeaderText*, contract H1–H6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderText {
    /// The card's title (FR-002).
    pub title: String,
    /// The title, then the display name on a second line when it differs.
    pub title_tooltip: String,
    /// The card's state; its word is [`CardState::label`] (FR-003).
    pub state: CardState,
    /// Why it is in that state, the state chip's tooltip.
    pub state_tooltip: String,
    /// The short branch, the full branch in its tooltip; `None` without a branch (FR-004).
    pub branch: Option<Chip>,
    /// The current pull request in words; `None` without one (FR-005).
    pub pull_request: Option<Chip>,
    /// The Changes button's label (H5, FR-006).
    pub changes: String,
}

impl HeaderText {
    /// What the header says for `model`'s location, with its current pull request `pr` and what
    /// is known of its change counts.
    pub fn of(model: &CardModel, pr: Option<&PrChipInput>, counts: CountsReading) -> Self {
        let title = model.title.clone();
        let title_tooltip = if model.display_name == title {
            title.clone()
        } else {
            format!("{title}\n{}", model.display_name)
        };
        Self {
            title,
            title_tooltip,
            state: model.state,
            state_tooltip: model.reason.label().to_string(),
            branch: model.branch.as_deref().map(branch_chip),
            pull_request: pr.map(pull_request_chip),
            changes: changes_label(counts),
        }
    }
}

/// The Changes button's label (H5): `Changes`, then the files and lines, or `no changes`; only
/// `Changes` while the counts are unknown. `−` is U+2212, as the Changes view's rows write it.
pub fn changes_label(counts: CountsReading) -> String {
    match counts {
        CountsReading::Unknown => "Changes".to_string(),
        CountsReading::Known(ChangeSummary { files: 0, .. }) => "Changes \u{b7} no changes".into(),
        CountsReading::Known(ChangeSummary {
            files,
            added,
            removed,
        }) => {
            let noun = if files == 1 { "file" } else { "files" };
            format!("Changes \u{b7} {files} {noun} \u{b7} +{added} \u{2212}{removed}")
        }
    }
}

/// The branch chip (H3): the short form when the branch has one, the full branch on hover.
fn branch_chip(branch: &str) -> Chip {
    Chip {
        text: crate::overview_text::short_branch(branch).unwrap_or_else(|| branch.to_string()),
        tooltip: branch.to_string(),
    }
}

/// The pull request chip (H4): `PR #n · <state>[ · <phrase>]`, the stale note in its tooltip.
fn pull_request_chip(pr: &PrChipInput) -> Chip {
    let word = match pr.state {
        PrState::Open { .. } => "Open",
        PrState::Draft { .. } => "Draft",
        PrState::Merged => "Merged",
        PrState::Closed => "Closed",
    };
    let mut text = format!("PR #{} \u{b7} {word}", pr.number);
    if let Some(phrase) = StatusPhrase::of_state(pr.state, pr.review, pr.conflict) {
        text.push_str(" \u{b7} ");
        text.push_str(phrase.label());
    }
    let tooltip = if pr.stale {
        format!("{text}\n{STALE_NOTE}")
    } else {
        text.clone()
    };
    Chip { text, tooltip }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overview::{Activation, CardReason, LocationType, PrDisplay};
    use crate::pull_request::CheckStatus;
    use crate::session::SessionLocation;
    use crate::task_progress::{Progress, Source};

    const ISSUE_TITLE: &str = "CI red on main: daemon tests";

    fn model(title: &str, branch: Option<&str>, name: &str) -> CardModel {
        let location = SessionLocation::Worktree(name.into());
        CardModel {
            location: location.clone(),
            display_name: name.into(),
            branch: branch.map(Into::into),
            kind: LocationType::Feature,
            sessions: (1, 0),
            state: CardState::Idle,
            reason: CardReason::Idle,
            title: title.into(),
            prs: PrDisplay::Hidden,
            pull_requests: None,
            progress: Progress {
                phase: None,
                ratio: None,
                source: Source::None,
            },
            activity: None,
            activation: Activation::RevealOnly(location),
            dir: std::path::PathBuf::from(name),
            issue_link: None,
        }
    }

    fn default_entry() -> CardModel {
        CardModel {
            location: SessionLocation::Default,
            ..model("Default", None, "Default")
        }
    }

    fn pr(number: u64, state: PrState) -> PrChipInput {
        PrChipInput {
            number,
            state,
            review: ReviewState::None,
            conflict: false,
            stale: false,
        }
    }

    fn pr_text(input: &PrChipInput) -> Option<String> {
        let m = model("t", Some("b"), "n");
        HeaderText::of(&m, Some(input), CountsReading::Unknown)
            .pull_request
            .map(|c| c.text)
    }

    /// H1: the title is the card's, the tooltip adds the display name on a second line when it
    /// differs.
    #[test]
    fn the_title_is_the_cards_and_its_tooltip_names_the_location() {
        let rows = [
            (
                model(
                    ISSUE_TITLE,
                    Some("fix/1117_ci-red-on-main"),
                    "1117_ci-red-on-main",
                ),
                ISSUE_TITLE,
                format!("{ISSUE_TITLE}\n1117_ci-red-on-main"),
            ),
            (
                model("feat/x", Some("feat/x"), "x"),
                "feat/x",
                "feat/x\nx".to_string(),
            ),
            (
                model("scratch", None, "scratch"),
                "scratch",
                "scratch".to_string(),
            ),
            (default_entry(), "Default", "Default".to_string()),
        ];
        for (m, title, tooltip) in rows {
            let text = HeaderText::of(&m, None, CountsReading::Unknown);
            assert_eq!(text.title, title, "H1 title of {:?}", m.location);
            assert_eq!(
                text.title_tooltip, tooltip,
                "H1 tooltip of {:?}",
                m.location
            );
        }
    }

    /// H2: the state is the card's, its tooltip the card's reason.
    #[test]
    fn the_state_is_the_cards_with_its_reason_as_tooltip() {
        let rows = [
            (CardState::NeedsAction, CardReason::NeedsPermission),
            (CardState::NeedsAttention, CardReason::ChecksFailing),
            (CardState::Working, CardReason::Working),
            (CardState::Done, CardReason::Merged),
            (CardState::Idle, CardReason::Idle),
        ];
        for (state, reason) in rows {
            let m = CardModel {
                state,
                reason,
                ..model("t", None, "n")
            };
            let text = HeaderText::of(&m, None, CountsReading::Unknown);
            assert_eq!(text.state, state, "H2 state");
            assert_eq!(text.state.label(), state.label());
            assert_eq!(
                text.state_tooltip,
                reason.label(),
                "H2 tooltip of {state:?}"
            );
        }
    }

    /// H3: the short branch, the full branch on hover; no chip without a branch.
    #[test]
    fn the_branch_chip_is_short_with_the_full_branch_on_hover() {
        let chip = |branch: Option<&str>| {
            HeaderText::of(&model("t", branch, "n"), None, CountsReading::Unknown).branch
        };
        assert_eq!(
            chip(Some("fix/1117_ci-red-on-main")),
            Some(Chip {
                text: "fix/1117".into(),
                tooltip: "fix/1117_ci-red-on-main".into(),
            })
        );
        assert_eq!(
            chip(Some("main")),
            Some(Chip {
                text: "main".into(),
                tooltip: "main".into(),
            })
        );
        assert_eq!(chip(None), None, "no branch, no chip");
        assert_eq!(
            HeaderText::of(&default_entry(), None, CountsReading::Unknown).branch,
            None
        );
    }

    /// H4: the pull request chip's words for each state, phrase and stale reading.
    #[test]
    fn the_pull_request_chip_names_number_state_and_phrase() {
        let m = model("t", Some("b"), "n");
        assert_eq!(
            HeaderText::of(&m, None, CountsReading::Unknown).pull_request,
            None,
            "no input, no chip"
        );
        let open = |checks| PrState::Open { checks };
        let rows = [
            (
                pr(1118, open(CheckStatus::Failing)),
                "PR #1118 \u{b7} Open \u{b7} checks failing",
            ),
            (
                pr(
                    7,
                    PrState::Draft {
                        checks: CheckStatus::Pending,
                    },
                ),
                "PR #7 \u{b7} Draft \u{b7} checks pending",
            ),
            (
                PrChipInput {
                    review: ReviewState::ChangesRequested,
                    ..pr(7, open(CheckStatus::None))
                },
                "PR #7 \u{b7} Open \u{b7} changes requested",
            ),
            (
                PrChipInput {
                    conflict: true,
                    ..pr(7, open(CheckStatus::Passing))
                },
                "PR #7 \u{b7} Open \u{b7} merge conflict",
            ),
            (pr(7, open(CheckStatus::None)), "PR #7 \u{b7} Open"),
            (pr(9, PrState::Merged), "PR #9 \u{b7} Merged"),
            (pr(9, PrState::Closed), "PR #9 \u{b7} Closed"),
        ];
        for (input, text) in rows {
            assert_eq!(pr_text(&input).as_deref(), Some(text), "H4 {input:?}");
            let chip = HeaderText::of(&m, Some(&input), CountsReading::Unknown)
                .pull_request
                .unwrap();
            assert_eq!(chip.tooltip, text, "a fresh reading's tooltip is the text");
            let stale = PrChipInput {
                stale: true,
                ..input
            };
            let chip = HeaderText::of(&m, Some(&stale), CountsReading::Unknown)
                .pull_request
                .unwrap();
            assert_eq!(chip.text, text, "stale: same text");
            assert!(
                chip.tooltip.starts_with(text) && chip.tooltip.ends_with(STALE_NOTE),
                "stale: the tooltip ends with the note: {:?}",
                chip.tooltip
            );
        }
    }

    /// H5: the Changes button's label for files and lines, no changes, and unknown counts.
    #[test]
    fn the_changes_label_names_files_and_lines_or_says_there_are_none() {
        let m = model("t", Some("b"), "n");
        let known = |files, added, removed| {
            CountsReading::Known(ChangeSummary {
                files,
                added,
                removed,
            })
        };
        let rows = [
            (
                known(3, 42, 7),
                "Changes \u{b7} 3 files \u{b7} +42 \u{2212}7",
            ),
            (known(1, 0, 5), "Changes \u{b7} 1 file \u{b7} +0 \u{2212}5"),
            (known(0, 0, 0), "Changes \u{b7} no changes"),
            (known(0, 4, 1), "Changes \u{b7} no changes"),
            (CountsReading::Unknown, "Changes"),
        ];
        for (counts, label) in rows {
            assert_eq!(
                HeaderText::of(&m, None, counts).changes,
                label,
                "H5 {counts:?}"
            );
        }
    }

    /// H6: the summary counts every row and sums its lines, a binary row adding none.
    #[test]
    fn the_summary_counts_rows_and_sums_their_lines() {
        use crate::review::base::ReviewScope;
        use crate::review::changes::{ChangeKind, ChangedFile, Content, Origin};
        use crate::review::RelPath;
        let file = |path: &str, kind, content, added, removed| ChangedFile {
            path: RelPath::from_git(path),
            kind,
            content,
            added,
            removed,
            large: false,
            origin: Origin::Uncommitted,
        };
        let list = ChangeList {
            files: vec![
                file("a.rs", ChangeKind::Modified, Content::Text, 10, 2),
                file("logo.png", ChangeKind::Added, Content::Binary, 0, 0),
                file(
                    "b.rs",
                    ChangeKind::Renamed {
                        from: RelPath::from_git("old.rs"),
                    },
                    Content::Text,
                    1,
                    1,
                ),
            ],
            scope: ReviewScope::RootUncommitted,
        };
        assert_eq!(
            ChangeSummary::of(&list),
            ChangeSummary {
                files: 3,
                added: 11,
                removed: 3,
            }
        );
        let huge = ChangeList {
            files: vec![
                file("x", ChangeKind::Modified, Content::Text, u32::MAX, 1),
                file("y", ChangeKind::Modified, Content::Text, 1, u32::MAX),
            ],
            scope: ReviewScope::RootUncommitted,
        };
        assert_eq!(
            ChangeSummary::of(&huge),
            ChangeSummary {
                files: 2,
                added: u32::MAX,
                removed: u32::MAX,
            },
            "the sums saturate"
        );
    }
}

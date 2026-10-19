//! What an Overview card says (feature 1120, research R1): its title, short branch, sessions line,
//! progress row and activity line, derived from a [`CardModel`]. The card widget only places these
//! words; every rule with a branch in it lives here so it is tested without the GUI (FR-018).
//!
//! Pure: no I/O and no logging. The activity line carries a user's prompt text, so nothing here may
//! be logged (FR-020); the types derive `Debug` only for tests' sake.

use crate::task_progress::Ratio;

/// One line of text and the tooltip that shows the whole of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// What the line shows.
    pub text: String,
    /// What hovering it shows.
    pub tooltip: String,
}

/// The progress row: its words and the ratio a bar is drawn for (`None` draws no bar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressRow {
    /// *Implement · 6/10*, *6/10* or the phase alone.
    pub text: String,
    /// The ratio a bar is drawn for.
    pub bar: Option<Ratio>,
}

/// What one card says (data-model *CardText*); `None` fields are rows the card does not show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardText {
    /// The issue title, else the branch, else the display name (FR-006).
    pub title: String,
    /// The title's tooltip: the location's display name.
    pub title_tooltip: String,
    /// The short branch with the full branch as tooltip, unless the title is the branch (FR-007).
    pub branch: Option<Line>,
    /// *No sessions*, *1 session · 1 running* (FR-008).
    pub sessions: String,
    /// The progress row, when there is a phase or a ratio (FR-009).
    pub progress: Option<ProgressRow>,
    /// The pull request row and, for the unknown mark, its tooltip (FR-010).
    pub prs: Option<(String, Option<String>)>,
    /// What the agent is doing now, when its hooks report (FR-011).
    pub activity: Option<Line>,
}

/// The card's title: the known issue title, else the branch, else the display name (C4, FR-006).
/// A blank issue title counts as unknown.
pub fn card_title(issue_title: Option<&str>, branch: Option<&str>, display_name: &str) -> String {
    issue_title
        .filter(|title| !title.trim().is_empty())
        .or(branch)
        .unwrap_or(display_name)
        .to_string()
}

/// `type/number` of a conventional branch, `None` for any other (C5, FR-007, research R3): a
/// [`ConventionalType`](crate::naming::ConventionalType) token, `/` or `-`, ASCII digits, then the
/// end, or `_` or `-` and a non-empty slug.
pub fn short_branch(branch: &str) -> Option<String> {
    let sep = branch.find(['/', '-'])?;
    let type_ = crate::naming::ConventionalType::from_token(&branch[..sep])?;
    let rest = &branch[sep + 1..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let (number, tail) = rest.split_at(digits);
    let ends_well = match tail.as_bytes() {
        [] => true,
        [b'_' | b'-', slug @ ..] => !slug.is_empty(),
        _ => false,
    };
    ends_well.then(|| format!("{}/{number}", type_.as_str()))
}

/// The branch line: `None` without a branch or when the title is the branch, else the short (or
/// unchanged) branch with the full branch as tooltip (C6, FR-007).
pub fn branch_line(title: &str, branch: Option<&str>) -> Option<Line> {
    let branch = branch.filter(|b| *b != title)?;
    Some(Line {
        text: short_branch(branch).unwrap_or_else(|| branch.to_string()),
        tooltip: branch.to_string(),
    })
}

/// The sessions line (C7, FR-008): *No sessions*, *1 session*, *n sessions*, then *· k running*
/// when some run. `running` is clamped to `total` for display; no cap.
pub fn sessions_line((total, running): (u32, u32)) -> String {
    let sessions = match total {
        0 => return "No sessions".to_string(),
        1 => "1 session".to_string(),
        n => format!("{n} sessions"),
    };
    match running.min(total) {
        0 => sessions,
        k => format!("{sessions} \u{b7} {k} running"),
    }
}

/// The progress row, `None` with neither a phase nor a ratio; a bar only with a ratio (C8, FR-009).
pub fn progress_row(progress: &crate::task_progress::Progress) -> Option<ProgressRow> {
    let text = match (progress.phase, progress.ratio) {
        (Some(phase), Some(r)) => format!("{} \u{b7} {}/{}", phase.label(), r.done, r.total),
        (None, Some(r)) => format!("{}/{}", r.done, r.total),
        (Some(phase), None) => phase.label().to_string(),
        (None, None) => return None,
    };
    Some(ProgressRow {
        text,
        bar: progress.ratio,
    })
}

/// The activity line with its todo ratio (*Writing tests · 2/5*), the same text as tooltip; `None`
/// for no activity or a blank line, so a ratio never shows alone (C3).
pub fn activity_line(activity: Option<&crate::hook_activity::CurrentActivity>) -> Option<Line> {
    let activity = activity?;
    let line = activity.line.trim();
    if line.is_empty() {
        return None;
    }
    let text = match activity.ratio {
        Some(r) => format!("{line} \u{b7} {}/{}", r.done, r.total),
        None => line.to_string(),
    };
    Some(Line {
        tooltip: text.clone(),
        text,
    })
}

impl CardText {
    /// What `model`'s card says, each field from its rule (data-model *CardText*).
    pub fn of(model: &crate::overview::CardModel) -> Self {
        Self {
            branch: branch_line(&model.title, model.branch.as_deref()),
            title: model.title.clone(),
            title_tooltip: model.display_name.clone(),
            sessions: sessions_line(model.sessions),
            progress: progress_row(&model.progress),
            prs: model.prs.footer(),
            activity: activity_line(model.activity.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook_activity::CurrentActivity;
    use crate::overview::PrDisplay;
    use crate::overview::{Activation, CardModel, CardReason, CardState, LocationType, PrCounts};
    use crate::session::SessionLocation;
    use crate::task_progress::{Phase, Progress, Source};

    const ISSUE_TITLE: &str = "Overview cards: readable layout";

    fn ratio(done: u32, total: u32) -> Ratio {
        Ratio { done, total }
    }

    fn progress(phase: Option<Phase>, ratio: Option<Ratio>) -> Progress {
        Progress {
            phase,
            ratio,
            source: Source::TaskList,
        }
    }

    fn activity(line: &str, ratio: Option<Ratio>) -> CurrentActivity {
        CurrentActivity {
            line: line.into(),
            ratio,
        }
    }

    /// C4: the issue title wins, then the branch, then the display name.
    #[test]
    fn the_title_is_the_issue_title_else_the_branch_else_the_display_name() {
        assert_eq!(
            card_title(Some(ISSUE_TITLE), Some("feat/1120_x"), "feat-1120_x"),
            ISSUE_TITLE,
            "a known issue title is the title"
        );
        assert_eq!(
            card_title(None, Some("fix/1115_container"), "fix-1115_container"),
            "fix/1115_container",
            "without a title the branch is"
        );
        assert_eq!(
            card_title(None, None, "Default"),
            "Default",
            "with neither the display name is"
        );
        assert_eq!(
            card_title(Some("   "), Some("main"), "d"),
            "main",
            "a blank issue title counts as unknown"
        );
    }

    /// C5: a conventional type, `/` or `-`, a decimal number, then the end or `_`/`-` and a slug.
    #[test]
    fn a_conventional_branch_shortens_to_type_and_number() {
        for (branch, short) in [
            ("fix/1115_container-daemon-fails-to-start", "fix/1115"),
            ("fix/1115-foo", "fix/1115"),
            ("feat-813_worktree-overview", "feat/813"),
            ("chore/42", "chore/42"),
        ] {
            assert_eq!(
                short_branch(branch).as_deref(),
                Some(short),
                "{branch} has a type and a number"
            );
        }
    }

    /// C5: no type, no number, an unconventional type, an empty slug or a number with letters in
    /// it stays as written.
    #[test]
    fn any_other_branch_has_no_short_form() {
        for branch in [
            "main",
            "claude/project-thread-7041o6",
            "feat/make-branch-selector",
            "scratch/023-docs-gate",
            "fix/1115_",
            "fix/12a",
        ] {
            assert_eq!(short_branch(branch), None, "{branch} is shown unchanged");
        }
    }

    /// C6, US2.3, US2.4, US2.8: the short (or unchanged) branch with the full one as tooltip; no
    /// line without a branch or when the title already is the branch.
    #[test]
    fn the_branch_line_is_short_with_the_full_branch_on_hover() {
        assert_eq!(
            branch_line(
                ISSUE_TITLE,
                Some("fix/1115_container-daemon-fails-to-start")
            ),
            Some(Line {
                text: "fix/1115".into(),
                tooltip: "fix/1115_container-daemon-fails-to-start".into(),
            })
        );
        assert_eq!(
            branch_line(ISSUE_TITLE, Some("claude/project-thread-7041o6")),
            Some(Line {
                text: "claude/project-thread-7041o6".into(),
                tooltip: "claude/project-thread-7041o6".into(),
            }),
            "a branch without a short form shows unchanged"
        );
        assert_eq!(branch_line("Default", None), None, "no branch, no line");
        assert_eq!(
            branch_line("feat/813_x", Some("feat/813_x")),
            None,
            "the title is the branch: not repeated"
        );
    }

    /// C7, US2.5: singular for one, the running count only when some run, clamped to the total.
    #[test]
    fn the_sessions_line_counts_sessions_and_the_running_ones() {
        for (counts, line) in [
            ((0, 0), "No sessions"),
            ((1, 1), "1 session \u{b7} 1 running"),
            ((1, 0), "1 session"),
            ((2, 0), "2 sessions"),
            ((3, 2), "3 sessions \u{b7} 2 running"),
            ((120, 101), "120 sessions \u{b7} 101 running"),
            ((2, 5), "2 sessions \u{b7} 2 running"),
        ] {
            assert_eq!(sessions_line(counts), line, "{counts:?}");
        }
    }

    /// C8, US2.6: no row with neither a phase nor a ratio; a bar only with a ratio.
    #[test]
    fn the_progress_row_is_hidden_without_a_phase_or_a_ratio() {
        assert_eq!(progress_row(&progress(None, None)), None);
        assert_eq!(
            progress_row(&progress(Some(Phase::Implement), Some(ratio(6, 10)))),
            Some(ProgressRow {
                text: "Implement \u{b7} 6/10".into(),
                bar: Some(ratio(6, 10)),
            })
        );
        assert_eq!(
            progress_row(&progress(None, Some(ratio(6, 10)))),
            Some(ProgressRow {
                text: "6/10".into(),
                bar: Some(ratio(6, 10)),
            })
        );
        assert_eq!(
            progress_row(&progress(Some(Phase::Plan), None)),
            Some(ProgressRow {
                text: "Plan".into(),
                bar: None,
            }),
            "a phase alone draws no bar"
        );
    }

    /// C3: the line with its todo ratio, the same text on hover; nothing for no activity or a
    /// blank line.
    #[test]
    fn the_activity_line_carries_its_ratio_and_is_hidden_when_blank() {
        let with = activity("Writing tests", Some(ratio(2, 5)));
        assert_eq!(
            activity_line(Some(&with)),
            Some(Line {
                text: "Writing tests \u{b7} 2/5".into(),
                tooltip: "Writing tests \u{b7} 2/5".into(),
            })
        );
        let without = activity("Writing tests", None);
        assert_eq!(
            activity_line(Some(&without)).map(|l| l.text),
            Some("Writing tests".into())
        );
        assert_eq!(activity_line(None), None);
        assert_eq!(
            activity_line(Some(&activity("   ", Some(ratio(1, 2))))),
            None,
            "a blank line is not shown, and the ratio not alone"
        );
    }

    fn model() -> CardModel {
        let location = SessionLocation::Worktree("feat-1120_x".into());
        CardModel {
            location: location.clone(),
            display_name: "feat-1120_x".into(),
            branch: Some("feat/1120_x".into()),
            kind: LocationType::Feature,
            sessions: (1, 1),
            state: CardState::Working,
            reason: CardReason::Working,
            title: ISSUE_TITLE.into(),
            prs: PrDisplay::Counts(PrCounts {
                open: 1,
                merged: 0,
                open_capped: false,
                merged_capped: false,
            }),
            progress: progress(None, None),
            activity: Some(activity("Writing tests", None)),
            activation: Activation::RevealOnly(location),
        }
    }

    /// Data-model *CardText*: each field from its rule.
    #[test]
    fn card_text_takes_each_field_from_its_rule() {
        let m = model();
        assert_eq!(
            CardText::of(&m),
            CardText {
                title: ISSUE_TITLE.into(),
                title_tooltip: "feat-1120_x".into(),
                branch: Some(Line {
                    text: "feat/1120".into(),
                    tooltip: "feat/1120_x".into(),
                }),
                sessions: "1 session \u{b7} 1 running".into(),
                progress: None,
                prs: Some(("1 open".into(), None)),
                activity: Some(Line {
                    text: "Writing tests".into(),
                    tooltip: "Writing tests".into(),
                }),
            }
        );
    }
}

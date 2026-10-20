//! What an Overview card says (feature 1120, research R1): its title, short branch, sessions line,
//! progress row and activity line, derived from a [`CardModel`]. The card widget only places these
//! words; every rule with a branch in it lives here so it is tested without the GUI (FR-018).
//!
//! Pure: no I/O and no logging. The activity line carries a user's prompt text, so nothing here may
//! be logged (FR-020); the types derive `Debug` only for tests' sake.

use crate::task_progress::Ratio;

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
    /// The short branch, unless the title is the branch (FR-007). The full branch is on the
    /// expanded card (feature 1121, FR-004), so the collapsed one has no tooltip.
    pub branch: Option<String>,
    /// *No sessions*, *1 session · 1 running* (FR-008).
    pub sessions: String,
    /// The progress row, when there is a phase or a ratio (FR-009).
    pub progress: Option<ProgressRow>,
    /// The pull request row and, for the unknown mark, its tooltip (FR-010).
    pub prs: Option<(String, Option<String>)>,
    /// What the agent is doing now, when its hooks report (FR-011).
    pub activity: Option<String>,
}

/// The card's title: the known issue title, else the branch, else the display name (C4, FR-006).
/// A blank issue title counts as unknown.
pub fn card_title(issue_title: Option<&str>, branch: Option<&str>, display_name: &str) -> String {
    issue_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
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
/// unchanged) branch (C6, FR-007).
pub fn branch_line(title: &str, branch: Option<&str>) -> Option<String> {
    let branch = branch.filter(|b| *b != title)?;
    Some(short_branch(branch).unwrap_or_else(|| branch.to_string()))
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

/// What an activity text says (contract C2): its words without markup ([`readable`]), and a slash
/// command as a sentence, *Running autopilot on #1117*; `None` when nothing readable is left.
///
/// [`readable`]: crate::markup::readable
pub fn activity_text(text: &str) -> Option<String> {
    let words = crate::markup::readable(text)?;
    let Some(command) = words.strip_prefix('/') else {
        return Some(words);
    };
    let (name, args) = command.split_once(' ').unwrap_or((command, ""));
    let is_name = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.');
    if name.is_empty() || !name.chars().all(is_name) {
        return Some(words);
    }
    let name = match name.strip_prefix("speckit-") {
        Some(rest) if !rest.is_empty() => rest,
        _ => name,
    };
    let name = name.replace('-', " ");
    let name = name.trim();
    if name.is_empty() {
        return Some(words);
    }
    Some(match args {
        "" => format!("Running {name}"),
        args => format!("Running {name} on {args}"),
    })
}

/// The activity line with its todo ratio (*Writing tests · 2/5*); `None` for no activity or a
/// blank line, so a ratio never shows alone (C3).
pub fn activity_line(activity: Option<&crate::hook_activity::CurrentActivity>) -> Option<String> {
    let activity = activity?;
    let line = activity_text(&activity.line)?;
    Some(match activity.ratio {
        Some(r) => format!("{line} \u{b7} {}/{}", r.done, r.total),
        None => line,
    })
}

/// The worktree directory as the expanded card shows it (contract K6, FR-014, D2): a path inside
/// `home` is `~`, the platform separator, then the rest; `home` itself is `~`; any other path, or
/// no `home`, is shown in full. Whole path components only, so `/home/al` is not inside
/// `/home/alice`.
pub fn worktree_dir(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    let Some(rest) = home.and_then(|home| path.strip_prefix(home).ok()) else {
        return path.display().to_string();
    };
    if rest.as_os_str().is_empty() {
        "~".to_string()
    } else {
        format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display())
    }
}

/// What an expanded card says beyond the collapsed one (contract K5, data-model *ExpandedText*):
/// every text in full, for the card to wrap with no ellipsis (FR-004, FR-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedText {
    /// The title in full.
    pub title: String,
    /// The location's display name, when it differs from the title.
    pub display_name: Option<String>,
    /// The full branch, when there is one and the title is not the branch.
    pub branch: Option<String>,
    /// The worktree directory, home folder as `~` ([`worktree_dir`]).
    pub dir: String,
    /// The whole activity line, when the hooks report one.
    pub activity: Option<String>,
}

impl ExpandedText {
    /// What `model`'s expanded card says, with `home` the user's home folder (K5).
    pub fn of(model: &crate::overview::CardModel, home: Option<&std::path::Path>) -> Self {
        let title = &model.title;
        Self {
            title: title.clone(),
            display_name: Some(&model.display_name)
                .filter(|name| *name != title)
                .cloned(),
            branch: model.branch.clone().filter(|branch| branch != title),
            dir: worktree_dir(&model.dir, home),
            activity: activity_line(model.activity.as_ref()),
        }
    }
}

impl CardText {
    /// What `model`'s card says, each field from its rule (data-model *CardText*).
    pub fn of(model: &crate::overview::CardModel) -> Self {
        Self {
            branch: branch_line(&model.title, model.branch.as_deref()),
            title: model.title.clone(),
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
        assert_eq!(
            card_title(Some("  Fix the gate \n"), Some("main"), "d"),
            "Fix the gate",
            "an issue title is trimmed"
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

    /// C6, US2.3, US2.4, US2.8: the short (or unchanged) branch; no line without a branch or when
    /// the title already is the branch.
    #[test]
    fn the_branch_line_is_short() {
        assert_eq!(
            branch_line(
                ISSUE_TITLE,
                Some("fix/1115_container-daemon-fails-to-start")
            ),
            Some("fix/1115".into())
        );
        assert_eq!(
            branch_line(ISSUE_TITLE, Some("claude/project-thread-7041o6")),
            Some("claude/project-thread-7041o6".into()),
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
        assert_eq!(
            progress_row(&progress(None, None)),
            None,
            "C8: no phase and no ratio hides the row"
        );
        assert_eq!(
            progress_row(&progress(Some(Phase::Implement), Some(ratio(6, 10)))),
            Some(ProgressRow {
                text: "Implement \u{b7} 6/10".into(),
                bar: Some(ratio(6, 10)),
            }),
            "C8: phase and ratio show text and bar"
        );
        assert_eq!(
            progress_row(&progress(None, Some(ratio(6, 10)))),
            Some(ProgressRow {
                text: "6/10".into(),
                bar: Some(ratio(6, 10)),
            }),
            "C8: a ratio alone shows the ratio and a bar"
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

    /// C3: the line with its todo ratio; nothing for no activity or a blank line.
    #[test]
    fn the_activity_line_carries_its_ratio_and_is_hidden_when_blank() {
        let with = activity("Writing tests", Some(ratio(2, 5)));
        assert_eq!(
            activity_line(Some(&with)),
            Some("Writing tests \u{b7} 2/5".into())
        );
        let without = activity("Writing tests", None);
        assert_eq!(activity_line(Some(&without)), Some("Writing tests".into()));
        assert_eq!(activity_line(None), None);
        assert_eq!(
            activity_line(Some(&activity("   ", Some(ratio(1, 2))))),
            None,
            "a blank line is not shown, and the ratio not alone"
        );
    }

    /// C2: markup removed, a slash command read as a sentence.
    #[test]
    fn a_slash_command_reads_as_a_sentence() {
        for (text, want) in [
            ("/speckit-autopilot #1117", "Running autopilot on #1117"),
            ("/clear", "Running clear"),
            ("/compact keep the plan", "Running compact on keep the plan"),
            (
                r#"/speckit-autopilot <agent-message from="o">#1117</agent-message>"#,
                "Running autopilot on #1117",
            ),
            ("/speckit-", "Running speckit"),
            ("/etc", "Running etc"),
            ("/speckit-tdd-run", "Running tdd run"),
        ] {
            assert_eq!(activity_text(text).as_deref(), Some(want), "{text}");
        }
    }

    /// C2: a path, a lone slash and plain words stay as written; markup only is nothing.
    #[test]
    fn other_text_stays_and_markup_only_is_none() {
        for text in ["/home/user/x", "/", "Writing tests"] {
            assert_eq!(activity_text(text).as_deref(), Some(text), "{text}");
        }
        assert_eq!(
            activity_text(r#"<agent-message from="x"></agent-message>"#),
            None
        );
    }

    /// C3 with sanitising: the words of a wrapped prompt, on one line; markup only hides the line
    /// and its ratio; a todo item is sanitised the same way.
    #[test]
    fn the_activity_line_shows_no_markup() {
        let wrapped = activity(
            r#"<agent-message from="orchestrator">Continue M2</agent-message>"#,
            None,
        );
        assert_eq!(activity_line(Some(&wrapped)), Some("Continue M2".into()));
        let markup_only = activity(
            "<agent-message from=\"x\"></agent-message>",
            Some(ratio(1, 2)),
        );
        assert_eq!(
            activity_line(Some(&markup_only)),
            None,
            "the ratio never shows alone"
        );
        let todo = activity("<b>Writing</b> tests", Some(ratio(2, 5)));
        assert_eq!(
            activity_line(Some(&todo)),
            Some("Writing tests \u{b7} 2/5".into())
        );
        let multi = activity("fix the bug\n  and the test", None);
        assert_eq!(
            activity_line(Some(&multi)),
            Some("fix the bug and the test".into())
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
            dir: std::path::Path::new(HOME)
                .join("code")
                .join("app")
                .join(".worktrees")
                .join("feat-1120_x"),
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
                branch: Some("feat/1120".into()),
                sessions: "1 session \u{b7} 1 running".into(),
                progress: None,
                prs: Some(("1 open".into(), None)),
                activity: Some("Writing tests".into()),
            },
            "CardText: each field comes from its rule"
        );
    }

    /// K6 (FR-014, D2): a path inside the home folder starts with `~`, in the platform's
    /// separators; the home folder itself is `~`.
    #[test]
    fn a_directory_inside_the_home_folder_starts_with_a_tilde() {
        let home = std::path::Path::new("/home/alice");
        let dir = home.join("code").join("app").join(".worktrees").join("x");
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            worktree_dir(&dir, Some(home)),
            format!("~{sep}code{sep}app{sep}.worktrees{sep}x"),
            "the home folder at the start of the path is shown as ~"
        );
        assert_eq!(
            worktree_dir(home, Some(home)),
            "~",
            "the home folder itself is ~"
        );
    }

    /// K6: a path outside the home folder, or with no home folder known, is shown in full; a path
    /// that only shares a string prefix with the home folder is outside it.
    #[test]
    fn a_directory_outside_the_home_folder_is_shown_in_full() {
        let home = std::path::Path::new("/home/alice");
        let outside = std::path::Path::new("/srv/repos/app");
        assert_eq!(
            worktree_dir(outside, Some(home)),
            outside.display().to_string(),
            "a path outside the home folder is not shortened"
        );
        let inside = home.join("code");
        assert_eq!(
            worktree_dir(&inside, None),
            inside.display().to_string(),
            "with no home folder known the path is shown in full"
        );
        let prefix_only = std::path::Path::new("/home/al");
        assert_eq!(
            worktree_dir(&home.join("x"), Some(prefix_only)),
            home.join("x").display().to_string(),
            "a home folder that is only a string prefix does not shorten the path"
        );
    }

    const HOME: &str = "/home/alice";

    fn expanded(model: &CardModel) -> ExpandedText {
        ExpandedText::of(model, Some(std::path::Path::new(HOME)))
    }

    /// K5 (FR-004): the expanded card shows the title, display name, full branch, directory and
    /// whole activity, each in full.
    #[test]
    fn the_expanded_text_holds_every_text_in_full() {
        let long_activity = "Writing the tests for the expanded card, which must show the whole \
                             activity line however long it is, never clipped to one line";
        let m = CardModel {
            activity: Some(activity(long_activity, Some(ratio(2, 5)))),
            ..model()
        };
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            expanded(&m),
            ExpandedText {
                title: ISSUE_TITLE.into(),
                display_name: Some("feat-1120_x".into()),
                branch: Some("feat/1120_x".into()),
                dir: format!("~{sep}code{sep}app{sep}.worktrees{sep}feat-1120_x"),
                activity: Some(format!("{long_activity} \u{b7} 2/5")),
            },
            "ExpandedText: every text in full, the branch not shortened"
        );
    }

    /// K5: the display name and the branch are left out when the title already is them; no
    /// activity, no activity line.
    #[test]
    fn the_expanded_text_leaves_out_what_the_title_already_says() {
        let named = CardModel {
            title: "feat-1120_x".into(),
            branch: None,
            activity: None,
            ..model()
        };
        let text = expanded(&named);
        assert_eq!(text.title, "feat-1120_x");
        assert_eq!(
            text.display_name, None,
            "a title that is the display name shows once"
        );
        assert_eq!(text.branch, None, "no branch, no branch line");
        assert_eq!(text.activity, None, "no activity, no activity line");
        let branch_titled = CardModel {
            title: "feat/1120_x".into(),
            ..model()
        };
        assert_eq!(
            expanded(&branch_titled).branch,
            None,
            "a title that is the branch shows once"
        );
    }

    /// K5 (Edge Case): an orphan or invalid location has no branch and shows its recorded path.
    #[test]
    fn an_orphan_shows_its_recorded_directory_and_no_branch() {
        let orphan = CardModel {
            title: "gone".into(),
            display_name: "gone".into(),
            branch: None,
            kind: LocationType::OrphanInvalid,
            dir: std::path::PathBuf::from("/srv/old/gone"),
            ..model()
        };
        let text = expanded(&orphan);
        assert_eq!(text.branch, None);
        assert_eq!(
            text.dir,
            std::path::Path::new("/srv/old/gone").display().to_string(),
            "the recorded path, in full"
        );
    }
}

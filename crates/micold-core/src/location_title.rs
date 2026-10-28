//! A worktree's location title (feature 1504, FR-001): one chooser that the Overview card, the
//! sidebar row and the session-area header all read, so they never disagree (FR-007). The inputs
//! are gathered by the client; this module only picks the first usable one, in the order of
//! `specs/1504-readable-worktree-titles/contracts/location-title.md` (L1), after normalising each
//! to its first line (L4, FR-012). The *title ticket* it is read for comes from the folder name,
//! else the branch name (L2, FR-002).
//!
//! Title texts stay on this machine (FR-011): [`LocationTitle`]'s `Debug` leaves its text out.

use crate::naming;

/// The ticket a location's title is read for (FR-002): the folder name's ticket, else the branch
/// name's. Used only for the title; the linked ticket (issue tag, Jira chip) stays folder-only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TitleTicket {
    /// An all-digit ticket: a GitHub issue number.
    Github(u64),
    /// A `<LETTERS>-<digits>` Jira key, uppercased.
    Jira(String),
}

/// Where a location title came from, in FR-001 order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleSource {
    /// The title ticket's GitHub issue title.
    GithubIssue(u64),
    /// The title ticket's Jira issue summary.
    JiraIssue(String),
    /// The open or draft pull request of the branch.
    PullRequest(u64),
    /// The feature spec's heading.
    Spec,
    /// The main session's title.
    Session,
    /// The subject of the branch's latest own commit.
    Commit,
    /// The branch name.
    Branch,
    /// The folder (the location's display name).
    Folder,
}

impl TitleSource {
    /// The source in words for a title's tooltip (FR-008, L3); `None` for [`TitleSource::Folder`],
    /// whose title keeps today's tooltip.
    pub fn words(&self) -> Option<String> {
        Some(match self {
            TitleSource::GithubIssue(number) => format!("From GitHub issue #{number}"),
            TitleSource::JiraIssue(key) => format!("From Jira {key}"),
            TitleSource::PullRequest(number) => format!("From pull request #{number}"),
            TitleSource::Spec => "From the feature spec".into(),
            TitleSource::Session => "From the session title".into(),
            TitleSource::Commit => "From the latest commit".into(),
            TitleSource::Branch => "Branch name".into(),
            TitleSource::Folder => return None,
        })
    }
}

/// What [`choose`] picks from (data-model *TitleInputs*). A field that is `None` gives no title.
#[derive(Debug, Clone, Copy)]
pub struct TitleInputs<'a> {
    /// The location's title ticket ([`title_ticket`]).
    pub ticket: Option<&'a TitleTicket>,
    /// The title ticket's issue title (GitHub) or summary (Jira), once read.
    pub issue_title: Option<&'a str>,
    /// The branch's open or draft pull request: its number and title.
    pub pull_request: Option<(u64, &'a str)>,
    /// The feature spec's heading.
    pub spec: Option<&'a str>,
    /// The main session's title.
    pub session: Option<&'a str>,
    /// The subject of the branch's latest own commit.
    pub commit: Option<&'a str>,
    /// The branch, used as it is.
    pub branch: Option<&'a str>,
    /// The location's display name, used as it is: the last resort.
    pub display_name: &'a str,
}

impl<'a> TitleInputs<'a> {
    /// Inputs with only the display name: a location with nothing else to say, such as the project
    /// root.
    pub fn named(display_name: &'a str) -> Self {
        TitleInputs {
            ticket: None,
            issue_title: None,
            pull_request: None,
            spec: None,
            session: None,
            commit: None,
            branch: None,
            display_name,
        }
    }
}

/// A location's title and where it came from.
#[derive(Clone, PartialEq, Eq)]
pub struct LocationTitle {
    /// The title as shown.
    pub text: String,
    /// Its source.
    pub source: TitleSource,
}

/// Leaves the text out: a title is the user's data and must never reach a log (FR-011, L9).
impl std::fmt::Debug for LocationTitle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocationTitle")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// The usable part of a source's text (FR-012, L4): a leading BOM stripped, the first line only,
/// trimmed (which drops a trailing `\r`). `None` when nothing is left.
pub fn first_line(text: &str) -> Option<&str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let line = text.split('\n').next().unwrap_or_default().trim();
    (!line.is_empty()).then_some(line)
}

/// The title in a feature spec's heading (FR-012, L4): its first `# ` line, normalised as
/// [`first_line`], without the `Feature Specification:` prefix. `None` without such a line, and
/// for the template's `[FEATURE NAME]` or any other text that is only a `[...]` placeholder.
pub fn spec_title(text: &str) -> Option<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let heading = text.lines().find_map(|line| line.strip_prefix("# "))?;
    let heading = first_line(heading)?;
    let title = heading
        .strip_prefix("Feature Specification:")
        .unwrap_or(heading)
        .trim();
    let placeholder = title.starts_with('[') && title.ends_with(']');
    (!title.is_empty() && !placeholder).then(|| title.to_string())
}

/// The spec heading title of the worktree at `root` (FR-001 item 4, L7): the `spec.md` of the
/// folder `.specify/feature.json` names, else of the one `specs/<ticket>-*` folder. `ticket` is the
/// number of the worktree's GitHub title ticket. Blocking I/O: call it off the UI thread. A
/// missing, unreadable or huge file gives none.
pub fn read_spec_title(root: &std::path::Path, ticket: Option<u64>) -> Option<String> {
    let ticket = ticket.map(|number| number.to_string());
    let folder = crate::task_progress::feature_folder(root, ticket.as_deref())?;
    spec_title(&crate::task_progress::read_text(
        &root.join(folder).join("spec.md"),
    )?)
}

/// The location title (FR-001, L1): the first input in order that gives a text. The issue title
/// counts only for its ticket's kind; `branch` and `display_name` are used as they are.
pub fn choose(inputs: &TitleInputs) -> LocationTitle {
    let issue = match inputs.ticket {
        Some(TitleTicket::Github(number)) => Some(TitleSource::GithubIssue(*number)),
        Some(TitleTicket::Jira(key)) => Some(TitleSource::JiraIssue(key.clone())),
        None => None,
    };
    let normalised = [
        issue.zip(inputs.issue_title),
        inputs
            .pull_request
            .map(|(number, text)| (TitleSource::PullRequest(number), text)),
        inputs.spec.map(|text| (TitleSource::Spec, text)),
        inputs.session.map(|text| (TitleSource::Session, text)),
        inputs.commit.map(|text| (TitleSource::Commit, text)),
    ]
    .into_iter()
    .flatten()
    .find_map(|(source, text)| first_line(text).map(|text| (source, text)));
    let (source, text) = normalised
        .or_else(|| {
            inputs
                .branch
                .filter(|branch| !branch.is_empty())
                .map(|branch| (TitleSource::Branch, branch))
        })
        .unwrap_or((TitleSource::Folder, inputs.display_name));
    LocationTitle {
        text: text.to_string(),
        source,
    }
}

/// The title ticket of a worktree (FR-002, research R2, L2): the folder name's ticket when it is
/// one; else the branch's, read as the app names branches (`<type>/<ticket>_<name>`, past any
/// namespace segment that is no conventional type, such as `jh/`), else as the card's short branch
/// reads it (`<type>/<digits>` then the end, `_` or `-`). A ticket that is neither a number nor a
/// Jira key ([`issue_link::ticket_ref`](crate::issue_link::ticket_ref)) is no title ticket.
pub fn title_ticket(dir_name: &str, branch: Option<&str>) -> Option<TitleTicket> {
    if let Some(ticket) = naming::ticket(dir_name).as_deref().and_then(classify) {
        return Some(ticket);
    }
    let branch = branch?;
    naming::ticket(&naming::dir_name_from_branch(unnamespaced(branch)))
        .as_deref()
        .and_then(classify)
        .or_else(|| {
            let short = crate::overview_text::short_branch(branch)?;
            classify(short.rsplit('/').next()?)
        })
}

/// The branch from its first conventional type segment on, or its last segment when it has none:
/// a leading `jh/` or `user/` names a person, not a Jira project (review A M1, F2).
fn unnamespaced(branch: &str) -> &str {
    let mut rest = branch;
    while let Some((head, tail)) = rest.split_once('/') {
        if naming::ConventionalType::from_token(head).is_some() {
            break;
        }
        rest = tail;
    }
    rest
}

/// A ticket as a tracker writes it, through the one rule the issue link uses.
fn classify(ticket: &str) -> Option<TitleTicket> {
    match crate::issue_link::ticket_ref(ticket)? {
        crate::issue_link::IssueRef::Github(number) => Some(TitleTicket::Github(number)),
        crate::issue_link::IssueRef::Jira(key) => Some(TitleTicket::Jira(key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUE_TITLE: &str = "Readable worktree titles";

    // ---- L4: first_line ----

    #[test]
    fn first_line_strips_a_bom_and_keeps_the_first_line_trimmed() {
        assert_eq!(first_line("\u{feff}Title"), Some("Title"));
        assert_eq!(first_line("First\nSecond"), Some("First"));
        assert_eq!(first_line("Windows line\r\nnext"), Some("Windows line"));
        assert_eq!(first_line("  padded \t"), Some("padded"));
        assert_eq!(first_line("\u{feff}  Both \r\n"), Some("Both"));
    }

    #[test]
    fn first_line_of_nothing_is_none() {
        assert_eq!(first_line(""), None);
        assert_eq!(first_line("   \t "), None);
        assert_eq!(first_line("\u{feff}"), None);
        assert_eq!(first_line("\n second line only"), None);
    }

    // ---- L1: choose ----

    static GITHUB: TitleTicket = TitleTicket::Github(1504);

    /// Every source given a text: the full set [`choose`] runs down.
    fn all<'a>(ticket: &'a TitleTicket) -> TitleInputs<'a> {
        TitleInputs {
            ticket: Some(ticket),
            issue_title: Some(ISSUE_TITLE),
            pull_request: Some((1520, "Pull request title")),
            spec: Some("Spec heading"),
            session: Some("Session title"),
            commit: Some("Commit subject"),
            branch: Some("feat/1504_x"),
            display_name: "folder",
        }
    }

    /// Drop the sources above `source`, one at a time, and check each in turn wins over all
    /// below it (FR-001 order).
    #[test]
    fn each_source_wins_over_every_lower_one_in_order() {
        let mut inputs = all(&GITHUB);
        let expected = [
            (TitleSource::GithubIssue(1504), ISSUE_TITLE),
            (TitleSource::PullRequest(1520), "Pull request title"),
            (TitleSource::Spec, "Spec heading"),
            (TitleSource::Session, "Session title"),
            (TitleSource::Commit, "Commit subject"),
            (TitleSource::Branch, "feat/1504_x"),
            (TitleSource::Folder, "folder"),
        ];
        for (step, (source, text)) in expected.into_iter().enumerate() {
            let title = choose(&inputs);
            assert_eq!(
                (title.source, title.text.as_str()),
                (source, text),
                "step {step}"
            );
            match step {
                0 => inputs.issue_title = None,
                1 => inputs.pull_request = None,
                2 => inputs.spec = None,
                3 => inputs.session = None,
                4 => inputs.commit = None,
                5 => inputs.branch = None,
                _ => {}
            }
        }
    }

    #[test]
    fn a_jira_ticket_names_its_summary_and_wins_over_the_pull_request() {
        let jira = TitleTicket::Jira("ABC-42".into());
        let title = choose(&TitleInputs {
            issue_title: Some("Login fails"),
            ..all(&jira)
        });
        assert_eq!(title.source, TitleSource::JiraIssue("ABC-42".into()));
        assert_eq!(title.text, "Login fails");
    }

    #[test]
    fn an_issue_title_without_a_ticket_gives_no_title() {
        let title = choose(&TitleInputs {
            ticket: None,
            issue_title: Some(ISSUE_TITLE),
            ..TitleInputs::named("folder")
        });
        assert_eq!(title.source, TitleSource::Folder);
    }

    #[test]
    fn a_source_that_normalises_to_nothing_is_skipped() {
        let title = choose(&TitleInputs {
            issue_title: Some("  \n Second line"),
            pull_request: Some((7, "\u{feff}")),
            spec: Some(""),
            session: Some(" \r\n"),
            commit: Some("  Fix the gate \r\nbody"),
            ..all(&GITHUB)
        });
        assert_eq!(title.source, TitleSource::Commit);
        assert_eq!(title.text, "Fix the gate");
    }

    // The five cases of the old `overview_text::card_title` (feature 1120, C4), now `choose`'s.

    #[test]
    fn a_known_issue_title_is_the_title() {
        let ticket = TitleTicket::Github(1120);
        let title = choose(&TitleInputs {
            ticket: Some(&ticket),
            issue_title: Some(ISSUE_TITLE),
            branch: Some("feat/1120_x"),
            ..TitleInputs::named("feat-1120_x")
        });
        assert_eq!(title.text, ISSUE_TITLE);
    }

    #[test]
    fn without_a_title_the_branch_is_used_as_it_is() {
        let title = choose(&TitleInputs {
            branch: Some("fix/1115_container"),
            ..TitleInputs::named("fix-1115_container")
        });
        assert_eq!(
            (title.source, title.text.as_str()),
            (TitleSource::Branch, "fix/1115_container")
        );
    }

    /// L1: a branch that is empty names nothing, as a missing one does: the folder is the title.
    #[test]
    fn an_empty_branch_falls_through_to_the_folder() {
        let title = choose(&TitleInputs {
            branch: Some(""),
            ..TitleInputs::named("wt-b")
        });
        assert_eq!(
            (title.source, title.text.as_str()),
            (TitleSource::Folder, "wt-b")
        );
    }

    #[test]
    fn with_neither_the_display_name_is_used() {
        let title = choose(&TitleInputs::named("Default"));
        assert_eq!(
            (title.source, title.text.as_str()),
            (TitleSource::Folder, "Default")
        );
    }

    #[test]
    fn a_blank_issue_title_counts_as_unknown() {
        let title = choose(&TitleInputs {
            ticket: Some(&GITHUB),
            issue_title: Some("   "),
            branch: Some("main"),
            ..TitleInputs::named("d")
        });
        assert_eq!(title.text, "main");
    }

    #[test]
    fn an_issue_title_is_trimmed() {
        let title = choose(&TitleInputs {
            ticket: Some(&GITHUB),
            issue_title: Some("  Fix the gate \n"),
            branch: Some("main"),
            ..TitleInputs::named("d")
        });
        assert_eq!(title.text, "Fix the gate");
    }

    // ---- L9: privacy ----

    #[test]
    fn a_location_titles_debug_leaves_its_text_out() {
        let title = choose(&all(&GITHUB));
        let dump = format!("{title:?}");
        assert!(!dump.contains(ISSUE_TITLE), "{dump}");
        assert!(dump.contains("GithubIssue(1504)"), "{dump}");
    }

    // ---- L2: title_ticket ----

    #[test]
    fn the_title_ticket_follows_the_contract_table() {
        let github = |n| Some(TitleTicket::Github(n));
        let jira = |k: &str| Some(TitleTicket::Jira(k.into()));
        for (dir, branch, expected) in [
            ("feat-813_overview", Some("feat/900_x"), github(813)),
            (
                "project-thread-t2a600",
                Some("feat/1504-readable-titles"),
                github(1504),
            ),
            (
                "project-thread-t2a600",
                Some("feat/1504_readable-titles"),
                github(1504),
            ),
            ("feat-ABC-42_login", Some("feat/1504_x"), jira("ABC-42")),
            ("feat-ABC-42_login", None, jira("ABC-42")),
            ("wt", Some("feat/abc-42_login"), jira("ABC-42")),
            ("wt", Some("fix/some_bug"), None),
            (
                "claude-project-thread-7041o6",
                Some("claude/project-thread-7041o6"),
                None,
            ),
            ("wt", None, None),
            ("wt", Some("feat/1504_überblick"), github(1504)),
            // Review A M1: a folder ticket that is neither kind leaves the branch to answer (F1);
            // a branch namespace that is no conventional type is not a Jira project (F2); a Jira
            // project key may hold digits after its first letter (F3).
            ("login_fix", Some("feat/1504_x"), github(1504)),
            ("wt", Some("jh/1504_x"), github(1504)),
            ("wt", Some("user/42_fix"), github(42)),
            ("wt", Some("jh/ABC-42_login"), jira("ABC-42")),
            ("feat-R2D2-5_x", None, jira("R2D2-5")),
            ("wt", Some("feat/ab1-42_x"), jira("AB1-42")),
        ] {
            assert_eq!(title_ticket(dir, branch), expected, "{dir} {branch:?}");
        }
    }

    // ---- L3: tooltip words ----

    /// One row per L3 line: each source in words, and none for the folder (FR-008).
    #[test]
    fn each_source_has_its_l3_words_and_the_folder_none() {
        let rows = [
            (
                TitleSource::GithubIssue(1504),
                Some("From GitHub issue #1504"),
            ),
            (
                TitleSource::JiraIssue("ABC-42".into()),
                Some("From Jira ABC-42"),
            ),
            (
                TitleSource::PullRequest(1520),
                Some("From pull request #1520"),
            ),
            (TitleSource::Spec, Some("From the feature spec")),
            (TitleSource::Session, Some("From the session title")),
            (TitleSource::Commit, Some("From the latest commit")),
            (TitleSource::Branch, Some("Branch name")),
            (TitleSource::Folder, None),
        ];
        for (source, words) in rows {
            assert_eq!(source.words().as_deref(), words, "{source:?}");
        }
    }

    // ---- L4: spec headings ----

    #[test]
    fn a_spec_heading_loses_its_feature_specification_prefix() {
        assert_eq!(
            spec_title(
                "# Feature Specification: Overview cards, readable layout and content\n\nbody"
            )
            .as_deref(),
            Some("Overview cards, readable layout and content")
        );
        assert_eq!(
            spec_title("\u{feff}# Feature Specification:   Padded  \r\nnext").as_deref(),
            Some("Padded")
        );
        assert_eq!(
            spec_title("Some preamble\n\n# A plain heading\n## Sub").as_deref(),
            Some("A plain heading"),
            "the first `# ` line, without the prefix it lacks"
        );
    }

    #[test]
    fn a_placeholder_or_missing_spec_heading_gives_no_title() {
        for text in [
            "# Feature Specification: [FEATURE NAME]\n",
            "# [Some placeholder]\n",
            "# Feature Specification:\n",
            "## Only a second-level heading\n",
            "No heading at all\n",
            "",
        ] {
            assert_eq!(spec_title(text), None, "{text:?}");
        }
    }

    fn write(root: &std::path::Path, path: &str, text: &[u8]) {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    #[test]
    fn the_spec_title_is_read_from_the_folder_feature_json_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            ".specify/feature.json",
            br#"{"feature_directory":"specs/900-named"}"#,
        );
        write(
            root,
            "specs/900-named/spec.md",
            b"# Feature Specification: Named\n",
        );
        write(
            root,
            "specs/1504-ticket/spec.md",
            b"# Feature Specification: Ticket\n",
        );
        assert_eq!(read_spec_title(root, Some(1504)).as_deref(), Some("Named"));
    }

    #[test]
    fn without_feature_json_the_spec_folder_of_the_ticket_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "specs/1504-ticket/spec.md",
            "\u{feff}# Feature Specification: Windows heading\r\n\r\nbody\r\n".as_bytes(),
        );
        write(
            root,
            "specs/1503-other/spec.md",
            b"# Feature Specification: Other\n",
        );
        assert_eq!(
            read_spec_title(root, Some(1504)).as_deref(),
            Some("Windows heading"),
            "CRLF and a BOM"
        );
        assert_eq!(read_spec_title(root, None), None, "no ticket, no folder");
        assert_eq!(read_spec_title(root, Some(42)), None, "no folder for 42");
    }

    #[test]
    fn a_missing_spec_file_or_worktree_gives_no_spec_title() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("specs/1504-empty")).unwrap();
        assert_eq!(read_spec_title(root, Some(1504)), None);
        assert_eq!(read_spec_title(&root.join("missing"), Some(1504)), None);
    }

    #[test]
    fn a_worktree_with_a_non_ascii_path_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wt-überblick");
        write(
            &root,
            "specs/7-ü/spec.md",
            "# Feature Specification: Überblick\n".as_bytes(),
        );
        assert_eq!(
            read_spec_title(&root, Some(7)).as_deref(),
            Some("Überblick")
        );
    }
}

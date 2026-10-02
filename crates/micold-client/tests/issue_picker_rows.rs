//! The rows the issue picker's view builds (feature 038, contracts/picker-row.md §4).
//!
//! One function builds the row of every issue the form holds, so a listed issue, an issue from the
//! search beyond the cap and an issue from a typed number carry the same two lines (FR-006): the
//! issue's title line as the label and its details line under it, each with the emphasis the issue
//! maps the match's spans to.

use std::path::PathBuf;

use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueList, SearchState, WorktreeForm,
};
use micold_client::ui::issue_rows;
use micold_core::github::{GithubRepo, Issue, IssueListing};
use micold_core::typeahead::{rank, Query};

fn issue(number: u64, title: &str, reporter: &str, labels: &[&str]) -> Issue {
    Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-09-29T00:00:00Z".to_string(),
    )
    .reported_by(reporter)
}

/// A form on the issue source holding `listed` from the load and `searched` from a search, with
/// `query` typed and the matches derived the way the application derives them.
fn form(listed: Vec<Issue>, searched: Vec<Issue>, query: &str) -> WorktreeForm {
    let mut form = WorktreeForm {
        source: BranchSource::Issue,
        github: GithubAvailability::Available(
            GithubRepo::from_remote_url("git@github.com:octo/widgets.git").expect("a GitHub URL"),
        ),
        issues: IssueList::Loaded {
            listing: IssueListing {
                total_open: 1_200,
                complete: false,
                issues: listed,
            },
            gh: PathBuf::from("/usr/bin/gh"),
            searched,
            search: SearchState::Idle,
        },
        ..WorktreeForm::default()
    };
    form.issue_query = query.to_string();
    let matches = rank(&form.issues.held(), |i| i.row_text(), &Query::new(query));
    form.issue_matches = matches;
    form
}

/// The issue at each row, in row order: `issue_matches` indexes the held issues.
fn issues_in_row_order(form: &WorktreeForm) -> Vec<Issue> {
    let held = form.issues.held();
    form.issue_matches
        .iter()
        .map(|(index, _)| (*held[*index]).clone())
        .collect()
}

/// U36, A1–A3: a row's label is the title line and its details the reporter and the labels; an
/// issue without labels has the reporter alone.
#[test]
fn a_listed_issues_row_carries_the_title_line_and_the_details_line() {
    let form = form(
        vec![
            issue(
                42,
                "Crash when opening empty project",
                "octocat",
                &["bug", "ui"],
            ),
            issue(7, "Sidebar flickers on resize", "hubot", &[]),
        ],
        Vec::new(),
        "",
    );
    let (rows, selected) = issue_rows(&form);
    assert_eq!(selected, None);
    assert_eq!(rows.len(), 2);

    assert_eq!(rows[0].label, "#42 Crash when opening empty project");
    assert_eq!(
        rows[0].details,
        Some(("octocat  ·  bug, ui".to_string(), Vec::new()))
    );
    assert_eq!(rows[1].label, "#7 Sidebar flickers on resize");
    assert_eq!(rows[1].details, Some(("hubot".to_string(), Vec::new())));
}

/// U36, A8 (FR-006): a listed issue, an issue from the search beyond the cap and an issue from a
/// typed number all get the two lines, with the emphasis `Issue::emphasis` gives for the match.
#[test]
fn listed_searched_and_typed_number_issues_get_the_same_two_lines() {
    // `1100` typed: the listed issue matches by its title, the first searched issue is the one
    // with that number, the second came from GitHub's search for the text.
    let form = form(
        vec![issue(17, "Follow-up to 1100", "octocat", &["bug"])],
        vec![
            issue(
                1100,
                "Titles are cut off",
                "ghost-writer",
                &["ui", "1100-series"],
            ),
            issue(2048, "Regression of 1100 on resize", "hubot", &[]),
        ],
        "1100",
    );
    let (rows, _) = issue_rows(&form);
    let issues = issues_in_row_order(&form);
    assert_eq!(rows.len(), 3, "every held issue matches `1100`");
    let mut numbers: Vec<u64> = issues.iter().map(Issue::number).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, vec![17, 1100, 2048]);

    for ((row, issue), (_, matched)) in rows.iter().zip(&issues).zip(&form.issue_matches) {
        let emphasis = issue.emphasis(&matched.spans);
        assert_eq!(row.label, issue.title_line(), "#{}", issue.number());
        assert_eq!(row.spans, emphasis.title, "#{}", issue.number());
        assert_eq!(
            row.details,
            Some((issue.details_line(), emphasis.details)),
            "#{}",
            issue.number()
        );
        assert!(
            !row.spans.is_empty() || !row.details.as_ref().expect("details").1.is_empty(),
            "#{}: a matched row emphasises its match on one of its lines",
            issue.number()
        );
    }

    // The match in a label is emphasised in the details line, after the reporter.
    let by_label = rows
        .iter()
        .zip(&issues)
        .find(|(_, issue)| issue.number() == 1100)
        .map(|(row, _)| row)
        .expect("#1100 is offered");
    assert_eq!(by_label.label, "#1100 Titles are cut off");
    assert_eq!(by_label.spans, vec![1..5]);
}

/// The picked issue's row is the one reported as selected, whatever its source.
#[test]
fn the_picked_issues_row_is_the_selected_one() {
    let mut form = form(
        vec![issue(17, "Follow-up to 1100", "octocat", &["bug"])],
        vec![issue(1100, "Titles are cut off", "hubot", &[])],
        "1100",
    );
    form.picked_issue = Some(1100);
    let (rows, selected) = issue_rows(&form);
    let at = selected.expect("the picked issue is offered");
    assert_eq!(rows[at].label, "#1100 Titles are cut off");
}

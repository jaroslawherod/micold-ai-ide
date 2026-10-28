//! The New worktree form on the GitHub source with labels filtered (feature 1400), for the covered
//! states and gates that draw the GitHub Filters button, its panel and the active line.

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::github_filter::{
    LabelAction, LabelList, Msg as GithubMsg,
};
use micold_client::features::worktree_form::{BranchSource, Msg};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing};
use micold_core::github_filter::{LabelChoices, LabelName};
use micold_core::jira_query::QueryMode;

/// The repository's labels the fixtures offer: six long enough that their chips wrap.
pub const LABELS: &[&str] = &[
    "blocked",
    "documentation",
    "good first issue",
    "in-progress",
    "needs-design-review",
    "waiting-for-upstream",
    "bug",
];

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn github(state: &mut State, msg: GithubMsg) {
    send(state, Msg::GithubFilter(msg));
}

fn issue(number: u64, title: &str, labels: &[&str]) -> Issue {
    Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-09-29T00:00:00Z".to_string(),
    )
}

/// Answer the listing the form awaits: the default one or the filtered one, capped.
fn answer(state: &mut State, issues: Vec<Issue>, complete: bool) {
    let seq = state
        .worktree_form
        .awaited_issue_load()
        .expect("a load in flight");
    let listing = IssueListing {
        total_open: if complete { issues.len() as u64 } else { 1_234 },
        complete,
        issues,
    };
    let read = (listing, PathBuf::from("/usr/bin/gh"));
    if state.worktree_form.github_filtered_load().is_some() {
        github(
            state,
            GithubMsg::Loaded {
                seq,
                result: Ok(read),
                resolved_env: None,
            },
        );
    } else {
        send(
            state,
            Msg::IssuesLoaded {
                seq,
                result: Ok(read),
                resolved_env: None,
            },
        );
    }
}

/// Exclude each of `names` through the panel's Exclude field.
fn exclude(state: &mut State, names: &[&str]) {
    for name in names {
        github(state, GithubMsg::Focused(LabelList::Exclude));
        let label = LabelName::parse(name).expect("a quotable label");
        let rows = state
            .worktree_form
            .form
            .as_ref()
            .expect("the form is open")
            .github_selector
            .rows(LabelList::Exclude);
        let index = rows
            .iter()
            .position(|row| row.action == LabelAction::Choice(label.clone()))
            .expect("the label is offered");
        github(state, GithubMsg::Picked(LabelList::Exclude, index));
    }
}

/// The GitHub source of `o/r` showing 034's list.
fn github_source(state: &mut State) {
    send(state, Msg::Opened);
    // No session, as the other picker states: the first prompt is not this state's subject.
    send(state, Msg::SessionCliChosen(None));
    send(
        state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    send(state, Msg::SourceChanged(BranchSource::Issue));
    answer(
        state,
        vec![issue(42, "Crash when opening empty project", &["bug"])],
        true,
    );
}

/// The GitHub source of `o/r` showing, the labels loaded, `names` excluded and the filtered list
/// answered, capped; the panel left open.
fn filtered_with(mut state: State, names: &[&str]) -> State {
    github_source(&mut state);
    github(&mut state, GithubMsg::PanelToggled);
    github(&mut state, GithubMsg::Focused(LabelList::Exclude));
    let seq = state
        .worktree_form
        .awaited_label_choices()
        .expect("the labels in flight");
    github(
        &mut state,
        GithubMsg::LabelsLoaded {
            seq,
            result: Ok(LabelChoices {
                labels: LABELS.iter().filter_map(|l| LabelName::parse(l)).collect(),
                not_offered: 0,
                partial: false,
            }),
        },
    );
    exclude(&mut state, names);
    github(&mut state, GithubMsg::Dismissed(LabelList::Exclude));
    answer(
        &mut state,
        vec![
            issue(42, "Crash when opening empty project", &["bug"]),
            issue(7, "Sidebar flickers on resize", &["ui"]),
        ],
        false,
    );
    state
}

/// `in-progress` and `blocked` excluded over a capped list, the panel closed: the Filters button,
/// Clear filters, the active line and the cap caption.
pub fn github_filtered(state: State) -> State {
    let mut state = filtered_with(state, &["in-progress", "blocked"]);
    github(&mut state, GithubMsg::PanelDismissed);
    state
}

/// Six labels excluded and the panel open: the Exclude field's chips wrap inside the panel.
pub fn github_filters_panel(state: State) -> State {
    filtered_with(state, &LABELS[..6])
}

/// The query [`github_advanced`] runs: a sort qualifier, so the cap caption names the first issues.
pub const ADVANCED_QUERY: &str = "-label:in-progress sort:created-asc";

/// Advanced mode with [`ADVANCED_QUERY`] run over a capped list (feature 1400, US3): the switch, the
/// Query input and **Run**, the search and the cap caption "Showing the first 2 of 1,234 issues…".
pub fn github_advanced(mut state: State) -> State {
    github_source(&mut state);
    github(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    github(
        &mut state,
        GithubMsg::DraftChanged(ADVANCED_QUERY.to_string()),
    );
    github(&mut state, GithubMsg::Run);
    answer(
        &mut state,
        vec![
            issue(3, "Settings page loses its scroll position", &["ui"]),
            issue(9, "Crash when opening empty project", &["bug"]),
        ],
        false,
    );
    state
}

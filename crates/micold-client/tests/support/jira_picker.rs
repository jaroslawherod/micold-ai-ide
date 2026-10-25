//! The New worktree form open on a loaded issue list (feature 1101), for tests that draw the
//! picker's rows.

use std::path::PathBuf;

use micold_client::app::State;
use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueList, SearchState, WorktreeForm,
};
use micold_core::github::{GithubRepo, Issue, IssueListing};
use micold_core::typeahead::{rank, Query};

use super::layout::StateUnderTest;

/// A Jira-shaped ticket with `status` (a name and a `statusCategory.key`), if any.
pub fn ticket(number: u64, title: &str, status: Option<(&str, &str)>) -> Issue {
    Issue::jira(
        &format!("ABC-{number}"),
        number,
        title.to_string(),
        Vec::new(),
        "2026-10-14T00:00:00Z".to_string(),
    )
    .reported_by("mia")
    .with_status(status.and_then(|(name, key)| {
        micold_core::jira_status::TicketStatus::from_jira(Some(name), Some(key))
    }))
}

/// The form with `listed` loaded, `query` typed and the list open.
pub fn form_with(listed: Vec<Issue>, query: &str) -> WorktreeForm {
    let mut form = WorktreeForm {
        source: BranchSource::Issue,
        github: GithubAvailability::Available(
            GithubRepo::from_remote_url("git@github.com:octo/widgets.git").expect("a GitHub URL"),
        ),
        issues: IssueList::Loaded {
            listing: IssueListing {
                total_open: listed.len() as u64,
                complete: true,
                issues: listed,
            },
            gh: PathBuf::from("/usr/bin/gh"),
            searched: Vec::new(),
            search: SearchState::Idle,
            descriptions: Default::default(),
        },
        issue_list_open: true,
        ..WorktreeForm::default()
    };
    form.issue_query = query.to_string();
    form.issue_matches = rank(&form.issues.held(), |i| i.row_text(), &Query::new(query));
    form
}

/// The application state with the form open on `form`.
pub fn state_with(form: WorktreeForm) -> StateUnderTest {
    let mut workspace = super::workspace_with(vec![("/fixture/project", vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.worktree_form.form = Some(form);
    StateUnderTest::new(state)
}

/// `state` (a project open) connected to Jira, its New worktree form on the Jira source with the
/// component `Payments` chosen over a capped listing (feature 1185): the filter row, the active
/// line with **Clear filters** and the cap caption all show. Driven through the reducer, as the
/// application gets there, so the field shows the chosen value.
pub fn jira_filtered(mut state: State) -> State {
    use micold_client::app::Message;
    use micold_client::features::worktree_form::jira_filters::FilterMsg;
    use micold_client::features::worktree_form::Msg;
    use micold_core::jira::{connection_key, JiraConnection};
    use micold_core::jira_filter::FilterKind;
    let project = state.workspace.active.clone().expect("a project is open");
    state.settings.jira_enabled = true;
    state.settings.jira_connections.insert(
        connection_key(&project),
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com")
            .expect("a Jira connection"),
    );
    let mut send = |msg: Msg| state.update(Message::WorktreeForm(msg));
    send(Msg::Opened);
    // No session, as the other Jira states: the first prompt is not this state's subject.
    send(Msg::SessionCliChosen(None));
    send(Msg::RemotesListed(Ok(Vec::new())));
    send(Msg::SourceChanged(BranchSource::Issue));
    let capped = |numbers: &[u64]| IssueListing {
        issues: numbers
            .iter()
            .map(|n| ticket(*n, "Refund is charged twice", None))
            .collect(),
        total_open: numbers.len() as u64,
        complete: false,
    };
    let answer = |state: &mut State, numbers: &[u64]| {
        let seq = state
            .worktree_form
            .awaited_issue_load()
            .expect("a load in flight");
        state.update(Message::WorktreeForm(Msg::IssuesLoaded {
            seq,
            result: Ok((capped(numbers), PathBuf::new())),
            resolved_env: None,
        }));
    };
    answer(&mut state, &[1, 2, 3]);
    let filter = |state: &mut State, msg: FilterMsg| {
        state.update(Message::WorktreeForm(Msg::JiraFilter(msg)));
    };
    filter(&mut state, FilterMsg::Focused(FilterKind::Component));
    let seq = state
        .worktree_form
        .awaited_choices()
        .expect("components in flight")
        .1;
    filter(
        &mut state,
        FilterMsg::ComponentsLoaded {
            seq,
            result: Ok(vec![
                "Payments".to_string(),
                "Auth".to_string(),
                "Ä \"quoted\" \\ name".to_string(),
            ]),
        },
    );
    filter(&mut state, FilterMsg::Picked(FilterKind::Component, 1));
    answer(&mut state, &[1, 3]);
    state
}

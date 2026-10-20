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

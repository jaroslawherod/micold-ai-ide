//! The selector's advanced JQL mode, as state transitions (feature 1186, data-model §6). The
//! switch, the draft, the run and its stale answers, the basic state kept across a switch, the
//! GitHub round trip and the pick are all decided by the reducer, so all of it is asserted here
//! with no network. That a run reaches the source exactly once is `main_tests.rs`'s `jira_query_run`.

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::jira_query::{
    self, JiraQuery, Msg as QueryMsg, QueryRun,
};
use micold_client::features::worktree_form::{
    BranchSource, IssueList, JiraAvailability, JiraSite, Msg, WorktreeForm,
};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing, IssueLoadError};
use micold_core::issue_types::default_mapping;
use micold_core::jira::{connection_key, IssueProvider, JiraConnection, ProjectKey};
use micold_core::jira_query::{QueryFailure, QueryMode, QueryRefusal, ScopedQuery, UserQuery};
use micold_core::typeahead::Direction;

fn connection() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn project() -> PathBuf {
    PathBuf::from("/work/acme")
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn query(state: &mut State, msg: QueryMsg) {
    send(state, Msg::JiraQuery(msg));
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.form.as_ref().expect("the form is open")
}

fn form_mut(state: &mut State) -> &mut WorktreeForm {
    state.worktree_form.form.as_mut().expect("the form is open")
}

fn jq(state: &State) -> &JiraQuery {
    &form(state).jira_query
}

fn settings_with_jira() -> State {
    let mut state = State::default();
    state.workspace.active = Some(project());
    state.settings.jira_enabled = true;
    state
        .settings
        .jira_connections
        .insert(connection_key(&project()), connection());
    state
}

/// A Jira-only project's form, the remotes answered: no GitHub remote.
fn jira_form() -> State {
    let mut state = settings_with_jira();
    send(&mut state, Msg::Opened);
    send(&mut state, Msg::RemotesListed(Ok(Vec::new())));
    state
}

fn github_remote() -> Vec<GitRemote> {
    vec![GitRemote {
        name: "origin".into(),
        url: "git@github.com:o/r.git".into(),
    }]
}

/// A project with a GitHub remote and a Jira connection, Jira chosen, the Issue source showing and
/// its basic list loaded.
fn both_form_on_jira() -> State {
    let mut state = settings_with_jira();
    state
        .workspace
        .issue_sources
        .insert(project(), IssueProvider::Jira);
    send(&mut state, Msg::Opened);
    send(&mut state, Msg::RemotesListed(Ok(github_remote())));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited_load(&state);
    send(&mut state, basic_loaded(seq));
    state
}

fn issue(key: &str, number: u64, summary: &str, labels: &[&str], type_name: &str) -> Issue {
    Issue::jira(
        key,
        number,
        summary.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .with_type_name(type_name)
}

fn basic_issues() -> Vec<Issue> {
    vec![
        issue("ABC-123", 123, "Add login rate limit", &[], "Bug"),
        issue("ABC-9", 9, "Tidy the footer", &["ui"], "Task"),
    ]
}

fn listing(issues: Vec<Issue>) -> IssueListing {
    IssueListing {
        total_open: issues.len() as u64,
        complete: true,
        issues,
    }
}

fn basic_loaded(seq: u64) -> Msg {
    Msg::IssuesLoaded {
        seq,
        result: Ok((listing(basic_issues()), PathBuf::new())),
        resolved_env: None,
    }
}

fn awaited_load(state: &State) -> u64 {
    match form(state).issues {
        IssueList::Loading { seq } => seq,
        ref other => panic!("expected a basic load in flight, found {other:?}"),
    }
}

/// The run in flight: its seq and scoped JQL.
fn running(state: &State) -> (u64, String) {
    match &jq(state).run {
        QueryRun::Running { seq, scoped } => (*seq, scoped.as_str().to_string()),
        other => panic!("expected a run in flight, found {other:?}"),
    }
}

/// The Jira form with the Issue source showing and its basic list loaded.
fn basic_shown() -> State {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited_load(&state);
    send(&mut state, basic_loaded(seq));
    state
}

/// The same, switched to advanced mode.
fn advanced_shown() -> State {
    let mut state = basic_shown();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    state
}

/// Advanced mode with `text` run and its answer `issues` landed.
fn advanced_loaded(text: &str, issues: Vec<Issue>) -> State {
    let mut state = advanced_shown();
    query(&mut state, QueryMsg::DraftChanged(text.into()));
    query(&mut state, QueryMsg::Run);
    let (seq, _) = running(&state);
    query(
        &mut state,
        QueryMsg::Ran {
            seq,
            result: Ok(listing(issues)),
        },
    );
    state
}

fn scoped(text: &str) -> String {
    ScopedQuery::new(
        &ProjectKey::parse("ABC").unwrap(),
        &UserQuery::parse(text).unwrap(),
    )
    .as_str()
    .to_string()
}

// ---- FR-001: the switch is offered only with the Jira source showing ----

#[test]
fn the_switch_is_offered_only_while_the_jira_source_shows() {
    let mut state = jira_form();
    assert!(
        !form(&state).offers_query_modes(),
        "the Issue source is not shown yet"
    );
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert!(form(&state).offers_query_modes());

    // GitHub: no switch, and pressing it changes nothing.
    let mut github = settings_with_jira();
    send(&mut github, Msg::Opened);
    send(&mut github, Msg::RemotesListed(Ok(github_remote())));
    send(&mut github, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(form(&github).provider, IssueProvider::Github);
    assert!(!form(&github).offers_query_modes());
    query(&mut github, QueryMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(jq(&github).mode, QueryMode::Basic);

    // The integration off: the GitHub form of 034, no switch.
    send(&mut state, Msg::JiraSettingsChanged(JiraAvailability::Off));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert!(!form(&state).offers_query_modes());
}

// ---- FR-004, FR-004a: switching and typing never run ----

#[test]
fn switching_and_typing_never_start_a_run() {
    let mut state = basic_shown();
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    query(&mut state, QueryMsg::DraftChanged("priority = High".into()));
    query(
        &mut state,
        QueryMsg::DraftChanged("priority = Highest".into()),
    );
    assert_eq!(jq(&state).run, QueryRun::Idle);
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    assert_eq!(state.worktree_form.issue_request_seq, before);
    assert_eq!(jq(&state).draft, "priority = Highest");
    assert_eq!(jq(&state).last_run, "", "typing is not running");
}

// ---- FR-007: Run cleans the draft, then runs the scoped query or refuses it ----

#[test]
fn run_cleans_the_draft_and_runs_the_scoped_query() {
    let mut state = advanced_shown();
    query(
        &mut state,
        QueryMsg::DraftChanged("  assignee = currentUser()\n AND priority = High\t".into()),
    );
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::Run);
    let cleaned = "assignee = currentUser()  AND priority = High";
    assert_eq!(jq(&state).draft, cleaned);
    assert_eq!(jq(&state).last_run, cleaned);
    let (seq, jql) = running(&state);
    assert_eq!(seq, before + 1, "the run takes the form's next seq");
    assert_eq!(state.worktree_form.issue_request_seq, seq);
    assert_eq!(state.worktree_form.awaited_jira_query(), Some(seq));
    assert_eq!(jql, scoped(cleaned));
    assert!(jql.starts_with("project = \"ABC\" AND ("), "{jql}");
}

#[test]
fn a_refused_query_sends_nothing_and_keeps_the_input() {
    let mut state = advanced_shown();
    query(
        &mut state,
        QueryMsg::DraftChanged("summary ~ \"login".into()),
    );
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::Run);
    assert_eq!(
        jq(&state).run,
        QueryRun::Refused(QueryRefusal::UnclosedQuote)
    );
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    assert_eq!(state.worktree_form.issue_request_seq, before);
    assert_eq!(jq(&state).draft, "summary ~ \"login");
    assert_eq!(jq(&state).last_run, "", "a refused query is not a run");

    query(&mut state, QueryMsg::DraftChanged("(a = b".into()));
    query(&mut state, QueryMsg::Run);
    assert_eq!(
        jq(&state).run,
        QueryRun::Refused(QueryRefusal::UnbalancedParentheses)
    );
}

// ---- Stale answers ----

#[test]
fn only_the_awaited_answer_lands_and_the_latest_run_wins() {
    let mut state = advanced_shown();
    query(&mut state, QueryMsg::DraftChanged("priority = High".into()));
    query(&mut state, QueryMsg::Run);
    let (first, _) = running(&state);
    query(&mut state, QueryMsg::DraftChanged("priority = Low".into()));
    query(&mut state, QueryMsg::Run);
    let (second, jql) = running(&state);
    assert_ne!(first, second);
    assert_eq!(jql, scoped("priority = Low"));

    // The first run answers late: dropped.
    query(
        &mut state,
        QueryMsg::Ran {
            seq: first,
            result: Ok(listing(basic_issues())),
        },
    );
    assert_eq!(running(&state).0, second);

    let found = vec![issue("ABC-7", 7, "Low one", &[], "Task")];
    query(
        &mut state,
        QueryMsg::Ran {
            seq: second,
            result: Ok(listing(found.clone())),
        },
    );
    match &jq(&state).run {
        QueryRun::Loaded { scoped: s, listing } => {
            assert_eq!(s.as_str(), scoped("priority = Low"));
            assert_eq!(listing.issues, found);
        }
        other => panic!("expected the latest result, found {other:?}"),
    }
    assert_eq!(
        form(&state).shown_issues(),
        found.iter().collect::<Vec<_>>()
    );
}

// ---- US1 AS6, FR-012: an empty result is not a failure ----

#[test]
fn an_empty_result_is_loaded_and_a_failure_is_failed() {
    let state = advanced_loaded("project = XYZ", Vec::new());
    match &jq(&state).run {
        QueryRun::Loaded { listing, .. } => assert!(listing.issues.is_empty()),
        other => panic!("an empty result is Loaded, found {other:?}"),
    }

    let mut failing = advanced_shown();
    query(&mut failing, QueryMsg::DraftChanged("asignee = me".into()));
    query(&mut failing, QueryMsg::Run);
    let (seq, _) = running(&failing);
    let rejected = QueryFailure::Rejected(vec!["Field 'asignee' does not exist.".into()]);
    query(
        &mut failing,
        QueryMsg::Ran {
            seq,
            result: Err(rejected.clone()),
        },
    );
    match &jq(&failing).run {
        QueryRun::Failed { failure, .. } => assert_eq!(*failure, rejected),
        other => panic!("expected Failed, found {other:?}"),
    }
    assert_eq!(
        jq(&failing).draft,
        "asignee = me",
        "the query stays to be fixed"
    );
}

// ---- US1 AS1: switching to advanced keeps the list until a run ----

#[test]
fn switching_to_advanced_keeps_the_list_shown_until_a_run() {
    let mut state = basic_shown();
    let shown: Vec<Issue> = form(&state).shown_issues().into_iter().cloned().collect();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    assert!(form(&state).shows_advanced());
    assert_eq!(
        form(&state)
            .shown_issues()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        shown
    );
    assert!(matches!(form(&state).issues, IssueList::Loaded { .. }));
}

#[test]
fn switching_to_advanced_discards_a_basic_load_in_flight() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let stale = awaited_load(&state);
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert!(state.worktree_form.issue_request_seq > stale);
    send(&mut state, basic_loaded(stale));
    assert_eq!(
        form(&state).issues,
        IssueList::NotRequested,
        "the late load drops"
    );
}

// ---- FR-003, US1 AS7: back to basic restores what was left ----

#[test]
fn switching_back_to_basic_restores_the_basic_state_and_keeps_the_draft() {
    let mut state = basic_shown();
    send(&mut state, Msg::IssueQueryChanged("footer".into()));
    let basic_list = form(&state).issues.clone();
    let matches = form(&state).issue_matches.clone();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    query(&mut state, QueryMsg::DraftChanged("priority = High".into()));
    query(&mut state, QueryMsg::Run);
    let (stale, _) = running(&state);

    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Basic));
    assert_eq!(
        jq(&state).run,
        QueryRun::Idle,
        "the run in flight is discarded"
    );
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    assert_eq!(form(&state).issue_query, "footer");
    assert_eq!(form(&state).issues, basic_list);
    assert_eq!(form(&state).issue_matches, matches);
    assert_eq!(
        state.worktree_form.awaited_issue_load(),
        None,
        "nothing to load"
    );
    assert!(state.worktree_form.issue_request_seq >= before);
    query(
        &mut state,
        QueryMsg::Ran {
            seq: stale,
            result: Ok(listing(Vec::new())),
        },
    );
    assert_eq!(jq(&state).run, QueryRun::Idle, "its late answer drops");

    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(jq(&state).draft, "priority = High", "the draft is kept");
    assert_eq!(jq(&state).run, QueryRun::Idle, "switching runs nothing");
}

#[test]
fn switching_to_basic_loads_a_basic_list_never_requested() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Basic));
    assert_eq!(state.worktree_form.awaited_issue_load(), Some(before + 1));
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
}

// ---- FR-003a, FR-004a: GitHub and back ----

#[test]
fn github_and_back_keeps_advanced_resets_the_draft_and_runs_the_last_query() {
    let mut state = both_form_on_jira();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    query(&mut state, QueryMsg::DraftChanged("priority = High".into()));
    query(&mut state, QueryMsg::Run);
    let (stale, _) = running(&state);
    query(&mut state, QueryMsg::DraftChanged("priority = Hi".into()));

    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Github));
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    assert!(!form(&state).offers_query_modes());
    query(
        &mut state,
        QueryMsg::Ran {
            seq: stale,
            result: Ok(listing(basic_issues())),
        },
    );
    assert_eq!(
        jq(&state).run,
        QueryRun::Idle,
        "the run's late answer drops"
    );

    let github_load = awaited_load(&state);
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_eq!(jq(&state).mode, QueryMode::Advanced, "the mode is kept");
    assert_eq!(
        jq(&state).draft,
        "priority = High",
        "the draft is the last run"
    );
    let (seq, jql) = running(&state);
    assert!(seq > github_load);
    assert_eq!(jql, scoped("priority = High"));
    assert_eq!(
        state.worktree_form.awaited_issue_load(),
        None,
        "advanced mode loads no basic list"
    );
}

#[test]
fn github_and_back_with_no_last_query_shows_the_prompt_and_sends_nothing() {
    let mut state = both_form_on_jira();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    query(&mut state, QueryMsg::DraftChanged("never run".into()));
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Github));
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_eq!(jq(&state).run, QueryRun::Idle);
    assert_eq!(jq(&state).draft, "");
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert_eq!(form(&state).issues, IssueList::NotRequested);
}

// ---- Edge "Running a new query while one is in flight": a reset drops the run ----

#[test]
fn leaving_the_issue_source_drops_a_run_in_flight() {
    let mut state = advanced_shown();
    query(&mut state, QueryMsg::DraftChanged("priority = High".into()));
    query(&mut state, QueryMsg::Run);
    let (stale, _) = running(&state);
    send(&mut state, Msg::SourceChanged(BranchSource::New));
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
    query(
        &mut state,
        QueryMsg::Ran {
            seq: stale,
            result: Ok(listing(basic_issues())),
        },
    );
    assert_eq!(jq(&state).run, QueryRun::Idle);

    // Shown again in advanced mode, the last query runs once more under a fresh seq.
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let (seq, jql) = running(&state);
    assert!(seq > stale);
    assert_eq!(jql, scoped("priority = High"));
}

// ---- FR-009, US1 AS4: a pick fills the form as a basic pick does ----

#[test]
fn an_advanced_pick_fills_the_form_as_a_basic_pick_of_the_same_issue() {
    let picked = issue("ABC-123", 123, "Add login rate limit", &[], "Bug");
    let mut advanced = advanced_loaded(
        "assignee = currentUser()",
        vec![issue("ABC-500", 500, "Other", &[], "Task"), picked.clone()],
    );
    assert_eq!(advanced.worktree_form.issue_number_at(1), Some(123));
    assert_eq!(advanced.worktree_form.issue_number_at(2), None);
    send(
        &mut advanced,
        Msg::IssuePicked {
            number: 123,
            mapping: default_mapping(),
        },
    );

    let mut basic = basic_shown();
    send(
        &mut basic,
        Msg::IssuePicked {
            number: 123,
            mapping: default_mapping(),
        },
    );
    let (a, b) = (form(&advanced), form(&basic));
    assert_eq!(a.ticket, "ABC-123");
    assert_eq!(a.ticket, b.ticket);
    assert_eq!(a.name, b.name);
    assert_eq!(a.type_, b.type_);
    assert_eq!(a.picked_issue, b.picked_issue);
    assert_eq!(a.first_prompt, b.first_prompt);
    assert!(!jq(&advanced).list_open, "the pick closes the list");

    // A row the advanced results hold and the basic list does not is picked too.
    send(
        &mut advanced,
        Msg::IssuePicked {
            number: 500,
            mapping: default_mapping(),
        },
    );
    assert_eq!(form(&advanced).ticket, "ABC-500");
}

#[test]
fn the_keyboard_moves_over_the_results_and_a_dismiss_closes_them() {
    let mut state = advanced_loaded(
        "type = Bug",
        vec![
            issue("ABC-1", 1, "One", &[], "Bug"),
            issue("ABC-2", 2, "Two", &[], "Bug"),
        ],
    );
    assert!(jq(&state).list_open);
    query(&mut state, QueryMsg::HighlightMoved(Direction::Next));
    query(&mut state, QueryMsg::HighlightMoved(Direction::Next));
    query(&mut state, QueryMsg::HighlightMoved(Direction::Next));
    assert_eq!(jq(&state).highlight, Some(1), "saturating at the last row");
    query(&mut state, QueryMsg::Dismissed);
    assert!(!jq(&state).list_open);
    query(&mut state, QueryMsg::Focused);
    assert!(jq(&state).list_open);
}

/// Editing the query takes the keyboard off the rows, so the next Enter runs the edited query
/// rather than picking a row of the last one's results (contracts/selector-ui.md §3).
#[test]
fn editing_the_query_drops_the_highlight() {
    let mut state = advanced_loaded("type = Bug", vec![issue("ABC-1", 1, "One", &[], "Bug")]);
    query(&mut state, QueryMsg::HighlightMoved(Direction::Next));
    assert_eq!(jq(&state).highlight, Some(0));
    query(
        &mut state,
        QueryMsg::DraftChanged("type = Story".to_string()),
    );
    assert_eq!(jq(&state).highlight, None);
}

// ---- FR-004a: no basic load while advanced mode shows ----

#[test]
fn no_basic_load_is_awaited_while_advanced_mode_shows() {
    let mut state = both_form_on_jira();
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    send(&mut state, Msg::SourceChanged(BranchSource::New));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Github));
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    send(&mut state, Msg::IssueRetry);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
}

#[test]
fn retry_of_a_failed_basic_list_loads_nothing_while_advanced_mode_shows() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited_load(&state);
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Err(IssueLoadError::CredentialsRefused),
            resolved_env: None,
        },
    );
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    send(&mut state, Msg::IssueRetry);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Basic));
    send(&mut state, Msg::IssueRetry);
    assert!(
        state.worktree_form.awaited_issue_load().is_some(),
        "basic retries"
    );
}

#[test]
fn an_advanced_pick_keeps_its_title_in_the_prompt_after_a_switch_to_basic() {
    let mut state = advanced_loaded(
        "type = Task",
        vec![issue("ABC-500", 500, "Only in advanced", &[], "Task")],
    );
    form_mut(&mut state).default_prompt_template = "{issue_title}".into();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 500,
            mapping: default_mapping(),
        },
    );
    assert_eq!(form(&state).first_prompt, "Only in advanced");
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Basic));
    send(&mut state, Msg::NameChanged("renamed".into()));
    assert_eq!(form(&state).first_prompt, "Only in advanced");
}

// ---- FR-015: the notice names the query ----

#[test]
fn the_notice_in_advanced_mode_names_the_query() {
    let mut state = basic_shown();
    let basic = form(&state).issue_notice().expect("a notice");
    assert!(basic.contains("your search text"), "{basic}");
    query(&mut state, QueryMsg::ModeChanged(QueryMode::Advanced));
    let site = JiraSite::of(&connection());
    let advanced = form(&state).issue_notice().expect("a notice");
    assert_eq!(advanced, jira_query::notice(&site));
    assert_eq!(
        advanced,
        "Reads issues of project ABC matching your query from https://acme.atlassian.net. Sends \
         the project key, your query and your credentials, and, with this dialog closed, the keys \
         of the tickets your worktrees are linked to, to show their status."
    );
}

// The load error type is the basic one: a Load failure is classified as basic mode's (533 FR-018).
#[test]
fn a_load_failure_lands_as_failed() {
    let mut state = advanced_shown();
    query(&mut state, QueryMsg::Run);
    let (seq, jql) = running(&state);
    assert_eq!(
        jql, "project = \"ABC\" ORDER BY updated DESC",
        "an empty query runs"
    );
    query(
        &mut state,
        QueryMsg::Ran {
            seq,
            result: Err(QueryFailure::Load(IssueLoadError::CredentialsRefused)),
        },
    );
    assert!(matches!(
        jq(&state).run,
        QueryRun::Failed {
            failure: QueryFailure::Load(IssueLoadError::CredentialsRefused),
            ..
        }
    ));
}

// ---- US2: Jira's refusal, a new run after it, and Retry (FR-010, FR-011) ----

/// Advanced mode with `text` run and failed with `failure`.
fn advanced_failed(text: &str, failure: QueryFailure) -> State {
    let mut state = advanced_shown();
    query(&mut state, QueryMsg::DraftChanged(text.into()));
    query(&mut state, QueryMsg::Run);
    let (seq, _) = running(&state);
    query(
        &mut state,
        QueryMsg::Ran {
            seq,
            result: Err(failure),
        },
    );
    state
}

fn rejected() -> QueryFailure {
    QueryFailure::Rejected(vec![
        "Field 'asignee' does not exist or you do not have permission to view it.".into(),
    ])
}

#[test]
fn a_rejected_query_keeps_the_query_as_typed() {
    let state = advanced_failed("asignee = currentUser() ORDER BY rank", rejected());
    assert!(matches!(
        &jq(&state).run,
        QueryRun::Failed { failure, .. } if *failure == rejected()
    ));
    assert_eq!(jq(&state).draft, "asignee = currentUser() ORDER BY rank");
    assert_eq!(jq(&state).last_run, "asignee = currentUser() ORDER BY rank");
    assert_eq!(form(&state).shown_issues(), Vec::<&Issue>::new());
}

#[test]
fn editing_and_running_again_replaces_the_error_with_the_new_result() {
    let mut state = advanced_failed("asignee = currentUser()", rejected());
    query(
        &mut state,
        QueryMsg::DraftChanged("assignee = currentUser()".into()),
    );
    assert!(
        matches!(jq(&state).run, QueryRun::Failed { .. }),
        "typing alone keeps the error until the next run"
    );
    query(&mut state, QueryMsg::Run);
    let (seq, jql) = running(&state);
    assert_eq!(jql, scoped("assignee = currentUser()"));
    let found = vec![issue("ABC-42", 42, "Mine", &[], "Task")];
    query(
        &mut state,
        QueryMsg::Ran {
            seq,
            result: Ok(listing(found.clone())),
        },
    );
    assert!(matches!(&jq(&state).run, QueryRun::Loaded { listing, .. } if listing.issues == found));
}

#[test]
fn retry_after_a_load_failure_runs_the_same_scoped_query_again() {
    let mut state = advanced_failed(
        "priority = High",
        QueryFailure::Load(IssueLoadError::TimedOut),
    );
    let failed_scoped = match &jq(&state).run {
        QueryRun::Failed { scoped, .. } => scoped.clone(),
        other => panic!("expected Failed, found {other:?}"),
    };
    // An edit after the failure does not change what Retry runs (FR-011).
    query(&mut state, QueryMsg::DraftChanged("priority = Low".into()));
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::Retry);
    match &jq(&state).run {
        QueryRun::Running { seq, scoped } => {
            assert_eq!(*seq, before + 1, "Retry takes the form's next seq");
            assert_eq!(*scoped, failed_scoped);
        }
        other => panic!("expected the run again, found {other:?}"),
    }
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    assert_eq!(state.worktree_form.awaited_jira_query(), Some(before + 1));
    assert_eq!(jq(&state).last_run, "priority = High");
}

#[test]
fn retry_is_ignored_unless_a_load_failure_shows() {
    let idle = advanced_shown();
    let refused = {
        let mut state = advanced_shown();
        query(&mut state, QueryMsg::DraftChanged("(a = b".into()));
        query(&mut state, QueryMsg::Run);
        state
    };
    let in_flight = {
        let mut state = advanced_shown();
        query(&mut state, QueryMsg::DraftChanged("a = b".into()));
        query(&mut state, QueryMsg::Run);
        state
    };
    let loaded = advanced_loaded("a = b", basic_issues());
    let rejected = advanced_failed("asignee = me", rejected());
    let basic = {
        let mut state = advanced_failed("a = b", QueryFailure::Load(IssueLoadError::TimedOut));
        query(&mut state, QueryMsg::ModeChanged(QueryMode::Basic));
        state
    };
    for (name, mut state) in [
        ("idle", idle),
        ("refused", refused),
        ("running", in_flight),
        ("loaded", loaded),
        ("rejected", rejected),
        ("basic mode", basic),
    ] {
        let run = jq(&state).run.clone();
        let seq = state.worktree_form.issue_request_seq;
        let awaited = state.worktree_form.awaited_jira_query();
        query(&mut state, QueryMsg::Retry);
        assert_eq!(jq(&state).run, run, "{name}");
        assert_eq!(state.worktree_form.issue_request_seq, seq, "{name}");
        assert_eq!(state.worktree_form.awaited_jira_query(), awaited, "{name}");
    }
}

#[test]
fn a_refused_query_keeps_the_draft_and_sends_nothing_even_after_a_failure() {
    let mut state = advanced_failed("a = b", QueryFailure::Load(IssueLoadError::TimedOut));
    query(
        &mut state,
        QueryMsg::DraftChanged("project = XYZ OR (a = b".into()),
    );
    let before = state.worktree_form.issue_request_seq;
    query(&mut state, QueryMsg::Run);
    assert_eq!(
        jq(&state).run,
        QueryRun::Refused(QueryRefusal::UnbalancedParentheses)
    );
    assert_eq!(jq(&state).draft, "project = XYZ OR (a = b");
    assert_eq!(state.worktree_form.issue_request_seq, before);
    assert_eq!(state.worktree_form.awaited_jira_query(), None);
}

//! The Jira filters of the issue picker, as state transitions (feature 1185, contracts/filter-ui.md,
//! data-model §6–§8). Everything the filters do is decided by the reducer, so it is asserted here
//! through `State` messages with no network: the choices and listings Jira would send are fed in
//! as the messages the shell answers with. What the shell asks Jira for, and when, is driven over
//! `FakeJiraSource` in `main_tests.rs` (`jira_filter_shell`).

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::jira_filters::{
    shows_filters, ChoiceLoad, FilterMsg, LabelSource, OFFERED,
};
use micold_client::features::worktree_form::jira_query::{Msg as QueryMsg, QueryRun};
use micold_client::features::worktree_form::{
    BranchSource, IssueList, JiraAvailability, Msg, SearchState, WorktreeForm,
};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing, IssueLoadError, NoticeSource};
use micold_core::issue_types::default_mapping;
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
use micold_core::jira_filter::{AccountId, FilterKind, JiraUser, VersionChoice};
use micold_core::jira_query::QueryMode;
use micold_core::typeahead::Direction;

const COMPONENT: FilterKind = FilterKind::Component;
const FIX_VERSION: FilterKind = FilterKind::FixVersion;

fn connection() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn project() -> PathBuf {
    PathBuf::from("/work/acme")
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn filter(state: &mut State, msg: FilterMsg) {
    send(state, Msg::JiraFilter(msg));
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.form.as_ref().expect("the form is open")
}

fn with_jira(remotes: Vec<GitRemote>) -> State {
    let mut state = State::default();
    state.workspace.active = Some(project());
    state.settings.jira_enabled = true;
    state
        .settings
        .jira_connections
        .insert(connection_key(&project()), connection());
    send(&mut state, Msg::Opened);
    send(&mut state, Msg::RemotesListed(Ok(remotes)));
    state
}

/// A Jira-only project's form, the remotes answered: no GitHub remote.
fn jira_form() -> State {
    with_jira(Vec::new())
}

fn issue(key: &str, number: u64, summary: &str, labels: &[&str]) -> Issue {
    Issue::jira(
        key,
        number,
        summary.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .with_type_name("Bug")
}

fn listing(issues: Vec<Issue>, complete: bool) -> IssueListing {
    IssueListing {
        total_open: issues.len() as u64,
        complete,
        issues,
    }
}

fn all_issues() -> Vec<Issue> {
    vec![
        issue("ABC-1", 1, "Refund twice", &["payments"]),
        issue("ABC-2", 2, "Login slow", &[]),
        issue("ABC-3", 3, "Refund button", &[]),
    ]
}

fn payments_issues() -> Vec<Issue> {
    vec![
        issue("ABC-1", 1, "Refund twice", &["payments"]),
        issue("ABC-3", 3, "Refund button", &[]),
    ]
}

fn answer(state: &mut State, seq: u64, result: Result<IssueListing, IssueLoadError>) {
    send(
        state,
        Msg::IssuesLoaded {
            seq,
            result: result.map(|listing| (listing, PathBuf::new())),
            resolved_env: None,
        },
    );
}

fn awaited(state: &State) -> u64 {
    match form(state).issues {
        IssueList::Loading { seq } => seq,
        ref other => panic!("expected a load in flight, found {other:?}"),
    }
}

/// The Jira source showing its unfiltered listing.
fn loaded() -> State {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    state
}

fn components(state: &State) -> &ChoiceLoad<String> {
    &form(state).jira_filters.components
}

fn choice_seq(state: &State) -> u64 {
    match components(state) {
        ChoiceLoad::Loading { seq } => *seq,
        other => panic!("expected components in flight, found {other:?}"),
    }
}

/// Focus Component and answer its request with `result`.
fn focus_and_answer(state: &mut State, result: Result<Vec<String>, IssueLoadError>) {
    filter(state, FilterMsg::Focused(COMPONENT));
    let seq = choice_seq(state);
    filter(state, FilterMsg::ComponentsLoaded { seq, result });
}

fn names() -> Vec<String> {
    ["Payments", "Auth", "Ä \"quoted\" \\ name"]
        .iter()
        .map(|n| n.to_string())
        .collect()
}

fn source() -> NoticeSource<'static> {
    NoticeSource::Jira {
        host: "acme.atlassian.net",
        key: "ABC",
    }
}

fn row_labels(state: &State) -> Vec<String> {
    form(state)
        .jira_filters
        .rows(COMPONENT)
        .iter()
        .map(|row| row.label(COMPONENT, Some(source())))
        .collect()
}

/// Pick the row of the component list that reads `label`.
fn pick(state: &mut State, label: &str) {
    let index = row_labels(state)
        .iter()
        .position(|l| l == label)
        .unwrap_or_else(|| panic!("no row {label:?} in {:?}", row_labels(state)));
    filter(state, FilterMsg::Picked(COMPONENT, index));
}

/// The unfiltered listing, Payments chosen and its listing answered.
fn filtered() -> State {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), true)));
    state
}

fn shown_keys(state: &State) -> Vec<String> {
    let held = form(state).issues.held();
    form(state)
        .issue_matches
        .iter()
        .filter_map(|(index, _)| held.get(*index).and_then(|i| i.key()).map(str::to_owned))
        .collect()
}

#[test]
fn the_filters_show_only_while_the_jira_source_shows() {
    let mut state = jira_form();
    assert!(!shows_filters(form(&state)), "the source is not chosen");
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert!(shows_filters(form(&state)), "while the list loads");
    let seq = awaited(&state);
    answer(&mut state, seq, Err(IssueLoadError::TimedOut));
    assert!(shows_filters(form(&state)), "after a failed load");
    send(&mut state, Msg::IssueRetry);
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    assert!(shows_filters(form(&state)), "over the loaded list");

    let github = {
        let mut state = State::default();
        state.workspace.active = Some(project());
        send(&mut state, Msg::Opened);
        send(&mut state, Msg::RemotesListed(Ok(github_remote())));
        send(&mut state, Msg::SourceChanged(BranchSource::Issue));
        state
    };
    assert!(!shows_filters(form(&github)), "never for GitHub");

    let mut off = jira_form();
    send(&mut off, Msg::JiraSettingsChanged(JiraAvailability::Off));
    send(&mut off, Msg::SourceChanged(BranchSource::Issue));
    assert!(!shows_filters(form(&off)), "never with Jira off");
}

#[test]
fn focusing_component_asks_once_and_offers_any_then_the_components() {
    let mut state = loaded();
    assert_eq!(*components(&state), ChoiceLoad::NotRequested);
    let before = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    assert!(form(&state).jira_filters.field(COMPONENT).open);
    assert_eq!(choice_seq(&state), before + 1);
    assert_eq!(state.worktree_form.filter_request_seq, before + 1);
    assert_eq!(row_labels(&state), ["Any component", "Loading components…"]);
    let seq = choice_seq(&state);
    filter(
        &mut state,
        FilterMsg::ComponentsLoaded {
            seq,
            result: Ok(names()),
        },
    );
    assert_eq!(
        row_labels(&state),
        ["Any component", "Payments", "Auth", "Ä \"quoted\" \\ name"]
    );
    filter(&mut state, FilterMsg::Dismissed(COMPONENT));
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    assert_eq!(
        state.worktree_form.filter_request_seq,
        before + 1,
        "a second focus asks nothing"
    );
}

#[test]
fn picking_a_component_reloads_under_a_new_seq_with_it_and_repeating_changes_nothing() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    let before = state.worktree_form.issue_request_seq;
    pick(&mut state, "Payments");
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    assert_eq!(form(&state).issues, IssueList::Loading { seq: before + 1 });
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#]
    );
    assert!(!form(&state).jira_filters.field(COMPONENT).open);
    assert_eq!(form(&state).jira_filters.field(COMPONENT).query, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), true)));
    assert_eq!(shown_keys(&state), ["ABC-1", "ABC-3"]);

    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    pick(&mut state, "Payments");
    assert_eq!(state.worktree_form.issue_request_seq, before, "same value");
    assert!(matches!(form(&state).issues, IssueList::Loaded { .. }));
}

#[test]
fn search_text_narrows_within_the_filtered_list_and_a_search_beyond_carries_the_filter() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    send(&mut state, Msg::IssueQueryChanged("refund".into()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Ok(listing(
            vec![
                issue("ABC-1", 1, "Refund twice", &[]),
                issue("ABC-4", 4, "Card declined", &[]),
            ],
            false,
        )),
    );
    assert_eq!(form(&state).issue_query, "refund", "the text is kept");
    assert_eq!(shown_keys(&state), ["ABC-1"], "rematched on the new list");

    send(&mut state, Msg::IssueQueryChanged("refund b".into()));
    let IssueList::Loaded {
        search: SearchState::Pending { seq },
        ..
    } = form(&state).issues
    else {
        panic!("a search beyond the capped list waits");
    };
    send(&mut state, Msg::IssueSearchDue { seq });
    let (text, filters) = state
        .worktree_form
        .jira_search_request()
        .expect("a search runs");
    assert_eq!(text, "refund b");
    assert_eq!(filters.clauses(), [r#"component = "Payments""#]);
}

#[test]
fn a_key_typed_with_filters_is_searched_with_them() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), false)));
    send(&mut state, Msg::IssueQueryChanged("ABC-2".into()));
    let IssueList::Loaded {
        search: SearchState::Pending { seq },
        ..
    } = form(&state).issues
    else {
        panic!("a key on a capped list is searched beyond it");
    };
    send(&mut state, Msg::IssueSearchDue { seq });
    let (text, filters) = state
        .worktree_form
        .jira_search_request()
        .expect("searching");
    assert_eq!(text, "ABC-2");
    assert_eq!(filters.clauses(), [r#"component = "Payments""#]);
    // Jira found nothing under the filter: the key is not shown.
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![]),
        },
    );
    assert!(!shown_keys(&state).contains(&"ABC-2".to_string()));
    assert_eq!(
        form(&state).issue_no_match_text(),
        "No open issue matches the search and filters."
    );
}

#[test]
fn any_clears_the_component_and_clear_filters_clears_all_each_reloading() {
    let mut state = filtered();
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    pick(&mut state, "Any component");
    assert!(state.worktree_form.jira_filters().is_empty());
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    assert_eq!(form(&state).jira_filters.field(COMPONENT).query, "");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    assert_eq!(form(&state).jira_filters.active_line(), None);

    filter(&mut state, FilterMsg::Focused(COMPONENT));
    pick(&mut state, "Auth");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(vec![], true)));
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::ClearedAll);
    assert!(state.worktree_form.jira_filters().is_empty());
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    assert_eq!(shown_keys(&state), ["ABC-1", "ABC-2", "ABC-3"]);

    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::ClearedAll);
    assert_eq!(
        state.worktree_form.issue_request_seq, before,
        "nothing to clear"
    );
}

#[test]
fn a_filtered_list_with_no_issue_says_so_and_offers_clear_filters() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Auth");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(vec![], true)));
    assert_eq!(
        form(&state).filtered_empty_text(),
        Some("No open issue matches the filters.")
    );
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by component Auth"),
        "the line naming the filters"
    );
    // Without a filter an empty list keeps its own wording.
    let mut unfiltered = jira_form();
    send(&mut unfiltered, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&unfiltered);
    answer(&mut unfiltered, seq, Ok(listing(vec![], true)));
    assert_eq!(form(&unfiltered).filtered_empty_text(), None);
    assert_eq!(
        form(&unfiltered).issue_no_match_text(),
        "No open issue matches."
    );
}

#[test]
fn a_project_without_components_says_so() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(vec![]));
    assert_eq!(
        row_labels(&state),
        ["Any component", "This project has no components"]
    );
}

#[test]
fn each_choice_failure_shows_its_cause_and_a_retry_while_the_picker_still_works() {
    let causes = [
        IssueLoadError::Unreachable("connection refused".into()),
        IssueLoadError::TimedOut,
        IssueLoadError::CredentialsRefused,
        IssueLoadError::NoAccess,
    ];
    for cause in causes {
        let mut state = loaded();
        focus_and_answer(&mut state, Err(cause.clone()));
        assert_eq!(
            row_labels(&state),
            [
                "Any component".to_string(),
                cause.message(source()),
                "Retry".to_string()
            ],
            "{cause:?}"
        );
        // The rest of the picker still works: search, pick, and the form can be filled by hand.
        send(&mut state, Msg::IssueQueryChanged("login".into()));
        assert_eq!(shown_keys(&state), ["ABC-2"], "{cause:?}");
        send(
            &mut state,
            Msg::IssuePicked {
                number: 2,
                mapping: default_mapping(),
            },
        );
        assert_eq!(form(&state).ticket, "ABC-2", "{cause:?}");
        send(&mut state, Msg::NameChanged("by hand".into()));
        assert!(form(&state).preview().is_ok(), "{cause:?}");
        // Retry asks again under a new seq.
        let before = state.worktree_form.filter_request_seq;
        pick(&mut state, "Retry");
        assert_eq!(choice_seq(&state), before + 1, "{cause:?}");
        let seq = choice_seq(&state);
        filter(
            &mut state,
            FilterMsg::ComponentsLoaded {
                seq,
                result: Ok(names()),
            },
        );
        assert_eq!(row_labels(&state).len(), 4, "{cause:?}");
    }
    let mut state = loaded();
    focus_and_answer(&mut state, Err(IssueLoadError::TimedOut));
    let before = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::ChoicesRetry(COMPONENT));
    assert_eq!(choice_seq(&state), before + 1, "the Retry message asks too");
}

#[test]
fn a_filtered_listing_failure_keeps_the_filter_row_and_the_active_line() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Err(IssueLoadError::RateLimited));
    assert!(matches!(form(&state).issues, IssueList::Failed { .. }));
    assert!(shows_filters(form(&state)));
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by component Payments")
    );
    // Retry reloads with the filter still chosen.
    send(&mut state, Msg::IssueRetry);
    assert!(matches!(form(&state).issues, IssueList::Loading { .. }));
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#]
    );
}

#[test]
fn a_chosen_value_gone_in_jira_shows_the_empty_state_or_the_error_with_the_filter() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Auth");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(vec![], true)));
    assert!(form(&state).filtered_empty_text().is_some());
    assert!(form(&state).jira_filters.active_line().is_some());
    send(&mut state, Msg::IssueQueryChanged(String::new()));

    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Auth");
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Err(IssueLoadError::Other(
            "Jira returned an error (400).".into(),
        )),
    );
    assert!(matches!(form(&state).issues, IssueList::Failed { .. }));
    assert!(shows_filters(form(&state)));
    assert!(form(&state).jira_filters.active_line().is_some());
}

#[test]
fn the_keyboard_reaches_every_row_picks_with_enter_and_escape_keeps_the_choice() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    let highlight = |state: &State| form(state).jira_filters.field(COMPONENT).highlight;
    filter(
        &mut state,
        FilterMsg::HighlightMoved(COMPONENT, Direction::Next),
    );
    assert_eq!(highlight(&state), Some(0), "Any is row 0");
    filter(
        &mut state,
        FilterMsg::HighlightMoved(COMPONENT, Direction::Next),
    );
    filter(
        &mut state,
        FilterMsg::HighlightMoved(COMPONENT, Direction::Next),
    );
    filter(
        &mut state,
        FilterMsg::HighlightMoved(COMPONENT, Direction::Prev),
    );
    assert_eq!(highlight(&state), Some(1));
    // Enter picks the highlighted row.
    let row = highlight(&state).unwrap();
    filter(&mut state, FilterMsg::Picked(COMPONENT, row));
    assert_eq!(
        state.worktree_form.jira_filters().component(),
        Some("Payments")
    );
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), true)));
    // Escape closes the list without changing the choice, and puts the field's text back.
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    filter(&mut state, FilterMsg::QueryChanged(COMPONENT, "Au".into()));
    assert_eq!(row_labels(&state), ["Any component", "Auth"]);
    filter(&mut state, FilterMsg::Dismissed(COMPONENT));
    let field = form(&state).jira_filters.field(COMPONENT);
    assert!(!field.open);
    assert_eq!(field.query, "Payments");
    assert_eq!(
        state.worktree_form.jira_filters().component(),
        Some("Payments")
    );
}

#[test]
fn rapid_changes_with_answers_out_of_order_show_only_the_last_combination() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    let mut seqs = Vec::new();
    for name in ["Payments", "Auth", "Payments", "Any component", "Auth"] {
        filter(&mut state, FilterMsg::Focused(COMPONENT));
        pick(&mut state, name);
        seqs.push(awaited(&state));
    }
    let last = *seqs.last().unwrap();
    // Answers arrive newest first except the last, then the stale ones again.
    for (n, seq) in seqs.iter().enumerate().rev().skip(1) {
        answer(
            &mut state,
            *seq,
            Ok(listing(
                vec![issue("ABC-9", 9 + n as u64, "Stale", &[])],
                true,
            )),
        );
        assert_eq!(form(&state).issues, IssueList::Loading { seq: last });
    }
    answer(
        &mut state,
        last,
        Ok(listing(vec![issue("ABC-7", 7, "Auth issue", &[])], true)),
    );
    assert_eq!(shown_keys(&state), ["ABC-7"]);
    assert_eq!(state.worktree_form.jira_filters().component(), Some("Auth"));
    for seq in &seqs[..seqs.len() - 1] {
        answer(&mut state, *seq, Ok(listing(all_issues(), true)));
    }
    assert_eq!(
        shown_keys(&state),
        ["ABC-7"],
        "a late stale answer changes nothing"
    );
}

fn github_remote() -> Vec<GitRemote> {
    vec![GitRemote {
        name: "origin".into(),
        url: "git@github.com:o/r.git".into(),
    }]
}

fn assert_reset(state: &State, why: &str) {
    let filters = &form(state).jira_filters;
    assert!(filters.chosen.is_empty(), "{why}");
    assert_eq!(filters.components, ChoiceLoad::NotRequested, "{why}");
    assert_eq!(filters.versions, ChoiceLoad::NotRequested, "{why}");
    assert_eq!(filters.field(COMPONENT).query, "", "{why}");
    assert_eq!(filters.field(FIX_VERSION).query, "", "{why}");
}

#[test]
fn leaving_jira_or_the_source_or_reopening_starts_with_no_filter_and_no_choices() {
    // GitHub and back, on a project with both sources.
    let mut state = with_jira(github_remote());
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Jira));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Github));
    assert_reset(&state, "switched to GitHub");
    send(&mut state, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_reset(&state, "back on Jira");

    // Leaving the Issue source.
    let mut state = filtered();
    send(&mut state, Msg::SourceChanged(BranchSource::New));
    assert_reset(&state, "left the Issue source");
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_reset(&state, "back on the Issue source");

    // A Jira availability change.
    let mut state = filtered();
    send(&mut state, Msg::JiraSettingsChanged(JiraAvailability::Off));
    assert_reset(&state, "Jira went off");

    // Reopening the form.
    let mut state = filtered();
    send(&mut state, Msg::Cancelled);
    send(&mut state, Msg::Opened);
    send(&mut state, Msg::RemotesListed(Ok(Vec::new())));
    assert_reset(&state, "reopened");
}

#[test]
fn a_pick_from_a_filtered_list_fills_the_form_as_an_unfiltered_pick() {
    let pick_one = |mut state: State| {
        send(
            &mut state,
            Msg::IssuePicked {
                number: 1,
                mapping: default_mapping(),
            },
        );
        let form = form(&state);
        (form.ticket.clone(), form.name.clone(), form.type_)
    };
    assert_eq!(pick_one(filtered()), pick_one(loaded()));
}

#[test]
fn the_cap_caption_of_a_filtered_list_follows_the_active_line() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    let many: Vec<Issue> = (1..=1000)
        .map(|n| issue(&format!("ABC-{n}"), n, "Issue", &[]))
        .collect();
    answer(&mut state, seq, Ok(listing(many, false)));
    // Short, because it follows the line naming the filters in one paragraph (ledger D12).
    assert_eq!(
        form(&state).issue_cap_caption().as_deref(),
        Some("Showing the 1,000 most recently updated.")
    );
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by component Payments")
    );
}

#[test]
fn a_filter_chosen_in_one_dialog_leaves_another_unchanged() {
    let mut one = loaded();
    let mut other = loaded();
    focus_and_answer(&mut other, Ok(vec!["Auth".into()]));
    let other_before = form(&other).clone();
    focus_and_answer(&mut one, Ok(names()));
    pick(&mut one, "Payments");
    assert_eq!(*form(&other), other_before);
    assert!(other.worktree_form.jira_filters().is_empty());
}

#[test]
fn a_reload_under_new_filters_with_text_typed_on_a_capped_list_searches_beyond_it_again() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), false)));
    send(&mut state, Msg::IssueQueryChanged("ABC-1500".into()));
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), false)));
    let IssueList::Loaded { search, .. } = &form(&state).issues else {
        panic!("loaded");
    };
    assert_eq!(*search, SearchState::Pending { seq });
    send(&mut state, Msg::IssueSearchDue { seq });
    let (text, filters) = state
        .worktree_form
        .jira_search_request()
        .expect("searching");
    assert_eq!(text, "ABC-1500");
    assert_eq!(filters.clauses(), [r#"component = "Payments""#]);

    // A complete list, or nothing typed, needs no search.
    let mut state = filtered();
    let IssueList::Loaded { search, .. } = &form(&state).issues else {
        panic!("loaded");
    };
    assert_eq!(*search, SearchState::Idle);
    filter(&mut state, FilterMsg::ClearedAll);
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), false)));
    assert!(state.worktree_form.pending_issue_search().is_none());
}

#[test]
fn a_pick_the_new_filters_leave_out_keeps_its_title_in_the_first_prompt() {
    let mut state = loaded();
    send(
        &mut state,
        Msg::SessionCliChosen(Some(micold_core::session::AiCli::ALL[0])),
    );
    state
        .worktree_form
        .form
        .as_mut()
        .unwrap()
        .default_prompt_template = "Work on {issue_title}".to_string();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 2,
            mapping: default_mapping(),
        },
    );
    assert_eq!(form(&state).first_prompt, "Work on Login slow");
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), true)));
    assert_eq!(form(&state).picked_issue, Some(2));
    assert_eq!(form(&state).first_prompt, "Work on Login slow");
}

#[test]
fn a_filter_list_closes_even_behind_a_running_create() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    assert!(form(&state).jira_filters.field(COMPONENT).open);
    state.worktree_form.form.as_mut().unwrap().status =
        micold_client::features::worktree_form::WorktreeFormStatus::Creating;
    filter(&mut state, FilterMsg::Dismissed(COMPONENT));
    assert!(!form(&state).jira_filters.field(COMPONENT).open);
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::Picked(COMPONENT, 1));
    assert_eq!(
        state.worktree_form.issue_request_seq, before,
        "a pick waits for editing"
    );
}

// ---- Feature 1186 FR-002, FR-003: advanced mode hides the filters; Basic restores them ----

#[test]
fn advanced_mode_neither_applies_nor_drops_the_chosen_filters() {
    let mut state = filtered();
    let chosen = form(&state).jira_filters.chosen.clone();
    let line = form(&state).jira_filters.active_line();
    assert!(line.is_some(), "Payments is chosen");
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    filter(&mut state, FilterMsg::QueryChanged(COMPONENT, "Au".into()));
    send(
        &mut state,
        Msg::JiraQuery(QueryMsg::ModeChanged(QueryMode::Advanced)),
    );
    assert!(form(&state).shows_advanced());
    let field = form(&state).jira_filters.field(COMPONENT);
    assert!(!field.open, "a hidden filter's list closes");
    assert_eq!(field.query, "Payments", "its unpicked text gives way");
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::ClearedAll);
    filter(&mut state, FilterMsg::Cleared(COMPONENT));
    assert_eq!(
        form(&state).jira_filters.chosen,
        chosen,
        "a hidden filter is not cleared"
    );
    assert_eq!(
        state.worktree_form.issue_request_seq, before,
        "a hidden filter loads no basic list"
    );
    send(
        &mut state,
        Msg::JiraQuery(QueryMsg::DraftChanged("assignee = currentUser()".into())),
    );
    send(&mut state, Msg::JiraQuery(QueryMsg::Run));
    match &form(&state).jira_query.run {
        QueryRun::Running { scoped, .. } => assert!(
            !scoped.as_str().to_lowercase().contains("component"),
            "the run carries no basic filter: {}",
            scoped.as_str()
        ),
        other => panic!("expected a run in flight, found {other:?}"),
    }
    send(
        &mut state,
        Msg::JiraQuery(QueryMsg::ModeChanged(QueryMode::Basic)),
    );
    assert_eq!(form(&state).jira_filters.chosen, chosen);
    assert_eq!(form(&state).jira_filters.active_line(), line);
    assert_eq!(shown_keys(&state), ["ABC-1", "ABC-3"]);
}

// Feature 1185 M2: the fix-version filter (US1 AS3, AS5; FR-003; SC-001).

fn version(name: &str, released: bool) -> VersionChoice {
    VersionChoice::new(name, released).expect("usable name")
}

/// What Jira's versions parse to for the fake project: 2.4 unreleased, 2.3 released, 1.0 archived
/// (dropped by `parse_versions`).
fn versions() -> Vec<VersionChoice> {
    vec![version("2.4", false), version("2.3", true)]
}

fn version_seq(state: &State) -> u64 {
    match &form(state).jira_filters.versions {
        ChoiceLoad::Loading { seq } => *seq,
        other => panic!("expected versions in flight, found {other:?}"),
    }
}

/// Focus Fix version and answer its request with `result`.
fn focus_versions(state: &mut State, result: Result<Vec<VersionChoice>, IssueLoadError>) {
    filter(state, FilterMsg::Focused(FIX_VERSION));
    let seq = version_seq(state);
    filter(state, FilterMsg::VersionsLoaded { seq, result });
}

fn labels_of(state: &State, kind: FilterKind) -> Vec<String> {
    form(state)
        .jira_filters
        .rows(kind)
        .iter()
        .map(|row| row.label(kind, Some(source())))
        .collect()
}

fn pick_in(state: &mut State, kind: FilterKind, label: &str) {
    let labels = labels_of(state, kind);
    let index = labels
        .iter()
        .position(|l| l == label)
        .unwrap_or_else(|| panic!("no row {label:?} in {labels:?}"));
    filter(state, FilterMsg::Picked(kind, index));
}

/// Payments chosen and answered, then fix version 2.4 chosen and answered.
fn both() -> State {
    let mut state = filtered();
    focus_versions(&mut state, Ok(versions()));
    pick_in(&mut state, FIX_VERSION, "2.4");
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Ok(listing(vec![issue("ABC-3", 3, "Refund button", &[])], true)),
    );
    state
}

#[test]
fn focusing_fix_version_asks_once_and_offers_unreleased_first_without_archived() {
    let mut state = loaded();
    assert_eq!(form(&state).jira_filters.versions, ChoiceLoad::NotRequested);
    let before = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::Focused(FIX_VERSION));
    assert!(form(&state).jira_filters.field(FIX_VERSION).open);
    assert_eq!(version_seq(&state), before + 1);
    assert_eq!(
        *components(&state),
        ChoiceLoad::NotRequested,
        "only the focused filter asks"
    );
    assert_eq!(
        labels_of(&state, FIX_VERSION),
        ["Any fix version", "Loading versions…"]
    );
    let seq = version_seq(&state);
    filter(
        &mut state,
        FilterMsg::VersionsLoaded {
            seq,
            result: Ok(versions()),
        },
    );
    assert_eq!(
        labels_of(&state, FIX_VERSION),
        ["Any fix version", "2.4", "2.3"]
    );
    filter(&mut state, FilterMsg::Dismissed(FIX_VERSION));
    filter(&mut state, FilterMsg::Focused(FIX_VERSION));
    assert_eq!(
        state.worktree_form.filter_request_seq,
        before + 1,
        "a second focus asks nothing"
    );
}

#[test]
fn a_versions_answer_for_another_seq_or_to_the_components_is_dropped() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::Focused(FIX_VERSION));
    let seq = version_seq(&state);
    filter(
        &mut state,
        FilterMsg::VersionsLoaded {
            seq: seq + 7,
            result: Ok(versions()),
        },
    );
    assert_eq!(version_seq(&state), seq);
    filter(
        &mut state,
        FilterMsg::ComponentsLoaded {
            seq,
            result: Ok(names()),
        },
    );
    assert_eq!(
        version_seq(&state),
        seq,
        "a components answer fills nothing here"
    );
    assert_eq!(*components(&state), ChoiceLoad::NotRequested);
}

#[test]
fn a_component_and_a_fix_version_load_with_both_clauses_anded() {
    let mut state = filtered();
    focus_versions(&mut state, Ok(versions()));
    let before = state.worktree_form.issue_request_seq;
    pick_in(&mut state, FIX_VERSION, "2.4");
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#, r#"fixVersion = "2.4""#]
    );
    assert_eq!(form(&state).jira_filters.field(FIX_VERSION).query, "2.4");
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by component Payments, fix version 2.4")
    );
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Ok(listing(vec![issue("ABC-3", 3, "Refund button", &[])], true)),
    );
    assert_eq!(shown_keys(&state), ["ABC-3"]);
}

#[test]
fn clearing_one_filter_keeps_the_other() {
    let mut state = both();
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, FilterMsg::Cleared(COMPONENT));
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"fixVersion = "2.4""#]
    );
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by fix version 2.4")
    );

    let mut state = both();
    filter(&mut state, FilterMsg::Focused(FIX_VERSION));
    pick_in(&mut state, FIX_VERSION, "Any fix version");
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#]
    );
    assert_eq!(form(&state).jira_filters.field(FIX_VERSION).query, "");

    let mut state = both();
    filter(&mut state, FilterMsg::ClearedAll);
    assert!(state.worktree_form.jira_filters().is_empty());
    assert_eq!(form(&state).jira_filters.field(COMPONENT).query, "");
    assert_eq!(form(&state).jira_filters.field(FIX_VERSION).query, "");
}

#[test]
fn a_versions_failure_shows_its_cause_and_retry_on_that_filter_only() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    focus_versions(&mut state, Err(IssueLoadError::TimedOut));
    assert_eq!(
        labels_of(&state, FIX_VERSION),
        [
            "Any fix version".to_string(),
            IssueLoadError::TimedOut.message(source()),
            "Retry".to_string()
        ]
    );
    assert_eq!(
        labels_of(&state, COMPONENT),
        ["Any component", "Payments", "Auth", "Ä \"quoted\" \\ name"],
        "the component filter keeps working"
    );
    pick(&mut state, "Payments");
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#]
    );
    let before = state.worktree_form.filter_request_seq;
    pick_in(&mut state, FIX_VERSION, "Retry");
    assert_eq!(version_seq(&state), before + 1);
    assert_eq!(*components(&state), ChoiceLoad::Loaded(names()));
    let seq = version_seq(&state);
    filter(
        &mut state,
        FilterMsg::VersionsLoaded {
            seq,
            result: Ok(versions()),
        },
    );
    assert_eq!(labels_of(&state, FIX_VERSION).len(), 3);

    let mut state = loaded();
    focus_versions(&mut state, Err(IssueLoadError::NoAccess));
    let before = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::ChoicesRetry(FIX_VERSION));
    assert_eq!(
        version_seq(&state),
        before + 1,
        "the Retry message asks too"
    );
}

#[test]
fn a_project_without_versions_says_so() {
    let mut state = loaded();
    focus_versions(&mut state, Ok(vec![]));
    assert_eq!(
        labels_of(&state, FIX_VERSION),
        ["Any fix version", "This project has no versions"]
    );
}

#[test]
fn typing_in_fix_version_narrows_its_choices() {
    let mut state = loaded();
    focus_versions(&mut state, Ok(versions()));
    filter(
        &mut state,
        FilterMsg::QueryChanged(FIX_VERSION, "2.3".into()),
    );
    assert_eq!(labels_of(&state, FIX_VERSION), ["Any fix version", "2.3"]);
}

/// SC-001 (as amended by ledger D15): from the dialog opened on Jira, five interactions (open
/// Filters, focus Component, pick, focus Fix version, pick) reach the listing of one component and
/// one fix version, with no typing; the panel stays open between the picks. The shell's answers are
/// fed between them; they are not interactions.
#[test]
fn five_interactions_reach_the_two_clause_listing() {
    let mut state = loaded();
    let mut interactions = 0;
    let mut act = |state: &mut State, msg: FilterMsg| {
        interactions += 1;
        filter(state, msg);
    };
    act(&mut state, FilterMsg::PanelToggled);
    act(&mut state, FilterMsg::Focused(COMPONENT));
    let seq = choice_seq(&state);
    filter(
        &mut state,
        FilterMsg::ComponentsLoaded {
            seq,
            result: Ok(names()),
        },
    );
    act(&mut state, FilterMsg::Picked(COMPONENT, 1));
    act(&mut state, FilterMsg::Focused(FIX_VERSION));
    let seq = version_seq(&state);
    filter(
        &mut state,
        FilterMsg::VersionsLoaded {
            seq,
            result: Ok(versions()),
        },
    );
    act(&mut state, FilterMsg::Picked(FIX_VERSION, 1));
    assert_eq!(interactions, 5);
    assert!(form(&state).jira_filters.panel_open, "the panel stays open");
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"component = "Payments""#, r#"fixVersion = "2.4""#]
    );
    assert!(matches!(form(&state).issues, IssueList::Loading { .. }));
    assert_eq!(form(&state).issue_query, "", "nothing typed");
}

#[test]
fn a_focus_while_another_filters_choices_are_out_awaits_its_own() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    let components_seq = choice_seq(&state);
    filter(&mut state, FilterMsg::Focused(FIX_VERSION));
    let versions_seq = version_seq(&state);
    assert!(versions_seq > components_seq);
    assert_eq!(
        state.worktree_form.awaited_choices(),
        Some((FIX_VERSION, versions_seq)),
        "the shell starts the request just put in flight"
    );
    filter(
        &mut state,
        FilterMsg::VersionsLoaded {
            seq: versions_seq,
            result: Ok(versions()),
        },
    );
    filter(
        &mut state,
        FilterMsg::ComponentsLoaded {
            seq: components_seq,
            result: Ok(names()),
        },
    );
    assert_eq!(labels_of(&state, FIX_VERSION).len(), 3);
    assert_eq!(labels_of(&state, COMPONENT).len(), 4);
}

// Feature 1185 M3: the Filters panel (ledger D15) and the reporter filter (US2; FR-004, FR-006).

const REPORTER: FilterKind = FilterKind::Reporter;

fn person(account: &str, name: &str) -> JiraUser {
    JiraUser {
        account: AccountId::parse(account).expect("an account id"),
        name: name.to_string(),
        is_me: false,
    }
}

fn jane() -> JiraUser {
    JiraUser {
        is_me: true,
        ..person("me-1", "Jane Doe")
    }
}

/// The own user's request in flight.
fn me_seq(state: &State) -> u64 {
    match &form(state).jira_filters.me {
        ChoiceLoad::Loading { seq } => *seq,
        other => panic!("expected the own user in flight, found {other:?}"),
    }
}

/// Focus Reporter and answer `/myself` with `result`.
fn focus_reporter(state: &mut State, result: Result<Option<JiraUser>, IssueLoadError>) {
    filter(state, FilterMsg::Focused(REPORTER));
    let seq = me_seq(state);
    filter(state, FilterMsg::MeLoaded { seq, result });
}

/// Type `text` into Reporter, let the debounce end, and answer the search with `result`.
fn search_people(state: &mut State, text: &str, result: Result<Vec<JiraUser>, IssueLoadError>) {
    filter(state, FilterMsg::QueryChanged(REPORTER, text.into()));
    let seq = state
        .worktree_form
        .pending_people_search()
        .expect("the search waits out its debounce");
    filter(state, FilterMsg::PeopleSearchDue { seq });
    assert_eq!(
        state.worktree_form.awaited_people_search(),
        Some((seq, text.trim().to_string()))
    );
    filter(state, FilterMsg::PeopleLoaded { seq, result });
}

#[test]
fn the_filters_button_opens_and_closes_the_panel_and_a_close_closes_its_lists() {
    let mut state = loaded();
    assert!(!form(&state).jira_filters.panel_open, "closed at first");
    filter(&mut state, FilterMsg::PanelToggled);
    assert!(form(&state).jira_filters.panel_open);
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    filter(&mut state, FilterMsg::QueryChanged(COMPONENT, "Pa".into()));
    filter(&mut state, FilterMsg::PanelDismissed);
    let filters = &form(&state).jira_filters;
    assert!(!filters.panel_open);
    assert!(!filters.field(COMPONENT).open, "its open list closes too");
    assert_eq!(
        filters.field(COMPONENT).query,
        "",
        "the unpicked text gives way"
    );
    filter(&mut state, FilterMsg::PanelToggled);
    filter(&mut state, FilterMsg::PanelToggled);
    assert!(
        !form(&state).jira_filters.panel_open,
        "a second press closes it"
    );
}

#[test]
fn the_panel_does_not_open_without_the_jira_source_or_in_advanced_mode() {
    let mut state = jira_form();
    filter(&mut state, FilterMsg::PanelToggled);
    assert!(
        !form(&state).jira_filters.panel_open,
        "no listing asked for, no filters"
    );
    let mut state = filtered();
    filter(&mut state, FilterMsg::PanelToggled);
    filter(&mut state, FilterMsg::Focused(COMPONENT));
    send(
        &mut state,
        Msg::JiraQuery(QueryMsg::ModeChanged(QueryMode::Advanced)),
    );
    let filters = &form(&state).jira_filters;
    assert!(
        !filters.panel_open,
        "the switch to Advanced closes the panel"
    );
    assert!(!filters.field(COMPONENT).open);
    filter(&mut state, FilterMsg::PanelToggled);
    assert!(
        !form(&state).jira_filters.panel_open,
        "advanced mode refuses to open it"
    );
}

#[test]
fn a_reload_under_the_open_panel_leaves_the_issue_list_shut() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::PanelToggled);
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(payments_issues(), true)));
    assert!(form(&state).jira_filters.panel_open);
    assert!(
        !form(&state).issue_list_open,
        "the issue list would open under the panel"
    );
}

/// US2 AS1: the first focus of Reporter asks for the own user once; "Any reporter", then "You
/// (name)" first.
#[test]
fn focusing_reporter_asks_for_the_own_user_once_and_offers_you_first() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::Focused(REPORTER));
    let seq = me_seq(&state);
    assert_eq!(state.worktree_form.awaited_choices(), Some((REPORTER, seq)));
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "Loading people…"]
    );
    filter(
        &mut state,
        FilterMsg::MeLoaded {
            seq,
            result: Ok(Some(jane())),
        },
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)"]
    );
    filter(&mut state, FilterMsg::Dismissed(REPORTER));
    filter(&mut state, FilterMsg::Focused(REPORTER));
    assert_eq!(state.worktree_form.awaited_choices(), None, "asked once");
    assert_eq!(
        state.worktree_form.pending_people_search(),
        None,
        "nothing typed, nobody searched"
    );
}

#[test]
fn typing_waits_for_the_debounce_and_a_newer_text_discards_an_older_result() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    filter(&mut state, FilterMsg::QueryChanged(REPORTER, "sa".into()));
    let first = state
        .worktree_form
        .pending_people_search()
        .expect("pending");
    assert_eq!(
        state.worktree_form.awaited_people_search(),
        None,
        "nothing is sent before the debounce ends"
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)", "Searching people…"]
    );
    filter(&mut state, FilterMsg::PeopleSearchDue { seq: first });
    assert_eq!(
        state.worktree_form.awaited_people_search(),
        Some((first, "sa".to_string()))
    );
    filter(&mut state, FilterMsg::QueryChanged(REPORTER, "sam".into()));
    let second = state
        .worktree_form
        .pending_people_search()
        .expect("pending");
    assert!(second > first);
    filter(
        &mut state,
        FilterMsg::PeopleLoaded {
            seq: first,
            result: Ok(vec![person("a9", "Sasha Old")]),
        },
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)", "Searching people…"],
        "an answer for text typed over is dropped"
    );
    filter(&mut state, FilterMsg::PeopleSearchDue { seq: first });
    assert_eq!(
        state.worktree_form.awaited_people_search(),
        None,
        "a stale due starts nothing"
    );
    filter(&mut state, FilterMsg::PeopleSearchDue { seq: second });
    filter(
        &mut state,
        FilterMsg::PeopleLoaded {
            seq: first,
            result: Ok(vec![person("a9", "Sasha Old")]),
        },
    );
    filter(
        &mut state,
        FilterMsg::PeopleLoaded {
            seq: second,
            result: Ok(vec![person("a2", "Sam Lee"), jane()]),
        },
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)", "Sam Lee"],
        "the older answer is dropped and the own user is offered once"
    );
}

/// US2 AS2: picking a person loads the list by their account.
#[test]
fn picking_a_person_loads_with_their_account() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    search_people(&mut state, "sam", Ok(vec![person("a2", "Sam Lee")]));
    let before = state.worktree_form.issue_request_seq;
    pick_in(&mut state, REPORTER, "Sam Lee");
    assert!(state.worktree_form.issue_request_seq > before);
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"reporter = "a2""#]
    );
    let filters = &form(&state).jira_filters;
    assert_eq!(filters.field(REPORTER).query, "Sam Lee");
    assert_eq!(
        filters.active_line().as_deref(),
        Some("Filtered by reporter Sam Lee")
    );
    let reported: Vec<Issue> = payments_issues()
        .into_iter()
        .map(|issue| issue.reported_by("Sam Lee"))
        .collect();
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(reported, true)));
    assert_eq!(shown_keys(&state), ["ABC-1", "ABC-3"]);
    pick_in(&mut state, REPORTER, "You (Jane Doe)");
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"reporter = "me-1""#]
    );
}

/// US2 AS3: two people sharing a display name stay two choices; only the picked one's account is
/// sent.
#[test]
fn two_people_sharing_a_name_stay_distinct() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    search_people(
        &mut state,
        "alex",
        Ok(vec![person("a1", "Alex Kim"), person("a5", "Alex Kim")]),
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)", "Alex Kim", "Alex Kim"]
    );
    filter(&mut state, FilterMsg::Picked(REPORTER, 3));
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"reporter = "a5""#]
    );
    assert_eq!(
        form(&state).jira_filters.selected_row(REPORTER),
        Some(3),
        "the picked person, not the first of that name, is marked"
    );
}

#[test]
fn a_search_that_finds_no_one_still_offers_you() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    search_people(&mut state, "zz", Ok(vec![jane()]));
    assert_eq!(
        labels_of(&state, REPORTER),
        [
            "Any reporter",
            "You (Jane Doe)",
            "No project user matches; you are still offered"
        ]
    );
}

#[test]
fn a_myself_failure_shows_its_cause_and_retry_on_reporter_only() {
    let mut state = loaded();
    focus_and_answer(&mut state, Ok(names()));
    focus_reporter(&mut state, Err(IssueLoadError::TimedOut));
    let labels = labels_of(&state, REPORTER);
    assert_eq!(labels.len(), 3, "{labels:?}");
    assert_eq!(labels[0], "Any reporter");
    assert_eq!(labels[2], "Retry");
    assert_eq!(row_labels(&state).len(), 4, "the components are untouched");
    let before = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::ChoicesRetry(REPORTER));
    let seq = me_seq(&state);
    assert!(seq > before);
    assert_eq!(state.worktree_form.awaited_choices(), Some((REPORTER, seq)));
}

#[test]
fn a_people_search_failure_keeps_the_own_user_offered() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    search_people(&mut state, "sam", Err(IssueLoadError::TimedOut));
    let labels = labels_of(&state, REPORTER);
    assert_eq!(labels[..2], ["Any reporter", "You (Jane Doe)"]);
    assert_eq!(labels.last().map(String::as_str), Some("Retry"));
    let retry = labels.len() - 1;
    filter(&mut state, FilterMsg::Picked(REPORTER, retry));
    let (seq, text) = state
        .worktree_form
        .awaited_people_search()
        .expect("Retry searches again");
    assert_eq!(text, "sam");
    assert_eq!(
        state.worktree_form.awaited_choices(),
        None,
        "not the own user"
    );
    filter(
        &mut state,
        FilterMsg::PeopleLoaded {
            seq,
            result: Ok(vec![person("a2", "Sam Lee")]),
        },
    );
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)", "Sam Lee"]
    );
}

#[test]
fn clearing_the_typed_text_forgets_the_search() {
    let mut state = loaded();
    focus_reporter(&mut state, Ok(Some(jane())));
    filter(&mut state, FilterMsg::QueryChanged(REPORTER, "sa".into()));
    filter(&mut state, FilterMsg::QueryChanged(REPORTER, "  ".into()));
    assert_eq!(state.worktree_form.pending_people_search(), None);
    assert_eq!(form(&state).jira_filters.people, ChoiceLoad::NotRequested);
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "You (Jane Doe)"]
    );
}

#[test]
fn no_search_runs_before_the_own_user_loads_and_no_match_without_you_says_only_that() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::Focused(REPORTER));
    filter(&mut state, FilterMsg::QueryChanged(REPORTER, "sam".into()));
    assert_eq!(
        state.worktree_form.pending_people_search(),
        None,
        "the list shows the own user's load, not people"
    );
    let seq = me_seq(&state);
    filter(
        &mut state,
        FilterMsg::MeLoaded {
            seq,
            result: Ok(None),
        },
    );
    search_people(&mut state, "zz", Ok(Vec::new()));
    assert_eq!(
        labels_of(&state, REPORTER),
        ["Any reporter", "No reporter matches"],
        "nobody is offered, so the row does not say you are"
    );
}

// Feature 1185 M4: the label filter (US3; FR-001, FR-005; research R5).

const LABEL: FilterKind = FilterKind::Label;

fn labelled_issues() -> Vec<Issue> {
    vec![
        issue("ABC-5", 5, "Login slow", &["backend", "Perf"]),
        issue("ABC-6", 6, "Session drops", &["backend", "login-flow"]),
        issue("ABC-7", 7, "Crash on start", &["backend"]),
        issue("ABC-8", 8, "Button colour", &["ui", "alpha"]),
    ]
}

/// The Jira source showing `issues` as its unfiltered listing.
fn loaded_with(issues: Vec<Issue>, complete: bool) -> State {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(issues, complete)));
    state
}

#[test]
fn the_panel_offers_all_four_filters_in_order() {
    assert_eq!(
        OFFERED,
        [FilterKind::Reporter, LABEL, COMPONENT, FIX_VERSION]
    );
}

#[test]
fn the_label_filter_offers_the_loaded_labels_alphabetically_without_a_request() {
    let mut state = loaded_with(labelled_issues(), true);
    let asked = state.worktree_form.filter_request_seq;
    filter(&mut state, FilterMsg::PanelToggled);
    filter(&mut state, FilterMsg::Focused(LABEL));
    assert_eq!(
        state.worktree_form.filter_request_seq, asked,
        "the labels ask Jira nothing"
    );
    assert_eq!(state.worktree_form.awaited_choices(), None);
    assert!(form(&state).jira_filters.field(LABEL).open);
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "alpha", "backend", "login-flow", "Perf", "ui"]
    );
}

#[test]
fn the_label_filter_says_why_it_has_no_choices() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "Labels load with the issues"]
    );
    let seq = awaited(&state);
    answer(&mut state, seq, Err(IssueLoadError::TimedOut));
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "Labels need the issue list: retry it above"]
    );
    send(&mut state, Msg::IssueRetry);
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "Labels load with the issues"],
        "the list's Retry brings the labels back to loading"
    );
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    assert_eq!(labels_of(&state, LABEL), ["Any label", "payments"]);

    let state = loaded_with(vec![issue("ABC-2", 2, "Login slow", &[])], true);
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "No labels on the loaded issues"]
    );
}

#[test]
fn a_capped_listing_adds_the_most_recent_issues_notice() {
    let state = loaded_with(labelled_issues(), false);
    let rows = form(&state).jira_filters.rows(LABEL);
    let last = rows.last().expect("rows");
    assert_eq!(
        last.label(LABEL, Some(source())),
        "Only labels of the 1,000 most recent issues"
    );
    assert!(!last.enabled(), "the notice cannot be picked");
}

#[test]
fn choosing_a_label_and_typing_loads_with_it_and_matches_key_summary_or_label() {
    let mut state = loaded_with(labelled_issues(), true);
    filter(&mut state, FilterMsg::Focused(LABEL));
    pick_in(&mut state, LABEL, "backend");
    assert_eq!(
        state.worktree_form.jira_filters().clauses(),
        [r#"labels = "backend""#]
    );
    assert_eq!(
        form(&state).jira_filters.active_line().as_deref(),
        Some("Filtered by label backend")
    );
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Ok(listing(labelled_issues()[..3].to_vec(), true)),
    );
    send(&mut state, Msg::IssueQueryChanged("login".into()));
    assert_eq!(shown_keys(&state), ["ABC-5", "ABC-6"]);
}

#[test]
fn a_filtered_load_never_replaces_the_label_choices() {
    let mut state = loaded_with(labelled_issues(), true);
    filter(&mut state, FilterMsg::Focused(LABEL));
    pick_in(&mut state, LABEL, "ui");
    let seq = awaited(&state);
    answer(
        &mut state,
        seq,
        Ok(listing(
            vec![issue("ABC-8", 8, "Button colour", &["ui"])],
            true,
        )),
    );
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "alpha", "backend", "login-flow", "Perf", "ui"],
        "choosing a filter does not shrink the labels"
    );
    filter(&mut state, FilterMsg::ClearedAll);
    let seq = awaited(&state);
    answer(&mut state, seq, Ok(listing(all_issues(), true)));
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "alpha", "backend", "login-flow", "Perf", "ui"],
        "the first unfiltered listing's labels are kept"
    );
}

#[test]
fn a_filter_chosen_before_the_unfiltered_load_returns_still_gets_its_labels() {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let unfiltered = awaited(&state);
    focus_and_answer(&mut state, Ok(names()));
    pick(&mut state, "Payments");
    let filtered_seq = awaited(&state);
    assert_eq!(
        form(&state).jira_filters.labels,
        LabelSource::Waiting { seq: unfiltered }
    );
    answer(&mut state, unfiltered, Ok(listing(labelled_issues(), true)));
    assert_eq!(
        awaited(&state),
        filtered_seq,
        "the superseded listing is not shown"
    );
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "alpha", "backend", "login-flow", "Perf", "ui"]
    );
    answer(
        &mut state,
        filtered_seq,
        Ok(listing(payments_issues(), true)),
    );
    assert_eq!(shown_keys(&state), ["ABC-1", "ABC-3"]);
    assert_eq!(
        labels_of(&state, LABEL),
        ["Any label", "alpha", "backend", "login-flow", "Perf", "ui"]
    );
}

// Feature 1185 M5 (FR-006, FR-015; quickstart Part B step 9): a field reached by the keyboard opens
// without the press that sends `Focused`. Its first opening by any message asks for its choices.

/// The seq `kind`'s choices are awaited under, while in flight.
fn awaited_choice(state: &State, kind: FilterKind) -> Option<u64> {
    form(state)
        .jira_filters
        .awaited_choices()
        .filter(|(awaited, _)| *awaited == kind)
        .map(|(_, seq)| seq)
}

/// Answer `kind`'s choices awaited under `seq` with its fake project's choices.
fn answer_choices(state: &mut State, kind: FilterKind, seq: u64) {
    let msg = match kind {
        FilterKind::Component => FilterMsg::ComponentsLoaded {
            seq,
            result: Ok(names()),
        },
        FilterKind::FixVersion => FilterMsg::VersionsLoaded {
            seq,
            result: Ok(versions()),
        },
        FilterKind::Reporter => FilterMsg::MeLoaded {
            seq,
            result: Ok(Some(jane())),
        },
        FilterKind::Label => unreachable!("labels are not asked of Jira"),
    };
    filter(state, msg);
}

/// What `kind` holds chosen.
fn chosen_value(state: &State, kind: FilterKind) -> Option<String> {
    let chosen = state.worktree_form.jira_filters();
    match kind {
        FilterKind::Component => chosen.component().map(str::to_owned),
        FilterKind::FixVersion => chosen.fix_version().map(str::to_owned),
        FilterKind::Reporter => chosen.reporter().map(|user| user.name.clone()),
        FilterKind::Label => None,
    }
}

/// Open `kind` from the keyboard with `first` (no `Focused`), check it asks once, answer it, then
/// pick the first choice with Down, Down, Enter.
fn opened_by_keyboard_asks_once_and_picks(kind: FilterKind, first: FilterMsg, again: FilterMsg) {
    let mut state = loaded();
    let before = state.worktree_form.filter_request_seq;
    assert_eq!(
        awaited_choice(&state, kind),
        None,
        "{kind:?}: nothing asked yet"
    );
    filter(&mut state, first);
    assert!(
        form(&state).jira_filters.field(kind).open,
        "{kind:?}: the list opens"
    );
    assert_eq!(
        awaited_choice(&state, kind),
        Some(before + 1),
        "{kind:?}: its first opening asks for its choices (FR-006)"
    );
    assert_eq!(state.worktree_form.filter_request_seq, before + 1);
    filter(&mut state, again);
    assert_eq!(
        state.worktree_form.filter_request_seq,
        before + 1,
        "{kind:?}: a second keystroke asks nothing more"
    );
    answer_choices(&mut state, kind, before + 1);
    let labels = labels_of(&state, kind);
    assert!(labels.len() > 1, "{kind:?}: the choices show, {labels:?}");
    let field = |state: &State| form(state).jira_filters.field(kind).clone();
    for _ in 0..3 {
        if field(&state).highlight == Some(1) {
            break;
        }
        filter(&mut state, FilterMsg::HighlightMoved(kind, Direction::Next));
    }
    assert_eq!(
        field(&state).highlight,
        Some(1),
        "{kind:?}: Down reaches the first choice"
    );
    filter(&mut state, FilterMsg::Picked(kind, 1));
    let expected = labels[1].clone();
    let chosen = chosen_value(&state, kind).expect("Down and Enter picked a choice");
    assert!(
        expected.contains(&chosen),
        "{kind:?}: picked {chosen:?}, row read {expected:?}"
    );
}

#[test]
fn a_field_typed_into_without_focus_asks_for_its_choices_once_and_picks_by_keyboard() {
    for kind in [COMPONENT, FIX_VERSION, REPORTER] {
        opened_by_keyboard_asks_once_and_picks(
            kind,
            FilterMsg::QueryChanged(kind, String::new()),
            FilterMsg::QueryChanged(kind, String::new()),
        );
    }
}

#[test]
fn a_field_moved_in_without_focus_asks_for_its_choices_once_and_picks_by_keyboard() {
    for kind in [COMPONENT, FIX_VERSION, REPORTER] {
        opened_by_keyboard_asks_once_and_picks(
            kind,
            FilterMsg::HighlightMoved(kind, Direction::Next),
            FilterMsg::HighlightMoved(kind, Direction::Next),
        );
    }
}

#[test]
fn typing_into_a_component_field_before_its_choices_arrive_narrows_them_when_they_do() {
    let mut state = loaded();
    filter(&mut state, FilterMsg::QueryChanged(COMPONENT, "Pa".into()));
    filter(&mut state, FilterMsg::QueryChanged(COMPONENT, "Pay".into()));
    let seq = choice_seq(&state);
    answer_choices(&mut state, COMPONENT, seq);
    assert_eq!(row_labels(&state), ["Any component", "Payments"]);
}

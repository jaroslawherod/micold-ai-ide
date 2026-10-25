//! The Jira filters of the issue picker, as state transitions (feature 1185, contracts/filter-ui.md,
//! data-model §6–§8). Everything the filters do is decided by the reducer, so it is asserted here
//! through `State` messages with no network: the choices and listings Jira would send are fed in
//! as the messages the shell answers with. What the shell asks Jira for, and when, is driven over
//! `FakeJiraSource` in `main_tests.rs` (`jira_filter_shell`).

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::jira_filters::{shows_filters, ChoiceLoad, FilterMsg};
use micold_client::features::worktree_form::{
    BranchSource, IssueList, JiraAvailability, Msg, SearchState, WorktreeForm,
};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing, IssueLoadError, NoticeSource};
use micold_core::issue_types::default_mapping;
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
use micold_core::jira_filter::{FilterKind, VersionChoice};
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

/// SC-001: from the dialog opened on Jira, four interactions (focus Component, pick, focus Fix
/// version, pick) reach the listing of one component and one fix version, with no typing. The
/// shell's answers are fed between them; they are not interactions.
#[test]
fn four_interactions_reach_the_two_clause_listing() {
    let mut state = loaded();
    let mut interactions = 0;
    let mut act = |state: &mut State, msg: FilterMsg| {
        interactions += 1;
        filter(state, msg);
    };
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
    assert_eq!(interactions, 4);
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

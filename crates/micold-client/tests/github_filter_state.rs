//! The GitHub filters of the issue picker, as state transitions (feature 1400, US1, US2,
//! data-model §10, contracts/selector-ui.md, contracts/source-and-storage.md §3).
//!
//! The reducer decides everything; the shell only runs what the form newly awaits. So these tests
//! drive `State` with the messages the shell would answer with, and stand in for the shell with
//! [`serve`]: when the form awaits a filtered listing, a filtered search or the label choices, it
//! runs the same core function the shell runs (`load_filtered`, `search_filtered`,
//! `load_label_choices`) against a `FakeIssueSource`, whose `asked_searches()` is then what a
//! filtered request sent to GitHub.

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::github_filter::{
    LabelAction, LabelList, LabelRow, LabelText, Msg as GithubMsg,
};
use micold_client::features::worktree_form::{
    BranchSource, IssueList, Msg, SearchState, WorktreeForm,
};
use micold_client::features::Outcome;
use micold_core::git::GitRemote;
use micold_core::github::{FakeIssueSource, Issue, IssueLoadError, IssuePage, NamePage};
use micold_core::github_filter::{
    load_filtered, load_label_choices, BasicFilters, FilterFailure, IssueState, LabelName,
    QueryRefusal, RememberedSelector, SelectorChange, NOT_ACCEPTED,
};
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
use micold_core::jira_query::QueryMode;
use micold_core::typeahead::Direction;

const EXCLUDE_IN_PROGRESS: &str =
    "repo:o/r is:issue is:open -label:\"in-progress\" sort:updated-desc";
const EXCLUDE_BOTH: &str =
    "repo:o/r is:issue is:open -label:\"in-progress\" -label:\"blocked\" sort:updated-desc";
const INCLUDE_BUG: &str = "repo:o/r is:issue is:open label:\"bug\" sort:updated-desc";
const INCLUDE_BUG_UI: &str =
    "repo:o/r is:issue is:open label:\"bug\" label:\"ui\" sort:updated-desc";

// --- fixtures -------------------------------------------------------------------------------

fn project() -> PathBuf {
    PathBuf::from("/work/acme")
}

fn gh() -> PathBuf {
    PathBuf::from("/usr/bin/gh")
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn filter(state: &mut State, msg: GithubMsg) {
    send(state, Msg::GithubFilter(msg));
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.form.as_ref().expect("the form is open")
}

fn label(name: &str) -> LabelName {
    LabelName::parse(name).expect("a quotable label")
}

fn issue(number: u64, title: &str, labels: &[&str]) -> Issue {
    Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-09-29T00:00:00Z".to_string(),
    )
}

fn page(issues: Vec<Issue>, total: u64, next: Option<&str>) -> IssuePage {
    IssuePage {
        issues,
        total_open: total,
        next_cursor: next.map(str::to_string),
    }
}

fn names(names: &[&str], next: Option<&str>) -> NamePage {
    NamePage {
        names: names.iter().map(|n| n.to_string()).collect(),
        next_cursor: next.map(str::to_string),
    }
}

/// A form whose repository is `o/r` on github.com, GitHub issues on.
fn available() -> State {
    let mut state = State::default();
    state.workspace.active = Some(project());
    send(&mut state, Msg::Opened);
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    state
}

/// The GitHub source showing 034's default listing of `issues`, complete.
fn shown(issues: Vec<Issue>) -> State {
    let mut state = available();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = state
        .worktree_form
        .awaited_issue_load()
        .expect("the source shown loads");
    let total = issues.len() as u64;
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues,
                    total_open: total,
                    complete: true,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    state
}

fn default_issues() -> Vec<Issue> {
    vec![
        issue(7, "Sidebar flickers", &["ui", "in-progress"]),
        issue(42, "Crash on open", &["bug"]),
        issue(108, "Blocked docs", &["blocked"]),
    ]
}

/// The label choices, answered under the awaited seq, from `fake`.
fn serve_labels(state: &mut State, fake: &FakeIssueSource) {
    let seq = state
        .worktree_form
        .awaited_label_choices()
        .expect("the label choices are awaited");
    let repo = form(state).github_repo().cloned().expect("a repository");
    let result = load_label_choices(fake, &repo);
    filter(state, GithubMsg::LabelsLoaded { seq, result });
}

/// What the shell does after a message: run the filtered listing the form awaits, if any.
/// `false` when nothing filtered was awaited.
fn serve(state: &mut State, fake: &FakeIssueSource) -> bool {
    let Some((seq, repo, search)) = state.worktree_form.github_filtered_load() else {
        return false;
    };
    let result = load_filtered(fake, &repo, &search).map(|listing| (listing, gh()));
    filter(
        state,
        GithubMsg::Loaded {
            seq,
            result,
            resolved_env: None,
        },
    );
    true
}

/// The GitHub source showing 034's default listing on a project also connected to Jira, so the
/// GitHub / Jira choice is offered.
fn both_providers() -> State {
    let mut both = State::default();
    both.workspace.active = Some(project());
    both.settings.jira_enabled = true;
    both.settings.jira_connections.insert(
        connection_key(&project()),
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
    );
    send(&mut both, Msg::Opened);
    send(
        &mut both,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    send(&mut both, Msg::SourceChanged(BranchSource::Issue));
    let seq = both.worktree_form.awaited_issue_load().unwrap();
    send(
        &mut both,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: default_issues(),
                    total_open: 3,
                    complete: true,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    both
}

/// The labels `in-progress`, `blocked`, `bug` and `ui` loaded into the selector, the panel open.
fn with_labels(state: &mut State) {
    filter(state, GithubMsg::PanelToggled);
    filter(state, GithubMsg::Focused(LabelList::Exclude));
    let fake = FakeIssueSource::new().with_labels(vec![Ok(names(
        &["ui", "in-progress", "bug", "blocked"],
        None,
    ))]);
    serve_labels(state, &fake);
}

/// Pick `name` from `list`'s rows.
fn pick(state: &mut State, list: LabelList, name: &str) {
    let rows = form(state).github_selector.rows(list);
    let index = rows
        .iter()
        .position(|row| row.action == LabelAction::Choice(label(name)))
        .unwrap_or_else(|| panic!("{name} is offered in {list:?}"));
    filter(state, GithubMsg::Picked(list, index));
}

/// The labels `list` offers, in order.
fn offered(state: &State, list: LabelList) -> Vec<String> {
    form(state)
        .github_selector
        .rows(list)
        .iter()
        .filter_map(|row| match &row.action {
            LabelAction::Choice(label) => Some(label.as_str().to_string()),
            _ => None,
        })
        .collect()
}

/// A shown form excluding `in-progress` and `blocked`, its filtered list answered with `issues`.
fn excluding_both(issues: Vec<Issue>, complete: bool) -> State {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Exclude, "in-progress");
    pick(&mut state, LabelList::Exclude, "blocked");
    if complete {
        let total = issues.len() as u64;
        let fake = FakeIssueSource::new().with_search_pages(vec![Ok(page(issues, total, None))]);
        assert!(serve(&mut state, &fake));
        assert_eq!(fake.asked_searches(), vec![EXCLUDE_BOTH.to_string()]);
    } else {
        // A capped list: more match than the load holds.
        let (seq, _, _) = state.worktree_form.github_filtered_load().unwrap();
        let total = issues.len() as u64 + 1_000;
        filter(
            &mut state,
            GithubMsg::Loaded {
                seq,
                result: Ok((
                    micold_core::github::IssueListing {
                        issues,
                        total_open: total,
                        complete: false,
                    },
                    gh(),
                )),
                resolved_env: None,
            },
        );
    }
    state
}

// --- T020: the filtered listing -------------------------------------------------------------

/// FR-009: the default filter sends 034's listing, no scoped search.
#[test]
fn the_default_filter_loads_034s_listing() {
    let mut state = available();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert!(state.worktree_form.awaited_issue_load().is_some());
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert_eq!(
        form(&state).issue_notice().as_deref(),
        Some("Reads open issues of o/r from GitHub.")
    );
}

/// US1 AS2, FR-008: each exclusion reloads with search-rules §4's text, puts the list to loading
/// and drops the earlier request's answer.
#[test]
fn excluding_labels_reloads_with_each_and_drops_the_earlier_answer() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Exclude, "in-progress");
    let (first, _, search) = state
        .worktree_form
        .github_filtered_load()
        .expect("a filtered listing is awaited");
    assert_eq!(search.as_str(), EXCLUDE_IN_PROGRESS);
    pick(&mut state, LabelList::Exclude, "blocked");
    let (second, _, search) = state
        .worktree_form
        .github_filtered_load()
        .expect("a filtered listing is awaited");
    assert!(second > first);
    assert_eq!(search.as_str(), EXCLUDE_BOTH);
    assert_eq!(form(&state).issues, IssueList::Loading { seq: second });
    // The first change's answer arrives late: dropped.
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq: first,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: default_issues(),
                    total_open: 3,
                    complete: true,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    assert_eq!(form(&state).issues, IssueList::Loading { seq: second });
    let fake = FakeIssueSource::new().with_search_pages(vec![Ok(page(
        vec![issue(42, "Crash on open", &["bug"])],
        1,
        None,
    ))]);
    assert!(serve(&mut state, &fake));
    assert_eq!(fake.asked_searches(), vec![EXCLUDE_BOTH.to_string()]);
    let held: Vec<u64> = form(&state)
        .issues
        .held()
        .iter()
        .map(|i| i.number())
        .collect();
    assert_eq!(held, vec![42]);
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Excluding labels in-progress, blocked")
    );
}

/// US1 AS3: include `bug`, then `ui`.
#[test]
fn including_labels_reloads_with_each() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Include, "bug");
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), INCLUDE_BUG);
    pick(&mut state, LabelList::Include, "ui");
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), INCLUDE_BUG_UI);
}

/// Edge "Same label included and excluded": each field offers every label not in its own list, and
/// choosing one the other list holds moves it there (quickstart B4).
#[test]
fn a_label_chosen_in_one_list_moves_when_chosen_in_the_other() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Exclude, "blocked");
    assert!(offered(&state, LabelList::Include).contains(&"blocked".to_string()));
    assert!(!offered(&state, LabelList::Exclude).contains(&"blocked".to_string()));
    let before = state.worktree_form.issue_request_seq;
    pick(&mut state, LabelList::Include, "blocked");
    let basic = &form(&state).github_selector.basic;
    assert_eq!(basic.include(), &[label("blocked")]);
    assert!(basic.exclude().is_empty());
    let (seq, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert!(seq > before, "the move reloads the list");
    assert_eq!(
        search.as_str(),
        "repo:o/r is:issue is:open label:\"blocked\" sort:updated-desc"
    );
    assert!(offered(&state, LabelList::Exclude).contains(&"blocked".to_string()));
    assert!(!offered(&state, LabelList::Include).contains(&"blocked".to_string()));
}

/// US1 AS5: removing one value keeps the others; Clear filters is 034's listing again.
#[test]
fn removing_a_value_and_clear_filters() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    filter(&mut state, GithubMsg::Removed(label("blocked")));
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), EXCLUDE_IN_PROGRESS);
    filter(&mut state, GithubMsg::ClearedAll);
    assert!(form(&state).github_selector.basic.is_default());
    assert!(state.worktree_form.awaited_issue_load().is_some());
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert_eq!(form(&state).github_selector.active_line(), None);
    // Clearing nothing reloads nothing.
    let seq = state.worktree_form.issue_request_seq;
    filter(&mut state, GithubMsg::ClearedAll);
    assert_eq!(state.worktree_form.issue_request_seq, seq);
}

/// Rapid changes end on the last combination's list, whatever order the answers come in.
#[test]
fn rapid_changes_end_on_the_last_combinations_list() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    let mut awaited = Vec::new();
    for name in ["bug", "ui"] {
        pick(&mut state, LabelList::Include, name);
        awaited.push(state.worktree_form.github_filtered_load().unwrap());
    }
    let answer = |seq: u64, number: u64| GithubMsg::Loaded {
        seq,
        result: Ok((
            micold_core::github::IssueListing {
                issues: vec![issue(number, "x", &["bug"])],
                total_open: 1,
                complete: true,
            },
            gh(),
        )),
        resolved_env: None,
    };
    filter(&mut state, answer(awaited[1].0, 2));
    filter(&mut state, answer(awaited[0].0, 1));
    let held: Vec<u64> = form(&state)
        .issues
        .held()
        .iter()
        .map(|i| i.number())
        .collect();
    assert_eq!(held, vec![2]);
}

/// Edge "Switching source to Jira and back": the filters stay, the search text is cleared and
/// the list reloads with them.
#[test]
fn leaving_the_source_and_back_keeps_the_filters_and_reloads() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    send(&mut state, Msg::SourceChanged(BranchSource::New));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(form(&state).issue_query, "");
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), EXCLUDE_BOTH);

    // The same through the GitHub / Jira choice.
    let mut both = both_providers();
    with_labels(&mut both);
    pick(&mut both, LabelList::Exclude, "blocked");
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_eq!(both.worktree_form.github_filtered_load(), None);
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Github));
    let (_, _, search) = both.worktree_form.github_filtered_load().unwrap();
    assert!(search.as_str().contains("-label:\"blocked\""));
}

/// FR-025: with GitHub issues off, no filter control and no request.
#[test]
fn github_issues_off_asks_nothing() {
    let mut state = State::default();
    state.workspace.active = Some(project());
    state.settings.github_issues_enabled = false;
    send(&mut state, Msg::Opened);
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    assert!(!form(&state).shows_github_filters());
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert_eq!(state.worktree_form.awaited_label_choices(), None);
    assert!(!form(&state).github_selector.panel_open);

    // Switched off while filtered: the list and its request go.
    let mut on = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    send(&mut on, Msg::GithubIssuesChanged(false));
    assert!(!form(&on).shows_github_filters());
    assert_eq!(on.worktree_form.github_filtered_load(), None);
}

/// Two windows on one project keep their own filters.
#[test]
fn two_apps_keep_their_own_filters() {
    let mut one = shown(default_issues());
    let two = shown(default_issues());
    with_labels(&mut one);
    pick(&mut one, LabelList::Exclude, "blocked");
    assert!(!form(&one).github_selector.basic.is_default());
    assert!(form(&two).github_selector.basic.is_default());
}

// --- T021: the label choices ----------------------------------------------------------------

/// FR-005: the first focus of a label field asks for the labels once; none before, none after.
#[test]
fn the_first_focus_of_a_label_field_loads_the_labels_once() {
    let mut state = shown(default_issues());
    assert_eq!(state.worktree_form.awaited_label_choices(), None);
    filter(&mut state, GithubMsg::PanelToggled);
    assert_eq!(state.worktree_form.awaited_label_choices(), None);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    let fake = FakeIssueSource::new().with_labels(vec![Ok(names(&["bug"], None))]);
    serve_labels(&mut state, &fake);
    assert_eq!(fake.label_calls().len(), 1);
    let seq = state.worktree_form.filter_request_seq;
    filter(&mut state, GithubMsg::Dismissed(LabelList::Include));
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    filter(&mut state, GithubMsg::Focused(LabelList::Exclude));
    assert_eq!(state.worktree_form.awaited_label_choices(), None);
    assert_eq!(state.worktree_form.filter_request_seq, seq);
}

/// Visual pass M2 round 2 F2, F3: after Down and Enter add a label, the field keeps the keyboard,
/// so it stays drawn open (its list shows the labels left) and its highlight starts over: the next
/// letter typed highlights nothing, and Enter adds nothing until Down picks a row again.
#[test]
fn a_picked_label_leaves_the_field_open_with_no_highlight() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    filter(
        &mut state,
        GithubMsg::HighlightMoved(LabelList::Include, Direction::Next),
    );
    let highlighted = form(&state)
        .github_selector
        .field(LabelList::Include)
        .highlight;
    assert_eq!(highlighted, Some(0));
    filter(&mut state, GithubMsg::Picked(LabelList::Include, 0));
    let field = form(&state).github_selector.field(LabelList::Include);
    assert_eq!(field.query, "");
    assert!(
        field.open,
        "the field holds the keyboard, so it is drawn open"
    );
    assert_eq!(field.highlight, None);
    filter(
        &mut state,
        GithubMsg::QueryChanged(LabelList::Include, "u".into()),
    );
    assert_eq!(
        form(&state)
            .github_selector
            .field(LabelList::Include)
            .highlight,
        None
    );
}

/// US1 AS1: the labels alphabetically, less those chosen in the field's own list.
#[test]
fn the_choices_are_alphabetical_less_the_chosen() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    assert_eq!(
        offered(&state, LabelList::Include),
        vec!["blocked", "bug", "in-progress", "ui"]
    );
    pick(&mut state, LabelList::Exclude, "bug");
    pick(&mut state, LabelList::Include, "ui");
    assert_eq!(
        offered(&state, LabelList::Include),
        vec!["blocked", "bug", "in-progress"]
    );
    assert_eq!(
        offered(&state, LabelList::Exclude),
        vec!["blocked", "in-progress", "ui"]
    );
    // Typed text narrows the field's list.
    filter(
        &mut state,
        GithubMsg::QueryChanged(LabelList::Exclude, "blo".into()),
    );
    assert_eq!(offered(&state, LabelList::Exclude), vec!["blocked"]);
}

fn label_texts(state: &State, list: LabelList) -> Vec<String> {
    let source = form(state).notice_source().expect("a source");
    form(state)
        .github_selector
        .rows(list)
        .iter()
        .map(|row| row.label(source))
        .collect()
}

#[test]
fn a_repository_without_labels_says_so() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    assert_eq!(
        label_texts(&state, LabelList::Include),
        vec!["Loading labels…"]
    );
    serve_labels(
        &mut state,
        &FakeIssueSource::new().with_labels(vec![Ok(names(&[], None))]),
    );
    assert_eq!(
        label_texts(&state, LabelList::Include),
        vec!["This repository has no labels"]
    );
    assert!(form(&state).github_selector.no_labels());
}

#[test]
fn a_partial_list_and_labels_not_offered_are_named() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    // Ten pages, the last still with a cursor: partial. Two names hold a quote.
    let mut pages = vec![Ok(names(&["a", "say \"hi\"", "b\""], Some("C1")))];
    for n in 2..=10 {
        pages.push(Ok(NamePage {
            names: vec![format!("l{n}")],
            next_cursor: Some(format!("C{n}")),
        }));
    }
    serve_labels(&mut state, &FakeIssueSource::new().with_labels(pages));
    let texts = label_texts(&state, LabelList::Include);
    assert!(texts.contains(&"Only the first 1,000 labels are offered".to_string()));
    assert!(
        texts.contains(&"2 labels can't be offered because their names hold a quote".to_string())
    );
}

/// Edge "Choices fail to load": 034's notice and Retry on the field; the list and search work.
#[test]
fn a_choice_failure_shows_retry_on_that_field_only() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::Focused(LabelList::Include));
    serve_labels(
        &mut state,
        &FakeIssueSource::new().with_labels(vec![Err(IssueLoadError::TimedOut)]),
    );
    let rows = form(&state).github_selector.rows(LabelList::Include);
    assert!(rows.iter().any(|row| row.action == LabelAction::Retry));
    let source = form(&state).notice_source().unwrap();
    assert!(rows
        .iter()
        .any(|row| row.label(source) == IssueLoadError::TimedOut.message(source)));
    // Visual pass M2 F5: a cause is named for the labels, not as "Couldn't read issues".
    let failed = |error: IssueLoadError| LabelRow {
        text: LabelText::Failed(error),
        action: LabelAction::Inert,
        spans: Vec::new(),
    };
    assert_eq!(
        failed(IssueLoadError::Other("HTTP 502: Bad Gateway".into())).label(source),
        "HTTP 502: Bad Gateway"
    );
    assert_eq!(
        failed(IssueLoadError::NotSignedIn).label(source),
        "You're not signed in to GitHub. Run `gh auth login` in a terminal, then retry."
    );
    // The list and the search are untouched.
    assert!(matches!(form(&state).issues, IssueList::Loaded { .. }));
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    assert_eq!(form(&state).issue_matches.len(), 1);
    // Retry asks again, once.
    let retry = rows
        .iter()
        .position(|row| row.action == LabelAction::Retry)
        .unwrap();
    filter(&mut state, GithubMsg::Picked(LabelList::Include, retry));
    assert!(state.worktree_form.awaited_label_choices().is_some());
    assert_eq!(state.worktree_form.github_filtered_load(), None);
}

// --- T022: the list with a filter -----------------------------------------------------------

/// US1 AS4: a complete filtered list narrows locally, asking nothing.
#[test]
fn typed_text_narrows_a_complete_filtered_list_locally() {
    let mut state = excluding_both(
        vec![
            issue(42, "Crash on open", &["bug"]),
            issue(9, "Slow start", &[]),
        ],
        true,
    );
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    assert_eq!(form(&state).issue_matches.len(), 1);
    assert_eq!(state.worktree_form.pending_issue_search(), None);
}

/// FR-016: an incomplete filtered list searches beyond it with the scoped text, after the pause.
#[test]
fn an_incomplete_filtered_list_searches_beyond_it_with_the_filter() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    // Only quotes typed: no word, so nothing is sent.
    send(&mut state, Msg::IssueQueryChanged("\"\"".into()));
    assert_eq!(state.worktree_form.pending_issue_search(), None);
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    let seq = state
        .worktree_form
        .pending_issue_search()
        .expect("the search waits out the debounce");
    assert_eq!(state.worktree_form.github_filtered_search(), None);
    send(&mut state, Msg::IssueSearchDue { seq });
    let (asked, repo, search, located) = state
        .worktree_form
        .github_filtered_search()
        .expect("the filtered search runs");
    assert_eq!(asked, seq);
    assert_eq!(located, gh());
    assert_eq!(
        search.as_str(),
        "repo:o/r is:issue is:open -label:\"in-progress\" -label:\"blocked\" \"crash\""
    );
    let fake =
        FakeIssueSource::new().with_search_filtered(Ok(vec![issue(77, "Crash in sync", &["bug"])]));
    let result = micold_core::github::IssueSource::search_filtered(&fake, &repo, &search);
    filter(&mut state, GithubMsg::Searched { seq, result });
    assert_eq!(fake.asked_searches(), vec![search.as_str().to_string()]);
    let held: Vec<u64> = form(&state)
        .issues
        .held()
        .iter()
        .map(|i| i.number())
        .collect();
    assert_eq!(held, vec![42, 77]);
    assert_eq!(
        state.worktree_form.issue_search_request(),
        None,
        "034's unfiltered search is never built while a filter is active"
    );
}

/// Review A M2 F1: an edit to text with no word to search for (quotes alone) drops the search a
/// keystroke left pending, so it never becomes a search that cannot start.
#[test]
fn an_edit_to_quotes_alone_drops_the_pending_search() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    send(&mut state, Msg::IssueQueryChanged("\"c".into()));
    let seq = state
        .worktree_form
        .pending_issue_search()
        .expect("a word typed: the search waits out the debounce");
    send(&mut state, Msg::IssueQueryChanged("\"".into()));
    assert_eq!(state.worktree_form.pending_issue_search(), None);
    send(&mut state, Msg::IssueSearchDue { seq });
    assert_eq!(
        state.worktree_form.awaited_issue_search(),
        None,
        "no search spins with nothing to send"
    );
    assert_eq!(form(&state).issue_search_status(), None);
}

/// Review A M2 F2: Clear filters with text typed reloads 034's listing; capped, it searches beyond
/// it again after the pause, as a filtered reload does.
#[test]
fn clear_filters_with_text_typed_searches_beyond_a_capped_default_list() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    filter(&mut state, GithubMsg::ClearedAll);
    let seq = state
        .worktree_form
        .awaited_issue_load()
        .expect("034's listing reloads");
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: default_issues(),
                    total_open: 2_000,
                    complete: false,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    assert_eq!(state.worktree_form.pending_issue_search(), Some(seq));
    send(&mut state, Msg::IssueSearchDue { seq });
    let (_, text, _) = state
        .worktree_form
        .issue_search_request()
        .expect("034's search beyond runs");
    assert_eq!(text, "crash");
}

/// Review A M2 F3: GitHub refusing the search beyond says so plainly, with no Retry: the same
/// search would be refused again.
#[test]
fn a_refused_filtered_search_says_so_without_retry() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    let seq = state.worktree_form.pending_issue_search().unwrap();
    send(&mut state, Msg::IssueSearchDue { seq });
    filter(
        &mut state,
        GithubMsg::Searched {
            seq,
            result: Err(FilterFailure::Rejected(vec!["Bad query".into()])),
        },
    );
    assert_eq!(
        form(&state).issue_search_status().as_deref(),
        Some("GitHub did not accept the search query: Bad query")
    );
    assert!(!form(&state).issue_search_retries());
    send(&mut state, Msg::IssueRetry);
    assert_eq!(state.worktree_form.github_filtered_search(), None);
    // The loaded matches stay.
    assert_eq!(form(&state).issue_matches.len(), 1);
}

/// A failed or refused search beyond keeps the loaded matches and offers Retry of the same search.
#[test]
fn a_failed_filtered_search_retries_the_same_search() {
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    let seq = state.worktree_form.pending_issue_search().unwrap();
    send(&mut state, Msg::IssueSearchDue { seq });
    let (_, _, before, _) = state.worktree_form.github_filtered_search().unwrap();
    filter(
        &mut state,
        GithubMsg::Searched {
            seq,
            result: Err(FilterFailure::Load(IssueLoadError::TimedOut)),
        },
    );
    assert!(matches!(
        form(&state).issues,
        IssueList::Loaded {
            search: SearchState::Failed { .. },
            ..
        }
    ));
    send(&mut state, Msg::IssueRetry);
    let (_, _, again, _) = state.worktree_form.github_filtered_search().unwrap();
    assert_eq!(again, before);
}

/// Edge "Typed issue number with a filter".
#[test]
fn no_match_with_a_filter_says_the_filter_is_active() {
    let mut complete = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    send(&mut complete, Msg::IssueQueryChanged("#108".into()));
    assert!(form(&complete).issue_matches.is_empty());
    assert_eq!(
        form(&complete).issue_no_match_text(),
        "No issue matches; a filter is active."
    );
    let incomplete = excluding_both(vec![issue(42, "Crash on open", &["bug"])], false);
    assert_eq!(
        form(&incomplete).issue_no_match_text(),
        "No loaded issue matches. The issue may be outside the filter."
    );
    assert_eq!(
        form(&shown(default_issues())).issue_no_match_text(),
        "No open issue matches."
    );
}

/// FR-023: the statement names the filter while one is active; 034's otherwise.
#[test]
fn the_statement_names_the_filter_while_one_is_active() {
    let state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    assert_eq!(
        form(&state).issue_notice().as_deref(),
        Some("Reads issues of o/r from GitHub, sending the filter and the search text.")
    );
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Excluding labels in-progress, blocked")
    );
    let plain = shown(default_issues());
    assert_eq!(
        form(&plain).issue_notice().as_deref(),
        Some("Reads open issues of o/r from GitHub.")
    );
    assert_eq!(form(&plain).github_selector.active_line(), None);
}

/// FR-020: an empty filtered list says no issue matches the filters.
#[test]
fn an_empty_filtered_list_says_no_issue_matches_the_filters() {
    let state = excluding_both(Vec::new(), true);
    assert!(form(&state).issues.held().is_empty());
    assert_eq!(
        form(&state).filtered_empty_text(),
        Some("No issue matches these filters.")
    );
    assert_eq!(form(&shown(Vec::new())).filtered_empty_text(), None);
}

/// FR-018, FR-019: a rejection shows GitHub's messages; a load failure Retry repeats the request.
#[test]
fn a_rejection_and_a_load_failure() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Exclude, "blocked");
    let (seq, _, search) = state.worktree_form.github_filtered_load().unwrap();
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result: Err(FilterFailure::Rejected(vec!["bad label".into()])),
            resolved_env: None,
        },
    );
    assert_eq!(
        form(&state).issues,
        IssueList::Rejected(vec!["bad label".into()])
    );
    assert!(form(&state).issues.held().is_empty());
    assert_eq!(NOT_ACCEPTED, "GitHub did not accept the search query.");

    pick(&mut state, LabelList::Exclude, "bug");
    let (seq, _, _) = state.worktree_form.github_filtered_load().unwrap();
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result: Err(FilterFailure::Load(IssueLoadError::TimedOut)),
            resolved_env: None,
        },
    );
    assert_eq!(
        form(&state).issues,
        IssueList::Failed {
            error: IssueLoadError::TimedOut
        }
    );
    send(&mut state, Msg::IssueRetry);
    let (retried, _, again) = state.worktree_form.github_filtered_load().unwrap();
    assert!(retried > seq);
    assert_ne!(again, search);
    assert!(again.as_str().contains("-label:\"bug\""));
}

/// The redacted `Debug` (FR-024, research R15): no label value is printed.
#[test]
fn the_selector_debug_prints_no_value() {
    let mut state = shown(default_issues());
    with_labels(&mut state);
    pick(&mut state, LabelList::Exclude, "in-progress");
    filter(
        &mut state,
        GithubMsg::QueryChanged(LabelList::Include, "secret-typed".into()),
    );
    let printed = format!("{:?}", form(&state).github_selector);
    assert!(!printed.contains("in-progress"), "{printed}");
    assert!(!printed.contains("secret-typed"), "{printed}");
}

// --- T032: the remembered basic filters (US2) -----------------------------------------------

fn remembering(basic: BasicFilters) -> RememberedSelector {
    RememberedSelector::default().with_change(&SelectorChange::Basic(basic))
}

fn excluding_labels(names: &[&str]) -> BasicFilters {
    let mut basic = BasicFilters::default();
    for name in names {
        basic.exclude_label(label(name));
    }
    basic
}

/// A form opened on `project()` with `remembered` restored, as the shell restores it, then the
/// remotes answered and the GitHub source shown.
fn reopened(remembered: RememberedSelector) -> State {
    let mut state = State::default();
    state.workspace.active = Some(project());
    send(&mut state, Msg::Opened);
    filter(&mut state, GithubMsg::Restored(remembered));
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    state
}

/// A filter message, handing back what it asked the project to remember.
fn remembered_by(state: &mut State, msg: GithubMsg) -> Vec<Outcome> {
    state.update_worktree_form_for_effects(Msg::GithubFilter(msg))
}

fn remembers(basic: BasicFilters) -> Vec<Outcome> {
    vec![Outcome::GithubSelectorRemembered {
        project: project(),
        change: SelectorChange::Basic(basic),
    }]
}

/// US2 AS1, SC-006: the remembered filters apply as the source shows, with one listing and the
/// active line naming them, with no filter interaction.
#[test]
fn reopening_applies_the_remembered_filters_with_one_listing() {
    let before = State::default().worktree_form.issue_request_seq;
    let state = reopened(remembering(excluding_labels(&["in-progress", "blocked"])));
    let (seq, _, search) = state
        .worktree_form
        .github_filtered_load()
        .expect("one filtered listing");
    assert_eq!(search.as_str(), EXCLUDE_BOTH);
    assert_eq!(seq, before + 1, "one listing, no other request");
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Excluding labels in-progress, blocked")
    );
}

/// US2 AS2: another project opens with its own filters, or the default (034's listing).
#[test]
fn another_project_opens_with_its_own_filters_or_the_default() {
    let mut include = BasicFilters::default();
    include.include_label(label("bug"));
    let own = reopened(remembering(include));
    let (_, _, search) = own.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), INCLUDE_BUG);

    let none = reopened(RememberedSelector::default());
    assert_eq!(none.worktree_form.github_filtered_load(), None);
    assert!(none.worktree_form.awaited_issue_load().is_some());
    assert!(form(&none).github_selector.basic.is_default());
    assert_eq!(form(&none).github_selector.active_line(), None);
}

/// FR-021: each basic change is remembered as it is made (US2 AS5: **Clear filters** remembers
/// the default); nothing else is.
#[test]
fn each_basic_change_is_remembered_and_nothing_else_is() {
    let mut state = shown(default_issues());
    assert!(remembered_by(&mut state, GithubMsg::PanelToggled).is_empty());
    assert!(remembered_by(&mut state, GithubMsg::Focused(LabelList::Exclude)).is_empty());
    let fake = FakeIssueSource::new().with_labels(vec![Ok(names(
        &["ui", "in-progress", "bug", "blocked"],
        None,
    ))]);
    let seq = state.worktree_form.awaited_label_choices().unwrap();
    let repo = form(&state).github_repo().cloned().unwrap();
    let result = load_label_choices(&fake, &repo);
    assert!(remembered_by(&mut state, GithubMsg::LabelsLoaded { seq, result }).is_empty());
    assert!(remembered_by(
        &mut state,
        GithubMsg::QueryChanged(LabelList::Exclude, "in".into())
    )
    .is_empty());
    assert!(remembered_by(
        &mut state,
        GithubMsg::HighlightMoved(LabelList::Exclude, Direction::Next)
    )
    .is_empty());

    let index = offered_index(&state, LabelList::Exclude, "in-progress");
    assert_eq!(
        remembered_by(&mut state, GithubMsg::Picked(LabelList::Exclude, index)),
        remembers(excluding_labels(&["in-progress"]))
    );
    let index = offered_index(&state, LabelList::Exclude, "blocked");
    assert_eq!(
        remembered_by(&mut state, GithubMsg::Picked(LabelList::Exclude, index)),
        remembers(excluding_labels(&["in-progress", "blocked"]))
    );
    // The answer of the listing it started is not a change.
    let fake = FakeIssueSource::new().with_search_pages(vec![Ok(page(Vec::new(), 0, None))]);
    let (seq, repo, search) = state.worktree_form.github_filtered_load().unwrap();
    let result = load_filtered(&fake, &repo, &search).map(|listing| (listing, gh()));
    assert!(remembered_by(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result,
            resolved_env: None
        }
    )
    .is_empty());
    assert_eq!(
        remembered_by(&mut state, GithubMsg::Removed(label("in-progress"))),
        remembers(excluding_labels(&["blocked"]))
    );
    // Removing what is not chosen changes nothing, so remembers nothing.
    assert!(remembered_by(&mut state, GithubMsg::Removed(label("in-progress"))).is_empty());
    assert_eq!(
        remembered_by(&mut state, GithubMsg::ClearedAll),
        remembers(BasicFilters::default())
    );
    assert!(remembered_by(&mut state, GithubMsg::ClearedAll).is_empty());
    assert!(remembered_by(&mut state, GithubMsg::PanelDismissed).is_empty());
}

fn offered_index(state: &State, list: LabelList, name: &str) -> usize {
    form(state)
        .github_selector
        .rows(list)
        .iter()
        .position(|row| row.action == LabelAction::Choice(label(name)))
        .unwrap_or_else(|| panic!("{name} is offered in {list:?}"))
}

/// Restoring at form open is not a change: nothing is written back.
#[test]
fn restoring_remembers_nothing() {
    let mut state = State::default();
    state.workspace.active = Some(project());
    send(&mut state, Msg::Opened);
    assert!(remembered_by(
        &mut state,
        GithubMsg::Restored(remembering(excluding_labels(&["blocked"])))
    )
    .is_empty());
    assert_eq!(
        form(&state).github_selector.basic,
        excluding_labels(&["blocked"])
    );
}

/// Spec Edge "Remembered value no longer in the repository": the label still applies, and its
/// chip says it is not in this repository once the labels loaded without it.
#[test]
fn a_remembered_label_the_repository_lacks_still_applies_and_says_so() {
    let mut state = reopened(remembering(excluding_labels(&["in-progress", "retired"])));
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert!(search.as_str().contains("-label:\"retired\""), "{search:?}");
    let selector = &form(&state).github_selector;
    // Before the labels load, nothing is known about it.
    assert_eq!(selector.chip_text(&label("retired")), "retired");
    with_labels(&mut state);
    let selector = &form(&state).github_selector;
    assert_eq!(
        selector.chip_text(&label("retired")),
        "retired · not in this repository"
    );
    assert_eq!(selector.chip_text(&label("in-progress")), "in-progress");
    assert_eq!(
        selector.chosen(LabelList::Exclude),
        &[label("in-progress"), label("retired")]
    );
}

/// GitHub labels match case-insensitively, so a case change is not a missing label; and with only
/// the first labels read, an absent one may be further on, so nothing is said.
#[test]
fn a_label_in_another_case_or_beyond_a_partial_list_is_not_called_missing() {
    let mut state = reopened(remembering(excluding_labels(&["UI", "zzz"])));
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::Focused(LabelList::Exclude));
    let fake = FakeIssueSource::new().with_labels(
        (0..10)
            .map(|i| Ok(names(&["ui"], Some(&format!("c{i}")))))
            .collect(),
    );
    serve_labels(&mut state, &fake);
    let selector = &form(&state).github_selector;
    assert_eq!(selector.chip_text(&label("UI")), "UI");
    assert_eq!(selector.chip_text(&label("zzz")), "zzz");
}

// --- T036: the state filter (US5) -----------------------------------------------------------

const ONLY_CLOSED: &str = "repo:o/r is:issue is:closed sort:updated-desc";
const ANY_STATE: &str = "repo:o/r is:issue sort:updated-desc";

fn closed_issues() -> Vec<Issue> {
    vec![
        issue(31, "Old crash", &["bug"]).closed(true),
        issue(12, "Done docs", &[]).closed(true),
    ]
}

fn with_state(state: IssueState) -> BasicFilters {
    let mut basic = BasicFilters::default();
    basic.set_state(state);
    basic
}

/// A shown form whose State row chose `chosen`, its listing answered with `issues`.
fn stated(chosen: IssueState, issues: Vec<Issue>) -> State {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::PanelToggled);
    filter(&mut state, GithubMsg::StateChosen(chosen));
    let total = issues.len() as u64;
    let fake = FakeIssueSource::new().with_search_pages(vec![Ok(page(issues, total, None))]);
    assert!(serve(&mut state, &fake));
    state
}

/// US5 AS1: the State row offers Open, Closed and All, Open chosen by default.
#[test]
fn the_state_row_offers_open_closed_and_all_with_open_chosen() {
    let state = shown(default_issues());
    assert_eq!(
        form(&state).github_selector.state_choices(),
        vec![
            (IssueState::Open, "Open", true),
            (IssueState::Closed, "Closed", false),
            (IssueState::All, "All", false),
        ]
    );
}

/// US5 AS2: Closed sends one listing with `is:closed`; every listed issue is closed.
#[test]
fn closed_lists_the_closed_issues_with_one_listing() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::PanelToggled);
    let before = state.worktree_form.issue_request_seq;
    filter(&mut state, GithubMsg::StateChosen(IssueState::Closed));
    assert_eq!(state.worktree_form.issue_request_seq, before + 1);
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), ONLY_CLOSED);
    assert_eq!(
        form(&state).github_selector.state_choices()[1],
        (IssueState::Closed, "Closed", true)
    );

    let state = stated(IssueState::Closed, closed_issues());
    let held = form(&state).issues.held();
    assert_eq!(held.len(), 2);
    assert!(held.iter().all(|issue| issue.is_closed()));
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Closed issues")
    );
}

/// All sends no state term; going back to Open is 034's listing again.
#[test]
fn all_sends_no_state_term_and_open_is_the_default_listing() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::StateChosen(IssueState::All));
    let (_, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), ANY_STATE);
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Open and closed issues")
    );
    assert_eq!(form(&state).github_selector.basic.chosen_count(), 1);

    filter(&mut state, GithubMsg::StateChosen(IssueState::Open));
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert!(state.worktree_form.awaited_issue_load().is_some());
    assert_eq!(form(&state).github_selector.basic.chosen_count(), 0);
}

/// US5 AS3, FR-017: a closed issue picks like an open one.
#[test]
fn a_closed_issue_fills_the_form_as_an_open_one_does() {
    let pick_issue = |closed: bool| {
        let mut state = stated(
            IssueState::All,
            vec![issue(31, "Old crash", &["bug"]).closed(closed)],
        );
        send(
            &mut state,
            Msg::IssuePicked {
                number: 31,
                mapping: vec![],
            },
        );
        let f = form(&state);
        (
            f.ticket.clone(),
            f.name.clone(),
            f.type_,
            f.picked_issue,
            f.issue_list_open,
        )
    };
    let closed = pick_issue(true);
    assert_eq!(closed.0, "31");
    assert_eq!(closed.1, "Old crash");
    assert_eq!(closed.3, Some(31));
    assert_eq!(closed, pick_issue(false));
}

/// FR-021: each state change is remembered as a basic change; choosing the chosen state is not a
/// change.
#[test]
fn each_state_change_is_remembered() {
    let mut state = shown(default_issues());
    assert_eq!(
        remembered_by(&mut state, GithubMsg::StateChosen(IssueState::Closed)),
        remembers(with_state(IssueState::Closed))
    );
    assert!(remembered_by(&mut state, GithubMsg::StateChosen(IssueState::Closed)).is_empty());
    assert_eq!(
        remembered_by(&mut state, GithubMsg::StateChosen(IssueState::All)),
        remembers(with_state(IssueState::All))
    );
    assert_eq!(
        remembered_by(&mut state, GithubMsg::StateChosen(IssueState::Open)),
        remembers(BasicFilters::default())
    );
}

/// FR-021, US2 AS1: a remembered Closed state reopens with one `is:closed` listing.
#[test]
fn a_remembered_closed_state_reopens_with_one_closed_listing() {
    let before = State::default().worktree_form.issue_request_seq;
    let state = reopened(remembering(with_state(IssueState::Closed)));
    let (seq, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), ONLY_CLOSED);
    assert_eq!(seq, before + 1);
    assert_eq!(
        form(&state).github_selector.state_choices()[1],
        (IssueState::Closed, "Closed", true)
    );
}

/// A form restored with `chosen` and a label excluded (so the filtered listing is asked even for
/// Open), its listing capped: two of 1,234 held.
fn capped(chosen: IssueState) -> State {
    let mut basic = excluding_labels(&["wontfix"]);
    basic.set_state(chosen);
    let mut state = reopened(remembering(basic));
    let (seq, _, _) = state.worktree_form.github_filtered_load().unwrap();
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: vec![issue(2, "Two", &[]), issue(1, "One", &[])],
                    total_open: 1_234,
                    complete: false,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    state
}

/// FR-004, selector-ui §3: the cap caption names the state the count is of; it never says "open"
/// of closed issues.
#[test]
fn the_cap_caption_names_the_chosen_state() {
    for (chosen, count) in [
        (IssueState::Open, "1,234 open issues"),
        (IssueState::Closed, "1,234 closed issues"),
        (IssueState::All, "1,234 issues"),
    ] {
        assert_eq!(
            form(&capped(chosen)).issue_cap_caption(),
            Some(format!(
                "Showing the 2 most recently updated of {count} — search also looks on GitHub."
            )),
            "{chosen:?}"
        );
    }
}

/// FR-025: the statement before the source is chosen says which issues choosing it reads, from the
/// remembered state.
#[test]
fn the_pre_source_hint_names_the_remembered_state() {
    for (chosen, words) in [
        (IssueState::Open, "open issues"),
        (IssueState::Closed, "closed issues"),
        (IssueState::All, "open and closed issues"),
    ] {
        let mut state = reopened(remembering(with_state(chosen)));
        send(&mut state, Msg::SourceChanged(BranchSource::New));
        assert_eq!(
            form(&state).source_caption(),
            Some(format!("GitHub issue reads {words} of o/r from GitHub.")),
            "{chosen:?}"
        );
    }
}

// --- T041: advanced mode (US3) ----------------------------------------------------------------

/// Search-rules §4's scoped listing text of `query` in `o/r`, without a sort qualifier of its own.
fn advanced(condition: &str) -> String {
    format!("repo:o/r is:issue ({condition}) sort:updated-desc")
}

fn held_numbers(state: &State) -> Vec<u64> {
    form(state)
        .issues
        .held()
        .iter()
        .map(|i| i.number())
        .collect()
}

/// Type `query` into the Query input and run it.
fn run(state: &mut State, query: &str) {
    filter(state, GithubMsg::DraftChanged(query.to_string()));
    filter(state, GithubMsg::Run);
}

/// Answer the filtered listing the form awaits with `answer`, through `load_filtered` against a fake
/// source; what the fake was asked.
fn answer_with(state: &mut State, answer: Result<IssuePage, FilterFailure>) -> Vec<String> {
    let fake = FakeIssueSource::new().with_search_pages(vec![answer]);
    assert!(serve(state, &fake), "a filtered listing is awaited");
    fake.asked_searches()
}

/// US3 AS1, FR-002: switching to Advanced sends nothing, keeps the search text and the list.
#[test]
fn switching_to_advanced_sends_nothing_and_keeps_the_search_and_the_list() {
    let mut state = shown(default_issues());
    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    let seq = state.worktree_form.issue_request_seq;
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(form(&state).github_selector.mode, QueryMode::Advanced);
    assert_eq!(
        state.worktree_form.issue_request_seq, seq,
        "nothing is sent"
    );
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert_eq!(form(&state).issue_query, "crash");
    assert_eq!(held_numbers(&state), vec![7, 42, 108]);
    assert!(form(&state).shows_github_filters());
    assert_eq!(form(&state).github_selector.active_line(), None);
}

/// FR-010: editing the draft never sends.
#[test]
fn editing_the_draft_never_sends() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    let seq = state.worktree_form.issue_request_seq;
    for text in ["s", "st", "state:open"] {
        filter(&mut state, GithubMsg::DraftChanged(text.into()));
    }
    assert_eq!(form(&state).github_selector.draft, "state:open");
    assert_eq!(state.worktree_form.issue_request_seq, seq);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert_eq!(state.worktree_form.github_filtered_load(), None);
}

/// US3 AS2: Run (Enter or **Run**) sends one listing with search-rules §4's text; the draft shows
/// the query as run, and the list holds GitHub's answer in its order.
#[test]
fn run_lists_the_query_within_the_repository() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "  -label:in-progress\tstate:open ");
    let selector = &form(&state).github_selector;
    assert_eq!(selector.draft, "-label:in-progress state:open");
    assert_eq!(selector.listed_for, Some(QueryMode::Advanced));
    let asked = answer_with(
        &mut state,
        Ok(page(
            vec![
                issue(42, "Crash on open", &["bug"]),
                issue(108, "Blocked docs", &["blocked"]),
            ],
            2,
            None,
        )),
    );
    assert_eq!(asked, vec![advanced("-label:in-progress state:open")]);
    assert_eq!(held_numbers(&state), vec![42, 108]);
    assert_eq!(
        form(&state).issue_notice().as_deref(),
        Some("Reads issues of o/r from GitHub, sending the filter and the search text.")
    );
}

/// US3 AS3: a sort qualifier is kept, and the list is in GitHub's order for it; a capped list
/// names the first issues, not the most recently updated.
#[test]
fn a_sort_qualifier_is_kept_in_githubs_order() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "-label:blocked sort:created-asc");
    let (seq, _, search) = state.worktree_form.github_filtered_load().unwrap();
    assert_eq!(
        search.as_str(),
        "repo:o/r is:issue (-label:blocked) sort:created-asc"
    );
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: vec![issue(3, "Old", &[]), issue(9, "Newer", &[])],
                    total_open: 1_234,
                    complete: false,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    assert_eq!(held_numbers(&state), vec![3, 9]);
    assert_eq!(
        form(&state).issue_cap_caption().as_deref(),
        Some("Showing the first 2 of 1,234 issues — search also looks on GitHub.")
    );
}

/// US3 AS4: the query, not the app, decides the state: closed issues are listed and marked.
#[test]
fn closed_issues_a_query_returns_are_listed_and_marked() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "state:closed");
    answer_with(
        &mut state,
        Ok(page(
            vec![issue(5, "Fixed crash", &["bug"]).closed(true)],
            1,
            None,
        )),
    );
    let held = form(&state).issues.held();
    assert_eq!(held.len(), 1);
    assert!(held[0].is_closed());
}

/// US3 AS5, FR-020: an empty result says no issue matches the query, distinct from an error.
#[test]
fn an_empty_result_says_no_issue_matches_this_query() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:nothing");
    answer_with(&mut state, Ok(page(Vec::new(), 0, None)));
    assert!(matches!(form(&state).issues, IssueList::Loaded { .. }));
    assert_eq!(
        form(&state).filtered_empty_text(),
        Some("No issue matches this query.")
    );
}

/// US3 AS6, FR-002: back to Basic reloads the basic filters only when a query's list shows, and the
/// draft is kept for the next switch.
#[test]
fn switching_back_to_basic_restores_the_basic_list_and_keeps_the_draft() {
    // No query ran: the basic list never left, so nothing is sent.
    let mut state = excluding_both(vec![issue(42, "Crash on open", &["bug"])], true);
    let seq = state.worktree_form.issue_request_seq;
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    filter(&mut state, GithubMsg::DraftChanged("label:bug".into()));
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Basic));
    assert_eq!(state.worktree_form.issue_request_seq, seq);
    assert_eq!(held_numbers(&state), vec![42]);
    assert_eq!(
        form(&state).github_selector.active_line().as_deref(),
        Some("Excluding labels in-progress, blocked")
    );

    // A query ran: back to Basic loads the basic filters' list again.
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(form(&state).github_selector.draft, "label:bug");
    filter(&mut state, GithubMsg::Run);
    answer_with(
        &mut state,
        Ok(page(vec![issue(77, "Bug", &["bug"])], 1, None)),
    );
    filter(&mut state, GithubMsg::DraftChanged("label:ui".into()));
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Basic));
    assert_eq!(
        form(&state).github_selector.listed_for,
        Some(QueryMode::Basic)
    );
    let asked = answer_with(
        &mut state,
        Ok(page(vec![issue(42, "Crash on open", &["bug"])], 1, None)),
    );
    assert_eq!(asked, vec![EXCLUDE_BOTH.to_string()]);
    assert_eq!(held_numbers(&state), vec![42]);
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(form(&state).github_selector.draft, "label:ui");
}

/// A mode switch discards an answer in flight: the run's when back to Basic, the basic filters'
/// when to Advanced, where the prompt then shows until a query runs.
#[test]
fn a_mode_switch_discards_an_answer_in_flight() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:bug");
    let (ran, _, _) = state.worktree_form.github_filtered_load().unwrap();
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Basic));
    let basic = state
        .worktree_form
        .awaited_issue_load()
        .expect("the basic list loads");
    assert!(basic > ran);
    let late = |seq| GithubMsg::Loaded {
        seq,
        result: Ok((
            micold_core::github::IssueListing {
                issues: vec![issue(77, "Bug", &["bug"])],
                total_open: 1,
                complete: true,
            },
            gh(),
        )),
        resolved_env: None,
    };
    filter(&mut state, late(ran));
    assert_eq!(form(&state).issues, IssueList::Loading { seq: basic });

    // The basic list in flight, then Advanced: its answer is dropped, nothing else is sent.
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq: basic,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: default_issues(),
                    total_open: 3,
                    complete: true,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    assert!(form(&state).issues.held().is_empty());
    assert!(form(&state).shows_github_filters(), "the switch stays");
}

/// FR-016 in advanced mode: the search beyond a capped list carries the query, the typed words
/// quoted (search-rules §4's `beyond`).
#[test]
fn the_search_beyond_carries_the_query() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "a OR b sort:created-asc");
    let (seq, _, _) = state.worktree_form.github_filtered_load().unwrap();
    filter(
        &mut state,
        GithubMsg::Loaded {
            seq,
            result: Ok((
                micold_core::github::IssueListing {
                    issues: vec![issue(3, "Old crash", &[])],
                    total_open: 2_000,
                    complete: false,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    send(&mut state, Msg::IssueQueryChanged("crash OR ui".into()));
    let seq = state.worktree_form.pending_issue_search().unwrap();
    send(&mut state, Msg::IssueSearchDue { seq });
    let (_, _, search, _) = state.worktree_form.github_filtered_search().unwrap();
    assert_eq!(
        search.as_str(),
        "repo:o/r is:issue (a OR b) \"crash\" \"OR\" \"ui\""
    );
}

/// Spec Edge "Switching source to Jira and back": in advanced mode the last run query runs again,
/// not the draft; with none run, nothing is sent and the prompt shows.
#[test]
fn back_from_jira_in_advanced_runs_the_last_run_query_not_the_draft() {
    let mut both = both_providers();
    filter(&mut both, GithubMsg::ModeChanged(QueryMode::Advanced));
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Jira));
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Github));
    assert_eq!(
        both.worktree_form.awaited_issue_load(),
        None,
        "no query ran: nothing is sent"
    );
    assert!(form(&both).shows_github_filters());

    run(&mut both, "state:open");
    answer_with(&mut both, Ok(page(vec![issue(42, "Crash", &[])], 1, None)));
    filter(&mut both, GithubMsg::DraftChanged("label:x".into()));
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Jira));
    assert_eq!(both.worktree_form.github_filtered_load(), None);
    send(&mut both, Msg::IssueProviderChanged(IssueProvider::Github));
    let (_, _, search) = both.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), advanced("state:open"));
    assert_eq!(form(&both).github_selector.draft, "label:x");

    // The same through the branch source.
    send(&mut both, Msg::SourceChanged(BranchSource::New));
    send(&mut both, Msg::SourceChanged(BranchSource::Issue));
    let (_, _, search) = both.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), advanced("state:open"));
}

// --- T046: refusals and GitHub's errors (US4) -------------------------------------------------

/// US4 AS5, SC-004, FR-012: a query that could leave the scope, or is malformed or too long, is
/// refused before sending; the reason stands in place of the list and the draft is kept.
#[test]
fn a_refused_query_is_never_sent() {
    let long = "a".repeat(1_001);
    for (query, refusal) in [
        ("repo:other/x", QueryRefusal::ScopeQualifier),
        ("(a", QueryRefusal::UnbalancedParentheses),
        ("label:\"x", QueryRefusal::UnclosedQuote),
        (long.as_str(), QueryRefusal::TooLong),
    ] {
        let mut state = shown(default_issues());
        filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
        run(&mut state, query);
        assert_eq!(
            form(&state).issues,
            IssueList::Refused(refusal),
            "{query:.20}"
        );
        assert_eq!(state.worktree_form.awaited_issue_load(), None);
        assert_eq!(state.worktree_form.github_filtered_load(), None);
        assert_eq!(form(&state).github_selector.draft, query);
        assert!(form(&state).issues.held().is_empty());
    }
    assert!(QueryRefusal::ScopeQualifier.message().contains("repo:"));
}

/// US4 AS1, AS2: GitHub's refusal shows its messages in order, the draft kept; editing and running
/// again replaces the error with the new result.
#[test]
fn githubs_refusal_shows_its_messages_and_a_new_run_replaces_them() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "lable:bug");
    answer_with(
        &mut state,
        Err(FilterFailure::Rejected(vec!["m1".into(), "m2".into()])),
    );
    assert_eq!(
        form(&state).issues,
        IssueList::Rejected(vec!["m1".into(), "m2".into()])
    );
    assert_eq!(form(&state).github_selector.draft, "lable:bug");
    run(&mut state, "label:bug");
    let asked = answer_with(
        &mut state,
        Ok(page(vec![issue(42, "Crash", &["bug"])], 1, None)),
    );
    assert_eq!(asked, vec![advanced("label:bug")]);
    assert_eq!(held_numbers(&state), vec![42]);
}

/// US4 AS3, FR-019: another failure shows 034's notice, and Retry runs the same query again.
#[test]
fn a_failed_run_retries_the_same_query() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:bug");
    answer_with(
        &mut state,
        Err(FilterFailure::Load(IssueLoadError::Offline)),
    );
    assert_eq!(
        form(&state).issues,
        IssueList::Failed {
            error: IssueLoadError::Offline
        }
    );
    filter(&mut state, GithubMsg::DraftChanged("label:ui".into()));
    send(&mut state, Msg::IssueRetry);
    let asked = answer_with(&mut state, Ok(page(Vec::new(), 0, None)));
    assert_eq!(asked, vec![advanced("label:bug")]);
}

/// US4 AS4: a refusal without a message says so in plain words.
#[test]
fn a_refusal_without_a_message_says_so_plainly() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:bug");
    answer_with(&mut state, Err(FilterFailure::Rejected(Vec::new())));
    assert_eq!(form(&state).issues, IssueList::Rejected(Vec::new()));
    assert_eq!(NOT_ACCEPTED, "GitHub did not accept the search query.");
}

// --- M5 review round 1 fixes ------------------------------------------------------------------

/// Review A F3: after a query ran, a basic list dropped by a switch to Advanced while it loaded
/// leaves nothing listed: no filter is active, the statement does not say one is sent, and the
/// prompt stands in place of the list; nothing is sent.
#[test]
fn a_basic_load_dropped_for_advanced_leaves_no_active_filter() {
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:bug");
    answer_with(
        &mut state,
        Ok(page(vec![issue(77, "Bug", &["bug"])], 1, None)),
    );
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Basic));
    assert!(state.worktree_form.awaited_issue_load().is_some());
    let seq = state.worktree_form.issue_request_seq;
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    assert_eq!(
        state.worktree_form.issue_request_seq, seq,
        "nothing is sent"
    );
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert!(form(&state).shows_github_advanced(), "the prompt shows");
    assert_eq!(form(&state).github_selector.active_filter(), None);
    assert_eq!(form(&state).github_filter(), None);
    assert_eq!(
        form(&state).issue_notice().as_deref(),
        Some("Reads open issues of o/r from GitHub.")
    );
}

/// Visual pass F1 (spec Edge "Empty advanced query"): running an empty or whitespace-only query
/// sends nothing: the prompt stands in place of the list, as before any run, also after a list
/// showed.
#[test]
fn an_empty_run_sends_nothing_and_keeps_the_prompt() {
    for text in ["", "   ", "\t \n"] {
        let mut state = shown(default_issues());
        filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
        run(&mut state, text);
        assert_eq!(state.worktree_form.github_filtered_load(), None, "{text:?}");
        assert_eq!(state.worktree_form.awaited_issue_load(), None, "{text:?}");
        assert_eq!(form(&state).issues, IssueList::NotRequested, "{text:?}");
        assert!(form(&state).shows_github_advanced(), "the prompt shows");
        assert_eq!(form(&state).github_selector.last_run, None);
        assert_eq!(form(&state).github_selector.active_filter(), None);
    }

    // After a query's list: an empty run drops it for the prompt, and sends nothing.
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    run(&mut state, "label:bug");
    answer_with(
        &mut state,
        Ok(page(vec![issue(77, "Bug", &["bug"])], 1, None)),
    );
    run(&mut state, "  ");
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert!(form(&state).shows_github_advanced(), "the prompt shows");
    assert_eq!(form(&state).github_selector.last_run, None);
    assert_eq!(form(&state).github_selector.draft, "");
    assert_eq!(form(&state).github_filter(), None);
    // Back to Basic then loads the basic list again.
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Basic));
    assert!(state.worktree_form.awaited_issue_load().is_some());
}

// --- T050: the remembered advanced mode and last run query (US2 AS3, AS4) -------------------------

/// What the project remembers after `mode` and the query `ran` (if any) were written, as the shell
/// writes them: one field per change.
fn remembered_advanced(ran: Option<&str>) -> RememberedSelector {
    let mut remembered =
        RememberedSelector::default().with_change(&SelectorChange::Mode(QueryMode::Advanced));
    if let Some(query) = ran {
        remembered = remembered.with_change(&SelectorChange::Query(query.into()));
    }
    remembered
}

fn remembers_change(change: SelectorChange) -> Vec<Outcome> {
    vec![Outcome::GithubSelectorRemembered {
        project: project(),
        change,
    }]
}

/// Write what a session handed back into `remembered`, as the shell does.
fn persist(remembered: RememberedSelector, outcomes: Vec<Outcome>) -> RememberedSelector {
    outcomes
        .into_iter()
        .fold(remembered, |held, outcome| match outcome {
            Outcome::GithubSelectorRemembered { change, .. } => held.with_change(&change),
            _ => held,
        })
}

/// US2 AS3, FR-013: a project last used in Advanced reopens in Advanced with its last run query in
/// the input, and runs it once as soon as the GitHub source shows, with no interaction.
#[test]
fn reopening_in_advanced_runs_the_last_run_query_once() {
    let before = State::default().worktree_form.issue_request_seq;
    let mut state = reopened(remembered_advanced(Some("label:bug")));
    let selector = &form(&state).github_selector;
    assert_eq!(selector.mode, QueryMode::Advanced);
    assert_eq!(selector.draft, "label:bug");
    assert_eq!(
        selector.last_run.as_ref().map(|q| q.as_str()),
        Some("label:bug")
    );
    assert!(form(&state).shows_github_advanced());
    let (seq, _, search) = state
        .worktree_form
        .github_filtered_load()
        .expect("the last run query is listed");
    assert_eq!(search.as_str(), advanced("label:bug"));
    assert_eq!(seq, before + 1, "one listing, no other request");

    let asked = answer_with(
        &mut state,
        Ok(page(vec![issue(42, "Crash", &["bug"])], 1, None)),
    );
    assert_eq!(asked, vec![advanced("label:bug")]);
    assert_eq!(held_numbers(&state), vec![42]);
    assert_eq!(state.worktree_form.github_filtered_load(), None, "run once");
    assert_eq!(form(&state).github_selector.active_line(), None);
}

/// FR-013: Advanced remembered but no query ever run: the prompt shows and nothing is sent.
#[test]
fn reopening_in_advanced_with_no_query_run_shows_the_prompt_and_sends_nothing() {
    let state = reopened(remembered_advanced(None));
    assert_eq!(form(&state).github_selector.mode, QueryMode::Advanced);
    assert_eq!(form(&state).github_selector.draft, "");
    assert_eq!(form(&state).github_selector.last_run, None);
    assert_eq!(state.worktree_form.github_filtered_load(), None);
    assert_eq!(state.worktree_form.awaited_issue_load(), None);
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert!(form(&state).shows_github_advanced(), "the prompt shows");
}

/// M5 review (D5): an empty run is not a run, so a file holding one reopens on the prompt.
#[test]
fn an_empty_run_is_not_remembered_as_run() {
    // The empty run hands back nothing to remember.
    let mut state = shown(default_issues());
    filter(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced));
    for text in ["", "   "] {
        filter(&mut state, GithubMsg::DraftChanged(text.into()));
        assert!(
            remembered_by(&mut state, GithubMsg::Run).is_empty(),
            "{text:?}"
        );
    }
    // A remembered empty query, ran or not, reopens on the prompt with nothing sent.
    for ran in [Some(""), None] {
        let state = reopened(remembered_advanced(ran));
        assert_eq!(state.worktree_form.github_filtered_load(), None);
        assert_eq!(state.worktree_form.awaited_issue_load(), None);
        assert_eq!(form(&state).issues, IssueList::NotRequested);
        assert_eq!(form(&state).github_selector.last_run, None);
        assert!(form(&state).shows_github_advanced());
    }
}

/// US2 AS4: Q2 typed and not run is not remembered: the form reopens showing Q, running Q.
#[test]
fn a_draft_typed_and_not_run_is_not_remembered() {
    let mut state = reopened(remembered_advanced(Some("label:bug")));
    answer_with(
        &mut state,
        Ok(page(vec![issue(42, "Crash", &["bug"])], 1, None)),
    );
    let typed = remembered_by(&mut state, GithubMsg::DraftChanged("label:ui".into()));
    assert!(typed.is_empty(), "a draft is never remembered");
    assert_eq!(form(&state).github_selector.draft, "label:ui");

    // Closing the form with the draft unrun leaves the file as it was.
    let held = remembered_advanced(Some("label:bug"));
    let again = reopened(persist(held.clone(), typed));
    assert_eq!(form(&again).github_selector.draft, "label:bug");
    let (_, _, search) = again.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), advanced("label:bug"));
}

/// FR-021: `Remembered(Mode)` at each switch that changes the mode, `Remembered(Query)` at each
/// accepted run with the query as cleaned; never for a draft, a refused run or an empty run.
#[test]
fn the_mode_and_each_accepted_run_are_remembered_and_nothing_else_is() {
    let mut state = shown(default_issues());
    assert_eq!(
        remembered_by(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced)),
        remembers_change(SelectorChange::Mode(QueryMode::Advanced))
    );
    assert!(
        remembered_by(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced)).is_empty(),
        "no switch, nothing remembered"
    );
    assert!(remembered_by(&mut state, GithubMsg::DraftChanged("label:bug".into())).is_empty());

    assert_eq!(
        remembered_by(&mut state, GithubMsg::Run),
        remembers_change(SelectorChange::Query("label:bug".into()))
    );
    answer_with(
        &mut state,
        Ok(page(vec![issue(42, "Crash", &["bug"])], 1, None)),
    );

    // A run the checks refuse: shown, not remembered.
    filter(&mut state, GithubMsg::DraftChanged("repo:other/x".into()));
    assert!(remembered_by(&mut state, GithubMsg::Run).is_empty());
    assert!(matches!(form(&state).issues, IssueList::Refused(_)));

    // An empty run: not remembered.
    filter(&mut state, GithubMsg::DraftChanged("  ".into()));
    assert!(remembered_by(&mut state, GithubMsg::Run).is_empty());

    // A second accepted run is remembered, trimmed (the text as the input shows it).
    filter(
        &mut state,
        GithubMsg::DraftChanged("  label:ui   state:open ".into()),
    );
    assert_eq!(
        remembered_by(&mut state, GithubMsg::Run),
        remembers_change(SelectorChange::Query("label:ui   state:open".into()))
    );

    // The switch back to Basic is remembered as a mode change alone (the basic filters did not
    // change), and a Run in basic mode is nothing.
    assert_eq!(
        remembered_by(&mut state, GithubMsg::ModeChanged(QueryMode::Basic)),
        remembers_change(SelectorChange::Mode(QueryMode::Basic))
    );
    assert!(remembered_by(&mut state, GithubMsg::Run).is_empty());
}

/// The whole round trip: what a session hands back, written as the shell writes it, reopens the
/// same form.
#[test]
fn what_a_session_remembers_reopens_the_same_form() {
    let mut state = shown(default_issues());
    let mut held = RememberedSelector::default();
    held = persist(
        held,
        remembered_by(&mut state, GithubMsg::ModeChanged(QueryMode::Advanced)),
    );
    filter(&mut state, GithubMsg::DraftChanged("label:bug".into()));
    held = persist(held, remembered_by(&mut state, GithubMsg::Run));
    filter(&mut state, GithubMsg::DraftChanged("not run".into()));

    let again = reopened(held);
    assert_eq!(form(&again).github_selector.mode, QueryMode::Advanced);
    assert_eq!(form(&again).github_selector.draft, "label:bug");
    let (_, _, search) = again.worktree_form.github_filtered_load().unwrap();
    assert_eq!(search.as_str(), advanced("label:bug"));
}

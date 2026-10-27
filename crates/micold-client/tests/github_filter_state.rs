//! The GitHub filters of the issue picker, as state transitions (feature 1400, US1,
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
use micold_core::git::GitRemote;
use micold_core::github::{FakeIssueSource, Issue, IssueLoadError, IssuePage, NamePage};
use micold_core::github_filter::{
    load_filtered, load_label_choices, FilterFailure, LabelName, NOT_ACCEPTED,
};
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
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

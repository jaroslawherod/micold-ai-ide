//! The GitHub issue source of the add-worktree form, as state transitions (feature 034,
//! data-model §5).
//!
//! Everything the picker decides lives in the reducer: whether the source may be chosen, which
//! load result is the one being waited for, what a pick writes into the form, and what switching
//! away keeps. None of it needs `gh`, a network or a display, so all of it is asserted here. The
//! shell half — who asks the daemon, who runs `gh` — is in `src/main_tests.rs` (tests named
//! `issue…`); the rendered half is quickstart §B.

use std::path::PathBuf;

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueList, Msg, SearchState, WorktreeForm,
};
use micold_core::git::GitRemote;
use micold_core::github::{GithubRepo, Issue, IssueListing, IssueLoadError};
use micold_core::issue_types::{default_mapping, LabelTypeEntry};
use micold_core::naming::ConventionalType;
use micold_core::typeahead::Direction;

// --- fixtures -------------------------------------------------------------------------------

const OWNER_NAME: &str = "o/r";
const NO_GITHUB_REMOTE: &str = "This repository has no GitHub remote.";
const CHECKING: &str = "Checking for a GitHub remote…";

fn remote(name: &str, url: &str) -> GitRemote {
    GitRemote {
        name: name.to_string(),
        url: url.to_string(),
    }
}

fn issue(number: u64, title: &str, labels: &[&str]) -> Issue {
    Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "2026-09-29T00:00:00Z".to_string(),
    )
}

/// Three open issues, most recently updated first: a number, a label and a title fragment each
/// find exactly one of them.
fn issues() -> Vec<Issue> {
    vec![
        issue(7, "Sidebar flickers on resize", &["ui"]),
        issue(42, "Crash when opening empty project", &["bug"]),
        issue(108, "Document the sandbox placement", &["documentation"]),
    ]
}

fn listing(issues: Vec<Issue>) -> IssueListing {
    IssueListing {
        total_open: issues.len() as u64,
        complete: true,
        issues,
    }
}

fn gh() -> PathBuf {
    PathBuf::from("/usr/bin/gh")
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.form.as_ref().expect("the form is open")
}

/// A form just opened: the remotes are still being read.
fn opened() -> State {
    let mut state = State::default();
    send(&mut state, Msg::Opened);
    state
}

/// A form whose repository has `origin` on github.com.
fn available() -> State {
    let mut state = opened();
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![remote("origin", "git@github.com:o/r.git")])),
    );
    state
}

/// The seq the form is waiting for, when it is loading.
fn awaited(state: &State) -> Option<u64> {
    match &form(state).issues {
        IssueList::Loading { seq } => Some(*seq),
        _ => None,
    }
}

/// A form on the issue source, its load in flight; returns the awaited seq.
fn loading() -> (State, u64) {
    let mut state = available();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state).expect("choosing an available source starts a load");
    (state, seq)
}

fn loaded_result(seq: u64, issues: Vec<Issue>) -> Msg {
    Msg::IssuesLoaded {
        seq,
        result: Ok((listing(issues), gh())),
        resolved_env: None,
    }
}

/// A form on the issue source with `issues()` loaded.
fn loaded() -> State {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq, issues()));
    state
}

fn is_loaded(state: &State) -> bool {
    matches!(form(state).issues, IssueList::Loaded { .. })
}

/// The numbers currently offered, in the order offered.
fn offered(state: &State) -> Vec<u64> {
    (0..form(state).issue_matches.len())
        .map(|i| {
            state
                .worktree_form
                .issue_number_at(i)
                .expect("every match resolves to a number")
        })
        .collect()
}

fn query(state: &mut State, text: &str) {
    send(state, Msg::IssueQueryChanged(text.to_string()));
}

// --- T018: availability, loading, staleness ------------------------------------------------

/// U96 — the caption under the switch says what the chip can do, and before the choice that
/// contacts GitHub it names the repository it would read (FR-002, FR-025).
#[test]
fn captions_follow_availability() {
    let state = opened();
    assert_eq!(form(&state).github, GithubAvailability::Checking);
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert_eq!(
        form(&state).source_caption().as_deref(),
        Some(CHECKING),
        "while the remotes are read the chip is disabled and says why"
    );

    let mut state = opened();
    send(&mut state, Msg::RemotesListed(Ok(vec![])));
    assert_eq!(
        form(&state).source_caption().as_deref(),
        Some(NO_GITHUB_REMOTE)
    );

    let state = available();
    assert_eq!(
        form(&state).source_caption(),
        Some(format!(
            "GitHub issue reads open issues of {OWNER_NAME} from GitHub."
        )),
        "the opt-in notice names the repository before anything is sent (FR-025)"
    );
    assert_eq!(
        form(&state).issue_notice(),
        None,
        "the body notice belongs to the issue source only"
    );

    let (state, _) = loading();
    assert_eq!(
        form(&state).source_caption(),
        None,
        "once chosen, the body's notice takes over"
    );
    assert_eq!(
        form(&state).issue_notice(),
        Some(format!("Reads open issues of {OWNER_NAME} from GitHub."))
    );
}

/// U44 — the remotes decide whether the chip is usable, and a failed lookup says why (FR-002).
#[test]
fn remotes_decide_availability() {
    let state = available();
    assert_eq!(
        form(&state).github,
        GithubAvailability::Available(
            GithubRepo::from_remote_url("https://github.com/o/r").unwrap()
        )
    );

    let mut state = opened();
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![remote("origin", "https://gitlab.com/o/r.git")])),
    );
    assert_eq!(
        form(&state).github,
        GithubAvailability::Unavailable(NO_GITHUB_REMOTE.to_string())
    );

    let mut state = opened();
    send(
        &mut state,
        Msg::RemotesListed(Err("not connected to the session service".to_string())),
    );
    assert_eq!(
        form(&state).github,
        GithubAvailability::Unavailable(
            "Couldn't read this repository's remotes: not connected to the session service"
                .to_string()
        )
    );
}

/// U45 — the source can be chosen only once a GitHub repository is known (invariant 1, FR-003).
#[test]
fn the_source_is_chosen_only_when_available() {
    let mut state = opened();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(
        form(&state).source,
        BranchSource::New,
        "while checking, the chip is disabled and a stray press changes nothing"
    );
    assert_eq!(form(&state).issues, IssueList::NotRequested);

    let mut state = opened();
    send(&mut state, Msg::RemotesListed(Ok(vec![])));
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(form(&state).source, BranchSource::New);
    assert_eq!(form(&state).issues, IssueList::NotRequested);

    let before = available().worktree_form.issue_request_seq;
    let (state, seq) = loading();
    assert_eq!(form(&state).source, BranchSource::Issue);
    assert!(
        seq > before,
        "a load is given a fresh seq ({seq} after {before})"
    );
    assert_eq!(state.worktree_form.issue_request_seq, seq);
}

/// U46 — only the result the form is waiting for applies (invariant 2, FR-007a).
#[test]
fn only_the_awaited_result_applies() {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq + 1, issues()));
    assert_eq!(
        awaited(&state),
        Some(seq),
        "a result for another request is dropped"
    );

    send(&mut state, loaded_result(seq, issues()));
    assert!(is_loaded(&state), "the awaited result applies");

    let (mut state, seq) = loading();
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Err(IssueLoadError::NotSignedIn),
            resolved_env: None,
        },
    );
    assert_eq!(
        form(&state).issues,
        IssueList::Failed {
            error: IssueLoadError::NotSignedIn
        }
    );

    let mut closed = State::default();
    send(&mut closed, loaded_result(1, issues()));
    assert_eq!(
        closed.worktree_form.form, None,
        "with no form open a result is dropped, not a reason to open one"
    );
}

/// U47 — Retry acts only on a failed load, and never starts a first one (invariant 1).
#[test]
fn retry_only_from_failed() {
    let mut state = available();
    send(&mut state, Msg::IssueRetry);
    assert_eq!(
        form(&state).issues,
        IssueList::NotRequested,
        "retry is not a way to start a first load"
    );

    let mut state = loaded();
    send(&mut state, Msg::IssueRetry);
    assert!(
        is_loaded(&state),
        "nothing failed, so there is nothing to retry"
    );

    let (mut state, seq) = loading();
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Err(IssueLoadError::Offline),
            resolved_env: None,
        },
    );
    send(&mut state, Msg::IssueRetry);
    let retried = awaited(&state).expect("a retry from a failure loads again");
    assert!(
        retried > seq,
        "the retry waits for a new seq, not the failed one"
    );
}

/// U48 — leaving the issue source keeps what was typed or picked, forgets the issues, and makes a
/// late result stale (invariant 7; Edge "switch mid-load", "switch back after a pick").
#[test]
fn switching_away_keeps_fields_and_drops_the_load() {
    let (mut state, seq) = loading();
    send(&mut state, Msg::TypeSelected(ConventionalType::Fix));
    send(&mut state, Msg::TicketChanged("12".into()));
    send(&mut state, Msg::NameChanged("typed".into()));

    send(&mut state, Msg::SourceChanged(BranchSource::New));
    assert_eq!(form(&state).type_, Some(ConventionalType::Fix));
    assert_eq!(form(&state).ticket, "12");
    assert_eq!(form(&state).name, "typed");
    assert_eq!(form(&state).issues, IssueList::NotRequested);

    send(&mut state, loaded_result(seq, issues()));
    assert_eq!(
        form(&state).issues,
        IssueList::NotRequested,
        "the load of a source the user left is stale"
    );

    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let again = awaited(&state).expect("returning loads afresh");
    assert!(again > seq);
}

/// U49 — zero open issues is a state of its own, not an empty picker (FR-008).
#[test]
fn no_open_issues() {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq, vec![]));
    let IssueList::Loaded { listing, .. } = &form(&state).issues else {
        panic!("an empty listing is still a loaded listing");
    };
    assert!(listing.issues.is_empty());
    assert!(listing.complete);
    assert!(form(&state).issue_matches.is_empty());
}

/// U50 — closing the form forgets the issues; reopening starts over (FR-023, Edge "form closed").
#[test]
fn closing_the_form_forgets_issues() {
    let mut state = loaded();
    send(&mut state, Msg::Cancelled);
    send(&mut state, Msg::Opened);

    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert_eq!(form(&state).github, GithubAvailability::Checking);
    assert!(form(&state).issue_matches.is_empty());
    assert_eq!(form(&state).source, BranchSource::New);
}

/// U51 — the seq outlives the form, so a closed form's late result cannot match a new form's
/// request (FR-007a, research R9).
#[test]
fn a_closed_forms_result_never_matches_a_new_form() {
    let (mut state, first) = loading();
    send(&mut state, Msg::Cancelled);
    send(&mut state, Msg::Opened);
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![remote("origin", "git@github.com:o/r.git")])),
    );
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let second = awaited(&state).expect("the new form loads");
    assert_ne!(first, second, "the seq is never reset with the form");

    send(&mut state, loaded_result(first, issues()));
    assert_eq!(
        awaited(&state),
        Some(second),
        "the old form's result must not fill the new form"
    );
}

// --- T019: ranking and the pick -------------------------------------------------------------

/// U52 — the query ranks the row text: number, label and title fragment (US1 AS3, FR-005).
#[test]
fn the_query_ranks_row_text() {
    let mut state = loaded();
    assert_eq!(
        offered(&state),
        vec![7, 42, 108],
        "no query offers every issue"
    );

    query(&mut state, "42");
    assert_eq!(
        offered(&state).first(),
        Some(&42),
        "a number finds its issue first"
    );

    query(&mut state, "documentation");
    assert_eq!(offered(&state), vec![108], "a label finds its issue");

    query(&mut state, "flicker");
    assert_eq!(offered(&state), vec![7], "a title fragment narrows");
    assert_eq!(form(&state).issue_query, "flicker");
}

/// U53 — the highlight never points past the results (invariant 3).
#[test]
fn highlight_stays_in_range() {
    let mut state = loaded();
    send(&mut state, Msg::IssueFocused);
    assert!(form(&state).issue_list_open);
    for _ in 0..5 {
        send(&mut state, Msg::IssueHighlightMoved(Direction::Next));
    }
    assert_eq!(
        form(&state).issue_highlight,
        Some(2),
        "saturates at the last row"
    );

    query(&mut state, "flicker");
    let f = form(&state);
    assert!(
        f.issue_highlight.is_none_or(|i| i < f.issue_matches.len()),
        "re-ranking under the highlight re-seats it, got {:?} over {} rows",
        f.issue_highlight,
        f.issue_matches.len()
    );

    query(&mut state, "zzzz");
    assert_eq!(form(&state).issue_highlight, None, "no rows, nowhere to be");

    send(&mut state, Msg::IssueDismissed);
    assert!(!form(&state).issue_list_open);
}

/// U54 — an index into the shown results resolves to that row's number, and only inside the
/// results (both sides of the bound).
#[test]
fn issue_number_at_bounds() {
    let mut state = loaded();
    query(&mut state, "documentation");
    assert_eq!(state.worktree_form.issue_number_at(0), Some(108));
    assert_eq!(
        state.worktree_form.issue_number_at(1),
        None,
        "one past the end is no row"
    );
    assert_eq!(State::default().worktree_form.issue_number_at(0), None);
}

/// U55 — a pick fills the ticket (no `#`) and the name from the title, clears the error and closes
/// the list (FR-009, FR-010). The type follows the mapping (U83); here the mapping is empty.
#[test]
fn a_pick_fills_ticket_and_name() {
    let mut state = loaded();
    send(&mut state, Msg::TypeSelected(ConventionalType::Chore));
    send(&mut state, Msg::Submitted);
    send(&mut state, Msg::IssueFocused);

    send(
        &mut state,
        Msg::IssuePicked {
            number: 42,
            mapping: vec![],
        },
    );

    let f = form(&state);
    assert_eq!(f.ticket, "42");
    assert_eq!(f.name, "Crash when opening empty project");
    assert_eq!(f.error, None);
    assert_eq!(f.picked_issue, Some(42));
    assert!(!f.issue_list_open, "a pick closes the list");
    assert_eq!(
        f.type_, None,
        "with no mapping entry for the issue's labels the pick clears the type (FR-014)"
    );

    send(&mut state, Msg::NameChanged("edited".into()));
    assert_eq!(
        form(&state).name,
        "edited",
        "picked values stay editable (FR-011)"
    );
}

/// U56 — a pick for a number the listing does not hold, or on a form not loaded, does nothing.
#[test]
fn a_stale_pick_is_ignored() {
    let mut state = loaded();
    send(&mut state, Msg::NameChanged("mine".into()));
    send(
        &mut state,
        Msg::IssuePicked {
            number: 9999,
            mapping: vec![],
        },
    );
    assert_eq!(form(&state).name, "mine");
    assert_eq!(form(&state).picked_issue, None);

    let (mut state, _) = loading();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 42,
            mapping: vec![],
        },
    );
    assert_eq!(form(&state).ticket, "");
    assert_eq!(form(&state).picked_issue, None);
}

/// U57 — the issue source previews and validates exactly as the new-branch source (FR-012).
#[test]
fn issue_source_previews_as_new() {
    let mut state = loaded();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 42,
            mapping: vec![],
        },
    );
    let as_issue = form(&state).clone();
    assert!(
        !as_issue.can_submit(),
        "with no type chosen the issue source refuses as the new-branch one does"
    );

    let mut as_new = as_issue.clone();
    as_new.source = BranchSource::New;
    assert_eq!(as_issue.preview(), as_new.preview());
    assert_eq!(as_issue.can_submit(), as_new.can_submit());

    send(&mut state, Msg::TypeSelected(ConventionalType::Fix));
    let as_issue = form(&state).clone();
    let mut as_new = as_issue.clone();
    as_new.source = BranchSource::New;
    assert!(as_issue.can_submit());
    assert_eq!(as_issue.preview(), as_new.preview());
    assert_eq!(
        as_issue.preview().map(|d| d.branch),
        Ok("fix/42_crash-when-opening-empty-project".to_string())
    );
}

// --- US2: the labels choose the type -----------------------------------------------------------

fn entry(label: &str, type_: ConventionalType) -> LabelTypeEntry {
    LabelTypeEntry {
        label: label.to_string(),
        type_,
    }
}

fn pick(state: &mut State, number: u64, mapping: Vec<LabelTypeEntry>) {
    send(state, Msg::IssuePicked { number, mapping });
}

/// U83 — a pick sets the type from the first matching mapping entry, replaces a selected type, and
/// clears it when no label matches, so the existing "type required" validation applies (AS1, AS3,
/// AS6, FR-013, FR-014).
#[test]
fn the_pick_sets_or_clears_the_type() {
    let mut state = loaded();
    pick(&mut state, 42, default_mapping());
    assert_eq!(
        form(&state).type_,
        Some(ConventionalType::Fix),
        "an issue labelled `bug` selects `fix` (AS1)"
    );

    send(&mut state, Msg::TypeSelected(ConventionalType::Chore));
    pick(&mut state, 108, default_mapping());
    assert_eq!(
        form(&state).type_,
        Some(ConventionalType::Docs),
        "a mapped label replaces the selected type (AS6)"
    );

    pick(&mut state, 7, default_mapping());
    assert_eq!(
        form(&state).type_,
        None,
        "no mapped label clears the type; nothing carries over (AS3, FR-014)"
    );
    assert!(
        !form(&state).can_submit(),
        "with the type cleared the form asks for one, as it does today"
    );

    pick(&mut state, 42, default_mapping());
    assert_eq!(form(&state).type_, Some(ConventionalType::Fix));
    send(&mut state, Msg::TypeSelected(ConventionalType::Refactor));
    assert_eq!(
        form(&state).type_,
        Some(ConventionalType::Refactor),
        "a type chosen by hand after the pick is kept (AS4, FR-015)"
    );
}

/// U84 — the mapping is the one handed over at the pick: a second pick under a different mapping
/// uses that mapping, and nothing recomputes the first pick's type (FR-014a).
#[test]
fn the_mapping_is_read_at_the_pick() {
    let mut state = loaded();
    pick(&mut state, 42, vec![entry("bug", ConventionalType::Chore)]);
    assert_eq!(form(&state).type_, Some(ConventionalType::Chore));

    send(&mut state, Msg::IssueQueryChanged("crash".into()));
    assert_eq!(
        form(&state).type_,
        Some(ConventionalType::Chore),
        "no later message recomputes the picked issue's type"
    );

    pick(&mut state, 42, vec![entry("BUG", ConventionalType::Perf)]);
    assert_eq!(
        form(&state).type_,
        Some(ConventionalType::Perf),
        "the next pick uses the mapping it carries"
    );
}

// --- review follow-ups ------------------------------------------------------------------------

/// U103 — leaving the issue source forgets which issue was picked, so returning never marks a row
/// the ticket and name no longer come from (Review A).
#[test]
fn leaving_the_source_forgets_the_pick() {
    let mut state = loaded();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 42,
            mapping: vec![],
        },
    );
    send(&mut state, Msg::SourceChanged(BranchSource::New));
    assert_eq!(form(&state).picked_issue, None);
    assert_eq!(
        form(&state).ticket,
        "42",
        "the filled fields themselves stay"
    );
}

/// U104 — the cap caption counts what was loaded and what is open, and is absent when every open
/// issue is held (FR-004, Review A/B).
#[test]
fn the_cap_caption_counts_the_loaded_and_the_open() {
    let state = loaded();
    assert_eq!(form(&state).issue_cap_caption(), None, "a complete listing");

    let (mut state, seq) = loading();
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                IssueListing {
                    issues: issues(),
                    total_open: 1_234,
                    complete: false,
                },
                gh(),
            )),
            resolved_env: None,
        },
    );
    assert_eq!(
        form(&state).issue_cap_caption().as_deref(),
        Some("Showing the 3 most recently updated of 1,234 open issues — search also looks on GitHub.")
    );
}

// --- T035: the search beyond the loaded issues (FR-005a, FR-007a) ---------------------------

/// A form on the issue source holding `issues()` of 1,234 open: the list is capped.
fn capped() -> State {
    capped_with(issues())
}

/// A form on the issue source holding `issues` of 1,234 open.
fn capped_with(issues: Vec<Issue>) -> State {
    let (mut state, seq) = loading();
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                IssueListing {
                    issues,
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

fn search(state: &State) -> SearchState {
    match &form(state).issues {
        IssueList::Loaded { search, .. } => search.clone(),
        other => panic!("not loaded: {other:?}"),
    }
}

fn pending_seq(state: &State) -> u64 {
    match search(state) {
        SearchState::Pending { seq } => seq,
        other => panic!("no search pending: {other:?}"),
    }
}

/// Type `text` and let its debounce fire; returns the running search's seq.
fn searching(state: &mut State, text: &str) -> u64 {
    query(state, text);
    let seq = pending_seq(state);
    send(state, Msg::IssueSearchDue { seq });
    assert_eq!(search(state), SearchState::Searching { seq });
    seq
}

/// U71 — only a capped list with something typed may reach GitHub (invariant 6).
#[test]
fn search_only_when_incomplete() {
    let mut state = loaded();
    query(&mut state, "crash");
    assert_eq!(
        search(&state),
        SearchState::Idle,
        "every open issue is held"
    );

    let mut state = capped();
    query(&mut state, "crash");
    let first = pending_seq(&state);
    query(&mut state, "crash o");
    assert!(
        pending_seq(&state) > first,
        "each keystroke restarts the wait with a fresh seq"
    );
    query(&mut state, "  ");
    assert_eq!(
        search(&state),
        SearchState::Idle,
        "nothing typed, nothing asked"
    );
}

/// U72 — the debounce acts only for the current keystroke, and only the current search's answer
/// applies (FR-007a).
#[test]
fn a_newer_keystroke_discards_an_older_search() {
    let mut state = capped();
    query(&mut state, "cra");
    let stale = pending_seq(&state);
    query(&mut state, "crash");
    send(&mut state, Msg::IssueSearchDue { seq: stale });
    assert!(
        matches!(search(&state), SearchState::Pending { .. }),
        "an older keystroke's debounce starts nothing"
    );

    let old = searching(&mut state, "crash");
    let new = searching(&mut state, "crash when");
    send(
        &mut state,
        Msg::IssueSearched {
            seq: old,
            result: Ok(vec![issue(1200, "Crash when saving", &[])]),
        },
    );
    assert_eq!(search(&state), SearchState::Searching { seq: new });
    assert!(
        !offered(&state).contains(&1200),
        "the older answer is dropped"
    );

    send(
        &mut state,
        Msg::IssueSearched {
            seq: new,
            result: Ok(vec![
                issue(1200, "Crash when saving", &[]),
                issue(42, "Crash when opening empty project", &["bug"]),
            ]),
        },
    );
    assert_eq!(search(&state), SearchState::Idle);
    let shown = offered(&state);
    assert!(
        shown.contains(&1200),
        "the searched issue joins the loaded matches"
    );
    assert_eq!(
        shown.iter().filter(|n| **n == 42).count(),
        1,
        "a searched issue already loaded is shown once (invariant 4)"
    );
    send(
        &mut state,
        Msg::IssuePicked {
            number: 1200,
            mapping: vec![],
        },
    );
    assert_eq!(
        form(&state).ticket,
        "1200",
        "a searched issue can be picked (AS10)"
    );
    assert_eq!(form(&state).name, "Crash when saving");
}

/// U73 — a failed search keeps the loaded matches, and Retry runs the search again.
#[test]
fn a_failed_search_keeps_loaded_matches() {
    let mut state = capped();
    let seq = searching(&mut state, "crash");
    let before = offered(&state);
    assert_eq!(before, vec![42]);
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Err(IssueLoadError::Offline),
        },
    );
    assert_eq!(
        search(&state),
        SearchState::Failed {
            error: IssueLoadError::Offline
        }
    );
    assert_eq!(offered(&state), before, "the loaded matches stay shown");
    assert_eq!(
        form(&state).issue_search_status().as_deref(),
        Some("Search beyond the loaded issues failed — Couldn't reach GitHub. Check your connection, then retry.")
    );

    send(&mut state, Msg::IssueRetry);
    let retried = match search(&state) {
        SearchState::Searching { seq } => seq,
        other => panic!("Retry searches again: {other:?}"),
    };
    assert!(retried > seq, "the retry is a new request");
    assert!(is_loaded(&state), "the list itself is not reloaded");
    assert_eq!(
        form(&state).issue_search_status().as_deref(),
        Some("Searching GitHub…")
    );
}

/// U70 (reducer) — a searched issue that matches the query by neither number, title nor label is
/// not offered (invariant 5).
#[test]
fn an_unmatched_searched_issue_is_hidden() {
    let mut state = capped();
    let seq = searching(&mut state, "crash");
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![
                issue(1300, "Mentions it only in the body", &[]),
                issue(1200, "Crash when saving", &[]),
            ]),
        },
    );
    let shown = offered(&state);
    assert!(shown.contains(&1200));
    assert!(
        !shown.contains(&1300),
        "GitHub matched #1300 in its body only"
    );
}

// --- M3 review A --------------------------------------------------------------------------------

/// U107 — a search answer that re-ranks the list keeps the highlight on the issue it was on, so
/// Enter never picks a row that moved under it (review A #1).
#[test]
fn a_search_answer_keeps_the_highlighted_issue() {
    // `#999999 Crash later` matches later in its row than a searched `#5 Crash`, so the answer
    // ranks above it and pushes it down a row.
    let mut state = capped_with(vec![issue(999_999, "Crash later", &[])]);
    let seq = searching(&mut state, "crash");
    send(&mut state, Msg::IssueHighlightMoved(Direction::Next));
    let before = form(&state).issue_highlight.expect("a row is highlighted");
    let highlighted = state.worktree_form.issue_number_at(before);
    assert_eq!(highlighted, Some(999_999));
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![issue(5, "Crash", &[])]),
        },
    );
    assert_eq!(offered(&state), vec![5, 999_999], "the answer ranks first");
    let after = form(&state).issue_highlight.expect("still highlighted");
    assert_eq!(
        state.worktree_form.issue_number_at(after),
        highlighted,
        "the highlight follows #999999, wherever it now ranks"
    );
}

/// U108 — clearing the query forgets the last search's issues, so an empty query offers the loaded
/// list alone (review A #4).
#[test]
fn clearing_the_query_forgets_searched_issues() {
    let mut state = capped();
    let seq = searching(&mut state, "crash");
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![issue(1200, "Crash when saving", &[])]),
        },
    );
    query(&mut state, "");
    assert_eq!(offered(&state), vec![7, 42, 108]);
}

/// U109 — a keystroke that leaves the trimmed text as it was asks GitHub nothing new (review A #6).
#[test]
fn whitespace_alone_does_not_search_again() {
    let mut state = capped();
    let seq = searching(&mut state, "crash");
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![issue(1200, "Crash when saving", &[])]),
        },
    );
    query(&mut state, "crash ");
    assert_eq!(search(&state), SearchState::Idle);
    assert!(
        offered(&state).contains(&1200),
        "the answer for `crash` stays"
    );
}

/// U110 — a debounce that ends while a create is running still searches: the keystroke was the
/// named event, and a search left pending would never run once the form is back to editing
/// (review A round 2 #1, superseding round 1 #7).
#[test]
fn a_search_due_while_creating_is_not_left_pending() {
    let mut state = capped();
    query(&mut state, "crash");
    let seq = pending_seq(&state);
    send(
        &mut state,
        Msg::CreateStarted(micold_core::worktree::CreateMode::default()),
    );
    send(&mut state, Msg::IssueSearchDue { seq });
    assert_eq!(search(&state), SearchState::Searching { seq });
}

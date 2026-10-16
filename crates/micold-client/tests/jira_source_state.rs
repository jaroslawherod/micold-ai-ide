//! The Jira issue source of the add-worktree form, as state transitions (feature 533,
//! data-model §4). The Jira twin of `issue_source_state.rs`: the provider, the opt-in, what a
//! pick writes and what a failure shows are all decided by the reducer, so all of it is asserted
//! here with no network. What the shell does with the token file is in `jira_connection_shell.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::{
    BranchSource, DescriptionPass, IssueList, JiraAvailability, JiraSite, Msg, SearchState,
    WorktreeForm,
};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing, IssueLoadError};
use micold_core::issue_types::default_mapping;
use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
use micold_core::naming::ConventionalType;

fn connection() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn project() -> PathBuf {
    PathBuf::from("/work/acme")
}

fn connected() -> JiraAvailability {
    JiraAvailability::Connected(JiraSite::of(&connection()))
}

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.form.as_ref().expect("the form is open")
}

/// A state whose settings have the switch on and a connection for `project()`.
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

fn issues() -> Vec<Issue> {
    vec![
        issue("ABC-123", 123, "Add login rate limit", &[], "Bug"),
        issue("ABC-9", 9, "Tidy the footer", &["ui"], "Task"),
    ]
}

fn listing(complete: bool) -> IssueListing {
    IssueListing {
        total_open: 2,
        complete,
        issues: issues(),
    }
}

fn loaded_result(seq: u64, complete: bool) -> Msg {
    Msg::IssuesLoaded {
        seq,
        result: Ok((listing(complete), PathBuf::new())),
        resolved_env: None,
    }
}

fn awaited(state: &State) -> u64 {
    match form(state).issues {
        IssueList::Loading { seq } => seq,
        ref other => panic!("expected a load in flight, found {other:?}"),
    }
}

/// The Jira form with its load in flight.
fn loading() -> (State, u64) {
    let mut state = jira_form();
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let seq = awaited(&state);
    (state, seq)
}

fn loaded() -> State {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq, true));
    state
}

#[test]
fn availability_derives_off_or_connected_from_the_settings() {
    let connections: BTreeMap<String, JiraConnection> =
        [(connection_key(&project()), connection())].into();
    let derive = |enabled, active: Option<&Path>| {
        JiraAvailability::from_settings(enabled, &connections, active)
    };
    assert_eq!(derive(false, Some(&project())), JiraAvailability::Off);
    assert_eq!(derive(true, None), JiraAvailability::Off);
    assert_eq!(
        derive(true, Some(Path::new("/other"))),
        JiraAvailability::Off
    );
    assert_eq!(derive(true, Some(&project())), connected());
}

#[test]
fn a_project_without_a_github_remote_lists_jira_and_offers_no_source_choice() {
    let state = jira_form();
    assert_eq!(form(&state).provider, IssueProvider::Jira);
    assert!(form(&state).issue_source_ready());
    assert_eq!(
        form(&state).source_caption().as_deref(),
        Some("Reads open issues of project ABC from https://acme.atlassian.net. Sends the project key, your search text and your credentials.")
    );
}

#[test]
fn a_project_with_a_github_remote_keeps_the_github_picker() {
    let mut state = settings_with_jira();
    send(&mut state, Msg::Opened);
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    assert_eq!(form(&state).provider, IssueProvider::Github);
    assert!(form(&state).jira_site().is_none());
}

#[test]
fn issues_leave_not_requested_only_on_choosing_the_source_while_connected() {
    let mut state = jira_form();
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    assert!(matches!(form(&state).issues, IssueList::Loading { .. }));

    // Switch off: nothing to choose, nothing requested.
    let mut off = State::default();
    send(&mut off, Msg::Opened);
    send(&mut off, Msg::RemotesListed(Ok(Vec::new())));
    send(&mut off, Msg::SourceChanged(BranchSource::Issue));
    assert_eq!(form(&off).issues, IssueList::NotRequested);
    assert_eq!(form(&off).source, BranchSource::New);
}

#[test]
fn switching_off_resets_the_list_and_a_late_result_drops() {
    let (mut state, seq) = loading();
    let before = state.worktree_form.issue_request_seq;
    send(&mut state, Msg::JiraSettingsChanged(JiraAvailability::Off));
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert_eq!(form(&state).source, BranchSource::New);
    assert_eq!(form(&state).provider, IssueProvider::Github);
    assert!(form(&state).issue_query.is_empty());
    assert!(state.worktree_form.issue_request_seq > before);
    send(&mut state, loaded_result(seq, true));
    assert_eq!(form(&state).issues, IssueList::NotRequested);
    assert!(state.worktree_form.awaited_issue_load().is_none());
}

#[test]
fn a_jira_listing_never_starts_a_description_pass() {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq, false));
    let IssueList::Loaded { descriptions, .. } = &form(&state).issues else {
        panic!("loaded");
    };
    assert_eq!(*descriptions, DescriptionPass::Done);
    assert!(state.worktree_form.issue_description_request().is_none());
    assert_eq!(
        form(&state).issue_cap_caption().as_deref(),
        Some("Showing the 2 most recently updated open issues.")
    );
}

fn capped() -> State {
    let (mut state, seq) = loading();
    send(&mut state, loaded_result(seq, false));
    state
}

fn search(state: &State) -> SearchState {
    match &form(state).issues {
        IssueList::Loaded { search, .. } => search.clone(),
        other => panic!("not loaded: {other:?}"),
    }
}

fn query(state: &mut State, text: &str) {
    send(state, Msg::IssueQueryChanged(text.into()));
}

fn shown(state: &State) -> Vec<u64> {
    (0..form(state).issue_matches.len())
        .map(|row| form(state).issue_number_at(row).unwrap())
        .collect()
}

/// Type `text` and let its debounce fire; returns the running search's seq.
fn searching(state: &mut State, text: &str) -> u64 {
    query(state, text);
    let SearchState::Pending { seq } = search(state) else {
        panic!("no search pending: {:?}", search(state));
    };
    send(state, Msg::IssueSearchDue { seq });
    assert_eq!(search(state), SearchState::Searching { seq });
    seq
}

/// US1 AS2 — the loaded issues narrow by key, summary and label, ignoring case.
#[test]
fn typing_narrows_by_key_summary_and_label_ignoring_case() {
    let mut state = loaded();
    for (text, expected) in [
        ("abc-123", vec![123]),
        ("LOGIN", vec![123]),
        ("Rate Limit", vec![123]),
        ("UI", vec![9]),
        ("nothing like it", vec![]),
    ] {
        query(&mut state, text);
        assert_eq!(shown(&state), expected, "for {text:?}");
    }
    query(&mut state, "");
    assert_eq!(shown(&state), vec![123, 9]);
}

/// Invariant 6 — only a capped list with something typed may reach Jira.
#[test]
fn search_only_when_incomplete_and_something_is_typed() {
    let mut state = loaded();
    query(&mut state, "login");
    assert_eq!(
        search(&state),
        SearchState::Idle,
        "every open issue is held"
    );

    let mut state = capped();
    query(&mut state, "login");
    let SearchState::Pending { seq: first } = search(&state) else {
        panic!("a capped list waits out the debounce");
    };
    query(&mut state, "login r");
    let SearchState::Pending { seq: second } = search(&state) else {
        panic!("still pending");
    };
    assert!(second > first, "each keystroke restarts the wait");
    send(&mut state, Msg::IssueSearchDue { seq: first });
    assert!(matches!(search(&state), SearchState::Pending { .. }));
    query(&mut state, "  ");
    assert_eq!(
        search(&state),
        SearchState::Idle,
        "nothing typed, nothing asked"
    );
}

/// A running search asks Jira for the trimmed text; an answer merges de-duplicated by number and
/// only the issues that match the query locally are shown (FR-012).
#[test]
fn a_search_answer_merges_without_duplicates_and_keeps_local_matches() {
    let mut state = capped();
    let seq = searching(&mut state, " login ");
    assert_eq!(
        state.worktree_form.jira_search_text().as_deref(),
        Some("login")
    );
    assert_eq!(
        form(&state).issue_search_status().as_deref(),
        Some("Searching Jira…")
    );
    let beyond = issue("ABC-2001", 2001, "Login page flashes", &[], "Bug");
    let unrelated = issue("ABC-2002", 2002, "Unrelated", &[], "Task");
    let duplicate = issue("ABC-123", 123, "Add login rate limit", &[], "Bug");
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Ok(vec![beyond, unrelated, duplicate]),
        },
    );
    assert_eq!(search(&state), SearchState::Idle);
    assert_eq!(shown(&state).len(), 2);
    assert!(shown(&state).contains(&123) && shown(&state).contains(&2001));
}

/// FR-007a — an older search's answer is dropped.
#[test]
fn a_newer_keystroke_discards_an_older_search_answer() {
    let mut state = capped();
    let old = searching(&mut state, "login");
    let new = searching(&mut state, "login r");
    send(
        &mut state,
        Msg::IssueSearched {
            seq: old,
            result: Ok(vec![issue("ABC-2001", 2001, "Login", &[], "Bug")]),
        },
    );
    assert_eq!(search(&state), SearchState::Searching { seq: new });
    assert!(!shown(&state).contains(&2001));
}

/// A failed search leaves the loaded matches and says so; retry searches again.
#[test]
fn a_failed_search_keeps_the_loaded_matches_and_retry_searches_again() {
    let mut state = capped();
    let seq = searching(&mut state, "login");
    send(
        &mut state,
        Msg::IssueSearched {
            seq,
            result: Err(IssueLoadError::RateLimited),
        },
    );
    assert_eq!(shown(&state), vec![123]);
    assert!(form(&state)
        .issue_search_status()
        .is_some_and(|line| line.starts_with("Search beyond the loaded issues failed")));
    send(&mut state, Msg::IssueRetry);
    assert!(state.worktree_form.jira_search_text().is_some());
}

#[test]
fn a_pick_fills_ticket_name_and_type_and_a_hand_edit_stands() {
    let mut state = loaded();
    send(
        &mut state,
        Msg::IssuePicked {
            number: 123,
            mapping: default_mapping(),
        },
    );
    assert_eq!(form(&state).ticket, "ABC-123");
    assert_eq!(form(&state).name, "Add login rate limit");
    assert_eq!(form(&state).type_, Some(ConventionalType::Fix));
    send(&mut state, Msg::TicketChanged("ABC-124".into()));
    assert_eq!(form(&state).ticket, "ABC-124");
    send(
        &mut state,
        Msg::IssuePicked {
            number: 9,
            mapping: default_mapping(),
        },
    );
    assert_eq!(form(&state).ticket, "ABC-9");
}

#[test]
fn a_failure_shows_its_notice_and_retry_reloads_with_the_inputs_live() {
    let (mut state, seq) = loading();
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Err(IssueLoadError::CredentialsRefused),
            resolved_env: None,
        },
    );
    let IssueList::Failed { error } = &form(&state).issues else {
        panic!("failed");
    };
    let site = form(&state).notice_source().expect("a source");
    let text = error.message(site);
    assert!(text.contains("Settings"), "{text}");
    send(&mut state, Msg::NameChanged("by hand".into()));
    assert_eq!(form(&state).name, "by hand");
    send(&mut state, Msg::IssueRetry);
    assert!(matches!(form(&state).issues, IssueList::Loading { .. }));
}

#[test]
fn a_site_that_is_not_reachable_names_the_host() {
    let (state, _) = loading();
    let source = form(&state).notice_source().expect("a source");
    let text = IssueLoadError::Unreachable("dns error".into()).message(source);
    assert_eq!(text, "Couldn't reach acme.atlassian.net: dns error.");
}

// ---- Feature 533, US4: the source choice of a project with both (FR-016, FR-017) ----

fn github_remote() -> Vec<GitRemote> {
    vec![GitRemote {
        name: "origin".into(),
        url: "git@github.com:o/r.git".into(),
    }]
}

/// A project with a GitHub remote and a Jira connection, `remembered` already chosen in it.
fn both_form(remembered: Option<IssueProvider>) -> State {
    let mut state = settings_with_jira();
    if let Some(provider) = remembered {
        state.workspace.issue_sources.insert(project(), provider);
    }
    send(&mut state, Msg::Opened);
    send(&mut state, Msg::RemotesListed(Ok(github_remote())));
    state
}

#[test]
fn both_sources_offer_the_choice_and_open_on_the_remembered_one_else_github() {
    let fresh = both_form(None);
    assert!(form(&fresh).offers_issue_provider_choice());
    assert_eq!(form(&fresh).provider, IssueProvider::Github);

    let jira = both_form(Some(IssueProvider::Jira));
    assert!(form(&jira).offers_issue_provider_choice());
    assert_eq!(form(&jira).provider, IssueProvider::Jira);

    let github = both_form(Some(IssueProvider::Github));
    assert_eq!(form(&github).provider, IssueProvider::Github);
}

#[test]
fn a_single_source_offers_no_choice() {
    // Jira only, GitHub only, the switch off: the 034 dialog or the Jira-only one (US4 AS3, AS4).
    assert!(!form(&jira_form()).offers_issue_provider_choice());
    let mut github_only = State::default();
    github_only.workspace.active = Some(project());
    send(&mut github_only, Msg::Opened);
    send(&mut github_only, Msg::RemotesListed(Ok(github_remote())));
    assert!(!form(&github_only).offers_issue_provider_choice());
    let mut no_connection = State::default();
    no_connection.workspace.active = Some(project());
    no_connection.settings.jira_enabled = true;
    send(&mut no_connection, Msg::Opened);
    send(&mut no_connection, Msg::RemotesListed(Ok(github_remote())));
    assert!(!form(&no_connection).offers_issue_provider_choice());
    // A remembered Jira does not select Jira once it is not connected.
    no_connection
        .workspace
        .issue_sources
        .insert(project(), IssueProvider::Jira);
    send(&mut no_connection, Msg::Opened);
    send(&mut no_connection, Msg::RemotesListed(Ok(github_remote())));
    assert_eq!(form(&no_connection).provider, IssueProvider::Github);
}

#[test]
fn choosing_a_provider_resets_the_list_drops_the_other_result_and_is_remembered() {
    let mut state = both_form(None);
    send(&mut state, Msg::SourceChanged(BranchSource::Issue));
    let stale = awaited(&state);
    send(&mut state, Msg::IssueQueryChanged("login".into()));
    let before = state.worktree_form.issue_request_seq;

    let outcomes = micold_client::features::worktree_form::update(
        &mut state,
        Msg::IssueProviderChanged(IssueProvider::Jira),
    );
    assert_eq!(
        outcomes,
        vec![micold_client::features::Outcome::IssueSourceChosen {
            project: project(),
            provider: IssueProvider::Jira,
        }]
    );
    assert_eq!(form(&state).provider, IssueProvider::Jira);
    assert!(form(&state).issue_query.is_empty());
    assert!(form(&state).issue_matches.is_empty());
    assert!(state.worktree_form.issue_request_seq > before);
    let fresh = awaited(&state);
    assert_ne!(fresh, stale);

    // The GitHub load that was in flight answers late and is dropped.
    send(&mut state, loaded_result(stale, true));
    assert_eq!(form(&state).issues, IssueList::Loading { seq: fresh });
    send(&mut state, loaded_result(fresh, true));
    assert!(matches!(form(&state).issues, IssueList::Loaded { .. }));

    // Through the root the choice is also held for the next open of the dialog.
    let mut root = both_form(None);
    root.update(Message::WorktreeForm(Msg::IssueProviderChanged(
        IssueProvider::Jira,
    )));
    assert_eq!(root.workspace.issue_sources[&project()], IssueProvider::Jira);
    send(&mut root, Msg::Cancelled);
    send(&mut root, Msg::Opened);
    send(&mut root, Msg::RemotesListed(Ok(github_remote())));
    assert_eq!(form(&root).provider, IssueProvider::Jira);
}

#[test]
fn jira_is_never_chosen_without_a_connected_site_and_a_repeat_changes_nothing() {
    // Integration off: the choice is refused.
    let mut off = State::default();
    off.workspace.active = Some(project());
    send(&mut off, Msg::Opened);
    send(&mut off, Msg::RemotesListed(Ok(github_remote())));
    let outcomes = micold_client::features::worktree_form::update(
        &mut off,
        Msg::IssueProviderChanged(IssueProvider::Jira),
    );
    assert!(outcomes.is_empty());
    assert_eq!(form(&off).provider, IssueProvider::Github);

    // Choosing the provider already chosen is a no-op: no seq bump, no outcome.
    let mut state = both_form(None);
    let before = state.worktree_form.issue_request_seq;
    let outcomes = micold_client::features::worktree_form::update(
        &mut state,
        Msg::IssueProviderChanged(IssueProvider::Github),
    );
    assert!(outcomes.is_empty());
    assert_eq!(state.worktree_form.issue_request_seq, before);
}

#[test]
fn switching_the_connection_off_in_settings_drops_a_remembered_jira_choice_from_the_form() {
    let mut state = both_form(Some(IssueProvider::Jira));
    send(&mut state, Msg::JiraSettingsChanged(JiraAvailability::Off));
    assert_eq!(form(&state).provider, IssueProvider::Github);
    assert!(!form(&state).offers_issue_provider_choice());
}

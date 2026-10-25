//! Filter choices and the filtered source (feature 1185, contracts/jira-filter-source.md §2 and
//! §4): account ids, redaction, parsing a project's components and versions, and loading a
//! filtered listing
//! through `FakeJiraSource`.

use micold_core::github::{Issue, IssueLoadError, IssuePage, ISSUE_LOAD_CAP};
use micold_core::jira::{
    load_filtered, ApiToken, ChoiceCall, FakeJiraSource, FilteredAsk, JiraConnection, JiraSource,
    FILTERS_UNSUPPORTED,
};
use micold_core::jira_filter::{
    parse_components, parse_versions, AccountId, JiraFilters, JiraUser, VersionChoice,
};

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.example").expect("valid")
}

fn token() -> ApiToken {
    ApiToken::parse("secret").expect("a token")
}

fn issue(n: u64) -> Issue {
    Issue::jira(
        &format!("ABC-{n}"),
        n,
        format!("Issue {n}"),
        vec![],
        String::new(),
    )
}

fn page(first: u64, len: u64, more: bool) -> IssuePage {
    IssuePage {
        issues: (first..first + len).map(issue).collect(),
        total_open: len,
        next_cursor: more.then(|| format!("tok-{}", first + len - 1)),
    }
}

fn user(account: &str, name: &str) -> JiraUser {
    JiraUser {
        account: AccountId::parse(account).expect("an account id"),
        name: name.to_owned(),
        is_me: false,
    }
}

fn component(name: &str) -> JiraFilters {
    let mut filters = JiraFilters::default();
    assert!(filters.set_component(Some(name.into())));
    filters
}

#[test]
fn an_account_id_is_non_empty_without_control_characters() {
    assert!(AccountId::parse("5b10ac8d82e05b22cc7d4ef5").is_some());
    assert!(AccountId::parse("qm:1:2").is_some());
    assert!(AccountId::parse("").is_none());
    assert!(AccountId::parse("a\nb").is_none());
    assert!(AccountId::parse("\u{1b}").is_none());
}

#[test]
fn debug_never_prints_a_name_or_an_account() {
    let ann = user("acc-secret-1", "Ann Secret");
    assert_eq!(format!("{:?}", ann.account), "AccountId(<redacted>)");
    assert_eq!(format!("{ann:?}"), "JiraUser(<redacted>)");
    let mut filters = JiraFilters::default();
    assert!(filters.set_reporter(Some(ann)));
    let printed = format!("{filters:?}");
    assert!(!printed.contains("Ann"), "{printed}");
    assert!(!printed.contains("acc-secret-1"), "{printed}");
}

#[test]
fn users_are_equal_by_account_only() {
    assert_ne!(user("acc-1", "Ann"), user("acc-2", "Ann"));
    assert_eq!(user("acc-1", "Ann"), user("acc-1", "Ann B."));
}

#[test]
fn a_filtered_load_pages_by_cursor_and_records_the_clauses() {
    let source = FakeJiraSource::new()
        .with_page(page(1, 2, true))
        .with_page(page(3, 1, false));
    let filters = component("Payments");
    let listing = load_filtered(&source, &conn(), &token(), &filters).expect("loaded");
    let keys: Vec<_> = listing.issues.iter().filter_map(Issue::key).collect();
    assert_eq!(keys, ["ABC-1", "ABC-2", "ABC-3"]);
    assert!(listing.complete);
    let clauses = vec![r#"component = "Payments""#.to_string()];
    assert_eq!(
        source.filtered_calls(),
        [
            (clauses.clone(), FilteredAsk::List(None)),
            (clauses, FilteredAsk::List(Some("tok-2".into()))),
        ]
    );
}

#[test]
fn a_filtered_load_stops_at_the_cap_and_is_incomplete() {
    let mut source = FakeJiraSource::new();
    for n in 0..11 {
        source = source.with_page(page(n * 100 + 1, 100, true));
    }
    let listing =
        load_filtered(&source, &conn(), &token(), &component("Payments")).expect("loaded");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(!listing.complete);
    assert_eq!(source.filtered_calls().len(), ISSUE_LOAD_CAP / 100);
}

#[test]
fn a_filtered_search_records_its_clauses_and_text() {
    let source = FakeJiraSource::new().with_search(Ok(vec![issue(7)]));
    let found = source
        .search_filtered(&conn(), &token(), "refund", &component("Payments"))
        .expect("found");
    assert_eq!(found, [issue(7)]);
    assert_eq!(
        source.filtered_calls(),
        [(
            vec![r#"component = "Payments""#.to_string()],
            FilteredAsk::Search("refund".into())
        )]
    );
}

/// A source that implements only the unfiltered methods.
struct Unfiltered;

impl JiraSource for Unfiltered {
    fn list_open(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        _cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        Ok(page(1, 1, false))
    }

    fn search_open(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        _text: &str,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        Ok(vec![issue(9)])
    }
}

#[test]
fn a_source_without_filters_answers_unfiltered_requests_and_refuses_filtered_ones() {
    let none = JiraFilters::default();
    assert_eq!(
        Unfiltered.list_filtered(&conn(), &token(), &none, None),
        Ok(page(1, 1, false))
    );
    assert_eq!(
        Unfiltered.search_filtered(&conn(), &token(), "x", &none),
        Ok(vec![issue(9)])
    );
    let refused = Err(IssueLoadError::Other(FILTERS_UNSUPPORTED.into()));
    assert_eq!(
        Unfiltered
            .list_filtered(&conn(), &token(), &component("C"), None)
            .map(|_| ()),
        refused
    );
    assert_eq!(
        Unfiltered
            .search_filtered(&conn(), &token(), "x", &component("C"))
            .map(|_| ()),
        refused
    );
    assert_eq!(Unfiltered.components(&conn(), &token()), Ok(vec![]));
    assert_eq!(Unfiltered.versions(&conn(), &token()), Ok(vec![]));
    assert_eq!(
        FILTERS_UNSUPPORTED,
        "filters are not supported by this source"
    );
}

#[test]
fn every_combination_lists_exactly_the_issues_scripted_for_its_clauses() {
    for mask in 0u8..16 {
        let mut filters = JiraFilters::default();
        if mask & 1 != 0 {
            assert!(filters.set_reporter(Some(user("acc-1", "Ann"))));
        }
        if mask & 2 != 0 {
            assert!(filters.set_label(Some("backend".into())));
        }
        if mask & 4 != 0 {
            assert!(filters.set_component(Some("Payments".into())));
        }
        if mask & 8 != 0 {
            assert!(filters.set_fix_version(Some("2.4".into())));
        }
        let scripted = page(u64::from(mask) * 10 + 1, u64::from(mask) + 1, false);
        let source = FakeJiraSource::new().with_page(scripted.clone());
        let listing = load_filtered(&source, &conn(), &token(), &filters).expect("loaded");
        assert_eq!(listing.issues, scripted.issues, "combination {mask:04b}");
        assert_eq!(
            source.filtered_calls(),
            [(filters.clauses(), FilteredAsk::List(None))],
            "combination {mask:04b}"
        );
    }
}

#[test]
fn components_are_read_in_jiras_order_without_duplicates_or_unusable_names() {
    let body = r#"[
        {"name": "Payments", "id": "1"},
        {"name": "Auth"},
        {"name": "Payments"},
        {"name": ""},
        {"name": "bad\u0007name"},
        {"id": "no name"},
        {"name": "Ä \"quoted\" \\ name"}
    ]"#
    .as_bytes();
    assert_eq!(
        parse_components(body),
        Ok(vec![
            "Payments".to_string(),
            "Auth".to_string(),
            "Ä \"quoted\" \\ name".to_string(),
        ])
    );
    assert_eq!(parse_components(b"[]"), Ok(vec![]));
}

#[test]
fn a_components_body_of_another_shape_is_not_jira_cloud() {
    for body in [&b"<html>"[..], b"{\"values\": []}", b"\"Payments\"", b""] {
        assert_eq!(parse_components(body), Err(IssueLoadError::NotJiraCloud));
    }
}

#[test]
fn the_fake_answers_scripted_components_and_records_the_request() {
    let source = FakeJiraSource::new()
        .with_components(Ok(vec!["Payments".into()]))
        .with_components(Err(IssueLoadError::TimedOut));
    assert_eq!(
        source.components(&conn(), &token()),
        Ok(vec!["Payments".to_string()])
    );
    assert_eq!(
        source.components(&conn(), &token()),
        Err(IssueLoadError::TimedOut)
    );
    assert_eq!(
        source.choice_calls(),
        [ChoiceCall::Components, ChoiceCall::Components]
    );
}

fn version(name: &str, released: bool) -> VersionChoice {
    VersionChoice::new(name, released).expect("usable name")
}

#[test]
fn a_version_choice_is_non_empty_without_control_characters() {
    let choice = version("2.4", false);
    assert_eq!(choice.name(), "2.4");
    assert!(!choice.released());
    assert!(version("2.3", true).released());
    assert_eq!(VersionChoice::new("", false), None);
    assert_eq!(VersionChoice::new("2.\u{7}4", true), None);
    assert_eq!(VersionChoice::new("line\nbreak", false), None);
}

#[test]
fn versions_drop_archived_and_list_unreleased_in_jiras_order_then_released_newest_first() {
    let body = r#"[
        {"name": "1.0", "archived": true, "released": true},
        {"name": "2.2", "archived": false, "released": true},
        {"name": "2.3", "archived": false, "released": true},
        {"name": "2.4", "archived": false, "released": false},
        {"name": "3.0", "archived": false, "released": false},
        {"name": "0.9", "archived": true, "released": false}
    ]"#
    .as_bytes();
    assert_eq!(
        parse_versions(body),
        Ok(vec![
            version("2.4", false),
            version("3.0", false),
            version("2.3", true),
            version("2.2", true),
        ])
    );
    assert_eq!(parse_versions(b"[]"), Ok(vec![]));
}

#[test]
fn versions_skip_duplicates_unusable_names_and_entries_without_a_name() {
    let body = r#"[
        {"name": "2.4", "archived": false, "released": false},
        {"name": "2.4", "archived": false, "released": true},
        {"name": "", "archived": false, "released": false},
        {"name": "bad\u0007name", "archived": false, "released": false},
        {"archived": false, "released": false},
        {"name": "Ä \"q\" \\ v"}
    ]"#
    .as_bytes();
    assert_eq!(
        parse_versions(body),
        Ok(vec![version("2.4", false), version("Ä \"q\" \\ v", false)])
    );
}

#[test]
fn a_versions_body_of_another_shape_is_not_jira_cloud() {
    for body in [&b"<html>"[..], b"{\"values\": []}", b"\"2.4\"", b""] {
        assert_eq!(parse_versions(body), Err(IssueLoadError::NotJiraCloud));
    }
}

#[test]
fn the_fake_answers_scripted_versions_and_records_the_request() {
    let source = FakeJiraSource::new()
        .with_versions(Ok(vec![version("2.4", false)]))
        .with_versions(Err(IssueLoadError::TimedOut));
    assert_eq!(
        source.versions(&conn(), &token()),
        Ok(vec![version("2.4", false)])
    );
    assert_eq!(
        source.versions(&conn(), &token()),
        Err(IssueLoadError::TimedOut)
    );
    assert_eq!(
        source.choice_calls(),
        [ChoiceCall::Versions, ChoiceCall::Versions]
    );
}

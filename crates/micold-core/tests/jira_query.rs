//! The advanced-JQL rules and load (feature 1186, specs/1186-jira-advanced-jql/contracts/jql-rules.md).
//!
//! Cleaning, parsing and scoping (§1–§3), Jira's messages (§4), the texts (§5) and the advanced
//! load (§6), the last driven through `FakeJiraSource::with_query`.

use micold_core::github::{Issue, IssueLoadError, IssuePage, ISSUE_LOAD_CAP};
use micold_core::jira::{ApiToken, FakeJiraSource, JiraConnection, ProjectKey};
use micold_core::jira_query::{
    load, rejection_messages, QueryFailure, QueryMode, QueryRefusal, ScopedQuery, UserQuery,
    CAPPED, EMPTY, NOT_VALID_JQL, PROMPT,
};
use micold_core::jira_status::TicketStatus;

fn parsed(raw: &str) -> UserQuery {
    UserQuery::parse(raw).unwrap_or_else(|refusal| panic!("{raw:?} refused: {refusal:?}"))
}

fn refusal(raw: &str) -> QueryRefusal {
    UserQuery::parse(raw).expect_err(raw)
}

fn key() -> ProjectKey {
    ProjectKey::parse("ABC").expect("a key")
}

fn scoped(raw: &str) -> String {
    ScopedQuery::new(&key(), &parsed(raw)).as_str().to_string()
}

// §1 Cleaning

#[test]
fn cleaning_turns_line_breaks_and_tabs_into_spaces_drops_other_controls_and_trims() {
    for (raw, cleaned) in [
        ("  type = Bug\n", "type = Bug"),
        ("a\u{0007}b", "ab"),
        ("a\tAND\nb", "a AND b"),
        ("a\r\nb", "a  b"),
        ("Type  =  BUG", "Type  =  BUG"),
    ] {
        assert_eq!(
            parsed(raw).as_str(),
            cleaned,
            "cleaning changes only control characters and the ends ({raw:?})"
        );
    }
}

// §2 Parse

#[test]
fn the_empty_text_parses_with_no_condition_and_no_sort() {
    for raw in ["", "   ", "\n\t"] {
        let query = parsed(raw);
        assert_eq!(query.as_str(), "", "an empty query is a query ({raw:?})");
        assert_eq!(query.condition(), "");
        assert_eq!(query.sort_clause(), None);
    }
}

#[test]
fn a_query_without_a_sort_clause_is_all_condition() {
    let query = parsed("assignee = currentUser() AND priority = High");
    assert_eq!(
        query.condition(),
        "assignee = currentUser() AND priority = High"
    );
    assert_eq!(query.sort_clause(), None, "no ORDER BY, no sort clause");
}

#[test]
fn the_sort_clause_starts_at_the_first_order_by_outside_quoted_strings() {
    for (raw, condition, sort) in [
        (
            "status = \"In Review\" ORDER BY priority DESC, created ASC",
            "status = \"In Review\"",
            "ORDER BY priority DESC, created ASC",
        ),
        ("ORDER BY created ASC", "", "ORDER BY created ASC"),
        (
            "type = Bug order  by created",
            "type = Bug",
            "order  by created",
        ),
        (
            "type = Bug ORDER\nBY created",
            "type = Bug",
            "ORDER BY created",
        ),
        (
            "summary ~ \"order by\" ORDER BY created",
            "summary ~ \"order by\"",
            "ORDER BY created",
        ),
        (
            "x = 1 ORDER BY a ORDER BY b",
            "x = 1",
            "ORDER BY a ORDER BY b",
        ),
        ("(x = 1)ORDER BY a", "(x = 1)", "ORDER BY a"),
    ] {
        let query = parsed(raw);
        assert_eq!(query.condition(), condition, "condition of {raw:?}");
        assert_eq!(query.sort_clause(), Some(sort), "sort clause of {raw:?}");
    }
}

#[test]
fn order_by_inside_a_string_a_word_or_parentheses_is_not_a_sort_clause() {
    for raw in [
        "summary ~ \"order by\"",
        "summary ~ 'order by'",
        "reorder by = 1",
        "x ORDER BYTES",
        "x ORDER_BY y",
        "x ORDERBY y",
        "order",
        "(a ORDER BY b) OR (project = X)",
    ] {
        let query = parsed(raw);
        assert_eq!(query.sort_clause(), None, "{raw:?} has no sort clause");
        assert_eq!(query.condition(), raw, "{raw:?} is all condition");
    }
}

#[test]
fn quoted_parentheses_and_escaped_quotes_are_text() {
    for raw in [
        "summary ~ 'a)'",
        "summary ~ \"a\\\"b)\"",
        "summary ~ \"it's (x\"",
        "summary ~ 'say \"(\"'",
    ] {
        assert!(
            UserQuery::parse(raw).is_ok(),
            "a parenthesis or the other quote inside a string is text ({raw:?})"
        );
    }
}

#[test]
fn an_unclosed_quote_is_refused() {
    for raw in ["summary ~ \"abc", "summary ~ 'abc", "summary ~ \"a\\\""] {
        assert_eq!(
            refusal(raw),
            QueryRefusal::UnclosedQuote,
            "the text ends inside a string ({raw:?})"
        );
    }
}

#[test]
fn parentheses_that_do_not_balance_outside_strings_are_refused() {
    for raw in [
        "x = 1) OR project = OTHER OR (x = 1",
        "(x = 1",
        "x = 1 ORDER BY created) OR (project = X",
        ")",
        "summary ~ \"(\" AND (x = 1",
    ] {
        assert_eq!(
            refusal(raw),
            QueryRefusal::UnbalancedParentheses,
            "a query that could leave the scope is refused ({raw:?})"
        );
    }
}

#[test]
fn the_limit_is_two_thousand_chars_after_cleaning() {
    assert_eq!(
        refusal(&"a".repeat(2_001)),
        QueryRefusal::TooLong,
        "2,001 characters are over the limit"
    );
    let at_limit = parsed(&format!("{}\n", "a".repeat(2_000)));
    assert_eq!(
        at_limit.as_str().chars().count(),
        2_000,
        "the trailing line break is cleaned before counting"
    );
    assert!(
        UserQuery::parse(&"é".repeat(2_000)).is_ok(),
        "the limit counts characters, not bytes"
    );
    assert_eq!(refusal(&"é".repeat(2_001)), QueryRefusal::TooLong);
}

#[test]
fn too_long_comes_before_an_unclosed_quote_and_a_quote_before_parentheses() {
    assert_eq!(
        refusal(&format!("({}\"", "a".repeat(2_000))),
        QueryRefusal::TooLong,
        "the limit is checked before the scan"
    );
    assert_eq!(
        refusal("(x = \"a"),
        QueryRefusal::UnclosedQuote,
        "an unclosed quote outranks an open parenthesis"
    );
    assert_eq!(
        refusal("x) = \"a"),
        QueryRefusal::UnclosedQuote,
        "an unclosed quote outranks a closing parenthesis too many"
    );
}

// §3 The scoped query

#[test]
fn the_scoped_query_wraps_the_condition_and_keeps_or_adds_the_sort() {
    for (raw, jql) in [
        (
            "type = Bug ORDER BY created ASC",
            "project = \"ABC\" AND (type = Bug) ORDER BY created ASC",
        ),
        (
            "type = Bug",
            "project = \"ABC\" AND (type = Bug) ORDER BY updated DESC",
        ),
        (
            "ORDER BY created ASC",
            "project = \"ABC\" ORDER BY created ASC",
        ),
        ("", "project = \"ABC\" ORDER BY updated DESC"),
    ] {
        assert_eq!(scoped(raw), jql, "scope of {raw:?}");
    }
}

#[test]
fn the_acceptance_queries_are_scoped_as_the_spec_states() {
    assert_eq!(
        scoped("assignee = currentUser() AND priority = High"),
        "project = \"ABC\" AND (assignee = currentUser() AND priority = High) ORDER BY updated DESC",
        "US1 AS2"
    );
    assert_eq!(
        scoped("status = \"In Review\" ORDER BY priority DESC, created ASC"),
        "project = \"ABC\" AND (status = \"In Review\") ORDER BY priority DESC, created ASC",
        "US1 AS3"
    );
}

#[test]
fn no_status_category_clause_is_ever_added() {
    for raw in ["", "type = Bug", "ORDER BY created", "status = Done"] {
        assert!(
            !scoped(raw).contains("statusCategory"),
            "the user's query decides which statuses appear ({raw:?})"
        );
    }
}

#[test]
fn equal_user_queries_give_equal_scoped_queries() {
    let first = ScopedQuery::new(&key(), &parsed("type = Bug\n"));
    let again = ScopedQuery::new(&key(), &parsed(" type = Bug"));
    assert_eq!(first, again, "a retry re-sends an equal query");
    assert_ne!(
        first,
        ScopedQuery::new(
            &ProjectKey::parse("XYZ").expect("a key"),
            &parsed("type = Bug")
        ),
        "another project is another query"
    );
}

// §5 Texts

#[test]
fn the_refusal_texts_are_spelled_as_the_contract() {
    assert_eq!(
        QueryRefusal::UnclosedQuote.message(),
        "The query has a quote that is not closed. Close it and run the query again."
    );
    assert_eq!(
        QueryRefusal::UnbalancedParentheses.message(),
        "The query's parentheses do not match. Fix them and run the query again."
    );
    assert_eq!(
        QueryRefusal::TooLong.message(),
        "The query is longer than 2,000 characters. Shorten it and run the query again."
    );
}

#[test]
fn the_selector_texts_are_spelled_as_the_contract() {
    assert_eq!(
        NOT_VALID_JQL,
        "Jira did not accept the query: it is not valid JQL."
    );
    assert_eq!(EMPTY, "No issue matches this query.");
    assert_eq!(PROMPT, "Type a JQL query and press Enter to list issues.");
    assert_eq!(CAPPED, "Showing the first 1,000 issues this query returns.");
}

#[test]
fn no_text_contains_the_query_that_produced_it() {
    for raw in [
        "summary ~ \"secret-marker",
        "(secret-marker = 1",
        &format!("secret-marker{}", "a".repeat(2_000)),
    ] {
        let message = refusal(raw).message();
        assert!(
            !message.contains("secret-marker"),
            "a refusal names no part of the query (FR-016): {message}"
        );
    }
}

#[test]
fn the_mode_defaults_to_basic_and_is_written_in_lower_case() {
    assert_eq!(QueryMode::default(), QueryMode::Basic);
    assert_eq!(
        serde_json::to_string(&QueryMode::Advanced).expect("serializes"),
        "\"advanced\""
    );
    assert_eq!(
        serde_json::from_str::<QueryMode>("\"basic\"").expect("reads back"),
        QueryMode::Basic
    );
}

// §6 The advanced load

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.example").expect("valid")
}

fn token() -> ApiToken {
    ApiToken::parse("secret").expect("a token")
}

fn issue(key: &str) -> Issue {
    Issue::jira(key, 1, format!("Issue {key}"), vec![], String::new())
}

fn page_of(issues: Vec<Issue>, next: Option<&str>) -> IssuePage {
    IssuePage {
        total_open: issues.len() as u64,
        issues,
        next_cursor: next.map(str::to_owned),
    }
}

fn numbered(first: u64, len: u64, next: Option<&str>) -> IssuePage {
    page_of(
        (first..first + len)
            .map(|n| issue(&format!("ABC-{n}")))
            .collect(),
        next,
    )
}

fn query() -> ScopedQuery {
    ScopedQuery::new(&key(), &parsed("type = Bug"))
}

fn keys(issues: &[Issue]) -> Vec<&str> {
    issues.iter().filter_map(Issue::key).collect()
}

#[test]
fn pages_are_followed_by_cursor_and_listed_in_jiras_order() {
    let source = FakeJiraSource::new()
        .with_query(Ok(page_of(
            vec![issue("ABC-9"), issue("ABC-2")],
            Some("tok-1"),
        )))
        .with_query(Ok(page_of(vec![issue("ABC-5")], None)));
    let listing = load(&source, &conn(), &token(), &query()).expect("loaded");
    assert_eq!(
        keys(&listing.issues),
        ["ABC-9", "ABC-2", "ABC-5"],
        "Jira's order, not the app's"
    );
    assert!(listing.complete, "the last page left no cursor");
    assert_eq!(listing.total_open, 3, "Jira has no total: the count held");
    let jql = query().as_str().to_string();
    assert_eq!(
        source.query_calls(),
        [(jql.clone(), None), (jql, Some("tok-1".to_string()))],
        "every page asks for the scoped query, the second from the first page's token"
    );
}

#[test]
fn the_load_stops_at_the_cap_and_says_it_is_incomplete() {
    let mut source = FakeJiraSource::new();
    for n in 0..11 {
        source = source.with_query(Ok(numbered(n * 100 + 1, 100, Some(&format!("tok-{n}")))));
    }
    let listing = load(&source, &conn(), &token(), &query()).expect("loaded");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(!listing.complete, "more issues match than were loaded");
    assert_eq!(
        source.query_calls().len(),
        10,
        "no page is asked for past the cap"
    );
}

#[test]
fn issues_of_another_project_are_dropped_and_moved_issues_kept() {
    let source = FakeJiraSource::new().with_query(Ok(page_of(
        vec![
            issue("XYZ-1"),
            issue("ABC-"),
            issue("ABCD-3"),
            issue("ABC-1x"),
            issue("abc-4"),
            Issue::new(5, "no key".into(), vec![], String::new()),
            issue("ABC-42"),
        ],
        None,
    )));
    let listing = load(&source, &conn(), &token(), &query()).expect("loaded");
    assert_eq!(
        keys(&listing.issues),
        ["ABC-42"],
        "only <KEY>-<digits> of the connected project is listed (FR-008); an issue moved into \
         ABC carries its ABC key and is kept"
    );
    assert_eq!(listing.total_open, 1);
}

#[test]
fn issues_in_a_done_status_are_listed() {
    let done = TicketStatus::from_jira(Some("Done"), Some("done"));
    assert!(done.is_some(), "a usable Done status");
    let source =
        FakeJiraSource::new().with_query(Ok(page_of(vec![issue("ABC-1").with_status(done)], None)));
    let listing = load(&source, &conn(), &token(), &query()).expect("loaded");
    assert_eq!(
        keys(&listing.issues),
        ["ABC-1"],
        "the query, not the app, decides which statuses appear (US1 AS5)"
    );
}

#[test]
fn the_first_error_aborts_the_load() {
    let source = FakeJiraSource::new()
        .with_query(Ok(numbered(1, 2, Some("tok-1"))))
        .with_query(Err(QueryFailure::Load(IssueLoadError::Offline)))
        .with_query(Ok(numbered(3, 1, None)));
    assert_eq!(
        load(&source, &conn(), &token(), &query()),
        Err(QueryFailure::Load(IssueLoadError::Offline)),
        "a partial list is never shown as the list"
    );
    assert_eq!(
        source.query_calls().len(),
        2,
        "nothing is asked after the error"
    );
}

#[test]
fn a_rejection_passes_through_unchanged() {
    let rejected = QueryFailure::Rejected(vec!["Field 'asignee' does not exist.".into()]);
    let source = FakeJiraSource::new().with_query(Err(rejected.clone()));
    assert_eq!(load(&source, &conn(), &token(), &query()), Err(rejected));
}

#[test]
fn an_unscripted_query_answers_a_load_failure() {
    let source = FakeJiraSource::new();
    assert_eq!(
        load(&source, &conn(), &token(), &query()),
        Err(QueryFailure::Load(IssueLoadError::Other(
            "FakeJiraSource: no query scripted for this call".into()
        )))
    );
}

// §4 Jira's messages (FR-010)

fn messages(body: &str) -> Vec<String> {
    rejection_messages(body.as_bytes())
}

#[test]
fn jiras_messages_come_from_error_messages_then_errors() {
    assert_eq!(
        messages(
            r#"{"errorMessages":["Field 'asignee' does not exist or you do not have permission to view it."],"errors":{}}"#
        ),
        ["Field 'asignee' does not exist or you do not have permission to view it."]
    );
    assert_eq!(
        messages(
            r#"{"errorMessages":[],"errors":{"jql":"Expecting ')' before the end of the query."}}"#
        ),
        ["Expecting ')' before the end of the query."]
    );
    assert_eq!(messages(r#"{"errorMessages":["a","b"]}"#), ["a", "b"]);
}

#[test]
fn error_messages_keep_jiras_order_and_errors_follow_in_key_order() {
    assert_eq!(
        messages(
            r#"{"errorMessages":["z first","a second"],"errors":{"b":"from b","a":"from a"}}"#
        ),
        ["z first", "a second", "from a", "from b"]
    );
}

#[test]
fn each_message_is_cleaned_trimmed_and_an_empty_one_dropped() {
    assert_eq!(
        messages(
            r#"{"errorMessages":["  line one\nline two\t", "a\u0007b", "   ", "", 7],"errors":{"x":"\n","y":3}}"#
        ),
        ["line one line two", "ab"]
    );
}

#[test]
fn a_long_message_is_cut_at_300_chars_and_at_most_10_are_kept() {
    let long = "é".repeat(400);
    let cut = messages(&format!(r#"{{"errorMessages":["{long}"]}}"#));
    assert_eq!(cut, [format!("{}…", "é".repeat(300))]);
    let exact = "x".repeat(300);
    assert_eq!(
        messages(&format!(r#"{{"errorMessages":["{exact}"]}}"#)),
        [exact],
        "300 chars is not cut"
    );

    let twelve: Vec<String> = (1..=12).map(|n| format!("m{n}")).collect();
    let body = serde_json::json!({ "errorMessages": twelve }).to_string();
    assert_eq!(messages(&body), twelve[..10]);
}

#[test]
fn a_body_with_no_messages_gives_none() {
    for body in [
        "<html><body>Bad request</body></html>",
        "{}",
        "",
        "[]",
        r#"{"errorMessages":"not a list","errors":["not","an object"]}"#,
        "\u{fffd}",
    ] {
        assert!(messages(body).is_empty(), "{body:?}");
    }
    assert!(rejection_messages(&[0xff, 0xfe]).is_empty());
}

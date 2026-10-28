//! The GitHub selector's filter rules, filtered load, label choices and remembered state (feature
//! 1400, `specs/1400-github-selector-filters/contracts/search-rules.md` and data-model.md).
//!
//! One test per contract row where the contract has a table: §1 cleaning, §2 refusals, §3 the sort
//! split, §4 the scoped search text, §5 values, §6 answers; then the filtered load and the label
//! choices through `FakeIssueSource`, and the remembered selector (data-model §7).

use micold_core::github::{
    FakeIssueSource, GithubRepo, Issue, IssueLoadError, IssuePage, NamePage, ISSUE_LOAD_CAP,
};
use micold_core::github_filter::{
    load_filtered, load_label_choices, load_milestone_choices, parse_people, parse_search_page,
    Assignee, Author, BasicFilters, FilterFailure, GithubFilter, IssueState, LabelChoices,
    LabelName, Login, Milestone, MilestoneChoices, MilestoneTitle, QueryRefusal,
    RememberedSelector, ScopedSearch, SelectorChange, UserQuery, NOT_ACCEPTED,
    SEARCH_FILTERED_QUERY, SEARCH_PAGE_QUERY,
};
use micold_core::jira_query::QueryMode;
use micold_core::store::{github_selector_path, project_id_of};
use serde_json::json;

fn label(name: &str) -> LabelName {
    LabelName::parse(name).unwrap_or_else(|| panic!("{name:?} is a label"))
}

fn parsed(raw: &str) -> UserQuery {
    UserQuery::parse(raw).unwrap_or_else(|refusal| panic!("{raw:?} refused: {refusal:?}"))
}

fn refusal(raw: &str) -> QueryRefusal {
    UserQuery::parse(raw).expect_err(raw)
}

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/app").expect("a repository")
}

fn including(names: &[&str]) -> BasicFilters {
    let mut filters = BasicFilters::default();
    for name in names {
        filters.include_label(label(name));
    }
    filters
}

fn excluding(names: &[&str]) -> BasicFilters {
    let mut filters = BasicFilters::default();
    for name in names {
        filters.exclude_label(label(name));
    }
    filters
}

fn listing(filter: &GithubFilter) -> String {
    ScopedSearch::listing(&repo(), filter).as_str().to_string()
}

fn beyond(filter: &GithubFilter, typed: &str) -> Option<String> {
    ScopedSearch::beyond(&repo(), filter, typed).map(|s| s.as_str().to_string())
}

fn labels(names: &[LabelName]) -> Vec<&str> {
    names.iter().map(LabelName::as_str).collect()
}

// §5 Values

#[test]
fn a_label_name_is_any_non_empty_name_without_a_quote_backslash_or_control_character() {
    for name in [
        "bug",
        "good first issue",
        "area: ui",
        "(wip)",
        "日本語",
        " lead space",
    ] {
        assert_eq!(
            LabelName::parse(name).map(|l| l.as_str().to_string()),
            Some(name.to_string()),
            "{name:?} is offered and kept as is"
        );
    }
    for name in ["say \"hi\"", "tab\there", "", "foo\\", "a\\b"] {
        assert_eq!(LabelName::parse(name), None, "{name:?} is not a label");
    }
}

#[test]
fn the_state_is_open_by_default_and_serialised_in_lowercase() {
    assert_eq!(IssueState::default(), IssueState::Open);
    for (state, text) in [
        (IssueState::Open, "\"open\""),
        (IssueState::Closed, "\"closed\""),
        (IssueState::All, "\"all\""),
    ] {
        assert_eq!(serde_json::to_string(&state).unwrap(), text);
        assert_eq!(serde_json::from_str::<IssueState>(text).unwrap(), state);
    }
}

// Data-model §2: BasicFilters

#[test]
fn including_a_label_removes_it_from_the_excluded_and_back() {
    let mut filters = excluding(&["bug", "ui"]);
    filters.include_label(label("bug"));
    assert_eq!(labels(filters.include()), ["bug"]);
    assert_eq!(labels(filters.exclude()), ["ui"]);
    filters.exclude_label(label("bug"));
    assert_eq!(labels(filters.include()), Vec::<&str>::new());
    assert_eq!(labels(filters.exclude()), ["ui", "bug"]);
}

#[test]
fn a_label_chosen_twice_is_held_once_in_chosen_order() {
    let filters = including(&["b", "a", "b"]);
    assert_eq!(labels(filters.include()), ["b", "a"]);
    let filters = excluding(&["x", "x", "y"]);
    assert_eq!(labels(filters.exclude()), ["x", "y"]);
}

#[test]
fn removing_a_label_drops_it_from_either_list_and_keeps_the_rest() {
    let mut filters = including(&["bug", "ui"]);
    filters.exclude_label(label("blocked"));
    filters.remove_label(&label("bug"));
    filters.remove_label(&label("blocked"));
    assert_eq!(labels(filters.include()), ["ui"]);
    assert!(filters.exclude().is_empty());
}

#[test]
fn the_default_filter_is_no_label_and_open_issues() {
    let mut filters = BasicFilters::default();
    assert!(filters.is_default());
    assert_eq!(filters.chosen_count(), 0);
    assert_eq!(filters.described(), None);
    filters.set_state(IssueState::Closed);
    assert!(!filters.is_default());
    filters.set_state(IssueState::Open);
    assert!(filters.is_default());
    let filters = including(&["bug"]);
    assert!(!filters.is_default());
    assert!(filters.cleared().is_default());
    assert_eq!(filters.cleared(), BasicFilters::default());
}

#[test]
fn the_count_is_one_per_label_and_one_for_a_state_other_than_open() {
    let mut filters = including(&["bug", "ui"]);
    filters.exclude_label(label("blocked"));
    assert_eq!(filters.chosen_count(), 3);
    filters.set_state(IssueState::All);
    assert_eq!(filters.chosen_count(), 4);
}

#[test]
fn the_active_line_names_each_part_in_order() {
    assert_eq!(
        including(&["bug", "ui"]).described().as_deref(),
        Some("Labels bug, ui")
    );
    assert_eq!(
        excluding(&["in-progress", "blocked"])
            .described()
            .as_deref(),
        Some("Excluding labels in-progress, blocked")
    );
    let mut closed = BasicFilters::default();
    closed.set_state(IssueState::Closed);
    assert_eq!(closed.described().as_deref(), Some("Closed issues"));
    let mut all = excluding(&["blocked"]);
    all.include_label(label("bug"));
    all.set_state(IssueState::All);
    assert_eq!(
        all.described().as_deref(),
        Some("Labels bug; Excluding labels blocked; Open and closed issues")
    );
}

#[test]
fn basic_filters_read_leniently() {
    let read: BasicFilters = serde_json::from_value(json!({
        "include": ["bug", "say \"hi\"", "bug", "both", 7],
        "exclude": ["blocked", "", "both", "blocked"],
        "state": "closed"
    }))
    .expect("lenient");
    assert_eq!(labels(read.include()), ["bug", "both"]);
    assert_eq!(labels(read.exclude()), ["blocked"]);
    assert_eq!(read.state(), IssueState::Closed);

    let odd: BasicFilters =
        serde_json::from_value(json!({"include": "bug", "state": "sideways"})).expect("lenient");
    assert_eq!(odd, BasicFilters::default());

    let mut chosen = including(&["bug"]);
    chosen.exclude_label(label("blocked"));
    chosen.set_state(IssueState::All);
    let round: BasicFilters =
        serde_json::from_slice(&serde_json::to_vec(&chosen).unwrap()).unwrap();
    assert_eq!(round, chosen);
}

#[test]
fn basic_filters_never_print_a_label() {
    let mut filters = including(&["secret-include"]);
    filters.exclude_label(label("secret-exclude"));
    let printed = format!("{filters:?} {:?}", label("secret-label"));
    assert!(!printed.contains("secret"), "{printed}");
    assert_eq!(printed.matches("<redacted>").count(), 3, "{printed}");
}

// §1 Cleaning and limits

#[test]
fn a_query_is_cleaned_before_it_is_checked() {
    assert_eq!(
        parsed("  is:open\n-label:x\t ").as_str(),
        "is:open -label:x"
    );
    assert_eq!(parsed("a\u{7}b").as_str(), "ab");
    assert_eq!(parsed("").as_str(), "");
    assert_eq!(parsed("   ").as_str(), "");
}

#[test]
fn a_query_holds_at_most_a_thousand_characters_after_cleaning() {
    let at_limit = "日".repeat(1_000);
    assert_eq!(parsed(&format!("  {at_limit}\n")).as_str(), at_limit);
    assert_eq!(refusal(&"a".repeat(1_001)), QueryRefusal::TooLong);
    assert_eq!(
        refusal(&format!("\"{}", "a".repeat(1_000))),
        QueryRefusal::TooLong,
        "too long outranks every other refusal"
    );
}

// §2 Refusals

#[test]
fn an_unclosed_quote_is_refused() {
    assert_eq!(refusal("label:\"in progress"), QueryRefusal::UnclosedQuote);
    assert_eq!(
        refusal("repo:x (label:\"a"),
        QueryRefusal::UnclosedQuote,
        "an unclosed quote outranks a scope qualifier and parentheses"
    );
}

#[test]
fn a_scope_qualifier_is_refused() {
    for raw in [
        "repo:other/x is:open",
        "-repo:other/x",
        "org:acme",
        "USER:bob",
        "owner:me",
        "(a OR repo:x)",
        "(a)repo:other/x",
        "\"x\"repo:other/x",
        "label:a,repo:other/x",
    ] {
        assert_eq!(refusal(raw), QueryRefusal::ScopeQualifier, "{raw}");
    }
    assert_eq!(
        refusal("(repo:x"),
        QueryRefusal::ScopeQualifier,
        "a scope qualifier outranks unbalanced parentheses"
    );
}

#[test]
fn a_scope_qualifier_in_a_quoted_string_or_inside_a_word_is_text() {
    for raw in ["\"repo:x\" label:bug", "myrepo:x", "my_repo:x", "my-repo:x"] {
        assert_eq!(parsed(raw).as_str(), raw);
    }
}

#[test]
fn unbalanced_parentheses_are_refused() {
    for raw in ["(label:a OR label:b", "label:a) OR (label:b"] {
        assert_eq!(refusal(raw), QueryRefusal::UnbalancedParentheses, "{raw}");
    }
    assert_eq!(parsed("\"(\" label:a").as_str(), "\"(\" label:a");
}

#[test]
fn githubs_own_qualifiers_run() {
    for raw in [
        "state:open no:assignee -linked:pr -label:blocked sort:created-asc",
        "is:pr",
    ] {
        assert_eq!(parsed(raw).as_str(), raw);
    }
}

#[test]
fn every_refusal_has_its_own_text_without_the_query() {
    for (refusal, text) in [
        (
            QueryRefusal::TooLong,
            "The query is longer than 1,000 characters. Shorten it and run it again.",
        ),
        (
            QueryRefusal::UnclosedQuote,
            "The query has a quote that is not closed. Close it and run it again.",
        ),
        (
            QueryRefusal::ScopeQualifier,
            "The query names a repository, organization or user. The selector already searches \
             this repository's issues; remove repo:, org:, user: or owner: and run it again.",
        ),
        (
            QueryRefusal::UnbalancedParentheses,
            "The query's parentheses do not match. Fix them and run it again.",
        ),
    ] {
        assert_eq!(refusal.message(), text);
    }
}

#[test]
fn a_user_query_never_prints_its_text() {
    let printed = format!("{:?}", parsed("label:secret sort:created-asc"));
    assert!(!printed.contains("secret"), "{printed}");
    assert!(printed.contains("<redacted>"), "{printed}");
}

// §3 Sort split

#[test]
fn sort_qualifiers_are_split_from_the_condition() {
    for (raw, condition, sorts) in [
        (
            "-label:x sort:created-asc",
            "-label:x",
            vec!["sort:created-asc"],
        ),
        ("sort:created-asc", "", vec!["sort:created-asc"]),
        ("(sort:updated label:a)", "( label:a)", vec!["sort:updated"]),
        ("\"sort:x\" bug", "\"sort:x\" bug", vec![]),
        ("SORT:comments-desc a", "a", vec!["SORT:comments-desc"]),
    ] {
        let query = parsed(raw);
        assert_eq!(query.condition(), condition, "{raw}");
        assert_eq!(query.sorts(), sorts, "{raw}");
        assert_eq!(query.as_str(), raw, "{raw}: the text runs as typed");
    }
}

// §4 Scoped search text

#[test]
fn only_the_default_basic_filter_is_default() {
    assert!(GithubFilter::Basic(BasicFilters::default()).is_default());
    assert!(!GithubFilter::Basic(excluding(&["blocked"])).is_default());
    assert!(!GithubFilter::Advanced(parsed("")).is_default());
}

#[test]
fn a_basic_filter_scopes_its_labels_and_state_to_the_repository() {
    assert_eq!(
        listing(&GithubFilter::Basic(excluding(&["in-progress", "blocked"]))),
        "repo:acme/app is:issue is:open -label:\"in-progress\" -label:\"blocked\" sort:updated-desc"
    );
    let mut all = including(&["bug", "ui"]);
    all.set_state(IssueState::All);
    assert_eq!(
        listing(&GithubFilter::Basic(all)),
        "repo:acme/app is:issue label:\"bug\" label:\"ui\" sort:updated-desc"
    );
    let mut closed = BasicFilters::default();
    closed.set_state(IssueState::Closed);
    assert_eq!(
        listing(&GithubFilter::Basic(closed)),
        "repo:acme/app is:issue is:closed sort:updated-desc"
    );
    assert_eq!(
        listing(&GithubFilter::Basic(including(&["good first issue", "日本語"]))),
        "repo:acme/app is:issue is:open label:\"good first issue\" label:\"日本語\" sort:updated-desc"
    );
}

#[test]
fn an_advanced_query_runs_in_parentheses_inside_the_scope() {
    for (raw, scoped) in [
        (
            "-label:in-progress state:open",
            "repo:acme/app is:issue (-label:in-progress state:open) sort:updated-desc",
        ),
        (
            "a OR b sort:created-asc",
            "repo:acme/app is:issue (a OR b) sort:created-asc",
        ),
        ("", "repo:acme/app is:issue sort:updated-desc"),
        (
            "sort:created-asc",
            "repo:acme/app is:issue sort:created-asc",
        ),
    ] {
        assert_eq!(
            listing(&GithubFilter::Advanced(parsed(raw))),
            scoped,
            "{raw}"
        );
    }
}

#[test]
fn the_search_beyond_appends_the_typed_words_quoted() {
    let blocked = GithubFilter::Basic(excluding(&["blocked"]));
    let base = "repo:acme/app is:issue is:open -label:\"blocked\"";
    for (typed, words) in [
        ("crash", " \"crash\""),
        ("crash OR ui", " \"crash\" \"OR\" \"ui\""),
        ("a)", " \"a)\""),
        ("repo:x", " \"repo:x\""),
        ("-label:ui (x", " \"-label:ui\" \"(x\""),
        ("say \"hi\"", " \"say\" \"hi\""),
        ("a\\ OR b", " \"a\" \"OR\" \"b\""),
    ] {
        assert_eq!(
            beyond(&blocked, typed),
            Some(format!("{base}{words}")),
            "{typed}"
        );
    }
    assert_eq!(
        beyond(
            &GithubFilter::Advanced(parsed("a OR b sort:created-asc")),
            "crash OR ui"
        ),
        Some("repo:acme/app is:issue (a OR b) \"crash\" \"OR\" \"ui\"".to_string())
    );
}

#[test]
fn no_search_beyond_is_sent_without_a_word() {
    for filter in [
        GithubFilter::Basic(excluding(&["blocked"])),
        GithubFilter::Advanced(parsed("a OR b")),
    ] {
        for typed in ["", "   ", "\"\"", "\\", "\\\" \\"] {
            assert_eq!(beyond(&filter, typed), None, "{typed:?}");
        }
    }
}

#[test]
fn a_scoped_search_never_prints_its_text() {
    let printed = format!(
        "{:?}",
        ScopedSearch::listing(&repo(), &GithubFilter::Basic(including(&["secret"])))
    );
    assert!(!printed.contains("secret"), "{printed}");
    assert!(printed.contains("<redacted>"), "{printed}");
}

// §6 Answers

#[test]
fn the_search_queries_use_the_advanced_search_type() {
    for query in [SEARCH_PAGE_QUERY, SEARCH_FILTERED_QUERY] {
        assert!(query.contains("search(type: ISSUE_ADVANCED"), "{query}");
        assert!(query.contains("issueCount"), "{query}");
        assert!(query.contains("bodyText state repository { nameWithOwner }"));
        assert!(!query.contains('\n'), "one line");
    }
    assert!(SEARCH_PAGE_QUERY.contains("first: 100, after: $cursor"));
    assert!(SEARCH_PAGE_QUERY.contains("$cursor: String"));
    assert!(SEARCH_FILTERED_QUERY.contains("first: 50)"));
    assert!(!SEARCH_FILTERED_QUERY.contains("cursor"));
}

fn node(number: u64, state: &str, repository: &str) -> serde_json::Value {
    json!({
        "number": number,
        "title": format!("Issue {number}"),
        "updatedAt": "2026-10-01T00:00:00Z",
        "labels": {"nodes": [{"name": "bug"}]},
        "author": {"login": "octo"},
        "bodyText": "body",
        "state": state,
        "repository": {"nameWithOwner": repository}
    })
}

fn answer(nodes: Vec<serde_json::Value>, count: u64, cursor: Option<&str>) -> Vec<u8> {
    serde_json::to_vec(&json!({"data": {"search": {
        "issueCount": count,
        "pageInfo": {"hasNextPage": cursor.is_some(), "endCursor": cursor},
        "nodes": nodes
    }}}))
    .unwrap()
}

fn numbers(issues: &[Issue]) -> Vec<u64> {
    issues.iter().map(Issue::number).collect()
}

#[test]
fn a_search_page_lists_open_and_closed_issues_of_the_repository_in_githubs_order() {
    let page = parse_search_page(
        &answer(
            vec![
                node(3, "OPEN", "acme/app"),
                node(1, "CLOSED", "acme/app"),
                node(2, "OPEN", "ACME/App"),
            ],
            57,
            Some("NEXT"),
        ),
        &repo(),
    )
    .expect("a page");
    assert_eq!(numbers(&page.issues), [3, 1, 2]);
    assert_eq!(
        page.issues.iter().map(Issue::is_closed).collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(page.total_open, 57, "issueCount");
    assert_eq!(page.next_cursor.as_deref(), Some("NEXT"));
    assert_eq!(page.issues[0].description(), "body", "the body comes along");
}

#[test]
fn a_search_page_drops_other_repositories_and_pull_requests() {
    let page = parse_search_page(
        &answer(
            vec![
                node(1, "OPEN", "other/x"),
                json!({}),
                node(2, "OPEN", "acme/app"),
            ],
            3,
            None,
        ),
        &repo(),
    )
    .expect("a page");
    assert_eq!(numbers(&page.issues), [2]);
    assert_eq!(page.next_cursor, None);
}

fn failure(body: serde_json::Value) -> FilterFailure {
    parse_search_page(&serde_json::to_vec(&body).unwrap(), &repo()).expect_err("a failure")
}

#[test]
fn githubs_refusal_of_the_search_is_rejected_with_its_messages_in_order() {
    assert_eq!(
        failure(json!({"data": null, "errors": [
            {"type": "INVALID", "path": ["search"], "message": "m1"},
            {"path": ["search"], "message": "m2"}
        ]})),
        FilterFailure::Rejected(vec!["m1".into(), "m2".into()])
    );
    let users = "The listed users cannot be searched either because the users do not exist or \
                 you do not have permission to view the users.";
    assert_eq!(
        failure(json!({"data": {"search": null}, "errors": [
            {"type": "NOT_FOUND", "path": ["search"], "message": users}
        ]})),
        FilterFailure::Rejected(vec![users.into()])
    );
    assert_eq!(
        failure(json!({"errors": [{"message": "x"}]})),
        FilterFailure::Rejected(vec!["x".into()])
    );
}

#[test]
fn a_rate_limit_is_a_load_failure() {
    assert_eq!(
        failure(json!({"data": null, "errors": [
            {"type": "RATE_LIMITED", "path": ["search"], "message": "API rate limit exceeded"}
        ]})),
        FilterFailure::Load(IssueLoadError::RateLimited)
    );
}

#[test]
fn a_refused_access_or_missing_scope_is_a_load_failure_not_a_rejection() {
    for kind in ["FORBIDDEN", "INSUFFICIENT_SCOPES"] {
        assert_eq!(
            failure(json!({"data": {"search": null}, "errors": [
                {"type": "INVALID", "path": ["search"], "message": "m1"},
                {"type": kind, "path": ["search"], "message": "Resource protected by SAML"}
            ]})),
            FilterFailure::Load(IssueLoadError::Other("Resource protected by SAML".into())),
            "{kind} goes to 034's classification, which knows SAML and scopes"
        );
    }
}

#[test]
fn githubs_messages_are_cleaned_cut_and_counted() {
    let long = "m".repeat(400);
    let mut errors = vec![
        json!({"path": ["search"], "message": long}),
        json!({"path": ["search"], "message": "line\nbreak"}),
        json!({"path": ["search"], "message": ""}),
    ];
    errors.extend((0..12).map(|n| json!({"path": ["search"], "message": format!("e{n}")})));
    let FilterFailure::Rejected(messages) = failure(json!({"data": null, "errors": errors})) else {
        panic!("a rejection")
    };
    assert_eq!(messages.len(), 10, "at most 10");
    assert_eq!(messages[0], format!("{}…", "m".repeat(300)));
    assert_eq!(messages[1], "line break");
    assert_eq!(messages[2], "e0", "the empty message is dropped");
}

#[test]
fn a_rejection_without_messages_says_github_did_not_accept_the_query() {
    assert_eq!(NOT_ACCEPTED, "GitHub did not accept the search query.");
    assert_eq!(
        FilterFailure::Rejected(vec![]).rejection(),
        Some(vec![NOT_ACCEPTED.to_string()])
    );
    assert_eq!(
        FilterFailure::Rejected(vec!["m1".into()]).rejection(),
        Some(vec!["m1".to_string()])
    );
    assert_eq!(
        FilterFailure::Load(IssueLoadError::Offline).rejection(),
        None
    );
    assert_eq!(
        failure(json!({"data": null, "errors": [{"path": ["search"], "message": ""}]})),
        FilterFailure::Rejected(vec![])
    );
}

#[test]
fn an_answer_of_another_shape_is_a_load_failure() {
    for body in [&b"not json"[..], br#"{"data": {"repository": {}}}"#] {
        assert!(
            matches!(
                parse_search_page(body, &repo()),
                Err(FilterFailure::Load(IssueLoadError::Other(_)))
            ),
            "{}",
            String::from_utf8_lossy(body)
        );
    }
}

// The filtered load

fn page(range: std::ops::Range<u64>, count: u64, cursor: Option<&str>) -> IssuePage {
    IssuePage {
        issues: range
            .map(|n| Issue::new(n, format!("Issue {n}"), vec![], String::new()))
            .collect(),
        total_open: count,
        next_cursor: cursor.map(str::to_string),
    }
}

fn blocked_search() -> ScopedSearch {
    ScopedSearch::listing(&repo(), &GithubFilter::Basic(excluding(&["blocked"])))
}

#[test]
fn a_filtered_load_pages_by_cursor_in_githubs_order() {
    let source = FakeIssueSource::new().with_search_pages(vec![
        Ok(page(0..100, 150, Some("C1"))),
        Ok(page(100..150, 150, None)),
    ]);
    let listing = load_filtered(&source, &repo(), &blocked_search()).expect("a listing");
    assert_eq!(listing.issues.len(), 150);
    assert_eq!(listing.issues[100].number(), 100);
    assert_eq!(listing.total_open, 150);
    assert!(listing.complete);
    assert_eq!(
        source.asked_searches(),
        vec![blocked_search().as_str().to_string(); 2],
        "each page asks with the scoped text"
    );
}

#[test]
fn a_filtered_load_holds_at_most_the_cap() {
    let pages = (0..11)
        .map(|n| Ok(page(n * 100..(n + 1) * 100, 5_000, Some(&format!("C{n}")))))
        .collect();
    let source = FakeIssueSource::new().with_search_pages(pages);
    let listing = load_filtered(&source, &repo(), &blocked_search()).expect("a listing");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(!listing.complete);
    assert_eq!(listing.total_open, 5_000);
    assert_eq!(source.asked_searches().len(), 10, "no page past the cap");
}

#[test]
fn issues_of_other_repositories_are_dropped_before_the_cap_counts() {
    let mut nodes: Vec<_> = (0..100).map(|n| node(n, "OPEN", "acme/app")).collect();
    nodes[5] = node(5, "OPEN", "other/x");
    nodes[6] = json!({});
    let first = parse_search_page(&answer(nodes, 1_001, Some("C1")), &repo()).unwrap();
    let pages = std::iter::once(Ok(first))
        .chain((1..11).map(|n| Ok(page(n * 100..(n + 1) * 100, 1_001, Some(&format!("D{n}"))))))
        .collect();
    let source = FakeIssueSource::new().with_search_pages(pages);
    let listing = load_filtered(&source, &repo(), &blocked_search()).expect("a listing");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(!numbers(&listing.issues).contains(&5));
    assert!(!numbers(&listing.issues).contains(&6));
    assert_eq!(
        source.asked_searches().len(),
        11,
        "the dropped issues left room for a page more"
    );
}

#[test]
fn the_first_failure_aborts_a_filtered_load() {
    let source = FakeIssueSource::new().with_search_pages(vec![
        Ok(page(0..100, 300, Some("C1"))),
        Err(FilterFailure::Load(IssueLoadError::Offline)),
        Ok(page(200..300, 300, None)),
    ]);
    assert_eq!(
        load_filtered(&source, &repo(), &blocked_search()).unwrap_err(),
        FilterFailure::Load(IssueLoadError::Offline)
    );
    let rejected = FakeIssueSource::new()
        .with_search_pages(vec![Err(FilterFailure::Rejected(vec!["bad".into()]))]);
    assert_eq!(
        load_filtered(&rejected, &repo(), &blocked_search()).unwrap_err(),
        FilterFailure::Rejected(vec!["bad".into()]),
        "GitHub's refusal passes through unchanged"
    );
}

// Label choices

fn names(names: &[&str], cursor: Option<&str>) -> Result<NamePage, IssueLoadError> {
    Ok(NamePage {
        names: names.iter().map(|n| n.to_string()).collect(),
        next_cursor: cursor.map(str::to_string),
    })
}

#[test]
fn label_choices_are_sorted_without_case_and_held_once() {
    let source = FakeIssueSource::new().with_labels(vec![
        names(&["bug", "Zebra", "apple"], Some("C1")),
        names(&["say \"hi\"", "Bug", "bug", "tab\there"], None),
    ]);
    let choices: LabelChoices = load_label_choices(&source, &repo()).expect("choices");
    assert_eq!(labels(&choices.labels), ["apple", "Bug", "bug", "Zebra"]);
    assert_eq!(choices.not_offered, 2);
    assert!(!choices.partial);
}

#[test]
fn label_choices_stop_after_ten_pages_and_say_so() {
    let pages = (0..12)
        .map(|n| {
            let held: Vec<String> = (0..100).map(|i| format!("l{n:02}-{i:03}")).collect();
            Ok(NamePage {
                names: held,
                next_cursor: Some(format!("C{n}")),
            })
        })
        .collect();
    let source = FakeIssueSource::new().with_labels(pages);
    let choices = load_label_choices(&source, &repo()).expect("choices");
    assert_eq!(choices.labels.len(), 1_000);
    assert!(choices.partial, "a cursor remained");
    assert_eq!(source.label_calls().len(), 10);
}

#[test]
fn a_label_load_failure_aborts() {
    let source = FakeIssueSource::new().with_labels(vec![
        names(&["bug"], Some("C1")),
        Err(IssueLoadError::RateLimited),
    ]);
    assert_eq!(
        load_label_choices(&source, &repo()).unwrap_err(),
        IssueLoadError::RateLimited
    );
}

// Data-model §7: the remembered selector

fn remembered() -> RememberedSelector {
    let mut basic = excluding(&["blocked"]);
    basic.set_state(IssueState::All);
    RememberedSelector::default()
        .with_change(&SelectorChange::Mode(QueryMode::Advanced))
        .with_change(&SelectorChange::Basic(basic))
        .with_change(&SelectorChange::Query("label:a sort:created-asc".into()))
}

#[test]
fn the_remembered_selector_round_trips() {
    let held = remembered();
    assert_eq!(RememberedSelector::from_json(&held.to_json()), held);
    assert_eq!(held.mode, QueryMode::Advanced);
    assert_eq!(labels(held.basic.exclude()), ["blocked"]);
    assert_eq!(held.query, "label:a sort:created-asc");
    assert!(held.ran);
}

#[test]
fn an_unreadable_remembered_selector_is_the_default() {
    for bytes in [&b""[..], b"not json", b"[1, 2]"] {
        assert_eq!(
            RememberedSelector::from_json(bytes),
            RememberedSelector::default()
        );
    }
    let default = RememberedSelector::default();
    assert_eq!(default.mode, QueryMode::Basic);
    assert!(default.basic.is_default());
    assert_eq!(default.query, "");
    assert!(!default.ran);
}

#[test]
fn invalid_remembered_values_are_dropped() {
    let read = RememberedSelector::from_json(
        br#"{"mode":"sideways","basic":{"include":["bug","say \"hi\""],"state":"closed"},"query":"repo:x","ran":true}"#,
    );
    assert_eq!(read.mode, QueryMode::Basic);
    assert_eq!(labels(read.basic.include()), ["bug"]);
    assert_eq!(read.basic.state(), IssueState::Closed);
    assert_eq!(read.query, "", "a refused query is not remembered");
    assert!(!read.ran);
}

#[test]
fn each_change_sets_one_field() {
    let start = remembered();
    let moded = start
        .clone()
        .with_change(&SelectorChange::Mode(QueryMode::Basic));
    assert_eq!(moded.mode, QueryMode::Basic);
    assert_eq!(moded.basic, start.basic);
    assert_eq!(moded.query, start.query);

    let based = start
        .clone()
        .with_change(&SelectorChange::Basic(BasicFilters::default()));
    assert!(based.basic.is_default());
    assert_eq!((based.mode, &based.query), (start.mode, &start.query));

    let fresh = RememberedSelector::default().with_change(&SelectorChange::Query("  ".into()));
    assert_eq!(fresh.query, "");
    assert!(!fresh.ran, "an empty run is not a run");
    let kept = start
        .clone()
        .with_change(&SelectorChange::Query("  ".into()));
    assert_eq!(kept, start, "an empty run leaves the last run query");

    let refused = start
        .clone()
        .with_change(&SelectorChange::Query("org:x".into()));
    assert_eq!(refused, start, "a refused query changes nothing");
}

#[test]
fn the_remembered_selector_never_prints_a_value() {
    let printed = format!(
        "{:?} {:?}",
        RememberedSelector::default().with_change(&SelectorChange::Query("label:secret".into())),
        SelectorChange::Query("secret".into())
    );
    assert!(!printed.contains("secret"), "{printed}");
}

#[test]
fn each_project_remembers_its_selector_in_its_own_file() {
    let dir = std::path::Path::new("/data/github");
    let project = std::path::Path::new("/work/a");
    assert_eq!(
        github_selector_path(dir, project),
        dir.join(format!("{}.selector.json", project_id_of(project)))
    );
    assert_ne!(
        github_selector_path(dir, project),
        github_selector_path(dir, std::path::Path::new("/work/b"))
    );
}

// US6 (T053): assignee, author and milestone

fn login(text: &str) -> Login {
    Login::parse(text).unwrap_or_else(|| panic!("{text:?} is a login"))
}

fn milestone(title: &str) -> MilestoneTitle {
    MilestoneTitle::parse(title).unwrap_or_else(|| panic!("{title:?} is a milestone title"))
}

/// search-rules §5: 1–39 ASCII letters, digits and single inner hyphens.
#[test]
fn a_login_is_letters_digits_and_single_inner_hyphens() {
    let longest = "a".repeat(39);
    for valid in ["octocat", "octo-cat", "a", "Octo-Cat-42", longest.as_str()] {
        assert_eq!(
            Login::parse(valid)
                .map(|l| l.as_str().to_string())
                .as_deref(),
            Some(valid),
            "{valid:?}"
        );
    }
    let too_long = "a".repeat(40);
    for invalid in [
        "-octo",
        "octo-",
        "oc--to",
        "octo cat",
        "octo:cat",
        too_long.as_str(),
        "",
        "octo\"",
        "ö",
        "@me",
    ] {
        assert_eq!(Login::parse(invalid), None, "{invalid:?}");
    }
}

/// data-model §1, FR-006: non-empty, no `"`, no control character, and (as a label) no `\`.
#[test]
fn a_milestone_title_is_any_non_empty_title_without_a_quote_backslash_or_control_character() {
    for valid in [
        "v2.4",
        "Sprint 12: polish",
        "(wip)",
        "日本語",
        " lead space",
    ] {
        assert_eq!(
            MilestoneTitle::parse(valid)
                .map(|t| t.as_str().to_string())
                .as_deref(),
            Some(valid),
            "{valid:?}"
        );
    }
    for invalid in ["", "say \"hi\"", "tab\there", "foo\\", "a\\b"] {
        assert_eq!(MilestoneTitle::parse(invalid), None, "{invalid:?}");
    }
}

fn us6_filters() -> BasicFilters {
    let mut filters = BasicFilters::default();
    filters.set_assignee(Some(Assignee::Me));
    filters.set_author(Some(Author::User(login("octo-cat"))));
    filters.set_milestone(Some(Milestone::Titled(milestone("v2.4"))));
    filters
}

#[test]
fn each_us6_value_counts_once_and_makes_the_filter_not_default() {
    let mut filters = BasicFilters::default();
    filters.set_assignee(Some(Assignee::Nobody));
    assert!(!filters.is_default());
    assert_eq!(filters.chosen_count(), 1);
    assert_eq!(filters.assignee(), Some(&Assignee::Nobody));
    filters.set_assignee(None);
    assert!(filters.is_default());
    filters.set_author(Some(Author::Me));
    assert!(!filters.is_default());
    filters.set_author(None);
    filters.set_milestone(Some(Milestone::Nobody));
    assert!(!filters.is_default());
    assert_eq!(filters.milestone(), Some(&Milestone::Nobody));
    filters.set_milestone(None);
    assert!(filters.is_default());

    let mut all = us6_filters();
    all.exclude_label(label("blocked"));
    assert_eq!(all.chosen_count(), 4);
    assert_eq!(all.author(), Some(&Author::User(login("octo-cat"))));
    assert!(all.cleared().is_default());
}

/// data-model §2: the US6 parts follow the state, in order.
#[test]
fn the_active_line_names_the_us6_parts_after_the_state() {
    let mut filters = us6_filters();
    filters.exclude_label(label("blocked"));
    filters.set_state(IssueState::Closed);
    assert_eq!(
        filters.described().as_deref(),
        Some(
            "Excluding labels blocked; Closed issues; Assigned to me; By octo-cat; Milestone v2.4"
        )
    );
    let mut others = BasicFilters::default();
    others.set_assignee(Some(Assignee::Nobody));
    others.set_author(Some(Author::Me));
    others.set_milestone(Some(Milestone::Nobody));
    assert_eq!(
        others.described().as_deref(),
        Some("No assignee; By me; No milestone")
    );
    let mut person = BasicFilters::default();
    person.set_assignee(Some(Assignee::User(login("hubot"))));
    assert_eq!(person.described().as_deref(), Some("Assigned to hubot"));
}

/// search-rules §4, the two US6 rows: after the label terms, before the sort.
#[test]
fn the_us6_values_are_scoped_after_the_labels() {
    assert_eq!(
        listing(&GithubFilter::Basic(us6_filters())),
        "repo:acme/app is:issue is:open assignee:@me author:octo-cat milestone:\"v2.4\" sort:updated-desc"
    );
    let mut nobody = BasicFilters::default();
    nobody.set_assignee(Some(Assignee::Nobody));
    nobody.set_milestone(Some(Milestone::Nobody));
    assert_eq!(
        listing(&GithubFilter::Basic(nobody)),
        "repo:acme/app is:issue is:open no:assignee no:milestone sort:updated-desc"
    );
    let mut people = excluding(&["blocked"]);
    people.set_assignee(Some(Assignee::User(login("hubot"))));
    people.set_author(Some(Author::Me));
    assert_eq!(
        listing(&GithubFilter::Basic(people)),
        "repo:acme/app is:issue is:open -label:\"blocked\" assignee:hubot author:@me sort:updated-desc"
    );
}

/// FR-021: a file written before US6 (M3–M6) reads unchanged; the three values round-trip; an
/// invalid one is dropped.
#[test]
fn the_us6_values_are_remembered_and_older_files_read_unchanged() {
    let older = RememberedSelector::from_json(
        br#"{"mode":"advanced","basic":{"include":["bug"],"exclude":[],"state":"closed"},"query":"label:a","ran":true}"#,
    );
    assert_eq!(labels(older.basic.include()), ["bug"]);
    assert_eq!(older.basic.state(), IssueState::Closed);
    assert_eq!(older.basic.assignee(), None);
    assert_eq!(older.basic.author(), None);
    assert_eq!(older.basic.milestone(), None);
    assert_eq!(older.query, "label:a");

    let held = RememberedSelector::default().with_change(&SelectorChange::Basic(us6_filters()));
    let read = RememberedSelector::from_json(&held.to_json());
    assert_eq!(read, held);
    let mut nobody = BasicFilters::default();
    nobody.set_assignee(Some(Assignee::Nobody));
    nobody.set_milestone(Some(Milestone::Nobody));
    let round: BasicFilters =
        serde_json::from_slice(&serde_json::to_vec(&nobody).unwrap()).unwrap();
    assert_eq!(round, nobody);
    let mut person = BasicFilters::default();
    person.set_assignee(Some(Assignee::User(login("hubot"))));
    let round: BasicFilters =
        serde_json::from_slice(&serde_json::to_vec(&person).unwrap()).unwrap();
    assert_eq!(round, person);

    // The default filter is written as before US6: no new key.
    let written = String::from_utf8(serde_json::to_vec(&BasicFilters::default()).unwrap()).unwrap();
    assert!(!written.contains("assignee"), "{written}");

    let invalid: BasicFilters = serde_json::from_value(json!({
        "assignee": {"user": "-bad"},
        "author": "sideways",
        "milestone": {"titled": "say \"hi\""}
    }))
    .expect("lenient");
    assert_eq!(invalid, BasicFilters::default());
}

#[test]
fn us6_values_never_print_a_login_or_title() {
    let mut filters = BasicFilters::default();
    filters.set_assignee(Some(Assignee::User(login("secret-assignee"))));
    filters.set_author(Some(Author::User(login("secret-author"))));
    filters.set_milestone(Some(Milestone::Titled(milestone("secret-milestone"))));
    let printed = format!(
        "{filters:?} {:?} {:?}",
        login("secret"),
        milestone("secret")
    );
    assert!(!printed.contains("secret"), "{printed}");
}

#[test]
fn milestone_choices_are_sorted_by_title_and_quoted_titles_not_offered() {
    let source = FakeIssueSource::new().with_milestones(vec![
        names(&["v2.4", "alpha", "say \"hi\""], Some("C1")),
        names(&["Beta", "v1.0", "alpha"], None),
    ]);
    let choices: MilestoneChoices = load_milestone_choices(&source, &repo()).expect("choices");
    let titles: Vec<&str> = choices
        .milestones
        .iter()
        .map(MilestoneTitle::as_str)
        .collect();
    assert_eq!(titles, ["alpha", "Beta", "v1.0", "v2.4"]);
    assert_eq!(choices.not_offered, 1);
    assert_eq!(source.milestone_calls(), [None, Some("C1".to_string())]);

    let failing = FakeIssueSource::new().with_milestones(vec![Err(IssueLoadError::RateLimited)]);
    assert_eq!(
        load_milestone_choices(&failing, &repo()).unwrap_err(),
        IssueLoadError::RateLimited
    );
}

/// data-model §6: people with a valid login, in GitHub's order; a redacted `Debug`.
#[test]
fn people_skip_invalid_logins_and_never_print_one() {
    let answer = json!({"data": {"repository": {"assignableUsers": {"nodes": [
        {"login": "octocat", "name": "The Octocat"},
        {"login": "octo-cat", "name": ""},
        {"login": "-bad", "name": "Bad"},
        {"login": "hubot", "name": null},
        {"name": "No login"}
    ]}}}});
    let people = parse_people(answer.to_string().as_bytes()).expect("people");
    let logins: Vec<&str> = people.iter().map(|p| p.login.as_str()).collect();
    assert_eq!(logins, ["octocat", "octo-cat", "hubot"]);
    let named: Vec<Option<&str>> = people.iter().map(|p| p.name.as_deref()).collect();
    assert_eq!(named, [Some("The Octocat"), None, None]);
    let printed = format!("{people:?}");
    assert!(
        !printed.contains("octo") && !printed.contains("hubot"),
        "{printed}"
    );

    let refused = json!({"data": null, "errors": [{"type": "RATE_LIMITED", "message": "slow"}]});
    assert_eq!(
        parse_people(refused.to_string().as_bytes()).unwrap_err(),
        IssueLoadError::RateLimited
    );
}

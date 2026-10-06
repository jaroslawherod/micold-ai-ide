//! Descriptions follow the list in a second pass (feature 038, FR-024, FR-026, SC-008;
//! contracts/issue-fields.md §1, §6; research R14).
//!
//! With `bodyText` in the list query a list of 1,000 issues took 1.88 times as long to appear. So
//! the list is read without it, and the same connection is read again for `number bodyText` once
//! the list is on screen. These tests hold the query, its arguments, its parsing, the merge into
//! the held issues and the rule that ends the pass.

use std::path::PathBuf;

use micold_core::github::{
    describe_listed, descriptions_args, list_args, next_description_cursor,
    parse_descriptions_page, parse_list_page, DescriptionPage, FakeIssueSource, GithubRepo, Issue,
    IssueLoadError, IssueSource, DESCRIPTIONS_QUERY, DESCRIPTION_MAX_CHARS, DESCRIPTION_PAGE_CAP,
    ISSUE_LOAD_CAP, ISSUE_NODE_SELECTION, LIST_QUERY, SEARCH_QUERY, SEARCH_WITH_NUMBER_QUERY,
};

/// A body no test output may show.
const BODY: &str = "SECRET-BODY-TEXT";

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/o/r").expect("a GitHub remote")
}

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A description page's answer holding `nodes`, with a next page at `cursor` when given.
fn answer(nodes: &str, cursor: Option<&str>) -> Vec<u8> {
    let page_info = match cursor {
        Some(cursor) => format!(r#"{{"hasNextPage":true,"endCursor":"{cursor}"}}"#),
        None => r#"{"hasNextPage":false,"endCursor":null}"#.to_string(),
    };
    format!(
        r#"{{"data":{{"repository":{{"issues":{{"pageInfo":{page_info},"nodes":[{nodes}]}}}}}}}}"#
    )
    .into_bytes()
}

fn page(descriptions: &[(u64, &str)], next_cursor: Option<&str>) -> DescriptionPage {
    DescriptionPage {
        descriptions: descriptions
            .iter()
            .map(|(number, text)| (*number, (*text).to_string()))
            .collect(),
        next_cursor: next_cursor.map(str::to_string),
    }
}

fn issue(number: u64) -> Issue {
    Issue::new(number, format!("Issue {number}"), vec![], "t".into()).reported_by("ana")
}

/// U27, U90 — the list query asks for no body, so the list arrives as fast as it did before the
/// description (SC-008); the searches, one small request each, still do.
#[test]
fn the_list_query_asks_for_no_body() {
    assert!(
        !LIST_QUERY.contains("body"),
        "the list is read without any rendering of the body: {LIST_QUERY}"
    );
    assert!(
        !ISSUE_NODE_SELECTION.contains("body"),
        "the selection every query shares holds no body: {ISSUE_NODE_SELECTION}"
    );
    for (name, query, nodes) in [
        ("SEARCH_QUERY", SEARCH_QUERY, 1),
        ("SEARCH_WITH_NUMBER_QUERY", SEARCH_WITH_NUMBER_QUERY, 2),
    ] {
        assert_eq!(
            query.matches("bodyText").count(),
            nodes,
            "{name} asks for the body as text once per issue node"
        );
    }
}

/// U91 — the pass reads the connection the list reads, in its order and page size, and selects
/// only the number and the body as text.
#[test]
fn the_descriptions_query_reads_the_lists_connection() {
    let connection = "issues(states: OPEN, first: 100, after: $cursor, \
                      orderBy: {field: UPDATED_AT, direction: DESC})";
    for (name, query) in [
        ("LIST_QUERY", LIST_QUERY),
        ("DESCRIPTIONS_QUERY", DESCRIPTIONS_QUERY),
    ] {
        assert!(
            query.contains(connection),
            "{name} reads the open issues, most recently updated first, 100 a page: {query}"
        );
    }
    assert!(
        DESCRIPTIONS_QUERY.contains("pageInfo { hasNextPage endCursor } nodes { number bodyText }"),
        "only the number and the body as text are selected: {DESCRIPTIONS_QUERY}"
    );
    for field in ["title", "labels", "author", "updatedAt", "totalCount"] {
        assert!(
            !DESCRIPTIONS_QUERY.contains(field),
            "the pass does not read {field} again: {DESCRIPTIONS_QUERY}"
        );
    }
    assert!(
        !DESCRIPTIONS_QUERY.contains('\n'),
        "one line, like every query `gh` receives"
    );
}

/// U91 — what the pass sends is what the list sends, but for the query (FR-026).
#[test]
fn the_descriptions_arguments_are_the_lists() {
    for cursor in [None, Some("CUR")] {
        let listed = list_args(&repo(), cursor);
        let described = descriptions_args(&repo(), cursor);
        assert_eq!(described.len(), listed.len(), "as many arguments");
        for (sent, list) in described.iter().zip(&listed) {
            if list.starts_with("query=") {
                assert_eq!(sent, &format!("query={DESCRIPTIONS_QUERY}"));
            } else {
                assert_eq!(sent, list, "every other argument is the list's");
            }
        }
    }
    assert_eq!(
        DESCRIPTION_PAGE_CAP * 100,
        ISSUE_LOAD_CAP,
        "the pass reads as many pages as hold the load cap, and no more"
    );
}

/// U92 — a page gives each issue's number with its folded, bounded description, and where the
/// next page starts.
#[test]
fn a_descriptions_page_parses_to_numbers_and_descriptions() {
    let long = "x".repeat(DESCRIPTION_MAX_CHARS + 50);
    let nodes = format!(
        r#"{{"number":7,"bodyText":"Problem\nThe list cuts  long titles off."}},
           {{"number":8,"bodyText":"{long}"}},
           {{"number":9,"bodyText":null}},
           {{"number":10}},
           {{"number":11,"bodyText":"  \n "}},
           {{"bodyText":"a node without a number"}}"#
    );
    let page = parse_descriptions_page(&answer(&nodes, Some("NEXT"))).expect("a well-formed page");
    assert_eq!(page.next_cursor.as_deref(), Some("NEXT"));
    let numbers: Vec<u64> = page.descriptions.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        numbers,
        [7, 8, 9, 10, 11],
        "every numbered node, in order; the one without a number is skipped"
    );
    assert_eq!(
        page.descriptions[0].1,
        "Problem The list cuts long titles off."
    );
    assert_eq!(
        page.descriptions[1].1.chars().count(),
        DESCRIPTION_MAX_CHARS + 1,
        "cut to the limit and marked"
    );
    for (number, description) in &page.descriptions[2..] {
        assert_eq!(description, "", "#{number} has no text to show");
    }

    let last = parse_descriptions_page(&answer(r#"{"number":7,"bodyText":"a"}"#, None))
        .expect("the last page");
    assert_eq!(last.next_cursor, None, "no next page");
}

/// U92 — the scenario's hand-written fixture node gives, through the two passes, the description it gave when
/// it rode the list (US3 scenario 12).
#[test]
fn the_two_passes_give_the_scenarios_description() {
    let node = fixture("issue_node_description.json");
    let descriptions = parse_descriptions_page(&answer(&node, None)).expect("a page");
    let mut held = vec![issue(519), issue(520)];
    describe_listed(&mut held, &descriptions);
    assert_eq!(
        held[0].description(),
        "Problem The list cuts long titles off."
    );
    assert_eq!(held[1].description(), "", "an issue the page does not name");
}

/// U92 — GraphQL errors are classified as the list's are.
#[test]
fn a_descriptions_error_is_classified_like_the_lists() {
    for (kind, expected) in [
        ("NOT_FOUND", IssueLoadError::NoAccess),
        ("RATE_LIMITED", IssueLoadError::RateLimited),
    ] {
        let stdout = format!(r#"{{"data":null,"errors":[{{"type":"{kind}","message":"m"}}]}}"#);
        assert_eq!(
            parse_descriptions_page(stdout.as_bytes()),
            Err(expected.clone()),
            "{kind}"
        );
        assert_eq!(
            parse_list_page(stdout.as_bytes()).map(drop),
            Err(expected),
            "as for the list"
        );
    }
}

/// U92 — neither a malformed answer's error nor a page's `Debug` shows any part of a body
/// (FR-025).
#[test]
fn neither_an_error_nor_a_page_shows_a_body() {
    for malformed in [
        format!(r#"{{"data":{{"repository":{{"bodyText":"{BODY}"}}}}}}"#),
        format!(r#"{{"data":{{"repository":{{"issues":{{"nodes":[{{"bodyText":"{BODY}"#),
        format!(r#"{{"data":{{"bodyText":"{BODY}" {BODY} }}}}"#),
        format!(r#"{{"data":{{"repository":{{"issues":{{"nodes":"{BODY}"}}}}}}}}"#),
    ] {
        let error = parse_descriptions_page(malformed.as_bytes()).expect_err("not a page");
        for shown in [format!("{error:?}"), error.message(&repo())] {
            assert!(!shown.contains(BODY), "the error shows the body: {shown}");
        }
    }

    let nodes = format!(r#"{{"number":7,"bodyText":"{BODY}"}}"#);
    let page = parse_descriptions_page(&answer(&nodes, None)).expect("a page");
    assert_eq!(page.descriptions[0].1, BODY, "the page holds it");
    let shown = format!("{page:?}");
    assert!(!shown.contains(BODY), "`Debug` shows the body: {shown}");
    assert!(
        shown.contains('1'),
        "`Debug` says how many it holds: {shown}"
    );
}

/// U93 — a page's descriptions land on the held issues with those numbers, and nothing else moves.
#[test]
fn describe_listed_matches_by_number() {
    let mut held = vec![issue(3), issue(2).described("kept"), issue(1)];
    let before: Vec<(u64, String, String)> = held
        .iter()
        .map(|i| (i.number(), i.title().to_string(), i.row_text().to_string()))
        .collect();
    describe_listed(
        &mut held,
        &page(&[(1, "one"), (99, "not held"), (3, "three")], None),
    );
    assert_eq!(held[0].description(), "three");
    assert_eq!(
        held[1].description(),
        "kept",
        "a page that does not name it"
    );
    assert_eq!(held[2].description(), "one");
    let after: Vec<(u64, String, String)> = held
        .iter()
        .map(|i| (i.number(), i.title().to_string(), i.row_text().to_string()))
        .collect();
    assert_eq!(
        after, before,
        "order, titles and match text are as they were"
    );
}

/// U94 — the pass goes on while pages remain and ends on the last page, on a page that adds
/// nothing, on a cursor it was just asked with, and once the cap's pages are read.
#[test]
fn the_pass_ends_where_the_list_would() {
    let more = page(&[(1, "a")], Some("B"));
    assert_eq!(
        next_description_cursor(None, &more, 1),
        Some("B".to_string()),
        "the first page names the second"
    );
    assert_eq!(
        next_description_cursor(Some("A"), &more, DESCRIPTION_PAGE_CAP - 1),
        Some("B".to_string()),
        "the page before the cap's last is followed"
    );
    assert_eq!(
        next_description_cursor(Some("A"), &page(&[(1, "a")], None), 2),
        None,
        "the last page"
    );
    assert_eq!(
        next_description_cursor(Some("A"), &page(&[], Some("B")), 2),
        None,
        "a page with no issue would be asked for again forever"
    );
    assert_eq!(
        next_description_cursor(Some("B"), &more, 2),
        None,
        "a page that hands back the cursor it was asked with"
    );
    assert_eq!(
        next_description_cursor(Some("A"), &more, DESCRIPTION_PAGE_CAP),
        None,
        "the cap's pages are read: no request beyond them (FR-026)"
    );
}

/// U94 — the fake source answers `describe_open` in order and records what was asked.
#[test]
fn the_fake_source_scripts_and_records_description_pages() {
    let source = FakeIssueSource::new()
        .with_descriptions(Ok(page(&[(1, "a")], Some("B"))))
        .with_descriptions(Err(IssueLoadError::Offline));
    assert_eq!(
        source.describe_open(&repo(), None),
        Ok(page(&[(1, "a")], Some("B")))
    );
    assert_eq!(
        source.describe_open(&repo(), Some("B")),
        Err(IssueLoadError::Offline)
    );
    assert!(
        source.describe_open(&repo(), Some("C")).is_err(),
        "nothing scripted is an error, not a hang"
    );
    assert_eq!(
        source.description_calls(),
        [
            ("o/r".to_string(), None),
            ("o/r".to_string(), Some("B".to_string())),
            ("o/r".to_string(), Some("C".to_string())),
        ]
    );
    assert!(source.calls().is_empty(), "no list page was asked for");
}

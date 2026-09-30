//! Reading one page of open issues from `gh api graphql`, and what is sent to get it (feature 034,
//! contracts/github-issue-source.md §3–4, research R2).

use std::path::PathBuf;

use micold_core::github::{
    list_args, parse_list_page, parse_search, search_args, GithubRepo, Issue, IssueLoadError,
    LIST_QUERY, SEARCH_QUERY, SEARCH_WITH_NUMBER_QUERY,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn repo(url: &str) -> GithubRepo {
    GithubRepo::from_remote_url(url).expect("a GitHub remote")
}

/// The GitHub label display is truncated long before this; the query asks for this many.
const LABELS_PER_ISSUE: usize = 20;

#[test]
fn a_list_page_parses() {
    let page = parse_list_page(&fixture("list_page.json")).expect("a well-formed page");
    assert_eq!(page.total_open, 142, "totalCount is the open-issue count");
    assert_eq!(
        page.next_cursor.as_deref(),
        Some("Y3Vyc29yOnYyOpK5"),
        "a page with more after it carries its end cursor"
    );
    let numbers: Vec<u64> = page.issues.iter().map(Issue::number).collect();
    assert_eq!(numbers, [42, 7, 99], "GitHub's order is kept");
    let first = &page.issues[0];
    assert_eq!(
        first.title(),
        "Crash when opening empty project",
        "the title as GitHub sent it"
    );
    assert_eq!(
        first.updated_at(),
        "2026-09-28T10:00:00Z",
        "updatedAt as GitHub sent it"
    );
    assert_eq!(
        first.labels(),
        ["bug", "good first issue"],
        "label names in GitHub's order"
    );
    assert_eq!(
        page.issues[2].labels().len(),
        LABELS_PER_ISSUE,
        "at most 20 labels are held per issue"
    );

    let last = parse_list_page(&fixture("list_page_last.json")).expect("a last page");
    assert_eq!(
        last.next_cursor, None,
        "`hasNextPage: false` ends paging even though an end cursor is present"
    );
}

#[test]
fn row_text_shows_labels_only_when_present() {
    let page = parse_list_page(&fixture("list_page.json")).unwrap();
    assert_eq!(
        page.issues[0].row_text(),
        "#42 Crash when opening empty project  ·  bug, good first issue",
        "number, title, then the labels after a separator"
    );
    assert_eq!(
        page.issues[1].row_text(),
        "#7 Document the settings file",
        "no separator when the issue has no labels"
    );
    let built = Issue::new(5, "Title".into(), vec!["docs".into()], "t".into());
    assert_eq!(
        built.row_text(),
        "#5 Title  ·  docs",
        "an issue built in code writes the same row"
    );
}

#[test]
fn graphql_errors_are_classified() {
    assert_eq!(
        parse_list_page(&fixture("list_not_found.json")).unwrap_err(),
        IssueLoadError::NoAccess,
        "an unresolvable repository is one the sign-in cannot see"
    );
    assert_eq!(
        parse_list_page(&fixture("list_rate_limited.json")).unwrap_err(),
        IssueLoadError::RateLimited,
        "a RATE_LIMITED error type is a rate limit"
    );
    assert!(
        matches!(
            parse_list_page(b"<html>502 Bad Gateway</html>"),
            Err(IssueLoadError::Other(_))
        ),
        "anything that is not the expected JSON is `Other`, never a panic"
    );
    assert!(
        matches!(
            parse_list_page(br#"{"data":{}}"#),
            Err(IssueLoadError::Other(_))
        ),
        "JSON without the issues connection is `Other`"
    );
}

#[test]
fn list_args_send_only_the_repository() {
    let o_r = repo("https://github.com/o/r");
    assert_eq!(
        list_args(&o_r, None),
        [
            "api",
            "graphql",
            "--hostname",
            "github.com",
            "-f",
            &format!("query={LIST_QUERY}"),
            "-f",
            "owner=o",
            "-f",
            "name=r",
        ],
        "the first page: the query, owner and name as raw strings, nothing else"
    );
    let paged = list_args(&o_r, Some("CUR"));
    assert_eq!(
        &paged[paged.len() - 2..],
        ["-f", "cursor=CUR"],
        "the cursor rides as a raw string, only when there is one"
    );
    assert!(
        !paged.iter().any(|a| a == "-F"),
        "`-F` would turn a repository named `1` or `true` into a number or a boolean"
    );

    let odd = repo("https://github.com/1/true");
    let args = list_args(&odd, None);
    assert!(
        args.contains(&"owner=1".to_string()) && args.contains(&"name=true".to_string()),
        "owner and name are sent as written: {args:?}"
    );
    assert!(
        args.windows(2)
            .filter(|w| w[1].starts_with("owner=") || w[1].starts_with("name="))
            .all(|w| w[0] == "-f"),
        "every string variable uses `-f`: {args:?}"
    );

    // FR-025: nothing about the project but owner/name leaves the machine.
    let values: Vec<&String> = args.iter().filter(|a| a.contains('=')).collect();
    assert_eq!(
        values.len(),
        3,
        "query, owner and name are the only values sent: {values:?}"
    );
}

// --- M3: the search beyond the loaded issues (FR-005a) -------------------------------------

fn numbers(issues: &[Issue]) -> Vec<u64> {
    issues.iter().map(Issue::number).collect()
}

/// U66 — the search hits and the numbered lookup are one result: open only, each number once, in
/// the order GitHub gave them (search first).
#[test]
fn search_unions_and_dedupes() {
    let found = parse_search(&fixture("search_number.json")).expect("a well-formed answer");
    assert_eq!(
        numbers(&found),
        [1200, 1300],
        "#1200 came from both the search and the lookup, and is held once"
    );
    assert_eq!(
        found[0].title(),
        "Beyond the cap",
        "a search hit keeps its title"
    );
    assert_eq!(
        found[0].labels(),
        ["enhancement"],
        "a search hit keeps its labels"
    );
    assert_eq!(
        found[0].updated_at(),
        "2026-01-04T00:00:00Z",
        "a search hit keeps its updatedAt"
    );

    let plain = parse_search(
        br#"{"data":{"search":{"nodes":[
        {"number":5,"title":"Five","updatedAt":"t","state":"OPEN","labels":{"nodes":[]}},
        {"number":6,"title":"Six","updatedAt":"t","state":"CLOSED","labels":{"nodes":[]}}]}}}"#,
    )
    .expect("an answer without a lookup");
    assert_eq!(
        numbers(&plain),
        [5],
        "a closed search hit is not an open issue"
    );
}

/// U67 — a number that is a pull request's, or that does not exist, is "no such open issue": the
/// search hits stay and no error is raised. A closed numbered issue is dropped. Any other error is
/// classified.
#[test]
fn a_missing_number_is_not_an_error() {
    assert_eq!(
        numbers(&parse_search(&fixture("search_pr_number.json")).expect("hits kept")),
        [4312],
        "a pull request's number keeps the search hits; the empty node is the pull request"
    );
    assert_eq!(
        parse_search(&fixture("search_missing_number.json")).expect("no error"),
        [],
        "a number with no issue behind it is simply not found"
    );
    assert_eq!(
        numbers(&parse_search(&fixture("search_closed_number.json")).expect("no error")),
        [1500],
        "the lookup can answer a closed issue; it is not open, so it is dropped"
    );

    assert_eq!(
        parse_search(
            br#"{"data":{"search":null,"repository":null},
                 "errors":[{"type":"NOT_FOUND","path":["repository"],"message":"no repo"}]}"#
        )
        .unwrap_err(),
        IssueLoadError::NoAccess,
        "NOT_FOUND anywhere else is the repository the sign-in cannot see"
    );
    assert_eq!(
        parse_search(
            br#"{"data":{"search":{"nodes":[]},"repository":{"issue":null}},
                 "errors":[{"type":"NOT_FOUND","path":["repository","issue"]},
                           {"type":"RATE_LIMITED","message":"slow down"}]}"#
        )
        .unwrap_err(),
        IssueLoadError::RateLimited,
        "only a sole NOT_FOUND on the lookup is forgiven"
    );
    assert!(
        matches!(parse_search(b"not json"), Err(IssueLoadError::Other(_))),
        "anything that is not the expected JSON is `Other`"
    );
    assert!(
        matches!(
            parse_search(br#"{"data":{}}"#),
            Err(IssueLoadError::Other(_))
        ),
        "JSON without the search is `Other`"
    );
}

/// U68 — what is sent to search: the text inside `q`, and — only for `N` or `#N` fitting GraphQL's
/// `Int` — the repository and the number for the lookup (FR-025, research R2).
#[test]
fn search_args_send_only_the_query() {
    let o_r = repo("https://github.com/o/r");
    let head = ["api", "graphql", "--hostname", "github.com", "-f"];
    assert_eq!(
        search_args(&o_r, "crash on open"),
        [
            &head[..],
            &[
                &format!("query={SEARCH_QUERY}"),
                "-f",
                "q=repo:o/r is:issue is:open crash on open",
            ][..],
        ]
        .concat(),
        "plain text: one variable, the search query"
    );

    for typed in ["4312", "#4312"] {
        assert_eq!(
            search_args(&o_r, typed),
            [
                &head[..],
                &[
                    &format!("query={SEARCH_WITH_NUMBER_QUERY}"),
                    "-f",
                    &format!("q=repo:o/r is:issue is:open {typed}"),
                    "-f",
                    "owner=o",
                    "-f",
                    "name=r",
                    "-F",
                    "n=4312",
                ][..],
            ]
            .concat(),
            "{typed}: the number is looked up as well"
        );
    }

    let too_big = search_args(&o_r, "2147483648");
    assert_eq!(
        too_big[5],
        format!("query={SEARCH_QUERY}"),
        "a number past Int's range cannot be an issue number"
    );
    assert!(
        !too_big.iter().any(|a| a == "-F"),
        "no typed number variable is sent for text: {too_big:?}"
    );
    assert_eq!(
        search_args(&o_r, "2147483647")[5],
        format!("query={SEARCH_WITH_NUMBER_QUERY}"),
        "Int's largest value is still looked up"
    );
    assert_eq!(
        search_args(&o_r, "42 crash")[5],
        format!("query={SEARCH_QUERY}"),
        "a number followed by words is text"
    );
}

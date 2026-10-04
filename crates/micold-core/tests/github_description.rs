//! An issue's description: the start of its body as one plain paragraph (feature 038, FR-020 to
//! FR-022, FR-024, FR-026; contracts/issue-fields.md §1–2, data-model §2).
//!
//! GitHub renders the body to text (`bodyText`): Markdown markers, link addresses and HTML comments
//! are already gone. `description_from` folds what is left into one line and bounds its length, so
//! what a row hands its tooltip is small whatever the body's size.

use std::path::PathBuf;

use micold_core::github::{
    description_from, list_args, parse_list_page, parse_search, search_args, GithubRepo, Issue,
    DESCRIPTION_MAX_CHARS, ISSUE_LOAD_CAP, ISSUE_NODE_SELECTION, LIST_QUERY, SEARCH_QUERY,
    SEARCH_WITH_NUMBER_QUERY,
};

/// What a cut description ends in.
const ELLIPSIS: char = '…';

/// The longest body GitHub accepts, in characters.
const GITHUB_BODY_MAX: usize = 65_536;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The one issue of a listing page whose only node is `node`.
fn listed(node: &str) -> Issue {
    let page = format!(
        r#"{{"data":{{"repository":{{"issues":{{"totalCount":1,
            "pageInfo":{{"hasNextPage":false,"endCursor":null}},"nodes":[{node}]}}}}}}}}"#
    );
    let mut page = parse_list_page(page.as_bytes()).expect("a well-formed page");
    assert_eq!(page.issues.len(), 1, "the page holds one node");
    page.issues.remove(0)
}

/// A node with every field but the body, then `body_field` (a `"bodyText": …` member, or nothing).
fn node_with(body_field: &str) -> String {
    let comma = if body_field.is_empty() { "" } else { "," };
    format!(
        r#"{{"number":7,"title":"T","updatedAt":"t","labels":{{"nodes":[]}},
            "author":{{"login":"ana"}}{comma}{body_field}}}"#
    )
}

/// U22 — every run of Unicode whitespace becomes one space, and both ends are trimmed.
#[test]
fn whitespace_runs_fold_to_one_space_and_the_ends_are_trimmed() {
    assert_eq!(
        description_from("  Problem\n\n\tThe list\r\n cuts   titles off. \n"),
        "Problem The list cuts titles off.",
        "line breaks, tabs and runs of spaces are one space each"
    );
    assert_eq!(
        description_from("\u{a0}a\u{2003}\u{2028}b\u{3000}"),
        "a b",
        "Unicode whitespace counts: no-break space, em space, line separator, ideographic space"
    );
    assert_eq!(
        description_from("already one line"),
        "already one line",
        "a text with nothing to fold is unchanged"
    );
}

/// U23 — more than 600 characters is cut on a character boundary to 600, trimmed, and marked.
#[test]
fn a_long_text_is_cut_to_the_limit_and_marked() {
    assert_eq!(DESCRIPTION_MAX_CHARS, 600, "the limit the contract names");

    let exact = "x".repeat(DESCRIPTION_MAX_CHARS);
    assert_eq!(
        description_from(&exact),
        exact,
        "exactly the limit is shown whole, without a mark"
    );

    let over = "x".repeat(DESCRIPTION_MAX_CHARS + 1);
    let cut = description_from(&over);
    assert_eq!(
        cut.chars().count(),
        DESCRIPTION_MAX_CHARS + 1,
        "600 characters and the mark"
    );
    assert!(cut.ends_with(ELLIPSIS), "a cut text ends in the mark");
    assert!(
        cut.starts_with(&exact),
        "what is kept is the start of the text"
    );

    // The limit counts characters, not bytes: each `é` is two bytes.
    let accented = "é".repeat(DESCRIPTION_MAX_CHARS + 5);
    let cut = description_from(&accented);
    assert_eq!(
        cut,
        format!("{}{ELLIPSIS}", "é".repeat(DESCRIPTION_MAX_CHARS)),
        "the cut falls on a character boundary, after 600 characters"
    );

    // The 600th character is a space: it is dropped before the mark.
    let spaced = format!("{} tail of the text", "x".repeat(DESCRIPTION_MAX_CHARS - 1));
    assert_eq!(
        description_from(&spaced),
        format!("{}{ELLIPSIS}", "x".repeat(DESCRIPTION_MAX_CHARS - 1)),
        "no space is left before the mark"
    );
}

/// U23 — the limit is measured after folding: a body that is long only by its blank space is whole.
#[test]
fn the_limit_is_measured_on_the_folded_text() {
    let padded = format!("a{}b", " ".repeat(DESCRIPTION_MAX_CHARS * 2));
    assert_eq!(
        description_from(&padded),
        "a b",
        "blank space does not count toward the limit"
    );
}

/// U24 — an input with no non-whitespace character gives the empty string.
#[test]
fn a_blank_text_gives_no_description() {
    for blank in ["", " ", "  \n\n ", "\t\r\n", "\u{a0}\u{2003}"] {
        assert_eq!(
            description_from(blank),
            "",
            "{blank:?} holds nothing to show"
        );
    }
}

/// U25, A27 — story 3 scenario 12: a hidden comment, `## Problem`, a blank line and a sentence with
/// emphasis and a link. GitHub's `bodyText` for it is the heading's word, a line break, the plain
/// sentence; the description joins them with one space.
#[test]
fn the_scenario_body_parses_to_one_plain_paragraph() {
    let issue = listed(&fixture("issue_node_description.json"));
    assert_eq!(issue.number(), 519, "the captured node is issue 519");
    assert_eq!(
        issue.description(),
        "Problem The list cuts long titles off.",
        "no marker, no address, no comment text, no line break"
    );
}

/// U25, A28 — story 3 scenario 13: a body that is only an HTML comment renders to no text.
#[test]
fn a_comment_only_body_parses_to_no_description() {
    let issue = listed(&fixture("issue_node_comment_only.json"));
    assert_eq!(issue.number(), 520, "the issue is still listed");
    assert_eq!(issue.description(), "", "there is nothing to show");
}

/// U25 — a node parses to the same description from the listing, the search and the typed-number
/// lookup (FR-006, story 3 scenario 11).
#[test]
fn the_description_parses_alike_from_every_source() {
    let node = fixture("issue_node_description.json");
    let listed = listed(&node);
    let searched =
        parse_search(format!(r#"{{"data":{{"search":{{"nodes":[{node}]}}}}}}"#).as_bytes())
            .expect("a search answer");
    let looked_up = parse_search(
        format!(r#"{{"data":{{"search":{{"nodes":[]}},"repository":{{"issue":{node}}}}}}}"#)
            .as_bytes(),
    )
    .expect("a lookup answer");
    for (source, issues) in [("search", searched), ("lookup", looked_up)] {
        assert_eq!(issues.len(), 1, "the {source} answers one issue");
        assert_eq!(
            issues[0].description(),
            listed.description(),
            "the {source} gives the description the listing gives"
        );
        assert!(
            !issues[0].description().is_empty(),
            "and it is the fixture's text"
        );
    }
}

/// U26, A22 — a body that is `null`, absent, empty or blank gives no description, and the issue is
/// still listed.
#[test]
fn a_node_without_a_body_is_listed_with_no_description() {
    for body_field in [
        r#""bodyText": null"#,
        "",
        r#""bodyText": """#,
        r#""bodyText": "  \n\n ""#,
    ] {
        let issue = listed(&node_with(body_field));
        assert_eq!(issue.number(), 7, "listed with {body_field:?}");
        assert_eq!(
            issue.description(),
            "",
            "no description from {body_field:?}"
        );
    }
}

/// U26 — a body of GitHub's maximum length gives 600 characters and the mark.
#[test]
fn the_longest_body_gives_a_bounded_description() {
    let body = "word ".repeat(GITHUB_BODY_MAX / 5 + 1);
    let body: String = body.chars().take(GITHUB_BODY_MAX).collect();
    assert_eq!(body.chars().count(), GITHUB_BODY_MAX, "the fixture's length");
    let issue = listed(&node_with(&format!(r#""bodyText": "{body}""#)));
    let description = issue.description();
    assert!(
        description.chars().count() <= DESCRIPTION_MAX_CHARS + 1,
        "at most 600 characters and the mark: {}",
        description.chars().count()
    );
    assert!(description.ends_with(ELLIPSIS), "cut, so marked");

    let unbroken = "x".repeat(GITHUB_BODY_MAX);
    let issue = listed(&node_with(&format!(r#""bodyText": "{unbroken}""#)));
    assert_eq!(
        issue.description().chars().count(),
        DESCRIPTION_MAX_CHARS + 1,
        "a body without spaces gives exactly 601 characters"
    );
}

/// `Issue::described` applies the same rule as parsing does.
#[test]
fn described_holds_the_folded_bounded_text() {
    let issue = Issue::new(7, "T".into(), vec![], "t".into()).described(" a \n b ");
    assert_eq!(issue.description(), "a b", "folded and trimmed");
    let plain = Issue::new(7, "T".into(), vec![], "t".into());
    assert_eq!(plain.description(), "", "an issue has none until described");
}

/// U27 — every query asks for the body through the one shared selection, once per issue node.
#[test]
fn every_query_asks_for_the_body_text_in_the_shared_selection() {
    assert!(
        ISSUE_NODE_SELECTION.contains("bodyText"),
        "the shared selection asks for the body as text: {ISSUE_NODE_SELECTION}"
    );
    for (name, query, nodes) in [
        ("LIST_QUERY", LIST_QUERY, 1),
        ("SEARCH_QUERY", SEARCH_QUERY, 1),
        ("SEARCH_WITH_NUMBER_QUERY", SEARCH_WITH_NUMBER_QUERY, 2),
    ] {
        assert_eq!(
            query.matches("bodyText").count(),
            nodes,
            "{name} names the body only inside the shared selection"
        );
        assert!(
            !query.contains("bodyHTML") && !query.contains(" body "),
            "{name} asks for no other rendering of the body"
        );
    }
}

/// U27, U6 — what is sent is what was sent before the description: the same variables, the same
/// page size and cap, and nothing more about the project (FR-024, FR-026).
#[test]
fn the_request_arguments_are_unchanged() {
    let repo = GithubRepo::from_remote_url("https://github.com/o/r").expect("a GitHub remote");
    assert_eq!(
        list_args(&repo, Some("CUR")),
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
            "-f",
            "cursor=CUR",
        ],
        "the listing: the query, the repository and the cursor"
    );
    assert_eq!(
        search_args(&repo, "#42"),
        [
            "api",
            "graphql",
            "--hostname",
            "github.com",
            "-f",
            &format!("query={SEARCH_WITH_NUMBER_QUERY}"),
            "-f",
            "q=repo:o/r is:issue is:open #42",
            "-f",
            "owner=o",
            "-f",
            "name=r",
            "-F",
            "n=42",
        ],
        "the search with a number: the query, the text, the repository and the number"
    );
    assert_eq!(
        search_args(&repo, "crash").len(),
        8,
        "the plain search: the query and the text"
    );
    assert!(
        LIST_QUERY.contains("first: 100"),
        "the page size is unchanged: {LIST_QUERY}"
    );
    assert!(
        SEARCH_QUERY.contains("first: 50"),
        "the search size is unchanged: {SEARCH_QUERY}"
    );
    assert_eq!(ISSUE_LOAD_CAP, 1_000, "the load cap is unchanged");
    for args in [
        list_args(&repo, None),
        search_args(&repo, "crash"),
        search_args(&repo, "#42"),
    ] {
        for arg in args.iter().filter(|a| !a.starts_with("query=")) {
            assert!(
                !arg.contains("author:") && !arg.to_lowercase().contains("body"),
                "no variable names an author or a body: {arg}"
            );
        }
    }
}

/// U20 — the description is not part of the match text, so text found only in it matches nothing
/// (FR-014).
#[test]
fn the_match_text_never_holds_the_description() {
    let plain = Issue::new(7, "Fix it".into(), vec!["bug".into()], "t".into()).reported_by("ana");
    let described = plain.clone().described("A zebra crossing the list.");
    assert_eq!(
        described.description(),
        "A zebra crossing the list.",
        "the fixture issue holds the description"
    );
    assert_eq!(
        described.row_text(),
        plain.row_text(),
        "the match text is the same with and without a description"
    );
    assert!(
        !described.row_text().contains("zebra"),
        "a word of the description is not in it: {}",
        described.row_text()
    );
    assert!(
        !described.title_line().contains("zebra") && !described.details_line().contains("zebra"),
        "nor on either of the row's lines"
    );
}

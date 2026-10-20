//! Parsing a Jira Cloud `search/jql` page (feature 533, contracts/jira-cloud-source.md §4) from
//! hand-written bodies in `tests/fixtures/jira/`.

use micold_core::github::{Issue, IssueLoadError, GHOST_LOGIN};
use micold_core::jira::parse_page;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/jira/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

fn page() -> micold_core::github::IssuePage {
    parse_page(&fixture("search_page.json")).expect("a page")
}

fn by_key<'a>(issues: &'a [Issue], key: &str) -> &'a Issue {
    issues
        .iter()
        .find(|i| i.key() == Some(key))
        .unwrap_or_else(|| panic!("{key} parsed"))
}

#[test]
fn issues_are_keyed_and_a_key_without_a_number_is_skipped() {
    let page = page();
    let keys: Vec<_> = page.issues.iter().filter_map(Issue::key).collect();
    assert_eq!(keys, ["ABC-123", "ABC-124", "ABC-125", "ABC-126"]);
    let first = by_key(&page.issues, "ABC-123");
    assert_eq!(first.number(), 123);
    assert_eq!(first.title(), "Add login rate limit");
    assert_eq!(first.labels(), ["backend", "security"]);
    assert_eq!(first.updated_at(), "2026-10-14T09:30:11.123+0200");
    assert_eq!(first.type_name(), Some("Bug"));
    assert_eq!(first.title_line(), "ABC-123 Add login rate limit");
}

#[test]
fn the_cursor_is_the_next_page_token_and_the_count_is_the_page() {
    let page = page();
    assert_eq!(page.next_cursor.as_deref(), Some("CAEaAggD"));
    assert_eq!(page.total_open, 4, "Jira has no total: the issues parsed");
}

#[test]
fn the_last_page_has_no_cursor_even_when_a_token_comes_with_it() {
    let last = parse_page(&fixture("last_page.json")).expect("a page");
    assert_eq!(last.next_cursor, None);
    assert_eq!(last.total_open, 1);
    let stale = parse_page(&fixture("last_page_with_stale_token.json")).expect("a page");
    assert_eq!(stale.next_cursor, None, "isLast wins over a token");
    let empty = parse_page(&fixture("empty_page.json")).expect("a page");
    assert!(empty.issues.is_empty());
    assert_eq!(empty.total_open, 0);
}

#[test]
fn the_reporter_is_a_name_then_an_email_local_part_then_the_ghost_never_an_account_id() {
    let page = page();
    assert_eq!(by_key(&page.issues, "ABC-123").reporter(), "Mia Jansen");
    assert_eq!(by_key(&page.issues, "ABC-124").reporter(), "noah.k");
    assert_eq!(by_key(&page.issues, "ABC-125").reporter(), GHOST_LOGIN);
    assert_eq!(by_key(&page.issues, "ABC-126").reporter(), GHOST_LOGIN);
    for issue in &page.issues {
        assert!(
            !issue.row_text().contains("5b10a284"),
            "no account id in a row"
        );
    }
}

#[test]
fn an_adf_description_is_flattened_with_breaks_and_paragraphs_as_spaces() {
    let page = page();
    assert_eq!(
        by_key(&page.issues, "ABC-123").description(),
        "Throttle failed logins per account. five tries"
    );
    assert_eq!(by_key(&page.issues, "ABC-124").description(), "");
}

#[test]
fn the_row_text_carries_the_key_reporter_and_labels() {
    let page = page();
    assert_eq!(
        by_key(&page.issues, "ABC-123").row_text(),
        "ABC-123 Add login rate limit  ·  Mia Jansen  ·  backend, security"
    );
}

#[test]
fn at_most_twenty_labels_are_kept() {
    let labels: Vec<String> = (0..30).map(|n| format!("\"l{n}\"")).collect();
    let body = format!(
        r#"{{"issues":[{{"key":"ABC-1","fields":{{"summary":"s","labels":[{}]}}}}],"isLast":true}}"#,
        labels.join(",")
    );
    let page = parse_page(body.as_bytes()).expect("a page");
    assert_eq!(page.issues[0].labels().len(), 20);
}

#[test]
fn a_node_with_only_a_key_and_summary_is_still_an_issue() {
    let body = br#"{"issues":[{"key":"ABC-9","fields":{"summary":"bare"}}],"isLast":true}"#;
    let page = parse_page(body).expect("a page");
    assert_eq!(page.issues[0].updated_at(), "");
    assert_eq!(page.issues[0].type_name(), None);
    assert_eq!(page.issues[0].reporter(), GHOST_LOGIN);
}

#[test]
fn an_answer_that_is_not_a_listing_is_not_jira_cloud() {
    for body in [
        fixture("html_login_page.html"),
        fixture("data_center.json"),
        b"".to_vec(),
        b"[]".to_vec(),
        br#"{"issues":"nope"}"#.to_vec(),
    ] {
        assert_eq!(
            parse_page(&body).unwrap_err(),
            IssueLoadError::NotJiraCloud,
            "{}",
            String::from_utf8_lossy(&body)
        );
    }
}

#[test]
fn a_deeply_nested_description_does_not_overflow_the_stack() {
    let depth = 5_000;
    let nested = format!(
        "{}{}",
        r#"{"type":"bulletList","content":["#.repeat(depth),
        "]}".repeat(depth)
    );
    let body = format!(
        r#"{{"issues":[{{"key":"ABC-1","fields":{{"summary":"s","description":{nested}}}}}],"isLast":true}}"#
    );
    // serde_json refuses past its own recursion limit; either way no panic and a typed answer.
    match parse_page(body.as_bytes()) {
        Ok(page) => assert_eq!(page.issues[0].description(), ""),
        Err(e) => assert_eq!(e, IssueLoadError::NotJiraCloud),
    }
}

#[test]
fn the_status_name_and_category_are_read_and_a_missing_status_is_none() {
    use micold_core::jira_status::StatusCategory;
    let page = page();
    let review = by_key(&page.issues, "ABC-123").status().expect("a status");
    assert_eq!(review.name, "In Review");
    assert_eq!(review.category, Some(StatusCategory::InProgress));
    let todo = by_key(&page.issues, "ABC-124").status().expect("a status");
    assert_eq!(
        (todo.name.as_str(), todo.category),
        ("To Do", Some(StatusCategory::ToDo))
    );
    let done = by_key(&page.issues, "ABC-125").status().expect("a status");
    assert_eq!(done.category, Some(StatusCategory::Done));
    assert_eq!(by_key(&page.issues, "ABC-126").status(), None);
}

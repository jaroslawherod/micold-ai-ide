//! JQL building (feature 533, contracts/jira-cloud-source.md §3): the one place a project key and
//! a user's search text become query text, so the only things a query can carry.

use micold_core::jira::{list_jql, search_jql, ProjectKey};

fn key(text: &str) -> ProjectKey {
    ProjectKey::parse(text).expect("a valid key")
}

#[test]
fn the_listing_asks_for_the_projects_open_issues_newest_first() {
    assert_eq!(
        list_jql(&key("abc")),
        r#"project = "ABC" AND statusCategory != Done ORDER BY updated DESC"#
    );
}

#[test]
fn a_search_matches_summary_or_label_within_the_project() {
    assert_eq!(
        search_jql(&key("ABC"), "login").as_deref(),
        Some(
            r#"project = "ABC" AND statusCategory != Done AND (summary ~ "login*" OR labels = "login") ORDER BY updated DESC"#
        )
    );
}

#[test]
fn an_empty_search_builds_no_query() {
    assert_eq!(search_jql(&key("ABC"), ""), None);
    assert_eq!(search_jql(&key("ABC"), "  \t\n "), None);
    assert_eq!(
        search_jql(&key("ABC"), "\u{7}\n"),
        None,
        "control characters alone are empty"
    );
}

#[test]
fn quotes_and_backslashes_are_escaped_so_the_text_stays_a_string() {
    let jql = search_jql(&key("ABC"), r#"a" OR project = "XYZ\"#).expect("a query");
    assert!(
        jql.contains(r#"summary ~ "a\" OR project = \"XYZ\\*""#),
        "{jql}"
    );
    assert!(
        jql.contains(r#"labels = "a\" OR project = \"XYZ\\""#),
        "{jql}"
    );
}

#[test]
fn control_characters_and_newlines_are_removed_and_the_text_trimmed() {
    let jql = search_jql(&key("ABC"), "  lo\ngin\r\u{0}  ").expect("a query");
    assert!(jql.contains(r#"summary ~ "login*""#), "{jql}");
    assert!(!jql.chars().any(char::is_control), "{jql:?}");
}

#[test]
fn the_text_is_cut_at_a_hundred_characters() {
    let jql = search_jql(&key("ABC"), &"é".repeat(150)).expect("a query");
    assert!(
        jql.contains(&format!(r#"labels = "{}""#, "é".repeat(100))),
        "{jql}"
    );
    assert!(!jql.contains(&"é".repeat(101)));
}

#[test]
fn a_typed_key_of_this_project_adds_a_key_clause_upper_cased() {
    for typed in ["ABC-123", "abc-123", " Abc-7 "] {
        let jql = search_jql(&key("ABC"), typed).expect("a query");
        let want = typed.trim().to_ascii_uppercase();
        assert!(
            jql.contains(&format!(r#"OR key = "{want}")"#)),
            "{typed}: {jql}"
        );
    }
}

#[test]
fn a_key_of_another_project_or_not_a_key_adds_no_key_clause() {
    for typed in [
        "XYZ-123",
        "ABC-",
        "ABC-12a",
        "-12",
        "ABC 123",
        "12-3",
        "ABCD-1",
        "login-fix",
    ] {
        let jql = search_jql(&key("ABC"), typed).expect("a query");
        assert!(!jql.contains("key ="), "{typed}: {jql}");
    }
}

#[test]
fn a_query_carries_nothing_but_the_key_and_the_text() {
    let jql = search_jql(&key("ABC"), "x").expect("a query");
    assert!(jql.starts_with(r#"project = "ABC" AND statusCategory != Done AND ("#));
    assert!(jql.ends_with(" ORDER BY updated DESC"));
}

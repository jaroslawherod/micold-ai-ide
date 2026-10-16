//! A Jira issue's row, ticket and type (feature 533, contracts/issue-naming.md §1-2).

use micold_core::github::Issue;

fn jira() -> Issue {
    Issue::jira(
        "ABC-123",
        123,
        "Add login rate limit".into(),
        vec!["backend".into(), "security".into()],
        "2026-10-14T09:30:11.123+0200".into(),
    )
    .reported_by("Mia Jansen")
    .with_type_name("Bug")
}

#[test]
fn a_keyed_issue_shows_its_key_where_github_shows_a_number() {
    let issue = jira();
    assert_eq!(issue.key(), Some("ABC-123"));
    assert_eq!(issue.ticket(), "ABC-123");
    assert_eq!(issue.number(), 123);
    assert_eq!(issue.type_name(), Some("Bug"));
    assert_eq!(issue.title_line(), "ABC-123 Add login rate limit");
    assert_eq!(issue.details_line(), "Mia Jansen  ·  backend, security");
    assert_eq!(
        issue.row_text(),
        "ABC-123 Add login rate limit  ·  Mia Jansen  ·  backend, security"
    );
}

#[test]
fn the_row_text_follows_the_reporter_set_after_the_type() {
    let issue = jira().reported_by("Noah").with_type_name("Task");
    assert!(issue.row_text().contains("Noah"));
    assert!(issue.row_text().starts_with("ABC-123 "));
    assert_eq!(issue.type_name(), Some("Task"));
}

#[test]
fn a_github_issue_is_unchanged() {
    let issue =
        Issue::new(12, "Add login".into(), vec!["l1".into()], String::new()).reported_by("octo");
    assert_eq!(issue.key(), None);
    assert_eq!(issue.type_name(), None);
    assert_eq!(issue.ticket(), "12");
    assert_eq!(issue.title_line(), "#12 Add login");
    assert_eq!(issue.row_text(), "#12 Add login  ·  octo  ·  l1");
}

#[test]
fn emphasis_ranges_land_on_the_key() {
    let issue = jira();
    let emphasis = issue.emphasis(std::slice::from_ref(&(0..3)));
    assert_eq!(emphasis.title, vec![0..3]);
}

#[test]
fn debug_still_redacts_the_reporter_and_the_description() {
    let issue = jira().described("secret plan");
    let shown = format!("{issue:?}");
    assert!(!shown.contains("Mia Jansen"), "{shown}");
    assert!(!shown.contains("secret plan"), "{shown}");
    assert!(shown.contains("ABC-123"), "{shown}");
}

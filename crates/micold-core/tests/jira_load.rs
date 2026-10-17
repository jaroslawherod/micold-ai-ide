//! Paging a Jira project's open issues up to the cap (feature 533, contracts/jira-cloud-source.md
//! §1): the same paging helper GitHub uses, driven through `FakeJiraSource`.

use micold_core::github::{Issue, IssueLoadError, IssuePage, ISSUE_LOAD_CAP};
use micold_core::jira::{load, ApiToken, FakeJiraSource, JiraConnection};

fn conn() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.example").expect("valid")
}

fn token() -> ApiToken {
    ApiToken::parse("secret").expect("a token")
}

fn page(first: u64, len: u64, more: bool) -> IssuePage {
    IssuePage {
        issues: (first..first + len)
            .map(|n| {
                Issue::jira(
                    &format!("ABC-{n}"),
                    n,
                    format!("Issue {n}"),
                    vec![],
                    String::new(),
                )
            })
            .collect(),
        total_open: len,
        next_cursor: more.then(|| format!("tok-{}", first + len - 1)),
    }
}

#[test]
fn pages_concatenate_until_the_last_and_the_listing_is_complete() {
    let source = FakeJiraSource::new()
        .with_page(page(1, 2, true))
        .with_page(page(3, 1, false));
    let listing = load(&source, &conn(), &token()).expect("loaded");
    let keys: Vec<_> = listing.issues.iter().filter_map(Issue::key).collect();
    assert_eq!(keys, ["ABC-1", "ABC-2", "ABC-3"]);
    assert!(listing.complete);
    assert_eq!(listing.total_open, 3);
    assert_eq!(
        source.calls(),
        [None, Some("tok-2".to_string())],
        "the second request continues from the first page's token"
    );
}

#[test]
fn the_cap_truncates_the_crossing_page_and_the_listing_is_incomplete() {
    let mut source = FakeJiraSource::new();
    for n in 0..11 {
        source = source.with_page(page(n * 100 + 1, 100, true));
    }
    let listing = load(&source, &conn(), &token()).expect("loaded");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(!listing.complete, "a cursor remained");
    assert_eq!(
        source.calls().len(),
        10,
        "no page past the cap is asked for"
    );
}

#[test]
fn a_last_page_exactly_at_the_cap_is_complete() {
    let mut source = FakeJiraSource::new();
    for n in 0..10 {
        source = source.with_page(page(n * 100 + 1, 100, n < 9));
    }
    let listing = load(&source, &conn(), &token()).expect("loaded");
    assert_eq!(listing.issues.len(), ISSUE_LOAD_CAP);
    assert!(listing.complete);
}

#[test]
fn a_page_that_hands_back_its_cursor_ends_the_load_incomplete() {
    let mut stuck = page(1, 2, true);
    stuck.next_cursor = Some("same".into());
    let source = FakeJiraSource::new()
        .with_page(stuck.clone())
        .with_page(IssuePage {
            issues: page(3, 1, false).issues,
            ..stuck
        });
    let listing = load(&source, &conn(), &token()).expect("loaded");
    assert_eq!(listing.issues.len(), 3);
    assert!(
        !listing.complete,
        "a cursor remained, so the listing is not known to be whole"
    );
}

#[test]
fn the_first_error_aborts_the_load() {
    let source = FakeJiraSource::new()
        .with_page(page(1, 2, true))
        .with_error(IssueLoadError::RateLimited);
    assert_eq!(
        load(&source, &conn(), &token()).unwrap_err(),
        IssueLoadError::RateLimited
    );
}

#[test]
fn an_unscripted_call_is_an_error_not_a_hang() {
    let source = FakeJiraSource::new();
    assert!(matches!(
        load(&source, &conn(), &token()),
        Err(IssueLoadError::Other(_))
    ));
}

#[test]
fn a_failure_on_a_later_page_discards_the_issues_read_and_asks_for_no_more() {
    let source = FakeJiraSource::new()
        .with_page(page(1, 2, true))
        .with_error(IssueLoadError::TimedOut)
        .with_page(page(3, 1, false));
    assert_eq!(
        load(&source, &conn(), &token()).unwrap_err(),
        IssueLoadError::TimedOut
    );
    assert_eq!(
        source.calls(),
        [None, Some("tok-2".to_string())],
        "the load stops at the failed request"
    );
}

#[test]
fn the_listing_counts_the_issues_held_not_the_total_a_page_claims() {
    let mut claimed = page(1, 2, false);
    claimed.total_open = 999;
    let source = FakeJiraSource::new().with_page(claimed);
    let listing = load(&source, &conn(), &token()).expect("loaded");
    assert_eq!(
        listing.total_open, 2,
        "Jira gives no total; the count is what was read"
    );
    assert!(listing.complete);
}

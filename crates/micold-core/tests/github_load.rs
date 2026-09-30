//! Paging open issues up to the load cap (feature 034, FR-004, contracts/github-issue-source.md §4).

use micold_core::github::{
    load_listing, FakeIssueSource, GithubRepo, Issue, IssueLoadError, IssuePage, ISSUE_LOAD_CAP,
};

/// GitHub's page size for the list query.
const PAGE: u64 = 100;

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/o/r").unwrap()
}

/// A page holding issues `first..first + len`, claiming `total` open, continuing when `more`.
fn page(first: u64, len: u64, total: u64, more: bool) -> IssuePage {
    IssuePage {
        issues: (first..first + len)
            .map(|n| Issue::new(n, format!("Issue {n}"), vec![], String::new()))
            .collect(),
        total_open: total,
        next_cursor: more.then(|| format!("after-{}", first + len - 1)),
    }
}

#[test]
fn pages_concatenate_until_the_last() {
    let source = FakeIssueSource::new()
        .with_page(page(1, 2, 3, true))
        .with_page(page(3, 1, 3, false));
    let listing = load_listing(&source, &repo()).expect("loaded");

    let numbers: Vec<u64> = listing.issues.iter().map(Issue::number).collect();
    assert_eq!(
        numbers,
        [1, 2, 3],
        "pages are joined in the order GitHub sent them"
    );
    assert_eq!(listing.total_open, 3, "GitHub's open count is kept");
    assert!(listing.complete, "every open issue is held");
    assert_eq!(
        source.calls(),
        [
            ("o/r".to_string(), None),
            ("o/r".to_string(), Some("after-2".to_string())),
        ],
        "the second request continues from the first page's cursor"
    );
}

#[test]
fn the_cap_is_1000() {
    assert_eq!(ISSUE_LOAD_CAP, 1_000, "FR-004's cap");

    // 1,001 open: ten full pages reach the cap and paging stops although GitHub has more.
    let mut over = FakeIssueSource::new();
    for i in 0..11 {
        over = over.with_page(page(1 + i * PAGE, PAGE, 1_001, true));
    }
    let listing = load_listing(&over, &repo()).unwrap();
    assert_eq!(
        listing.issues.len(),
        ISSUE_LOAD_CAP,
        "1,001 open: exactly the cap is held"
    );
    assert_eq!(
        listing.total_open, 1_001,
        "the open count is GitHub's, not what was held, so the form can say 1,000 of 1,001"
    );
    assert_eq!(over.calls().len(), 10, "no request is made past the cap");
    assert!(
        !listing.complete,
        "1,000 held of 1,001 open is incomplete, so search may reach GitHub"
    );

    // Exactly 1,000 open: the other side of the boundary.
    let mut exact = FakeIssueSource::new();
    for i in 0..10 {
        exact = exact.with_page(page(1 + i * PAGE, PAGE, 1_000, i < 9));
    }
    let listing = load_listing(&exact, &repo()).unwrap();
    assert_eq!(
        listing.issues.len(),
        ISSUE_LOAD_CAP,
        "1,000 open: all are held"
    );
    assert!(listing.complete, "1,000 held of 1,000 open is complete");

    // A last page that would cross the cap is truncated to it.
    let mut crossing = FakeIssueSource::new().with_page(page(1, 950, 2_000, true));
    crossing = crossing.with_page(page(951, 100, 2_000, true));
    let listing = load_listing(&crossing, &repo()).unwrap();
    assert_eq!(
        listing.issues.len(),
        ISSUE_LOAD_CAP,
        "950 then 100: the second page is cut to the 50 that fit"
    );
    assert_eq!(
        listing.issues.last().map(Issue::number),
        Some(1_000),
        "the page that crosses the cap keeps only what fits, in order"
    );
}

#[test]
fn first_error_aborts_and_empty_is_complete() {
    let failing = FakeIssueSource::new()
        .with_page(page(1, PAGE, 500, true))
        .with_error(IssueLoadError::Offline)
        .with_page(page(101, PAGE, 500, true));
    assert_eq!(
        load_listing(&failing, &repo()).unwrap_err(),
        IssueLoadError::Offline,
        "a failed page fails the whole load; a partial list is never shown as the list"
    );
    assert_eq!(
        failing.calls().len(),
        2,
        "nothing is requested after the error"
    );

    let none = FakeIssueSource::new().with_page(page(1, 0, 0, false));
    let listing = load_listing(&none, &repo()).unwrap();
    assert!(listing.issues.is_empty(), "no open issues, none held");
    assert!(
        listing.complete,
        "no open issues is a complete, empty list (FR-008)"
    );
}

#[test]
fn a_page_that_makes_no_progress_ends_the_load() {
    // An empty page that still claims more: stop rather than ask again forever.
    let stalled = IssuePage {
        issues: vec![],
        total_open: 5,
        next_cursor: Some("after-0".into()),
    };
    let source = FakeIssueSource::new().with_page(stalled);
    let listing = load_listing(&source, &repo()).expect("a stall is not an error");
    assert_eq!(
        source.calls().len(),
        1,
        "no second request after an empty page"
    );
    assert!(!listing.complete, "0 of 5 held is not complete");

    // A page that hands back the cursor it was asked with: stop too.
    let mut repeat = page(3, 2, 10, true);
    repeat.next_cursor = Some("after-2".into());
    let source = FakeIssueSource::new()
        .with_page(page(1, 2, 10, true))
        .with_page(repeat);
    let listing = load_listing(&source, &repo()).expect("a repeated cursor is not an error");
    assert_eq!(
        source.calls().len(),
        2,
        "no third request with the same cursor"
    );
    assert_eq!(
        listing.issues.len(),
        4,
        "both pages are kept, the repeated one included"
    );
    assert!(!listing.complete, "4 of 10 held is not complete");
}

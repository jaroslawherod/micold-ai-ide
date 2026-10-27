//! Reading one issue by number (550 FR-005, FR-007, FR-008, contracts/issue-lookup.md): the
//! request that leaves the machine, the classification of `gh`'s answer, and the fake the daemon's
//! tests inject.

use micold_core::github::{
    lookup_args, parse_lookup, parse_title, FakeIssueSource, GithubRepo, IssueLoadError,
    IssueLookup, IssueLookupError, IssueSnapshot, NotOpen,
};

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/o/r.git").unwrap()
}

#[test]
fn the_request_carries_only_owner_name_and_number() {
    let args = lookup_args(&repo(), 123);
    assert_eq!(
        &args[..4],
        ["api", "graphql", "--hostname", "github.com"].map(String::from)
    );
    let pairs: Vec<(&str, &str)> = args[4..]
        .chunks(2)
        .map(|c| (c[0].as_str(), c[1].as_str()))
        .collect();
    assert_eq!(pairs.len(), 4, "{args:?}");
    assert_eq!(pairs[0].0, "-f");
    assert!(pairs[0].1.starts_with("query="), "{args:?}");
    assert_eq!(pairs[1], ("-f", "owner=o"));
    assert_eq!(pairs[2], ("-f", "name=r"));
    assert_eq!(
        pairs[3],
        ("-F", "n=123"),
        "the number is the only typed variable"
    );
}

fn parse(json: &str) -> Result<Result<IssueSnapshot, NotOpen>, IssueLoadError> {
    parse_lookup(json.as_bytes())
}

#[test]
fn an_open_issue_gives_its_number_title_and_labels() {
    let got = parse(
        r#"{"data":{"repository":{"issue":{"number":123,"title":"Login crash","state":"OPEN",
        "labels":{"nodes":[{"name":"bug"},{"name":"ui"}]}}}}}"#,
    );
    assert_eq!(
        got,
        Ok(Ok(IssueSnapshot {
            number: 123,
            title: "Login crash".into(),
            labels: vec!["bug".into(), "ui".into()],
            body_text: String::new(),
        }))
    );
}

/// 1161 T020: the single-issue read carries the whole plain-text body for the prompt template.
#[test]
fn an_open_issue_gives_its_body_text() {
    let got = parse(
        r#"{"data":{"repository":{"issue":{"number":42,"title":"t","state":"OPEN",
        "labels":{"nodes":[]},"bodyText":"It crashes.\n\nSteps: open it."}}}}"#,
    );
    let snapshot = got.unwrap().unwrap();
    assert_eq!(snapshot.body_text, "It crashes.\n\nSteps: open it.");
    assert!(
        micold_core::github::ISSUE_QUERY.contains("bodyText"),
        "the query asks for the body"
    );
}

#[test]
fn a_closed_issue_is_not_open() {
    let got = parse(
        r#"{"data":{"repository":{"issue":{"number":4,"title":"t","state":"CLOSED",
        "labels":{"nodes":[]}}}}}"#,
    );
    assert_eq!(got, Ok(Err(NotOpen { closed: true })));
}

#[test]
fn a_missing_issue_or_pull_request_is_not_open_even_with_a_not_found_error() {
    assert_eq!(
        parse(r#"{"data":{"repository":{"issue":null}}}"#),
        Ok(Err(NotOpen { closed: false }))
    );
    assert_eq!(
        parse(
            r#"{"data":{"repository":{"issue":null}},"errors":[{"type":"NOT_FOUND",
            "path":["repository","issue"],"message":"Could not resolve"}]}"#
        ),
        Ok(Err(NotOpen { closed: false }))
    );
}

#[test]
fn a_missing_repository_is_no_access() {
    assert_eq!(
        parse(r#"{"data":{"repository":null},"errors":[{"type":"NOT_FOUND","message":"x"}]}"#),
        Err(IssueLoadError::NoAccess)
    );
}

#[test]
fn a_rate_limit_and_unreadable_answers_are_load_errors() {
    assert_eq!(
        parse(r#"{"errors":[{"type":"RATE_LIMITED","message":"slow down"}]}"#),
        Err(IssueLoadError::RateLimited)
    );
    assert!(matches!(parse("not json"), Err(IssueLoadError::Other(_))));
    assert!(matches!(
        parse(r#"{"data":null}"#),
        Err(IssueLoadError::Other(_))
    ));
}

#[test]
fn not_open_reasons_name_the_issue_and_the_repository() {
    let closed = IssueLookupError::NotOpenIssue { closed: true }.message(&repo(), 7);
    assert_eq!(closed, "issue #7 in o/r is closed");
    let missing = IssueLookupError::NotOpenIssue { closed: false }.message(&repo(), 7);
    assert_eq!(
        missing,
        "#7 is not an open issue in o/r: it may be a pull request or not exist"
    );
}

#[test]
fn a_timeout_has_the_forms_ten_second_text() {
    let fake =
        FakeIssueSource::new().with_lookup(Err(IssueLookupError::Load(IssueLoadError::TimedOut)));
    let err = fake.read_issue(&repo(), 5).unwrap_err();
    assert_eq!(
        err.message(&repo(), 5),
        "GitHub didn't answer within 10 seconds."
    );
    assert_eq!(fake.lookup_calls(), vec![("o/r".to_string(), 5)]);
}

#[test]
fn a_fake_with_nothing_scripted_has_seen_no_lookup() {
    let fake = FakeIssueSource::new();
    assert!(fake.lookup_calls().is_empty());
}

fn title(json: &str) -> Result<Option<String>, IssueLoadError> {
    parse_title(json.as_bytes())
}

/// BUG-1488: the Overview's title read takes an issue in any state; a closed one keeps its title.
#[test]
fn a_title_is_read_for_an_open_and_a_closed_issue() {
    for state in ["OPEN", "CLOSED"] {
        let json = format!(
            r#"{{"data":{{"repository":{{"issue":{{"number":4,"title":"Basic image","state":"{state}",
            "labels":{{"nodes":[]}},"bodyText":""}}}}}}}}"#
        );
        assert_eq!(title(&json), Ok(Some("Basic image".to_string())), "{state}");
    }
}

#[test]
fn a_missing_issue_or_pull_request_has_no_title() {
    assert_eq!(title(r#"{"data":{"repository":{"issue":null}}}"#), Ok(None));
    assert_eq!(
        title(
            r#"{"data":{"repository":{"issue":null}},"errors":[{"type":"NOT_FOUND",
            "path":["repository","issue"],"message":"Could not resolve"}]}"#
        ),
        Ok(None)
    );
}

#[test]
fn a_title_read_fails_like_the_lookup_without_data_or_repository() {
    for json in [
        r#"{"errors":[{"type":"RATE_LIMITED","message":"slow down"}]}"#,
        r#"{"data":{"repository":null},"errors":[{"type":"NOT_FOUND","message":"x"}]}"#,
        "not json",
    ] {
        assert_eq!(
            title(json),
            Err(parse(json).expect_err("the lookup fails too")),
            "{json}"
        );
    }
}

#[test]
fn the_default_title_read_goes_through_the_lookup() {
    let open = IssueSnapshot {
        number: 5,
        title: "Login crash".into(),
        labels: vec![],
        body_text: String::new(),
    };
    let fake = FakeIssueSource::new()
        .with_lookup(Ok(open))
        .with_lookup(Err(IssueLookupError::NotOpenIssue { closed: true }))
        .with_lookup(Err(IssueLookupError::Load(IssueLoadError::NoAccess)));
    assert_eq!(fake.read_title(&repo(), 5), Ok(Some("Login crash".into())));
    assert_eq!(fake.read_title(&repo(), 6), Ok(None));
    assert_eq!(
        fake.read_title(&repo(), 7),
        Err(IssueLookupError::Load(IssueLoadError::NoAccess))
    );
}

#[test]
fn a_scripted_title_answers_the_title_read_without_a_lookup() {
    let fake = FakeIssueSource::new().with_title(Ok(Some("Closed one".into())));
    assert_eq!(fake.read_title(&repo(), 9), Ok(Some("Closed one".into())));
    assert!(fake.lookup_calls().is_empty());
}

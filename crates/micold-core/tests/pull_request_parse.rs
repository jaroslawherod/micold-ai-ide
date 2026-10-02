//! Reading the answer of one pull request request (feature 040,
//! contracts/pull-request-source.md §5): `split_response` separates what `gh api --include` prints,
//! `parse_status` turns the body into one status per branch.
//!
//! The answers are the `tests/fixtures/gh/pr_*.txt` files; `pr_README.md` beside them says how each
//! was recorded.

use std::path::PathBuf;

/// The 14 recorded answers of contracts/pull-request-source.md §5.
const FIXTURES: [&str; 14] = [
    "pr_three_branches.txt",
    "pr_checks_failing.txt",
    "pr_checks_pending.txt",
    "pr_checks_passing.txt",
    "pr_no_checks.txt",
    "pr_draft.txt",
    "pr_closed.txt",
    "pr_review_states.txt",
    "pr_cross_repository.txt",
    "pr_repo_not_found.txt",
    "pr_rate_limited_graphql.txt",
    "pr_rate_limited_secondary.txt",
    "pr_rate_limited_secondary_no_retry_after.txt",
    "pr_truncated.txt",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name)
}

#[test]
fn every_recorded_answer_is_present_and_holds_no_credential() {
    for name in FIXTURES {
        let path = fixture_path(name);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{name} is one of the 14 recorded answers: {e}"));
        let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        for credential in ["gho_", "ghp_", "github_pat_", "authorization:"] {
            assert!(
                !text.contains(credential),
                "{name} must not hold a token or an Authorization header, found {credential}"
            );
        }
        assert!(
            text.starts_with("http/"),
            "{name} is recorded with --include, so it starts with the status line"
        );
    }
}

use micold_core::pull_request::{
    parse_status, split_response, CheckStatus, PrState, PullRequestStatus, ReadingFailure,
    ReviewState,
};
use std::collections::BTreeMap;

fn body_of(name: &str) -> Vec<u8> {
    let bytes = std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let Some(response) = split_response(&bytes) else {
        panic!("{name} must split into a status line, headers and a body");
    };
    response.body.to_vec()
}

fn branches(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn read(name: &str, names: &[&str]) -> BTreeMap<String, PullRequestStatus> {
    match parse_status(&body_of(name), &branches(names)) {
        Ok(map) => map,
        Err(e) => panic!("{name} is a readable answer, got {e:?}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn check(
    map: &BTreeMap<String, PullRequestStatus>,
    key: &str,
    number: u64,
    title: &str,
    url: &str,
    head: &str,
    state: PrState,
    review: ReviewState,
) {
    let Some(s) = map.get(key) else {
        panic!("branch {key} has a pull request and must have an entry (FR-001)");
    };
    assert_eq!(s.number, number, "{key}: number (FR-001)");
    assert_eq!(s.title, title, "{key}: title");
    assert_eq!(s.url, url, "{key}: url");
    assert_eq!(s.head, head, "{key}: head");
    assert_eq!(s.state, state, "{key}: state (FR-002)");
    assert_eq!(s.review, review, "{key}: review");
}

// U7
#[test]
fn split_response_reads_status_headers_and_body_with_crlf() {
    let bytes = b"HTTP/2.0 200 OK\r\nX-RateLimit-Remaining: 4990 \r\nContent-Type: application/json\r\n\r\n{\"a\":1}\r\n\r\n{\"b\":2}";
    let Some(r) = split_response(bytes) else {
        panic!("a status line and an empty line must split");
    };
    assert_eq!(r.status, 200, "status comes from the status line");
    assert_eq!(
        r.body, b"{\"a\":1}\r\n\r\n{\"b\":2}",
        "body is everything after the first empty line"
    );
    for name in [
        "X-RateLimit-Remaining",
        "x-ratelimit-remaining",
        "X-RATELIMIT-REMAINING",
    ] {
        assert_eq!(
            r.header(name),
            Some("4990"),
            "header {name} found without case, trimmed"
        );
    }
    assert_eq!(r.header("Retry-After"), None, "an absent header is None");
}

// U7
#[test]
fn split_response_reads_status_headers_and_body_with_lf() {
    let bytes = b"HTTP/1.1 403 Forbidden\nRetry-After: 30\nX-RateLimit-Remaining: 0\n\n{\"message\":\"x\"}\n\nrest";
    let Some(r) = split_response(bytes) else {
        panic!("LF line ends must split too");
    };
    assert_eq!(r.status, 403, "status comes from the status line");
    assert_eq!(r.body, b"{\"message\":\"x\"}\n\nrest", "body stays whole");
    assert_eq!(
        r.header("retry-after"),
        Some("30"),
        "header found without case"
    );
    assert_eq!(
        r.header("X-RATELIMIT-REMAINING"),
        Some("0"),
        "header found without case"
    );
    assert_eq!(
        r.header("x-ratelimit-reset"),
        None,
        "an absent header is None"
    );
}

// U7
#[test]
fn split_response_reads_a_recorded_answer() {
    let bytes = std::fs::read(fixture_path("pr_three_branches.txt")).expect("fixture");
    let Some(r) = split_response(&bytes) else {
        panic!("a recorded answer must split");
    };
    assert_eq!(r.status, 200, "recorded status");
    assert!(
        r.header("x-ratelimit-reset").is_some(),
        "recorded answer carries x-ratelimit-reset"
    );
    assert!(r.body.starts_with(b"{"), "body starts with the JSON object");
}

// U8
#[test]
fn split_response_without_a_status_line_is_none() {
    assert!(
        split_response(b"{\"data\":{}}").is_none(),
        "no status line means no answer (FR-019)"
    );
    assert!(
        split_response(b"").is_none(),
        "an empty input has no status line"
    );
}

// U8
#[test]
fn split_response_without_an_empty_line_is_none() {
    assert!(
        split_response(b"HTTP/2.0 200 OK\r\nX-A: b\r\n").is_none(),
        "no empty line means no body boundary (FR-019)"
    );
}

// U9
#[test]
fn parse_status_three_branches_keys_by_given_names() {
    let names = ["alpha", "beta", "gamma"];
    let map = read("pr_three_branches.txt", &names);
    assert_eq!(
        map.len(),
        2,
        "gamma has no pull request, so no entry (FR-001)"
    );
    assert!(!map.contains_key("gamma"), "no entry for gamma");
    check(
        &map,
        "alpha",
        347,
        "chore(main): release micold-ai-ide 0.16.0",
        "https://github.com/jaroslawherod/micold-ai-ide/pull/347",
        "b9e8917918758c1f236350a25d3660d6904a3e2b",
        PrState::Open {
            checks: CheckStatus::None,
        },
        ReviewState::None,
    );
    let Some(beta) = map.get("beta") else {
        panic!("beta has #536")
    };
    assert_eq!(beta.number, 536, "beta: newest of the merged ones (FR-005)");
    assert_eq!(beta.state, PrState::Merged, "beta: merged (FR-002)");
    assert_eq!(
        beta.title,
        "docs(040): clarify, plan and cut milestones for pull request and check status for each worktree"
    );
    assert_eq!(
        beta.url,
        "https://github.com/jaroslawherod/micold-ai-ide/pull/536"
    );
    assert_eq!(beta.head, "a318b9e56fee82419257d1ced289acc19a01ecab");
}

// U10
#[test]
fn parse_status_draft() {
    let map = read("pr_draft.txt", &["d"]);
    check(
        &map,
        "d",
        14578,
        "Replace extension browse with Bubble Tea",
        "https://github.com/cli/cli/pull/14578",
        "e15e40d099bb09c62c4bc549a0d58397917efb45",
        PrState::Draft {
            checks: CheckStatus::Passing,
        },
        ReviewState::ReviewRequired,
    );
}

// U10
#[test]
fn parse_status_closed() {
    let map = read("pr_closed.txt", &["c"]);
    check(
        &map,
        "c",
        302,
        "docs(028): link the macOS packaging page's contracts on GitHub",
        "https://github.com/jaroslawherod/micold-ai-ide/pull/302",
        "9fbebb0470dbfd43eebf792c7c3f83b5f8cd758c",
        PrState::Closed,
        ReviewState::None,
    );
}

// U11
#[test]
fn parse_status_reduces_checks() {
    let cases = [
        ("pr_checks_failing.txt", 339083, CheckStatus::Failing),
        ("pr_checks_pending.txt", 339346, CheckStatus::Pending),
        ("pr_checks_passing.txt", 14566, CheckStatus::Passing),
        ("pr_no_checks.txt", 347, CheckStatus::None),
    ];
    for (name, number, checks) in cases {
        let map = read(name, &["b"]);
        let Some(s) = map.get("b") else {
            panic!("{name}: one entry (FR-001)")
        };
        assert_eq!(s.number, number, "{name}: number");
        assert_eq!(
            s.state,
            PrState::Open { checks },
            "{name}: check status (FR-003)"
        );
    }
}

// U12
#[test]
fn parse_status_reads_review_decisions_in_branch_order() {
    let map = read("pr_review_states.txt", &["x", "y", "z"]);
    let got: Vec<(u64, Option<ReviewState>)> = ["x", "y", "z"]
        .iter()
        .map(|k| {
            (
                map.get(*k).map_or(0, |s| s.number),
                map.get(*k).map(|s| s.review),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (339339, Some(ReviewState::Approved)),
            (339165, Some(ReviewState::ChangesRequested)),
            (339346, Some(ReviewState::ReviewRequired)),
        ],
        "review decisions in branch order (FR-010)"
    );
}

// U12
#[test]
fn parse_status_null_review_decision_is_none() {
    let map = read("pr_no_checks.txt", &["b"]);
    let Some(s) = map.get("b") else {
        panic!("one entry")
    };
    assert_eq!(
        s.review,
        ReviewState::None,
        "null reviewDecision is no decision"
    );
}

// U12
#[test]
fn parse_status_unknown_review_decision_is_none() {
    let text = String::from_utf8(body_of("pr_review_states.txt")).expect("utf-8");
    let changed = text.replace("\"APPROVED\"", "\"SOMETHING_NEW\"");
    assert_ne!(changed, text, "the replacement must have happened");
    let map = match parse_status(changed.as_bytes(), &branches(&["x", "y", "z"])) {
        Ok(m) => m,
        Err(e) => panic!("an unknown decision is still a readable answer, got {e:?}"),
    };
    let Some(s) = map.get("x") else {
        panic!("x has an entry")
    };
    assert_eq!(
        s.review,
        ReviewState::None,
        "an unknown decision word is no decision"
    );
    assert_eq!(s.number, 339339, "the rest of the entry is read");
}

// U13
#[test]
fn parse_status_truncated_answer_is_passing() {
    let r = parse_status(&body_of("pr_truncated.txt"), &branches(&["a", "b", "c"]));
    assert_eq!(
        r,
        Err(ReadingFailure::Passing),
        "a cut body is a passing failure (FR-019)"
    );
}

fn three_body() -> serde_json::Value {
    serde_json::from_slice(&body_of("pr_three_branches.txt")).expect("fixture body is JSON")
}

fn three_result(
    v: &serde_json::Value,
) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure> {
    parse_status(v.to_string().as_bytes(), &branches(&["a", "b", "c"]))
}

// U14
#[test]
fn parse_status_missing_alias_is_passing() {
    let mut v = three_body();
    let removed = v["data"]["repository"]
        .as_object_mut()
        .expect("repository")
        .remove("r1");
    assert!(removed.is_some(), "r1 existed");
    assert_eq!(
        three_result(&v),
        Err(ReadingFailure::Passing),
        "never a partial map (FR-019)"
    );
}

// U14
#[test]
fn parse_status_null_repository_is_passing() {
    let mut v = three_body();
    v["data"]["repository"] = serde_json::Value::Null;
    assert_eq!(
        three_result(&v),
        Err(ReadingFailure::Passing),
        "null repository (FR-019)"
    );
}

// U14
#[test]
fn parse_status_graphql_errors_are_passing() {
    let mut v = three_body();
    v["errors"] = serde_json::json!([{"type":"SOME_ERROR","message":"x"}]);
    assert_eq!(
        three_result(&v),
        Err(ReadingFailure::Passing),
        "an errors entry fails all (FR-019)"
    );
}

// U14
#[test]
fn parse_status_fewer_aliases_than_branches_is_passing() {
    let r = parse_status(
        &body_of("pr_three_branches.txt"),
        &branches(&["a", "b", "c", "d"]),
    );
    assert_eq!(
        r,
        Err(ReadingFailure::Passing),
        "a missing o3 is a failure (FR-019)"
    );
}

// U14
#[test]
fn parse_status_not_json_is_passing() {
    let r = parse_status(b"this is not json", &branches(&["a"]));
    assert_eq!(r, Err(ReadingFailure::Passing), "not JSON (FR-019)");
}

// U15
#[test]
fn parse_status_only_cross_repository_is_empty() {
    let map = read("pr_cross_repository.txt", &["patch-1"]);
    assert!(
        map.is_empty(),
        "a fork's pull request is not this branch's (FR-005, SC-002)"
    );
}

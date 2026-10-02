//! Turning a `gh` run that produced no statuses into a `ReadingFailure` (feature 040,
//! contracts/pull-request-source.md §5): the table of `reading_failure` and the arithmetic of
//! `rate_limit_pause`.
//!
//! Stdout fixtures `tests/fixtures/gh/pr_*.txt` are whole `gh api --include` recordings (see
//! `pr_README.md`); stderr fixtures `*.stderr` are feature 034's, read through `github::classify`.
//! Rate limiting is checked before access because GitHub reports both as HTTP 403.

use std::path::PathBuf;

use micold_core::process::RunOutcome;
use micold_core::pull_request::{
    rate_limit_pause, reading_failure, split_response, ReadingFailure,
};

const NOW: u64 = 1_790_000_000;

fn fixture(file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn stderr(name: &str) -> String {
    String::from_utf8(fixture(&format!("{name}.stderr"))).expect("stderr fixture is UTF-8")
}

fn exited(code: i32, stdout: Vec<u8>, stderr: &str) -> RunOutcome {
    RunOutcome::Exited {
        code,
        stdout,
        stderr: stderr.to_string(),
    }
}

fn stderr_only(code: i32, name: &str) -> RunOutcome {
    exited(code, Vec::new(), &stderr(name))
}

fn answer(status_line: &str, headers: &[&str], body: &str) -> Vec<u8> {
    let mut text = format!("{status_line}\r\n");
    for h in headers {
        text.push_str(h);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    text.push_str(body);
    text.into_bytes()
}

fn pause(bytes: &[u8]) -> u64 {
    let response = split_response(bytes).expect("a whole answer splits");
    rate_limit_pause(&response, NOW)
}

/// U31
#[test]
fn timed_out_is_passing() {
    assert_eq!(
        reading_failure(
            &RunOutcome::TimedOut {
                stderr: String::new()
            },
            NOW
        ),
        ReadingFailure::Passing,
        "no answer in time is waited out, not removed (FR-019, FR-021)"
    );
}

/// U32
#[test]
fn missing_gh_is_unavailable() {
    assert_eq!(
        reading_failure(
            &RunOutcome::SpawnFailed("No such file or directory (os error 2)".into()),
            NOW
        ),
        ReadingFailure::Unavailable,
        "a `gh` that vanished means pull requests cannot be read (FR-025)"
    );
}

/// U32
#[test]
fn other_spawn_failure_is_passing() {
    assert_eq!(
        reading_failure(
            &RunOutcome::SpawnFailed("Permission denied (os error 13)".into()),
            NOW
        ),
        ReadingFailure::Passing,
        "a spawn failure that is not a missing tool may pass (FR-019)"
    );
}

/// U33 (a)
#[test]
fn graphql_rate_limited_error_pauses_until_reset() {
    assert_eq!(
        reading_failure(&exited(1, fixture("pr_rate_limited_graphql.txt"), ""), NOW),
        ReadingFailure::RateLimited {
            until: 1_790_966_100
        },
        "a RATE_LIMITED error with remaining 0 waits for the reset (FR-024)"
    );
}

/// U33 (a): the errors are read whatever the data beside them holds.
#[test]
fn a_rate_limit_beside_unreadable_data_is_still_a_rate_limit() {
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &["X-Ratelimit-Remaining: 0", "X-Ratelimit-Reset: 1790966100"],
        r#"{"data":{"repository":{"o0":{"nodes":[null]}}},"errors":[{"type":"RATE_LIMITED","message":"API rate limit already exceeded"}]}"#,
    );
    assert_eq!(
        reading_failure(&exited(1, bytes, ""), NOW),
        ReadingFailure::RateLimited {
            until: 1_790_966_100
        },
        "data this version cannot read must not hide the RATE_LIMITED error beside it (FR-024)"
    );
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &[],
        r#"{"data":{"repository":7},"errors":[{"type":"NOT_FOUND","path":["repository"],"message":"x"}]}"#,
    );
    assert_eq!(
        reading_failure(&exited(1, bytes, ""), NOW),
        ReadingFailure::Unavailable,
        "nor the NOT_FOUND on the repository (FR-025)"
    );
}

/// U33 (b)
#[test]
fn http_429_pauses_a_minute() {
    let bytes = answer("HTTP/2.0 429 Too Many Requests", &[], "{}");
    assert_eq!(
        reading_failure(&exited(1, bytes, ""), NOW),
        ReadingFailure::RateLimited { until: NOW + 60 },
        "HTTP 429 is a rate limit with no header to read (FR-024)"
    );
}

/// U33 (c)
#[test]
fn secondary_limit_403_follows_retry_after() {
    assert_eq!(
        reading_failure(
            &exited(
                1,
                fixture("pr_rate_limited_secondary.txt"),
                "gh: You have exceeded a secondary rate limit (HTTP 403)"
            ),
            NOW
        ),
        ReadingFailure::RateLimited { until: NOW + 30 },
        "HTTP 403 with a rate-limit text is a pause by Retry-After, not no access (FR-024)"
    );
}

/// U33 (d)
#[test]
fn secondary_limit_403_is_read_from_the_body_alone() {
    let bytes = fixture("pr_rate_limited_secondary_no_retry_after.txt");
    assert!(
        String::from_utf8_lossy(&bytes)
            .to_lowercase()
            .contains("rate limit"),
        "the fixture's body says \"rate limit\""
    );
    assert_eq!(
        reading_failure(&exited(1, bytes, ""), NOW),
        ReadingFailure::RateLimited { until: NOW + 60 },
        "a 403 whose body says rate limit, with remaining above 0 and no Retry-After, waits a minute (FR-024)"
    );
}

/// U33 (e)
#[test]
fn rate_limit_on_stderr_alone_pauses_a_minute() {
    for name in [
        "rate_limit_403",
        "secondary_rate_limit",
        "graphql_rate_limited",
    ] {
        assert_eq!(
            reading_failure(&stderr_only(1, name), NOW),
            ReadingFailure::RateLimited { until: NOW + 60 },
            "{name} has no response to read headers from, so the pause is now + 60 (FR-024)"
        );
    }
}

/// U34
#[test]
fn repository_not_found_answer_is_unavailable() {
    assert_eq!(
        reading_failure(&exited(1, fixture("pr_repo_not_found.txt"), ""), NOW),
        ReadingFailure::Unavailable,
        "a NOT_FOUND error on the repository means the sign-in cannot see it (FR-025)"
    );
}

/// U34
#[test]
fn http_401_is_unavailable() {
    let bytes = answer(
        "HTTP/2.0 401 Unauthorized",
        &[],
        "{\"message\":\"Bad credentials\"}",
    );
    assert_eq!(
        reading_failure(&exited(1, bytes, ""), NOW),
        ReadingFailure::Unavailable,
        "HTTP 401 means the sign-in is refused (FR-025)"
    );
}

/// U34
#[test]
fn sign_in_and_access_failures_on_stderr_are_unavailable() {
    for (code, name) in [
        (4, "not_logged_in"),
        (1, "bad_credentials"),
        (1, "saml"),
        (1, "insufficient_scopes"),
        (1, "http_403"),
        (1, "http_404"),
        (1, "repository_not_found"),
    ] {
        assert_eq!(
            reading_failure(&stderr_only(code, name), NOW),
            ReadingFailure::Unavailable,
            "{name} at exit {code} means pull requests cannot be read (FR-025)"
        );
    }
}

/// U35
#[test]
fn offline_is_passing() {
    for name in [
        "offline_error_connecting",
        "offline_connection_refused",
        "offline_no_such_host",
        "offline_unreachable",
    ] {
        assert_eq!(
            reading_failure(&stderr_only(1, name), NOW),
            ReadingFailure::Passing,
            "{name} means GitHub could not be reached; what a row shows stays (FR-019)"
        );
    }
}

/// U36
#[test]
fn http_502_is_passing() {
    let bytes = answer("HTTP/2.0 502 Bad Gateway", &[], "");
    assert_eq!(
        reading_failure(&exited(1, bytes, "gh: HTTP 502"), NOW),
        ReadingFailure::Passing,
        "a 5xx answer is waited out (FR-019)"
    );
}

/// U36
#[test]
fn truncated_answer_is_passing() {
    assert_eq!(
        reading_failure(&exited(0, fixture("pr_truncated.txt"), ""), NOW),
        ReadingFailure::Passing,
        "an answer that cannot be understood is waited out (FR-019)"
    );
}

/// U36
#[test]
fn unknown_stderr_is_passing() {
    assert_eq!(
        reading_failure(&stderr_only(1, "unknown"), NOW),
        ReadingFailure::Passing,
        "an unknown failure is waited out (FR-019)"
    );
}

/// U36
#[test]
fn empty_output_is_passing() {
    assert_eq!(
        reading_failure(&exited(0, Vec::new(), ""), NOW),
        ReadingFailure::Passing,
        "no output at all is waited out (FR-019)"
    );
}

/// U37
#[test]
fn retry_after_adds_its_seconds() {
    assert_eq!(
        pause(&fixture("pr_rate_limited_secondary.txt")),
        NOW + 30,
        "Retry-After: 30 pauses 30 seconds from now (FR-024)"
    );
}

/// U37
#[test]
fn retry_after_wins_over_a_reset() {
    let bytes = answer(
        "HTTP/2.0 403 Forbidden",
        &[
            "Retry-After: 30",
            "X-Ratelimit-Remaining: 0",
            "X-Ratelimit-Reset: 1790966100",
        ],
        "{}",
    );
    assert_eq!(
        pause(&bytes),
        NOW + 30,
        "Retry-After is checked before the reset (FR-024)"
    );
}

/// U38
#[test]
fn remaining_zero_waits_for_the_reset() {
    assert_eq!(
        pause(&fixture("pr_rate_limited_graphql.txt")),
        1_790_966_100,
        "remaining 0 pauses until X-RateLimit-Reset (FR-024)"
    );
}

/// U39
#[test]
fn remaining_above_zero_ignores_the_reset() {
    assert_eq!(
        pause(&fixture("pr_rate_limited_secondary_no_retry_after.txt")),
        NOW + 60,
        "the reset is the primary window's, so a secondary limit waits a minute (FR-024)"
    );
}

/// U40
#[test]
fn a_reset_in_the_past_pauses_one_second() {
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &[
            "X-Ratelimit-Remaining: 0",
            &format!("X-Ratelimit-Reset: {}", NOW - 500),
        ],
        "{}",
    );
    assert_eq!(pause(&bytes), NOW + 1, "never before now + 1 (FR-024)");
}

/// U40
#[test]
fn retry_after_zero_pauses_one_second() {
    let bytes = answer("HTTP/2.0 429 Too Many Requests", &["Retry-After: 0"], "{}");
    assert_eq!(pause(&bytes), NOW + 1, "never before now + 1 (FR-024)");
}

/// U41
#[test]
fn unparsable_retry_after_is_skipped() {
    let bytes = answer(
        "HTTP/2.0 403 Forbidden",
        &[
            "Retry-After: soon",
            "X-Ratelimit-Remaining: 0",
            "X-Ratelimit-Reset: 1790966100",
        ],
        "{}",
    );
    assert_eq!(
        pause(&bytes),
        1_790_966_100,
        "an unparsable Retry-After is skipped as if absent (FR-024)"
    );
}

/// U41
#[test]
fn unparsable_reset_pauses_a_minute() {
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &["X-Ratelimit-Remaining: 0", "X-Ratelimit-Reset: later"],
        "{}",
    );
    assert_eq!(
        pause(&bytes),
        NOW + 60,
        "an unparsable reset is skipped (FR-024)"
    );
}

/// U41
#[test]
fn unparsable_remaining_pauses_a_minute() {
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &[
            "X-Ratelimit-Remaining: none",
            "X-Ratelimit-Reset: 1790966100",
        ],
        "{}",
    );
    assert_eq!(
        pause(&bytes),
        NOW + 60,
        "an unparsable remaining is not 0, so the reset is not used (FR-024)"
    );
}

/// U41
#[test]
fn header_names_are_read_without_case() {
    let bytes = answer("HTTP/2.0 403 Forbidden", &["retry-after: 30"], "{}");
    assert_eq!(
        pause(&bytes),
        NOW + 30,
        "header names are compared without case"
    );
    let bytes = answer(
        "HTTP/2.0 200 OK",
        &["x-ratelimit-remaining: 0", "x-ratelimit-reset: 1790966100"],
        "{}",
    );
    assert_eq!(
        pause(&bytes),
        1_790_966_100,
        "header names are compared without case"
    );
}

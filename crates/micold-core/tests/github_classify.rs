//! Turning a failed `gh` run into one of FR-007's plain-language reasons (feature 034, research R8,
//! contracts/github-issue-source.md §5).
//!
//! The `tests/fixtures/gh/*.stderr` files are what `classify` reads. Captured from `gh` 2.54.0 on
//! Linux: `not_logged_in` (`GH_TOKEN=` with an empty `GH_CONFIG_DIR`, exit 4), `bad_credentials`
//! (`GH_TOKEN=bad`), `repository_not_found` (a repository that does not exist; its stdout is
//! `list_not_found.json`), `offline_connection_refused` (`HTTPS_PROXY=http://127.0.0.1:9`) and
//! `offline_error_connecting` (an unresolvable `--hostname`). The rest are written from GitHub's
//! documented responses, because producing them needs a real rate limit, an organization enforcing
//! SAML, or another OS's resolver; `unknown` is invented on purpose. A future `gh` that words one
//! of these differently shows up here as a failing case, not as a silent `Other`.

use std::path::PathBuf;

use micold_core::github::{classify, GithubRepo, IssueLoadError};
use micold_core::process::RunOutcome;

fn stderr(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(format!("{name}.stderr"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn exited(code: i32, fixture: &str) -> RunOutcome {
    RunOutcome::Exited {
        code,
        stdout: Vec::new(),
        stderr: stderr(fixture),
    }
}

#[test]
fn not_signed_in() {
    // `(1, "not_logged_in")`: the `gh auth login` text is read at any exit status, not only 4.
    for (code, fixture) in [
        (4, "not_logged_in"),
        (1, "not_logged_in"),
        (1, "bad_credentials"),
    ] {
        assert_eq!(
            classify(&exited(code, fixture)),
            IssueLoadError::NotSignedIn,
            "{fixture} at exit {code} means the user has to sign in"
        );
    }
    assert_eq!(
        classify(&RunOutcome::Exited {
            code: 4,
            stdout: Vec::new(),
            stderr: String::new(),
        }),
        IssueLoadError::NotSignedIn,
        "exit 4 is gh's own authentication status, whatever it printed"
    );
}

#[test]
fn no_access() {
    for fixture in [
        "repository_not_found",
        "http_404",
        "http_403",
        "saml",
        "graphql_forbidden",
        "insufficient_scopes",
    ] {
        assert_eq!(
            classify(&exited(1, fixture)),
            IssueLoadError::NoAccess,
            "{fixture} means the sign-in cannot see the repository"
        );
    }
}

/// U33 — connection failures are `Offline`.
#[test]
fn offline() {
    for fixture in [
        "offline_error_connecting",
        "offline_connection_refused",
        "offline_no_such_host",
        "offline_unreachable",
    ] {
        assert_eq!(
            classify(&exited(1, fixture)),
            IssueLoadError::Offline,
            "{fixture} means GitHub could not be reached"
        );
    }
}

/// U33 — rate-limit text is `RateLimited`, whatever HTTP status carries it.
#[test]
fn rate_limited() {
    for fixture in [
        "rate_limit_403",
        "secondary_rate_limit",
        "graphql_rate_limited",
    ] {
        assert_eq!(
            classify(&exited(1, fixture)),
            IssueLoadError::RateLimited,
            "{fixture} is a rate limit, even when it arrives as HTTP 403"
        );
    }
}

/// U33 — a run past the bound is `TimedOut`; a `gh` that could not be started is missing.
#[test]
fn timed_out_and_spawn_failed() {
    assert_eq!(
        classify(&RunOutcome::TimedOut {
            stderr: String::new()
        }),
        IssueLoadError::TimedOut,
        "a run killed at the bound timed out, whatever it printed"
    );
    assert_eq!(
        classify(&RunOutcome::SpawnFailed(
            "No such file or directory (os error 2)".into()
        )),
        IssueLoadError::ToolMissing,
        "a `gh` that vanished between locating and running it is missing"
    );
}

#[test]
fn unknown_text_is_other() {
    assert_eq!(
        classify(&exited(1, "unknown")),
        IssueLoadError::Other("something unexpected happened".into()),
        "the first non-empty stderr line, trimmed"
    );
    assert_eq!(
        classify(&RunOutcome::Exited {
            code: 1,
            stdout: Vec::new(),
            stderr: String::new()
        }),
        IssueLoadError::Other("gh exited with status 1".into()),
        "a silent failure still says something: the exit status"
    );
}

#[test]
fn messages_name_cause_and_remedy() {
    let repo = GithubRepo::from_remote_url("https://github.com/o/r").unwrap();
    for (error, text) in [
        (
            IssueLoadError::ToolMissing,
            "Couldn't read issues: the GitHub CLI (`gh`) isn't installed. Install it from \
             cli.github.com, then sign in with `gh auth login`.",
        ),
        (
            IssueLoadError::NotSignedIn,
            "Couldn't read issues: you're not signed in to GitHub. Run `gh auth login` in a \
             terminal, then retry.",
        ),
        (
            IssueLoadError::NoAccess,
            "Couldn't read issues: your GitHub sign-in can't access o/r.",
        ),
        (
            IssueLoadError::Offline,
            "Couldn't reach GitHub. Check your connection, then retry.",
        ),
        (
            IssueLoadError::RateLimited,
            "GitHub's rate limit was reached. Wait a minute, then retry.",
        ),
        (
            IssueLoadError::TimedOut,
            "GitHub didn't answer within 10 seconds.",
        ),
        (
            IssueLoadError::Other("boom".into()),
            "Couldn't read issues: boom",
        ),
    ] {
        assert_eq!(error.message(&repo), text, "{error:?}");
    }
}

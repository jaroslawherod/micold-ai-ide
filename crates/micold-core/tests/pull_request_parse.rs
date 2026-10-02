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

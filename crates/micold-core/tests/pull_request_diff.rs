//! Reading a pull request's diff (feature 775, contracts/pull-request-reads.md §2): the arguments,
//! the failure kinds and the scripted source. Nothing here runs `gh`.

use micold_core::github::GithubRepo;
use micold_core::process::RunOutcome;
use micold_core::pull_request::{
    diff_args, diff_outcome, DiffReadFailure, FakePullRequestSource, PullRequestSource,
    ReadingFailure,
};

const NOW: u64 = 1_790_000_000;

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a github.com remote")
}

fn exited(code: i32, stdout: &str, stderr: &str) -> RunOutcome {
    RunOutcome::Exited {
        code,
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.to_string(),
    }
}

#[test]
fn the_diff_is_asked_for_with_the_diff_media_type_and_only_owner_repo_number() {
    let args = diff_args(&repo(), 42);
    assert_eq!(
        args,
        [
            "api",
            "--hostname",
            "github.com",
            "-H",
            "Accept: application/vnd.github.v3.diff",
            "repos/acme/widgets/pulls/42"
        ]
    );
}

#[test]
fn an_answer_is_the_diff_text() {
    let out = diff_outcome(&exited(0, "diff --git a/x b/x\n", ""), NOW);
    assert_eq!(out, Ok("diff --git a/x b/x\n".to_string()));
}

#[test]
fn http_406_is_too_large() {
    let out = diff_outcome(
        &exited(
            1,
            "",
            "gh: Sorry, the diff exceeded the maximum number of lines (20000) (HTTP 406)\n",
        ),
        NOW,
    );
    assert_eq!(out, Err(DiffReadFailure::TooLarge));
}

#[test]
fn any_other_failure_is_failed_with_its_reading_failure() {
    let out = diff_outcome(&exited(1, "", "gh: Not Found (HTTP 404)\n"), NOW);
    assert!(matches!(out, Err(DiffReadFailure::Failed(_))));
    let out = diff_outcome(
        &RunOutcome::TimedOut {
            stderr: String::new(),
        },
        NOW,
    );
    assert_eq!(out, Err(DiffReadFailure::Failed(ReadingFailure::Passing)));
}

#[test]
fn the_fake_answers_in_order_and_records_the_calls() {
    let fake = FakePullRequestSource::new()
        .with_diff_answer("diff --git a/x b/x\n".into())
        .with_diff_failure(DiffReadFailure::TooLarge);
    assert!(fake.diff(&repo(), 7, NOW).is_ok());
    assert_eq!(fake.diff(&repo(), 8, NOW), Err(DiffReadFailure::TooLarge));
    assert_eq!(
        fake.diff_calls(),
        vec![
            ("acme/widgets".to_string(), 7),
            ("acme/widgets".to_string(), 8)
        ]
    );
}

#[test]
fn a_rate_limit_is_named_with_its_pause_and_a_missing_gh_is_unavailable() {
    let limited = diff_outcome(
        &exited(1, "", "gh: API rate limit exceeded (HTTP 403)\n"),
        NOW,
    );
    assert_eq!(
        limited,
        Err(DiffReadFailure::Failed(ReadingFailure::RateLimited {
            until: NOW + 60
        }))
    );
    let missing = diff_outcome(
        &RunOutcome::SpawnFailed("No such file or directory (os error 2)".into()),
        NOW,
    );
    assert_eq!(
        missing,
        Err(DiffReadFailure::Failed(ReadingFailure::Unavailable))
    );
    let signed_out = diff_outcome(
        &exited(
            4,
            "",
            "To get started with GitHub CLI, please run:  gh auth login\n",
        ),
        NOW,
    );
    assert_eq!(
        signed_out,
        Err(DiffReadFailure::Failed(ReadingFailure::Unavailable))
    );
}

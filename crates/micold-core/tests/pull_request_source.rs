//! The pull request source seam (feature 040, contracts/pull-request-source.md §1 and §5): the
//! fake, reading in chunks of 50, and `read_outcome`. Nothing here runs `gh`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_core::github::GithubRepo;
use micold_core::process::RunOutcome;
use micold_core::pull_request::{
    read_in_chunks, read_outcome, FakePullRequestSource, PrState, PullRequestSource,
    PullRequestStatus, ReadingFailure, ReviewState, BRANCHES_PER_REQUEST,
};

const NOW: u64 = 1_790_000_000;

fn fixture(file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a github.com remote")
}

fn names(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("branch-{i:03}")).collect()
}

fn status(number: u64) -> PullRequestStatus {
    PullRequestStatus {
        number,
        title: format!("title {number}"),
        url: format!("https://github.com/acme/widgets/pull/{number}"),
        state: PrState::Merged,
        review: ReviewState::None,
        head: "abc".into(),
    }
}

type Answer = Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>;

/// One entry per branch of the chunk.
fn one_each(chunk: &[String]) -> Answer {
    Ok(chunk
        .iter()
        .enumerate()
        .map(|(i, b)| (b.clone(), status(i as u64 + 1)))
        .collect())
}

/// U42
#[test]
fn fake_answers_in_order_and_records_calls() {
    let answer: BTreeMap<_, _> = [("feat/a".to_string(), status(7))].into();
    let source = FakePullRequestSource::new()
        .with_answer(answer.clone())
        .with_failure(ReadingFailure::Unavailable);
    let repo = repo();
    let first = vec!["feat/a".to_string()];
    let second = vec!["x".to_string(), "y".to_string()];
    let third = vec!["z".to_string()];

    assert_eq!(
        source.read(&repo, &first, NOW),
        Ok(answer),
        "the first read gets the first scripted answer"
    );
    assert_eq!(
        source.read(&repo, &second, NOW),
        Err(ReadingFailure::Unavailable),
        "the second read gets the scripted failure"
    );
    assert_eq!(
        source.read(&repo, &third, NOW),
        Err(ReadingFailure::Passing),
        "a read with nothing scripted answers Passing"
    );
    assert_eq!(
        source.calls(),
        vec![
            ("acme/widgets".to_string(), first),
            ("acme/widgets".to_string(), second),
            ("acme/widgets".to_string(), third),
        ],
        "every read is recorded with its repository and branches, in order"
    );
}

/// U43
#[test]
fn no_branches_makes_no_request() {
    let mut asked = 0;
    let result = read_in_chunks(&[], |_| {
        asked += 1;
        Ok(BTreeMap::new())
    });
    assert_eq!(
        result,
        Ok(BTreeMap::new()),
        "no branches is an empty answer"
    );
    assert_eq!(asked, 0, "no branches makes no request");
}

/// U44
#[test]
fn branches_are_read_in_chunks_of_fifty() {
    assert_eq!(
        BRANCHES_PER_REQUEST, 50,
        "one request asks about 50 branches"
    );
    for (n, sizes) in [(50, vec![50]), (51, vec![50, 1]), (120, vec![50, 50, 20])] {
        let branches = names(n);
        let seen = RefCell::new(Vec::<Vec<String>>::new());
        let result = read_in_chunks(&branches, |chunk| {
            seen.borrow_mut().push(chunk.to_vec());
            one_each(chunk)
        });
        let seen = seen.into_inner();
        assert_eq!(
            seen.iter().map(Vec::len).collect::<Vec<_>>(),
            sizes,
            "{n} branches are asked for in chunks of {sizes:?}"
        );
        assert_eq!(
            seen.concat(),
            branches,
            "{n} branches: the chunks are the branches in order, none repeated or lost"
        );
        let map = result.expect("every chunk answered");
        assert_eq!(
            map.keys().cloned().collect::<Vec<_>>(),
            {
                let mut sorted = branches.clone();
                sorted.sort();
                sorted
            },
            "{n} branches: the result holds every chunk's entries"
        );
    }
}

/// U45
#[test]
fn a_failing_chunk_ends_the_reading_with_nothing_half_read() {
    let branches = names(120);
    let mut asked = 0;
    let result = read_in_chunks(&branches, |chunk| {
        asked += 1;
        if asked == 2 {
            Err(ReadingFailure::Unavailable)
        } else {
            one_each(chunk)
        }
    });
    assert_eq!(
        result,
        Err(ReadingFailure::Unavailable),
        "the first chunk that fails ends the reading with its failure; nothing of chunk one is returned"
    );
    assert_eq!(asked, 2, "the third chunk is never asked for");
}

/// U45
#[test]
fn a_rate_limit_failure_is_passed_on_unchanged() {
    let branches = names(120);
    let mut asked = 0;
    let result = read_in_chunks(&branches, |chunk| {
        asked += 1;
        if asked == 2 {
            Err(ReadingFailure::RateLimited {
                until: 1_790_966_100,
            })
        } else {
            one_each(chunk)
        }
    });
    assert_eq!(
        result,
        Err(ReadingFailure::RateLimited {
            until: 1_790_966_100
        }),
        "the pause time reaches the caller unchanged (FR-024)"
    );
    assert_eq!(asked, 2, "no chunk is asked for after the limit");
}

fn exited(code: i32, stdout: Vec<u8>, stderr: &str) -> RunOutcome {
    RunOutcome::Exited {
        code,
        stdout,
        stderr: stderr.into(),
    }
}

/// `read_outcome`
#[test]
fn a_good_answer_is_keyed_by_the_branches_passed() {
    let branches = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
    let map = read_outcome(
        &exited(0, fixture("pr_three_branches.txt"), ""),
        &branches,
        NOW,
    )
    .expect("exit 0 with HTTP 200 and a readable body is the answer");
    assert_eq!(
        map.keys().cloned().collect::<Vec<_>>(),
        vec!["alpha".to_string(), "beta".to_string()],
        "two branches have a pull request, keyed by the names passed in"
    );
    assert_eq!(
        map["alpha"].number, 347,
        "the first branch's open pull request"
    );
    assert_eq!(
        map["beta"].number, 536,
        "the second branch's merged pull request"
    );
}

/// `read_outcome`
#[test]
fn a_failed_run_is_its_failure() {
    let branches = vec!["main".to_string()];
    assert_eq!(
        read_outcome(
            &exited(
                1,
                fixture("pr_repo_not_found.txt"),
                "gh: Could not resolve to a Repository with the name 'jaroslawherod/no-such-repository-040'."
            ),
            &branches,
            NOW
        ),
        Err(ReadingFailure::Unavailable),
        "a repository the sign-in cannot see removes the rows' status (FR-025)"
    );
    assert_eq!(
        read_outcome(&exited(0, fixture("pr_truncated.txt"), ""), &branches, NOW),
        Err(ReadingFailure::Passing),
        "a cut answer is waited out (FR-019)"
    );
    assert_eq!(
        read_outcome(
            &RunOutcome::TimedOut {
                stderr: String::new()
            },
            &branches,
            NOW
        ),
        Err(ReadingFailure::Passing),
        "no answer in time is waited out (FR-019, FR-021)"
    );
    assert_eq!(
        read_outcome(
            &exited(0, fixture("pr_rate_limited_graphql.txt"), ""),
            &branches,
            NOW
        ),
        Err(ReadingFailure::RateLimited {
            until: 1_790_966_100
        }),
        "a RATE_LIMITED answer pauses until the reset, even at exit 0 (FR-024)"
    );
}

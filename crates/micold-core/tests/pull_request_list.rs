//! Every pull request of a worktree's branch set (feature 1003, contracts/pull-request-list.md §2,
//! research R3): the query, its arguments, reading the multi-branch answer, the chunked read, and
//! the scripted source. Nothing here runs `gh`.

use std::cell::RefCell;
use std::path::PathBuf;

use micold_core::github::GithubRepo;
use micold_core::process::RunOutcome;
use micold_core::pull_request::{
    list_args, list_in_chunks, list_query, read_list_outcome, BranchPrState, BranchPullRequest,
    FakePullRequestSource, PullRequestSource, ReadingFailure, LIST_BRANCHES_PER_REQUEST,
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

fn branches(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn answered(file: &str) -> RunOutcome {
    RunOutcome::Exited {
        code: 0,
        stdout: fixture(file),
        stderr: String::new(),
    }
}

fn read(file: &str, asked: &[&str]) -> Result<Vec<BranchPullRequest>, ReadingFailure> {
    read_list_outcome(&answered(file), &branches(asked), NOW)
}

fn numbers(list: &[BranchPullRequest]) -> Vec<u64> {
    list.iter().map(|p| p.number).collect()
}

/// A listed pull request made at `created_at`.
fn pr(number: u64, created_at: &str) -> BranchPullRequest {
    BranchPullRequest {
        number,
        title: format!("PR {number}"),
        url: format!("https://github.com/acme/widgets/pull/{number}"),
        state: BranchPrState::Open,
        created_at: created_at.to_string(),
        base: "main".to_string(),
        head: format!("{number:040x}"),
        head_branch: format!("feat/{number}"),
    }
}

/// The `-f` values of a `gh` argument list.
fn fields(args: &[String]) -> Vec<&str> {
    args.iter()
        .enumerate()
        .filter(|(i, _)| *i > 0 && args[i - 1] == "-f")
        .map(|(_, a)| a.as_str())
        .collect()
}

#[test]
fn every_pull_request_of_every_branch_is_listed_newest_first_with_its_head_branch() {
    let list = read(
        "pr_list_branches_three.txt",
        &["feat/a", "feat/b", "feat/c"],
    )
    .expect("readable");
    let seen: Vec<(u64, BranchPrState, &str)> = list
        .iter()
        .map(|p| (p.number, p.state, p.head_branch.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            (14, BranchPrState::Open, "feat/c"),
            (13, BranchPrState::Draft, "feat/c"),
            (12, BranchPrState::Closed, "feat/b"),
            (11, BranchPrState::Merged, "feat/a"),
        ],
        "across branches, by creation time (FR-001, FR-004)"
    );
    assert_eq!(list[0].title, "Add the export");
    assert_eq!(list[0].base, "main");
}

#[test]
fn a_branch_reopened_under_the_same_name_lists_all_its_pull_requests() {
    let list = read("pr_list_branches_reopened.txt", &["feat/x"]).expect("readable");
    assert_eq!(numbers(&list), [32, 31, 30]);
    assert!(list.iter().all(|p| p.head_branch == "feat/x"));
}

#[test]
fn the_default_branchs_pull_requests_are_dropped() {
    let list = read("pr_list_branches_default.txt", &["main", "feat/d"]).expect("readable");
    assert_eq!(numbers(&list), [41], "main is never part of the set (FR-002)");
}

#[test]
fn a_pull_request_from_a_fork_is_dropped() {
    let list = read("pr_list_branches_fork.txt", &["feat/e"]).expect("readable");
    assert_eq!(numbers(&list), [20], "FR-014");
}

#[test]
fn equal_creation_times_put_the_higher_number_first() {
    let list = read("pr_list_branches_tie.txt", &["feat/a", "feat/b"]).expect("readable");
    assert_eq!(numbers(&list), [9, 7], "FR-004");
}

#[test]
fn branches_without_pull_requests_give_an_empty_list() {
    assert_eq!(
        read("pr_list_branches_empty.txt", &["feat/a", "feat/b"]).expect("readable"),
        []
    );
}

#[test]
fn an_answer_missing_an_asked_branch_is_passing_not_a_short_list() {
    assert_eq!(
        read("pr_list_branches_empty.txt", &["feat/a", "feat/b", "feat/c"]),
        Err(ReadingFailure::Passing),
        "no branch is silently dropped (FR-013)"
    );
}

#[test]
fn an_unknown_state_makes_the_read_passing_not_a_wrong_mark() {
    assert_eq!(
        read("pr_list_branches_unknown_state.txt", &["feat/a"]),
        Err(ReadingFailure::Passing)
    );
}

#[test]
fn a_failed_run_is_classified_like_the_status_read() {
    let outcome = RunOutcome::Exited {
        code: 1,
        stdout: Vec::new(),
        stderr: "gh: To get started with GitHub CLI, please run:  gh auth login".to_string(),
    };
    assert_eq!(
        read_list_outcome(&outcome, &branches(&["feat/a"]), NOW),
        Err(ReadingFailure::Unavailable)
    );
}

#[test]
fn the_query_has_one_alias_per_branch_and_names_no_branch() {
    let query = list_query(3);
    for i in 0..3 {
        assert!(query.contains(&format!("$b{i}: String!")), "declares $b{i}");
        assert!(
            query.contains(&format!(
                "p{i}: pullRequests(headRefName: $b{i}, first: 100, orderBy: {{field: CREATED_AT, direction: DESC}})"
            )),
            "alias p{i} reads $b{i}"
        );
    }
    assert!(!query.contains("$b3"), "exactly n variables");
    assert!(query.contains("defaultBranchRef { name }"));
    assert!(query.contains("headRefName"), "each pull request's own branch");
    assert!(query.contains("rateLimit { remaining resetAt }"));
}

#[test]
fn the_args_send_every_branch_as_a_string_variable() {
    let asked = branches(&["true", "@x", "feat/odd\"name{x}"]);
    let args = list_args(&repo(), &asked);
    assert_eq!(
        &args[..5],
        ["api", "graphql", "--hostname", "github.com", "--include"]
    );
    let sent = fields(&args);
    assert!(sent[0].starts_with("query="));
    assert!(asked.iter().all(|b| !sent[0].contains(b.as_str())));
    assert_eq!(
        sent[1..],
        [
            "owner=acme",
            "name=widgets",
            "b0=true",
            "b1=@x",
            "b2=feat/odd\"name{x}"
        ]
    );
    assert!(
        !args.iter().any(|a| a == "-F"),
        "-f keeps `true` and `@x` strings"
    );
}

#[test]
fn twenty_five_branches_are_one_request_and_twenty_six_are_two() {
    let asks = |n: usize| {
        let all: Vec<String> = (0..n).map(|i| format!("b{i}")).collect();
        let sizes = RefCell::new(Vec::new());
        list_in_chunks(&all, |chunk| {
            sizes.borrow_mut().push(chunk.len());
            Ok(Vec::new())
        })
        .expect("every chunk answered");
        sizes.into_inner()
    };
    assert_eq!(LIST_BRANCHES_PER_REQUEST, 25);
    assert_eq!(asks(25), [25], "SC-007: 25 branches, one request");
    assert_eq!(asks(26), [25, 1]);
    assert!(asks(0).is_empty(), "no branch, no request");
}

#[test]
fn the_chunks_are_merged_newest_first_then_by_number() {
    let all: Vec<String> = (0..26).map(|i| format!("b{i}")).collect();
    let mut answers = vec![
        vec![pr(1, "2026-10-01T09:00:00Z"), pr(5, "2026-10-07T09:00:00Z")],
        vec![pr(2, "2026-10-05T09:00:00Z"), pr(7, "2026-10-07T09:00:00Z")],
    ]
    .into_iter();
    let list = list_in_chunks(&all, |_| Ok(answers.next().unwrap())).expect("both answered");
    assert_eq!(numbers(&list), [7, 5, 2, 1]);
}

#[test]
fn a_failing_second_chunk_fails_the_whole_read() {
    let all: Vec<String> = (0..26).map(|i| format!("b{i}")).collect();
    let mut answers = vec![
        Ok(vec![pr(1, "2026-10-01T09:00:00Z")]),
        Err(ReadingFailure::Unavailable),
    ]
    .into_iter();
    assert_eq!(
        list_in_chunks(&all, |_| answers.next().unwrap()),
        Err(ReadingFailure::Unavailable),
        "whole or a failure: no branch silently dropped (FR-013)"
    );
}

#[test]
fn the_fake_answers_in_script_order_and_records_each_call() {
    let one = vec![pr(1, "2026-10-01T09:00:00Z")];
    let source = FakePullRequestSource::new()
        .with_list_answer(one.clone())
        .with_list_failure(ReadingFailure::Unavailable);
    let ab = branches(&["feat/a", "feat/b"]);
    let c = branches(&["feat/c"]);
    assert_eq!(source.list_branches(&repo(), &ab, NOW), Ok(one));
    assert_eq!(
        source.list_branches(&repo(), &c, NOW),
        Err(ReadingFailure::Unavailable)
    );
    assert_eq!(
        source.list_branches(&repo(), &c, NOW),
        Err(ReadingFailure::Passing),
        "nothing scripted answers Passing"
    );
    assert_eq!(
        source.list_calls(),
        [
            ("acme/widgets".to_string(), ab.clone()),
            ("acme/widgets".to_string(), c.clone()),
            ("acme/widgets".to_string(), c),
        ]
    );
}

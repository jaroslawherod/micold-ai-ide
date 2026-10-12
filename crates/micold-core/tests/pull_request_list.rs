//! The pull requests of one branch (feature 775, contracts/pull-request-reads.md §1): the query,
//! its arguments, reading the answer, and the scripted source. Nothing here runs `gh`.

use std::path::PathBuf;

use micold_core::github::GithubRepo;
use micold_core::process::RunOutcome;
use micold_core::pull_request::{
    list_args, list_query, read_list_outcome, BranchPrState, FakePullRequestSource,
    PullRequestSource, ReadingFailure,
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

fn answered(file: &str) -> RunOutcome {
    RunOutcome::Exited {
        code: 0,
        stdout: fixture(file),
        stderr: String::new(),
    }
}

fn read(file: &str) -> Result<Vec<micold_core::pull_request::BranchPullRequest>, ReadingFailure> {
    read_list_outcome(&answered(file), NOW)
}

#[test]
fn every_state_is_listed_with_its_own_mark_newest_first() {
    let list = read("pr_list_mixed.txt").expect("readable");
    let seen: Vec<(u64, BranchPrState)> = list.iter().map(|p| (p.number, p.state)).collect();
    assert_eq!(
        seen,
        [
            (14, BranchPrState::Open),
            (13, BranchPrState::Draft),
            (12, BranchPrState::Merged),
            (11, BranchPrState::Closed),
        ]
    );
    assert_eq!(list[0].title, "Add the export");
    assert_eq!(list[0].base, "main");
}

#[test]
fn a_pull_request_from_a_fork_is_dropped() {
    let numbers: Vec<u64> = read("pr_list_fork.txt")
        .expect("readable")
        .iter()
        .map(|p| p.number)
        .collect();
    assert_eq!(numbers, [20]);
}

#[test]
fn an_answer_out_of_order_is_sorted_newest_first() {
    let numbers: Vec<u64> = read("pr_list_unordered.txt")
        .expect("readable")
        .iter()
        .map(|p| p.number)
        .collect();
    assert_eq!(numbers, [43, 42, 41]);
}

#[test]
fn a_branch_without_pull_requests_gives_an_empty_list() {
    assert_eq!(read("pr_list_empty.txt").expect("readable"), []);
}

#[test]
fn an_unknown_state_makes_the_read_passing_not_a_wrong_mark() {
    assert_eq!(
        read("pr_list_unknown_state.txt"),
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
        read_list_outcome(&outcome, NOW),
        Err(ReadingFailure::Unavailable)
    );
}

#[test]
fn the_query_holds_no_branch_and_the_args_send_variables_only() {
    let branch = "feat/odd\"name{x}";
    let args = list_args(&repo(), branch);
    assert!(!list_query().contains(branch));
    assert_eq!(
        &args[..5],
        ["api", "graphql", "--hostname", "github.com", "--include"]
    );
    let fields: Vec<&str> = args
        .iter()
        .enumerate()
        .filter(|(i, _)| *i > 0 && args[i - 1] == "-f")
        .map(|(_, a)| a.as_str())
        .collect();
    assert_eq!(fields.len(), 4);
    assert!(fields[0].starts_with("query="));
    assert!(!fields[0].contains(branch));
    assert_eq!(fields[1..], ["owner=acme", "name=widgets", &format!("b={branch}")]);
    assert!(!args.iter().any(|a| a == "-F"));
}

#[test]
fn the_fake_answers_in_script_order_and_records_each_call() {
    let one = read("pr_list_mixed.txt").expect("readable");
    let source = FakePullRequestSource::new()
        .with_list_answer(one.clone())
        .with_list_failure(ReadingFailure::Unavailable);
    assert_eq!(source.list_branch(&repo(), "feat/a", NOW), Ok(one));
    assert_eq!(
        source.list_branch(&repo(), "feat/b", NOW),
        Err(ReadingFailure::Unavailable)
    );
    assert_eq!(
        source.list_branch(&repo(), "feat/c", NOW),
        Err(ReadingFailure::Passing),
        "nothing scripted answers Passing"
    );
    assert_eq!(
        source.list_calls(),
        [
            ("acme/widgets".to_string(), "feat/a".to_string()),
            ("acme/widgets".to_string(), "feat/b".to_string()),
            ("acme/widgets".to_string(), "feat/c".to_string()),
        ]
    );
}

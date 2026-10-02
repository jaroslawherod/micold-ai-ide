//! The request `status_query` and `status_args` build for one batch of branches (feature 040,
//! contracts/pull-request-source.md section 2).
//!
//! `tests/fixtures/gh/status_query_50.graphql` pins the document for 50 branches; it was sent to
//! GitHub once and answered without `errors`. The n = 1 document is pinned in this file.

use std::path::PathBuf;

use micold_core::github::GithubRepo;
use micold_core::pull_request::{status_args, status_query};

const QUERY_1: &str = "query($owner: String!, $name: String!, $b0: String!) { repository(owner: $owner, name: $name) { o0: pullRequests(headRefName: $b0, states: OPEN, first: 10, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...pr } } r0: pullRequests(headRefName: $b0, first: 10, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...pr } } } rateLimit { remaining resetAt } } fragment pr on PullRequest { number title url state isDraft createdAt isCrossRepository headRefOid reviewDecision commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 1) { checkRunCount checkRunCountsByState { state count } statusContextCount statusContextCountsByState { state count } } } } } } }";

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a github.com url")
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// U2.
#[test]
fn u2_query_for_one_branch_is_pinned() {
    let q = status_query(1);
    assert_eq!(
        q, QUERY_1,
        "U2: the n = 1 document must match the pinned text byte for byte"
    );
    assert!(!q.contains('\n'), "U2: the document must be one line");
    for part in [
        "o0:",
        "r0:",
        "rateLimit { remaining resetAt }",
        "fragment pr on PullRequest",
        "checkRunCountsByState",
        "statusContextCountsByState",
    ] {
        assert!(q.contains(part), "U2: the document must hold {part:?}");
    }
}

/// U3.
#[test]
fn u3_query_for_fifty_branches_is_pinned() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gh/status_query_50.graphql");
    let pinned =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let q = status_query(50);
    assert_eq!(
        q,
        pinned.trim_end_matches('\n'),
        "U3: the n = 50 document must match the pin file"
    );
    assert!(!q.contains('\n'), "U3: the document must be one line");
    for i in 0..50 {
        for part in [
            format!(" o{i}: "),
            format!(" r{i}: "),
            format!("$b{i}: String!"),
        ] {
            assert!(q.contains(&part), "U3: the document must hold {part:?}");
        }
    }
    for part in [" o50: ", " r50: ", "$b50: String!"] {
        assert!(!q.contains(part), "U3: the document must not hold {part:?}");
    }
}

/// U4.
#[test]
fn u4_no_branch_name_appears_in_the_document() {
    let branches = strings(&[
        "zq-distinct-alpha",
        "zq-distinct-beta/gamma",
        "zq-distinct-delta",
    ]);
    let args = status_args(&repo(), &branches);
    let query = args
        .iter()
        .find_map(|a| a.strip_prefix("query="))
        .expect("U4: status_args must hold a query= argument");
    for b in &branches {
        assert!(
            !query.contains(b.as_str()),
            "U4: the document must not hold the branch name {b:?}"
        );
    }
    assert_eq!(
        query,
        status_query(branches.len()),
        "U4: the query= argument must be exactly status_query(n)"
    );
}

/// U5.
#[test]
fn u5_branch_names_pass_through_unchanged() {
    let names = [
        "feat/\"quoted\"",
        "price$var",
        "with space",
        "-leading-dash",
        "true",
        "123",
        "@file",
    ];
    let branches = strings(&names);
    let args = status_args(&repo(), &branches);
    for (i, b) in names.iter().enumerate() {
        let want = format!("b{i}={b}");
        let hits: Vec<usize> = (0..args.len()).filter(|&k| args[k] == want).collect();
        assert_eq!(
            hits.len(),
            1,
            "U5: exactly one argument {want:?} must exist"
        );
        assert_eq!(
            args[hits[0] - 1],
            "-f",
            "U5: {want:?} must follow its own -f"
        );
    }
    let f_count = args.iter().filter(|a| *a == "-f").count();
    assert_eq!(
        f_count,
        3 + names.len(),
        "U5: one -f per query, owner, name and branch"
    );
}

/// U6, FR-028, FR-031.
#[test]
fn u6_args_are_exactly_the_contract() {
    let branches = strings(&["feat/a", "main"]);
    let args = status_args(&repo(), &branches);
    let want = vec![
        "api".to_string(),
        "graphql".to_string(),
        "--hostname".to_string(),
        "github.com".to_string(),
        "--include".to_string(),
        "-f".to_string(),
        format!("query={}", status_query(2)),
        "-f".to_string(),
        "owner=acme".to_string(),
        "-f".to_string(),
        "name=widgets".to_string(),
        "-f".to_string(),
        "b0=feat/a".to_string(),
        "-f".to_string(),
        "b1=main".to_string(),
    ];
    assert_eq!(
        args, want,
        "U6: the arguments must be exactly the contract's, in order"
    );
    for a in args.iter().filter(|a| !a.starts_with("query=")) {
        assert!(
            !["-F", "-H", "--header"].contains(&a.as_str()),
            "U6: no {a:?} argument (FR-028)"
        );
        let low = a.to_lowercase();
        assert!(
            !low.contains("token") && !low.contains("authorization"),
            "U6: no credential in {a:?} (FR-028)"
        );
    }
}

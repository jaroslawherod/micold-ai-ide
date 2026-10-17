//! Feature 813 (T024, contracts/wire.md W5): the status reading also asks for each branch's open and
//! merged pull request counts and for whether the current one conflicts, with no further request.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_core::overview::PrCounts;
use micold_core::pull_request::{
    parse_status, split_response, status_query, ReadingFailure, StatusReading,
};

fn branches(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_string()).collect()
}

fn body_of(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gh")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let response = split_response(&bytes).expect("a recorded answer");
    serde_json::from_slice(response.body).expect("the body is JSON")
}

fn parse(v: &serde_json::Value, asked: &[&str]) -> Result<StatusReading, ReadingFailure> {
    parse_status(v.to_string().as_bytes(), &branches(asked))
}

fn counts(open: u8, merged: u8) -> PrCounts {
    PrCounts {
        open,
        merged,
        open_capped: false,
        merged_capped: false,
    }
}

fn repository(v: &mut serde_json::Value) -> &mut serde_json::Map<String, serde_json::Value> {
    v["data"]["repository"].as_object_mut().expect("repository")
}

fn nodes(cross: &[bool]) -> serde_json::Value {
    let list: Vec<_> = cross
        .iter()
        .map(|c| serde_json::json!({ "isCrossRepository": c }))
        .collect();
    serde_json::json!({ "nodes": list })
}

#[test]
fn pull_request_query_asks_mergeable_and_both_counts() {
    assert!(status_query(1).contains("mergeable"), "FR-019: one request");
    let q = status_query(2);
    for i in 0..2 {
        for (alias, states) in [("oc", "OPEN"), ("mc", "MERGED")] {
            let part = format!(
                "{alias}{i}: pullRequests(headRefName: $b{i}, states: {states}, first: 100) \
                 {{ nodes {{ isCrossRepository }} }}"
            );
            assert!(q.contains(&part), "the document must hold {part:?}");
        }
    }
}

#[test]
fn pull_request_counts_come_from_the_recorded_answer_for_every_asked_branch() {
    let reading = parse(&body_of("pr_three_branches.txt"), &["a", "b", "c"]).expect("readable");
    assert_eq!(
        reading.counts,
        BTreeMap::from([
            ("a".to_string(), counts(1, 8)),
            ("b".to_string(), counts(0, 2)),
            ("c".to_string(), counts(0, 0)),
        ]),
        "a branch with no pull request reads 0 / 0, not unknown"
    );
    assert_eq!(reading.statuses.len(), 2, "the statuses are as before");
}

#[test]
fn pull_request_counts_leave_out_pull_requests_from_forks() {
    let mut v = body_of("pr_three_branches.txt");
    let repo = repository(&mut v);
    repo.insert("oc0".into(), nodes(&[false, true]));
    repo.insert("mc0".into(), nodes(&[true, true, false, false, false]));
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    assert_eq!(reading.counts["a"], counts(1, 3));
    let reading = parse(&body_of("pr_cross_repository.txt"), &["patch-1"]).expect("readable");
    assert_eq!(reading.counts["patch-1"], counts(0, 0), "only forks");
}

#[test]
fn pull_request_counts_cap_at_a_full_page() {
    let mut v = body_of("pr_three_branches.txt");
    let repo = repository(&mut v);
    repo.insert("mc0".into(), nodes(&[false; 100]));
    repo.insert("oc1".into(), nodes(&[false; 100]));
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    assert_eq!(
        reading.counts["a"],
        PrCounts {
            open: 1,
            merged: 100,
            open_capped: false,
            merged_capped: true
        }
    );
    assert_eq!(
        reading.counts["b"],
        PrCounts {
            open: 100,
            merged: 2,
            open_capped: true,
            merged_capped: false
        }
    );
}

#[test]
fn pull_request_a_missing_count_alias_fails_the_whole_reading() {
    for alias in ["oc1", "mc2"] {
        let mut v = body_of("pr_three_branches.txt");
        assert!(repository(&mut v).remove(alias).is_some());
        assert_eq!(parse(&v, &["a", "b", "c"]), Err(ReadingFailure::Passing));
    }
}

#[test]
fn pull_request_conflict_is_true_only_for_conflicting() {
    for (mergeable, conflict) in [
        (Some("CONFLICTING"), true),
        (Some("MERGEABLE"), false),
        (Some("UNKNOWN"), false),
        (None, false),
    ] {
        let mut v = body_of("pr_three_branches.txt");
        if let Some(value) = mergeable {
            v["data"]["repository"]["o0"]["nodes"][0]["mergeable"] = value.into();
        }
        let reading = parse(&v, &["a", "b", "c"]).expect("readable");
        assert_eq!(reading.statuses["a"].conflict, conflict, "{mergeable:?}");
    }
}

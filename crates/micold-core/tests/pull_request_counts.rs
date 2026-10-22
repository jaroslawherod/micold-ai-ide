//! Feature 813 (T024, contracts/wire.md W5): the status reading also gives each branch's open and
//! merged pull request counts and whether the current one conflicts, with no further request.
//! Feature 1123 (contracts/reading.md P3, P4): the counts come from the listed pull requests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_core::overview::PrCounts;
use micold_core::pull_request::{parse_status, split_response, ReadingFailure, StatusReading};

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

/// A `merged` fragment node per entry: (number, from a fork).
fn merged_nodes(nodes: &[(u64, bool)]) -> serde_json::Value {
    let list: Vec<_> = nodes
        .iter()
        .map(|(number, fork)| {
            serde_json::json!({
                "number": number,
                "url": format!("https://github.com/acme/widgets/pull/{number}"),
                "state": "MERGED",
                "isDraft": false,
                "createdAt": format!("2026-01-{:02}T00:00:00Z", number % 28 + 1),
                "isCrossRepository": fork,
            })
        })
        .collect();
    serde_json::json!({ "nodes": list })
}

/// `pr` fragment nodes copied from the open pull request of branch `a` (#347), renumbered; `fork`
/// flags each.
fn open_nodes(v: &serde_json::Value, forks: &[bool]) -> serde_json::Value {
    let template = v["data"]["repository"]["o0"]["nodes"][0].clone();
    let list: Vec<_> = forks
        .iter()
        .enumerate()
        .map(|(i, fork)| {
            let mut node = template.clone();
            node["number"] = (1000 + i as u64).into();
            node["createdAt"] = format!("2026-02-{:02}T00:00:00Z", i + 1).into();
            node["isCrossRepository"] = (*fork).into();
            node
        })
        .collect();
    serde_json::json!({ "nodes": list })
}

#[test]
fn pull_request_counts_come_from_the_recorded_answer_for_every_asked_branch() {
    let reading = parse(&body_of("pr_three_branches.txt"), &["a", "b", "c"]).expect("readable");
    let got: BTreeMap<_, _> = reading
        .lists
        .iter()
        .map(|(branch, list)| (branch.clone(), list.counts()))
        .collect();
    assert_eq!(
        got,
        BTreeMap::from([
            ("a".to_string(), counts(1, 8)),
            ("b".to_string(), counts(0, 2)),
            ("c".to_string(), counts(0, 0)),
        ]),
        "a branch with no pull request reads 0 / 0, not unknown"
    );
    assert_eq!(reading.statuses.len(), 2, "the statuses are as before");
}

/// Feature 1123 P3: forks are listed nowhere and counted nowhere; a full open page with a fork
/// reads its own count with *+*.
#[test]
fn pull_request_lists_leave_out_pull_requests_from_forks() {
    let mut v = body_of("pr_three_branches.txt");
    let merged = merged_nodes(&[(1, true), (2, true), (3, false), (4, false), (5, false)]);
    repository(&mut v).insert("m0".into(), merged);
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    // Branch a's recent page (`r0`) of the fixture holds eight merged pull requests of its own
    // (#310 … #217), listed beside the three non-fork nodes of the merged page above.
    const MERGED_ON_THE_RECENT_PAGE: u8 = 8;
    assert_eq!(
        reading.lists["a"].counts(),
        counts(1, 3 + MERGED_ON_THE_RECENT_PAGE)
    );
    assert!(reading.lists["a"]
        .prs()
        .iter()
        .all(|pr| ![1, 2].contains(&pr.number)));
    let reading = parse(&body_of("pr_cross_repository.txt"), &["patch-1"]).expect("readable");
    assert!(reading.lists["patch-1"].prs().is_empty(), "only forks");
    assert_eq!(reading.lists["patch-1"].counts(), counts(0, 0));

    let mut v = body_of("pr_three_branches.txt");
    let mut forks = [false; 10];
    forks[3] = true;
    let open = open_nodes(&v, &forks);
    repository(&mut v).insert("o1".into(), open);
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    let b = reading.lists["b"].counts();
    assert_eq!((b.open, b.open_capped), (9, true), "9+ open");
}

/// Feature 1123 P4: a full page of ten reads capped, nine reads exact.
#[test]
fn pull_request_counts_cap_at_a_full_page_of_ten() {
    let mut v = body_of("pr_three_branches.txt");
    let open = open_nodes(&v, &[false; 10]);
    let repo = repository(&mut v);
    repo.insert("o1".into(), open);
    repo.insert(
        "m2".into(),
        merged_nodes(&(1..=10).map(|n| (n, false)).collect::<Vec<_>>()),
    );
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    let b = reading.lists["b"].counts();
    assert_eq!((b.open, b.open_capped), (10, true), "10+ open");
    let c = reading.lists["c"].counts();
    assert_eq!((c.merged, c.merged_capped), (10, true), "10+ merged");

    let mut v = body_of("pr_three_branches.txt");
    let open = open_nodes(&v, &[false; 9]);
    let repo = repository(&mut v);
    repo.insert("o1".into(), open);
    repo.insert(
        "m2".into(),
        merged_nodes(&(1..=9).map(|n| (n, false)).collect::<Vec<_>>()),
    );
    let reading = parse(&v, &["a", "b", "c"]).expect("readable");
    assert_eq!(reading.lists["b"].counts().open, 9);
    assert!(!reading.lists["b"].counts().open_capped);
    assert_eq!(reading.lists["c"].counts(), counts(0, 9), "9 merged, exact");
}

#[test]
fn pull_request_a_missing_merged_alias_fails_the_whole_reading() {
    for alias in ["m1", "m2"] {
        let mut v = body_of("pr_three_branches.txt");
        assert!(repository(&mut v).remove(alias).is_some());
        assert_eq!(parse(&v, &["a", "b", "c"]), Err(ReadingFailure::Passing));
    }
}

/// Data-model §2 (tdd-verify close r1 F4): an unknown state on the merged page fails the chunk,
/// as on the open page, rather than reading as merged.
#[test]
fn pull_request_an_unknown_state_on_the_merged_page_fails_the_reading() {
    let mut v = body_of("pr_three_branches.txt");
    let mut merged = merged_nodes(&[(3, false)]);
    merged["nodes"][0]["state"] = "FOO".into();
    repository(&mut v).insert("m0".into(), merged);
    assert_eq!(parse(&v, &["a", "b", "c"]), Err(ReadingFailure::Passing));
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

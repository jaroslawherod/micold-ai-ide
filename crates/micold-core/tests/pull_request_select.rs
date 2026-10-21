//! Choosing the one pull request a branch shows (feature 040, contracts/pull-request-source.md §3).
//!
//! An open pull request beats a newer merged or closed one, the later-created wins within a group,
//! cross-repository nodes are ignored, and a state this version does not know is unreadable
//! (FR-004, FR-005, FR-002, FR-003).

use micold_core::pull_request::{
    select_pull_request, CheckCounts, CheckStatus, PrNode, PrState, PullRequestStatus, ReviewState,
    Unreadable,
};

fn node(number: u64, state: &str, created_at: &str) -> PrNode {
    PrNode {
        number,
        title: format!("title {number}"),
        url: format!("https://github.com/o/r/pull/{number}"),
        state: state.to_string(),
        is_draft: false,
        created_at: created_at.to_string(),
        is_cross_repository: false,
        head: format!("head{number}"),
        review_decision: None,
        checks: None,
        conflict: false,
    }
}

fn selected_number(open: &[PrNode], recent: &[PrNode]) -> Option<u64> {
    select_pull_request(open, recent)
        .expect("readable nodes")
        .map(|s| s.number)
}

// U16
#[test]
fn an_open_node_is_selected_over_a_newer_merged_one() {
    let open_node = node(1, "OPEN", "2024-01-01T00:00:00Z");
    let merged = node(2, "MERGED", "2024-06-01T00:00:00Z");
    let status = select_pull_request(
        std::slice::from_ref(&open_node),
        &[open_node.clone(), merged],
    )
    .expect("readable")
    .expect("a node is selected");
    assert_eq!(
        status.number, 1,
        "an open pull request wins over a newer merged one (FR-004)"
    );
    assert!(
        matches!(status.state, PrState::Open { .. }),
        "the selected state is open (FR-004)"
    );
}

// U17
#[test]
fn of_two_open_nodes_the_one_created_later_is_selected() {
    let older = node(1, "OPEN", "2024-01-01T00:00:00Z");
    let newer = node(2, "OPEN", "2024-03-01T00:00:00Z");
    assert_eq!(
        selected_number(&[older.clone(), newer.clone()], &[]),
        Some(2),
        "the later-created open node wins, older first (FR-004)"
    );
    assert_eq!(
        selected_number(&[newer, older], &[]),
        Some(2),
        "the later-created open node wins, newer first (FR-004)"
    );
}

// U18
#[test]
fn with_no_open_node_the_later_created_of_merged_and_closed_is_selected() {
    let merged_old = node(1, "MERGED", "2024-01-01T00:00:00Z");
    let closed_new = node(2, "CLOSED", "2024-05-01T00:00:00Z");
    for recent in [
        [merged_old.clone(), closed_new.clone()],
        [closed_new.clone(), merged_old.clone()],
    ] {
        let status = select_pull_request(&[], &recent)
            .expect("readable")
            .expect("a node is selected");
        assert_eq!(status.number, 2, "the newer closed node wins (FR-005)");
        assert_eq!(status.state, PrState::Closed, "it shows as closed (FR-005)");
    }

    let merged_new = node(3, "MERGED", "2024-07-01T00:00:00Z");
    let closed_old = node(4, "CLOSED", "2024-02-01T00:00:00Z");
    for recent in [
        [merged_new.clone(), closed_old.clone()],
        [closed_old, merged_new],
    ] {
        let status = select_pull_request(&[], &recent)
            .expect("readable")
            .expect("a node is selected");
        assert_eq!(status.number, 3, "the newer merged node wins (FR-005)");
        assert_eq!(status.state, PrState::Merged, "it shows as merged (FR-005)");
    }
}

// U19
#[test]
fn a_cross_repository_open_node_is_ignored() {
    let cross = PrNode {
        is_cross_repository: true,
        ..node(1, "OPEN", "2024-09-01T00:00:00Z")
    };
    let closed = node(2, "CLOSED", "2024-01-01T00:00:00Z");
    let status = select_pull_request(std::slice::from_ref(&cross), &[cross.clone(), closed])
        .expect("readable")
        .expect("the same-repository node is selected");
    assert_eq!(
        status.number, 2,
        "a cross-repository node is dropped first (FR-004)"
    );
    assert_eq!(
        status.state,
        PrState::Closed,
        "the closed one shows (FR-004)"
    );
}

// U20
#[test]
fn only_cross_repository_nodes_select_nothing() {
    let cross = |n, state: &str| PrNode {
        is_cross_repository: true,
        ..node(n, state, "2024-01-01T00:00:00Z")
    };
    let open = [cross(1, "OPEN")];
    let recent = [cross(1, "OPEN"), cross(2, "MERGED")];
    assert_eq!(
        select_pull_request(&open, &recent),
        Ok(None),
        "only cross-repository nodes leave no pull request (FR-004)"
    );
}

// U20
#[test]
fn two_empty_slices_select_nothing() {
    assert_eq!(
        select_pull_request(&[], &[]),
        Ok(None),
        "no nodes at all leave no pull request (FR-004)"
    );
}

// U21
#[test]
fn an_open_node_is_a_draft_or_open_by_is_draft_with_its_checks_reduced() {
    let failing = CheckCounts {
        check_runs: vec![("FAILURE".to_string(), 1), ("SUCCESS".to_string(), 2)],
        status_contexts: vec![],
    };
    let draft = PrNode {
        is_draft: true,
        checks: Some(failing.clone()),
        ..node(1, "OPEN", "2024-01-01T00:00:00Z")
    };
    let ready = PrNode {
        is_draft: false,
        checks: Some(failing),
        ..node(2, "OPEN", "2024-01-01T00:00:00Z")
    };
    let unchecked = node(3, "OPEN", "2024-01-01T00:00:00Z");

    let state_of = |n: &PrNode| {
        select_pull_request(std::slice::from_ref(n), &[])
            .expect("readable")
            .expect("selected")
            .state
    };
    assert_eq!(
        state_of(&draft),
        PrState::Draft {
            checks: CheckStatus::Failing
        },
        "an open draft is a draft with reduced checks (FR-002, FR-008)"
    );
    assert_eq!(
        state_of(&ready),
        PrState::Open {
            checks: CheckStatus::Failing
        },
        "an open non-draft is open with reduced checks (FR-002, FR-008)"
    );
    assert_eq!(
        state_of(&unchecked),
        PrState::Open {
            checks: CheckStatus::None
        },
        "an open node without a rollup has no checks (FR-008)"
    );
}

// U21
#[test]
fn a_merged_node_is_merged_even_when_marked_draft() {
    let merged = PrNode {
        is_draft: true,
        ..node(1, "MERGED", "2024-01-01T00:00:00Z")
    };
    let status = select_pull_request(&[], &[merged])
        .expect("readable")
        .expect("selected");
    assert_eq!(
        status.state,
        PrState::Merged,
        "isDraft is ignored for a merged node (FR-002)"
    );
}

// U22
#[test]
fn an_unknown_state_is_unreadable() {
    let locked = node(1, "LOCKED", "2024-01-01T00:00:00Z");
    assert_eq!(
        select_pull_request(&[], &[locked]),
        Err(Unreadable),
        "a state this version does not know cannot be read (FR-002)"
    );
}

// Whole-struct copy of the node's fields, and the review mapping.
#[test]
fn the_selected_status_equals_what_the_node_held() {
    let n = PrNode {
        review_decision: Some("CHANGES_REQUESTED".to_string()),
        ..node(7, "OPEN", "2024-01-01T00:00:00Z")
    };
    assert_eq!(
        select_pull_request(std::slice::from_ref(&n), &[]),
        Ok(Some(PullRequestStatus {
            number: 7,
            title: "title 7".to_string(),
            url: "https://github.com/o/r/pull/7".to_string(),
            state: PrState::Open {
                checks: CheckStatus::None
            },
            review: ReviewState::ChangesRequested,
            head: "head7".to_string(),
            conflict: false,
        })),
        "the status copies number, title, url, review and head from the node (FR-004)"
    );
}

// Review decision mapping.
#[test]
fn review_decisions_map_and_unknown_text_is_none() {
    for (text, expected) in [
        (Some("APPROVED"), ReviewState::Approved),
        (Some("CHANGES_REQUESTED"), ReviewState::ChangesRequested),
        (Some("REVIEW_REQUIRED"), ReviewState::ReviewRequired),
        (None, ReviewState::None),
        (Some("SOMETHING_NEW"), ReviewState::None),
    ] {
        let n = PrNode {
            review_decision: text.map(str::to_string),
            ..node(1, "OPEN", "2024-01-01T00:00:00Z")
        };
        let status = select_pull_request(&[n], &[])
            .expect("readable")
            .expect("selected");
        assert_eq!(
            status.review, expected,
            "review decision {text:?} maps as stated"
        );
    }
}

// ---- feature 1123 K3: the card's first row is the sidebar's pull request ----

use micold_core::pull_request::parse_status;

/// A `pr` fragment node of `state` (`OPEN`, `DRAFT` for an open draft, `MERGED`, `CLOSED`).
fn pr_json(number: u64, state: &str, created_at: &str, fork: bool) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": format!("title {number}"),
        "url": format!("https://github.com/o/r/pull/{number}"),
        "state": if state == "DRAFT" { "OPEN" } else { state },
        "isDraft": state == "DRAFT",
        "createdAt": created_at,
        "isCrossRepository": fork,
        "headRefOid": format!("head{number}"),
        "reviewDecision": null,
        "mergeable": "MERGEABLE",
        "commits": { "nodes": [] },
    })
}

/// The `merged` fragment's fields of a `pr` node.
fn merged_json(node: &serde_json::Value) -> serde_json::Value {
    let keep = [
        "number",
        "url",
        "state",
        "isDraft",
        "createdAt",
        "isCrossRepository",
    ];
    keep.iter()
        .map(|k| ((*k).to_string(), node[*k].clone()))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The answer for one branch with these pages.
fn answer(
    open: &[serde_json::Value],
    recent: &[serde_json::Value],
    merged: &[serde_json::Value],
) -> Vec<u8> {
    serde_json::json!({ "data": { "repository": {
        "o0": { "nodes": open }, "r0": { "nodes": recent }, "m0": { "nodes": merged },
    } } })
    .to_string()
    .into_bytes()
}

/// (first listed, shown) for one branch's answer.
fn first_and_shown(body: &[u8]) -> (Option<u64>, Option<u64>) {
    let reading = parse_status(body, &["x".to_string()]).expect("readable");
    (
        reading.lists["x"].prs().first().map(|pr| pr.number),
        reading.statuses.get("x").map(|status| status.number),
    )
}

/// K3: every combination of up to three own pull requests in the four states, distinct
/// `createdAt`: the list's first entry is the one `select_pull_request` picks.
#[test]
fn k3_the_first_listed_is_the_selected_one_for_every_combination() {
    const STATES: [&str; 4] = ["OPEN", "DRAFT", "MERGED", "CLOSED"];
    for n in 1..=3u32 {
        for combination in 0..4usize.pow(n) {
            let nodes: Vec<_> = (0..n as usize)
                .map(|i| {
                    let state = STATES[combination / 4usize.pow(i as u32) % 4];
                    pr_json(
                        i as u64 + 1,
                        state,
                        &format!("2026-01-0{}T00:00:00Z", i + 1),
                        false,
                    )
                })
                .collect();
            let mut recent = nodes.clone();
            recent.reverse(); // newest first, as GitHub orders the page
            let open: Vec<_> = recent
                .iter()
                .filter(|n| n["state"] == "OPEN")
                .cloned()
                .collect();
            let merged: Vec<_> = recent
                .iter()
                .filter(|n| n["state"] == "MERGED")
                .map(merged_json)
                .collect();
            let (first, shown) = first_and_shown(&answer(&open, &recent, &merged));
            assert_eq!(first, shown, "n = {n}, combination {combination}");
            assert!(first.is_some());
        }
    }
}

/// K3 edge case (a), accepted (data-model §2): two own pull requests created in the same second.
/// The list puts the higher number first; `select_pull_request` keeps the later node of the page.
#[test]
fn k3_a_created_at_tie_lists_the_higher_number_and_selects_the_later_node() {
    let six = pr_json(6, "OPEN", "2026-01-01T00:00:00Z", false);
    let five = pr_json(5, "OPEN", "2026-01-01T00:00:00Z", false);
    let pages = [six, five];
    assert_eq!(
        first_and_shown(&answer(&pages, &pages, &[])),
        (Some(6), Some(5))
    );
}

/// K3 edge case (b), accepted: `r` full of ten forks while `m` holds an own merged one. The sidebar
/// shows none; the list's first row is the merged one.
#[test]
fn k3_a_recent_page_of_forks_lists_the_own_merged_one() {
    let forks: Vec<_> = (100..110)
        .map(|n| pr_json(n, "CLOSED", "2026-02-01T00:00:00Z", true))
        .collect();
    let own = merged_json(&pr_json(7, "MERGED", "2026-01-01T00:00:00Z", false));
    assert_eq!(
        first_and_shown(&answer(&[], &forks, &[own])),
        (Some(7), None)
    );
}

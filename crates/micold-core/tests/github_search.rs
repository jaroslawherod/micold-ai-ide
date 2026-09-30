//! What the search beyond the loaded issues adds to the list (feature 034, FR-005a,
//! contracts/github-issue-source.md §4, data-model §5 invariants 4 and 5).
//!
//! GitHub's search answers with issues the form may already hold, and with issues that matched only
//! in a body or a comment. The first would be listed twice; the second would be shown for a query
//! the user can see nothing of in the row. Both are settled here, before the reducer sees them.

use micold_core::github::{merge_searched, Issue};
use micold_core::typeahead::{rank, Query};

fn issue(number: u64, title: &str) -> Issue {
    Issue::new(number, title.to_string(), Vec::new(), "t".to_string())
}

fn numbers(issues: &[Issue]) -> Vec<u64> {
    issues.iter().map(Issue::number).collect()
}

/// U69 — a searched issue already loaded is not added again (invariant 4); the rest keep GitHub's
/// order.
#[test]
fn merge_drops_loaded_numbers() {
    let loaded = [issue(7, "Loaded"), issue(42, "Also loaded")];
    let searched = vec![
        issue(1200, "Beyond the cap"),
        issue(42, "Also loaded"),
        issue(1100, "Further beyond"),
    ];
    assert_eq!(numbers(&merge_searched(&loaded, searched)), [1200, 1100]);
    assert_eq!(
        merge_searched(&loaded, Vec::new()),
        [],
        "nothing found adds nothing"
    );
}

/// U70 (core) — a searched issue GitHub matched only in its body has nothing in its row for the
/// query, so ranking drops it (invariant 5, spec Clarifications).
#[test]
fn a_body_only_match_is_hidden() {
    let held = [
        issue(1200, "Crash when saving"),
        issue(1300, "Unrelated title"),
    ];
    let found = rank(&held, |i| i.row_text(), &Query::new("crash"));
    let shown: Vec<u64> = found.iter().map(|(i, _)| held[*i].number()).collect();
    assert_eq!(
        shown,
        [1200],
        "#1300 matched `crash` only in its body on GitHub's side"
    );
}

//! The worktree row's menu offers **Open pull request** only for a row with a pull request
//! (feature 040, contracts/pull-request-ui.md §4; story 2 scenarios 5 and 9; U115).

use micold_client::ui::worktree_menu_items;

fn labels(dir: &str, included: bool, claimable: bool, has_pull_request: bool) -> Vec<String> {
    worktree_menu_items(dir, "Feat a", included, claimable, has_pull_request)
        .into_iter()
        .map(|item| item.label)
        .collect()
}

/// U115 (A23): the entry sits directly above **Delete**, in every shape the menu takes.
#[test]
fn open_pull_request_is_directly_above_delete() {
    for (included, claimable) in [(false, false), (true, false), (false, true), (true, true)] {
        let items = labels("feat-a", included, claimable, true);
        let delete = items.iter().position(|l| l == "Delete").expect("Delete");
        assert_eq!(delete, items.len() - 1, "Delete stays last: {items:?}");
        assert_eq!(
            items[delete - 1],
            "Open pull request",
            "included={included} claimable={claimable}: {items:?}"
        );
        assert_eq!(
            items.iter().filter(|l| *l == "Open pull request").count(),
            1
        );
    }
}

/// U115 (story 2 scenario 9): a row without a pull request has today's menu, entry for entry.
#[test]
fn a_row_without_a_pull_request_has_todays_menu() {
    assert_eq!(
        labels("feat-a", false, false, false),
        ["Review changes", "Copy name", "Rename", "Delete"]
    );
    assert_eq!(
        labels("feat-a", true, true, false),
        [
            "Review changes",
            "Copy name",
            "Rename",
            "Stop showing",
            "Claim as mine",
            "Delete"
        ]
    );
    // And with one, the same entries plus the one.
    let mut with = labels("feat-a", true, true, true);
    with.retain(|l| l != "Open pull request");
    assert_eq!(with, labels("feat-a", true, true, false));
}

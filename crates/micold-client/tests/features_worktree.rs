//! Worktree visibility and naming, in isolation (feature 021, SC-004).
//!
//! An honest caveat about what "isolation" can mean in Tier 1: `visible_worktrees`,
//! `has_visible_worktrees` and `worktree_display_name` are `impl State` methods, and `State` is
//! still the monolith. So this file does build a `State` — it has no choice, and pretending
//! otherwise by testing something weaker would be worse.
//!
//! What it does hold to is the other half of SC-004: it names no other *feature's* types. No
//! sidebar rows, no settings draft, no project switcher — only the worktree fields it sets and the
//! worktree methods it calls. When Tier 3 splits `State`, this file should stop naming it at all,
//! and that is the diff to watch for.

use micold_client::app::State;
use micold_client::features::worktree;
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn worktree(dir_name: &str, branch: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.into(),
        path: PathBuf::from("/p/.claude/worktrees").join(dir_name),
        branch: Some(branch.into()),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// An assistant-owned worktree, by the naming convention feature 014 reserves for them.
fn agent_worktree() -> Worktree {
    worktree(
        "agent-0123456789abcdef0",
        "worktree-agent-0123456789abcdef0",
    )
}

fn with_worktrees(worktrees: Vec<Worktree>, reveal: bool) -> State {
    State {
        worktree: worktree::State {
            worktrees,
            ..Default::default()
        },
        show_agent_worktrees: reveal,
        ..Default::default()
    }
}

#[test]
fn an_assistant_owned_worktree_is_hidden_until_it_is_revealed() {
    let hidden = with_worktrees(vec![agent_worktree()], false);
    let shown = with_worktrees(vec![agent_worktree()], true);

    assert_eq!(
        hidden.visible_worktrees().count(),
        0,
        "the assistant's own scratch worktrees are noise in the user's sidebar by default"
    );
    assert_eq!(
        shown.visible_worktrees().count(),
        1,
        "revealing them shows them; hiding is a filter, not a deletion"
    );
}

#[test]
fn hiding_a_worktree_does_not_remove_it() {
    let st = with_worktrees(vec![agent_worktree()], false);

    assert_eq!(
        st.worktree.worktrees.len(),
        1,
        "visibility is a view concern — pruning, renaming and session lookup all reason about \
         existence, and a hidden worktree still exists"
    );
    assert!(
        !st.has_visible_worktrees(),
        "nothing visible, so the sidebar must say 'no worktrees yet' rather than offering to \
         clear a filter that is not set"
    );
}

#[test]
fn a_user_owned_worktree_is_visible_whether_or_not_reveal_is_on() {
    let mine = worktree("feat-thing", "feat/thing");

    for reveal in [false, true] {
        assert_eq!(
            with_worktrees(vec![mine.clone()], reveal)
                .visible_worktrees()
                .count(),
            1,
            "the reveal control adds the assistant's worktrees; it must never subtract the \
             user's own (reveal = {reveal})"
        );
    }
}

#[test]
fn a_worktree_with_no_rename_falls_back_to_a_name_derived_from_its_directory() {
    let st = with_worktrees(vec![worktree("feat-add-thing", "feat/add-thing")], false);

    let name = st.worktree_display_name("feat-add-thing");

    assert!(
        !name.is_empty() && name != "feat-add-thing",
        "an unrenamed worktree still reads as prose rather than as its directory name: got \
         {name:?}"
    );
}

#[test]
fn an_unknown_directory_still_yields_a_name_rather_than_failing() {
    let st = State::default();

    assert!(
        !st.worktree_display_name("feat-gone").is_empty(),
        "the display name is derived, not looked up, so a worktree that vanished between a \
         render and a click cannot blank out the row"
    );
}

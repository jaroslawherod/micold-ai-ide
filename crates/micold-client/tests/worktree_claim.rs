//! T063/T064 [US5] — claiming a worktree the app did not create (feature 029, FR-020–FR-024).
//!
//! The reducer half. A claim is a statement about authorship, not an operation: it writes the same
//! record a create writes, and it writes it *now*, because the row has to stop being an agent row
//! in the frame the user clicked. Waiting for the daemon's ack would leave the `agent` chip in
//! place for a round trip, and would make the row vanish under the cursor if the user switched the
//! reveal control off in the meantime.
//!
//! The last test here is about something that must **not** exist. FR-024 is the decision that a
//! claim has no inverse: nothing in this app hides a worktree the user owns, and a record leaves
//! the set only when its worktree is deleted or its project forgotten. That is easy to erode by
//! adding an obvious-looking "not mine" beside the claim, so it is asserted rather than described.

use micold_client::app::{Message, State};
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_core::naming::Tag;
use micold_core::project::{Availability, Project};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn project() -> PathBuf {
    PathBuf::from("/repo")
}

fn managed(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: project().join(".claude/worktrees").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// A project holding `worktrees`, of which the app recorded creating those in `recorded`, with the
/// reveal control on — the only state from which a claim is reachable, since a hidden worktree
/// draws no row to right-click.
fn revealed(worktrees: Vec<Worktree>, recorded: &[&str]) -> State {
    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: project(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(project());
    for dir in recorded {
        state.workspace.record_user_created(&project(), dir);
    }
    state.worktree.worktrees = worktrees;
    state.sidebar.show_agent_worktrees = true;
    state
}

fn claim(state: &mut State, dir: &str) {
    state.update(Message::Worktree(WorktreeMsg::ClaimRequested(
        dir.to_string(),
    )));
}

fn node_tags(state: &State, dir: &str) -> Vec<Tag> {
    state
        .worktree_tree()
        .into_iter()
        // By `dir_name`, the identity — `display_name` is the friendly prose derived from it
        // ("hand-made" renders as "Hand made"), so matching on it finds nothing.
        .find(|n| n.worktree.dir_name == dir)
        .unwrap_or_else(|| panic!("no row for {dir}"))
        .tags
}

// --- FR-022: the row is the user's in the same frame -------------------------------------------

#[test]
fn a_claim_records_the_worktree_immediately() {
    let mut state = revealed(
        vec![managed("feat-login"), managed("hand-made")],
        &["feat-login"],
    );
    assert!(!state.workspace.is_user_created(&project(), "hand-made"));

    claim(&mut state, "hand-made");

    assert!(
        state.workspace.is_user_created(&project(), "hand-made"),
        "the record is written by the reducer, not awaited from the daemon (FR-022)"
    );
}

#[test]
fn a_claimed_row_loses_the_agent_chip_in_the_same_frame() {
    let mut state = revealed(vec![managed("hand-made")], &[]);
    assert!(
        node_tags(&state, "hand-made").contains(&Tag::Agent),
        "precondition: revealed, unrecorded, so marked"
    );

    claim(&mut state, "hand-made");

    assert!(
        !node_tags(&state, "hand-made").contains(&Tag::Agent),
        "the chip reads off the same record the claim just wrote"
    );
}

#[test]
fn a_claimed_worktree_survives_switching_the_reveal_control_off() {
    let mut state = revealed(
        vec![managed("feat-login"), managed("hand-made")],
        &["feat-login"],
    );
    claim(&mut state, "hand-made");

    state.sidebar.show_agent_worktrees = false;

    let listed: Vec<String> = state
        .visible_worktrees()
        .map(|w| w.dir_name.clone())
        .collect();
    assert_eq!(
        listed,
        vec!["feat-login".to_string(), "hand-made".to_string()],
        "the claim is what keeps it listed; if it only survived while revealed it would vanish \
         under the cursor the moment the user switched the control off"
    );
}

#[test]
fn claiming_is_idempotent() {
    let mut state = revealed(vec![managed("hand-made")], &[]);
    claim(&mut state, "hand-made");
    let after_first = state.workspace.worktree_provenance.clone();

    claim(&mut state, "hand-made");

    assert_eq!(
        state.workspace.worktree_provenance, after_first,
        "claiming what is already claimed changes nothing (FR-022)"
    );
}

#[test]
fn a_reserved_convention_name_is_claimable_like_any_other() {
    // FR-023: the naming veto binds the automatic backfill, which guesses. It does not bind the
    // user, who is telling the app a fact about authorship — and a worktree the user made and
    // happened to name `agent-<hex>` is exactly the case the veto would otherwise make permanent.
    let mut state = revealed(vec![managed("agent-a885b42dc521fbda1")], &[]);

    claim(&mut state, "agent-a885b42dc521fbda1");

    assert!(state
        .workspace
        .is_user_created(&project(), "agent-a885b42dc521fbda1"));
    state.sidebar.show_agent_worktrees = false;
    assert_eq!(state.visible_worktrees().count(), 1, "and it stays listed");
}

// --- Sole mutation ------------------------------------------------------------------------------

#[test]
fn the_claim_closes_the_row_menu_and_touches_nothing_else() {
    let mut state = revealed(vec![managed("hand-made")], &[]);
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "hand-made".to_string(),
        (12, 34),
    )));
    assert!(
        state.worktree.menu_open.is_some(),
        "precondition: menu open"
    );
    let worktrees_before = state.worktree.worktrees.clone();
    let filters_before = state.sidebar.filters.clone();

    claim(&mut state, "hand-made");

    assert!(
        state.worktree.menu_open.is_none(),
        "the claim was chosen from the menu, so the menu closes — the courtesy every other menu \
         action does"
    );
    assert!(
        state.sidebar.show_agent_worktrees,
        "and the reveal control is left exactly as the user set it: a claim says who a worktree \
         belongs to, not what the sidebar should be showing"
    );
    assert_eq!(state.sidebar.filters, filters_before, "filters untouched");
    assert_eq!(
        state.worktree.worktrees, worktrees_before,
        "and the worktree list itself is the daemon's, unchanged by a record being written"
    );
}

#[test]
fn a_claim_without_an_active_project_is_a_no_op() {
    // Nothing can reach this in the app — a claim comes off a row of the active project's tree —
    // but the reducer must not panic or write under an absent key if it ever does.
    let mut state = revealed(vec![managed("hand-made")], &[]);
    state.workspace.active = None;

    claim(&mut state, "hand-made");

    assert!(
        state.workspace.worktree_provenance.is_empty(),
        "no project, no record to write"
    );
}

// --- FR-024: there is no inverse ---------------------------------------------------------------

#[test]
fn nothing_in_the_reducer_takes_a_record_back() {
    // The whole Msg surface, applied to a claimed worktree. None of it may un-claim: a worktree
    // leaves the user-owned set by being deleted or by its project being forgotten, and by nothing
    // else. An "un-claim" would also be unusable — the row it hid would disappear on the spot.
    let mut state = revealed(vec![managed("hand-made")], &[]);
    claim(&mut state, "hand-made");

    let messages = vec![
        WorktreeMsg::MenuToggled("hand-made".to_string(), (1, 1)),
        WorktreeMsg::MenuDismissed,
        WorktreeMsg::ClaimRequested("hand-made".to_string()),
        WorktreeMsg::Hovered("hand-made".to_string()),
        WorktreeMsg::Unhovered("hand-made".to_string()),
        WorktreeMsg::RenameStarted("hand-made".to_string()),
        WorktreeMsg::RenameTextChanged("Hand made".to_string()),
        WorktreeMsg::RenameConfirmed,
        WorktreeMsg::RenameCancelled,
        WorktreeMsg::DeleteRequested("hand-made".to_string()),
        WorktreeMsg::DeleteKeepBranchToggled(true),
        WorktreeMsg::DeleteCancelled,
        WorktreeMsg::TextCopyRequested("hand-made".to_string()),
        WorktreeMsg::Loaded(vec![managed("hand-made")]),
    ];
    for msg in messages {
        let label = format!("{msg:?}");
        state.update(Message::Worktree(msg));
        assert!(
            state.workspace.is_user_created(&project(), "hand-made"),
            "{label} took the record back; only a delete and a project forget may (FR-024)"
        );
    }
}

#[test]
fn there_is_no_un_claim_anywhere_to_reach() {
    // The reducer test above proves no *existing* message takes a record back. This one proves the
    // obvious addition was not made either: no protocol variant to send, and no menu entry to send
    // it from. Asserted against the sources because both would be one line, and a one-line addition
    // is exactly what a later reader adds when the reason for its absence is only in a comment.
    //
    // The reason: un-claiming hides the row the user is acting on, from a menu on that row — the
    // worktree would vanish mid-gesture. Records leave the set when the worktree is deleted or the
    // project forgotten, both of which are about the worktree ceasing to exist here, not ownership.
    let messages = include_str!("../../micold-core/src/protocol/messages.rs");
    assert!(
        !messages.contains("WorktreeUnclaim"),
        "FR-024: no un-claim message may exist"
    );
    let menu = include_str!("../src/ui/mod.rs");
    assert!(
        menu.contains("Claim as mine"),
        "sanity: the claim entry is the menu item this test is the counterpart of"
    );
    for wording in ["Not mine", "Unclaim", "Disown", "Release"] {
        assert!(
            !menu.contains(wording),
            "FR-024: no un-claim menu entry may exist, found {wording:?}"
        );
    }
}

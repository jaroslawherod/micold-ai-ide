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
//!
//! Feature 029 added one name to that list: `micold_core::project::Project`. Visibility is now
//! decided per project — a worktree is the user's because the app recorded creating it *in this
//! project* — so a fixture cannot ask "is this visible?" without saying which project is active.
//! That is a core type rather than another client feature's, which is the line the paragraph above
//! is drawing.

use micold_client::app::State;
use micold_client::features::sidebar;
use micold_client::features::worktree;
use micold_core::project::{Availability, Project};
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

/// A worktree the app has no record of creating — an assistant's own, in practice (029 FR-004).
/// Its `agent-`-shaped name is a leftover from 014 and decides nothing; `with_worktrees` withholding
/// the record is what makes it assistant-owned here.
fn agent_worktree() -> Worktree {
    worktree(
        "agent-0123456789abcdef0",
        "worktree-agent-0123456789abcdef0",
    )
}

/// A state holding `worktrees`, none of which the app recorded creating.
fn with_worktrees(worktrees: Vec<Worktree>, reveal: bool) -> State {
    with_records(worktrees, &[], reveal)
}

/// A state holding `worktrees`, of which the app recorded creating those named in `recorded`.
fn with_records(worktrees: Vec<Worktree>, recorded: &[&str], reveal: bool) -> State {
    let path = PathBuf::from("/p");
    let mut state = State {
        sidebar: sidebar::State {
            show_agent_worktrees: reveal,
            ..Default::default()
        },

        worktree: worktree::State {
            worktrees,
            ..Default::default()
        },
        ..Default::default()
    };
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "p".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    for dir in recorded {
        state.workspace.record_user_created(&path, dir);
    }
    state
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
            with_records(vec![mine.clone()], &["feat-thing"], reveal)
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
    let st = with_records(
        vec![worktree("feat-add-thing", "feat/add-thing")],
        &["feat-add-thing"],
        false,
    );

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

// --- Feature 029: asking for the listing to be re-read ----------------------------------------

/// Rule T6 of data-model §2.2, for the message US1 introduces.
///
/// There is no outcome to assert here, and that is the codebase's shape rather than an omission:
/// an effectful worktree message is routed a second time in `main.rs` to
/// `shell::daemon_sync::on_*`, exactly as `IncludeRequested`, `ExcludeRequested` and
/// `DeleteConfirmed` already are. The reducer's job for those is to leave the model alone, and
/// this asserts it does — the wire half is covered by the shell's own tests.
///
/// What T6 protects is US3: a refresh that has been *asked for* has not yet re-read anything, so
/// nothing about the arrangement on screen may move at the moment of the press. The listing only
/// changes when `set_worktrees` applies what the daemon actually found.
#[test]
fn asking_for_a_refresh_does_not_itself_touch_the_worktree_listing() {
    let mut st = with_worktrees(
        vec![worktree("feat-a", "feat/a"), worktree("feat-b", "feat/b")],
        false,
    );
    st.sidebar.expanded.insert("feat-a".to_string());
    let before = st.worktree.worktrees.clone();

    st.update(micold_client::app::Message::Worktree(
        worktree::Msg::RefreshRequested,
    ));

    assert_eq!(
        st.worktree.worktrees, before,
        "pressing refresh re-reads the repository; it does not guess at the answer. Emptying or \
         reordering the list here would flicker the sidebar on every press and would be a second \
         path to a listing, which 029 FR-004 exists to prevent"
    );
    assert!(
        st.sidebar.expanded.contains("feat-a"),
        "and nothing else the user arranged moves either (FR-009, US3)"
    );
}

// --- Feature 029, US2: the busy state, as a transition table ----------------------------------
//
// The six rules of data-model §2.2, one assertion each. They are here rather than in the shell
// because they are the whole of what the *model* does with a refresh: the flag, and nothing else.

/// Drive one message through the reducer and report the flag it left behind.
fn after(st: &mut State, msg: worktree::Msg) -> bool {
    st.update(micold_client::app::Message::Worktree(msg));
    st.worktree.refreshing
}

/// T1 and T3: the flag is raised by the ask and lowered by the answer.
#[test]
fn the_busy_flag_is_raised_by_the_request_and_lowered_by_its_outcome() {
    let mut st = State::default();

    assert!(
        after(&mut st, worktree::Msg::RefreshRequested),
        "the control must report in progress from the press, not from the reply — otherwise a \
         refresh on an unchanged project looks like a control that did nothing (029 FR-006)"
    );
    assert!(
        !after(&mut st, worktree::Msg::RefreshFinished),
        "and it returns to idle when the request reaches a terminal outcome, so it can be \
         pressed again (FR-007)"
    );
}

/// T2: single-flight. The view already withholds `on_press` while busy, so this is the second
/// line of defence — a structural guarantee that lives only in a render is one refactor away
/// from not existing.
#[test]
fn a_second_request_while_one_is_running_is_dropped() {
    let mut st = State::default();
    assert!(after(&mut st, worktree::Msg::RefreshRequested));

    assert!(
        after(&mut st, worktree::Msg::RefreshRequested),
        "the running refresh continues uninterrupted rather than being restarted or cancelled \
         (029 FR-006, US2 scenario 3)"
    );
}

/// T4: a reply for a refresh nobody started is a late reply, not an error.
#[test]
fn an_outcome_arriving_with_nothing_in_flight_changes_nothing() {
    let mut st = State::default();

    assert!(!after(&mut st, worktree::Msg::RefreshFinished));
    assert!(!after(&mut st, worktree::Msg::RefreshTimedOut(7)));
}

/// T5: the bounded wait is a third exit from busy, so a reply that never comes cannot strand the
/// control (029 FR-007, US2 scenario 5).
#[test]
fn the_bounded_wait_returns_the_control_to_idle() {
    let mut st = State::default();
    assert!(after(&mut st, worktree::Msg::RefreshRequested));

    assert!(!after(&mut st, worktree::Msg::RefreshTimedOut(1)));
}

/// T6, for all three messages at once: the flag is the *only* field any of them writes.
///
/// This is what FR-009 and US3 rest on. The refreshed listing changes the list, and it does so
/// through `set_worktrees`, whose reconciliation is already tested — none of these transitions may
/// reach it.
///
/// US3's T035 asks for exactly this and is answered here rather than by a second copy beside it.
/// The rule it names (T6) is one rule; the story it protects is a different one. `sidebar_state.rs`
/// holds the other half — that the listing which *does* arrive is reconciled by the shared path.
#[test]
fn no_refresh_transition_touches_the_listing_or_the_arrangement_around_it() {
    let mut st = with_worktrees(
        vec![worktree("feat-a", "feat/a"), worktree("feat-b", "feat/b")],
        false,
    );
    st.sidebar.expanded.insert("feat-a".to_string());
    st.worktree.hovered = Some("feat-b".to_string());
    let listing = st.worktree.worktrees.clone();

    for msg in [
        worktree::Msg::RefreshRequested,
        worktree::Msg::RefreshRequested,
        worktree::Msg::RefreshFinished,
        worktree::Msg::RefreshTimedOut(3),
    ] {
        st.update(micold_client::app::Message::Worktree(msg));
        assert_eq!(st.worktree.worktrees, listing, "the listing is untouched");
        assert!(
            st.sidebar.expanded.contains("feat-a"),
            "expansion is untouched"
        );
        assert_eq!(
            st.worktree.hovered.as_deref(),
            Some("feat-b"),
            "and so is everything else the user arranged"
        );
    }
}

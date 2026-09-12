//! T015 [US1] — what the sidebar lists, now that provenance decides it (feature 029, FR-004/FR-005).
//!
//! Feature 014 answered "is this the user's?" from the directory and branch names. That question
//! cannot be answered from a name: the assistant's *session* worktrees are created by
//! `claude --worktree <name>` and land in the very same `.claude/worktrees/` root under whatever
//! name the person typed. 029 inverts it — the app records the worktrees it creates, and the list
//! shows the ones it knows about.
//!
//! These are the three visibility rules and the three invariants 014 established, restated against
//! the record instead of against the name. The invariants are the load-bearing part: an inversion
//! that quietly changed *ordering*, or made revealing show something other than the full list, or
//! dropped a hidden worktree from state, would be a regression 014's own tests can no longer catch,
//! because the fixtures they were written against have moved.

use micold_client::app::State;
use micold_client::features::sidebar;
use micold_client::features::worktree;
use micold_core::project::{Availability, Project};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn project() -> PathBuf {
    PathBuf::from("/repo")
}

/// A worktree directly under the directory the app manages — the only kind the record is consulted
/// for.
fn managed(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: project().join(".claude/worktrees").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// A worktree elsewhere in the filesystem that the user asked this project to show (016 BUG-002).
fn elsewhere(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/somewhere/else").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: true,
    }
}

/// A project holding `worktrees`, of which the app recorded creating those named in `recorded`.
fn state(worktrees: Vec<Worktree>, recorded: &[&str], reveal: bool) -> State {
    let mut st = State {
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
    st.workspace.projects.push(Project {
        path: project(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    st.workspace.active = Some(project());
    for dir in recorded {
        st.workspace.record_user_created(&project(), dir);
    }
    st
}

fn listed(st: &State) -> Vec<String> {
    st.visible_worktrees().map(|w| w.dir_name.clone()).collect()
}

// ---------------------------------------------------------------------------
// The three visibility rules
// ---------------------------------------------------------------------------

#[test]
fn a_worktree_the_app_recorded_creating_is_listed() {
    let st = state(vec![managed("feat-login")], &["feat-login"], false);
    assert_eq!(listed(&st), vec!["feat-login"]);
}

#[test]
fn a_worktree_under_the_root_with_no_record_is_hidden_whatever_it_is_called() {
    // The case 014 could not see: an assistant session worktree under an ordinary, human-chosen
    // name. Nothing about "fix-the-parser" distinguishes it from the user's own work — only the
    // absence of a record does.
    let st = state(vec![managed("fix-the-parser")], &[], false);
    assert!(
        listed(&st).is_empty(),
        "an unrecorded worktree is not known to be the user's, got {:?}",
        listed(&st)
    );
}

#[test]
fn a_worktree_outside_the_managed_root_is_always_listed() {
    // It is there because the user asked for it, so the app never claimed to have made it and the
    // reveal control has never applied to it (014 FR-017, 016 BUG-002). No record, still shown.
    let st = state(vec![elsewhere("borrowed")], &[], false);
    assert_eq!(listed(&st), vec!["borrowed"]);
}

// ---------------------------------------------------------------------------
// 014's three invariants, restated
// ---------------------------------------------------------------------------

#[test]
fn revealing_yields_the_full_list_in_its_original_order() {
    // Invariant 1: the visible set is a subsequence of the discovered list, and revealing is
    // exactly the identity on it. Both directions matter — a reveal that reordered would move the
    // user's own rows around under them every time they looked at the assistant's.
    let all = vec![
        managed("aaa-mine"),
        managed("bbb-theirs"),
        managed("ccc-mine"),
        managed("ddd-theirs"),
    ];
    let order: Vec<String> = all.iter().map(|w| w.dir_name.clone()).collect();

    let hidden = state(all.clone(), &["aaa-mine", "ccc-mine"], false);
    assert_eq!(listed(&hidden), vec!["aaa-mine", "ccc-mine"]);

    let revealed = state(all, &["aaa-mine", "ccc-mine"], true);
    assert_eq!(listed(&revealed), order);
}

#[test]
fn hiding_removes_nothing_from_state() {
    // Invariant 3: hiding is a view concern. Pruning, rename overrides and session lookup all
    // reason about existence, and a hidden worktree exists.
    let st = state(
        vec![managed("feat-login"), managed("theirs")],
        &["feat-login"],
        false,
    );
    assert_eq!(st.worktree.worktrees.len(), 2);
    assert_eq!(listed(&st), vec!["feat-login"]);
}

#[test]
fn a_project_whose_worktrees_are_all_unrecorded_has_none_visible() {
    // Drives the sidebar's choice between "No worktrees yet" and "No worktrees match the filter":
    // with no filter set, offering to clear one would be nonsense (014 research R7).
    let st = state(vec![managed("one"), managed("two")], &[], false);
    assert!(!st.has_visible_worktrees());

    let with_one = state(vec![managed("one"), managed("two")], &["one"], false);
    assert!(with_one.has_visible_worktrees());
}

#[test]
fn recording_one_worktree_does_not_reveal_its_neighbours() {
    // The record is per directory name, not a per-project switch: adopting one worktree must not
    // drag the rest of the project's contents into the list with it.
    let st = state(
        vec![managed("one"), managed("two"), managed("three")],
        &["two"],
        false,
    );
    assert_eq!(listed(&st), vec!["two"]);
}

#[test]
fn provenance_is_read_from_the_active_project() {
    // A record written for one project must not decide anything in another — the record set is
    // keyed by project path, and a lookup against the wrong key would silently show or hide the
    // wrong rows after a project switch.
    let mut st = state(vec![managed("feat-login")], &[], false);
    st.workspace
        .record_user_created(&PathBuf::from("/other-repo"), "feat-login");
    assert!(
        listed(&st).is_empty(),
        "another project's record must not count here, got {:?}",
        listed(&st)
    );
}

// ---------------------------------------------------------------------------
// T028 [US2] — a worktree the app just made is listed in the frame it appears
// ---------------------------------------------------------------------------

#[test]
fn a_worktree_the_app_just_created_is_listed_immediately() {
    // FR-010. The daemon writes the durable record and re-broadcasts, but this reducer runs first
    // and cannot wait for it. Without recording here the row would arrive unrecorded, classify as
    // the app's own, and be hidden for the frames in between — the user would watch the worktree
    // they just asked for flicker out of the list.
    let mut st = state(vec![], &[], false);

    worktree::created(&mut st, managed("feat-login"));

    assert_eq!(listed(&st), vec!["feat-login"]);
}

#[test]
fn an_app_created_worktree_stays_listed_when_the_durable_write_fails() {
    // The optimistic record lives in client state, so a storage fault daemon-side costs the user
    // nothing for this run: they made it, so they can see it. The client never learns whether the
    // write succeeded, which is precisely why this holds — there is no failure path to handle.
    let mut st = state(vec![], &[], false);

    worktree::created(&mut st, managed("feat-login"));

    assert!(
        st.workspace.is_user_created(&project(), "feat-login"),
        "the record is in client state regardless of what the disk did"
    );
    assert_eq!(listed(&st), vec!["feat-login"]);
}

#[test]
fn creating_one_worktree_does_not_reveal_the_ones_already_hidden() {
    // The optimism is scoped to the directory the create names. An assistant's worktree sitting
    // beside it is untouched, so "I made this one" never becomes "I made everything here".
    let mut st = state(vec![managed("tidy-the-parser")], &[], false);
    assert!(listed(&st).is_empty());

    worktree::created(&mut st, managed("feat-login"));

    assert_eq!(listed(&st), vec!["feat-login"]);
}

#[test]
fn the_optimistic_record_is_keyed_on_the_active_project() {
    // `created` writes into the active project's record set, which is the same set
    // `provenance_view` reads — so the two halves cannot drift apart. Asserted directly because
    // the reducer's own path to the project (`workspace.active_project()`) is the only thing
    // standing between "recorded" and "recorded somewhere nobody looks".
    let mut st = state(vec![], &[], false);

    worktree::created(&mut st, managed("feat-login"));

    assert!(st.workspace.is_user_created(&project(), "feat-login"));
    assert!(
        !st.workspace
            .is_user_created(&PathBuf::from("/other"), "feat-login"),
        "the record belongs to one project, not to every project"
    );
}

// ---------------------------------------------------------------------------
// T043/T046 [US3] — the hidden count, and what is never classified at all
// ---------------------------------------------------------------------------

#[test]
fn nothing_is_hidden_when_every_worktree_is_recorded() {
    let st = state(
        vec![managed("feat-a"), managed("fix-b")],
        &["feat-a", "fix-b"],
        false,
    );
    assert_eq!(st.hidden_worktree_count(), 0);
}

#[test]
fn the_count_is_the_number_of_unrecorded_worktrees() {
    let st = state(
        vec![
            managed("feat-a"),
            managed("tidy-the-parser"),
            managed("try-the-new-api"),
        ],
        &["feat-a"],
        false,
    );
    assert_eq!(st.hidden_worktree_count(), 2);
}

#[test]
fn the_count_is_zero_while_the_control_is_on() {
    // FR-025a. The number answers "what is this switch withholding?", and a switch that is on is
    // withholding nothing — showing `· 2` beside a control that is already showing those two would
    // read as a promise of two more.
    let st = state(
        vec![managed("feat-a"), managed("tidy-the-parser")],
        &["feat-a"],
        true,
    );
    assert_eq!(st.hidden_worktree_count(), 0);
}

#[test]
fn switching_the_control_on_adds_exactly_the_counted_rows() {
    // FR-025b, the promise the number makes, asserted end to end rather than by trusting that two
    // filters agree. This is why the count is a subtraction: it cannot be wrong about the list it
    // is derived from.
    let worktrees = vec![
        managed("feat-a"),
        managed("tidy-the-parser"),
        managed("try-the-new-api"),
        elsewhere("vendored"),
    ];
    let hidden_before = state(worktrees.clone(), &["feat-a"], false);
    let count = hidden_before.hidden_worktree_count();
    let revealed = state(worktrees, &["feat-a"], true);

    assert_eq!(
        listed(&revealed).len() - listed(&hidden_before).len(),
        count,
        "switching the control on must add exactly the number it advertised"
    );
}

#[test]
fn a_worktree_outside_the_managed_root_is_never_counted_as_hidden() {
    // FR-005: nothing outside the directory this app manages is classified at all, so an included
    // worktree can never be part of what the reveal control is withholding.
    let st = state(vec![elsewhere("vendored")], &[], false);
    assert_eq!(st.hidden_worktree_count(), 0);
    assert_eq!(listed(&st), vec!["vendored"]);
}

#[test]
fn a_project_hiding_everything_reports_no_visible_worktrees() {
    // FR-017 and research R7: the sidebar chooses "No worktrees yet" over "No worktrees match the
    // filter" from `has_visible_worktrees`, because offering to clear a filter nobody applied is
    // nonsense. Provenance widens the hidden set, so this branch matters more than it did.
    let st = state(
        vec![managed("tidy-the-parser"), managed("try-the-new-api")],
        &[],
        false,
    );
    assert!(!st.has_visible_worktrees());
    assert_eq!(st.hidden_worktree_count(), 2);
}

#[test]
fn the_project_root_is_not_a_worktree_and_is_never_hidden() {
    // FR-017. "Default" is the project root, not an entry in this list — it is added by
    // `sidebar_entries`, never by `visible_worktrees` — so no record can exist for it and no
    // classification can take it away. A project whose every worktree is hidden still offers
    // somewhere to start a session.
    let st = state(vec![managed("tidy-the-parser")], &[], false);
    assert!(listed(&st).is_empty());
    assert!(
        !st.sidebar_entries().is_empty(),
        "the Default entry survives however the worktrees classify"
    );
}

// --- T055 [US4]: refreshing the list prunes by existence, never by visibility -------------------

#[test]
fn a_refresh_does_not_drop_the_record_of_a_worktree_it_is_hiding() {
    // `feat-login` is recorded and listed; `agent-work` is not recorded, so it is hidden. Both
    // exist on disk, and a refresh delivers both.
    let mut st = state(
        vec![managed("feat-login"), managed("agent-work")],
        &["feat-login"],
        false,
    );
    st.workspace.record_user_created(&project(), "agent-work");
    // Now both are recorded, but the reveal is off and nothing is hidden — so hide one by
    // withdrawing its record, the way the app's own state would have it after a migration.
    st.workspace.forget_user_created(&project(), "agent-work");
    assert_eq!(
        st.visible_worktrees().count(),
        1,
        "precondition: one row hidden"
    );

    let _ = st.set_worktrees(vec![managed("feat-login"), managed("agent-work")]);

    assert!(
        st.workspace.is_user_created(&project(), "feat-login"),
        "the visible worktree keeps its record across a refresh"
    );
    assert_eq!(
        st.visible_worktrees().count(),
        1,
        "and the hidden one is still hidden rather than promoted by the refresh"
    );
}

#[test]
fn pruning_keys_on_existence_not_on_what_the_list_is_showing() {
    // Exactly 014's requirement of the rename override, restated for the record: a worktree the
    // sidebar is not drawing is still a worktree, and a refresh that delivers it must leave its
    // record alone. A prune written against `visible_worktrees()` instead of against the delivered
    // set would delete the record of every hidden worktree on the first refresh — and since the
    // record is what makes a worktree visible, the damage would be invisible and permanent.
    let mut st = state(
        vec![managed("feat-login"), managed("feat-search")],
        &["feat-login", "feat-search"],
        false,
    );
    // Hide `feat-search` from the sidebar by filtering, not by un-recording it.
    let _ = st.set_worktrees(vec![managed("feat-login"), managed("feat-search")]);

    assert!(
        st.workspace.is_user_created(&project(), "feat-search"),
        "still recorded after the refresh"
    );

    // A worktree that genuinely went away is a different question, and the record survives that
    // too: only an in-app delete forgets it (FR-009), because only a delete knows the directory
    // is gone for good rather than merely absent from one discovery run.
    let _ = st.set_worktrees(vec![managed("feat-login")]);
    assert!(
        st.workspace.is_user_created(&project(), "feat-search"),
        "a worktree missing from one refresh is not evidence it was deleted; the delete path is \
         the one writer that drops a record"
    );
}

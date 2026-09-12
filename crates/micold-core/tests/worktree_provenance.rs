//! T013/T029 (US1, US2) — classification from the record rather than from the name (feature 029,
//! FR-004/FR-011).
//!
//! The normative table lives in
//! `specs/029-worktree-provenance/contracts/worktree-classification.md` §5; every row there is a
//! case here, and §4's invariants are asserted at the bottom.
//!
//! The inversion in one sentence: feature 014 asked the directory and branch names who made a
//! worktree, which is a question names cannot answer — the assistant's *session* worktrees land in
//! the same directory under ordinary, human-chosen names. So the app now records what it creates,
//! and everything else under the directory it manages is, by construction, not known to be the
//! user's.

use micold_core::worktree::{
    classify_owner, ProvenanceView, Worktree, WorktreeOwner, WorktreeStatus,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// A worktree as `reconcile()` produces it for one directly under the managed root — the case the
/// record set is consulted for.
fn managed(dir_name: &str, branch: Option<&str>) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/repo/.claude/worktrees").join(dir_name),
        branch: branch.map(str::to_string),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// A worktree the repository knows about that lives somewhere else, which the user asked to see
/// (016 BUG-002). `reconcile` sets `included` for exactly these.
fn included(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/elsewhere").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: true,
    }
}

fn records(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// §5 — the classification table
// ---------------------------------------------------------------------------

#[test]
fn a_recorded_worktree_under_the_root_is_the_users() {
    let set = records(&["feat-login"]);
    let view = ProvenanceView::new(&set);
    assert_eq!(
        classify_owner(&managed("feat-login", Some("feat/login")), &view),
        WorktreeOwner::User
    );
}

#[test]
fn an_unrecorded_worktree_under_the_root_is_not_known_to_be_the_users() {
    // The whole point of the feature: this is an assistant session worktree, and nothing about
    // "fix-the-parser" says so.
    let set = records(&["feat-login"]);
    let view = ProvenanceView::new(&set);
    assert_eq!(
        classify_owner(&managed("fix-the-parser", Some("fix/the-parser")), &view),
        WorktreeOwner::Agent
    );
}

#[test]
fn an_included_worktree_is_always_the_users_even_with_no_records_at_all() {
    // It lives outside the directory this app manages, so the app never claimed to have made it
    // and the reveal control has never applied to it (014 FR-017, 016 BUG-002).
    let view = ProvenanceView::none();
    assert_eq!(
        classify_owner(&included("some-other-checkout"), &view),
        WorktreeOwner::User
    );
}

#[test]
fn an_included_worktree_is_the_users_even_when_its_name_is_the_reserved_convention() {
    let view = ProvenanceView::none();
    let mut w = included("agent-a885b42dc521fbda1");
    w.branch = Some("worktree-agent-a885b42dc521fbda1".to_string());
    assert_eq!(classify_owner(&w, &view), WorktreeOwner::User);
}

#[test]
fn with_no_records_every_managed_worktree_is_hidden() {
    // Honest and deliberate: before US2 writes anything, the app genuinely knows of none. This is
    // why US1 must not ship without US2.
    let view = ProvenanceView::none();
    for name in ["feat-login", "agent-a885b42dc521fbda1", "scratch"] {
        assert_eq!(
            classify_owner(&managed(name, None), &view),
            WorktreeOwner::Agent,
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------
// §4 invariant 6 — name-blindness
// ---------------------------------------------------------------------------

#[test]
fn two_worktrees_alike_but_for_a_reserved_name_classify_by_their_record() {
    // The sharpest statement of the inversion, in both directions: a reserved-convention name with
    // a record is the user's (FR-007b), and an ordinary name without one is not.
    let set = records(&["agent-a885b42dc521fbda1"]);
    let view = ProvenanceView::new(&set);

    let reserved = managed(
        "agent-a885b42dc521fbda1",
        Some("worktree-agent-a885b42dc521fbda1"),
    );
    let ordinary = managed("feat-login", Some("feat/login"));

    assert_eq!(classify_owner(&reserved, &view), WorktreeOwner::User);
    assert_eq!(classify_owner(&ordinary, &view), WorktreeOwner::Agent);
}

#[test]
fn the_branch_never_enters_the_decision() {
    // 014 read the branch as a second naming signal. It is not read at all now — same directory
    // name, every branch shape, one answer.
    let set = records(&["feat-login"]);
    let view = ProvenanceView::new(&set);
    for branch in [
        None,
        Some("feat/login"),
        Some("worktree-agent-a885b42dc521fbda1"),
        Some("main"),
    ] {
        assert_eq!(
            classify_owner(&managed("feat-login", branch), &view),
            WorktreeOwner::User,
            "{branch:?}"
        );
    }
}

#[test]
fn classification_is_health_blind() {
    // A broken worktree is still the user's, and still needs its row to be repaired or removed.
    let set = records(&["feat-login"]);
    let view = ProvenanceView::new(&set);
    for status in [
        WorktreeStatus::Valid,
        WorktreeStatus::Missing,
        WorktreeStatus::Invalid,
    ] {
        let mut w = managed("feat-login", Some("feat/login"));
        w.status = status;
        assert_eq!(classify_owner(&w, &view), WorktreeOwner::User, "{status:?}");
    }
}

#[test]
fn the_record_is_matched_on_the_directory_name_exactly() {
    // `dir_name` is the key the record is stored under, not a pattern. No prefix, no case folding.
    let set = records(&["feat-login"]);
    let view = ProvenanceView::new(&set);
    for name in ["feat-login-2", "feat-logi", "Feat-Login"] {
        assert_eq!(
            classify_owner(&managed(name, None), &view),
            WorktreeOwner::Agent,
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------
// T029 (US2) — FR-011, a failed read fails visible
// ---------------------------------------------------------------------------

#[test]
fn an_unreadable_project_shows_every_worktree_it_has() {
    // The hazard the `state_unreadable` flag exists for: a project whose state file did not parse
    // arrives with an empty record set, which is indistinguishable from "this app created none of
    // these" unless the two are kept apart. Conflating them would hide every worktree the user has
    // because of a storage fault.
    let view = ProvenanceView::unreadable();
    for name in ["feat-login", "agent-a885b42dc521fbda1", "scratch"] {
        assert_eq!(
            classify_owner(&managed(name, None), &view),
            WorktreeOwner::User,
            "{name}"
        );
    }
}

#[test]
fn unreadable_outranks_a_non_empty_record_set() {
    // Checked first, so a partially-recovered record set cannot hide the worktrees missing from it.
    let set = records(&["feat-login"]);
    let view = ProvenanceView {
        records: &set,
        state_unreadable: true,
    };
    assert_eq!(
        classify_owner(&managed("something-else", None), &view),
        WorktreeOwner::User
    );
}

#[test]
fn classification_is_stateless() {
    // No caching: the same worktree classified before and after the record appears answers
    // differently, in that order, with no invalidation step in between (FR-013).
    let w = managed("feat-login", Some("feat/login"));
    let empty = BTreeSet::new();
    assert_eq!(
        classify_owner(&w, &ProvenanceView::new(&empty)),
        WorktreeOwner::Agent
    );
    let set = records(&["feat-login"]);
    assert_eq!(
        classify_owner(&w, &ProvenanceView::new(&set)),
        WorktreeOwner::User
    );
}

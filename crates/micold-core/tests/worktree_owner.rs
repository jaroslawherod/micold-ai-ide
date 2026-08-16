//! T004/T010 — agent-worktree classification (feature 014, FR-001/FR-005/FR-006).
//!
//! The normative truth table lives in
//! `specs/014-hide-agent-worktrees/contracts/agent-worktree-classification.md`; every row there is
//! a case here. Positive rows (T004, US1) prove agent worktrees are recognized; negative and
//! boundary rows (T010, US2) prove nothing else is — the guard that keeps a user's own
//! `agent-*` worktree visible.

use micold_core::worktree::{Worktree, WorktreeOwner, WorktreeStatus};
use std::path::PathBuf;

/// A worktree as `reconcile()` would produce it: directly under the project's worktrees root.
/// The location half of FR-005 is therefore already satisfied; only naming is under test.
fn wt(dir_name: &str, branch: Option<&str>) -> Worktree {
    wt_with_status(dir_name, branch, WorktreeStatus::Valid)
}

fn wt_with_status(dir_name: &str, branch: Option<&str>, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/repo/.claude/worktrees").join(dir_name),
        branch: branch.map(str::to_string),
        status,
        included: false,
    }
}

// ---------------------------------------------------------------------------
// T004 (US1) — positive rows: real agent worktrees are recognized
// ---------------------------------------------------------------------------

#[test]
fn both_identifiers_matching_is_agent() {
    // The real-world case, as Claude Code creates it.
    let w = wt(
        "agent-a885b42dc521fbda1",
        Some("worktree-agent-a885b42dc521fbda1"),
    );
    assert_eq!(w.owner(), WorktreeOwner::Agent);
    assert!(w.is_agent_owned());
}

#[test]
fn directory_alone_is_enough() {
    // Orphan directory git no longer registers, or a detached worktree: no branch to consult.
    let w = wt("agent-abf6a58b16c3c9e6f", None);
    assert_eq!(w.owner(), WorktreeOwner::Agent);
}

#[test]
fn branch_alone_is_enough() {
    // Directory renamed or otherwise unrecognizable; the branch still carries the convention.
    let w = wt("unrelated-dir", Some("worktree-agent-ae474105b29fbeb68"));
    assert_eq!(w.owner(), WorktreeOwner::Agent);
}

#[test]
fn either_identifier_suffices_when_they_disagree() {
    // "Name/branch mismatch" edge case: OR, not AND.
    let w = wt("agent-a885b42dc521fbda1", Some("feat/real-work"));
    assert_eq!(w.owner(), WorktreeOwner::Agent);
}

// ---------------------------------------------------------------------------
// T010 (US2) — negative and boundary rows: nothing else is caught
// ---------------------------------------------------------------------------

#[test]
fn ordinary_words_after_the_prefix_stay_user_owned() {
    // FR-006, the case from the original request: `agent-foo` is a user's worktree. It fails twice
    // over — too short, and `o` is not a hex digit.
    assert_eq!(
        wt("agent-foo", Some("agent/foo")).owner(),
        WorktreeOwner::User
    );
}

#[test]
fn short_all_hex_tail_is_not_enough() {
    // "face" is valid hex but only 4 characters — well under the 16-character floor.
    assert_eq!(wt("agent-face", None).owner(), WorktreeOwner::User);
}

#[test]
fn long_enough_but_not_all_hex_stays_user_owned() {
    // The whole remainder must be hex; a real branch name with a hex-looking head is still a real
    // branch name.
    assert_eq!(
        wt("agent-deadbeefdeadbeef-parser", None).owner(),
        WorktreeOwner::User
    );
}

#[test]
fn the_sixteen_character_boundary_is_inclusive() {
    // Exactly 16 hex digits ⇒ Agent; 15 ⇒ User. These two rows are what pin the rule against a
    // later "simplification" back to bare prefix matching.
    assert_eq!(
        wt("agent-deadbeefdeadbeef", None).owner(),
        WorktreeOwner::Agent,
        "16 hex digits is the inclusive lower bound"
    );
    assert_eq!(
        wt("agent-deadbeefdeadbee", None).owner(),
        WorktreeOwner::User,
        "15 hex digits is one below the bound"
    );
}

#[test]
fn the_reserved_word_in_the_middle_is_not_a_match() {
    let w = wt("feat-1234-agent-runner", Some("feat/1234-agent-runner"));
    assert_eq!(w.owner(), WorktreeOwner::User);
}

#[test]
fn empty_identifier_is_not_a_match() {
    assert_eq!(wt("agent-", None).owner(), WorktreeOwner::User);
    assert_eq!(
        wt("some-dir", Some("worktree-agent-")).owner(),
        WorktreeOwner::User
    );
}

#[test]
fn hex_case_is_accepted_but_the_prefix_is_case_sensitive() {
    assert_eq!(
        wt("agent-A885B42DC521FBDA1", None).owner(),
        WorktreeOwner::Agent,
        "uppercase hex digits are still hex digits"
    );
    assert_eq!(
        wt("AGENT-a885b42dc521fbda1", None).owner(),
        WorktreeOwner::User,
        "the reserved prefix is matched case-sensitively"
    );
}

#[test]
fn the_branch_prefix_does_not_match_in_the_directory_position() {
    // Each prefix belongs to its own field; they are not interchangeable.
    assert_eq!(
        wt("worktree-agent-a885b42dc521fbda1", None).owner(),
        WorktreeOwner::User
    );
}

#[test]
fn a_plain_user_worktree_is_user_owned() {
    assert_eq!(
        wt("feat-abc-123-login", Some("feat/abc-123-login")).owner(),
        WorktreeOwner::User
    );
}

#[test]
fn classification_is_health_blind() {
    // FR-007: a Missing or Invalid agent worktree is still an agent worktree, so it is hidden
    // rather than surfacing as a broken entry.
    for status in [
        WorktreeStatus::Valid,
        WorktreeStatus::Missing,
        WorktreeStatus::Invalid,
    ] {
        let w = wt_with_status("agent-a885b42dc521fbda1", None, status);
        assert_eq!(
            w.owner(),
            WorktreeOwner::Agent,
            "status {status:?} must not change classification"
        );
    }
}

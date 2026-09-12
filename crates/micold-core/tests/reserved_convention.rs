//! T014 (US1) — the reserved machine-naming convention (feature 029, FR-007a).
//!
//! This is feature 014's fourteen-row truth table, carried over intact from the deleted
//! `worktree_owner.rs`. What changed is not a single expected value but what the answer *means*:
//! 014 asked "is this worktree the assistant's?" and hid the ones that said yes. 029 asks only
//! "is this name reserved for machine-generated worktrees?", and the sole consumer is the one-time
//! migration, which refuses to grandfather a match on the strength of a session found inside it.
//!
//! The rows are preserved rather than rewritten because the convention itself has not moved: the
//! boundary between a real assistant worktree and a user's own `agent-*` folder is still exactly
//! where 014 put it, and the migration veto is only as safe as that boundary is tight. A row lost
//! here is a worktree the migration would wrongly adopt or wrongly refuse.

use micold_core::worktree::matches_reserved_convention;

// ---------------------------------------------------------------------------
// Positive rows: the convention as Claude Code actually writes it
// ---------------------------------------------------------------------------

#[test]
fn both_identifiers_matching_is_reserved() {
    assert!(matches_reserved_convention(
        "agent-a885b42dc521fbda1",
        Some("worktree-agent-a885b42dc521fbda1")
    ));
}

#[test]
fn the_directory_alone_is_enough() {
    // The worktree git no longer registers: the folder survives, the branch does not.
    assert!(matches_reserved_convention("agent-a885b42dc521fbda1", None));
}

#[test]
fn the_branch_alone_is_enough() {
    // The folder was renamed by hand; the branch still carries the convention.
    assert!(matches_reserved_convention(
        "my-renamed-folder",
        Some("worktree-agent-a885b42dc521fbda1")
    ));
}

#[test]
fn a_detached_worktree_is_decided_by_its_directory() {
    assert!(matches_reserved_convention("agent-deadbeefcafebabe", None));
}

// ---------------------------------------------------------------------------
// Negative and boundary rows: the guard that keeps the convention tight
// ---------------------------------------------------------------------------

#[test]
fn a_branch_named_agent_something_is_not_the_convention() {
    // `agent/foo` is not `worktree-agent-<id>`, and `agent-foo` has no hex id.
    assert!(!matches_reserved_convention("agent-foo", Some("agent/foo")));
}

#[test]
fn a_short_hex_suffix_is_not_an_id() {
    assert!(!matches_reserved_convention("agent-face", None));
}

#[test]
fn an_id_with_a_human_suffix_is_not_an_id() {
    // The whole suffix must be the id: `agent-<id>-parser` is a person's folder.
    assert!(!matches_reserved_convention(
        "agent-deadbeefdeadbeef-parser",
        None
    ));
}

#[test]
fn sixteen_hex_digits_match_and_fifteen_do_not() {
    // The boundary pair. One character apart, opposite answers.
    assert!(matches_reserved_convention("agent-deadbeefdeadbeef", None));
    assert!(!matches_reserved_convention("agent-deadbeefdeadbee", None));
}

#[test]
fn uppercase_hex_matches_but_an_uppercase_prefix_does_not() {
    // The case-sensitivity pair: the id is hex (either case), the prefix is a literal.
    assert!(matches_reserved_convention("agent-A885B42DC521FBDA1", None));
    assert!(!matches_reserved_convention(
        "AGENT-a885b42dc521fbda1",
        None
    ));
}

#[test]
fn a_bare_prefix_with_no_id_is_not_the_convention() {
    assert!(!matches_reserved_convention("agent-", None));
    assert!(!matches_reserved_convention(
        "some-dir",
        Some("worktree-agent-")
    ));
}

#[test]
fn the_directory_prefix_is_not_the_branch_prefix() {
    // `worktree-agent-<id>` as a *directory* name is not the directory convention, which is
    // `agent-<id>`; the two prefixes are not interchangeable.
    assert!(!matches_reserved_convention(
        "worktree-agent-a885b42dc521fbda1",
        None
    ));
}

#[test]
fn an_ordinary_worktree_is_not_reserved() {
    assert!(!matches_reserved_convention(
        "feat-abc-123-login",
        Some("feat/abc-123-login")
    ));
}

#[test]
fn a_non_hex_character_disqualifies_an_otherwise_long_suffix() {
    // Length alone is not the rule; every character must be hex.
    assert!(!matches_reserved_convention("agent-deadbeefdeadbeeg", None));
}

#[test]
fn the_convention_says_nothing_about_who_the_worktree_belongs_to() {
    // The point of the rename. A user is free to name a folder this way, and if the app recorded
    // creating it the record wins (FR-007b) — this predicate is not consulted at all outside the
    // migration. Asserted here so the file cannot quietly drift back into being a hiding signal.
    assert!(matches_reserved_convention("agent-a885b42dc521fbda1", None));
}

//! T030/T031 (US2) — the one-time evidence backfill (feature 029, FR-006/FR-006a–d, FR-007a).
//!
//! Two pure functions carry the whole migration, and this file is their specification:
//!
//! - [`durably_known_worktrees`] answers "which worktrees did the user demonstrably work in before
//!   this feature existed?" from the only evidence the app already holds — a stored display-name
//!   override, and a session bound to a worktree. Nothing is read from disk to answer it.
//! - [`plan_backfill`] turns that evidence into the exact set of records to write, and says `None`
//!   when the migration must not run at all.
//!
//! The contract is `specs/029-worktree-provenance/contracts/provenance-store.md` §4. Every
//! invariant listed there is a test below.
//!
//! Why evidence at all: the inversion means a project upgrading to 029 starts with no records, so
//! without a backfill every worktree the user ever made through the app would vanish from the list
//! at once. The backfill grandfathers the ones the user can prove they used — and runs exactly
//! once per project, because a standing rule would let 014's "start a session in a revealed agent
//! worktree" quietly un-hide it forever (FR-006d).

use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{
    durably_known_worktrees, plan_backfill, session_worktree_dirs, Worktree, WorktreeStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const ROOT: &str = "/repo/.claude/worktrees";

fn managed(dir_name: &str, branch: Option<&str>) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from(ROOT).join(dir_name),
        branch: branch.map(str::to_string),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// A worktree the user asked to see that lives outside the managed root (016 BUG-002).
fn elsewhere(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/somewhere/else").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: true,
    }
}

fn overrides(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn session_in(dir: &str) -> Session {
    Session::start_new(
        SessionLocation::Worktree(dir.to_string()),
        AiCli::ClaudeCode,
    )
}

fn default_session() -> Session {
    Session::start_new(SessionLocation::Default, AiCli::ClaudeCode)
}

fn root() -> &'static Path {
    Path::new(ROOT)
}

// ---------------------------------------------------------------------------
// T030 — durably_known_worktrees: what counts as evidence
// ---------------------------------------------------------------------------

#[test]
fn a_display_name_override_counts_as_evidence() {
    // Renaming a worktree is something only the user can have done through this app.
    let map = overrides(&[("feat-login", "Login work")]);
    let known = durably_known_worktrees(Some(&map), std::iter::empty());
    assert_eq!(known, names(&["feat-login"]));
}

#[test]
fn a_session_bound_to_a_worktree_counts_as_evidence() {
    let sessions = vec![session_in("feat-login")];
    let known = durably_known_worktrees(None, session_worktree_dirs(&sessions));
    assert_eq!(known, names(&["feat-login"]));
}

#[test]
fn an_archived_session_still_counts_as_evidence() {
    // The sidebar never shows an archived session again (BUG-003), but the user did work there,
    // and this question is about history, not about what is on screen. `session_worktree_dirs`
    // deliberately does not filter — the snapshot's own `!archived` filter stays at its own call
    // site, where it is about display.
    let mut session = session_in("feat-shipped");
    session.archive();
    let sessions = vec![session];
    let known = durably_known_worktrees(None, session_worktree_dirs(&sessions));
    assert_eq!(known, names(&["feat-shipped"]));
}

#[test]
fn a_default_location_session_contributes_nothing() {
    // A "Default" session runs in the project root, which is not a worktree at all.
    let sessions = vec![default_session()];
    let known = durably_known_worktrees(None, session_worktree_dirs(&sessions));
    assert!(known.is_empty(), "got {known:?}");
}

#[test]
fn the_two_evidence_sources_are_unioned_and_deduplicated() {
    let map = overrides(&[("feat-login", "Login work"), ("fix-crash", "Crash")]);
    let sessions = vec![session_in("feat-login"), session_in("spike")];
    let known = durably_known_worktrees(Some(&map), session_worktree_dirs(&sessions));
    assert_eq!(known, names(&["feat-login", "fix-crash", "spike"]));
}

#[test]
fn a_project_with_no_history_yields_no_evidence() {
    assert!(durably_known_worktrees(None, std::iter::empty()).is_empty());
}

// ---------------------------------------------------------------------------
// T031 — plan_backfill: what the migration writes
// ---------------------------------------------------------------------------

#[test]
fn a_worktree_with_evidence_is_backfilled() {
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(names(&["feat-login"])));
}

#[test]
fn a_worktree_without_evidence_is_not_backfilled() {
    // The assistant's session worktrees are exactly this case: real, under the managed root, and
    // never renamed or opened as a session by the user. This is the hiding the feature exists for.
    let discovered = vec![
        managed("feat-login", Some("feat/login")),
        managed("tidy-up-the-parser", Some("tidy/parser")),
    ];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(names(&["feat-login"])));
}

#[test]
fn the_reserved_naming_convention_vetoes_a_backfill() {
    // FR-007a, the one surviving use of 014's rule: a directory named the way this app names its
    // machine-made scratch worktrees is not grandfathered, even with evidence behind it. 014
    // permitted starting a session in a revealed agent worktree, so a session there is not proof
    // the user made it.
    let discovered = vec![managed("agent-deadbeefdeadbeef", Some("feat/x"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["agent-deadbeefdeadbeef"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn the_veto_also_reads_the_branch() {
    // 014's rule had two halves; both veto. `worktree-agent-<hex>` on the branch is the other.
    let discovered = vec![managed(
        "ordinary-name",
        Some("worktree-agent-0123456789abcdef"),
    )];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["ordinary-name"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn a_worktree_outside_the_managed_root_is_not_backfilled() {
    // Nothing outside the root is ever classified from records (FR-005), so a record for it would
    // be dead weight that could only mislead a later reader.
    let discovered = vec![elsewhere("vendored-checkout")];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["vendored-checkout"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn an_existing_record_is_never_overwritten() {
    // FR-006b. The plan is what to *add*; a worktree already recorded contributes nothing, which
    // is what makes a second run over the same inputs empty.
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login"]),
        &names(&["feat-login"]),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn applying_the_plan_makes_the_next_run_empty() {
    // FR-006b stated as the property that matters: a crash between writing the records and writing
    // the marker is harmless, because re-running finds nothing left to do.
    let discovered = vec![
        managed("feat-login", Some("feat/login")),
        managed("fix-crash", Some("fix/crash")),
    ];
    let evidence = names(&["feat-login", "fix-crash"]);
    let first = plan_backfill(
        &discovered,
        root(),
        &evidence,
        &BTreeSet::new(),
        false,
        false,
    )
    .expect("first run should produce a plan");
    assert_eq!(first, names(&["feat-login", "fix-crash"]));

    let second = plan_backfill(&discovered, root(), &evidence, &first, false, false);
    assert_eq!(second, Some(BTreeSet::new()));
}

#[test]
fn an_already_migrated_project_is_not_planned_at_all() {
    // FR-006c. `None` is distinct from `Some(empty)`: it means "do not write, do not mark".
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login"]),
        &BTreeSet::new(),
        true,
        false,
    );
    assert_eq!(plan, None);
}

#[test]
fn an_unreadable_project_is_not_planned_at_all() {
    // FR-011. The project's one chance at a correct backfill has to survive to a run that can read
    // its records — migrating from an empty set we could not trust would silently write the wrong
    // answer and mark it done forever.
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login"]),
        &BTreeSet::new(),
        false,
        true,
    );
    assert_eq!(plan, None);
}

#[test]
fn unreadable_is_checked_before_already_migrated() {
    // Both say `None`, so the order is only observable in that neither can mask the other. Asserted
    // so a later refactor cannot reorder them into a state where an unreadable project gets marked.
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    assert_eq!(
        plan_backfill(
            &discovered,
            root(),
            &names(&["feat-login"]),
            &BTreeSet::new(),
            true,
            true
        ),
        None
    );
}

#[test]
fn a_project_with_no_evidence_still_produces_a_plan() {
    // `Some(empty)` — "ran, found nothing". The caller persists the marker for it, so the project
    // is never asked again. "Ran and found nothing" and "never ran" are different states.
    let discovered = vec![managed("tidy-up-the-parser", Some("tidy/parser"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &BTreeSet::new(),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn a_project_with_no_worktrees_produces_an_empty_plan() {
    let plan = plan_backfill(
        &[],
        root(),
        &names(&["ghost"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(BTreeSet::new()));
}

#[test]
fn evidence_for_a_worktree_that_no_longer_exists_writes_nothing() {
    // The evidence set is history; the discovered list is the present. A record is only ever
    // written for a directory that is actually there, so a deleted worktree leaves no ghost entry.
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let plan = plan_backfill(
        &discovered,
        root(),
        &names(&["feat-login", "deleted-long-ago"]),
        &BTreeSet::new(),
        false,
        false,
    );
    assert_eq!(plan, Some(names(&["feat-login"])));
}

#[test]
fn the_plan_is_read_only() {
    // FR-006a stated as the shape of the signature: everything it needs is already in hand, so
    // there is nothing for it to go and read. This test documents the property; it passes by
    // construction, and exists so that changing the signature to take a path breaks it.
    let discovered = vec![managed("feat-login", Some("feat/login"))];
    let evidence = names(&["feat-login"]);
    let existing = BTreeSet::new();
    let a = plan_backfill(&discovered, root(), &evidence, &existing, false, false);
    let b = plan_backfill(&discovered, root(), &evidence, &existing, false, false);
    assert_eq!(a, b, "the same inputs must always give the same plan");
}

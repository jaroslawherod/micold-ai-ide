//! Feature 582, T007 [US1]: which worktrees a project offers to attach (FR-001, FR-004, FR-013).
//!
//! Real git worktrees through the shared fixture, discovered with the production `discover`, so
//! the rule is checked over what git and the filesystem actually report.

#[path = "support/attach_fixture.rs"]
mod attach_fixture;

use std::collections::BTreeSet;

use attach_fixture::AttachFixture;
use micold_core::attach::{attachable_worktrees, AttachableWorktree, Availability, Unavailable};
use micold_core::git::GitCli;
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::worktree::{classify_owner, discover, ProvenanceView, WorktreeOwner};

fn offered(
    fx: &AttachFixture,
    records: &BTreeSet<String>,
    sessions: &[Session],
) -> Vec<AttachableWorktree> {
    let found = discover(&GitCli, &fx.repo, &[]);
    attachable_worktrees(&found, &ProvenanceView::new(records), sessions)
}

#[test]
fn agent_owned_unrecorded_worktrees_are_listed() {
    let fx = AttachFixture::new(&["alpha", "beta", "gamma"]);
    let list = offered(&fx, &BTreeSet::new(), &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["alpha", "beta", "gamma"],
        "FR-001: every unrecorded provider worktree is offered"
    );
    assert!(list
        .iter()
        .all(|w| w.availability == Availability::Attachable));
    assert_eq!(list[0].branch.as_deref(), Some("worktree-alpha"));
    assert_eq!(list[0].path, fx.worktree_path("alpha"));
}

#[test]
fn a_recorded_worktree_is_not_listed() {
    let fx = AttachFixture::new(&["alpha", "beta"]);
    let records: BTreeSet<String> = ["alpha".to_string()].into();
    let list = offered(&fx, &records, &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["beta"],
        "FR-004: an attached worktree is never offered again"
    );
}

#[test]
fn a_prunable_or_missing_worktree_is_unavailable_missing() {
    let fx = AttachFixture::new(&["alpha"]);
    fx.add_missing_worktree("gone");
    let list = offered(&fx, &BTreeSet::new(), &[]);
    let gone = list.iter().find(|w| w.dir_name == "gone").expect("listed");
    assert_eq!(
        gone.availability,
        Availability::Unavailable(Unavailable::Missing)
    );
}

#[test]
fn only_worktrees_directly_under_the_provider_directory_qualify() {
    let fx = AttachFixture::new(&["alpha"]);
    // Beside the repository, under the fixture's already-canonical base (no `\\?\` form on Windows).
    let outside = fx.repo.parent().unwrap().join("elsewhere");
    attach_fixture::git(
        &fx.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "outside",
            outside.to_str().unwrap(),
        ],
    );
    // Included by the user, so `discover` returns it with `included: true`.
    let found = discover(&GitCli, &fx.repo, std::slice::from_ref(&outside));
    assert!(
        found.iter().any(|w| w.included),
        "precondition: the outside worktree is discovered"
    );
    let records = BTreeSet::new();
    let list = attachable_worktrees(&found, &ProvenanceView::new(&records), &[]);
    let names: Vec<&str> = list.iter().map(|w| w.dir_name.as_str()).collect();
    assert_eq!(
        names,
        ["alpha"],
        "a worktree outside .claude/worktrees is never an attach candidate"
    );
}

#[test]
fn an_unattached_provider_worktree_stays_hidden() {
    let fx = AttachFixture::new(&["alpha"]);
    let found = discover(&GitCli, &fx.repo, &[]);
    let records = BTreeSet::new();
    assert_eq!(
        classify_owner(&found[0], &ProvenanceView::new(&records)),
        WorktreeOwner::Agent,
        "FR-013: listing it as attachable must not change that it is hidden until attached"
    );
}

#[test]
fn session_count_and_provider_come_from_the_catalog_sessions_at_that_worktree() {
    let fx = AttachFixture::new(&["alpha", "beta"]);
    let sessions = vec![
        Session::start_new(SessionLocation::Worktree("alpha".into()), AiCli::Pi),
        Session::start_new(SessionLocation::Worktree("alpha".into()), AiCli::Pi),
        Session::start_new(SessionLocation::Default, AiCli::ClaudeCode),
    ];
    let list = offered(&fx, &BTreeSet::new(), &sessions);
    let alpha = list.iter().find(|w| w.dir_name == "alpha").unwrap();
    let beta = list.iter().find(|w| w.dir_name == "beta").unwrap();
    assert_eq!((alpha.session_count, alpha.provider), (2, Some(AiCli::Pi)));
    assert_eq!((beta.session_count, beta.provider), (0, None));
}

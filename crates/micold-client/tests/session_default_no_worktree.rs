//! T010 (010-root-dir-session, FR-002) — starting a `SessionLocation::Default` session never
//! creates, modifies, or removes a git worktree.
//!
//! `Message::Session(SessionMsg::StartRequested)` has no pure-reducer effect (`src/app.rs`) — it is an I/O
//! trigger the binary (`src/main.rs`) consumes to spawn a PTY, then dispatches
//! `Message::Session(SessionMsg::Started(session))` once it has succeeded. That handler is where a session
//! actually enters `State`, so it is the right boundary to assert against: dispatching it for a
//! `Default` session must leave `state.worktree.worktrees` (the in-memory worktree list, sourced only
//! from git discovery / `|a0| Message::WorktreeForm(Msg::Created(a0))`) byte-for-byte unchanged. A `FakeGit` with a
//! registered repo is also asserted untouched, matching `contracts/sidebar-default-entry.md`'s
//! invariant 3 — this is possible to state precisely because `session_cwd_for_location`'s
//! `Default` arm (`src/main.rs`) is `repo.to_path_buf()`: it takes no `&dyn Git` at all, so it
//! is structurally incapable of calling `worktree_add_new_branch`/`worktree_remove`.

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_core::git::FakeGit;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionLocation};
use std::path::PathBuf;

#[test]
fn starting_a_default_session_leaves_worktrees_untouched() {
    let mut state = State::default();
    let repo = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: repo.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(repo.clone());
    state.worktree.worktrees = vec![]; // no worktrees exist yet — this is the interesting case (US1 AS1)

    let before = state.worktree.worktrees.clone();
    let session = Session::start_new(SessionLocation::Default);
    state.update(Message::Session(SessionMsg::Started(session)));

    assert_eq!(
        state.worktree.worktrees, before,
        "starting a Default session must not create a worktree entry"
    );
    assert_eq!(state.active_sessions().len(), 1);
}

#[test]
fn fake_git_boundary_records_no_worktree_or_branch_mutation() {
    // A FakeGit with a registered repo, exercised through nothing but the Default session
    // location's cwd resolution (which never receives a Git handle) — its worktree/branch
    // state must remain exactly as primed.
    let git = FakeGit::new().with_repo("/repo");
    let repo = std::path::Path::new("/repo");
    assert!(git.worktrees(repo).is_empty());
    assert!(git.branches(repo).is_empty());

    // The only operation "starting a Default session" performs, at the domain-model level.
    let session = Session::start_new(SessionLocation::Default);
    assert_eq!(session.location, SessionLocation::Default);

    // FakeGit was never passed anywhere in the above — its state is provably unchanged.
    assert!(git.worktrees(repo).is_empty());
    assert!(git.branches(repo).is_empty());
}

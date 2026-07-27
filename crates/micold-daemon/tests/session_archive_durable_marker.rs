//! Convergence fix (retrofit session, 2026-07-27): `Catalog::archive_session`,
//! `archive_worktree_sessions`, and `archive_session_ids` set the in-catalog `archived` flag but
//! never recorded the durable, provider-side suppression marker (bugfix BUG-003, FR-020c) — the
//! whole point of which is to survive the catalog/store itself being lost. This regressed when
//! feature 010 moved session-close/remove/worktree-delete handling into the daemon; the client's
//! own `SessionCloseRequested`/`SessionRemoveConfirmed` comments claimed it happened, but the
//! daemon-side code never actually called `AiCliProvider::mark_archived`.
//!
//! `CLAUDE_CONFIG_DIR` is a process-global env var read by `ClaudeProvider::config_dir()`. All
//! three scenarios below share one `#[test]` function (rather than one each) so they run
//! sequentially within this binary and never race each other over that global.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_core::project::{Availability, Project};
use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use uuid::Uuid;

fn catalog_with_session(
    data_dir: &std::path::Path,
    project_path: &std::path::Path,
    session_id: SessionId,
    location: SessionLocation,
) -> Catalog {
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_path.to_path_buf(),
        vec![Session::restored(
            session_id,
            location,
            SessionLabel::Named("Test session".into()),
            TerminalMode::AiCli,
        )],
    );
    let workspace = Workspace {
        projects: vec![Project::new(
            project_path.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project_path.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
    };

    let projects_path = data_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();

    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    )
}

#[test]
fn archiving_a_session_always_writes_the_durable_provider_marker() {
    let config = tempfile::tempdir().unwrap();
    // SAFETY: this is the only test in the workspace that reads/writes CLAUDE_CONFIG_DIR, and
    // all three scenarios run sequentially in this one function, so there is no cross-test race.
    std::env::set_var("CLAUDE_CONFIG_DIR", config.path());

    // --- Scenario 1: Catalog::archive_session (the Close/Remove RPC path, FR-015a/FR-015c) ---
    {
        let data_dir = tempfile::tempdir().unwrap();
        let project_path = PathBuf::from("/repo/alpha");
        let session_id = SessionId::from_uuid(Uuid::from_u128(0x1));
        let mut catalog = catalog_with_session(
            data_dir.path(),
            &project_path,
            session_id,
            SessionLocation::Default,
        );
        let cwd = SessionLocation::Default.cwd(&project_path);
        assert!(
            !ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "no marker before archiving"
        );

        catalog.archive_session(session_id).unwrap();

        assert!(
            ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "archive_session must record the durable provider-side marker (FR-020c), not just \
             the in-catalog `archived` flag — otherwise reconciliation resurrects it if the \
             catalog is lost"
        );
    }

    // --- Scenario 2: Catalog::archive_worktree_sessions (the WorktreeDelete path) ---
    {
        let data_dir = tempfile::tempdir().unwrap();
        let project_path = PathBuf::from("/repo/beta");
        let session_id = SessionId::from_uuid(Uuid::from_u128(0x2));
        let location = SessionLocation::Worktree("feat-x".into());
        let mut catalog =
            catalog_with_session(data_dir.path(), &project_path, session_id, location.clone());

        catalog
            .archive_worktree_sessions(&project_path, "feat-x")
            .unwrap();

        let cwd = location.cwd(&project_path);
        assert!(
            ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "archive_worktree_sessions (WorktreeDelete path) must record the durable \
             provider-side marker so a worktree recreated under the same name can't resurrect \
             its old sessions"
        );
    }

    // --- Scenario 3: Catalog::archive_session_ids (the empty-session pruning path) ---
    {
        let data_dir = tempfile::tempdir().unwrap();
        let project_path = PathBuf::from("/repo/gamma");
        let session_id = SessionId::from_uuid(Uuid::from_u128(0x3));
        let mut catalog = catalog_with_session(
            data_dir.path(),
            &project_path,
            session_id,
            SessionLocation::Default,
        );

        catalog.archive_session_ids(&[session_id]).unwrap();

        let cwd = SessionLocation::Default.cwd(&project_path);
        assert!(
            ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "archive_session_ids (empty-session pruning path) must record the durable marker too"
        );
    }
}

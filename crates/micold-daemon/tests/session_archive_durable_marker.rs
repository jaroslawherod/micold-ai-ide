//! Convergence fix (retrofit session, 2026-07-27): `Catalog::archive_session`,
//! `archive_worktree_sessions`, and `archive_session_ids` set the in-catalog `archived` flag but
//! never recorded the durable, provider-side suppression marker (bugfix BUG-003, FR-020c) — the
//! whole point of which is to survive the catalog/store itself being lost. This regressed when
//! feature 010 moved session-close/remove/worktree-delete handling into the daemon; the client's
//! own `SessionCloseRequested`/`SessionRemoveConfirmed` comments claimed it happened, but the
//! daemon-side code never actually called `AiCliProvider::mark_archived`.
//!
//! `CLAUDE_CONFIG_DIR`, `COPILOT_HOME` and `PI_CODING_AGENT_DIR` are process-global env vars, read
//! by each provider's `config_dir()`. All five scenarios below share one `#[test]` function (rather than one each) so they run sequentially
//! within this binary and never race each other over those globals.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::provider::{AiCliProvider, ClaudeProvider, CopilotProvider, PiProvider};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use uuid::Uuid;

fn catalog_with_session(
    data_dir: &Path,
    project_path: &Path,
    session_id: SessionId,
    location: SessionLocation,
    provider: AiCli,
) -> Catalog {
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_path.to_path_buf(),
        vec![Session::restored(
            session_id,
            location,
            SessionLabel::Named("Test session".into()),
            TerminalMode::AiCli,
            provider,
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
        ..Default::default()
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

/// Every path under `root`, recursively. Scenario 4 asserts the `claude` store is *untouched* by
/// a Copilot close, which is a stronger claim than "no marker at the one path `ClaudeProvider`
/// would look at".
fn tree(root: &Path) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(tree(&path));
        }
        out.insert(path);
    }
    out
}

#[test]
fn archiving_a_session_always_writes_the_durable_provider_marker() {
    let config = tempfile::tempdir().unwrap();
    let copilot_home = tempfile::tempdir().unwrap();
    // SAFETY: this is the only test in the workspace that reads/writes CLAUDE_CONFIG_DIR, and all
    // four scenarios run sequentially in this one function, so there is no cross-test race. The
    // same holds for COPILOT_HOME, which scenario 4 needs for the same reason: a real
    // `CopilotProvider::config_dir()` would otherwise resolve to the developer's own `~/.copilot`.
    std::env::set_var("CLAUDE_CONFIG_DIR", config.path());
    std::env::set_var("COPILOT_HOME", copilot_home.path());
    let pi_home = tempfile::tempdir().unwrap();
    std::env::set_var("PI_CODING_AGENT_DIR", pi_home.path());

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
            AiCli::ClaudeCode,
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
        let mut catalog = catalog_with_session(
            data_dir.path(),
            &project_path,
            session_id,
            location.clone(),
            AiCli::ClaudeCode,
        );

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
            AiCli::ClaudeCode,
        );

        catalog.archive_session_ids(&[session_id]).unwrap();

        let cwd = SessionLocation::Default.cwd(&project_path);
        assert!(
            ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "archive_session_ids (empty-session pruning path) must record the durable marker too"
        );
    }

    // --- Scenario 4: a **Copilot** session's marker lands in Copilot's store (FR-013, T044) ---
    //
    // All three paths above funnel into the one free function `catalog.rs::mark_archived_durable`,
    // so the provider substitution only needs proving once; what genuinely differs between them is
    // the *cwd*, which scenario 2 already covers. What is new here is the provider. Closing a
    // Copilot session has to write the marker where Copilot itself will look — a marker in the
    // wrong store suppresses nothing, and reconciliation (FR-020b) would hand the session back on
    // the next open, which is exactly the resurrection BUG-003 was about.
    {
        let data_dir = tempfile::tempdir().unwrap();
        let project_path = PathBuf::from("/repo/delta");
        let session_id = SessionId::from_uuid(Uuid::from_u128(0x4));
        let mut catalog = catalog_with_session(
            data_dir.path(),
            &project_path,
            session_id,
            SessionLocation::Default,
            AiCli::Copilot,
        );
        let cwd = SessionLocation::Default.cwd(&project_path);
        let claude_store_before = tree(config.path());

        catalog.archive_session(session_id).unwrap();

        assert!(
            CopilotProvider.is_archived(copilot_home.path(), &cwd, session_id.0),
            "closing a Copilot session must write its durable marker through the seam, into \
             Copilot's own store"
        );
        assert!(
            !ClaudeProvider.is_archived(config.path(), &cwd, session_id.0),
            "and not through `ClaudeProvider` — the marker must not be reachable in the `claude` \
             store under the layout `claude` reads"
        );
        assert_eq!(
            tree(config.path()),
            claude_store_before,
            "closing a Copilot session must leave the `claude` store untouched, not merely miss \
             the path `ClaudeProvider::is_archived` probes: the assertion above would pass for \
             free if a stray marker landed there under a different name"
        );
    }

    // --- Scenario 5: a closed **Pi** session stays closed after our own store is lost ---
    //
    // Feature 029, T052 (FR-016). The marker is the whole defence against resurrection, so the
    // assertion that matters is the one after the catalog is gone: a brand-new daemon over an
    // empty data directory, with Pi's conversation file still on disk, must not hand the session
    // back. Pi writes that file at creation, so unlike Copilot there is no window in which the
    // conversation is closed but not yet recorded — the scenario is the ordinary one.
    {
        let project_path = PathBuf::from("/repo/epsilon");
        let session_id = SessionId::from_uuid(Uuid::from_u128(0x5));
        let cwd = SessionLocation::Default.cwd(&project_path);

        let encoded: String = cwd
            .to_string_lossy()
            .trim_start_matches(['/', '\\'])
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':') {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        let conversations = pi_home
            .path()
            .join("sessions")
            .join(format!("--{encoded}--"));
        std::fs::create_dir_all(&conversations).unwrap();
        std::fs::write(
            conversations.join(format!("2026-09-13T10-00-00-000Z_{}.jsonl", session_id.0)),
            "{\"type\":\"session\"}\n",
        )
        .unwrap();

        {
            let data_dir = tempfile::tempdir().unwrap();
            let mut catalog = catalog_with_session(
                data_dir.path(),
                &project_path,
                session_id,
                SessionLocation::Default,
                AiCli::Pi,
            );
            catalog.archive_session(session_id).unwrap();
            assert!(
                PiProvider.is_archived(pi_home.path(), &cwd, session_id.0),
                "closing a Pi session must write its durable marker into Pi's own store"
            );
            // `data_dir` drops here: the application's own record of the close is gone.
        }

        let fresh = tempfile::tempdir().unwrap();
        let workspace = Workspace {
            projects: vec![Project::new(
                project_path.clone(),
                true,
                Availability::Available,
            )],
            active: Some(project_path.clone()),
            ..Default::default()
        };
        JsonFileStore::at(fresh.path().join("projects.json"))
            .save(&workspace)
            .unwrap();
        let state = micold_daemon::state::DaemonState::new(Catalog::load(
            Box::new(JsonFileStore::at(fresh.path().join("projects.json"))),
            Box::new(JsonFileSettingsStore::at(
                fresh.path().join("settings.json"),
            )),
        ));

        assert_eq!(
            state.discover_external_sessions(&project_path),
            0,
            "the conversation is still in Pi's store, and the marker alone keeps it closed"
        );
        assert!(state.sessions_for(&project_path).is_empty());
    }

    std::env::remove_var("PI_CODING_AGENT_DIR");
}

//! Sessions started outside this application are discovered (feature 026, T042a — FR-014, FR-015).
//!
//! # This is the gate, and `micold-core/tests/session_reconciliation.rs` is not
//!
//! That file says so in its own module doc: it is a **mirror** of a client function that no longer
//! exists, kept as a cheap place to pin the rules. Counting it as coverage for FR-014 would mean
//! the requirement was satisfied by a test of a function nothing calls.
//!
//! The real entry point is here — `DaemonState::discover_external_sessions`, run from the
//! `Attach` arm in the same `spawn_blocking` hop that refreshes the project's worktrees (research
//! R15). The daemon is `projects.json`'s single writer, it already has the location list in hand,
//! and a catalog snapshot is about to be sent.
//!
//! # What is asserted, and why each one is here
//!
//! - **It runs on a reopen, not only a first open.** A first-open-only rule would never surface the
//!   second session a user starts outside the application, which is the ordinary case rather than
//!   an edge one (Clarifications 2026-08-18).
//! - **Each id is judged by the provider whose store it came from**, so a Claude Code conversation
//!   is never adopted as a Copilot one or the reverse.
//! - **A closed session stays closed**, on the durable marker alone.
//! - **~250 recorded conversations all survive** — nothing is capped or aged out.
//! - **The ordering R15 depends on**: the catalog's known ids are subtracted *before* any
//!   `is_archived` stat, so a location holding hundreds of already-known conversations does no
//!   per-conversation filesystem work.
//! - **It is idempotent**, because a discovered session's `SessionId` is the CLI's own uuid.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// Each scenario below gets its **own** project, and therefore its own working directory.
///
/// Not a tidiness choice. The provider stores are shared across the whole function (the
/// environment is process-global, see [`ProviderStores`]), and every conversation is filed under
/// the cwd it was had in — so two scenarios sharing a cwd share their conversations, and the second
/// one starts by finding everything the first one wrote. That is what a "closed session stays
/// closed" assertion looks like when it fails for the wrong reason.
fn project_for(scenario: &str) -> PathBuf {
    PathBuf::from(format!("/repo/{scenario}"))
}

/// Two scratch provider stores, with `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` pointed at them.
///
/// Both are process-global and Rust runs tests on threads, so **every scenario in this file lives
/// inside one `#[test]` function** — the arrangement `session_archive_durable_marker.rs` already
/// uses, and for the same reason.
///
/// This was written as two functions first and both failed, in the way this kind of race does: one
/// saw the other's `claude` conversation and counted two sessions where one existed, and the other
/// had its `COPILOT_HOME` swapped mid-scenario and found nothing at all. Neither failure named the
/// environment; they read as discovery bugs.
struct ProviderStores {
    _base: tempfile::TempDir,
    claude: PathBuf,
    copilot: PathBuf,
}

impl ProviderStores {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        let claude = base.path().join("claude");
        let copilot = base.path().join("copilot");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude);
        std::env::set_var("COPILOT_HOME", &copilot);
        Self {
            _base: base,
            claude,
            copilot,
        }
    }

    /// Record a `claude` conversation for `cwd`, as `claude` itself would.
    fn claude_conversation(&self, cwd: &Path, id: Uuid) {
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let dir = self.claude.join("projects").join(encoded);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{id}.jsonl")),
            r#"{"type":"ai-title","aiTitle":"Started elsewhere"}"#,
        )
        .unwrap();
    }

    /// Record Copilot conversations for `cwd`: the per-cwd index plus each session's event log.
    fn copilot_conversations(&self, cwd: &Path, ids: &[Uuid]) {
        let hashed = micold_core::protocol::hashing::sha256_hex(cwd.to_string_lossy().as_bytes());
        let index_dir = self.copilot.join("sidebar-sessions-state");
        std::fs::create_dir_all(&index_dir).unwrap();
        let listed = ids
            .iter()
            .map(|id| format!("    \"{id}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        std::fs::write(
            index_dir.join(format!("{hashed}.json")),
            format!(
                "{{\n  \"schemaVersion\": 1,\n  \"cwd\": {:?},\n  \"sessionIds\": [\n{listed}\n  ]\n}}\n",
                cwd.to_string_lossy()
            ),
        )
        .unwrap();
        for id in ids {
            let dir = self.copilot.join("session-state").join(id.to_string());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("events.jsonl"), "{}\n").unwrap();
        }
    }
}

impl Drop for ProviderStores {
    fn drop(&mut self) {
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("COPILOT_HOME");
    }
}

fn state_with(data_dir: &Path, project: &Path, sessions: Vec<Session>) -> DaemonState {
    let project_path = project.to_path_buf();
    let mut by_project = BTreeMap::new();
    by_project.insert(project_path.clone(), sessions);
    let workspace = Workspace {
        projects: vec![Project::new(
            project_path.clone(),
            true,
            Availability::Available,
        )],
        active: Some(project_path),
        sessions: by_project,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = data_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    ))
}

/// The sessions the catalog holds for the project, as `(id, provider)`.
fn recorded(state: &DaemonState, project: &Path) -> Vec<(SessionId, AiCli)> {
    let mut out: Vec<(SessionId, AiCli)> = state
        .sessions_for(project)
        .into_iter()
        .map(|s| (s.id, s.provider))
        .collect();
    out.sort();
    out
}

#[test]
fn discovery_finds_what_the_clis_recorded_and_nothing_else() {
    let stores = ProviderStores::new();

    // --- A previously-unknown conversation per CLI is listed as that CLI's session ---
    {
        // The project root — the `Default` location. Worktrees would come from the git cache, which
        // is empty for a state built without a real repository, so every scenario uses the root of
        // its own project.
        let project = project_for("both-clis");
        let cwd = SessionLocation::Default.cwd(&project);
        let claude_id = Uuid::from_u128(0xC1);
        let copilot_id = Uuid::from_u128(0xC0);
        stores.claude_conversation(&cwd, claude_id);
        stores.copilot_conversations(&cwd, &[copilot_id]);

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(data_dir.path(), &project, Vec::new());

        assert_eq!(state.discover_external_sessions(&project), 2);
        assert_eq!(
            recorded(&state, &project),
            {
                let mut expected = vec![
                    (SessionId::from_uuid(claude_id), AiCli::ClaudeCode),
                    (SessionId::from_uuid(copilot_id), AiCli::Copilot),
                ];
                expected.sort();
                expected
            },
            "each id came back as a session of the CLI whose store it was found in"
        );

        // --- It runs on a reopen, and a reopen adds nothing ---
        assert_eq!(
            state.discover_external_sessions(&project),
            0,
            "a second pass finds both ids already known: a discovered session's id IS the CLI's \
             own conversation uuid, so a reopen is a no-op rather than a duplicate"
        );
        assert_eq!(recorded(&state, &project).len(), 2);

        // --- ...but a conversation started between opens IS surfaced ---
        // This is the half a first-open-only rule would miss, and it is the ordinary case: the
        // user starts a second session outside the application while the project is open.
        let later = Uuid::from_u128(0xC2);
        stores.copilot_conversations(&cwd, &[copilot_id, later]);
        assert_eq!(
            state.discover_external_sessions(&project),
            1,
            "discovery runs on every open, not only the first"
        );
        assert_eq!(recorded(&state, &project).len(), 3);
    }

    // --- A closed session stays closed, on the durable marker alone ---
    {
        let project = project_for("closed-stays-closed");
        let cwd = SessionLocation::Default.cwd(&project);
        let closed = Uuid::from_u128(0xDEAD);
        stores.copilot_conversations(&cwd, &[closed]);
        micold_core::provider::AiCliProvider::mark_archived(
            AiCli::Copilot.provider(),
            &stores.copilot,
            &cwd,
            closed,
        )
        .unwrap();

        // An entirely empty catalog: nothing in this application remembers the session was closed.
        // The provider-side sentinel alone has to suppress it (FR-015).
        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(data_dir.path(), &project, Vec::new());

        assert_eq!(state.discover_external_sessions(&project), 0);
        assert!(recorded(&state, &project).is_empty());
    }

    // --- A known session's provider is never re-derived from disk ---
    {
        let project = project_for("colliding-id");
        let cwd = SessionLocation::Default.cwd(&project);
        let shared = Uuid::from_u128(0xC011DE);
        stores.claude_conversation(&cwd, shared);
        stores.copilot_conversations(&cwd, &[shared]);

        let data_dir = tempfile::tempdir().unwrap();
        let known = Session::restored(
            SessionId::from_uuid(shared),
            SessionLocation::Default,
            SessionLabel::Named("Ours".to_string()),
            TerminalMode::AiCli,
            AiCli::Copilot,
        );
        let state = state_with(data_dir.path(), &project, vec![known]);

        assert_eq!(state.discover_external_sessions(&project), 0);
        assert_eq!(
            recorded(&state, &project),
            vec![(SessionId::from_uuid(shared), AiCli::Copilot)],
            "the same id exists in both stores; the persisted provider wins, so a live session's \
             CLI cannot be switched by what a scan happened to see last"
        );
    }

    // --- One provider unable to locate its store does not suppress the other ---
    {
        let project = project_for("unresolvable-store");
        let cwd = SessionLocation::Default.cwd(&project);
        // Each provider's `config_dir()` is resolved independently. `CLAUDE_CONFIG_DIR` points at
        // a directory that does not exist — so `claude` contributes nothing, exactly as it would on
        // a machine where it has never run — while Copilot's store is real and must still be read.
        //
        // The `config_dir() == None` arm proper needs the home directory itself to be unresolvable
        // and cannot be provoked portably; it is asserted through the injected-callback form in
        // `set_wide_provider_decisions.rs`.
        let restore = std::env::var_os("CLAUDE_CONFIG_DIR");
        std::env::set_var("CLAUDE_CONFIG_DIR", stores._base.path().join("nowhere"));

        let copilot_id = Uuid::from_u128(0xB0B);
        stores.copilot_conversations(&cwd, &[copilot_id]);

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(data_dir.path(), &project, Vec::new());

        assert_eq!(state.discover_external_sessions(&project), 1);
        assert_eq!(
            recorded(&state, &project),
            vec![(SessionId::from_uuid(copilot_id), AiCli::Copilot)]
        );
        if let Some(value) = restore {
            std::env::set_var("CLAUDE_CONFIG_DIR", value);
        }
    }

    // --- A long history is neither capped nor aged out, and costs nothing per conversation ---
    {
        let project = project_for("long-history");
        let cwd = SessionLocation::Default.cwd(&project);
        let ids: Vec<Uuid> = (1000..1250).map(Uuid::from_u128).collect();
        stores.copilot_conversations(&cwd, &ids);

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(data_dir.path(), &project, Vec::new());
        assert_eq!(
            state.discover_external_sessions(&project),
            250,
            "every recorded conversation is surfaced — nothing dropped by count or by age"
        );

        // The ordering R15 depends on, asserted where it is observable: with all 250 now known,
        // a second pass must not stat a marker for any of them. Make that measurable by planting a
        // marker for one *known* session — if the archived check ran before the known-ids
        // subtraction, that session would be dropped from the catalog on the next open.
        micold_core::provider::AiCliProvider::mark_archived(
            AiCli::Copilot.provider(),
            &stores.copilot,
            &cwd,
            ids[0],
        )
        .unwrap();
        assert_eq!(state.discover_external_sessions(&project), 0);
        assert_eq!(
            recorded(&state, &project).len(),
            250,
            "the known ids were subtracted before any per-id filesystem probe, so the marker was \
             never consulted — which is what keeps the pass proportional to *locations* rather \
             than to conversations (FR-014, R15)"
        );
    }
}

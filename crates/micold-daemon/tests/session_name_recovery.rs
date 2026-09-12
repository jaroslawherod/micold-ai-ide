//! A session that predates feature 029 gets its name back from the AI CLI's own records
//! (US2 — FR-006, FR-007, FR-008, FR-010, FR-004).
//!
//! # Why a second pass, and not a wider discovery pass
//!
//! `discover_external_sessions` (FR-014) already reads these same stores in this same blocking hop,
//! and it would be a smaller diff to let it stop subtracting the ids the catalog already knows.
//! That subtraction is what holds FR-014's per-*location* cost rule, though: reverse it and a
//! project with a long history stats every conversation on every open. Recovery cannot honour a
//! per-location rule — a name is per conversation — so it is a separate pass with a different bound
//! (research R4): a recovered name is **persisted**, so a session costs one read once and is
//! `Named` thereafter. The `a_second_pass_recovers_nothing` assertion below is that bound.
//!
//! # Everything lives in one `#[test]`
//!
//! `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` are process-global and Rust runs tests on threads, so two
//! `#[test]` functions here would swap each other's stores mid-scenario. The arrangement
//! `session_discovery.rs` and `session_archive_durable_marker.rs` already use, for the same reason.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// Each scenario gets its own project, and therefore its own working directory: the provider stores
/// are shared across the whole function and every conversation is filed under the cwd it was had
/// in, so two scenarios sharing a cwd share their conversations.
fn project_for(scenario: &str) -> PathBuf {
    PathBuf::from(format!("/repo/recover-{scenario}"))
}

/// Two scratch provider stores, with `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` pointed at them.
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

    fn claude_dir(&self, cwd: &Path) -> PathBuf {
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        self.claude.join("projects").join(encoded)
    }

    /// A `claude` transcript for `cwd` whose latest `ai-title` record is `title`.
    fn claude_conversation(&self, cwd: &Path, id: Uuid, title: &str) {
        let dir = self.claude_dir(cwd);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{id}.jsonl")),
            format!("{{\"type\":\"ai-title\",\"aiTitle\":{title:?}}}\n"),
        )
        .unwrap();
    }

    /// A `claude` transcript that exists but has never been titled — the FR-004 case.
    fn claude_untitled_conversation(&self, cwd: &Path, id: Uuid) {
        let dir = self.claude_dir(cwd);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{id}.jsonl")),
            "{\"type\":\"user\",\"message\":\"hello\"}\n",
        )
        .unwrap();
    }

    fn forget_claude_conversation(&self, cwd: &Path, id: Uuid) {
        std::fs::remove_file(self.claude_dir(cwd).join(format!("{id}.jsonl"))).unwrap();
    }

    /// A Copilot session directory whose `workspace.yaml` carries `name: <title>`.
    fn copilot_conversation(&self, id: Uuid, title: &str) {
        let dir = self.copilot.join("session-state").join(id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("events.jsonl"), "{}\n").unwrap();
        std::fs::write(
            dir.join("workspace.yaml"),
            format!("cwd: /repo\nname: {title}\n"),
        )
        .unwrap();
    }
}

impl Drop for ProviderStores {
    fn drop(&mut self) {
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("COPILOT_HOME");
    }
}

fn session(id: Uuid, label: SessionLabel, which: AiCli) -> Session {
    Session::restored(
        SessionId::from_uuid(id),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        which,
    )
}

fn workspace_with(project: &Path, sessions: Vec<Session>) -> Workspace {
    let path = project.to_path_buf();
    let mut by_project = BTreeMap::new();
    by_project.insert(path.clone(), sessions);
    Workspace {
        projects: vec![Project::new(path.clone(), true, Availability::Available)],
        active: Some(path),
        sessions: by_project,
        ..Default::default()
    }
}

fn state_with(data_dir: &Path, project: &Path, sessions: Vec<Session>) -> DaemonState {
    let projects_path = data_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace_with(project, sessions))
        .unwrap();
    DaemonState::new(catalog_at(data_dir))
}

fn catalog_at(data_dir: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(data_dir.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    )
}

/// The label the catalog holds for `id`, in id order — read from the durable side, not the
/// projection, since recovery is about what a restart will find.
fn label_in(catalog: &Catalog, project: &Path, id: Uuid) -> SessionLabel {
    catalog
        .workspace()
        .sessions
        .get(project)
        .and_then(|list| list.iter().find(|s| s.id.0 == id))
        .map(|s| s.label.clone())
        .expect("session is in the catalog")
}

fn label_of(state: &DaemonState, project: &Path, id: Uuid) -> SessionLabel {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id.0 == id)
        .map(|s| s.title)
        .expect("session is in the catalog")
}

#[test]
fn recovery_fills_pending_labels_from_the_clis_own_records_and_nothing_else() {
    let stores = ProviderStores::new();

    // --- A Pending session whose CLI records hold a name gets it, without being opened ---
    {
        let project = project_for("basic");
        let cwd = SessionLocation::Default.cwd(&project);
        let known = Uuid::from_u128(0xA1);
        stores.claude_conversation(&cwd, known, "Fix the flaky login test");

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(
            data_dir.path(),
            &project,
            vec![session(known, SessionLabel::Pending, AiCli::ClaudeCode)],
        );

        assert_eq!(
            state.recover_session_names(&project),
            1,
            "the session is known to the catalog and unnamed, and its CLI has a name for it"
        );
        assert_eq!(
            label_of(&state, &project, known),
            SessionLabel::Named("Fix the flaky login test".into()),
            "the row shows the name with the session never started — US2's independent test is \
             explicitly 'without opening it'"
        );

        // --- The recovered name is persisted, so the pass does not repeat (FR-007) ---
        assert_eq!(
            state.recover_session_names(&project),
            0,
            "a second pass finds the label already Named and skips it before any filesystem \
             access: this, not a per-location rule, is what bounds the pass (research R4)"
        );
        assert_eq!(
            label_in(&catalog_at(data_dir.path()), &project, known),
            SessionLabel::Named("Fix the flaky login test".into()),
            "and a freshly loaded catalog has it — recovery wrote, it did not merely project"
        );
    }

    // --- A session with no recorded name stays Pending, and writes nothing (FR-004) ---
    {
        let project = project_for("untitled");
        let cwd = SessionLocation::Default.cwd(&project);
        let untitled = Uuid::from_u128(0xB1);
        let missing = Uuid::from_u128(0xB2);
        stores.claude_untitled_conversation(&cwd, untitled);
        // `missing` has no transcript at all — the session was created and never used.

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(
            data_dir.path(),
            &project,
            vec![
                session(untitled, SessionLabel::Pending, AiCli::ClaudeCode),
                session(missing, SessionLabel::Pending, AiCli::ClaudeCode),
            ],
        );

        assert_eq!(
            state.recover_session_names(&project),
            0,
            "an untitled conversation and an absent one are both `None`, which is a no-op — \
             never an error and never a wrong name"
        );
        assert_eq!(label_of(&state, &project, untitled), SessionLabel::Pending);
        assert_eq!(label_of(&state, &project, missing), SessionLabel::Pending);
        assert_eq!(
            label_of(&state, &project, missing).display(),
            "New session",
            "\"New session\" stays a truthful statement about the conversation (FR-004)"
        );
    }

    // --- A Named session is skipped, and its name outlives the conversation (FR-008) ---
    {
        let project = project_for("already-named");
        let cwd = SessionLocation::Default.cwd(&project);
        let named = Uuid::from_u128(0xC1);
        stores.claude_conversation(&cwd, named, "A newer name in the transcript");

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(
            data_dir.path(),
            &project,
            vec![session(
                named,
                SessionLabel::Named("The name already recorded".into()),
                AiCli::ClaudeCode,
            )],
        );

        assert_eq!(
            state.recover_session_names(&project),
            0,
            "recovery only ever *fills* a Pending label; it is not a re-sync of every name"
        );
        assert_eq!(
            label_of(&state, &project, named),
            SessionLabel::Named("The name already recorded".into()),
            "the live path (US1) is what keeps a name current — recovery must not fight it, or a \
             restart would quietly revert a re-title the user just saw"
        );

        // The conversation goes away entirely. Nothing moves the label back to Pending.
        stores.forget_claude_conversation(&cwd, named);
        assert_eq!(state.recover_session_names(&project), 0);
        assert_eq!(
            label_of(&state, &project, named),
            SessionLabel::Named("The name already recorded".into()),
            "FR-008: a `None` read is a no-op, not a clear — there is no Named → Pending \
             transition anywhere in this feature"
        );
    }

    // --- Each session is read through its OWN provider (contract C16) ---
    {
        let project = project_for("own-provider");
        let cwd = SessionLocation::Default.cwd(&project);
        // The same uuid is recorded in *both* stores, with different names, and the catalog says
        // this session is Copilot's. One hoisted provider — or a loop that tried both — would read
        // `claude`'s transcript and name the row wrongly, silently, forever.
        let shared = Uuid::from_u128(0xD1);
        stores.claude_conversation(&cwd, shared, "Claude's name for this id");
        stores.copilot_conversation(shared, "Copilot's own name");

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(
            data_dir.path(),
            &project,
            vec![session(shared, SessionLabel::Pending, AiCli::Copilot)],
        );

        assert_eq!(state.recover_session_names(&project), 1);
        assert_eq!(
            label_of(&state, &project, shared),
            SessionLabel::Named("Copilot's own name".into()),
            "the name came from the store belonging to the CLI the session actually runs"
        );
    }

    // --- A provider whose config_dir is None does not suppress the other's (contract C20) ---
    {
        let project = project_for("one-provider-absent");
        let cwd = SessionLocation::Default.cwd(&project);
        let claude_id = Uuid::from_u128(0xE1);
        let copilot_id = Uuid::from_u128(0xE2);
        stores.claude_conversation(&cwd, claude_id, "Still recovered");
        stores.copilot_conversation(copilot_id, "Never reachable");

        let data_dir = tempfile::tempdir().unwrap();
        let state = state_with(
            data_dir.path(),
            &project,
            vec![
                session(claude_id, SessionLabel::Pending, AiCli::ClaudeCode),
                session(copilot_id, SessionLabel::Pending, AiCli::Copilot),
            ],
        );

        // Point Copilot's home at a directory that does not exist rather than unsetting the
        // variable: an unset `COPILOT_HOME` falls back to the real `~/.copilot`, which would make
        // this assertion depend on the developer's own machine.
        std::env::set_var("COPILOT_HOME", stores.copilot.join("gone"));
        let recovered = state.recover_session_names(&project);
        std::env::set_var("COPILOT_HOME", &stores.copilot);

        assert_eq!(
            recovered, 1,
            "the unreachable provider contributed nothing and stopped nothing"
        );
        assert_eq!(
            label_of(&state, &project, claude_id),
            SessionLabel::Named("Still recovered".into())
        );
        assert_eq!(label_of(&state, &project, copilot_id), SessionLabel::Pending);
    }

    // --- Recovery does not touch another project's sessions ---
    {
        let project = project_for("scoped");
        let other = project_for("scoped-other");
        let cwd = SessionLocation::Default.cwd(&project);
        let mine = Uuid::from_u128(0xF1);
        let theirs = Uuid::from_u128(0xF2);
        stores.claude_conversation(&cwd, mine, "Mine");
        stores.claude_conversation(&SessionLocation::Default.cwd(&other), theirs, "Theirs");

        let data_dir = tempfile::tempdir().unwrap();
        let mut ws = workspace_with(&project, vec![session(mine, SessionLabel::Pending, AiCli::ClaudeCode)]);
        ws.projects
            .push(Project::new(other.clone(), true, Availability::Available));
        ws.sessions.insert(
            other.clone(),
            vec![session(theirs, SessionLabel::Pending, AiCli::ClaudeCode)],
        );
        JsonFileStore::at(data_dir.path().join("projects.json"))
            .save(&ws)
            .unwrap();
        let state = DaemonState::new(catalog_at(data_dir.path()));

        assert_eq!(state.recover_session_names(&project), 1);
        assert_eq!(label_of(&state, &project, mine), SessionLabel::Named("Mine".into()));
        assert_eq!(
            label_of(&state, &other, theirs),
            SessionLabel::Pending,
            "the pass is per project, like the discovery pass beside it — the other project's \
             turn comes when it is opened"
        );
    }
}

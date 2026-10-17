//! Feature 551, US3 (T027): the discovery pass asks each provider which session started the open
//! sessions of a project, records the answer once (FR-001, FR-010), and leaves the sessions of a
//! provider that cannot say top-level (FR-002, SC-003). The Codex rollout is synthetic (see
//! `micold-core/tests/fixtures/codex/README.md`).
//!
//! `CODEX_HOME` and `CLAUDE_CONFIG_DIR` are process-global, so every scenario is one `#[test]`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::SystemTime;

use micold_core::project::{Availability, Project};
use micold_core::provider::{AiCliProvider, CodexProvider, ConversationRef};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

const PARENT_CONVERSATION: &str = "0190aaaa-0000-7000-8000-000000000001";
const CHILD_CONVERSATION: &str = "0190aaaa-0000-7000-8000-000000000002";

fn session(id: u128, provider: AiCli) -> Session {
    Session::restored(
        SessionId::from_uuid(Uuid::from_u128(id)),
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        provider,
    )
}

fn state_with(data_dir: &Path, project: &Path, sessions: Vec<Session>) -> DaemonState {
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
        ..Default::default()
    };
    let projects = data_dir.join("projects.json");
    JsonFileStore::at(projects.clone())
        .save(&workspace)
        .unwrap();
    DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects)),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    ))
}

fn origins(state: &DaemonState, project: &Path) -> Vec<(u128, Option<u128>)> {
    let mut out: Vec<_> = state
        .sessions_for(project)
        .into_iter()
        .map(|s| (s.id.0.as_u128(), s.origin.map(|o| o.id.0.as_u128())))
        .collect();
    out.sort();
    out
}

#[test]
fn discovery_records_what_codex_reports_and_leaves_a_gap_provider_top_level() {
    let base = tempfile::tempdir().unwrap();
    let codex = base.path().join("codex");
    let claude = base.path().join("claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::env::set_var("CODEX_HOME", &codex);
    std::env::set_var("CLAUDE_CONFIG_DIR", &claude);

    let project = Path::new("/repo/origins");
    let cwd = SessionLocation::Default.cwd(project);
    let day = codex.join("sessions/2026/10/13");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::write(
        day.join(format!(
            "rollout-2026-10-13T10-00-00-{CHILD_CONVERSATION}.jsonl"
        )),
        include_str!(
            "../../micold-core/tests/fixtures/codex/synthetic-rollout-child-thread-spawn.jsonl"
        ),
    )
    .unwrap();
    let bind = |session: u128, conversation: &str| {
        let found = ConversationRef {
            id: conversation.into(),
            cwd: cwd.clone(),
            created: SystemTime::now(),
        };
        CodexProvider
            .bind(&codex, Uuid::from_u128(session), &found)
            .unwrap();
    };
    bind(0xA1, PARENT_CONVERSATION);

    let data = tempfile::tempdir().unwrap();
    // The child is catalogued before its origin: both are known, the child's binding is made later.
    let state = state_with(
        data.path(),
        project,
        vec![
            session(0xA2, AiCli::Codex),
            session(0xC1, AiCli::ClaudeCode),
        ],
    );
    bind(0xA2, CHILD_CONVERSATION);

    // The parent is not catalogued yet: the origin is kept pending, the session stays top-level.
    assert_eq!(state.discover_origins(project), 1);
    assert_eq!(
        origins(&state, project),
        vec![(0xA2, Some(0xA1)), (0xC1, None)],
        "the Codex child names its origin; the Claude Code session, a gap, names none"
    );
    assert_eq!(state.discover_origins(project), 0, "recorded once");

    // The Claude Code session stays top-level however often the pass runs.
    assert_eq!(state.discover_origins(project), 0);
    assert_eq!(origins(&state, project)[1], (0xC1, None));
}

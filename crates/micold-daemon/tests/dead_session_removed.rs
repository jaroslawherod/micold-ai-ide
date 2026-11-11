#![cfg(unix)] // the test writes a `#!/bin/sh` stub and chmods it: Unix-only

//! BUG-1795 / 026 FR-008a — a session whose conversation is gone is removed, persistently.
//!
//! The refusal to resume it left the row in the catalog, neither running nor startable. Each case
//! ends with a reload of the catalog from disk, which is the restart.

use std::collections::BTreeMap;
use std::path::Path;

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

fn session(n: u128, label: SessionLabel) -> Session {
    Session::restored(
        SessionId::from_uuid(Uuid::from_u128(n)),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::Copilot,
    )
}

fn id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

/// A daemon over the catalog saved in `store`, as the service loads it at its start.
fn boot(store: &Path) -> DaemonState {
    DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    ))
}

fn listed(state: &DaemonState, project: &Path) -> Vec<SessionId> {
    state.sessions_for(project).iter().map(|s| s.id).collect()
}

fn record_conversation(home: &Path, id: SessionId) -> std::path::PathBuf {
    let dir = home.join("session-state").join(id.0.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("events.jsonl"), "{}\n").unwrap();
    dir
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `COPILOT_HOME` pointing at a fresh directory and `copilot` and `claude` stubs first on `PATH`,
/// until the guard drops. The environment is process-global, so the tests of this binary take turns.
fn env_with_stubs() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("COPILOT_HOME", home.path());
    let bin = tempfile::tempdir().unwrap().keep();
    for cli in ["copilot", "claude"] {
        let stub = bin.join(cli);
        std::fs::write(&stub, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&stub, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(bin).chain(
            std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths),
        ),
    )
    .unwrap();
    std::env::set_var("PATH", path);
    (guard, home)
}

#[test]
fn a_session_whose_conversation_is_gone_is_removed_and_stays_removed() {
    let (_env, home) = env_with_stubs();
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let project = project.path().to_path_buf();
    let (kept, vanishes, pending, derived_gone) = (id(1), id(2), id(3), id(4));
    JsonFileStore::at(store.path().join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions: BTreeMap::from([(
                project.clone(),
                vec![
                    session(1, SessionLabel::Named("kept".into())),
                    session(2, SessionLabel::Named("vanishes".into())),
                    session(3, SessionLabel::Pending),
                    session(4, SessionLabel::Derived("never listed twice".into())),
                ],
            )]),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();

    record_conversation(home.path(), kept);
    let vanishing = record_conversation(home.path(), vanishes);
    let state = boot(store.path());
    assert_eq!(
        state.present_interrupted_resumable_at_startup(),
        2,
        "the two sessions with a conversation are offered for resume"
    );

    // D3: startup removes nothing, not even a used session whose conversation is gone.
    assert_eq!(listed(&state, &project).len(), 4);
    let reloaded = boot(store.path());
    reloaded.present_interrupted_resumable_at_startup();
    assert!(
        listed(&reloaded, &project).contains(&derived_gone),
        "a Derived session whose conversation is gone is kept at startup (029 FR-008)"
    );

    // (a) the conversation vanishes after startup; the start is refused and the row goes.
    std::fs::remove_dir_all(&vanishing).unwrap();
    let result = state.start_session(vanishes, LaunchMode::Resume);
    assert!(result.is_err(), "the resume is refused");
    assert!(state.live_session(vanishes).is_none(), "nothing spawned");
    let now = listed(&state, &project);
    assert!(!now.contains(&vanishes), "not listed: {now:?}");
    assert!(now.contains(&kept) && now.contains(&pending), "not greedy");

    let restarted = boot(store.path());
    restarted.present_interrupted_resumable_at_startup();
    let after = listed(&restarted, &project);
    assert!(!after.contains(&vanishes), "not listed after the reload");
    assert!(
        after.contains(&kept) && after.contains(&pending),
        "a session with its conversation and a never-used one are kept: {after:?}"
    );
}

/// T095: no restored AI-CLI row is left neither running nor startable. One session of each
/// persisted shape is loaded; after startup each listed AI-CLI session is `Idle` or
/// `InterruptedResumable`, and a start of it is not refused for want of the session or its
/// conversation.
#[test]
fn every_restored_session_is_idle_or_resumable_and_can_start() {
    use micold_core::protocol::messages::WireLifecycle;
    let (_env, home) = env_with_stubs();
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let project = project.path().to_path_buf();
    let shapes = vec![
        session(11, SessionLabel::Pending),
        session(12, SessionLabel::Named("with conversation".into())),
        session(13, SessionLabel::Named("without conversation".into())),
        session(14, SessionLabel::Derived("derived".into())),
        Session::restored(
            id(15),
            SessionLocation::Default,
            SessionLabel::Pending,
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        ),
        Session::restored(
            id(16),
            SessionLocation::Default,
            SessionLabel::Pending,
            TerminalMode::Regular,
            AiCli::Copilot,
        ),
    ];
    JsonFileStore::at(store.path().join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions: BTreeMap::from([(project.clone(), shapes)]),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    record_conversation(home.path(), id(12));
    let state = boot(store.path());
    state.present_interrupted_resumable_at_startup();

    for summary in state.sessions_for(&project) {
        assert!(
            matches!(
                summary.lifecycle,
                WireLifecycle::Idle | WireLifecycle::InterruptedResumable
            ),
            "{:?} is neither running nor startable: {:?}",
            summary.id,
            summary.lifecycle
        );
        let launch = if summary.lifecycle == WireLifecycle::InterruptedResumable {
            LaunchMode::Resume
        } else {
            LaunchMode::Fresh
        };
        let started =
            state.start_session_gated(summary.id, launch, micold_core::occupancy::Additional::Yes);
        assert!(
            started.is_ok(),
            "{:?} could not start: {started:?}",
            summary.id
        );
    }
}

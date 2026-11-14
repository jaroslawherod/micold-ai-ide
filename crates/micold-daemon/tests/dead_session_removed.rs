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
    std::fs::create_dir_all(home.path().join("session-state")).unwrap();
    std::env::set_var("CLAUDE_CONFIG_DIR", home.path().join("claude"));
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

    // BUG-1852 (turns T092(b) around): startup removes a Named/Derived session whose conversation
    // is gone, and keeps the one whose conversation is there and the Pending one.
    assert_eq!(
        state.remove_gone_sessions_at_startup(),
        1,
        "the Derived one goes"
    );
    let now = listed(&state, &project);
    assert!(
        !now.contains(&derived_gone),
        "a gone Derived session is removed at startup"
    );
    assert!(now.contains(&kept) && now.contains(&vanishes) && now.contains(&pending));
    let reloaded = boot(store.path());
    reloaded.present_interrupted_resumable_at_startup();
    assert!(
        !listed(&reloaded, &project).contains(&derived_gone),
        "and stays removed after a reload"
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

fn save(store: &Path, project: &Path, sessions: Vec<Session>) {
    JsonFileStore::at(store.join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(
                project.to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project.to_path_buf()),
            sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
}

fn of(n: u128, label: SessionLabel, cli: AiCli, mode: TerminalMode) -> Session {
    Session::restored(id(n), SessionLocation::Default, label, mode, cli)
}

/// Run the startup passes on a daemon booted from `store`, then again on a reload; return what is
/// listed after each (the second is the restart).
fn startup_twice(store: &Path, project: &Path) -> (Vec<SessionId>, Vec<SessionId>) {
    let state = boot(store);
    state.present_interrupted_resumable_at_startup();
    state.remove_gone_sessions_at_startup();
    let first = listed(&state, project);
    let reloaded = boot(store);
    reloaded.present_interrupted_resumable_at_startup();
    reloaded.remove_gone_sessions_at_startup();
    (first, listed(&reloaded, project))
}

fn named(n: u128, cli: AiCli) -> Session {
    of(
        n,
        SessionLabel::Named(format!("s{n}")),
        cli,
        TerminalMode::AiCli,
    )
}

/// BUG-1852: a `Named` and a `Derived` session whose conversation is gone are removed at startup,
/// persistently, while the readable store still holds another conversation.
#[test]
fn startup_removes_named_and_derived_sessions_whose_conversation_is_gone() {
    let (_env, home) = env_with_stubs();
    let (store, project) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    save(
        store.path(),
        project.path(),
        vec![
            named(1, AiCli::Copilot),
            named(2, AiCli::Copilot),
            of(
                3,
                SessionLabel::Derived("d".into()),
                AiCli::Copilot,
                TerminalMode::AiCli,
            ),
        ],
    );
    record_conversation(home.path(), id(1));
    let (first, after) = startup_twice(store.path(), project.path());
    assert_eq!(first, vec![id(1)]);
    assert_eq!(after, vec![id(1)], "stays removed after a reload");
}

/// BUG-1852 keep cases: ignorance, other providers' ids, never-used and non-AI sessions.
#[test]
fn startup_keeps_every_session_it_cannot_prove_gone() {
    let (_env, home) = env_with_stubs();
    let (store, project) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    save(
        store.path(),
        project.path(),
        vec![
            named(1, AiCli::Copilot),
            named(2, AiCli::Copilot),
            of(
                3,
                SessionLabel::Pending,
                AiCli::Copilot,
                TerminalMode::AiCli,
            ),
            named(4, AiCli::Codex),
            of(
                5,
                SessionLabel::Named("shell".into()),
                AiCli::Copilot,
                TerminalMode::Regular,
            ),
        ],
    );
    record_conversation(home.path(), id(1));
    record_conversation(home.path(), id(2));
    let (first, after) = startup_twice(store.path(), project.path());
    let all = vec![id(1), id(2), id(3), id(4), id(5)];
    assert_eq!(first, all);
    assert_eq!(after, all);
}

/// BUG-1852: a config dir that resolves but is absent, or exists with no store in it, is a lost
/// store, not a lost conversation: nothing is removed.
#[test]
fn startup_removes_nothing_when_the_store_is_unreadable() {
    let (_env, home) = env_with_stubs();
    let (store, project) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    save(store.path(), project.path(), vec![named(1, AiCli::Copilot)]);

    // An empty store (emptied or recreated), one Named session: kept.
    assert_eq!(startup_twice(store.path(), project.path()).1, vec![id(1)]);

    // Config dir exists, no `session-state` in it (a single Named session: not "every candidate").
    std::fs::remove_dir_all(home.path().join("session-state")).unwrap();
    assert_eq!(startup_twice(store.path(), project.path()).1, vec![id(1)]);

    // Config dir itself absent (unmounted disk, moved home).
    std::env::set_var("COPILOT_HOME", home.path().join("not-mounted"));
    assert_eq!(startup_twice(store.path(), project.path()).1, vec![id(1)]);

    // The store path is a file, not a directory.
    std::fs::write(home.path().join("session-state"), "x").unwrap();
    std::env::set_var("COPILOT_HOME", home.path());
    assert_eq!(startup_twice(store.path(), project.path()).1, vec![id(1)]);
}

/// BUG-1852: a provider whose every session (more than one) looks gone at once is a lost store.
#[test]
fn startup_removes_nothing_of_a_provider_whose_every_session_looks_gone() {
    let (_env, _home) = env_with_stubs();
    let (store, project) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    save(
        store.path(),
        project.path(),
        vec![named(1, AiCli::Copilot), named(2, AiCli::Copilot)],
    );
    assert_eq!(
        startup_twice(store.path(), project.path()).1,
        vec![id(1), id(2)]
    );
}

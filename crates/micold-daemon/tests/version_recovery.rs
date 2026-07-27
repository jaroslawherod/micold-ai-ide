//! Phase 8 (US6) — contract mismatch fails loudly and recoverably (T071/T072, FR-006a/b).
//!
//! When the service restarts — a reboot, a crash, or a deliberate contract-mismatch restart — it
//! finds durable session records but no live processes. FR-006a/b require it to present the ones that
//! had a recorded conversation as **interrupted-but-resumable** — visibly distinct from both
//! `Running` and a deliberately stopped session — and to relaunch **none** of them; a single explicit
//! `SessionStart` is the only thing that resumes one. These drive the catalog reload logic directly
//! (no socket) and the domain `start` transition that resume rides on.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use micold_core::project::{Availability, Project};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{LoadOutcome, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;

/// A project store that serves a fixed in-memory workspace — the durable records a restarted service
/// would reload. (`Session` is not `Serialize`, so this is how a test injects loaded sessions.)
struct FakeStore(Workspace);

impl ProjectStore for FakeStore {
    fn load(&self) -> LoadOutcome {
        LoadOutcome {
            workspace: self.0.clone(),
            status: LoadStatus::Loaded,
        }
    }
    fn save(&self, _workspace: &Workspace) -> std::io::Result<()> {
        Ok(())
    }
}

/// A restored (loaded-from-disk) session — `Idle`, as every durable record loads.
fn restored(id: SessionId, mode: TerminalMode) -> Session {
    Session::restored(id, SessionLocation::Default, SessionLabel::Pending, mode)
}

/// A catalog freshly loaded from `sessions` under `project` — the just-restarted service's state.
fn loaded_catalog(project: &Path, sessions: Vec<Session>, settings_path: &Path) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
        worktree_names: BTreeMap::new(),
    };
    Catalog::load(
        Box::new(FakeStore(workspace)),
        Box::new(JsonFileSettingsStore::at(settings_path.to_path_buf())),
    )
}

fn lifecycle_of(catalog: &Catalog, id: SessionId) -> SessionLifecycle {
    catalog
        .workspace()
        .sessions
        .values()
        .flatten()
        .find(|s| s.id == id)
        .expect("session present")
        .lifecycle
}

#[test]
fn restart_presents_previously_running_sessions_as_interrupted_resumable() {
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();

    // Three durable records, all loaded `Idle`: one AI-CLI session that had a conversation (was
    // running), one AI-CLI session with none (created but never started / deliberately empty), and a
    // Regular shell (nothing to resume).
    let was_running = SessionId::new();
    let never_started = SessionId::new();
    let shell = SessionId::new();
    let mut catalog = loaded_catalog(
        project.path(),
        vec![
            restored(was_running, TerminalMode::AiCli),
            restored(never_started, TerminalMode::AiCli),
            restored(shell, TerminalMode::Regular),
        ],
        &settings.path().join("settings.json"),
    );

    // Everything loads Idle before the reload step runs.
    assert_eq!(lifecycle_of(&catalog, was_running), SessionLifecycle::Idle);

    // The service startup step: only sessions the provider has a recorded conversation for are
    // resumable. We inject that decision deterministically.
    let with_conversation: BTreeSet<SessionId> = [was_running].into_iter().collect();
    let marked =
        catalog.present_interrupted_resumable(|id, _cwd, _mode| with_conversation.contains(&id));

    assert_eq!(marked, 1, "only the session with a conversation is marked");
    assert_eq!(
        lifecycle_of(&catalog, was_running),
        SessionLifecycle::InterruptedResumable,
        "a previously-running session comes back interrupted-resumable (FR-006a)"
    );
    assert_eq!(
        lifecycle_of(&catalog, never_started),
        SessionLifecycle::Idle,
        "a session with no conversation stays Idle — distinct from interrupted-resumable"
    );
    assert_eq!(
        lifecycle_of(&catalog, shell),
        SessionLifecycle::Idle,
        "a Regular shell has no conversation to resume and is never marked"
    );
    // FR-006b: no process was launched — InterruptedResumable is a stopped state, not active.
    assert!(
        !catalog
            .workspace()
            .sessions
            .values()
            .flatten()
            .any(|s| s.is_active()),
        "service startup relaunches nothing (FR-006b)"
    );
}

#[test]
fn present_interrupted_resumable_never_overrides_a_running_or_failed_session() {
    // Defensive: the reload step only touches `Idle` records. A session already advanced past Idle
    // (e.g. it somehow reads Running/Failed) must not be reset — `present_interrupted_resumable`
    // marks only `Idle`, so an always-true predicate still leaves non-Idle sessions untouched.
    let project = tempfile::tempdir().unwrap();
    let settings = tempfile::tempdir().unwrap();
    let failed = SessionId::new();
    let mut sessions = vec![restored(failed, TerminalMode::AiCli)];
    // Force it Failed via the crash path (three unexpected exits).
    for _ in 0..3 {
        sessions[0].on_unexpected_exit();
    }
    assert_eq!(sessions[0].lifecycle, SessionLifecycle::Failed);

    let mut catalog = loaded_catalog(
        project.path(),
        sessions,
        &settings.path().join("settings.json"),
    );
    let marked = catalog.present_interrupted_resumable(|_id, _cwd, _mode| true);

    assert_eq!(marked, 0);
    assert_eq!(lifecycle_of(&catalog, failed), SessionLifecycle::Failed);
}

#[test]
fn session_start_is_the_single_explicit_resume_of_an_interrupted_session() {
    // FR-006a: an interrupted-resumable session is resumed by exactly one explicit action — the same
    // `start` the daemon's `SessionStart` handler drives (with `LaunchMode::Resume`, so it continues
    // the prior conversation). Starting moves it out of the stopped state into `Starting`.
    let mut session = restored(SessionId::new(), TerminalMode::AiCli);
    session.mark_interrupted_resumable();
    assert_eq!(session.lifecycle, SessionLifecycle::InterruptedResumable);
    assert!(
        !session.is_active(),
        "interrupted-resumable is a stopped state"
    );

    session.start();
    assert_eq!(
        session.lifecycle,
        SessionLifecycle::Starting,
        "the explicit start resumes it"
    );
    assert!(session.is_active());
}

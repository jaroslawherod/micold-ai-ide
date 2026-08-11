//! US2 (T046/T047) — the daemon's activity + title pipeline projects live runtime state onto the
//! catalog snapshot clients render (FR-016a–d, FR-011a).
//!
//! Two mechanisms feed a session's projected `SessionSummary`:
//! - **Hooks** (delivered by the loopback receiver) drive the activity FSM through
//!   [`DaemonState::note_activity`]; the derived signal appears in the snapshot immediately.
//! - **Terminal signals** — the OSC-0 title and a braille-spinner glyph — are drained on the
//!   supervisor cadence by [`DaemonState::drain_signals`]; the title becomes the live session title
//!   and a spinner is Working-only evidence (invariant H1a).
//!
//! Both are proven here against a real PTY. For activity-from-hooks a `cat` session is a
//! deterministic sink; for the OSC-title path a `printf` process emits the escape straight to its
//! stdout (raw, bypassing line-discipline echo), so the daemon's VT emulator parses `Event::Title`
//! exactly as a real `claude` emitting it would — no shell prompt to race or overwrite the title.
//! Each process is registered under a catalog-known session id, so it is both durable (appears in
//! the snapshot) and live (has a PTY to observe).

#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{ActivitySignal, SessionSummary};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use uuid::Uuid;

const SESSION_U128: u128 = 0x5E55;

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

/// A catalog holding one AI-CLI session (id [`SESSION_U128`]) at a project root, persisted so
/// [`Catalog::load`] adopts it. The `title` starts `Pending` so the OSC-title projection is visible.
fn catalog_with_session(project_dir: &std::path::Path, store_dir: &std::path::Path) -> Catalog {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_dir.to_path_buf(), vec![session]);

    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };

    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

/// Register a `cat` PTY under the catalog-known id so the session is both durable and live.
fn register_cat(state: &DaemonState, id: SessionId) -> std::sync::Arc<PtySession> {
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn cat session");
    state.register_session(session)
}

/// Register a process that emits `printf_body` (a `printf`-escaped string, e.g. an OSC-0 title
/// sequence) straight to its stdout, then idles — so the daemon's VT parser sees exactly what a real
/// process emitting that sequence would produce, with no line-discipline echo in the way.
fn register_emitter(
    state: &DaemonState,
    id: SessionId,
    printf_body: &str,
) -> std::sync::Arc<PtySession> {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(format!("printf '{printf_body}'; sleep 5"));
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn emitter session");
    state.register_session(session)
}

/// The projected summary for `id` from the current snapshot.
fn summary_of(state: &DaemonState, id: SessionId) -> SessionSummary {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id == id)
        .expect("the session appears in the snapshot")
}

#[test]
fn hooks_drive_the_projected_activity_signal() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session(project.path(), store.path()));
    let session = register_cat(&state, id);

    // A never-touched session projects Unknown (H1: no hooks → never AwaitingInput).
    assert_eq!(summary_of(&state, id).activity, ActivitySignal::Unknown);

    // UserPromptSubmit → Working, and the change is reported so the caller can push.
    assert!(state.note_activity(id, ActivityEvent::Hook(HookKind::UserPromptSubmit)));
    assert_eq!(summary_of(&state, id).activity, ActivitySignal::Working);

    // PostToolUse is a no-op — no change to report, still Working.
    assert!(!state.note_activity(id, ActivityEvent::Hook(HookKind::PostToolUse)));
    assert_eq!(summary_of(&state, id).activity, ActivitySignal::Working);

    // Stop → AwaitingInput (notification-grade).
    assert!(state.note_activity(id, ActivityEvent::Hook(HookKind::Stop)));
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::AwaitingInput
    );

    session.kill().expect("kill");
}

#[test]
fn a_hook_for_an_unhosted_session_reports_nothing() {
    // H1: a hook for a session the daemon is not hosting invents no state.
    let state = DaemonState::new(Catalog::ephemeral());
    assert!(!state.note_activity(SessionId::new(), ActivityEvent::Hook(HookKind::Stop)));
}

#[test]
fn an_osc_title_becomes_the_live_session_title_and_a_spinner_means_working() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session(project.path(), store.path()));
    // Emit an OSC-0 title carrying a braille spinner glyph (U+280B = octal \342\240\213).
    let session = register_emitter(&state, id, r"\033]0;\342\240\213 Fixing the parser\007");

    // Drain until the title lands. (The session starts Pending/Unknown; the emitter changes it.)
    let landed = wait_until(Duration::from_secs(5), || {
        state.drain_signals();
        summary_of(&state, id).title == SessionLabel::Named("Fixing the parser".into())
    });
    assert!(landed, "the OSC title must become the live session title");

    // The leading spinner glyph was stripped from the *title* but recorded as Working evidence.
    let after = summary_of(&state, id);
    assert_eq!(after.title, SessionLabel::Named("Fixing the parser".into()));
    assert_eq!(
        after.activity,
        ActivitySignal::Working,
        "a braille spinner glyph is Working-only evidence (H1a)"
    );

    session.kill().expect("kill");
}

#[test]
fn a_spinner_never_moves_a_session_out_of_awaiting_input() {
    // H1a end-to-end: once a hook says AwaitingInput, later spinner evidence cannot revert it to
    // Working — terminal evidence is monotone toward Working *only from Unknown*.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session(project.path(), store.path()));
    let session = register_emitter(&state, id, r"\033]0;\342\240\213 still spinning\007");

    // Apply the AwaitingInput hook *before* the first drain, so the spinner arrives into a
    // non-Unknown state — where H1a makes it a no-op.
    assert!(state.note_activity(id, ActivityEvent::Hook(HookKind::Stop)));
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::AwaitingInput
    );

    // Give the title time to land, draining throughout.
    wait_until(Duration::from_secs(2), || {
        state.drain_signals();
        summary_of(&state, id).title == SessionLabel::Named("still spinning".into())
    });
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::AwaitingInput,
        "spinner evidence must not override AwaitingInput"
    );

    session.kill().expect("kill");
}

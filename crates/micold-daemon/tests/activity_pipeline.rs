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
//!
//! # Two sources, one machine (feature 026, T057)
//!
//! Feature 026 leaves the FSM itself untouched — its transition table is unit-tested exhaustively
//! in `micold-daemon/src/activity.rs`, and this file proves the *pipeline* around it is likewise
//! unchanged. What the feature does change is arithmetic no one states out loud: a **Copilot**
//! session has **two** live sources, not one.
//!
//! - Its `events.jsonl`, tailed by [`DaemonState::open_event_log_tail`] and mapped by
//!   `copilot_event` — the provider-specific source, `ActivitySource::EventLog`.
//! - The braille-spinner scan, which is **shared and not provider-conditional**:
//!   `micold-daemon/src/terminal.rs` scans *every* PTY session's OSC-0 titles for a codepoint in
//!   U+2800..=U+28FF and raises `SpinnerObserved`. Nothing there asks which CLI is running.
//!
//! So the framing "only the event source differs" is wrong, and the tests below assert the true
//! claim instead: both sources reach a Copilot session, and they cannot contradict each other,
//! because `SpinnerObserved` only ever moves `Unknown -> Working` and is a no-op from every other
//! state (H1a/A1a). The event log can always overrule the spinner; the spinner can never overrule
//! the log.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{ActivitySignal, SessionSummary};
use micold_core::provider::ActivitySource;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
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
/// `cli` is a parameter because the pipeline is not supposed to care which one it is (T057).
fn catalog_with_session(
    project_dir: &std::path::Path,
    store_dir: &std::path::Path,
    cli: AiCli,
) -> Catalog {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        cli,
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

/// `COPILOT_HOME` is process-global, so every test that points the Copilot provider at a private
/// config directory takes this lock for its whole body; the guard clears the variable on drop.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A private `COPILOT_HOME` for the duration of one test.
struct CopilotHome {
    dir: tempfile::TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl CopilotHome {
    fn new() -> Self {
        // `into_inner` on a poisoned lock: a panicking test must not cascade into the others.
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("COPILOT_HOME", dir.path());
        Self { dir, _guard: guard }
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }
}

impl Drop for CopilotHome {
    fn drop(&mut self) {
        std::env::remove_var("COPILOT_HOME");
    }
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
    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
    ));
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
    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
    ));
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
    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
    ));
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

#[test]
fn a_copilot_session_is_watched_by_its_event_log_and_scanned_for_spinners_like_any_other() {
    // T057, half one: the braille-spinner path is shared, not provider-conditional. A Copilot
    // session names an `EventLog` source — and *still* gets the terminal scan every PTY gets.
    let home = CopilotHome::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));

    // The first of the two sources, named by the provider rather than assumed here.
    let provider = AiCli::Copilot.provider();
    assert!(
        matches!(
            provider.activity_source(home.path(), project.path(), id.0),
            ActivitySource::EventLog { .. }
        ),
        "a Copilot session's provider-specific source is its event log"
    );

    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::Copilot,
    ));
    // The second: an OSC-0 title carrying U+280B, emitted by a session the daemon knows is Copilot.
    let session = register_emitter(&state, id, r"\033]0;\342\240\213 Fixing the parser\007");

    let landed = wait_until(Duration::from_secs(5), || {
        state.drain_signals();
        summary_of(&state, id).title == SessionLabel::Named("Fixing the parser".into())
    });
    assert!(landed, "the OSC title must become the live session title");
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::Working,
        "the spinner scan reads every PTY session's title, whichever CLI produced it"
    );

    session.kill().expect("kill");
}

#[test]
fn a_copilot_event_log_and_the_shared_spinner_scan_cannot_contradict_each_other() {
    // T057, half two: with both real sources live on one session, the ordering that could produce a
    // contradiction does not. `assistant.turn_end` puts the session in AwaitingInput; the spinner
    // that arrives afterwards is a no-op there (H1a/A1a), so it cannot drag the badge back to
    // Working and hide a session that is waiting for its user.
    let home = CopilotHome::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));

    // The log the tail will watch. Created empty first: `EventLogTail::open` starts at the file's
    // current end, so a line written before the watch exists would never be seen.
    let events = home
        .path()
        .join("session-state")
        .join(id.0.to_string())
        .join("events.jsonl");
    std::fs::create_dir_all(events.parent().unwrap()).unwrap();
    std::fs::write(&events, "").unwrap();

    let state = Arc::new(DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::Copilot,
    )));
    // Registered before the watch is opened, because the tail is stored on the live session.
    let session = register_emitter(&state, id, r"\033]0;\342\240\213 still spinning\007");
    state.open_event_log_tail(id);
    assert_eq!(summary_of(&state, id).activity, ActivitySignal::Unknown);

    // Source one speaks. Deliberately *not* draining while we wait: the spinner must arrive into a
    // non-Unknown state, which is the case H1a is about.
    {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&events)
            .unwrap();
        writeln!(f, r#"{{"type":"assistant.turn_end","data":{{}}}}"#).unwrap();
    }
    assert!(
        wait_until(Duration::from_secs(10), || summary_of(&state, id).activity
            == ActivitySignal::AwaitingInput),
        "the event log's turn_end must reach the activity machine"
    );

    // Source two speaks, second — and is ignored.
    let landed = wait_until(Duration::from_secs(5), || {
        state.drain_signals();
        summary_of(&state, id).title == SessionLabel::Named("still spinning".into())
    });
    assert!(
        landed,
        "the spinner title must actually have been drained, or this proves nothing"
    );
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::AwaitingInput,
        "the shared spinner scan must not overrule what the event log said"
    );

    session.kill().expect("kill");
}

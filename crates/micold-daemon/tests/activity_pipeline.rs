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

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{ActivitySignal, SessionProcess, SessionSummary};
use micold_core::provider::ActivitySource;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, ShellInstanceId, TerminalMode,
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

fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let (cols, rows) = (grid.columns(), grid.screen_lines());
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

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
    catalog_with_session_in_mode(project_dir, store_dir, cli, TerminalMode::AiCli)
}

/// [`catalog_with_session`], with the session's terminal mode chosen by the caller.
fn catalog_with_session_in_mode(
    project_dir: &std::path::Path,
    store_dir: &std::path::Path,
    cli: AiCli,
    mode: TerminalMode,
) -> Catalog {
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Pending,
        mode,
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

/// Register a `cat` PTY (`cmd /q` on Windows) under the catalog-known id so the session is both
/// durable and live.
fn register_cat(state: &DaemonState, id: SessionId) -> std::sync::Arc<PtySession> {
    #[cfg(unix)]
    let mut cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.arg("/q");
        cmd
    };
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
    let mut cmd = emitter_command(printf_body);
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn emitter session");
    state.register_session(session)
}

#[cfg(unix)]
fn emitter_command(printf_body: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(format!("printf '{printf_body}'; sleep 5"));
    cmd
}

/// ConPTY does not pass a child's own escape sequences through verbatim: it keeps the console's
/// state and re-renders it as VT. A title reaches the parser as the OSC-0 ConPTY emits when the
/// console title changes, so on Windows the emitter sets the title and ConPTY writes the sequence.
#[cfg(windows)]
fn emitter_command(printf_body: &str) -> CommandBuilder {
    let decoded = printf_octal_unescape(printf_body);
    let title = decoded
        .strip_prefix("\x1b]0;")
        .and_then(|rest| rest.strip_suffix('\x07'))
        .expect("the Windows emitter only sets OSC-0 titles");
    let codes: Vec<String> = title.encode_utf16().map(|unit| unit.to_string()).collect();
    let mut cmd = CommandBuilder::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command"]);
    cmd.arg(format!(
        "[Console]::Title = -join [char[]]({}); Start-Sleep -Seconds 5",
        codes.join(",")
    ));
    cmd
}

/// Resolve the `\NNN` octal escapes a `printf` format uses, decoding the bytes as UTF-8.
#[cfg(windows)]
fn printf_octal_unescape(body: &str) -> String {
    let mut bytes = Vec::new();
    let mut rest = body.as_bytes();
    while let Some((&first, tail)) = rest.split_first() {
        if first == b'\\' && tail.len() >= 3 && tail[..3].iter().all(|b| (b'0'..=b'7').contains(b))
        {
            let octal = std::str::from_utf8(&tail[..3]).unwrap();
            bytes.push(u8::from_str_radix(octal, 8).expect("a three-digit octal escape"));
            rest = &tail[3..];
        } else {
            bytes.push(first);
            rest = tail;
        }
    }
    String::from_utf8(bytes).expect("the escaped body is UTF-8")
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
#[cfg_attr(
    windows,
    ignore = "first run on Windows in #332: PowerShell's own startup title is recorded as the name; fix in the #332 review follow-up"
)]
fn the_observed_title_is_handed_back_for_recording_exactly_once() {
    // Feature 029, contract C9/C10. Before it, `drain_signals` swallowed the title into
    // `LiveSession::last_title` — un-persisted by its own doc comment — and `overlay_live_summaries`
    // painted it over the catalog's label on the way out. The name was observed, projected, and
    // never recorded, which is the whole of the bug.
    //
    // `drain_signals` still does not write anything: it is lock-only and runs on the async
    // supervisor task, where blocking I/O does not belong (module invariant, research R2). It hands
    // the changes back instead, and the supervisor persists them in its blocking hop. This test is
    // the seam between those two halves, and it is the one that can be asserted without a catalog.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
    ));
    let session = register_emitter(&state, id, r"\033]0;\342\240\213 Fixing the parser\007");

    let mut observed: Vec<(SessionId, String)> = Vec::new();
    let landed = wait_until(Duration::from_secs(5), || {
        observed.extend(state.drain_signals().names);
        !observed.is_empty()
    });
    assert!(
        landed,
        "the observed title must be handed back to the caller"
    );
    assert_eq!(
        observed,
        vec![(id, "Fixing the parser".to_string())],
        "the value handed back is the *stripped* title — exactly what the row displays, so the \
         record and the screen cannot diverge by construction (research R1). The raw title would \
         write a spinner glyph into the catalog and make the persisted name flicker between glyph \
         frames, turning a once-per-conversation write into a once-per-tick one"
    );

    // C10: reported once. The supervisor ticks every 250ms for the life of the session, and the
    // title outlives the change — so a drain that re-reported an unchanged title would rewrite the
    // file holding every one of that project's session records four times a second.
    for _ in 0..3 {
        assert!(
            state.drain_signals().names.is_empty(),
            "a title that has not changed is not a change"
        );
    }

    session.kill().expect("kill");
}

#[test]
#[cfg_attr(
    windows,
    ignore = "first run on Windows in #332: PowerShell's own startup title is recorded as the name; fix in the #332 review follow-up"
)]
fn an_ai_clis_own_startup_title_is_not_a_name() {
    // Feature 029, T025 (FR-004, US3/AC2). Both CLIs title the terminal with their own product name
    // before any conversation exists — observed 2026-09-13 against `claude` 2.1.270 (`"✳ Claude
    // Code"`) and `copilot` (`"GitHub Copilot"`). That is not the session's name: recording it
    // would show "Claude Code" on a session that has never been named, after every restart, and
    // `recover_session_names` would then skip the session as already `Named` (contract C15) and
    // never find the real one.
    //
    //
    // `pi` has no fixed startup title: it titles the terminal `π - <folder>` while the conversation
    // has no name and `π - <name> - <folder>` once it has one (`interactive-mode.js`,
    // `updateTerminalTitle`). The folder is the session's working directory, so neither the
    // unnamed title nor the named one's decoration is the name.
    //
    // The real title follows the placeholder, so the test cannot pass by observing nothing.
    for cli in [AiCli::ClaudeCode, AiCli::Copilot, AiCli::Pi] {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        let folder = project.path().file_name().unwrap().to_str().unwrap();
        let (placeholder, named) = match cli {
            AiCli::ClaudeCode => (
                r"\342\234\263 Claude Code".to_string(),
                "Fixing the parser".to_string(),
            ),
            AiCli::Copilot => (
                "GitHub Copilot".to_string(),
                "Fixing the parser".to_string(),
            ),
            AiCli::Pi => (
                format!("π - {folder}"),
                format!("π - Fixing the parser - {folder}"),
            ),
        };
        let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
        let state = DaemonState::new(catalog_with_session(project.path(), store.path(), cli));
        let session = register_emitter(
            &state,
            id,
            &format!(r"\033]0;{placeholder}\007'; sleep 0.5; printf '\033]0;{named}\007"),
        );

        let mut observed: Vec<(SessionId, String)> = Vec::new();
        let mut shown: Vec<SessionLabel> = Vec::new();
        let landed = wait_until(Duration::from_secs(5), || {
            observed.extend(state.drain_signals().names);
            shown.push(summary_of(&state, id).title);
            observed.iter().any(|(_, name)| name == "Fixing the parser")
        });
        assert!(landed, "{cli:?}: the conversation's title must still land");
        assert_eq!(
            observed,
            vec![(id, "Fixing the parser".to_string())],
            "{cli:?}: the CLI's startup title must never be handed back for recording"
        );
        assert!(
            shown.iter().all(|t| matches!(t, SessionLabel::Pending)
                || *t == SessionLabel::Named("Fixing the parser".into())),
            "{cli:?}: an unnamed session reads \"New session\" until it has a name, got {shown:?}"
        );

        session.kill().expect("kill");
    }
}

#[test]
#[cfg_attr(
    windows,
    ignore = "first run on Windows in #332: PowerShell's own startup title is recorded as the name; fix in the #332 review follow-up"
)]
fn a_shell_tabs_title_never_becomes_the_sessions_name() {
    // Feature 029, T026 (FR-011). A session's name is its conversation's. A shell tab opened on
    // that session is a different process with its own title — bash's default `PS1` sets
    // `"user@host: ~/dir"` under the `TERM=xterm-256color` the daemon exports — and attaching it
    // must not record that over the name, where it would survive every restart.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
    ));
    let primary = register_emitter(&state, id, r"\033]0;\342\240\213 Fixing the parser\007");

    let mut observed: Vec<(SessionId, String)> = Vec::new();
    assert!(
        wait_until(Duration::from_secs(5), || {
            observed.extend(state.drain_signals().names);
            !observed.is_empty()
        }),
        "precondition: the conversation's title lands"
    );

    let instance = ShellInstanceId(1);
    state.open_shell(id, instance).expect("open a shell tab");
    let (shell, _) = state
        .attach_process(id, SessionProcess::Shell(instance))
        .expect("attach the shell tab");
    // The echoed input line holds `t026_$((1+1))`; only the *output* holds `t026_2`, so seeing it
    // proves the title escape before it was written by the shell rather than merely typed.
    state.session_input(
        id,
        0,
        b"printf '\\033]0;user@host: ~/proj\\007'; echo t026_$((1+1))\n",
    );
    let ran = wait_until(Duration::from_secs(5), || {
        observed.extend(state.drain_signals().names);
        visible_text(&shell).contains("t026_2")
    });
    assert!(ran, "precondition: the shell tab emitted its title");
    // One more tick after the output, so a drain that would pick the title up has had its chance.
    std::thread::sleep(Duration::from_millis(100));
    observed.extend(state.drain_signals().names);

    assert_eq!(
        observed,
        vec![(id, "Fixing the parser".to_string())],
        "only the AI CLI's title is the session's name"
    );
    assert_eq!(
        summary_of(&state, id).title,
        SessionLabel::Named("Fixing the parser".into())
    );

    for p in state.remove_session(id) {
        let _ = p.kill();
    }
    drop(primary);
}

#[test]
fn a_regular_terminal_sessions_shell_title_is_not_a_name() {
    // Feature 029, T026 (FR-011). A Regular Terminal session's primary process *is* a shell, so
    // "only the primary" is not enough on its own: there is no conversation to take a name from.
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let state = DaemonState::new(catalog_with_session_in_mode(
        project.path(),
        store.path(),
        AiCli::ClaudeCode,
        TerminalMode::Regular,
    ));
    // The title, then a spinner-titled marker: the spinner is activity evidence whatever the mode,
    // so `Working` proves the emitter's output was drained.
    let session = register_emitter(
        &state,
        id,
        r"\033]0;user@host: ~/proj\007'; sleep 0.3; printf '\033]0;\342\240\213 building\007",
    );

    let mut observed: Vec<(SessionId, String)> = Vec::new();
    let drained = wait_until(Duration::from_secs(5), || {
        observed.extend(state.drain_signals().names);
        summary_of(&state, id).activity == ActivitySignal::Working
    });
    assert!(drained, "precondition: the emitter's output was drained");
    assert!(
        observed.is_empty(),
        "a shell's title is not a session name, got {observed:?}"
    );
    assert_eq!(summary_of(&state, id).title, SessionLabel::Pending);

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

#[test]
fn an_event_log_append_pushes_the_new_badge_to_connected_clients() {
    // T086 (SC-005, FR-018). The state half of this is already proven above — an append reaches the
    // machine and the *snapshot* changes. What no test asked was whether anyone is **told**, and the
    // answer was no: the tail's callback discarded `note_activity`'s `bool`, and unlike the hook
    // receiver (`hooks.rs`) and the supervisor tick (`drain_signals`) there is nothing behind it to
    // push. A client that does not re-attach kept the old badge until some unrelated broadcast
    // happened to run — which is what §B measured at the display: eight frames spanning 0.24 s to
    // 0.76 s after the prompt, byte-identical, and identical again long after the reply finished.
    //
    // So the assertion is deliberately about the *push* and not about the state: reading
    // `catalog_snapshot()` here would pass against the defect.
    use micold_core::protocol::codec::Frame;
    use micold_core::protocol::messages::DaemonMsg;

    let home = CopilotHome::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));

    // Empty first: the tail starts at the file's current end, so a line written before the watch
    // exists is never seen.
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
    let session = register_cat(&state, id);
    let (_client, mut rx) = state.register(micold_core::protocol::messages::ClientIdentity::new(
        "test",
        micold_core::protocol::messages::ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    state.open_event_log_tail(id);

    let append = |line: &str| {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&events)
            .unwrap();
        writeln!(f, "{line}").unwrap();
    };
    // Every `CatalogChanged` this client has been sent, as the badge it carried for our session.
    let badges = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<Frame<DaemonMsg>>| {
        let mut seen = Vec::new();
        while let Ok(frame) = rx.try_recv() {
            if let Frame::Control(DaemonMsg::CatalogChanged { catalog }) = frame {
                if let Some(summary) = catalog
                    .projects
                    .iter()
                    .flat_map(|p| &p.sessions)
                    .find(|s| s.id == id)
                {
                    seen.push(summary.activity.clone());
                }
            }
        }
        seen
    };

    // One append that changes the signal: the client must hear about it, with nothing else running.
    // No `drain_signals`, no supervisor tick, no start — the tail is the only thing that can speak.
    append(r#"{"type":"user.message","data":{}}"#);
    let mut pushed = Vec::new();
    // Generous against SC-005's one-second budget on purpose: what is under test is that a push
    // happens at all, and a deadline tight enough to be the *measurement* would fail on a loaded
    // runner for a reason that is not the defect.
    assert!(
        wait_until(Duration::from_secs(10), || {
            pushed.extend(badges(&mut rx));
            !pushed.is_empty()
        }),
        "an event-log append that moves the badge must reach connected clients"
    );
    assert_eq!(
        pushed,
        vec![ActivitySignal::Working],
        "and it must carry the new signal"
    );

    // An event the machine ignores must **not** push. `tool.execution_complete` is a no-op from
    // Working (the model is still going), so an unconditional broadcast beside `note_activity`
    // would send a snapshot nobody needs — the receiver's `if changed` is what this pins down.
    // Ordering makes the absence testable: the no-op is followed by a change, and if the tail has
    // delivered the second it has delivered the first.
    append(r#"{"type":"tool.execution_complete","data":{}}"#);
    append(r#"{"type":"assistant.turn_end","data":{}}"#);
    assert!(
        wait_until(Duration::from_secs(10), || {
            pushed.extend(badges(&mut rx));
            pushed.contains(&ActivitySignal::AwaitingInput)
        }),
        "the turn end must reach the client too"
    );
    assert_eq!(
        pushed,
        vec![ActivitySignal::Working, ActivitySignal::AwaitingInput],
        "the no-op in between must push nothing"
    );

    session.kill().expect("kill");
}

/// A private `PI_CODING_AGENT_DIR` for the duration of one test, under the same lock as
/// [`CopilotHome`]: both are process-global, and the two must never be held at once.
struct PiHome {
    dir: tempfile::TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl PiHome {
    fn new() -> Self {
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("PI_CODING_AGENT_DIR", dir.path());
        Self { dir, _guard: guard }
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }
}

impl Drop for PiHome {
    fn drop(&mut self) {
        std::env::remove_var("PI_CODING_AGENT_DIR");
    }
}

#[test]
fn a_pi_activity_log_moves_the_badge_through_the_unchanged_machine() {
    // Feature 029, T037 (SC-005). The component's log is read by the tail `copilot` already uses and
    // fed to the same `Activity` machine; what is new is only where the log is and how a line maps.
    // So this drives the log **alone** — a `cat` sink, no title traffic — and watches the badge.
    let home = PiHome::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));

    let provider = AiCli::Pi.provider();
    let ActivitySource::Extension { log } =
        provider.activity_source(home.path(), project.path(), id.0)
    else {
        panic!("a Pi session's activity arrives through its component's log");
    };
    // Deliberately *not* created: the daemon creates the log's directory itself, because at spawn
    // nothing has written there yet. Only the directory is the daemon's; the file is the component's to start.

    let state = Arc::new(DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::Pi,
    )));
    let session = register_cat(&state, id);
    state.open_event_log_tail(id);
    assert!(
        log.parent().is_some_and(|dir| dir.is_dir()),
        "opening the tail prepared the directory the component will write into"
    );
    assert_eq!(summary_of(&state, id).activity, ActivitySignal::Unknown);

    let append = |kind: &str| {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .unwrap();
        writeln!(f, r#"{{"type":"{kind}","at":"2026-09-13T10:00:00.000Z"}}"#).unwrap();
    };

    append("turn_start");
    assert!(
        wait_until(Duration::from_secs(10), || summary_of(&state, id).activity
            == ActivitySignal::Working),
        "turn_start must reach the machine and read as working"
    );

    append("tool_execution_end");
    append("agent_settled");
    assert!(
        wait_until(Duration::from_secs(10), || summary_of(&state, id).activity
            == ActivitySignal::AwaitingInput),
        "agent_settled must put the session on its user"
    );

    session.kill().expect("kill");
}

#[test]
fn with_the_component_declined_a_pi_session_is_not_watched_and_reads_unknown() {
    // Feature 029, T045 (FR-012e, FR-012f). The provider still reports `Extension` — it is pure and
    // knows nothing of the switch — so the daemon is the one place that declines, and it declines
    // by not opening a tail. A line in the log is then not evidence the session produced, and the
    // badge stays `Unknown` rather than moving on something nobody asked to be reported.
    let home = PiHome::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));

    let ActivitySource::Extension { log } =
        AiCli::Pi
            .provider()
            .activity_source(home.path(), project.path(), id.0)
    else {
        panic!("the provider reports the same source whatever the switch says");
    };

    let state = Arc::new(DaemonState::new(catalog_with_session(
        project.path(),
        store.path(),
        AiCli::Pi,
    )));
    state
        .set_pi_activity_component(false)
        .expect("the switch persists");
    let session = register_cat(&state, id);
    state.open_event_log_tail(id);

    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    std::fs::write(
        &log,
        "{\"type\":\"turn_start\",\"at\":\"2026-09-13T10:00:00.000Z\"}\n",
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(750));
    assert_eq!(
        summary_of(&state, id).activity,
        ActivitySignal::Unknown,
        "no tail was opened, so nothing moved the badge"
    );

    session.kill().expect("kill");
}

/// One supervisor tick, as far as a session's name goes: drain the terminal signals, record the
/// names they carried, and look again for the names of sessions that gave a reason to.
fn tick(state: &DaemonState) {
    let drained = state.drain_signals();
    state.record_observed_names(&drained.names);
    state.recover_live_session_names();
}

/// Append one line to a file, creating it and its directory.
fn append_line(path: &std::path::Path, line: &str) {
    use std::io::Write as _;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(f, "{line}").unwrap();
}

#[test]
#[cfg_attr(
    windows,
    ignore = "first run on Windows in #332: fails with an invalid filename (os error 123); fix in the #332 review follow-up"
)]
fn a_running_pi_row_reads_its_first_message_until_it_is_named() {
    // Feature 029 (Pi), FR-011, quickstart §B finding 1. Pi's terminal title carries a name only
    // once `/name` has run, so the terminal alone takes a running row from the placeholder
    // straight to the name. The first message, which Pi has recorded by the end of the first
    // turn, must show in between — without waiting for a refresh to recover it.
    let home = PiHome::new();
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("demo");
    std::fs::create_dir(&project).unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = SessionId::from_uuid(Uuid::from_u128(SESSION_U128));
    let named = root.path().join(".named");

    let state = Arc::new(DaemonState::new(catalog_with_session(
        &project,
        store.path(),
        AiCli::Pi,
    )));
    // Pi's own two titles, in order: the second only once `/name` has run. `π` is U+03C0.
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(format!(
        r"printf '\033]0;\317\200 - demo\007'; while [ ! -e '{}' ]; do sleep 0.05; done; printf '\033]0;\317\200 - my task - demo\007'; sleep 10",
        named.display()
    ));
    cmd.cwd(std::env::temp_dir());
    let session = state.register_session(
        PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn emitter session"),
    );

    assert!(
        wait_until(Duration::from_secs(5), || session
            .signals()
            .title()
            .is_some()),
        "the terminal title must have been emitted, or this proves nothing"
    );
    for _ in 0..3 {
        tick(&state);
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        summary_of(&state, id).title,
        SessionLabel::Pending,
        "before any message Pi has recorded nothing, so the row is the placeholder"
    );

    // The first exchange lands in Pi's store, and the turn moving is what says so.
    let encoded = format!(
        "--{}--",
        project
            .to_string_lossy()
            .trim_start_matches('/')
            .replace('/', "-")
    );
    let conversation = home
        .path()
        .join("sessions")
        .join(encoded)
        .join(format!("2026-09-14T10-00-00-000Z_{}.jsonl", id.0));
    append_line(
        &conversation,
        &format!(
            r#"{{"type":"session","version":3,"id":"{}","timestamp":"2026-09-14T10:00:00.000Z","cwd":"{}"}}"#,
            id.0,
            project.display()
        ),
    );
    append_line(
        &conversation,
        r#"{"type":"message","id":"1","parentId":null,"timestamp":"2026-09-14T10:00:01.000Z","message":{"role":"user","content":"why is the row wrong"}}"#,
    );
    state.note_activity(id, ActivityEvent::Hook(HookKind::UserPromptSubmit));
    state.note_activity(id, ActivityEvent::Hook(HookKind::Stop));
    assert!(
        wait_until(Duration::from_secs(5), || {
            tick(&state);
            summary_of(&state, id).title == SessionLabel::Named("why is the row wrong".into())
        }),
        "with no name recorded, the running row reads the first message; got {:?}",
        summary_of(&state, id).title
    );

    // `/name`: Pi records the name and retitles its terminal.
    append_line(
        &conversation,
        r#"{"type":"session_info","id":"2","parentId":"1","timestamp":"2026-09-14T10:00:02.000Z","name":"my task"}"#,
    );
    std::fs::write(&named, "").unwrap();
    assert!(
        wait_until(Duration::from_secs(5), || {
            tick(&state);
            summary_of(&state, id).title == SessionLabel::Named("my task".into())
        }),
        "then the name wins; got {:?}",
        summary_of(&state, id).title
    );

    session.kill().expect("kill");
}

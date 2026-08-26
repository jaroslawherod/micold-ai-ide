//! T027 [US1] — a daemon PTY session keeps running and producing output with no client reading it
//! (FR-001).
//!
//! This is the core persistence property at the supervisor layer: [`PtySession`] owns the child and
//! the VT `Term`, and a per-session reader thread pumps output into the grid **whether or not**
//! anything consumes it. Wiring a session to a client connection lands in Phase 4; there is
//! deliberately no connection here — which is exactly the point: output continues with no viewer.
//!
//! T045a [US2] adds the other half of survival, one layer up and on a longer timescale (FR-012):
//! the daemon **process** goes away, and what comes back has to be the same session on the same AI
//! CLI. The first test is survival with nothing watching; the second is survival with nothing
//! *running*, where the only thing that crosses the gap is what was written to disk.

#![cfg(unix)]

use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::session::SessionId;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// The visible screen as text (rows joined by newlines), for content assertions.
fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

/// Count how many `tick` lines are currently on screen — a coarse "how much has been produced".
fn tick_count(session: &PtySession) -> usize {
    visible_text(session).matches("tick").count()
}

/// Poll `cond` until true or `timeout` elapses; returns whether it became true.
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

#[test]
fn a_session_keeps_producing_output_with_no_client_reading() {
    // A child that emits a line ~20×/s forever. Nothing attaches to consume it.
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("while true; do echo tick; sleep 0.05; done");

    let session =
        PtySession::spawn(SessionId::new(), cmd, 10_000, Some((80, 24))).expect("spawn session");

    // The child is running and the grid is receiving output — with no client attached.
    assert!(session.is_alive(), "child should be running");
    assert!(
        wait_until(Duration::from_secs(5), || tick_count(&session) >= 1),
        "grid should receive output from the unattended child"
    );

    // Let time pass with still no consumer, then confirm output kept flowing and the child lived.
    let before = tick_count(&session);
    assert!(
        wait_until(Duration::from_secs(5), || tick_count(&session) > before
            || before >= 20),
        "output must keep arriving while detached (before={before})"
    );
    assert!(
        session.is_alive(),
        "the child must outlive the absence of any client (FR-001)"
    );

    // Test-owned process: stop it so nothing leaks.
    session.kill().expect("kill session");
    assert!(
        wait_until(Duration::from_secs(5), || !session.is_alive()),
        "killed child should be reaped"
    );
}

// ---------------------------------------------------------------------------------------
// T045a [US2] — a session survives the daemon itself, on the CLI it was started on (FR-012)
// ---------------------------------------------------------------------------------------

/// A Copilot session outlives the daemon that created it, comes back as a *Copilot* session, and is
/// offered for resume because **Copilot's** store is the one consulted about it.
///
/// # The leg this covers, and why the neighbours do not
///
/// `micold-core`'s store round-trip (T036) proves a `Session` carrying `AiCli::Copilot` survives
/// serialisation, and `catalog_adoption.rs` proves a catalog reloads one. Both start from a
/// `projects.json` a *test* hand-wrote. `set_wide_provider_decisions.rs` proves the startup
/// presentation asks each session's own provider — again from a hand-written store.
///
/// Nothing asserted that the **daemon's own write** carries the provider. `create_session` is what
/// runs when a user clicks "+", and if it recorded the choice only in memory — or the save dropped
/// it — every test above would still be green, and the session would come back from a restart as a
/// Claude session: `claude` would find no conversation under an id it has never seen, the startup
/// pass would leave it `Idle` (indistinguishable from "created and never used"), and the next start
/// would be a **fresh** `--session-id`, beginning a new conversation under the old session's
/// identity. That is the failure FR-012 is about, and it lives in the gap between those files.
///
/// # What is modelled, and what is not
///
/// The restart is modelled as it is elsewhere in this suite: the `DaemonState` is dropped and a new
/// one loaded from the same store paths, so nothing but the files crosses the gap. It does not fork
/// a second `micold-daemon` binary — `daemon_singleton.rs` and `version_recovery.rs` own that, and
/// neither needs an AI CLI installed to do it.
///
/// The resume is asserted as argv, not as a spawn. This suite deliberately never spawns either CLI
/// (`session_start.rs` states why: CI runners have neither installed, and a test that needed both
/// would tie the suite to two vendors' installers). What is asserted here is the argv the
/// *reloaded* record implies, through the same `terminal::launch_args` the spawn itself calls.
#[test]
fn a_copilot_session_survives_a_daemon_restart_on_the_cli_it_was_started_on() {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use micold_core::project::{Availability, Project};
    use micold_core::protocol::messages::WireLifecycle;
    use micold_core::session::AiCli;
    use micold_core::settings::JsonFileSettingsStore;
    use micold_core::store::{JsonFileStore, ProjectStore};
    use micold_core::terminal::{launch_args, LaunchMode, LaunchSpec};
    use micold_core::workspace::Workspace;
    use micold_daemon::catalog::Catalog;
    use micold_daemon::state::DaemonState;

    let project = PathBuf::from("/repo/alpha");
    let store = tempfile::tempdir().unwrap();
    let projects_path = store.path().join("projects.json");
    let settings_path = store.path().join("settings.json");

    // Scratch provider stores. `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` are process-global, and no
    // other test in this binary reads either — the PTY test above spawns `sh`.
    let homes = tempfile::tempdir().unwrap();
    let claude_home = homes.path().join("claude");
    let copilot_home = homes.path().join("copilot");
    std::env::set_var("CLAUDE_CONFIG_DIR", &claude_home);
    std::env::set_var("COPILOT_HOME", &copilot_home);

    let load = |projects: PathBuf, settings: PathBuf| {
        Catalog::load(
            Box::new(JsonFileStore::at(projects)),
            Box::new(JsonFileSettingsStore::at(settings)),
        )
    };

    // --- The daemon that creates the sessions ---
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(project.clone(), true, Availability::Available)],
            active: Some(project.clone()),
            sessions: BTreeMap::new(),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();

    let (copilot_id, claude_id) = {
        let state = DaemonState::new(load(projects_path.clone(), settings_path.clone()));
        let copilot_id = state
            .create_session(&project, "feat-x", AiCli::Copilot)
            .expect("create must succeed");
        let claude_id = state
            .create_session(&project, "", AiCli::ClaudeCode)
            .expect("create must succeed");
        (copilot_id, claude_id)
        // …and the daemon goes away here. Everything below reads the files it left behind.
    };

    // Each CLI records a conversation for its own session, in its own documented layout. Written
    // directly rather than through the seam, which only ever reads.
    std::fs::create_dir_all(
        copilot_home
            .join("session-state")
            .join(copilot_id.0.to_string()),
    )
    .unwrap();
    std::fs::write(
        copilot_home
            .join("session-state")
            .join(copilot_id.0.to_string())
            .join("events.jsonl"),
        "{}\n",
    )
    .unwrap();
    let claude_cwd_key: String = project
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    std::fs::create_dir_all(claude_home.join("projects").join(&claude_cwd_key)).unwrap();
    std::fs::write(
        claude_home
            .join("projects")
            .join(&claude_cwd_key)
            .join(format!("{}.jsonl", claude_id.0)),
        "{}\n",
    )
    .unwrap();

    // --- The daemon that comes back ---
    let restarted = DaemonState::new(load(projects_path, settings_path));

    assert_eq!(
        restarted.present_interrupted_resumable_at_startup(),
        2,
        "each session was asked about with the CLI it was created on, so both are offered again. \
         Had the create-time provider not reached disk, the Copilot session would have reloaded as \
         a Claude one, `claude` would have found no conversation for it, and it would have stayed \
         `Idle` — created-and-never-used, which is not what it is"
    );

    let summaries = restarted.sessions_for(Path::new(&project));
    let summary = |id| {
        summaries
            .iter()
            .find(|s: &&micold_core::protocol::messages::SessionSummary| s.id == id)
            .expect("session survived the restart")
    };
    assert_eq!(
        summary(copilot_id).provider,
        AiCli::Copilot,
        "the daemon wrote which CLI it started, and a restart reads it back rather than defaulting"
    );
    assert_eq!(summary(claude_id).provider, AiCli::ClaudeCode);
    assert_eq!(
        summary(copilot_id).lifecycle,
        WireLifecycle::InterruptedResumable
    );
    assert_eq!(
        summary(claude_id).lifecycle,
        WireLifecycle::InterruptedResumable
    );

    // And the resume the reloaded record implies is Copilot's own — its conversation, not a fresh
    // one, and in Copilot's argv form rather than `claude`'s.
    let argv = |id: micold_core::session::SessionId, provider: AiCli| {
        launch_args(&LaunchSpec {
            cwd: project.clone(),
            session_id: id.0,
            provider,
            mode: LaunchMode::Resume,
            env: Vec::new(),
        })
    };
    assert_eq!(
        argv(copilot_id, summary(copilot_id).provider),
        vec![
            format!("--resume={}", copilot_id.0),
            "--no-remote".to_string()
        ],
    );
    assert_eq!(
        argv(claude_id, summary(claude_id).provider),
        vec!["--resume".to_string(), claude_id.0.to_string()],
        "and the neighbour still resumes in `claude`'s two-argument form — the records did not \
         converge on one CLI on the way through the store"
    );
}

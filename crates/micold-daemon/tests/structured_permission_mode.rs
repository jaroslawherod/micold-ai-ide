//! #942 milestone M4 (T041): a structured session's permission mode is chosen on `SessionCreate`,
//! listed in its summary, launched with (`--permission-mode` only when not Default) and kept across
//! a restart and a daemon restart (US2 scenarios 1–3, FR-013, FR-015, SC-005). A terminal session
//! cannot be given one, and its launch and binding carry none (FR-018).
//!
//! Milestone M5 (T044, T045): `SetPermissionMode` changes a running session's mode through a
//! stream-json control request, kept only when the CLI answers `success` (US2 scenario 5, FR-017),
//! and the mode follows what the CLI reports in its `init` and `status` lines (FR-016). The cases
//! that need a permission request (one pending across a change, one raised after it) are in
//! `structured_permission.rs`, beside its prompt-tool harness.
//!
//! The stand-in of `support/structured.rs` records its argv in the session's folder. A conversation
//! is "recorded" as `structured_resume.rs` records one: a transcript file in `claude`'s layout under
//! `CLAUDE_CONFIG_DIR`, which this binary points at a directory of its own, once.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use micold_core::occupancy::Additional;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult, SessionSummary};
use micold_core::session::{AiCli, ModeRefusal, PermissionMode, SessionId, SessionKind};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::{Catalog, SetModeError};
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use conn::{answer, connect, Client};
use history::{ai_session, service};
use structured::{clear_records, fake_stream_cli, record, stream_script};

/// Arguments that change a CLI's permission configuration; a terminal session carries none.
const PERMISSION_FLAGS: [&str; 4] = [
    "--permission-mode",
    "--allow-dangerously-skip-permissions",
    "--dangerously-skip-permissions",
    "--permission-prompt-tool",
];

/// The stand-in on `PATH` and `CLAUDE_CONFIG_DIR` at a directory of this process's, set once.
fn setup() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    fake_stream_cli();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("micold-942-mode-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("a claude config directory");
        std::env::set_var("CLAUDE_CONFIG_DIR", &home);
        home
    })
}

/// Record `id`'s conversation in `cwd` in `claude`'s layout, so a restart resumes it.
fn record_conversation(cwd: &Path, id: SessionId) {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let file = setup()
        .join("projects")
        .join(key)
        .join(format!("{}.jsonl", id.0));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, "{}\n").unwrap();
}

/// The argv of the stand-in's latest start in `dir`, once it recorded one.
fn argv(dir: &Path) -> Vec<String> {
    record(dir, "argv-record")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The mode `argv` names with `--permission-mode`, `None` when it names none; never twice.
fn launched_mode(argv: &[String]) -> Option<String> {
    let named: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--permission-mode")
        .map(|w| &w[1])
        .collect();
    assert!(named.len() <= 1, "one mode at most: {argv:#?}");
    named.first().map(|m| m.to_string())
}

/// What a launch in `mode` names: nothing for Default (FR-013), else the CLI's name of it.
fn expected(mode: PermissionMode) -> Option<String> {
    (!mode.is_default()).then(|| mode.cli_name().to_owned())
}

fn resumes(argv: &[String], id: SessionId) -> bool {
    argv.windows(2)
        .any(|w| w[0] == "--resume" && w[1] == id.0.to_string())
}

fn create(req: u64, project: &Path, kind: SessionKind, mode: PermissionMode) -> ClientMsg {
    ClientMsg::SessionCreate {
        req,
        project: project.to_path_buf(),
        worktree_dir: String::new(),
        provider: AiCli::ClaudeCode,
        allow_additional_session: true,
        kind,
        permission_mode: mode,
    }
}

async fn send(client: &mut Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.expect("sent");
}

async fn created(client: &mut Client, req: u64) -> SessionId {
    match answer(client, req).await {
        DaemonMsg::OperationOk {
            result: OperationResult::SessionCreated { session },
            ..
        } => session,
        other => panic!("not created: {other:?}"),
    }
}

fn summary(state: &DaemonState, project: &Path, id: SessionId) -> SessionSummary {
    state
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .expect("listed")
}

/// A daemon over the catalog saved in `store`, as the service loads it at its start.
fn daemon(store: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(load_catalog(store)))
}

fn load_catalog(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A saved catalog in `store` with `project` and no session yet.
fn save_project(store: &Path, project: &Path) {
    JsonFileStore::at(store.join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(
                project.to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project.to_path_buf()),
            sessions: BTreeMap::new(),
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
}

/// US2 scenarios 1–3, SC-005 (launch, restart): each mode chosen on `SessionCreate` is in the
/// summary and on the launch (`--permission-mode` only when not Default), and again on the
/// `SessionRestart` that resumes it.
#[tokio::test(flavor = "multi_thread")]
async fn each_mode_on_create_is_listed_launched_and_kept_across_a_restart() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let state = service(dir, vec![]);
    stream_script(dir, "");
    let mut client = connect(&state).await;
    for (n, mode) in (0u64..).zip(PermissionMode::ALL) {
        clear_records(dir);
        send(
            &mut client,
            create(n * 2 + 1, dir, SessionKind::Structured, mode),
        )
        .await;
        let id = created(&mut client, n * 2 + 1).await;
        assert_eq!(summary(&state, dir, id).permission_mode, mode, "listed");
        let launch = argv(dir);
        assert_eq!(
            launched_mode(&launch),
            expected(mode),
            "{mode:?}: {launch:#?}"
        );

        record_conversation(dir, id);
        state.stop_session(id);
        clear_records(dir);
        send(
            &mut client,
            ClientMsg::SessionRestart {
                session: id,
                allow_additional_session: true,
            },
        )
        .await;
        let relaunch = argv(dir);
        assert!(resumes(&relaunch, id), "{mode:?} resumed: {relaunch:#?}");
        assert_eq!(
            launched_mode(&relaunch),
            expected(mode),
            "{mode:?} restarted: {relaunch:#?}"
        );
        assert_eq!(
            summary(&state, dir, id).permission_mode,
            mode,
            "still listed"
        );
        state.stop_session(id);
    }
}

/// US2 scenario 3, SC-005 (daemon restart), FR-015: a daemon restarted over the saved catalog lists
/// each session in its mode and resumes it in that mode.
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_restart_lists_and_resumes_each_session_in_its_mode() {
    setup();
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    save_project(store.path(), dir);
    let ids: Vec<(PermissionMode, SessionId)> = {
        let state = daemon(store.path());
        PermissionMode::ALL
            .into_iter()
            .map(|mode| {
                let id = state
                    .create_session_in_mode(
                        dir,
                        "",
                        AiCli::ClaudeCode,
                        SessionKind::Structured,
                        mode,
                        Additional::Yes,
                    )
                    .expect("created");
                record_conversation(dir, id);
                (mode, id)
            })
            .collect()
        // …and the daemon goes away here, before any of them ran.
    };

    let restarted = daemon(store.path());
    restarted.present_interrupted_resumable_at_startup();
    stream_script(dir, "");
    let mut client = connect(&restarted).await;
    for (mode, id) in ids {
        assert_eq!(summary(&restarted, dir, id).permission_mode, mode, "listed");
        clear_records(dir);
        send(
            &mut client,
            ClientMsg::SessionStart {
                session: id,
                allow_additional_session: true,
            },
        )
        .await;
        let launch = argv(dir);
        assert!(resumes(&launch, id), "{mode:?} resumed: {launch:#?}");
        assert_eq!(
            launched_mode(&launch),
            expected(mode),
            "{mode:?}: {launch:#?}"
        );
        restarted.stop_session(id);
    }
}

/// FR-018: a terminal `SessionCreate` with a mode other than Default is refused with the
/// `ModeRefusal` sentence and writes no record; with Default (or none) it is created as before.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_session_create_with_a_mode_is_refused_and_not_recorded() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let state = service(dir, vec![]);
    let mut client = connect(&state).await;
    for (req, mode) in (1u64..).zip(&PermissionMode::ALL[1..]) {
        send(&mut client, create(req, dir, SessionKind::Terminal, *mode)).await;
        match answer(&mut client, req).await {
            DaemonMsg::OperationError { message, .. } => {
                assert_eq!(message, ModeRefusal::NotStructured.to_string(), "{mode:?}")
            }
            other => panic!("{mode:?} refused: {other:?}"),
        }
    }
    assert!(state.sessions_for(dir).is_empty(), "no record");

    stream_script(dir, "");
    send(
        &mut client,
        create(9, dir, SessionKind::Terminal, PermissionMode::Default),
    )
    .await;
    let id = created(&mut client, 9).await;
    assert_eq!(
        summary(&state, dir, id).permission_mode,
        PermissionMode::Default
    );
    state.stop_session(id);
}

/// FR-018: a bound terminal session's launch carries no permission argument, and its binding file
/// keeps its own tool-call timeout (the structured binding's unbounded one is not used).
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_sessions_launch_and_binding_carry_no_permission_setting() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let session = ai_session();
    let id = session.id;
    let state = service(dir, vec![session]);
    let bindings = tempfile::tempdir().unwrap();
    mcp::serve_tool_server(&state, bindings.path().to_path_buf()).await;
    state.set_permission_prompts(true);
    stream_script(dir, "");
    let starting = Arc::clone(&state);
    tokio::task::spawn_blocking(move || starting.start_session(id, LaunchMode::Fresh))
        .await
        .unwrap()
        .expect("starts");
    let launch = argv(dir);
    for flag in PERMISSION_FLAGS {
        assert!(!launch.iter().any(|a| a == flag), "no {flag}: {launch:#?}");
    }
    let config_at = launch
        .iter()
        .position(|a| a == "--mcp-config")
        .expect("bound");
    let binding: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&launch[config_at + 1]).unwrap()).unwrap();
    assert_ne!(
        binding["mcpServers"]["micold"]["timeout"],
        serde_json::json!(2_147_483_647u64),
        "a terminal binding keeps its own timeout, not the structured one (R3): {binding}"
    );
    let stopping = Arc::clone(&state);
    tokio::task::spawn_blocking(move || stopping.stop_session(id))
        .await
        .unwrap();
}

/// Data-model *PermissionMode*: `Catalog::set_permission_mode` persists a structured session's
/// mode, refuses a terminal session's with `ModeRefusal::NotStructured`, and names an unknown id.
#[test]
fn the_catalog_sets_a_structured_sessions_mode_and_refuses_a_terminal_one() {
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    save_project(store.path(), dir);
    let mut catalog = load_catalog(store.path());
    let structured = catalog
        .create_session_of_kind(
            dir,
            "",
            AiCli::ClaudeCode,
            SessionKind::Structured,
            Additional::Yes,
        )
        .expect("structured created");
    let terminal = catalog
        .create_session(dir, "", AiCli::ClaudeCode, Additional::Yes)
        .expect("terminal created");

    assert!(matches!(
        catalog.set_permission_mode(structured, PermissionMode::Bypass),
        Ok(true)
    ));
    assert!(
        matches!(
            catalog.set_permission_mode(structured, PermissionMode::Bypass),
            Ok(false)
        ),
        "an unchanged mode writes nothing"
    );
    assert!(matches!(
        catalog.set_permission_mode(terminal, PermissionMode::Plan),
        Err(SetModeError::Refused(ModeRefusal::NotStructured))
    ));
    assert!(matches!(
        catalog.set_permission_mode(SessionId::new(), PermissionMode::Plan),
        Err(SetModeError::NoSession)
    ));

    let reloaded = load_catalog(store.path());
    let mode_of = |id: SessionId| {
        reloaded
            .workspace()
            .find_session(id)
            .map(|(_, s)| s.permission_mode)
    };
    assert_eq!(
        mode_of(structured),
        Some(PermissionMode::Bypass),
        "persisted"
    );
    assert_eq!(
        mode_of(terminal),
        Some(PermissionMode::Default),
        "unchanged"
    );
}

/// A daemon over a saved catalog in `store` holding one structured session of `project` in `mode`.
struct Saved {
    store: tempfile::TempDir,
    project: tempfile::TempDir,
    state: Arc<DaemonState>,
    id: SessionId,
}

impl Saved {
    fn new(mode: PermissionMode) -> Self {
        setup();
        let store = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        save_project(store.path(), project.path());
        let state = daemon(store.path());
        micold_daemon::server::spawn_mode_reports(Arc::clone(&state));
        let id = state
            .create_session_in_mode(
                project.path(),
                "",
                AiCli::ClaudeCode,
                SessionKind::Structured,
                mode,
                Additional::Yes,
            )
            .expect("created");
        Self {
            store,
            project,
            state,
            id,
        }
    }

    fn dir(&self) -> &Path {
        self.project.path()
    }

    /// Start the session with `script`, once its stand-in recorded its argv.
    async fn start(&self, script: &str) -> Vec<String> {
        clear_records(self.dir());
        stream_script(self.dir(), script);
        let (state, id) = (Arc::clone(&self.state), self.id);
        tokio::task::spawn_blocking(move || state.start_session(id, LaunchMode::Fresh))
            .await
            .unwrap()
            .expect("the structured session starts");
        argv(self.dir())
    }

    fn listed(&self) -> PermissionMode {
        summary(&self.state, self.dir(), self.id).permission_mode
    }

    /// The mode the saved catalog holds, as a daemon restarted now would load it.
    fn recorded(&self) -> PermissionMode {
        load_catalog(self.store.path())
            .workspace()
            .find_session(self.id)
            .map(|(_, s)| s.permission_mode)
            .expect("recorded")
    }

    /// Ask for `mode` on `client` and return the answer.
    async fn change(&self, client: &mut Client, req: u64, mode: PermissionMode) -> DaemonMsg {
        send(
            client,
            ClientMsg::SetPermissionMode {
                req,
                session: self.id,
                mode,
            },
        )
        .await;
        answer(client, req).await
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        self.state.stop_session(self.id);
    }
}

/// The control requests the stand-in in `dir` read on its stdin, as JSON.
fn control_requests(dir: &Path) -> Vec<serde_json::Value> {
    record(dir, "stdin-record")
        .lines()
        .filter(|line| line.contains("control_request"))
        .map(|line| serde_json::from_str(line).expect("a JSON line"))
        .collect()
}

fn refused_with(answer: &DaemonMsg) -> &str {
    match answer {
        DaemonMsg::OperationError { message, .. } => message,
        other => panic!("refused: {other:?}"),
    }
}

/// Wait up to 10 s for `done`.
fn eventually(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "{what} never happened");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// T044 (US2 s5, FR-017): on a running session `SetPermissionMode` writes the contract's control
/// line to the CLI's stdin; its `success` answer is acknowledged, persisted and broadcast.
#[tokio::test(flavor = "multi_thread")]
async fn a_running_sessions_mode_change_is_sent_to_claude_and_kept_on_success() {
    let saved = Saved::new(PermissionMode::Default);
    saved.start("control success\n").await;
    let mut client = connect(&saved.state).await;
    let mut other = connect(&saved.state).await;
    let ok = saved
        .change(&mut client, 1, PermissionMode::AcceptEdits)
        .await;
    assert!(matches!(ok, DaemonMsg::OperationOk { .. }), "{ok:?}");

    let requests = control_requests(saved.dir());
    assert_eq!(requests.len(), 1, "{requests:#?}");
    let id = requests[0]["request_id"].as_str().expect("a request id");
    assert!(id.starts_with("micold-"), "{id}");
    assert_eq!(
        requests[0],
        serde_json::json!({"type": "control_request", "request_id": id,
            "request": {"subtype": "set_permission_mode", "mode": "acceptEdits"}})
    );
    assert_eq!(saved.listed(), PermissionMode::AcceptEdits, "listed");
    assert_eq!(saved.recorded(), PermissionMode::AcceptEdits, "persisted");
    shown_in(&mut other, saved.id, PermissionMode::AcceptEdits).await;
}

/// Wait up to 15 s for a `CatalogChanged` on `client` listing `id` in `mode`.
async fn shown_in(client: &mut Client, id: SessionId, mode: PermissionMode) {
    use futures_util::StreamExt;
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Frame::Control(DaemonMsg::CatalogChanged { catalog }) =
                client.next().await.expect("stream open").unwrap()
            {
                let listed = catalog.projects.iter().flat_map(|p| &p.sessions);
                if listed
                    .into_iter()
                    .any(|s| s.id == id && s.permission_mode == mode)
                {
                    return;
                }
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no client was shown {id:?} in {mode:?}"))
}

/// T044 (FR-017): a mode change the CLI refuses leaves the mode as it was and the client is told
/// the CLI's reason.
#[tokio::test(flavor = "multi_thread")]
async fn a_mode_change_claude_refuses_leaves_the_mode_and_says_why() {
    let saved = Saved::new(PermissionMode::Plan);
    saved
        .start("control error bypass_not_launched Cannot switch to bypass now\n")
        .await;
    let mut client = connect(&saved.state).await;
    let refused = saved.change(&mut client, 1, PermissionMode::Bypass).await;
    assert_eq!(refused_with(&refused), "Cannot switch to bypass now");
    assert_eq!(saved.listed(), PermissionMode::Plan);
    assert_eq!(saved.recorded(), PermissionMode::Plan);
}

/// T044 (FR-017, data-model *ControlRequest*): a mode change the CLI never answers is refused once
/// `CONTROL_ANSWER_BOUND` (10 s) passed, and the mode stays.
#[tokio::test(flavor = "multi_thread")]
async fn a_mode_change_claude_does_not_answer_is_refused_after_ten_seconds() {
    let saved = Saved::new(PermissionMode::Default);
    saved.start("control none\n").await;
    let mut client = connect(&saved.state).await;
    let asked = Instant::now();
    send(
        &mut client,
        ClientMsg::SetPermissionMode {
            req: 1,
            session: saved.id,
            mode: PermissionMode::Plan,
        },
    )
    .await;
    let refused = answer(&mut client, 1).await;
    assert_eq!(
        refused_with(&refused),
        "Claude did not answer the mode change."
    );
    assert!(
        asked.elapsed() >= micold_daemon::structured::CONTROL_ANSWER_BOUND,
        "not before the bound: {:?}",
        asked.elapsed()
    );
    assert_eq!(saved.listed(), PermissionMode::Default);
    assert_eq!(saved.recorded(), PermissionMode::Default);
}

/// T044 (data-model *ControlRequest*): the process ending before it answers ends the wait with
/// "Claude stopped before it answered.", and the mode stays.
#[tokio::test(flavor = "multi_thread")]
async fn a_process_ending_before_it_answers_ends_the_wait() {
    let saved = Saved::new(PermissionMode::Default);
    saved.start("await-stdin\nexit 0\n").await;
    let mut client = connect(&saved.state).await;
    let asked = Instant::now();
    let refused = saved.change(&mut client, 1, PermissionMode::Plan).await;
    assert_eq!(refused_with(&refused), "Claude stopped before it answered.");
    assert!(
        asked.elapsed() < micold_daemon::structured::CONTROL_ANSWER_BOUND,
        "at the end, not at the bound: {:?}",
        asked.elapsed()
    );
    assert_eq!(saved.listed(), PermissionMode::Default);
}

/// T044 (FR-017, FR-015): a session not running has its new mode persisted at once and is
/// launched in it next.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_not_running_is_persisted_at_once_and_launched_in_its_new_mode() {
    let saved = Saved::new(PermissionMode::Default);
    let mut client = connect(&saved.state).await;
    let ok = saved.change(&mut client, 1, PermissionMode::Plan).await;
    assert!(matches!(ok, DaemonMsg::OperationOk { .. }), "{ok:?}");
    assert_eq!(saved.listed(), PermissionMode::Plan);
    assert_eq!(saved.recorded(), PermissionMode::Plan, "persisted");
    let launch = saved.start("").await;
    assert_eq!(launched_mode(&launch), Some("plan".into()), "{launch:#?}");
    assert!(
        !std::fs::read_to_string(saved.dir().join("stdin-record"))
            .unwrap_or_default()
            .contains("control_request"),
        "nothing asked of the new process"
    );
}

/// T044 (FR-018): a terminal session's mode cannot be changed.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_sessions_mode_change_is_refused() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let session = ai_session();
    let id = session.id;
    let state = service(dir, vec![session]);
    let mut client = connect(&state).await;
    send(
        &mut client,
        ClientMsg::SetPermissionMode {
            req: 1,
            session: id,
            mode: PermissionMode::Plan,
        },
    )
    .await;
    let refused = answer(&mut client, 1).await;
    assert_eq!(
        refused_with(&refused),
        ModeRefusal::NotStructured.to_string()
    );
    assert_eq!(
        summary(&state, dir, id).permission_mode,
        PermissionMode::Default
    );
}

/// T045 (FR-016): a `status` or `init` line reporting another mode than the persisted one changes
/// the summary and the record.
#[tokio::test(flavor = "multi_thread")]
async fn a_mode_claude_reports_is_followed() {
    let saved = Saved::new(PermissionMode::Default);
    saved.start("status acceptEdits\n").await;
    eventually("the status mode listed", || {
        saved.listed() == PermissionMode::AcceptEdits
    });
    assert_eq!(saved.recorded(), PermissionMode::AcceptEdits, "persisted");
    saved.state.stop_session(saved.id);

    saved
        .start("{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s-942\",\"permissionMode\":\"plan\"}\n")
        .await;
    eventually("the init mode listed", || {
        saved.listed() == PermissionMode::Plan
    });
    assert_eq!(saved.recorded(), PermissionMode::Plan, "persisted");
}

/// T045 (FR-016, R6): a mode the app never sets (`dontAsk`) leaves the mode unchanged and is
/// logged once.
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_reported_mode_is_ignored_and_logged_once() {
    mcp::log();
    let saved = Saved::new(PermissionMode::Plan);
    saved
        .start("status dontAsk\nstatus dontAsk\nstatus acceptEdits\n")
        .await;
    // The known mode after the unknown ones is applied: both unknown ones were seen by then.
    eventually("the known mode listed", || {
        saved.listed() == PermissionMode::AcceptEdits
    });
    let lines = mcp::log_lines_for(saved.id);
    let unknown: Vec<&String> = lines.iter().filter(|l| l.contains("dontAsk")).collect();
    assert_eq!(unknown.len(), 1, "{lines:#?}");
}

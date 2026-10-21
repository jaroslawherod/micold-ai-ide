//! #942 milestone M4 (T041): a structured session's permission mode is chosen on `SessionCreate`,
//! listed in its summary, launched with (`--permission-mode` only when not Default) and kept across
//! a restart and a daemon restart (US2 scenarios 1–3, FR-013, FR-015, SC-005). A terminal session
//! cannot be given one, and its launch and binding carry none (FR-018).
//!
//! The stand-in of `support/structured.rs` records its argv in the session's folder. A conversation
//! is "recorded" as `structured_resume.rs` records one: a transcript file in `claude`'s layout under
//! `CLAUDE_CONFIG_DIR`, which this binary points at a directory of its own, once.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

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

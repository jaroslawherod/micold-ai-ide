//! Feature 938, milestone M4 (T029): a structured session resumes its Claude conversation as a
//! terminal session does (US3, FR-004, FR-021, FR-021a, SC-003). A new one starts a conversation of
//! its own id; a stop/start, a `SessionRestart` and a daemon restart over the saved catalog resume it
//! with the structured arguments and keep its kind; a resume whose conversation is gone fails as a
//! terminal session's does.
//!
//! The stand-in of `support/structured.rs` records its argv in the session's folder. A conversation
//! is "recorded" the way `session_survival.rs` records one: a transcript file in `claude`'s layout
//! under `CLAUDE_CONFIG_DIR`, which this binary points at a directory of its own, once.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use futures_util::SinkExt;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, OperationResult, SessionSummary, WireLifecycle,
};
use micold_core::session::{AiCli, SessionId, SessionKind, SessionLocation};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use conn::{connect, Client};
use history::service;
use structured::{
    clear_records, events_of, events_until, fake_stream_cli, fixture, fixture_events, record,
    starts, stream_script, structured_session, viewer, Received,
};

/// The seven structured arguments, in order (FR-005).
const STRUCTURED_ARGS: [&str; 7] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
];

/// The first prompt of the sessions that have one (FR-007).
const PROMPT: &str = "the first prompt";

/// The stand-in on `PATH` and `CLAUDE_CONFIG_DIR` at a directory of this process's, set once before
/// any case reads either; each case records its conversations under its own project's key.
fn setup() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    fake_stream_cli();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("micold-938-resume-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("a claude config directory");
        std::env::set_var("CLAUDE_CONFIG_DIR", &home);
        home
    })
}

/// `claude`'s transcript of `id` in `cwd` (its layout: `projects/<cwd with non-alphanumerics as
/// '-'>/<id>.jsonl`).
fn transcript(cwd: &Path, id: SessionId) -> PathBuf {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    setup()
        .join("projects")
        .join(key)
        .join(format!("{}.jsonl", id.0))
}

/// Record `id`'s conversation in `cwd`, as `claude` does once a session has run.
fn record_conversation(cwd: &Path, id: SessionId) {
    let file = transcript(cwd, id);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, "{}\n").unwrap();
}

/// The argv of the stand-in's latest start in `dir`.
fn argv(dir: &Path) -> Vec<String> {
    record(dir, "argv-record")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Whether `argv` starts with the structured arguments and resumes `id`'s conversation, never
/// naming a new one.
fn resumes(argv: &[String], id: SessionId) -> bool {
    argv.starts_with(&STRUCTURED_ARGS.map(String::from))
        && argv
            .windows(2)
            .any(|w| w[0] == "--resume" && w[1] == id.0.to_string())
        && !argv.iter().any(|a| a == "--session-id")
}

/// The replay of fixture `name`, as a `.fake-stream` line.
fn replay(name: &str) -> String {
    format!("replay {}\n", fixture(name).display())
}

/// A `.fake-stream` that waits for the first prompt, then answers it with `text-turn.jsonl`.
fn answering() -> String {
    format!("await-stdin\n{}", replay("text-turn.jsonl"))
}

/// The texts of the user messages the stand-in has read in `dir`, none when it read none.
fn delivered(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("stdin-record"))
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            value["message"]["content"][0]["text"]
                .as_str()
                .expect("a text block")
                .to_owned()
        })
        .collect()
}

/// Start `id` fresh with [`PROMPT`] as its first prompt, as a client creating it with one does.
async fn start_with_prompt(state: &Arc<DaemonState>, id: SessionId, project: &Path) {
    let (_, typed) = micold_daemon::ops::start_with_prompt(
        state,
        id,
        project,
        AiCli::ClaudeCode,
        LaunchMode::Fresh,
        micold_core::occupancy::Additional::Yes,
        Some(micold_daemon::ops::FirstPrompt {
            text: PROMPT,
            asked: tokio::time::Instant::now(),
            require_bracketed: false,
        }),
    )
    .await;
    assert!(typed.is_ok(), "the prompt was delivered: {typed:?}");
}

/// Wait, within 10 s, until the stand-in has started `n` times in `dir`.
async fn started_times(dir: &Path, n: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while starts(dir) < n {
        assert!(
            std::time::Instant::now() < deadline,
            "never started {n} times"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn send(client: &mut Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.expect("sent");
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
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
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

/// A session of `kind` created in a saved catalog in `store`, whose conversation `claude` recorded,
/// and the daemon after a restart over that catalog, its startup pass run.
fn restarted_with(
    store: &Path,
    project: &Path,
    kind: SessionKind,
) -> (Arc<DaemonState>, SessionId) {
    save_project(store, project);
    let id = daemon(store)
        .create_session_of_kind(
            project,
            "",
            AiCli::ClaudeCode,
            kind,
            micold_core::occupancy::Additional::Yes,
        )
        .expect("created");
    record_conversation(project, id);
    let restarted = daemon(store);
    restarted.present_interrupted_resumable_at_startup();
    (restarted, id)
}

/// US3 scenario 2, SC-003: a new structured session starts a conversation of its own id, never a
/// resume.
#[tokio::test(flavor = "multi_thread")]
async fn a_new_structured_session_starts_a_new_conversation() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let state = service(dir, Vec::new());
    stream_script(dir, "");
    let mut client = connect(&state).await;
    send(
        &mut client,
        ClientMsg::SessionCreate {
            req: 1,
            project: dir.to_path_buf(),
            worktree_dir: String::new(),
            provider: AiCli::ClaudeCode,
            allow_additional_session: true,
            kind: SessionKind::Structured,
        },
    )
    .await;
    let id = tokio::time::timeout(Duration::from_secs(20), async {
        use futures_util::StreamExt;
        loop {
            if let Frame::Control(DaemonMsg::OperationOk {
                req: 1,
                result: OperationResult::SessionCreated { session },
            }) = client.next().await.expect("stream open").unwrap()
            {
                return session;
            }
        }
    })
    .await
    .expect("created");
    let argv = argv(dir);
    assert!(
        argv.starts_with(&STRUCTURED_ARGS.map(String::from)),
        "{argv:?}"
    );
    assert!(
        argv.windows(2)
            .any(|w| w[0] == "--session-id" && w[1] == id.0.to_string()),
        "{argv:?}"
    );
    assert!(!argv.iter().any(|a| a == "--resume"), "{argv:?}");
    state.stop_session(id);
}

/// US3 scenario 1, SC-003, FR-007, FR-021a: once its conversation is recorded, a stop/start and a
/// `SessionRestart` launch the structured arguments resuming the same conversation; the first prompt
/// is not sent again; the earlier run's events stay in the history, the resumed run's follow them.
#[tokio::test(flavor = "multi_thread")]
async fn stop_start_and_restart_resume_the_same_conversation() {
    setup();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(dir, vec![session]);
    stream_script(dir, &answering());
    start_with_prompt(&state, id, dir).await;
    let first = fixture_events("text-turn.jsonl");
    let mut watching = viewer(&state, dir, id).await;
    events_until(&mut watching, id, |got| got.len() >= first.len()).await;
    record_conversation(dir, id);

    state.stop_session(id);
    stream_script(dir, &replay("resume-init.jsonl"));
    let mut client = connect(&state).await;
    send(
        &mut client,
        ClientMsg::SessionStart {
            session: id,
            allow_additional_session: true,
        },
    )
    .await;
    started_times(dir, 2).await;
    assert!(resumes(&argv(dir), id), "start: {:?}", argv(dir));
    let resumed = fixture_events("resume-init.jsonl");
    let mut late = viewer(&state, dir, id).await;
    let got: Vec<Received> = events_until(&mut late, id, |got| {
        got.len() >= first.len() + resumed.len()
    })
    .await;
    let mut expected = first.clone();
    expected.extend(resumed);
    assert_eq!(
        events_of(&got),
        expected,
        "the earlier events, then the resumed run's"
    );

    state.stop_session(id);
    send(
        &mut client,
        ClientMsg::SessionRestart {
            session: id,
            allow_additional_session: true,
        },
    )
    .await;
    started_times(dir, 3).await;
    assert!(resumes(&argv(dir), id), "restart: {:?}", argv(dir));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        delivered(dir),
        [PROMPT],
        "the first prompt is never sent again"
    );
    state.stop_session(id);
}

/// US3 scenarios 1 and 3, FR-004, FR-007, FR-021, FR-021a: after a daemon restart over the saved
/// catalog the session is listed structured and resumable, and its start resumes the conversation
/// with the structured arguments, without the first prompt; with no history store the history
/// starts at the resume.
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_restart_keeps_the_kind_and_resumes_the_conversation() {
    setup();
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    save_project(store.path(), dir);
    let id = {
        let state = daemon(store.path());
        let id = state
            .create_session_of_kind(
                dir,
                "",
                AiCli::ClaudeCode,
                SessionKind::Structured,
                micold_core::occupancy::Additional::Yes,
            )
            .expect("created");
        stream_script(dir, &answering());
        start_with_prompt(&state, id, dir).await;
        // The stand-in read it, so it recorded its start before the stop below.
        started_times(dir, 1).await;
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while delivered(dir).is_empty() {
            assert!(
                std::time::Instant::now() < deadline,
                "the prompt was never read"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        record_conversation(dir, id);
        state.stop_session(id);
        id
        // …and the daemon goes away here.
    };
    clear_records(dir);

    let restarted = daemon(store.path());
    restarted.present_interrupted_resumable_at_startup();
    let listed = summary(&restarted, dir, id);
    assert_eq!(listed.kind, SessionKind::Structured, "the kind is restored");
    assert_eq!(listed.lifecycle, WireLifecycle::InterruptedResumable);

    stream_script(dir, &replay("resume-init.jsonl"));
    let mut watching = viewer(&restarted, dir, id).await;
    let mut client = connect(&restarted).await;
    send(
        &mut client,
        ClientMsg::SessionStart {
            session: id,
            allow_additional_session: true,
        },
    )
    .await;
    started_times(dir, 2).await;
    assert!(resumes(&argv(dir), id), "{:?}", argv(dir));
    let resumed = fixture_events("resume-init.jsonl");
    let got = events_until(&mut watching, id, |got| got.len() >= resumed.len()).await;
    assert_eq!(events_of(&got), resumed, "the history starts at the resume");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        delivered(dir).is_empty(),
        "the first prompt is not sent again"
    );
    restarted.stop_session(id);
}

/// A resume of a session of `kind` whose recorded conversation was deleted after the daemon's
/// restart: the start's error and the lifecycle it leaves, the project's path and the id masked.
fn gone_resume(kind: SessionKind) -> (String, WireLifecycle) {
    let store = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let dir = project.path();
    let (state, id) = restarted_with(store.path(), dir, kind);
    std::fs::remove_file(transcript(dir, id)).unwrap();
    let error = match state.start_session(id, LaunchMode::Resume) {
        Ok(()) => "started".to_owned(),
        Err(e) => e.to_string(),
    }
    .replace(&dir.display().to_string(), "<project>")
    .replace(&id.0.to_string(), "<id>");
    (error, summary(&state, dir, id).lifecycle)
}

/// US3 scenario 4: a resume whose conversation is gone fails with a terminal session's report.
#[test]
fn a_resume_whose_conversation_is_gone_fails_as_a_terminal_sessions_does() {
    setup();
    let terminal = gone_resume(SessionKind::Terminal);
    assert!(
        terminal.0.contains("no longer has this conversation"),
        "{terminal:?}"
    );
    assert_eq!(gone_resume(SessionKind::Structured), terminal);
}

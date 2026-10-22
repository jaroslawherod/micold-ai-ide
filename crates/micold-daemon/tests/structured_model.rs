//! Feature 943, milestone M4 (T036): a session's model choice. `SetSessionModel` asks a running,
//! idle structured session's CLI with one `set_model` control request and keeps the choice only
//! when it accepts (FR-015, FR-016); a busy session's request waits for the turn's `result` and goes
//! before any later message; the choice is persisted and every later launch, in either kind, carries
//! `--model` and resumes with `--resume` (FR-017).
//!
//! Every case runs the stand-in of `support/structured.rs`; its `models` directive answers
//! `set_model` (#943 T035).
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
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::{AiCli, PermissionMode, SessionId, SessionKind};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::structured::model::{session_model, ModelChoice, ModelName};
use micold_core::structured::StructuredEvent;
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
use conn::{answer, connect, Client};
use structured::{
    clear_records, events_until, fake_stream_cli, fixture, record, stdin_lines, stream_script,
    viewer,
};

/// The message `set_model` is refused with for a terminal session (contracts/wire.md).
const TERMINAL_REFUSAL: &str =
    "Model switching is for structured sessions; the choice is used at the next structured launch";

/// The stand-in on `PATH` and `CLAUDE_CONFIG_DIR` at a directory of this process's, set once.
fn setup() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    fake_stream_cli();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("micold-943-model-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("a claude config directory");
        std::env::set_var("CLAUDE_CONFIG_DIR", &home);
        home
    })
}

fn named(name: &str) -> ModelChoice {
    ModelChoice::Named(ModelName::parse(name).unwrap())
}

/// Record `id`'s conversation in `cwd` in `claude`'s layout, so a resume finds it.
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

/// The argv of the stand-in's latest start in `dir`.
fn argv(dir: &Path) -> Vec<String> {
    record(dir, "argv-record")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The model `argv` names with `--model`, never twice.
fn launched_model(argv: &[String]) -> Option<String> {
    let named: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--model")
        .map(|w| &w[1])
        .collect();
    assert!(named.len() <= 1, "one model at most: {argv:#?}");
    named.first().map(|m| m.to_string())
}

fn resumes(argv: &[String], id: SessionId) -> bool {
    argv.windows(2)
        .any(|w| w[0] == "--resume" && w[1] == id.0.to_string())
}

/// The `set_model` requests the stand-in in `dir` read, as the model each asked for.
fn models_asked(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .iter()
        .filter(|line| line.contains("\"subtype\":\"set_model\""))
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            value["request"]["model"]
                .as_str()
                .expect("a model")
                .to_owned()
        })
        .collect()
}

/// Wait up to 10 s for `done`.
fn eventually(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "{what} never happened");
        std::thread::sleep(Duration::from_millis(10));
    }
}

async fn send(client: &mut Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.expect("sent");
}

fn pick(req: u64, session: SessionId, model: &ModelChoice) -> ClientMsg {
    ClientMsg::SetSessionModel {
        req,
        session,
        model: model.clone(),
    }
}

fn say(req: u64, session: SessionId, text: &str) -> ClientMsg {
    ClientMsg::SessionUserMessage {
        req,
        session,
        text: text.into(),
        images: Vec::new(),
    }
}

fn assert_ok(answer: &DaemonMsg) {
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "{answer:?}"
    );
}

fn refused_with(answer: &DaemonMsg) -> &str {
    match answer {
        DaemonMsg::OperationError { message, .. } => message,
        other => panic!("refused: {other:?}"),
    }
}

fn load_catalog(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A daemon over a catalog saved in a store of its own, with one project and, in it, sessions
/// created by the test; every one is stopped when it goes.
struct Saved {
    store: tempfile::TempDir,
    projects: Vec<tempfile::TempDir>,
    state: Arc<DaemonState>,
    ids: Vec<SessionId>,
}

impl Saved {
    /// One session of `kind` in each of `n` projects.
    fn new(n: usize, kind: SessionKind) -> Self {
        setup();
        let store = tempfile::tempdir().unwrap();
        let projects: Vec<_> = (0..n).map(|_| tempfile::tempdir().unwrap()).collect();
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&Workspace {
                projects: projects
                    .iter()
                    .map(|p| Project::new(p.path().to_path_buf(), true, Availability::Available))
                    .collect(),
                active: Some(projects[0].path().to_path_buf()),
                sessions: BTreeMap::new(),
                worktree_names: BTreeMap::new(),
                ..Default::default()
            })
            .unwrap();
        let state = Arc::new(DaemonState::new(load_catalog(store.path())));
        micold_daemon::server::spawn_input_driver(Arc::clone(&state));
        micold_daemon::server::spawn_model_answers(Arc::clone(&state));
        let ids = projects
            .iter()
            .map(|p| {
                state
                    .create_session_in_mode(
                        p.path(),
                        "",
                        AiCli::ClaudeCode,
                        kind,
                        PermissionMode::Default,
                        Additional::Yes,
                    )
                    .expect("created")
            })
            .collect();
        Self {
            store,
            projects,
            state,
            ids,
        }
    }

    fn dir(&self, n: usize) -> &Path {
        self.projects[n].path()
    }

    /// Start session `n` with `script`, once its stand-in recorded its argv.
    async fn start(&self, n: usize, script: &str, mode: LaunchMode) -> Vec<String> {
        clear_records(self.dir(n));
        stream_script(self.dir(n), script);
        let (state, id) = (Arc::clone(&self.state), self.ids[n]);
        tokio::task::spawn_blocking(move || state.start_session(id, mode))
            .await
            .unwrap()
            .expect("the session starts");
        argv(self.dir(n))
    }

    /// The model choice session `n`'s summary lists.
    fn listed(&self, n: usize) -> ModelChoice {
        self.state
            .sessions_for(self.dir(n))
            .into_iter()
            .find(|s| s.id == self.ids[n])
            .expect("listed")
            .model
    }

    /// The model choice the saved catalog holds for session `n`.
    fn recorded(&self, n: usize) -> ModelChoice {
        load_catalog(self.store.path())
            .workspace()
            .find_session(self.ids[n])
            .map(|(_, s)| s.model.clone())
            .expect("recorded")
    }

    /// Session `n`'s event history now.
    fn history(&self, n: usize) -> Vec<StructuredEvent> {
        self.state
            .event_stream(self.ids[n])
            .map(|stream| {
                let (_, events, _) = stream.saved_snapshot();
                events.into_iter().map(|e| e.event).collect()
            })
            .unwrap_or_default()
    }

    fn pid(&self, n: usize) -> String {
        record(self.dir(n), "pid-record")
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        for id in &self.ids {
            self.state.stop_session(*id);
        }
    }
}

fn changed_to(event: &StructuredEvent, model: &str) -> bool {
    matches!(event, StructuredEvent::ModelChanged { model: m } if m == model)
}

/// US3 s3, FR-015, SC-004: an idle session's pick is one `set_model` line to the same process, its
/// `ModelChanged` note reaches two viewing windows within 1 s of the answer, the choice is listed
/// and persisted, and the next turn is answered by the new model.
#[tokio::test(flavor = "multi_thread")]
async fn an_idle_pick_is_one_set_model_line_and_every_window_sees_it_within_a_second() {
    let saved = Saved::new(1, SessionKind::Structured);
    let turn = fixture("text-turn.jsonl");
    saved
        .start(
            0,
            &format!("models haiku\non-message {}\n", turn.display()),
            LaunchMode::Fresh,
        )
        .await;
    let (id, dir) = (saved.ids[0], saved.dir(0));
    let pid = saved.pid(0);
    let mut one = viewer(&saved.state, dir, id).await;
    let mut two = viewer(&saved.state, dir, id).await;
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, id, &named("haiku"))).await;
    assert_ok(&answer(&mut client, 1).await);
    let answered = Instant::now();
    for window in [&mut one, &mut two] {
        let got = events_until(window, id, |got| {
            got.iter().any(|r| changed_to(&r.event.event, "haiku"))
        })
        .await;
        let seen = got
            .iter()
            .find(|r| changed_to(&r.event.event, "haiku"))
            .unwrap();
        assert!(
            seen.at.saturating_duration_since(answered) <= Duration::from_secs(1),
            "SC-004"
        );
    }
    assert_eq!(models_asked(dir), ["haiku"], "one set_model line");
    assert_eq!(saved.pid(0), pid, "the same process");
    assert_eq!(saved.listed(0), named("haiku"));
    assert_eq!(saved.recorded(0), named("haiku"));

    send(&mut client, say(2, id, "next")).await;
    eventually("the next turn's result", || {
        saved
            .history(0)
            .iter()
            .any(|e| matches!(e, StructuredEvent::Result { .. }))
    });
    assert_eq!(
        session_model(
            &saved
                .history(0)
                .into_iter()
                .enumerate()
                .map(|(seq, event)| micold_core::structured::SequencedEvent {
                    seq: seq as u64,
                    event
                })
                .collect::<Vec<_>>()
        )
        .as_deref(),
        Some("haiku"),
        "the next turn runs the new model (SC-004)"
    );
}

/// The busy script: `models <accept>`, then one turn per message, each held until `turn-<n>`.
fn busy_script(accept: &str) -> String {
    format!(
        "models {accept}\non-message-gated {}\n",
        fixture("text-turn.jsonl").display()
    )
}

/// A busy pick: started with `busy_script(accept)`, a first message opens a turn, `model` is picked
/// (answered `OperationOk` at once) and a second message is queued behind the turn.
async fn busy_pick(saved: &Saved, accept: &str, model: &ModelChoice) -> Client {
    saved
        .start(0, &busy_script(accept), LaunchMode::Fresh)
        .await;
    let (id, dir) = (saved.ids[0], saved.dir(0));
    let mut client = connect(&saved.state).await;
    send(&mut client, say(1, id, "one")).await;
    assert_ok(&answer(&mut client, 1).await);
    eventually("the first message on stdin", || stdin_lines(dir).len() == 1);
    send(&mut client, pick(2, id, model)).await;
    assert_ok(&answer(&mut client, 2).await);
    send(&mut client, say(3, id, "two")).await;
    assert_ok(&answer(&mut client, 3).await);
    std::thread::sleep(Duration::from_millis(200));
    assert!(models_asked(dir).is_empty(), "nothing before the result");
    assert_eq!(stdin_lines(dir).len(), 1, "the second message waits");
    client
}

/// US3 s4, FR-015: a busy session's pick is answered at once and asked of the CLI at the turn's
/// `result`, before the next message; the turn in progress runs on, uninterrupted.
#[tokio::test(flavor = "multi_thread")]
async fn a_busy_pick_waits_for_the_result_and_goes_before_the_next_message() {
    let saved = Saved::new(1, SessionKind::Structured);
    let _client = busy_pick(&saved, "haiku", &named("haiku")).await;
    let dir = saved.dir(0);
    assert_eq!(saved.listed(0), named("haiku"), "listed at once");
    std::fs::write(dir.join("turn-1"), "").unwrap();
    eventually("the second message delivered", || {
        stdin_lines(dir).len() == 3
    });
    let lines = stdin_lines(dir);
    assert!(lines[0].contains("\"one\""), "{lines:#?}");
    assert!(lines[1].contains("\"subtype\":\"set_model\""), "{lines:#?}");
    assert!(lines[2].contains("\"two\""), "{lines:#?}");
    eventually("the ModelChanged note", || {
        saved.history(0).iter().any(|e| changed_to(e, "haiku"))
    });
    let history = saved.history(0);
    let result = history
        .iter()
        .position(|e| matches!(e, StructuredEvent::Result { .. }))
        .expect("the first turn ended");
    let changed = history.iter().position(|e| changed_to(e, "haiku")).unwrap();
    assert!(result < changed, "after the turn: {history:#?}");
    assert!(
        !history
            .iter()
            .any(|e| matches!(e, StructuredEvent::InterruptAsked)),
        "the turn was not interrupted"
    );
    assert_eq!(saved.recorded(0), named("haiku"));
}

/// US3 s7, FR-016: an idle pick the CLI refuses is an error with its reason; the choice stays.
#[tokio::test(flavor = "multi_thread")]
async fn an_idle_pick_claude_refuses_is_its_reason_and_the_choice_stays() {
    let saved = Saved::new(1, SessionKind::Structured);
    let id = saved.ids[0];
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, id, &named("sonnet"))).await;
    assert_ok(&answer(&mut client, 1).await);
    saved.start(0, "models sonnet\n", LaunchMode::Fresh).await;
    send(&mut client, pick(2, id, &named("no-such-model-xyz"))).await;
    let refused = answer(&mut client, 2).await;
    assert_eq!(
        refused_with(&refused),
        "Model 'no-such-model-xyz' not found"
    );
    assert_eq!(saved.listed(0), named("sonnet"));
    assert_eq!(saved.recorded(0), named("sonnet"));
    assert_eq!(
        structured::starts(saved.dir(0)),
        1,
        "the session keeps running"
    );
}

/// FR-016: a deferred pick the CLI refuses becomes a `ModelRefused` note with its reason, and the
/// choice goes back to the one before.
#[tokio::test(flavor = "multi_thread")]
async fn a_deferred_pick_claude_refuses_is_a_note_and_the_choice_goes_back() {
    let saved = Saved::new(1, SessionKind::Structured);
    let _client = busy_pick(&saved, "", &named("haiku")).await;
    std::fs::write(saved.dir(0).join("turn-1"), "").unwrap();
    eventually("the ModelRefused note", || {
        saved.history(0).iter().any(|e| {
            matches!(e, StructuredEvent::ModelRefused { model, message }
                if model == "haiku" && message == "Model 'haiku' not found")
        })
    });
    eventually("the choice back at Default", || {
        saved.listed(0) == ModelChoice::Default
    });
    assert_eq!(saved.recorded(0), ModelChoice::Default);
}

/// FR-017: a pick for a session that is not running is persisted only; its next launch carries it.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_for_a_session_not_running_is_persisted_and_launched_with() {
    let saved = Saved::new(1, SessionKind::Structured);
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, saved.ids[0], &named("haiku"))).await;
    assert_ok(&answer(&mut client, 1).await);
    assert_eq!(saved.listed(0), named("haiku"));
    assert_eq!(saved.recorded(0), named("haiku"));
    let launch = saved.start(0, "", LaunchMode::Fresh).await;
    assert_eq!(
        launched_model(&launch).as_deref(),
        Some("haiku"),
        "{launch:#?}"
    );
    assert!(models_asked(saved.dir(0)).is_empty());
}

/// FR-017, contracts/wire.md: a terminal session's pick is refused with the sentence and still
/// persisted; its terminal launch carries it.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_sessions_pick_is_refused_persisted_and_launched_with() {
    let saved = Saved::new(1, SessionKind::Terminal);
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, saved.ids[0], &named("haiku"))).await;
    assert_eq!(
        refused_with(&answer(&mut client, 1).await),
        TERMINAL_REFUSAL
    );
    assert_eq!(saved.listed(0), named("haiku"));
    assert_eq!(saved.recorded(0), named("haiku"));
    let launch = saved.start(0, "", LaunchMode::Fresh).await;
    assert_eq!(
        launched_model(&launch).as_deref(),
        Some("haiku"),
        "{launch:#?}"
    );
}

/// An unknown session's pick is refused.
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_sessions_pick_is_refused() {
    let saved = Saved::new(1, SessionKind::Structured);
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, SessionId::new(), &named("haiku"))).await;
    refused_with(&answer(&mut client, 1).await);
    assert_eq!(saved.listed(0), ModelChoice::Default);
}

/// Edge Cases: of two picks sent back to back, the last one wins, in the summary and on stdin.
#[tokio::test(flavor = "multi_thread")]
async fn of_two_picks_the_last_one_wins() {
    let saved = Saved::new(1, SessionKind::Structured);
    saved
        .start(0, "models haiku sonnet\n", LaunchMode::Fresh)
        .await;
    let id = saved.ids[0];
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, id, &named("haiku"))).await;
    send(&mut client, pick(2, id, &named("sonnet"))).await;
    assert_ok(&answer(&mut client, 1).await);
    assert_ok(&answer(&mut client, 2).await);
    assert_eq!(models_asked(saved.dir(0)), ["haiku", "sonnet"]);
    assert_eq!(saved.listed(0), named("sonnet"));
    assert_eq!(saved.recorded(0), named("sonnet"));
}

/// US3 s8, FR-017, FR-020: a restart and a daemon restart launch with the choice and resume the
/// session's conversation.
#[tokio::test(flavor = "multi_thread")]
async fn a_restart_and_a_daemon_restore_launch_with_the_choice_and_resume() {
    let saved = Saved::new(1, SessionKind::Structured);
    let (id, dir) = (saved.ids[0], saved.dir(0));
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, id, &named("haiku"))).await;
    assert_ok(&answer(&mut client, 1).await);
    saved.start(0, "", LaunchMode::Fresh).await;
    record_conversation(dir, id);
    saved.state.stop_session(id);
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
    assert!(resumes(&relaunch, id), "{relaunch:#?}");
    assert_eq!(launched_model(&relaunch).as_deref(), Some("haiku"));
    saved.state.stop_session(id);

    let restarted = Arc::new(DaemonState::new(load_catalog(saved.store.path())));
    restarted.present_interrupted_resumable_at_startup();
    clear_records(dir);
    let mut client = connect(&restarted).await;
    send(
        &mut client,
        ClientMsg::SessionStart {
            session: id,
            allow_additional_session: true,
        },
    )
    .await;
    let restored = argv(dir);
    assert!(resumes(&restored, id), "{restored:#?}");
    assert_eq!(launched_model(&restored).as_deref(), Some("haiku"));
    restarted.stop_session(id);
}

/// Edge Cases: one session's pick never reaches another's CLI, summary or history.
#[tokio::test(flavor = "multi_thread")]
async fn one_sessions_pick_never_touches_another() {
    let saved = Saved::new(2, SessionKind::Structured);
    saved.start(0, "models haiku\n", LaunchMode::Fresh).await;
    saved.start(1, "models haiku\n", LaunchMode::Fresh).await;
    let mut client = connect(&saved.state).await;
    send(&mut client, pick(1, saved.ids[0], &named("haiku"))).await;
    assert_ok(&answer(&mut client, 1).await);
    assert_eq!(models_asked(saved.dir(0)), ["haiku"]);
    assert!(models_asked(saved.dir(1)).is_empty());
    assert_eq!(saved.listed(1), ModelChoice::Default);
    assert_eq!(saved.recorded(1), ModelChoice::Default);
    assert!(!saved
        .history(1)
        .iter()
        .any(|e| matches!(e, StructuredEvent::ModelChanged { .. })));
}

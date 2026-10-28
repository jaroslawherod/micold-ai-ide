//! Feature 1554, milestone M2 (T014, T015): a session's reasoning effort choice.
//! `SetSessionEffort` asks a running, idle structured session's CLI with one
//! `apply_flag_settings` control request and keeps the choice only when it accepts (FR-004,
//! FR-005); a session not running has it stored with a note; a terminal session has it stored and
//! is told it applies at the next launch; another CLI's is refused (FR-012). Every later launch, in
//! either kind, carries the effective level as `--effort` (FR-010, FR-010a, FR-011, SC-004).
//!
//! Every case runs the stand-in of `support/structured.rs`; its `efforts` directive answers
//! `apply_flag_settings` and `initialize-effort` rewrites what a model supports (#1554 T013).
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::occupancy::Additional;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::session::{AiCli, PermissionMode, SessionId, SessionKind};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::structured::effort::{EffortChoice, EffortLevel};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::HistoryStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/structured.rs"]
mod structured;
use structured::{
    clear_records, fake_stream_cli, fixture, record, starts, stdin_lines, stream_script,
};

/// contracts/wire.md *SetSessionEffort*.
const NOT_CLAUDE: &str = "Reasoning effort is for Claude Code sessions";
const TERMINAL_REFUSAL: &str =
    "Effort is changed from the structured view; the choice is used at the next launch";
const BUSY_REFUSAL: &str = "The session is busy; the effort can be changed when the turn ends";

/// What a terminal stand-in prints.
const TERMINAL: &str = "ready>\n";

/// The stand-in on `PATH` and `CLAUDE_CONFIG_DIR` at a directory of this process's, set once.
fn setup() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    fake_stream_cli();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("micold-1554-effort-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("a claude config directory");
        std::env::set_var("CLAUDE_CONFIG_DIR", &home);
        home
    })
}

fn level(name: &str) -> EffortChoice {
    EffortChoice::Level(EffortLevel::parse(name).unwrap())
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

/// The level `argv` names with `--effort`, never twice.
fn launched_effort(argv: &[String]) -> Option<String> {
    let named: Vec<&String> = argv
        .windows(2)
        .filter(|w| w[0] == "--effort")
        .map(|w| &w[1])
        .collect();
    assert!(named.len() <= 1, "one level at most: {argv:#?}");
    named.first().map(|l| l.to_string())
}

fn resumes(argv: &[String], id: SessionId) -> bool {
    argv.windows(2)
        .any(|w| w[0] == "--resume" && w[1] == id.0.to_string())
}

/// Whether `session`, of the project at `dir`, runs with a living process in `state`.
fn runs(state: &DaemonState, dir: &Path, session: SessionId) -> bool {
    state
        .running_sessions_in(dir, "")
        .iter()
        .any(|(id, _)| *id == session)
}

fn is_structured(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "stream-json")
}

/// The `apply_flag_settings` requests the stand-in in `dir` read, as the level each set (`None`:
/// `null`, the CLI's default).
fn efforts_asked(dir: &Path) -> Vec<Option<String>> {
    stdin_lines(dir)
        .iter()
        .filter(|line| line.contains("\"subtype\":\"apply_flag_settings\""))
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
            let settings = &value["request"]["settings"];
            assert!(
                settings.get("effortLevel").is_some(),
                "it sets the effort: {line}"
            );
            settings["effortLevel"].as_str().map(str::to_owned)
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

fn pick(req: u64, session: SessionId, effort: &EffortChoice) -> ClientMsg {
    ClientMsg::SetSessionEffort {
        req,
        session,
        effort: effort.clone(),
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

fn refused(answer: &DaemonMsg) -> (ErrorKind, &str) {
    match answer {
        DaemonMsg::OperationError { kind, message, .. } => (*kind, message),
        other => panic!("refused: {other:?}"),
    }
}

fn load_catalog(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A daemon over the catalog saved in `store`, saving histories in `history`.
fn daemon(store: &Path, history: &Path) -> Arc<DaemonState> {
    let state = Arc::new(DaemonState::new(load_catalog(store)));
    state.set_history_store(HistoryStore::new(history.to_path_buf(), true));
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    state
}

/// One connected window: the answers it got and the effort choice each session's summary showed
/// it last, with when.
struct Window {
    client: conn::Client,
    answers: HashMap<u64, DaemonMsg>,
    efforts: HashMap<SessionId, (EffortChoice, Instant)>,
}

impl Window {
    async fn connect(state: &Arc<DaemonState>) -> Self {
        Self {
            client: conn::connect(state).await,
            answers: HashMap::new(),
            efforts: HashMap::new(),
        }
    }

    async fn send(&mut self, msg: ClientMsg) {
        self.client.send(Frame::Control(msg)).await.expect("sent");
    }

    /// Receive until `done` holds, within 15 s.
    async fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while !done(self) {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            assert!(!left.is_zero(), "{what}: never happened");
            if let Ok(frame) = tokio::time::timeout(left, self.client.next()).await {
                match frame.expect("the connection is open").expect("a frame") {
                    Frame::Control(
                        m @ (DaemonMsg::OperationOk { req, .. }
                        | DaemonMsg::OperationError { req, .. }),
                    ) => {
                        self.answers.insert(req, m);
                    }
                    Frame::Control(DaemonMsg::CatalogChanged { catalog }) => {
                        for s in catalog.projects.iter().flat_map(|p| &p.sessions) {
                            if self.efforts.get(&s.id).map(|(e, _)| e) != Some(&s.effort) {
                                self.efforts
                                    .insert(s.id, (s.effort.clone(), Instant::now()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    async fn request(&mut self, msg: ClientMsg, req: u64) -> DaemonMsg {
        self.send(msg).await;
        self.until(&format!("the answer to {req}"), |w| {
            w.answers.contains_key(&req)
        })
        .await;
        self.answers.remove(&req).unwrap()
    }

    /// When this window was shown `effort` for `session`, waiting for it.
    async fn shown(&mut self, session: SessionId, effort: &EffortChoice) -> Instant {
        self.until(&format!("{effort} shown"), |w| {
            w.efforts.get(&session).map(|(e, _)| e) == Some(effort)
        })
        .await;
        self.efforts[&session].1
    }
}

/// A daemon over a saved catalog with one project per session, each made by the test; every one
/// is stopped when it goes.
struct Rig {
    store: tempfile::TempDir,
    history: tempfile::TempDir,
    projects: Vec<tempfile::TempDir>,
    state: Arc<DaemonState>,
    ids: Vec<SessionId>,
}

impl Rig {
    /// Sessions of `cli` and `kind`, one in each of `n` projects.
    fn new(n: usize, cli: AiCli, kind: SessionKind) -> Self {
        setup();
        let store = tempfile::tempdir().unwrap();
        let history = tempfile::tempdir().unwrap();
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
        let state = daemon(store.path(), history.path());
        let ids = projects
            .iter()
            .map(|p| {
                state
                    .create_session_in_mode(
                        p.path(),
                        "",
                        cli,
                        kind,
                        PermissionMode::Default,
                        Additional::Yes,
                    )
                    .expect("created")
            })
            .collect();
        Self {
            store,
            history,
            projects,
            state,
            ids,
        }
    }

    fn structured(n: usize) -> Self {
        Self::new(n, AiCli::ClaudeCode, SessionKind::Structured)
    }

    fn dir(&self, n: usize) -> &Path {
        self.projects[n].path()
    }

    /// Start session `n` with `script`, once its stand-in recorded its argv.
    async fn start(&self, n: usize, script: &str) -> Vec<String> {
        clear_records(self.dir(n));
        stream_script(self.dir(n), script);
        let (state, id) = (Arc::clone(&self.state), self.ids[n]);
        tokio::task::spawn_blocking(move || state.start_session(id, LaunchMode::Fresh))
            .await
            .unwrap()
            .expect("the session starts");
        argv(self.dir(n))
    }

    /// Start session `n` with `script` after an `initialize` answer, and wait for its note.
    async fn start_reported(&self, n: usize, script: &str) {
        self.start(n, &format!("{script}initialize\n")).await;
        eventually("the CommandsReported note", || {
            self.history(n)
                .iter()
                .any(|e| matches!(e, StructuredEvent::CommandsReported { .. }))
        });
    }

    /// The effort choice session `n`'s summary lists, which the saved catalog must hold too.
    fn effort(&self, n: usize) -> EffortChoice {
        let listed = self
            .state
            .sessions_for(self.dir(n))
            .into_iter()
            .find(|s| s.id == self.ids[n])
            .expect("listed")
            .effort;
        let saved = load_catalog(self.store.path())
            .workspace()
            .find_session(self.ids[n])
            .map(|(_, s)| s.effort.clone())
            .expect("recorded");
        assert_eq!(listed, saved, "listed as saved");
        listed
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

    fn noted(&self, n: usize, effort: &EffortChoice) -> bool {
        self.history(n).iter().any(
            |e| matches!(e, StructuredEvent::EffortChanged { effort: noted } if noted == effort),
        )
    }

    fn pid(&self, n: usize) -> String {
        record(self.dir(n), "pid-record")
    }

    /// Restart session `n` with `script` through `window`, as a window does.
    async fn restart(&self, n: usize, script: &str, window: &mut Window) -> Vec<String> {
        let (id, dir) = (self.ids[n], self.dir(n));
        record_conversation(dir, id);
        self.state.stop_session(id);
        clear_records(dir);
        stream_script(dir, script);
        window
            .send(ClientMsg::SessionRestart {
                session: id,
                allow_additional_session: true,
            })
            .await;
        argv(dir)
    }

    /// A second daemon over this one's saved catalog and histories, this one stopped and its
    /// histories saved first; session `n` started in it with `script`.
    async fn restored(&self, n: usize, script: &str) -> (Arc<DaemonState>, Vec<String>) {
        let (id, dir) = (self.ids[n], self.dir(n));
        record_conversation(dir, id);
        self.state.stop_session(id);
        self.state
            .save_due_at(Instant::now() + Duration::from_secs(3600));
        let restarted = daemon(self.store.path(), self.history.path());
        restarted.present_interrupted_resumable_at_startup();
        clear_records(dir);
        stream_script(dir, script);
        let mut window = Window::connect(&restarted).await;
        window
            .send(ClientMsg::SessionStart {
                session: id,
                allow_additional_session: true,
            })
            .await;
        (restarted, argv(dir))
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        for id in &self.ids {
            self.state.stop_session(*id);
        }
    }
}

/// US1 s3, s6, FR-004, FR-006, SC-002: an idle session's pick is one `apply_flag_settings` line to
/// the same process, which is not relaunched; two windows show the choice within 1 s of the
/// answer; it is noted and saved. Default is a `null` line and shows Default.
#[tokio::test(flavor = "multi_thread")]
async fn an_idle_pick_is_one_line_to_the_same_process_and_every_window_shows_it() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_reported(0, "efforts high\n").await;
    let pid = rig.pid(0);
    let mut one = Window::connect(&rig.state).await;
    let mut two = Window::connect(&rig.state).await;
    let mut asker = Window::connect(&rig.state).await;
    assert_ok(&asker.request(pick(1, id, &level("high")), 1).await);
    let answered = Instant::now();
    for window in [&mut one, &mut two] {
        let shown = window.shown(id, &level("high")).await;
        assert!(
            shown.saturating_duration_since(answered) <= Duration::from_secs(1),
            "SC-002"
        );
    }
    assert_eq!(efforts_asked(dir), [Some("high".to_owned())]);
    assert_eq!(rig.pid(0), pid, "the same process");
    assert_eq!(starts(dir), 1, "not relaunched");
    assert!(!resumes(&argv(dir), id));
    assert_eq!(rig.effort(0), level("high"));
    eventually("the EffortChanged note", || rig.noted(0, &level("high")));

    assert_ok(&asker.request(pick(2, id, &EffortChoice::Default), 2).await);
    assert_eq!(
        efforts_asked(dir),
        [Some("high".to_owned()), None],
        "FR-006"
    );
    for window in [&mut one, &mut two] {
        window.shown(id, &EffortChoice::Default).await;
    }
    assert_eq!(rig.effort(0), EffortChoice::Default);
    eventually("the Default note", || rig.noted(0, &EffortChoice::Default));
}

/// US1 s7, FR-005: an idle pick the CLI refuses is an error with its reason; the choice stays and
/// the session keeps running.
#[tokio::test(flavor = "multi_thread")]
async fn an_idle_pick_claude_refuses_is_its_reason_and_the_choice_stays() {
    let rig = Rig::structured(1);
    rig.start(0, "efforts\n").await;
    let mut window = Window::connect(&rig.state).await;
    let answer = window.request(pick(1, rig.ids[0], &level("high")), 1).await;
    assert_eq!(
        refused(&answer),
        (
            ErrorKind::Refused,
            "apply_flag_settings: unknown effort level 'high'"
        )
    );
    assert_eq!(rig.effort(0), EffortChoice::Default);
    assert!(!rig.noted(0, &level("high")));
    assert_eq!(starts(rig.dir(0)), 1);
    assert!(
        runs(&rig.state, rig.dir(0), rig.ids[0]),
        "the session keeps running"
    );
}

/// FR-005: a pick whose CLI ends before it answers is an error; the choice stays.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_whose_cli_ends_unanswered_is_an_error_and_the_choice_stays() {
    let rig = Rig::structured(1);
    rig.start(0, "await-stdin\nexit 0\n").await;
    let mut window = Window::connect(&rig.state).await;
    let answer = window.request(pick(1, rig.ids[0], &level("high")), 1).await;
    assert_eq!(
        refused(&answer),
        (ErrorKind::Refused, "Claude stopped before it answered.")
    );
    assert_eq!(rig.effort(0), EffortChoice::Default);
}

/// FR-003: a model that does not support effort takes the pick as Claude Code's default (a `null`
/// line); the choice is stored all the same.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_for_a_model_without_effort_is_stored_and_sent_as_the_default() {
    let rig = Rig::structured(1);
    rig.start_reported(0, "efforts high\ninitialize-effort default none\n")
        .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, rig.ids[0], &level("high")), 1).await);
    assert_eq!(efforts_asked(rig.dir(0)), [None]);
    assert_eq!(rig.effort(0), level("high"));
    eventually("the note", || rig.noted(0, &level("high")));
}

/// R5: a session not running has the pick stored, with its note, and answered at once.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_for_a_session_not_running_is_stored_and_noted() {
    let rig = Rig::structured(1);
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, rig.ids[0], &level("high")), 1).await);
    assert_eq!(rig.effort(0), level("high"));
    window.shown(rig.ids[0], &level("high")).await;
    assert!(rig.noted(0, &level("high")), "{:#?}", rig.history(0));
}

/// contracts/wire.md: a terminal session's pick is stored and answered that it applies at the
/// next launch.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_sessions_pick_is_stored_and_refused_with_the_sentence() {
    let rig = Rig::new(1, AiCli::ClaudeCode, SessionKind::Terminal);
    let mut window = Window::connect(&rig.state).await;
    let answer = window.request(pick(1, rig.ids[0], &level("high")), 1).await;
    assert_eq!(refused(&answer), (ErrorKind::Refused, TERMINAL_REFUSAL));
    assert_eq!(rig.effort(0), level("high"));
}

/// FR-012: another CLI's pick is refused and nothing is stored; an unknown session's is not found.
#[tokio::test(flavor = "multi_thread")]
async fn another_clis_pick_is_refused_and_an_unknown_sessions_is_not_found() {
    let rig = Rig::new(1, AiCli::Copilot, SessionKind::Terminal);
    let mut window = Window::connect(&rig.state).await;
    let answer = window.request(pick(1, rig.ids[0], &level("high")), 1).await;
    assert_eq!(refused(&answer), (ErrorKind::Refused, NOT_CLAUDE));
    assert_eq!(rig.effort(0), EffortChoice::Default);
    let answer = window
        .request(pick(2, SessionId::new(), &level("high")), 2)
        .await;
    assert_eq!(refused(&answer).0, ErrorKind::NotFound);
}

/// Edge Cases "two windows": picks from two windows in turn; the last one is held and shown by
/// both.
#[tokio::test(flavor = "multi_thread")]
async fn of_picks_from_two_windows_the_last_is_held_and_shown_by_both() {
    let rig = Rig::structured(1);
    let id = rig.ids[0];
    rig.start(0, "efforts high max\n").await;
    let mut a = Window::connect(&rig.state).await;
    let mut b = Window::connect(&rig.state).await;
    assert_ok(&a.request(pick(1, id, &level("high")), 1).await);
    assert_ok(&b.request(pick(1, id, &level("max")), 1).await);
    assert_eq!(
        efforts_asked(rig.dir(0)),
        [Some("high".to_owned()), Some("max".to_owned())]
    );
    for window in [&mut a, &mut b] {
        window.shown(id, &level("max")).await;
    }
    assert_eq!(rig.effort(0), level("max"));
}

/// US3 s4: one session's pick never reaches another's CLI, choice or history.
#[tokio::test(flavor = "multi_thread")]
async fn one_sessions_pick_never_touches_another() {
    let rig = Rig::structured(2);
    rig.start(0, "efforts high\n").await;
    rig.start(1, "efforts high\n").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, rig.ids[0], &level("high")), 1).await);
    assert_eq!(efforts_asked(rig.dir(0)), [Some("high".to_owned())]);
    assert!(efforts_asked(rig.dir(1)).is_empty());
    assert_eq!(rig.effort(1), EffortChoice::Default);
    assert!(!rig
        .history(1)
        .iter()
        .any(|e| matches!(e, StructuredEvent::EffortChanged { .. })));
}

/// M2 (interim, replaced by M3): a busy session's pick is refused; nothing is stored or written.
#[tokio::test(flavor = "multi_thread")]
async fn a_busy_sessions_pick_is_refused_for_now() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start(
        0,
        &format!(
            "efforts high\non-message-gated {}\n",
            fixture("text-turn.jsonl").display()
        ),
    )
    .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    let answer = window.request(pick(2, id, &level("high")), 2).await;
    assert_eq!(refused(&answer), (ErrorKind::Busy, BUSY_REFUSAL));
    assert!(efforts_asked(dir).is_empty());
    assert_eq!(rig.effort(0), EffortChoice::Default);
    std::fs::write(dir.join("turn-1"), "").unwrap();
}

/// US3 s1, s3, FR-010, FR-011, SC-004: a new session launches with no `--effort`; a level stored
/// while not running is launched with, at a restart (resuming the conversation) and in a second
/// daemon that restored the session.
#[tokio::test(flavor = "multi_thread")]
async fn a_stored_level_is_launched_with_at_every_start_restart_and_restore() {
    let rig = Rig::structured(1);
    let id = rig.ids[0];
    let fresh = rig.start(0, "").await;
    assert_eq!(launched_effort(&fresh), None, "FR-011: {fresh:#?}");
    rig.state.stop_session(id);
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("xhigh")), 1).await);

    let started = rig.start(0, "").await;
    assert_eq!(launched_effort(&started).as_deref(), Some("xhigh"));
    let relaunch = rig.restart(0, "", &mut window).await;
    assert!(resumes(&relaunch, id), "{relaunch:#?}");
    assert_eq!(launched_effort(&relaunch).as_deref(), Some("xhigh"));

    let (restarted, restored) = rig.restored(0, "").await;
    assert!(resumes(&restored, id), "{restored:#?}");
    assert_eq!(launched_effort(&restored).as_deref(), Some("xhigh"));
    restarted.stop_session(id);
}

/// US3 s1, s2, FR-010: a structured → terminal → structured switch launches each kind with the
/// stored level.
#[tokio::test(flavor = "multi_thread")]
async fn a_switch_of_kind_launches_both_kinds_with_the_stored_level() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("xhigh")), 1).await);
    rig.start(0, "").await;
    record_conversation(dir, id);
    for (req, to, script) in [
        (2, SessionKind::Terminal, TERMINAL),
        (3, SessionKind::Structured, ""),
    ] {
        clear_records(dir);
        stream_script(dir, script);
        let switch = ClientMsg::SessionSwitchKind {
            req,
            session: id,
            to,
            command: None,
            confirmed: true,
        };
        assert_ok(&window.request(switch, req).await);
        let launched = argv(dir);
        assert_eq!(
            is_structured(&launched),
            to == SessionKind::Structured,
            "{launched:#?}"
        );
        assert!(resumes(&launched, id), "{launched:#?}");
        assert_eq!(
            launched_effort(&launched).as_deref(),
            Some("xhigh"),
            "{launched:#?}"
        );
    }
}

/// FR-010a: a stored level the session model is known not to take launches with none, and the
/// session runs: at a restart and in a second daemon that restored the session from its saved
/// history.
#[tokio::test(flavor = "multi_thread")]
async fn a_stored_level_the_model_does_not_take_launches_with_none() {
    let rig = Rig::structured(1);
    let id = rig.ids[0];
    rig.start_reported(
        0,
        "efforts low medium\ninitialize-effort default low,medium\n",
    )
    .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("max")), 1).await);
    assert_eq!(
        efforts_asked(rig.dir(0)),
        [None],
        "the CLI's default applies"
    );
    assert_eq!(rig.effort(0), level("max"), "the choice is kept");

    let relaunch = rig.restart(0, "", &mut window).await;
    assert!(resumes(&relaunch, id), "{relaunch:#?}");
    assert_eq!(launched_effort(&relaunch), None, "{relaunch:#?}");
    assert!(runs(&rig.state, rig.dir(0), id), "the session runs");

    let (restarted, restored) = rig.restored(0, "").await;
    assert_eq!(launched_effort(&restored), None, "{restored:#?}");
    assert!(runs(&restarted, rig.dir(0), id), "the session runs");
    restarted.stop_session(id);
}

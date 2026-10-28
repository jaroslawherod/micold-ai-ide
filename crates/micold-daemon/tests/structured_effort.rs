//! Feature 1554, milestone M2 (T014, T015): a session's reasoning effort choice.
//! `SetSessionEffort` asks a running, idle structured session's CLI with one
//! `apply_flag_settings` control request and keeps the choice only when it accepts (FR-004,
//! FR-005); a session not running has it stored with a note; a terminal session has it stored and
//! is told it applies at the next launch; another CLI's is refused (FR-012). Every later launch, in
//! either kind, carries the effective level as `--effort` (FR-010, FR-010a, FR-011, SC-004).
//!
//! Milestone M3 (T018, T019): a busy session's pick is answered at once and written at the turn's
//! end, after a model picked in the same turn; a refused one is put back with an `EffortRefused`
//! note (FR-005); a model change re-applies the effective level, never noted (research R4).
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
use micold_core::structured::model::{ModelChoice, ModelName};
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

/// The choices `history`'s `EffortChanged` notes name, in order.
fn effort_notes(history: &[StructuredEvent]) -> Vec<EffortChoice> {
    history
        .iter()
        .filter_map(|e| match e {
            StructuredEvent::EffortChanged { effort } => Some(effort.clone()),
            _ => None,
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

fn pick_model(req: u64, session: SessionId, name: &str) -> ClientMsg {
    ClientMsg::SetSessionModel {
        req,
        session,
        model: ModelChoice::Named(ModelName::parse(name).unwrap()),
    }
}

/// The stand-in's script for held turns: `setup`, the `initialize` answer, then one turn per
/// message, each held until the test writes `turn-<n>`.
fn turns(setup: &str) -> String {
    format!(
        "{setup}initialize\non-message-gated {}\n",
        fixture("text-turn.jsonl").display()
    )
}

/// Where in `history` the first event `is` holds for is.
fn position(history: &[StructuredEvent], is: impl Fn(&StructuredEvent) -> bool) -> usize {
    history
        .iter()
        .position(is)
        .unwrap_or_else(|| panic!("not in {history:#?}"))
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
    let state = undriven(store, history);
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    state
}

/// [`daemon`] with no input driver: the test takes the turn ends and hands them on.
fn undriven(store: &Path, history: &Path) -> Arc<DaemonState> {
    let state = Arc::new(DaemonState::new(load_catalog(store)));
    state.set_history_store(HistoryStore::new(history.to_path_buf(), true));
    micold_daemon::server::spawn_model_answers(Arc::clone(&state));
    micold_daemon::server::spawn_effort_answers(Arc::clone(&state));
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
        Self::build(n, cli, kind, daemon)
    }

    /// One structured session over [`undriven`].
    fn undriven() -> Self {
        Self::build(1, AiCli::ClaudeCode, SessionKind::Structured, undriven)
    }

    fn build(
        n: usize,
        cli: AiCli,
        kind: SessionKind,
        daemon: fn(&Path, &Path) -> Arc<DaemonState>,
    ) -> Self {
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
        self.start_until_reported(n, &format!("{script}initialize\n"))
            .await;
    }

    /// Start session `n` with [`turns`]`(setup)`, and wait for the `initialize` answer's note.
    async fn start_turns(&self, n: usize, setup: &str) {
        self.start_until_reported(n, &turns(setup)).await;
    }

    async fn start_until_reported(&self, n: usize, script: &str) {
        self.start(n, script).await;
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

/// Review A M2 F3: a pick that leaves a stopped session's choice as it was is answered and writes
/// no note.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_that_changes_nothing_writes_no_note() {
    let rig = Rig::structured(1);
    let mut window = Window::connect(&rig.state).await;
    assert_ok(
        &window
            .request(pick(1, rig.ids[0], &EffortChoice::Default), 1)
            .await,
    );
    assert_ok(&window.request(pick(2, rig.ids[0], &level("high")), 2).await);
    assert_ok(&window.request(pick(3, rig.ids[0], &level("high")), 3).await);
    assert_eq!(effort_notes(&rig.history(0)), [level("high")]);
}

/// Review A M2 F2: the notes of a stopped session's picks end with the choice the catalog holds,
/// whatever order they are written in.
#[tokio::test(flavor = "multi_thread")]
async fn the_notes_of_quick_picks_end_with_the_choice_held() {
    use micold_daemon::state::EffortPick;
    let rig = Rig::structured(1);
    let id = rig.ids[0];
    for effort in [level("high"), level("low")] {
        assert!(matches!(
            rig.state.pick_effort(id, effort),
            EffortPick::Stored
        ));
    }
    rig.state.note_effort_changed(id);
    rig.state.note_effort_changed(id);
    assert_eq!(rig.effort(0), level("low"));
    assert_eq!(effort_notes(&rig.history(0)), [level("low")]);
}

/// Review A M2 F1: a late answer to an idle pick never replaces a choice stored after it.
#[tokio::test(flavor = "multi_thread")]
async fn a_late_answer_never_replaces_a_later_stored_choice() {
    use micold_daemon::state::EffortPick;
    let rig = Rig::structured(1);
    let id = rig.ids[0];
    rig.start(0, "efforts high low\n").await;
    let EffortPick::Asked(asked) = rig.state.pick_effort(id, level("high")) else {
        panic!("the idle session's CLI is asked");
    };
    eventually("the CLI's answer", || rig.noted(0, &level("high")));
    rig.state.stop_session(id);
    assert!(matches!(
        rig.state.pick_effort(id, level("low")),
        EffortPick::Stored
    ));
    rig.state.finish_effort_pick(asked).await.expect("accepted");
    assert_eq!(rig.effort(0), level("low"));
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
    eventually("the session runs", || runs(&rig.state, rig.dir(0), id));

    let (restarted, restored) = rig.restored(0, "").await;
    assert_eq!(launched_effort(&restored), None, "{restored:#?}");
    eventually("the session runs", || runs(&restarted, rig.dir(0), id));
    restarted.stop_session(id);
}

/// Whether `event` is a `ModelChanged` note.
fn model_changed(event: &StructuredEvent) -> bool {
    matches!(event, StructuredEvent::ModelChanged { .. })
}

/// The `apply_flag_settings` or `set_model` request `line` is, or the user message's text.
fn kind_of(line: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(line).expect("a JSON line");
    match value["request"]["subtype"].as_str() {
        Some(subtype) => subtype.to_owned(),
        None => value["message"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    }
}

/// Every line the stand-in in `dir` read, by [`kind_of`].
fn stdin_kinds(dir: &Path) -> Vec<String> {
    stdin_lines(dir).iter().map(|l| kind_of(l)).collect()
}

/// M3 T018 (US1 s4, FR-004, SC-001): a busy session's pick is answered at once and shown, and
/// nothing is written while the turn runs; at its `result` exactly one `apply_flag_settings` line
/// goes out, after a `set_model` picked in the same turn and before the next message; the
/// `EffortChanged` note follows the turn, which was never interrupted.
#[tokio::test(flavor = "multi_thread")]
async fn a_busy_pick_is_answered_at_once_and_written_at_the_turns_end() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "models haiku\nefforts low\n").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick_model(2, id, "haiku"), 2).await);
    assert_ok(&window.request(pick(3, id, &level("low")), 3).await);
    window.shown(id, &level("low")).await;
    assert_ok(&window.request(say(4, id, "two"), 4).await);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(stdin_kinds(dir), ["one"], "nothing during the turn");

    std::fs::write(dir.join("turn-1"), "").unwrap();
    eventually("the next message delivered", || stdin_lines(dir).len() == 4);
    assert_eq!(
        stdin_kinds(dir),
        ["one", "set_model", "apply_flag_settings", "two"]
    );
    eventually("the notes", || {
        rig.noted(0, &level("low")) && rig.history(0).iter().any(model_changed)
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(efforts_asked(dir), [Some("low".to_owned())], "exactly one");
    let history = rig.history(0);
    let result = position(&history, |e| matches!(e, StructuredEvent::Result { .. }));
    let noted = position(
        &history,
        |e| matches!(e, StructuredEvent::EffortChanged { effort } if *effort == level("low")),
    );
    assert!(result < noted, "after the turn: {history:#?}");
    assert!(
        !history
            .iter()
            .any(|e| matches!(e, StructuredEvent::InterruptAsked)),
        "the turn was not interrupted"
    );
    assert_eq!(rig.effort(0), level("low"));
    std::fs::write(dir.join("turn-2"), "").unwrap();
}

/// M3 T018 (Edge Cases "two windows"): of two picks in one turn only the last is written, and
/// noted.
#[tokio::test(flavor = "multi_thread")]
async fn of_two_picks_in_one_turn_only_the_last_is_written() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "efforts high low\n").await;
    let mut a = Window::connect(&rig.state).await;
    let mut b = Window::connect(&rig.state).await;
    assert_ok(&a.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&a.request(pick(2, id, &level("high")), 2).await);
    assert_ok(&b.request(pick(2, id, &level("low")), 2).await);
    for window in [&mut a, &mut b] {
        window.shown(id, &level("low")).await;
    }
    std::fs::write(dir.join("turn-1"), "").unwrap();
    eventually("the note", || rig.noted(0, &level("low")));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(efforts_asked(dir), [Some("low".to_owned())]);
    assert_eq!(effort_notes(&rig.history(0)), [level("low")]);
    assert_eq!(rig.effort(0), level("low"));
}

/// M3 T018 (US1 s7, FR-005): a deferred pick the CLI refuses becomes an `EffortRefused` note with
/// its reason; the choice goes back to the one before and the session keeps running.
#[tokio::test(flavor = "multi_thread")]
async fn a_deferred_pick_claude_refuses_is_a_note_and_the_choice_goes_back() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "efforts high\n").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(2, id, &level("low")), 2).await);
    window.shown(id, &level("low")).await;
    std::fs::write(dir.join("turn-1"), "").unwrap();
    eventually("the EffortRefused note", || {
        rig.history(0).iter().any(|e| {
            matches!(e, StructuredEvent::EffortRefused { effort, message }
                if *effort == level("low")
                    && message == "apply_flag_settings: unknown effort level 'low'")
        })
    });
    window.shown(id, &EffortChoice::Default).await;
    assert_eq!(rig.effort(0), EffortChoice::Default);
    assert!(effort_notes(&rig.history(0)).is_empty());
    assert!(runs(&rig.state, dir, id), "the session keeps running");
}

/// M3 T018 (FR-005): a refused deferred pick is not put back over a pick made since.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_deferred_pick_is_not_put_back_over_a_later_pick() {
    use micold_daemon::state::EffortAnswer;
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "efforts\n").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(2, id, &level("low")), 2).await);
    rig.state.stop_session(id);
    assert_ok(&window.request(pick(3, id, &level("high")), 3).await);
    rig.state
        .follow_effort_answer(id, EffortAnswer::Refused(level("low")));
    assert_eq!(rig.effort(0), level("high"));
}

/// M3 T019 (research R4, FR-003): with a level chosen, an accepted change to a model that does
/// not take it writes `null`, and a change back to one that does writes the level again; neither
/// is noted.
#[tokio::test(flavor = "multi_thread")]
async fn a_model_change_reapplies_the_effective_level_without_a_note() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_reported(
        0,
        "models sonnet opus\nefforts high\ninitialize-effort sonnet low,medium\n",
    )
    .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("high")), 1).await);
    eventually("the note", || rig.noted(0, &level("high")));
    assert_ok(&window.request(pick_model(2, id, "sonnet"), 2).await);
    eventually("the null line", || {
        efforts_asked(dir) == [Some("high".to_owned()), None]
    });
    assert_ok(&window.request(pick_model(3, id, "opus"), 3).await);
    eventually("the high line", || efforts_asked(dir).len() == 3);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        efforts_asked(dir),
        [Some("high".to_owned()), None, Some("high".to_owned())]
    );
    assert_eq!(effort_notes(&rig.history(0)), [level("high")], "no note");
    assert_eq!(rig.effort(0), level("high"));
}

/// M3 T019 (research R4): with the choice `Default`, a model change writes no effort line.
#[tokio::test(flavor = "multi_thread")]
async fn with_the_default_choice_a_model_change_writes_no_effort() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_reported(0, "models sonnet\ninitialize-effort sonnet low,medium\n")
        .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick_model(1, id, "sonnet"), 1).await);
    eventually("the ModelChanged note", || {
        rig.history(0).iter().any(model_changed)
    });
    std::thread::sleep(Duration::from_millis(200));
    assert!(efforts_asked(dir).is_empty());
}

/// M3 T019 (research R4, FR-004): a model change during a turn writes the re-applied level at the
/// turn's end, after the `set_model` line and before the next message.
#[tokio::test(flavor = "multi_thread")]
async fn a_model_change_during_a_turn_reapplies_the_level_at_the_turns_end() {
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("high")), 1).await);
    rig.start_turns(
        0,
        "models sonnet\nefforts high\ninitialize-effort sonnet low,medium\n",
    )
    .await;
    assert_eq!(launched_effort(&argv(dir)).as_deref(), Some("high"));
    assert_ok(&window.request(say(2, id, "one"), 2).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick_model(3, id, "sonnet"), 3).await);
    assert_ok(&window.request(say(4, id, "two"), 4).await);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(stdin_kinds(dir), ["one"], "nothing during the turn");

    std::fs::write(dir.join("turn-1"), "").unwrap();
    eventually("the next message delivered", || stdin_lines(dir).len() == 4);
    assert_eq!(
        stdin_kinds(dir),
        ["one", "set_model", "apply_flag_settings", "two"]
    );
    eventually("the ModelChanged note", || {
        rig.history(0).iter().any(model_changed)
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(efforts_asked(dir), [None]);
    assert_eq!(effort_notes(&rig.history(0)), [level("high")], "no note");
    assert_eq!(rig.effort(0), level("high"));
    std::fs::write(dir.join("turn-2"), "").unwrap();
}

/// M3 review (FR-004): an idle pick made after a turn's `result` and before the turn's end is
/// handed on goes out after the pick deferred past that turn, so the CLI ends on the later pick,
/// which the catalog holds.
#[tokio::test(flavor = "multi_thread")]
async fn an_idle_pick_before_the_turns_end_is_handled_goes_after_the_deferred_one() {
    use micold_daemon::structured::ClosedBy;
    let rig = Rig::undriven();
    let (id, dir) = (rig.ids[0], rig.dir(0));
    let mut ends = rig
        .state
        .input_feed()
        .take_receiver()
        .expect("the turn ends");
    rig.start_turns(0, "efforts low high\n").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(2, id, &level("low")), 2).await);
    std::fs::write(dir.join("turn-1"), "").unwrap();
    let mut held = Vec::new();
    loop {
        let closed = tokio::time::timeout(Duration::from_secs(15), ends.recv())
            .await
            .expect("the turn ends")
            .expect("the feed is open");
        let result = closed.by == ClosedBy::Result;
        held.push(closed);
        if result {
            break;
        }
    }
    assert_ok(&window.request(pick(3, id, &level("high")), 3).await);
    let state = Arc::clone(&rig.state);
    tokio::task::spawn_blocking(move || held.into_iter().for_each(|c| state.turn_closed(c)))
        .await
        .unwrap();
    eventually("both notes", || effort_notes(&rig.history(0)).len() == 2);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        efforts_asked(dir),
        [Some("low".to_owned()), Some("high".to_owned())]
    );
    assert_eq!(effort_notes(&rig.history(0)), [level("low"), level("high")]);
    assert_eq!(rig.effort(0), level("high"));
}

/// M3 review (research R4): a model change answered while an idle pick's answer is awaited
/// re-applies that pick's level, not the stored choice before it.
#[tokio::test(flavor = "multi_thread")]
async fn a_model_change_while_an_idle_pick_is_unanswered_keeps_that_pick() {
    use micold_daemon::state::EffortPick;
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_reported(0, "models sonnet opus\nefforts high low\n")
        .await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(pick(1, id, &level("high")), 1).await);
    eventually("the note", || rig.noted(0, &level("high")));
    let EffortPick::Asked(asked) = rig.state.pick_effort(id, level("low")) else {
        panic!("an idle running session's CLI is asked");
    };
    assert_ok(&window.request(pick_model(2, id, "opus"), 2).await);
    eventually("the ModelChanged note", || {
        rig.history(0).iter().any(model_changed)
    });
    std::thread::sleep(Duration::from_millis(200));
    assert!(rig.state.finish_effort_pick(asked).await.is_ok());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        efforts_asked(dir),
        [Some("high".to_owned()), Some("low".to_owned())]
    );
    assert_eq!(rig.effort(0), level("low"));
}

/// M3 review (FR-005): a refused pick goes back to a deferred pick already written, not to the
/// choice before that one.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_pick_goes_back_to_a_written_one_not_to_the_choice_before_it() {
    use micold_daemon::state::EffortAnswer;
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(2, id, &level("low")), 2).await);
    std::fs::write(dir.join("turn-1"), "").unwrap();
    assert_ok(&window.request(say(3, id, "two"), 3).await);
    eventually("the next message delivered", || stdin_lines(dir).len() == 3);
    assert_eq!(stdin_kinds(dir), ["one", "apply_flag_settings", "two"]);
    assert_ok(&window.request(pick(4, id, &level("high")), 4).await);
    rig.state
        .follow_effort_answer(id, EffortAnswer::Refused(level("high")));
    assert_eq!(rig.effort(0), level("low"));
    std::fs::write(dir.join("turn-2"), "").unwrap();
}

/// M3 review (FR-004, FR-005): a pick deferred in a process that ended unwritten is not carried
/// into the next: there a busy pick of the stored choice writes nothing, and a refused one goes
/// back to the stored choice.
#[tokio::test(flavor = "multi_thread")]
async fn a_pick_left_unwritten_by_an_ended_process_is_not_carried_into_the_next() {
    use micold_daemon::state::EffortAnswer;
    let rig = Rig::structured(1);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.start_turns(0, "").await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "one"), 1).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(2, id, &level("low")), 2).await);
    rig.state.stop_session(id);

    rig.start_turns(0, "").await;
    assert_eq!(launched_effort(&argv(dir)).as_deref(), Some("low"));
    assert_ok(&window.request(say(3, id, "two"), 3).await);
    eventually("the message on stdin", || stdin_lines(dir).len() == 1);
    assert_ok(&window.request(pick(4, id, &level("low")), 4).await);
    assert_ok(&window.request(pick(5, id, &level("high")), 5).await);
    rig.state
        .follow_effort_answer(id, EffortAnswer::Refused(level("high")));
    assert_eq!(rig.effort(0), level("low"));
}

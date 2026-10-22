//! Feature 943, milestone M6 (T045, T046): `SessionSwitchKind` switches a Claude Code session
//! between structured and terminal (US4, US5, FR-019–FR-025, SC-005). The session keeps its id,
//! each launch is the other kind resuming the same conversation with the model choice (and, when
//! structured, the permission mode), the kind is persisted and broadcast, the event history is
//! kept with `KindSwitched` notes around the terminal period, the queue stays on hold, work in
//! progress needs a confirmation, and concurrent switches switch once.
//!
//! One stand-in plays both kinds: `support/structured.rs`'s `claude` reads its stdin a line at a
//! time whether that is a pipe (structured) or the PTY (terminal), and records its argv. Its
//! terminal script prints a prompt so the terminal's first input waits for readiness as a real
//! one's does. A conversation is "recorded" as `structured_resume.rs` records one.
// unix-only: the stand-in CLI of support/structured.rs and the PTY.
#![cfg(unix)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::occupancy::Additional;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, QueuedSummary};
use micold_core::session::{AiCli, PermissionMode, SessionId, SessionKind};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::structured::model::{ModelChoice, ModelName};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{EventsLoad, HistoryStore};
use micold_core::workspace::Workspace;
use micold_daemon::activity::{ActivityEvent, HookKind};
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

/// The seven structured arguments, in order (938 FR-005).
const STRUCTURED_ARGS: [&str; 7] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
];

/// The terminal side's script: a prompt drawn, then quiet, as a CLI ready for input is.
const TERMINAL: &str = "ready>\n";

/// The stand-in on `PATH`, `CLAUDE_CONFIG_DIR` at a directory of this process's, and every folder
/// under the temporary directory trusted there, so a terminal's first input is typed (034 FR-017).
fn setup() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    fake_stream_cli();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("micold-943-switch-{}", std::process::id()));
        std::fs::create_dir_all(&home).expect("a claude config directory");
        std::env::set_var("CLAUDE_CONFIG_DIR", &home);
        let tmp = std::env::temp_dir();
        let mut projects = serde_json::Map::new();
        for dir in [tmp.clone(), tmp.canonicalize().unwrap_or(tmp)] {
            projects.insert(
                dir.display().to_string(),
                serde_json::json!({"hasTrustDialogAccepted": true}),
            );
        }
        std::fs::write(
            home.join(".claude.json"),
            serde_json::json!({ "projects": projects }).to_string(),
        )
        .unwrap();
        home
    })
}

/// `claude`'s transcript of conversation `id` in `cwd`, in its layout.
fn transcript(cwd: &Path, id: uuid::Uuid) -> PathBuf {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    setup()
        .join("projects")
        .join(key)
        .join(format!("{id}.jsonl"))
}

/// Record conversation `id` in `cwd`, as `claude` does once a session has run.
fn record_conversation(cwd: &Path, id: SessionId) {
    let file = transcript(cwd, id.0);
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

fn is_structured(argv: &[String]) -> bool {
    argv.starts_with(&STRUCTURED_ARGS.map(String::from))
}

/// The value `argv` gives `flag`, if any.
fn value_of<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].as_str())
}

/// The texts of the user messages the structured stand-in in `dir` read in its latest run.
fn delivered(dir: &Path) -> Vec<String> {
    stdin_lines(dir)
        .iter()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["type"] == "user")
        .map(|value| {
            value["message"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

/// What was typed into the terminal stand-in in `dir` in its latest run, one entry per line.
fn typed(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("stdin-record"))
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Wait, within 15 s, until the stand-in in `dir` started `n` times, and check it started no more.
async fn started_times(dir: &Path, n: usize) {
    eventually(&format!("{n} starts"), || starts(dir) >= n).await;
    assert_eq!(starts(dir), n, "started {n} times");
}

/// Wait up to 15 s for `done`.
async fn eventually(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done() {
        assert!(Instant::now() < deadline, "{what} never happened");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn named(name: &str) -> ModelChoice {
    ModelChoice::Named(ModelName::parse(name).unwrap())
}

fn switch(req: u64, session: SessionId, to: SessionKind, command: Option<&str>) -> ClientMsg {
    ClientMsg::SessionSwitchKind {
        req,
        session,
        to,
        command: command.map(str::to_owned),
        confirmed: false,
    }
}

fn confirmed(req: u64, session: SessionId, to: SessionKind) -> ClientMsg {
    ClientMsg::SessionSwitchKind {
        req,
        session,
        to,
        command: None,
        confirmed: true,
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

#[track_caller]
fn assert_ok(answer: &DaemonMsg) {
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "{answer:?}"
    );
}

#[track_caller]
fn refused_as(answer: &DaemonMsg) -> ErrorKind {
    match answer {
        DaemonMsg::OperationError { kind, .. } => *kind,
        other => panic!("refused: {other:?}"),
    }
}

/// The script that answers every user message with `text-turn.jsonl`.
fn answering() -> String {
    format!("on-message {}\n", fixture("text-turn.jsonl").display())
}

/// The script that answers each user message only once the test creates `turn-<n>`.
fn gated() -> String {
    format!(
        "on-message-gated {}\n",
        fixture("text-turn.jsonl").display()
    )
}

fn load_catalog(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A daemon over the catalog saved in `store`, saving histories in `history`, as the service
/// loads them at its start.
fn daemon(store: &Path, history: &Path) -> Arc<DaemonState> {
    let state = Arc::new(DaemonState::new(load_catalog(store)));
    state.set_history_store(HistoryStore::new(history.to_path_buf(), true));
    micold_daemon::server::spawn_input_driver(Arc::clone(&state));
    state
}

/// A daemon over a saved catalog with one project per session, each session created by the test
/// in it; every one is stopped when it goes.
struct Rig {
    store: tempfile::TempDir,
    history: tempfile::TempDir,
    projects: Vec<tempfile::TempDir>,
    state: Arc<DaemonState>,
    ids: Vec<SessionId>,
}

impl Rig {
    /// Sessions of `cli` and `kind` in `mode`, one in each of `n` projects.
    fn new(n: usize, cli: AiCli, kind: SessionKind, mode: PermissionMode) -> Self {
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
                    .create_session_in_mode(p.path(), "", cli, kind, mode, Additional::Yes)
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

    fn claude(kind: SessionKind) -> Self {
        Self::new(1, AiCli::ClaudeCode, kind, PermissionMode::Default)
    }

    fn dir(&self, n: usize) -> &Path {
        self.projects[n].path()
    }

    /// What session `n`'s stand-in runs at its next start, its earlier records removed.
    fn next_run(&self, n: usize, script: &str) {
        clear_records(self.dir(n));
        stream_script(self.dir(n), script);
    }

    /// Start session `n` with `script`, its conversation recorded, as a session that has run.
    async fn started(&self, n: usize, script: &str) {
        self.next_run(n, script);
        let (state, id) = (Arc::clone(&self.state), self.ids[n]);
        tokio::task::spawn_blocking(move || state.start_session(id, LaunchMode::Fresh))
            .await
            .unwrap()
            .expect("the session starts");
        record_conversation(self.dir(n), id);
        record(self.dir(n), "argv-record");
    }

    /// Session `n`'s kind in the listing and in the saved catalog, which must agree.
    fn kind(&self, n: usize) -> SessionKind {
        let listed = self
            .state
            .sessions_for(self.dir(n))
            .into_iter()
            .find(|s| s.id == self.ids[n])
            .expect("listed")
            .kind;
        let saved = load_catalog(self.store.path())
            .workspace()
            .find_session(self.ids[n])
            .map(|(_, s)| s.kind)
            .expect("saved");
        assert_eq!(listed, saved, "the listed kind is the saved one");
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

    /// Session `n`'s event history as its saved file holds it.
    fn saved_history(&self, n: usize) -> Vec<StructuredEvent> {
        match HistoryStore::new(self.history.path().to_path_buf(), false).load_events(self.ids[n]) {
            EventsLoad::Events { events, .. } => events,
            other => panic!("no saved events: {other:?}"),
        }
    }

    fn pid(&self, n: usize) -> String {
        record(self.dir(n), "pid-record")
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        for id in &self.ids {
            self.state.stop_session(*id);
        }
    }
}

/// A window: a connection, and what it received of the kinds these tests read.
struct Window {
    client: conn::Client,
    answers: HashMap<u64, DaemonMsg>,
    inputs: HashMap<SessionId, (bool, Vec<QueuedSummary>)>,
    kinds: HashMap<SessionId, SessionKind>,
}

impl Window {
    async fn connect(state: &Arc<DaemonState>) -> Self {
        Self {
            client: conn::connect(state).await,
            answers: HashMap::new(),
            inputs: HashMap::new(),
            kinds: HashMap::new(),
        }
    }

    async fn send(&mut self, msg: ClientMsg) {
        self.client.send(Frame::Control(msg)).await.expect("sent");
    }

    /// Receive until `done` holds, within 30 s.
    async fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while !done(self) {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            assert!(!left.is_zero(), "{what}: never happened");
            if let Ok(frame) = tokio::time::timeout(left, self.client.next()).await {
                self.take(frame.expect("the connection is open").expect("a frame"));
            }
        }
    }

    fn take(&mut self, frame: Frame<DaemonMsg>) {
        match frame {
            Frame::Control(
                m @ (DaemonMsg::OperationOk { req, .. } | DaemonMsg::OperationError { req, .. }),
            ) => {
                self.answers.insert(req, m);
            }
            Frame::Control(DaemonMsg::StructuredInput {
                session,
                hold,
                queue,
                ..
            }) => {
                self.inputs.insert(session, (hold, queue));
            }
            Frame::Control(DaemonMsg::CatalogChanged { catalog }) => {
                for session in catalog.projects.iter().flat_map(|p| &p.sessions) {
                    self.kinds.insert(session.id, session.kind);
                }
            }
            _ => {}
        }
    }

    async fn answer(&mut self, req: u64) -> DaemonMsg {
        self.until(&format!("the answer to {req}"), |w| {
            w.answers.contains_key(&req)
        })
        .await;
        self.answers.remove(&req).expect("answered")
    }

    async fn request(&mut self, msg: ClientMsg, req: u64) -> DaemonMsg {
        self.send(msg).await;
        self.answer(req).await
    }

    /// Receive until `id`'s queue is `texts`, on hold.
    async fn held(&mut self, id: SessionId, texts: &[&str]) {
        self.until(&format!("{texts:?} on hold"), |w| {
            w.inputs.get(&id).is_some_and(|(hold, queue)| {
                *hold
                    && queue
                        .iter()
                        .map(|q| q.text.as_str())
                        .eq(texts.iter().copied())
            })
        })
        .await;
    }
}

fn switched(to: SessionKind) -> StructuredEvent {
    StructuredEvent::KindSwitched { to }
}

/// US4 scenarios 2–4, US5 scenarios 1, 5, 6, FR-020, FR-023, FR-024, SC-005: structured →
/// terminal → structured → terminal, then a daemon restart. Every launch is the other kind under
/// the same session, resuming its conversation with its model choice; only a structured launch has
/// the permission mode. The kind is listed, saved and in every window's catalog; the history keeps
/// the earlier events with one note at each switch, is saved while terminal, and is the same after
/// the daemon restart; the first message is never sent again.
#[tokio::test(flavor = "multi_thread")]
async fn a_round_trip_resumes_the_same_conversation_in_the_other_kind_every_time() {
    let mut rig = Rig::new(
        1,
        AiCli::ClaudeCode,
        SessionKind::Structured,
        PermissionMode::AcceptEdits,
    );
    let (id, dir) = (rig.ids[0], rig.dir(0).to_path_buf());
    assert!(matches!(
        rig.state.pick_model(id, named("haiku")),
        micold_daemon::state::ModelPick::Answered(Ok(()))
    ));
    rig.started(0, &answering()).await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "hello"), 1).await);
    eventually("the first turn's result", || {
        rig.history(0)
            .iter()
            .any(|e| matches!(e, StructuredEvent::Result { .. }))
    })
    .await;
    let before = rig.history(0);

    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(2, id, SessionKind::Terminal, None), 2)
            .await,
    );
    let launched = argv(&dir);
    assert!(!is_structured(&launched), "a terminal launch: {launched:?}");
    assert_eq!(
        value_of(&launched, "--resume"),
        Some(id.0.to_string().as_str())
    );
    assert_eq!(value_of(&launched, "--model"), Some("haiku"), "FR-024");
    assert_eq!(value_of(&launched, "--permission-mode"), None, "942 FR-018");
    assert_eq!(rig.kind(0), SessionKind::Terminal);
    window
        .until("every window is told the kind", |w| {
            w.kinds.get(&id) == Some(&SessionKind::Terminal)
        })
        .await;
    let mut terminal = before.clone();
    terminal.push(switched(SessionKind::Terminal));
    assert_eq!(
        rig.history(0),
        terminal,
        "the history is kept, then the note"
    );
    rig.state
        .save_due_at(std::time::Instant::now() + Duration::from_secs(3600));
    assert_eq!(
        rig.saved_history(0),
        terminal,
        "saved while terminal (Save terminal history, FR-023)"
    );

    rig.next_run(0, &answering());
    assert_ok(
        &window
            .request(switch(3, id, SessionKind::Structured, None), 3)
            .await,
    );
    let launched = argv(&dir);
    assert!(is_structured(&launched), "{launched:?}");
    assert_eq!(
        value_of(&launched, "--resume"),
        Some(id.0.to_string().as_str())
    );
    assert_eq!(value_of(&launched, "--model"), Some("haiku"));
    assert_eq!(
        value_of(&launched, "--permission-mode"),
        Some("acceptEdits")
    );
    assert_eq!(rig.kind(0), SessionKind::Structured);
    let mut back = terminal.clone();
    back.push(switched(SessionKind::Structured));
    assert_eq!(rig.history(0), back, "one note closes the terminal period");
    assert_ok(&window.request(say(4, id, "again"), 4).await);
    eventually("the resumed run read the message", || {
        delivered(&dir) == ["again"]
    })
    .await;

    eventually("the resumed run's turn is in the history", || {
        rig.history(0)
            .iter()
            .filter(|e| matches!(e, StructuredEvent::Result { .. }))
            .count()
            == 2
    })
    .await;
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(5, id, SessionKind::Terminal, None), 5)
            .await,
    );
    let kept = rig.history(0);
    assert_eq!(kept.last(), Some(&switched(SessionKind::Terminal)));
    rig.state
        .save_due_at(std::time::Instant::now() + Duration::from_secs(3600));
    rig.state.stop_session(id);
    drop(window);

    // The daemon restarts over the saved catalog and histories (US5 scenario 6).
    rig.state = daemon(rig.store.path(), rig.history.path());
    rig.state.present_interrupted_resumable_at_startup();
    assert_eq!(
        rig.kind(0),
        SessionKind::Terminal,
        "restored in its last kind"
    );
    rig.next_run(0, TERMINAL);
    let state = Arc::clone(&rig.state);
    tokio::task::spawn_blocking(move || state.start_session(id, LaunchMode::Resume))
        .await
        .unwrap()
        .expect("the session starts");
    let launched = argv(&dir);
    assert!(!is_structured(&launched), "{launched:?}");
    assert_eq!(
        value_of(&launched, "--resume"),
        Some(id.0.to_string().as_str())
    );
    assert_eq!(value_of(&launched, "--model"), Some("haiku"));
    let state = Arc::clone(&rig.state);
    let restored = tokio::task::spawn_blocking(move || state.event_stream_loading(id, false))
        .await
        .unwrap()
        .expect("the history is restored");
    let (_, events, _) = restored.saved_snapshot();
    let restored: Vec<_> = events.into_iter().map(|e| e.event).collect();
    assert_eq!(restored, kept, "no event lost or duplicated (SC-005)");
}

/// US4 scenarios 2 and 3, FR-022: a switch to terminal for a terminal-only command types it into
/// the terminal once its CLI is ready; one without a command types nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_only_command_is_the_terminals_first_input_and_nothing_else_is() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let mut window = Window::connect(&rig.state).await;

    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(1, id, SessionKind::Terminal, Some("/mcp")), 1)
            .await,
    );
    eventually("the command was typed", || typed(dir) == ["/mcp"]).await;

    rig.next_run(0, "");
    assert_ok(
        &window
            .request(switch(2, id, SessionKind::Structured, None), 2)
            .await,
    );
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(3, id, SessionKind::Terminal, None), 3)
            .await,
    );
    tokio::time::sleep(micold_core::mcp::submission::SETTLE_AFTER + Duration::from_secs(1)).await;
    assert_eq!(
        typed(dir),
        Vec::<String>::new(),
        "no input without a command"
    );
}

/// FR-020: a session whose conversation never started starts its first one in the new kind.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_that_never_ran_starts_fresh_in_the_new_kind() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    let mut window = Window::connect(&rig.state).await;
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(1, id, SessionKind::Terminal, None), 1)
            .await,
    );
    let launched = argv(dir);
    assert!(!is_structured(&launched), "{launched:?}");
    assert_eq!(
        value_of(&launched, "--session-id"),
        Some(id.0.to_string().as_str())
    );
    assert_eq!(value_of(&launched, "--resume"), None);
    assert_eq!(rig.kind(0), SessionKind::Terminal);
}

/// US5 scenario 7: a resume in the new kind that fails leaves the new kind, so the user can switch
/// back.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_resume_keeps_the_new_kind_and_the_session_switches_back() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let mut window = Window::connect(&rig.state).await;
    rig.next_run(0, "exit 1\n");
    assert_ok(
        &window
            .request(switch(1, id, SessionKind::Terminal, None), 1)
            .await,
    );
    started_times(dir, 2).await;
    assert_eq!(rig.kind(0), SessionKind::Terminal, "the kind stays");
    rig.next_run(0, "");
    assert_ok(
        &window
            .request(switch(2, id, SessionKind::Structured, None), 2)
            .await,
    );
    started_times(dir, 3).await;
    assert!(is_structured(&argv(dir)));
    assert_eq!(rig.kind(0), SessionKind::Structured);
}

/// FR-019, contracts/wire.md: another CLI's session is refused; a command with a switch to
/// structured, one that is not terminal-only, or one for a session already terminal is invalid; a
/// switch to the kind the session has is answered with nothing restarted.
#[tokio::test(flavor = "multi_thread")]
async fn refused_and_idle_switches_change_nothing() {
    let copilot = Rig::new(
        1,
        AiCli::Copilot,
        SessionKind::Terminal,
        PermissionMode::Default,
    );
    let mut window = Window::connect(&copilot.state).await;
    let answer = window
        .request(switch(1, copilot.ids[0], SessionKind::Structured, None), 1)
        .await;
    assert_eq!(refused_as(&answer), ErrorKind::Refused, "{answer:?}");
    assert_eq!(copilot.kind(0), SessionKind::Terminal);

    let terminal = Rig::claude(SessionKind::Terminal);
    let mut window = Window::connect(&terminal.state).await;
    let answer = window
        .request(
            switch(1, terminal.ids[0], SessionKind::Terminal, Some("/mcp")),
            1,
        )
        .await;
    assert_eq!(
        refused_as(&answer),
        ErrorKind::InvalidInput,
        "a terminal session gets no command through a switch: {answer:?}"
    );

    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let pid = rig.pid(0);
    let mut window = Window::connect(&rig.state).await;
    for (req, to, command) in [
        (2, SessionKind::Structured, Some("/mcp")),
        (3, SessionKind::Terminal, Some("/compact")),
        (4, SessionKind::Terminal, Some("hello")),
    ] {
        let answer = window.request(switch(req, id, to, command), req).await;
        assert_eq!(refused_as(&answer), ErrorKind::InvalidInput, "{answer:?}");
    }
    assert_ok(
        &window
            .request(switch(5, id, SessionKind::Structured, None), 5)
            .await,
    );
    assert_eq!(starts(dir), 1, "nothing restarted");
    assert_eq!(rig.pid(0), pid);
    assert_eq!(rig.kind(0), SessionKind::Structured);
}

/// US4 scenario 5, FR-021: a turn in progress, a pending permission request, or a terminal CLI that
/// is working needs a confirmation, and nothing changes without one; confirmed, the session
/// switches.
#[tokio::test(flavor = "multi_thread")]
async fn work_in_progress_is_ended_only_once_confirmed() {
    // A turn in progress.
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, &gated()).await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "work"), 1).await);
    eventually("the turn is open", || delivered(dir) == ["work"]).await;
    let answer = window
        .request(switch(2, id, SessionKind::Terminal, None), 2)
        .await;
    assert_eq!(
        refused_as(&answer),
        ErrorKind::NeedsConfirmation,
        "{answer:?}"
    );
    assert_eq!(starts(dir), 1, "nothing changed");
    assert_eq!(rig.kind(0), SessionKind::Structured);
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(confirmed(3, id, SessionKind::Terminal), 3)
            .await,
    );
    started_times(dir, 2).await;
    assert_eq!(rig.kind(0), SessionKind::Terminal);

    // A pending permission request.
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let call = micold_core::structured::permission::PromptCall {
        tool_name: "Bash".into(),
        input: serde_json::Map::new(),
        tool_use_id: Some("toolu_switch".into()),
    };
    rig.state.open_prompt(
        id,
        call,
        micold_core::structured::permission::RequestKind::Tool,
    );
    let mut window = Window::connect(&rig.state).await;
    let answer = window
        .request(switch(4, id, SessionKind::Terminal, None), 4)
        .await;
    assert_eq!(
        refused_as(&answer),
        ErrorKind::NeedsConfirmation,
        "{answer:?}"
    );
    assert_eq!(starts(dir), 1);
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(confirmed(5, id, SessionKind::Terminal), 5)
            .await,
    );
    assert_eq!(rig.kind(0), SessionKind::Terminal);

    // A terminal CLI that reports it is working.
    let rig = Rig::claude(SessionKind::Terminal);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, TERMINAL).await;
    rig.state
        .note_activity(id, ActivityEvent::Hook(HookKind::UserPromptSubmit));
    let mut window = Window::connect(&rig.state).await;
    let answer = window
        .request(switch(6, id, SessionKind::Structured, None), 6)
        .await;
    assert_eq!(
        refused_as(&answer),
        ErrorKind::NeedsConfirmation,
        "{answer:?}"
    );
    assert_eq!(starts(dir), 1);
    assert_eq!(rig.kind(0), SessionKind::Terminal);
    rig.next_run(0, "");
    assert_ok(
        &window
            .request(confirmed(7, id, SessionKind::Structured), 7)
            .await,
    );
    assert_eq!(rig.kind(0), SessionKind::Structured);
}

/// FR-025: two windows asking for the same switch at once switch the session once, 50 times over.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_switches_switch_once() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let mut one = Window::connect(&rig.state).await;
    let mut two = Window::connect(&rig.state).await;
    for i in 0..50u64 {
        let (to, script) = if i % 2 == 0 {
            (SessionKind::Terminal, TERMINAL)
        } else {
            (SessionKind::Structured, "")
        };
        stream_script(dir, script);
        one.send(switch(2 * i, id, to, None)).await;
        two.send(switch(2 * i + 1, id, to, None)).await;
        assert_ok(&one.answer(2 * i).await);
        assert_ok(&two.answer(2 * i + 1).await);
        started_times(dir, 2 + i as usize).await;
        assert_eq!(rig.kind(0), to);
    }
}

/// FR-025: a message sent while the session switches is queued on hold, never written to either
/// process, and delivered once after **Send queued**.
#[tokio::test(flavor = "multi_thread")]
async fn a_message_during_a_switch_waits_on_hold_and_is_delivered_once() {
    let rig = Rig::claude(SessionKind::Terminal);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, TERMINAL).await;
    let mut window = Window::connect(&rig.state).await;
    rig.next_run(0, &answering());
    window
        .send(switch(1, id, SessionKind::Structured, None))
        .await;
    window.send(say(2, id, "during")).await;
    assert_ok(&window.answer(2).await);
    assert_ok(&window.answer(1).await);
    window.held(id, &["during"]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        delivered(dir),
        Vec::<String>::new(),
        "nothing delivered on hold"
    );

    assert_ok(
        &window
            .request(
                ClientMsg::QueueRelease {
                    req: 3,
                    session: id,
                },
                3,
            )
            .await,
    );
    eventually("the message was delivered", || delivered(dir) == ["during"]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(delivered(dir), ["during"], "delivered once");
}

/// US5 scenario 3, FR-024: a queue is held and undelivered while the session is terminal, still
/// held after the switch back, and delivered once released.
#[tokio::test(flavor = "multi_thread")]
async fn the_queue_waits_on_hold_through_the_terminal_period() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, &gated()).await;
    let mut window = Window::connect(&rig.state).await;
    assert_ok(&window.request(say(1, id, "first"), 1).await);
    eventually("the turn is open", || delivered(dir) == ["first"]).await;
    assert_ok(&window.request(say(2, id, "queued"), 2).await);

    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(confirmed(3, id, SessionKind::Terminal), 3)
            .await,
    );
    window.held(id, &["queued"]).await;
    tokio::time::sleep(micold_core::mcp::submission::SETTLE_AFTER + Duration::from_millis(500))
        .await;
    assert_eq!(
        typed(dir),
        Vec::<String>::new(),
        "nothing reaches the terminal"
    );

    rig.next_run(0, &answering());
    assert_ok(
        &window
            .request(switch(4, id, SessionKind::Structured, None), 4)
            .await,
    );
    window.held(id, &["queued"]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(delivered(dir), Vec::<String>::new(), "still on hold");
    assert_ok(
        &window
            .request(
                ClientMsg::QueueRelease {
                    req: 5,
                    session: id,
                },
                5,
            )
            .await,
    );
    eventually("the queued message was delivered", || {
        delivered(dir) == ["queued"]
    })
    .await;
}

/// Switching session A leaves session B as it was: same process, same kind.
#[tokio::test(flavor = "multi_thread")]
async fn switching_one_session_never_touches_another() {
    let rig = Rig::new(
        2,
        AiCli::ClaudeCode,
        SessionKind::Structured,
        PermissionMode::Default,
    );
    rig.started(0, "").await;
    rig.started(1, "").await;
    let pid = rig.pid(1);
    let mut window = Window::connect(&rig.state).await;
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(1, rig.ids[0], SessionKind::Terminal, None), 1)
            .await,
    );
    assert_eq!(rig.kind(0), SessionKind::Terminal);
    assert_eq!(rig.kind(1), SessionKind::Structured);
    assert_eq!(starts(rig.dir(1)), 1);
    assert_eq!(rig.pid(1), pid);
}

/// Spec clarification 4 (BUG-904's check): after a switch to terminal, a conversation the terminal
/// CLI starts as a continuation (a `/clear` typed there) is not adopted as a second session.
#[tokio::test(flavor = "multi_thread")]
async fn a_cleared_terminal_conversation_is_no_second_session() {
    let rig = Rig::claude(SessionKind::Structured);
    let (id, dir) = (rig.ids[0], rig.dir(0));
    rig.started(0, "").await;
    let mut window = Window::connect(&rig.state).await;
    rig.next_run(0, TERMINAL);
    assert_ok(
        &window
            .request(switch(1, id, SessionKind::Terminal, None), 1)
            .await,
    );

    let successor = uuid::Uuid::new_v4();
    let lines = [
        format!(r#"{{"type":"mode","mode":"normal","sessionId":"{successor}"}}"#),
        r#"{"type":"file-history-snapshot","messageId":"m1","snapshot":{},"isSnapshotUpdate":false}"#
            .to_string(),
        format!(
            r#"{{"type":"user","sessionId":"{successor}","message":{{"role":"user","content":"<command-name>/clear</command-name>\n<command-message>clear</command-message>\n<command-args></command-args>"}}}}"#
        ),
    ];
    std::fs::write(transcript(dir, successor), lines.join("\n") + "\n").unwrap();
    assert_eq!(rig.state.discover_external_sessions(dir), 0);
    let listed: Vec<_> = rig.state.sessions_for(dir).iter().map(|s| s.id).collect();
    assert_eq!(listed, [id]);
}

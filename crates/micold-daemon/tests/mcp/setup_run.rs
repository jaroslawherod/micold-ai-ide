//! Feature 1289, US2 and US3 (daemon side): a project's setup command runs in each worktree the
//! app creates, its output reaches watchers, and `SetupRun` / `SetupStop` re-run and end it
//! (contracts C1–C2; tests A6, A8, A9, A10).
//!
//! Every test drives the real wire messages over a connection, as a window sends them.
// unix-only: the setup commands are POSIX shell lines (`touch`, `sleep`, `>&2`, `exit 3`) run by bash/sh; on Windows setup runs in PowerShell
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::occupancy::Additional;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, OperationResult};
use micold_core::session::{AiCli, SessionLocation, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::setup::{Failure, RunId, SetupCommand, SetupRun, SetupState};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_core::worktree::CreateMode;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

use crate::conn::{connect, Client};
use crate::mcp_support as mcp;
use mcp::{git, init_repo, session, sid, state_over};

const BOUND: Duration = Duration::from_secs(20);

/// A connection and everything it received that a test has not consumed, so waiting for one
/// message never loses another that arrived first.
struct Inbox {
    client: Client,
    seen: Vec<DaemonMsg>,
}

impl Inbox {
    async fn connect(state: &Arc<DaemonState>) -> Self {
        Self {
            client: connect(state).await,
            seen: Vec::new(),
        }
    }

    async fn send(&mut self, msg: ClientMsg) {
        self.client.send(Frame::Control(msg)).await.expect("sent");
    }

    /// The first message, seen or arriving within `within`, that `pick` accepts; removed.
    async fn take<T>(
        &mut self,
        within: Duration,
        mut pick: impl FnMut(&DaemonMsg) -> Option<T>,
    ) -> Option<T> {
        if let Some(i) = self.seen.iter().position(|m| pick(m).is_some()) {
            return pick(&self.seen.remove(i));
        }
        let client = &mut self.client;
        let seen = &mut self.seen;
        tokio::time::timeout(within, async {
            loop {
                if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                    if let Some(found) = pick(&msg) {
                        return found;
                    }
                    seen.push(msg);
                }
            }
        })
        .await
        .ok()
    }

    /// The answer to `req`.
    async fn answer(&mut self, req: u64) -> DaemonMsg {
        self.take(BOUND, |m| match m {
            DaemonMsg::OperationOk { req: r, .. } | DaemonMsg::OperationError { req: r, .. }
                if *r == req =>
            {
                Some(m.clone())
            }
            _ => None,
        })
        .await
        .expect("the request is answered")
    }

    /// The next `SetupChanged` for `location` whose state `want` accepts, within `within`.
    async fn changed(
        &mut self,
        location: &SessionLocation,
        within: Duration,
        want: impl Fn(&SetupState) -> bool,
    ) -> Option<SetupState> {
        self.changed_run(location, within, want)
            .await
            .map(|run| run.state().clone())
    }

    /// The next `SetupChanged` for `location` whose state `want` accepts, as its run's record.
    async fn changed_run(
        &mut self,
        location: &SessionLocation,
        within: Duration,
        want: impl Fn(&SetupState) -> bool,
    ) -> Option<SetupRun> {
        self.take(within, |m| match m {
            DaemonMsg::SetupChanged {
                location: l,
                run: Some(summary),
                ..
            } if l == location && want(summary.run.state()) => Some(summary.run.clone()),
            _ => None,
        })
        .await
    }

    /// Whether any `SetupChanged` was received and not consumed.
    fn any_setup_changed(&self) -> Vec<DaemonMsg> {
        self.seen
            .iter()
            .filter(|m| matches!(m, DaemonMsg::SetupChanged { .. }))
            .cloned()
            .collect()
    }

    /// The output lines of `location`, from the batches received, until one ends with an ended
    /// run's reason line (or `until` accepts the lines so far).
    async fn output_until(
        &mut self,
        location: &SessionLocation,
        until: impl Fn(&[String]) -> bool,
    ) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        while !until(&lines) {
            let batch = self
                .take(BOUND, |m| match m {
                    DaemonMsg::SetupOutput {
                        location: l,
                        first_line,
                        lines,
                        ..
                    } if l == location => Some((*first_line, lines.clone())),
                    _ => None,
                })
                .await
                .unwrap_or_else(|| panic!("output arrives; so far {lines:?}"));
            let (first, batch) = batch;
            // A batch may repeat lines already held (the backlog of a watch); keep each index once.
            let skip = (lines.len() as u64).saturating_sub(first) as usize;
            lines.extend(batch.into_iter().skip(skip));
        }
        lines
    }
}

/// Whether `lines` ends with an ended run's reason line.
fn ended(lines: &[String]) -> bool {
    lines.last().is_some_and(|l| {
        l.starts_with("Exited with status")
            || l.starts_with("Could not start:")
            || l.starts_with("Ended by signal")
            || l == "Stopped."
            || l.starts_with("Interrupted:")
    })
}

struct Fixture {
    state: Arc<DaemonState>,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let state = state_over(
            vec![(
                project.path().to_path_buf(),
                true,
                vec![session(
                    sid(1),
                    Some("busy"),
                    TerminalMode::Regular,
                    AiCli::ClaudeCode,
                )],
            )],
            store.path(),
        );
        Self {
            state,
            project,
            store,
        }
    }

    /// A fixture whose daemon keeps its catalog and setup files in the store folder, so a second
    /// daemon over that folder ([`Self::restart`]) is this one restarted.
    fn persistent() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        let path = project.path().to_path_buf();
        let workspace = Workspace {
            active: Some(path.clone()),
            projects: vec![Project::new(path, true, Availability::Available)],
            ..Default::default()
        };
        files_in(store.path()).save(&workspace).unwrap();
        let state = persistent_state(store.path());
        Self {
            state,
            project,
            store,
        }
    }

    /// A new daemon over this fixture's store: the restart of a persistent fixture's daemon.
    fn restart(&self) -> Arc<DaemonState> {
        persistent_state(self.store.path())
    }

    /// The store the daemon of a persistent fixture keeps its files in.
    fn files(&self) -> JsonFileStore {
        files_in(self.store.path())
    }

    fn path(&self) -> PathBuf {
        self.project.path().to_path_buf()
    }

    fn worktree_dir(&self, dir: &str) -> PathBuf {
        self.project.path().join(".claude/worktrees").join(dir)
    }
}

fn files_in(store: &Path) -> JsonFileStore {
    JsonFileStore::at(store.join("projects.json"))
}

fn persistent_state(store: &Path) -> Arc<DaemonState> {
    let catalog = Catalog::load(
        Box::new(files_in(store)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    );
    Arc::new(DaemonState::new(catalog))
}

fn wt(dir: &str) -> SessionLocation {
    SessionLocation::Worktree(dir.into())
}

async fn set_command(inbox: &mut Inbox, req: u64, project: &Path, command: Option<&str>) {
    inbox
        .send(ClientMsg::ProjectSetupSet {
            req,
            project: project.to_path_buf(),
            command: command.map(|c| SetupCommand::parse(c).expect("a command")),
        })
        .await;
    assert!(
        matches!(
            inbox.answer(req).await,
            DaemonMsg::OperationOk {
                result: OperationResult::Ack,
                ..
            }
        ),
        "ProjectSetupSet is acknowledged"
    );
}

async fn create(inbox: &mut Inbox, req: u64, project: &Path, dir: &str) {
    inbox
        .send(ClientMsg::WorktreeCreate {
            req,
            project: project.to_path_buf(),
            branch: dir.into(),
            dir_name: dir.into(),
            mode: CreateMode::NewBranch,
            session: None,
        })
        .await;
    match inbox.answer(req).await {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { .. },
            ..
        } => {}
        other => panic!("the worktree is created: {other:?}"),
    }
}

fn watch(project: &Path, location: SessionLocation) -> ClientMsg {
    ClientMsg::SetupOutputWatch {
        project: project.to_path_buf(),
        location,
        from_line: 0,
    }
}

fn running(state: &SetupState) -> bool {
    *state == SetupState::Running
}

/// A6, SC-005: the create's answer comes before setup ends; setup runs in the new worktree, goes
/// Running then Succeeded, and its output reaches a watcher with the reason line last.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_worktree_runs_the_setup_command_without_waiting_for_it() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(
        &mut window,
        1,
        &f.path(),
        Some("echo hi; touch marker; sleep 1"),
    )
    .await;
    create(&mut window, 2, &f.path(), "wt").await;
    assert!(
        !window
            .any_setup_changed()
            .iter()
            .any(|m| matches!(m, DaemonMsg::SetupChanged { run: Some(s), .. } if s.run.state().is_ended())),
        "the create was answered before setup ended"
    );
    window.send(watch(&f.path(), wt("wt"))).await;
    assert_eq!(
        window.changed(&wt("wt"), BOUND, running).await,
        Some(SetupState::Running)
    );
    let lines = window.output_until(&wt("wt"), ended).await;
    assert_eq!(lines.first().map(String::as_str), Some("hi"), "{lines:?}");
    assert_eq!(
        lines.last().map(String::as_str),
        Some("Exited with status 0."),
        "the reason line is last: {lines:?}"
    );
    assert_eq!(
        window
            .changed(&wt("wt"), BOUND, |s| *s == SetupState::Succeeded)
            .await,
        Some(SetupState::Succeeded)
    );
    assert!(
        f.worktree_dir("wt").join("marker").exists(),
        "the command ran in the new worktree"
    );

    // A window that opens the output after the end gets the backlog.
    let mut later = Inbox::connect(&f.state).await;
    later.send(watch(&f.path(), wt("wt"))).await;
    let backlog = later.output_until(&wt("wt"), ended).await;
    assert_eq!(backlog, lines, "the backlog is the whole output");
}

/// A6, FR-014: a session started in a worktree whose setup is running starts at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_starts_while_setup_runs() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("sleep 30")).await;
    create(&mut window, 2, &f.path(), "busy").await;
    window
        .changed(&wt("busy"), BOUND, running)
        .await
        .expect("setup runs");
    let started = tokio::time::timeout(
        Duration::from_secs(10),
        micold_daemon::ops::start_session(&f.state, sid(1), LaunchMode::Fresh, Additional::No),
    )
    .await
    .expect("the start does not wait for setup")
    .unwrap();
    assert!(started, "the session started");
    assert!(
        window
            .changed(&wt("busy"), Duration::from_millis(200), |s| s.is_ended())
            .await
            .is_none(),
        "setup was still running when the session started"
    );
    for pty in f.state.remove_session(sid(1)) {
        let _ = pty.kill();
    }
    window
        .send(ClientMsg::SetupStop {
            req: 3,
            project: f.path(),
            location: wt("busy"),
        })
        .await;
    window.answer(3).await;
}

/// A8, FR-017, SC-004: a non-zero exit is `Failed(Exit)` with the status as the last line.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failing_command_reports_its_exit_status() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("echo oops >&2; exit 3")).await;
    create(&mut window, 2, &f.path(), "wt").await;
    window.send(watch(&f.path(), wt("wt"))).await;
    assert_eq!(
        window.changed(&wt("wt"), BOUND, |s| s.is_ended()).await,
        Some(SetupState::Failed(Failure::Exit(3)))
    );
    let lines = window.output_until(&wt("wt"), ended).await;
    assert_eq!(
        lines,
        vec!["oops".to_string(), "Exited with status 3.".into()]
    );
}

/// A8: a run whose folder is gone before the spawn could not start, and says why last.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_whose_folder_is_gone_could_not_start() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("true")).await;
    create(&mut window, 2, &f.path(), "wt").await;
    window
        .changed(&wt("wt"), BOUND, |s| *s == SetupState::Succeeded)
        .await
        .expect("the first run succeeds");
    std::fs::remove_dir_all(f.worktree_dir("wt")).unwrap();
    window
        .send(ClientMsg::SetupRun {
            req: 3,
            project: f.path(),
            location: wt("wt"),
        })
        .await;
    assert!(matches!(
        window.answer(3).await,
        DaemonMsg::OperationOk { .. }
    ));
    let state = window
        .changed(&wt("wt"), BOUND, |s| s.is_ended())
        .await
        .expect("the run ends");
    assert!(
        matches!(state, SetupState::Failed(Failure::NotStarted(_))),
        "{state:?}"
    );
    window.send(watch(&f.path(), wt("wt"))).await;
    let lines = window.output_until(&wt("wt"), ended).await;
    assert!(
        lines.last().unwrap().starts_with("Could not start:"),
        "{lines:?}"
    );
}

/// A8, US2 scenario 7: with no command, a create starts nothing and pushes no `SetupChanged`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn with_no_command_a_create_runs_nothing() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("touch marker")).await;
    set_command(&mut window, 2, &f.path(), None).await;
    create(&mut window, 3, &f.path(), "wt").await;
    assert!(
        window
            .changed(&wt("wt"), Duration::from_millis(1500), |_| true)
            .await
            .is_none(),
        "no SetupChanged"
    );
    assert!(window.any_setup_changed().is_empty());
    assert!(!f.worktree_dir("wt").join("marker").exists(), "nothing ran");
}

/// A10, FR-020, R10: a worktree joined with `WorktreeInclude` or found by a refresh runs no
/// setup; one created with `WorktreeCreate` does (the positive control).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_a_worktree_the_app_creates_runs_setup() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("true")).await;

    let outside = tempfile::tempdir().unwrap();
    let included = outside.path().join("joined");
    git(
        &f.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "joined",
            included.to_str().unwrap(),
        ],
    );
    window
        .send(ClientMsg::WorktreeInclude {
            req: 2,
            project: f.path(),
            path: included.clone(),
        })
        .await;
    assert!(matches!(
        window.answer(2).await,
        DaemonMsg::OperationOk { .. }
    ));

    let found = f.worktree_dir("found");
    git(
        &f.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "found",
            found.to_str().unwrap(),
        ],
    );
    window
        .send(ClientMsg::WorktreeRefresh {
            req: 3,
            project: f.path(),
        })
        .await;
    window.answer(3).await;

    create(&mut window, 4, &f.path(), "made").await;
    assert_eq!(
        window
            .changed(&wt("made"), BOUND, |s| *s == SetupState::Succeeded)
            .await,
        Some(SetupState::Succeeded),
        "a created worktree runs setup"
    );
    let others: Vec<_> = window
        .any_setup_changed()
        .into_iter()
        .filter(
            |m| !matches!(m, DaemonMsg::SetupChanged { location, .. } if *location == wt("made")),
        )
        .collect();
    assert!(others.is_empty(), "no other worktree ran setup: {others:?}");
}

/// The text of an `OperationError`'s kind and message, or a panic.
fn error(msg: DaemonMsg) -> (ErrorKind, String) {
    match msg {
        DaemonMsg::OperationError { kind, message, .. } => (kind, message),
        other => panic!("expected an OperationError, got {other:?}"),
    }
}

/// Whether process `pid` is still alive (a zombie counts as gone: it has ended).
fn alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => !stat
            .rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z')),
        Err(_) => {
            if Path::new("/proc/self").exists() {
                return false;
            }
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .is_ok_and(|s| s.success())
        }
    }
}

/// A9, FR-022, FR-023, FR-023a: a second run is refused while one runs, a run needs a command,
/// Stop ends the whole tree as `Stopped`, Stop with nothing running is `NotFound`, and a run on
/// `Default` runs in the project folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn run_and_stop_setup() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(
        &mut window,
        1,
        &f.path(),
        Some("sleep 300 & echo $!; sleep 300 & echo $!; wait"),
    )
    .await;
    create(&mut window, 2, &f.path(), "wt").await;
    window
        .changed(&wt("wt"), BOUND, running)
        .await
        .expect("setup runs");

    window
        .send(ClientMsg::SetupRun {
            req: 3,
            project: f.path(),
            location: wt("wt"),
        })
        .await;
    let (kind, message) = error(window.answer(3).await);
    assert_eq!(kind, ErrorKind::Busy);
    assert_eq!(message, "Setup is already running in wt.");

    window.send(watch(&f.path(), wt("wt"))).await;
    let pids: Vec<u32> = window
        .output_until(&wt("wt"), |lines| lines.len() >= 2)
        .await
        .iter()
        .take(2)
        .map(|l| l.trim().parse().expect("a pid"))
        .collect();
    assert!(pids.iter().all(|&p| alive(p)), "both sleeps run");

    window
        .send(ClientMsg::SetupStop {
            req: 4,
            project: f.path(),
            location: wt("wt"),
        })
        .await;
    assert!(matches!(
        window.answer(4).await,
        DaemonMsg::OperationOk { .. }
    ));
    assert_eq!(
        window.changed(&wt("wt"), BOUND, |s| s.is_ended()).await,
        Some(SetupState::Stopped)
    );
    assert!(
        pids.iter().all(|&p| !alive(p)),
        "no process of the run's group is left: {pids:?}"
    );

    window
        .send(ClientMsg::SetupStop {
            req: 5,
            project: f.path(),
            location: wt("wt"),
        })
        .await;
    let (kind, message) = error(window.answer(5).await);
    assert_eq!(kind, ErrorKind::NotFound);
    assert_eq!(message, "Setup is not running in wt.");

    set_command(&mut window, 6, &f.path(), Some("pwd -P")).await;
    window
        .send(ClientMsg::SetupRun {
            req: 7,
            project: f.path(),
            location: SessionLocation::Default,
        })
        .await;
    assert!(matches!(
        window.answer(7).await,
        DaemonMsg::OperationOk { .. }
    ));
    window
        .send(watch(&f.path(), SessionLocation::Default))
        .await;
    let lines = window.output_until(&SessionLocation::Default, ended).await;
    assert_eq!(
        lines.first().map(PathBuf::from),
        Some(f.path().canonicalize().unwrap()),
        "Default runs in the project folder: {lines:?}"
    );

    set_command(&mut window, 8, &f.path(), None).await;
    window
        .send(ClientMsg::SetupRun {
            req: 9,
            project: f.path(),
            location: wt("wt"),
        })
        .await;
    let (kind, message) = error(window.answer(9).await);
    assert_eq!(kind, ErrorKind::InvalidInput);
    assert_eq!(message, "This project has no setup command.");
}

/// Wait up to `BOUND` for `pid` to end.
async fn gone(pid: u32) -> bool {
    let deadline = tokio::time::Instant::now() + BOUND;
    while alive(pid) {
        if tokio::time::Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    true
}

/// The pid a `sleep 300 & echo $!; wait` setup at `location` printed first.
async fn first_pid(inbox: &mut Inbox, project: &Path, location: &SessionLocation) -> u32 {
    inbox.send(watch(project, location.clone())).await;
    inbox
        .output_until(location, |lines| !lines.is_empty())
        .await[0]
        .trim()
        .parse()
        .expect("a pid")
}

/// The record stored for `location`.
fn stored(files: &JsonFileStore, project: &Path, location: &SessionLocation) -> Option<SetupRun> {
    files
        .load_setup(project)
        .into_iter()
        .find(|s| &s.location == location)
        .map(|s| s.run)
}

const SLEEPER: &str = "sleep 300 & echo $!; wait";

/// A11, FR-018, FR-019: a `Running` record and its log found at start load as `Interrupted`, the
/// record is rewritten so, and the output stays readable with the reason line last.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_running_record_found_at_start_is_interrupted_with_its_output() {
    let f = Fixture::persistent();
    let files = f.files();
    let running = SetupRun::start(RunId(5), SetupCommand::parse("make").unwrap(), 1);
    files
        .save_setup_record(&f.path(), &SessionLocation::Default, &running)
        .unwrap();
    files
        .append_setup_log(
            &f.path(),
            &SessionLocation::Default,
            &["one".into(), "two".into()],
        )
        .unwrap();

    let state = f.restart();
    let mut window = Inbox::connect(&state).await;
    window
        .send(watch(&f.path(), SessionLocation::Default))
        .await;
    let lines = window.output_until(&SessionLocation::Default, ended).await;
    assert_eq!(lines, ["one", "two", "Interrupted: the daemon stopped."]);
    let record = stored(&files, &f.path(), &SessionLocation::Default).expect("a record");
    assert_eq!(record.state(), &SetupState::Interrupted);
    assert_eq!(record.id(), RunId(5));
}

/// A11, FR-019: a daemon that unwinds while setup runs ends the run's process group and records it
/// `Interrupted`; the next daemon shows that, with the output the run printed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unwind_during_a_run_interrupts_it_and_ends_its_processes() {
    let f = Fixture::persistent();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some(SLEEPER)).await;
    create(&mut window, 2, &f.path(), "wt").await;
    let pid = first_pid(&mut window, &f.path(), &wt("wt")).await;
    assert!(alive(pid), "the setup runs");

    micold_daemon::server::unwind(&f.state, micold_daemon::idle::StopReason::Requested).await;
    assert_eq!(
        stored(&f.files(), &f.path(), &wt("wt")).map(|r| r.state().clone()),
        Some(SetupState::Interrupted),
        "the unwind recorded the run interrupted"
    );
    assert!(gone(pid).await, "the run's process group is ended");

    let state = f.restart();
    let mut later = Inbox::connect(&state).await;
    later.send(watch(&f.path(), wt("wt"))).await;
    let lines = later.output_until(&wt("wt"), ended).await;
    assert_eq!(lines.first(), Some(&pid.to_string()), "{lines:?}");
    assert_eq!(
        lines.last().map(String::as_str),
        Some("Interrupted: the daemon stopped.")
    );
}

/// A11, data-model §RunId: after a restart the next id is `max(counter file, highest record id +
/// 1)`, so a lost counter file never repeats an id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_run_id_after_a_restart_is_past_every_record_and_the_counter() {
    let f = Fixture::persistent();
    let files = f.files();
    let ended = SetupRun::start(RunId(41), SetupCommand::parse("true").unwrap(), 1)
        .finish(SetupState::Succeeded, 2)
        .unwrap();
    files
        .save_setup_record(&f.path(), &SessionLocation::Default, &ended)
        .unwrap();
    let _ = std::fs::remove_file(files.setup_counter_path());

    let next_run = |state: Arc<DaemonState>, req: u64| {
        let project = f.path();
        async move {
            let mut window = Inbox::connect(&state).await;
            set_command(&mut window, req, &project, Some("true")).await;
            window
                .send(ClientMsg::SetupRun {
                    req: req + 1,
                    project,
                    location: SessionLocation::Default,
                })
                .await;
            window
                .changed_run(&SessionLocation::Default, BOUND, running)
                .await
                .expect("a run starts")
                .id()
        }
    };
    assert_eq!(next_run(f.restart(), 1).await, RunId(42), "the record wins");
    assert_eq!(
        files.load_setup_counter(),
        Some(43),
        "the counter is stored"
    );

    files.save_setup_counter(100).unwrap();
    assert_eq!(
        next_run(f.restart(), 3).await,
        RunId(100),
        "the counter wins"
    );
}

/// A12, FR-024 (and M3 review A-F3): deleting a worktree whose setup runs stops it and awaits its
/// end before the worktree goes, then forgets the run: its record and log are gone, a late end
/// sends nothing, and a worktree recreated under the same name runs setup again with a fresh id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deleting_a_worktree_stops_its_setup_first_and_forgets_it() {
    let f = Fixture::persistent();
    let files = f.files();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some(SLEEPER)).await;
    create(&mut window, 2, &f.path(), "wt").await;
    let deleted = window
        .changed_run(&wt("wt"), BOUND, running)
        .await
        .expect("setup runs")
        .id();
    let pid = first_pid(&mut window, &f.path(), &wt("wt")).await;
    assert!(files.setup_record_path(&f.path(), &wt("wt")).exists());

    window
        .send(ClientMsg::WorktreeDelete {
            req: 3,
            project: f.path(),
            dir_name: "wt".into(),
            stop_sessions: true,
            delete_branch: true,
        })
        .await;
    let answer = window.answer(3).await;
    assert!(
        matches!(answer, DaemonMsg::OperationOk { .. }),
        "the delete succeeds: {answer:?}"
    );
    assert!(!alive(pid), "the run had ended when the delete answered");
    let forgotten = window
        .take(BOUND, |m| match m {
            DaemonMsg::SetupChanged {
                location,
                run: None,
                ..
            } if *location == wt("wt") => Some(()),
            _ => None,
        })
        .await;
    assert!(forgotten.is_some(), "the run is forgotten");
    assert!(!files.setup_record_path(&f.path(), &wt("wt")).exists());
    assert!(!files.setup_log_path(&f.path(), &wt("wt")).exists());
    window.seen.clear();
    tokio::time::sleep(micold_core::process::RUN_OVERRUN + Duration::from_millis(500)).await;
    assert!(
        window
            .changed_run(&wt("wt"), Duration::ZERO, |_| true)
            .await
            .is_none(),
        "nothing more is said of the deleted run"
    );
    assert!(!files.setup_log_path(&f.path(), &wt("wt")).exists());

    set_command(&mut window, 4, &f.path(), Some("exit 1")).await;
    create(&mut window, 5, &f.path(), "wt").await;
    let again = window
        .changed_run(&wt("wt"), BOUND, |s| s.is_ended())
        .await
        .expect("setup runs in the recreated worktree");
    assert_eq!(again.state(), &SetupState::Failed(Failure::Exit(1)));
    assert_ne!(
        again.id(),
        deleted,
        "the recreated worktree's run has a fresh id"
    );
}

/// A13, FR-018: a client attaching after runs ended gets one `SetupChanged` per location with a
/// record, right after `Attached`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn attaching_later_gets_each_locations_last_run() {
    let f = Fixture::new();
    let mut window = Inbox::connect(&f.state).await;
    set_command(&mut window, 1, &f.path(), Some("true")).await;
    for (req, dir) in [(2, "a"), (3, "b")] {
        create(&mut window, req, &f.path(), dir).await;
        window
            .changed(&wt(dir), BOUND, |s| s.is_ended())
            .await
            .expect("setup ends");
    }

    let mut later = Inbox::connect(&f.state).await;
    later
        .send(ClientMsg::Attach {
            project: f.path(),
            force: false,
        })
        .await;
    later
        .take(BOUND, |m| {
            matches!(m, DaemonMsg::Attached { .. }).then_some(())
        })
        .await
        .expect("attached");
    later.seen.clear();
    let mut next = Vec::new();
    for _ in 0..2 {
        next.push(
            later
                .take(BOUND, |m| Some(m.clone()))
                .await
                .expect("a message"),
        );
    }
    let mut places: Vec<SessionLocation> = next
        .iter()
        .map(|m| match m {
            DaemonMsg::SetupChanged {
                location,
                run: Some(summary),
                ..
            } => {
                assert_eq!(summary.run.state(), &SetupState::Succeeded);
                location.clone()
            }
            other => panic!("a SetupChanged right after Attached, got {other:?}"),
        })
        .collect();
    places.sort_by_key(|l| format!("{l:?}"));
    assert_eq!(places, [wt("a"), wt("b")]);
}

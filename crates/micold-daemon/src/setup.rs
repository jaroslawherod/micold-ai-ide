//! A project's setup command, run in each worktree the app creates (feature 1289, data-model
//! §Daemon, contracts C1–C2).
//!
//! [`SetupRuns`] lives in the daemon's state, under its own lock (not the daemon-wide one): the commands the app sent with
//! `ProjectSetupSet`, one record and output buffer per location that has run setup, and who watches
//! each location's output. Its operations do no I/O; what they would send goes into an
//! [`Outgoing`] list that [`DaemonState::setup`] delivers while it still holds the lock, so pushes
//! leave in the order the changes were made.
//!
//! A run itself ([`spawn_run`]) runs on the blocking pool and is awaited by nothing: a create
//! answers as soon as the worktree exists (FR-013), and a session starts whether or not setup has
//! ended (FR-014).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use micold_core::process::{ExitKind, Stream, StreamingChild, Terminator};
use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::session::SessionLocation;
use micold_core::setup::output::OutputBuffer;
use micold_core::setup::shell::{bash_on_path, shell_line, ShellPlatform};
use micold_core::setup::{
    Failure, RunId, SetupCommand, SetupRun, SetupRunSummary, SetupStart, SetupState,
};

use crate::state::{ClientId, DaemonState};

/// How often a running setup's new lines go out to its watchers (contract C2).
pub const BATCH_EVERY: Duration = Duration::from_millis(250);

/// The most line bytes one `SetupOutput` carries. The buffer can hold far more than a frame
/// ([`MAX_FRAME_LENGTH`], 16 MiB), so a backlog or a flush goes out as several messages; this
/// budget leaves room for escaping and the envelope (review A-F1).
const BATCH_BYTES: usize = 1024 * 1024;
const _: () = assert!(BATCH_BYTES * 8 < MAX_FRAME_LENGTH);

/// A run's place: the project as this daemon's catalog holds it, and the location in it.
type Key = (PathBuf, SessionLocation);

/// Who a message is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum To {
    /// Every connected client.
    All,
    /// One connection.
    One(ClientId),
}

/// The messages an operation on [`SetupRuns`] sends, in order.
pub type Outgoing = Vec<(To, DaemonMsg)>;

/// Every project's setup command and every location's last run (data-model §Daemon).
#[derive(Default)]
pub struct SetupRuns {
    /// From `ProjectSetupSet`. Memory only (R4): the app sends them again on each connect.
    commands: HashMap<PathBuf, SetupCommand>,
    runs: HashMap<Key, Entry>,
    /// Each watched location's watchers, with the index of the next line each is owed. Apart from
    /// the runs, because a watch outlives a run (a new run keeps it) and may come before the first.
    watchers: HashMap<Key, BTreeMap<ClientId, u64>>,
    /// The last id given out; ids are unique across the daemon, never per location.
    last_id: u64,
}

/// One location's last run.
struct Entry {
    record: SetupRun,
    output: OutputBuffer,
    /// Set once the shell is spawned, dropped once it ended.
    terminator: Option<Terminator>,
    /// **Stop setup** asked for this run: whatever it exits with, it is `Stopped` (FR-023).
    stop_requested: bool,
}

impl Entry {
    fn summary(&self) -> SetupRunSummary {
        SetupRunSummary {
            run: self.record.clone(),
            output_lines: self.output.next_index(),
        }
    }

    fn is_running(&self) -> bool {
        !self.record.state().is_ended()
    }
}

/// What a request to run setup got that is not a start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotStarted {
    /// The project has no setup command.
    NotConfigured,
    /// A run is in progress there (FR-023a).
    Busy,
}

impl From<NotStarted> for SetupStart {
    fn from(why: NotStarted) -> Self {
        match why {
            NotStarted::NotConfigured => Self::NotConfigured,
            NotStarted::Busy => Self::Busy,
        }
    }
}

/// How a run's shell ended, as the run task saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    /// It ran and exited.
    Exited(ExitKind),
    /// It could not be started, with the reason.
    NotStarted(String),
}

/// How `location` reads in a message: the worktree's folder name, or `Default`.
#[must_use]
pub fn place(location: &SessionLocation) -> &str {
    match location {
        SessionLocation::Worktree(dir) => dir,
        SessionLocation::Default => "Default",
    }
}

impl SetupRuns {
    /// Replace `project`'s command; `None` clears it. A run in progress is unaffected.
    pub fn set_command(&mut self, project: &Path, command: Option<SetupCommand>) {
        match command {
            Some(command) => self.commands.insert(project.to_path_buf(), command),
            None => self.commands.remove(project),
        };
    }

    /// Record a new run of `project`'s command at `location`, replacing the last one's record and
    /// output, and tell every client. The caller spawns it ([`spawn_run`]).
    pub fn start(
        &mut self,
        project: &Path,
        location: &SessionLocation,
        now: u64,
        out: &mut Outgoing,
    ) -> Result<(RunId, SetupCommand), NotStarted> {
        let command = self
            .commands
            .get(project)
            .cloned()
            .ok_or(NotStarted::NotConfigured)?;
        let key = (project.to_path_buf(), location.clone());
        if self.runs.get(&key).is_some_and(Entry::is_running) {
            return Err(NotStarted::Busy);
        }
        self.last_id += 1;
        let id = RunId(self.last_id);
        let entry = Entry {
            record: SetupRun::start(id, command.clone(), now),
            output: OutputBuffer::new(),
            terminator: None,
            stop_requested: false,
        };
        out.push((To::All, changed(&key, Some(entry.summary()))));
        self.runs.insert(key.clone(), entry);
        // The new run's output replaces the last one's: each watcher is owed it from the start.
        if let Some(watchers) = self.watchers.get_mut(&key) {
            watchers.values_mut().for_each(|next| *next = 0);
        }
        Ok((id, command))
    }

    /// The run's shell is spawned. `true` when **Stop setup** came first, so the caller ends it
    /// at once.
    pub fn spawned(&mut self, key: &Key, id: RunId, terminator: Terminator) -> bool {
        match self.current(key, id) {
            Some(entry) => {
                entry.terminator = Some(terminator);
                entry.stop_requested
            }
            None => true,
        }
    }

    /// Keep what the run printed.
    pub fn push(&mut self, key: &Key, id: RunId, stream: Stream, bytes: &[u8]) {
        if let Some(entry) = self.current(key, id) {
            entry.output.push(stream, bytes);
        }
    }

    /// Send each watcher of `key` the lines it has not had yet.
    pub fn flush(&mut self, key: &Key, id: RunId, out: &mut Outgoing) {
        let Some(entry) = self.runs.get(key).filter(|e| e.record.id() == id) else {
            return;
        };
        let Some(watchers) = self.watchers.get_mut(key) else {
            return;
        };
        for (conn, next) in watchers.iter_mut() {
            if *next < entry.output.next_index() {
                send_from(key, entry, *next, *conn, out);
                *next = entry.output.next_index();
            }
        }
    }

    /// The run ended: record its state, end its output with the reason line (SC-004), send the
    /// watchers the rest and tell every client. Dropped when the location's record is gone or
    /// belongs to a newer run.
    pub fn finish(&mut self, key: &Key, id: RunId, ended: Ended, now: u64, out: &mut Outgoing) {
        let Some(entry) = self.current(key, id) else {
            return;
        };
        let state = if entry.stop_requested {
            SetupState::Stopped
        } else {
            match ended {
                Ended::Exited(ExitKind::Code(0)) => SetupState::Succeeded,
                Ended::Exited(ExitKind::Code(code)) => SetupState::Failed(Failure::Exit(code)),
                Ended::Exited(ExitKind::Signal(signal)) => {
                    SetupState::Failed(Failure::Signal(signal))
                }
                Ended::Exited(ExitKind::WaitFailed(reason)) | Ended::NotStarted(reason) => {
                    SetupState::Failed(Failure::NotStarted(reason))
                }
            }
        };
        entry.output.finish();
        if let Some(reason) = state.reason_line() {
            entry
                .output
                .push(Stream::Stdout, format!("{reason}\n").as_bytes());
        }
        entry.terminator = None;
        match entry.record.clone().finish(state, now) {
            Ok(record) => entry.record = record,
            Err(refused) => {
                tracing::warn!(?refused, "a setup run's end was refused");
                return;
            }
        }
        self.flush(key, id, out);
        if let Some(entry) = self.runs.get(key) {
            out.push((To::All, changed(key, Some(entry.summary()))));
        }
    }

    /// **Stop setup** (FR-023): mark the run stopped and end its tree. Does not wait. `false` when
    /// nothing runs there.
    pub fn stop(&mut self, project: &Path, location: &SessionLocation) -> bool {
        let key = (project.to_path_buf(), location.clone());
        let Some(entry) = self.runs.get_mut(&key).filter(|e| e.is_running()) else {
            return false;
        };
        entry.stop_requested = true;
        if let Some(terminator) = &entry.terminator {
            terminator.terminate();
        }
        true
    }

    /// `conn` watches `location`'s output from `from_line`: it is sent the backlog at once, when
    /// there is a run, then every later batch (contract C1).
    pub fn watch(
        &mut self,
        conn: ClientId,
        project: &Path,
        location: &SessionLocation,
        from_line: u64,
        out: &mut Outgoing,
    ) {
        let key = (project.to_path_buf(), location.clone());
        let next = match self.runs.get(&key) {
            Some(entry) => {
                let from = from_line.max(entry.output.first_index());
                send_from(&key, entry, from, conn, out);
                entry.output.next_index()
            }
            None => from_line,
        };
        self.watchers.entry(key).or_default().insert(conn, next);
    }

    /// `conn` no longer watches `location`. Idempotent.
    pub fn unwatch(&mut self, conn: ClientId, project: &Path, location: &SessionLocation) {
        let key = (project.to_path_buf(), location.clone());
        if let Some(watchers) = self.watchers.get_mut(&key) {
            watchers.remove(&conn);
            if watchers.is_empty() {
                self.watchers.remove(&key);
            }
        }
    }

    /// `conn`'s connection ended: drop every watch it held.
    pub fn unwatch_all(&mut self, conn: ClientId) {
        self.watchers.retain(|_, watchers| {
            watchers.remove(&conn);
            !watchers.is_empty()
        });
    }

    /// The entry at `key` when it is run `id`'s.
    fn current(&mut self, key: &Key, id: RunId) -> Option<&mut Entry> {
        self.runs.get_mut(key).filter(|e| e.record.id() == id)
    }
}

fn changed(key: &Key, run: Option<SetupRunSummary>) -> DaemonMsg {
    DaemonMsg::SetupChanged {
        project: key.0.clone(),
        location: key.1.clone(),
        run,
    }
}

/// Send `conn` the lines of `entry` from `from` on, in batches of at most [`BATCH_BYTES`] line
/// bytes (a longer line goes alone). Always at least one message, so a watch with nothing to catch
/// up on still learns the run and what was dropped.
fn send_from(key: &Key, entry: &Entry, from: u64, conn: ClientId, out: &mut Outgoing) {
    let (mut first_line, lines) = entry.output.since(from);
    let message = |first_line, lines| DaemonMsg::SetupOutput {
        project: key.0.clone(),
        location: key.1.clone(),
        run: entry.record.id(),
        first_line,
        lines,
        dropped: entry.output.first_index(),
    };
    let mut batch = Vec::new();
    let mut bytes = 0;
    for line in lines {
        if !batch.is_empty() && bytes + line.len() > BATCH_BYTES {
            let sent = batch.len() as u64;
            out.push((
                To::One(conn),
                message(first_line, std::mem::take(&mut batch)),
            ));
            first_line += sent;
            bytes = 0;
        }
        bytes += line.len();
        batch.push(line);
    }
    out.push((To::One(conn), message(first_line, batch)));
}

/// Start `project`'s setup command at `location` (FR-010, FR-022). `None` when this daemon does
/// not know the project or the worktree. Nothing awaits the run.
pub fn start(
    state: &Arc<DaemonState>,
    project: &Path,
    location: SessionLocation,
) -> Option<SetupStart> {
    let dir = state.setup_dir(project, &location)?;
    Some(start_in(state, project, location, dir))
}

/// Start `project`'s setup command at `location`, in `dir`, a folder the caller already knows (a
/// create has the worktree it just made, so it does not look it up again: review A-F4).
pub fn start_in(
    state: &Arc<DaemonState>,
    project: &Path,
    location: SessionLocation,
    dir: PathBuf,
) -> SetupStart {
    let begun = state.setup(|runs, out| {
        runs.start(project, &location, micold_core::unix_time::unix_secs(), out)
    });
    match begun {
        Ok((id, command)) => {
            spawn_run(state, (project.to_path_buf(), location), dir, id, command);
            SetupStart::Started(id)
        }
        Err(why) => why.into(),
    }
}

/// Run `command` in `dir` for run `id`, sending its output to the watchers every
/// [`BATCH_EVERY`], and record how it ended.
fn spawn_run(state: &Arc<DaemonState>, key: Key, dir: PathBuf, id: RunId, command: SetupCommand) {
    let state = Arc::clone(state);
    tokio::spawn(async move {
        let mut run = {
            let state = Arc::clone(&state);
            let key = key.clone();
            tokio::task::spawn_blocking(move || run_blocking(&state, &key, id, &dir, &command))
        };
        let mut tick = tokio::time::interval(BATCH_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let ended = loop {
            tokio::select! {
                ended = &mut run => {
                    break ended.unwrap_or_else(|join| Ended::NotStarted(join.to_string()));
                }
                _ = tick.tick() => state.setup(|runs, out| runs.flush(&key, id, out)),
            }
        };
        state.setup(|runs, out| {
            runs.finish(&key, id, ended, micold_core::unix_time::unix_secs(), out);
        });
    });
}

/// Spawn the shell in `dir` with the environment a session there gets (FR-011, including the
/// sandbox `PATH` of FR-001), its input at end of file (FR-012), and wait for it.
fn run_blocking(
    state: &DaemonState,
    key: &Key,
    id: RunId,
    dir: &Path,
    command: &SetupCommand,
) -> Ended {
    if !dir.is_dir() {
        return Ended::NotStarted(format!("the folder {} does not exist", dir.display()));
    }
    let (vars, path) = state.setup_env_for(dir);
    let platform = if cfg!(windows) {
        ShellPlatform::Windows
    } else {
        ShellPlatform::Unix
    };
    let (program, args) = shell_line(platform, bash_on_path(&path), command);
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(dir).envs(vars);
    micold_core::process::no_window(&mut cmd);
    let child = match StreamingChild::spawn(cmd) {
        Ok(child) => child,
        Err(e) => return Ended::NotStarted(e.to_string()),
    };
    let terminator = child.terminator();
    if state.setup(|runs, _| runs.spawned(key, id, terminator.clone())) {
        terminator.terminate();
    }
    Ended::Exited(child.run(|stream, bytes| {
        state.setup(|runs, _| runs.push(key, id, stream, bytes));
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs_with(command: &str) -> SetupRuns {
        let mut runs = SetupRuns::default();
        runs.set_command(Path::new("/p"), SetupCommand::parse(command));
        runs
    }

    fn key(dir: &str) -> Key {
        (PathBuf::from("/p"), SessionLocation::Worktree(dir.into()))
    }

    #[test]
    fn a_start_without_a_command_is_not_configured_and_sends_nothing() {
        let mut runs = SetupRuns::default();
        let mut out = Vec::new();
        assert_eq!(
            runs.start(Path::new("/p"), &key("a").1, 1, &mut out),
            Err(NotStarted::NotConfigured)
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_second_start_while_running_is_busy_and_ids_are_daemon_wide() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (a, _) = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap();
        assert_eq!(
            runs.start(Path::new("/p"), &key("a").1, 1, &mut out),
            Err(NotStarted::Busy)
        );
        let (b, _) = runs
            .start(Path::new("/p"), &key("b").1, 1, &mut out)
            .unwrap();
        assert_ne!(a, b, "ids are unique across locations");
    }

    #[test]
    fn a_stop_requested_run_is_stopped_whatever_it_exits_with() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (id, _) = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap();
        assert!(runs.stop(Path::new("/p"), &key("a").1));
        runs.finish(&key("a"), id, Ended::Exited(ExitKind::Code(0)), 2, &mut out);
        assert_eq!(runs.runs[&key("a")].record.state(), &SetupState::Stopped);
        assert!(!runs.stop(Path::new("/p"), &key("a").1), "nothing runs now");
    }

    #[test]
    fn a_finish_for_a_replaced_run_is_dropped() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (old, _) = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap();
        runs.finish(
            &key("a"),
            old,
            Ended::Exited(ExitKind::Code(1)),
            2,
            &mut out,
        );
        let (new, _) = runs
            .start(Path::new("/p"), &key("a").1, 3, &mut out)
            .unwrap();
        out.clear();
        runs.finish(
            &key("a"),
            old,
            Ended::Exited(ExitKind::Code(0)),
            4,
            &mut out,
        );
        assert!(out.is_empty(), "the old run's end sends nothing");
        assert_eq!(runs.runs[&key("a")].record.id(), new);
        assert!(runs.runs[&key("a")].is_running());
    }

    #[test]
    fn a_watcher_gets_each_line_once_and_the_reason_line_last() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (id, _) = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap();
        runs.push(&key("a"), id, Stream::Stdout, b"one\n");
        out.clear();
        runs.watch(7, Path::new("/p"), &key("a").1, 0, &mut out);
        runs.push(&key("a"), id, Stream::Stdout, b"two\n");
        runs.flush(&key("a"), id, &mut out);
        runs.flush(&key("a"), id, &mut out);
        runs.finish(&key("a"), id, Ended::Exited(ExitKind::Code(2)), 2, &mut out);
        let lines: Vec<String> = out
            .iter()
            .filter_map(|(to, m)| match m {
                DaemonMsg::SetupOutput { lines, .. } => {
                    assert_eq!(*to, To::One(7));
                    Some(lines.clone())
                }
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(lines, ["one", "two", "Exited with status 2."]);
        runs.unwatch_all(7);
        assert!(runs.watchers.is_empty());
    }

    /// Review A-F1: the buffer holds more than one frame can carry, so a backlog or a flush goes
    /// out as several `SetupOutput`s, each encodable, contiguous, with every line once.
    #[test]
    fn a_backlog_bigger_than_a_frame_goes_out_in_several_encodable_batches() {
        use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (id, _) = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap();
        runs.watch(7, Path::new("/p"), &key("a").1, 0, &mut out);
        let line = format!("{}\n", "x".repeat(4000));
        for _ in 0..5000 {
            runs.push(&key("a"), id, Stream::Stdout, line.as_bytes());
        }
        let check = |out: &Outgoing, conn: ClientId| {
            let mut next = 0;
            let mut sent = 0;
            for (to, msg) in out {
                let DaemonMsg::SetupOutput {
                    first_line, lines, ..
                } = msg
                else {
                    continue;
                };
                if *to != To::One(conn) || lines.is_empty() {
                    continue;
                }
                assert!(serde_json::to_vec(msg).unwrap().len() < MAX_FRAME_LENGTH);
                assert_eq!(*first_line, next, "batches are contiguous");
                next += lines.len() as u64;
                sent += 1;
            }
            assert_eq!(next, 5000, "every line is sent once");
            assert!(sent > 1);
        };
        out.clear();
        runs.flush(&key("a"), id, &mut out);
        check(&out, 7);
        out.clear();
        runs.watch(8, Path::new("/p"), &key("a").1, 0, &mut out);
        check(&out, 8);
    }
}

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
//! ended (FR-014). Only a delete waits for it ([`stop_and_wait`], FR-024). On an app-owned sandbox
//! only one run's shell runs at a time, the others waiting their turn in order (#1439, contract
//! setup-turn): concurrent installs into the one home break.
//!
//! Each location's last run is kept on disk (M4, FR-016, FR-018): its record, its output log and
//! the daemon-wide run id counter, through [`SetupStore`]. The operations say what to write; the
//! callers write it after the lock is released. Each write is numbered under the lock, and the
//! store drops one older than the last it wrote for that location (M4 review A-F2), so a late
//! write never undoes a newer one.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use micold_core::process::{ExitKind, Stream, StreamingChild, Terminator};
use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::session::SessionLocation;
use micold_core::setup::output::{OutputBuffer, KEEP_LINES};
use micold_core::setup::shell::{bash_on_path, shell_line, ShellPlatform};
use micold_core::setup::{
    Failure, RunId, SetupCommand, SetupRun, SetupRunSummary, SetupStart, SetupState,
};
use micold_core::store::{JsonFileStore, SetupLog, StoredSetup};
use tokio::sync::watch;

use crate::state::{ClientId, DaemonState};

/// How often a running setup's new lines go out to its watchers (contract C2).
pub const BATCH_EVERY: Duration = Duration::from_millis(250);

/// The most line bytes one `SetupOutput` carries. The buffer can hold far more than a frame
/// ([`MAX_FRAME_LENGTH`], 16 MiB), so a backlog or a flush goes out as several messages; this
/// budget leaves room for escaping and the envelope (review A-F1).
const BATCH_BYTES: usize = 1024 * 1024;
const _: () = assert!(BATCH_BYTES * 8 < MAX_FRAME_LENGTH);

/// A log grows to this many lines before it is cut back to the buffer's last [`KEEP_LINES`]
/// (FR-021): rewriting it on every line past the bound would cost a full write per line.
const LOG_COMPACT_AT: u64 = 20_000;
const _: () = assert!(LOG_COMPACT_AT > KEEP_LINES as u64);

/// The first line of a run that waits for another setup on this daemon to end (#1439, contract
/// setup-turn S2).
pub const WAITING_LINE: &str = "Waiting for another setup on this daemon to finish…";

/// A run's place: the project as this daemon's catalog holds it, and the location in it.
type Key = (PathBuf, SessionLocation);

/// What a location's log needs written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogWrite {
    /// These lines, at its end.
    Append(Vec<String>),
    /// This log, in place of what it holds (a new run, or a compaction).
    Replace(SetupLog),
}

/// What one location needs written: its record, its log, or both.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Persist {
    /// Issued under the setup lock, in the order the changes were made: [`SetupStore`] drops a
    /// write older than the last one it wrote for the location (M4 review A-F2).
    pub seq: u64,
    pub record: Option<SetupRun>,
    pub log: Option<LogWrite>,
}

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
pub struct SetupRuns {
    /// From `ProjectSetupSet`. Memory only (R4): the app sends them again on each connect.
    commands: HashMap<PathBuf, SetupCommand>,
    runs: HashMap<Key, Entry>,
    /// Each watched location's watchers, with the index of the next line each is owed. Apart from
    /// the runs, because a watch outlives a run (a new run keeps it) and may come before the first.
    watchers: HashMap<Key, BTreeMap<ClientId, u64>>,
    /// The next id to give out; ids are unique across the daemon, never per location.
    next_id: u64,
    /// The last [`Persist::seq`] given out.
    issued: u64,
    /// The daemon is stopping: no run starts (M4 review A-F1).
    stopping: bool,
    /// Locations a delete holds: no run starts there until the delete ends (M4 review A-F4).
    deleting: HashSet<Key>,
}

impl Default for SetupRuns {
    fn default() -> Self {
        Self {
            commands: HashMap::new(),
            runs: HashMap::new(),
            watchers: HashMap::new(),
            next_id: 1,
            issued: 0,
            stopping: false,
            deleting: HashSet::new(),
        }
    }
}

/// One location's last run.
struct Entry {
    record: SetupRun,
    output: OutputBuffer,
    /// Set once the shell is spawned, dropped once it ended.
    terminator: Option<Terminator>,
    /// **Stop setup** asked for this run: whatever it exits with, it is `Stopped` (FR-023).
    stop_requested: bool,
    /// The index of the first line the location's log holds.
    log_first: u64,
    /// Becomes `true` once the run task wrote its end; `None` for a run loaded from disk, which
    /// has no task.
    done: Option<watch::Receiver<bool>>,
    /// Set by every path that ends the run (**Stop setup**, a delete, a daemon stop), and dropped
    /// when the run is forgotten: a run waiting for the setup turn ends at once (#1439, S3).
    stop: watch::Sender<bool>,
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

    /// End the output with `state`'s reason line and say what the log needs: the lines kept since
    /// the last write, or the whole buffer once the log has grown past [`LOG_COMPACT_AT`].
    fn end_output(&mut self, state: &SetupState) -> Vec<(u64, String)> {
        let mut done = self.output.finish();
        if let Some(reason) = state.reason_line() {
            done.extend(
                self.output
                    .push(Stream::Stdout, format!("{reason}\n").as_bytes()),
            );
        }
        done
    }

    /// What the log needs for `lines`, just kept.
    fn log_write(&mut self, lines: Vec<(u64, String)>) -> Option<LogWrite> {
        if lines.is_empty() {
            return None;
        }
        if self.output.next_index() - self.log_first > LOG_COMPACT_AT {
            return Some(self.whole_log());
        }
        Some(LogWrite::Append(
            lines.into_iter().map(|(_, line)| line).collect(),
        ))
    }

    /// The log cut back to the buffer: what it holds, numbered on from the lines dropped.
    fn whole_log(&mut self) -> LogWrite {
        self.log_first = self.output.first_index();
        LogWrite::Replace(SetupLog {
            first_line: self.log_first,
            lines: self.output.since(0).1,
        })
    }
}

/// The next [`Persist::seq`].
fn issue(issued: &mut u64) -> u64 {
    *issued += 1;
    *issued
}

/// A run [`SetupRuns::start`] recorded, for the caller to persist and spawn.
pub struct Begun {
    pub id: RunId,
    pub command: SetupCommand,
    /// The next id after this one, for the counter file.
    pub next_id: u64,
    /// The record to write, and the log emptied.
    pub persist: Persist,
    /// Set by the run task once its end is written.
    pub done: watch::Sender<bool>,
    /// Becomes `true`, or closes, when the run is ended before its shell spawned.
    pub stop: watch::Receiver<bool>,
}

/// What a request to run setup got that is not a start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotStarted {
    /// The project has no setup command.
    NotConfigured,
    /// A run is in progress there (FR-023a), the worktree is being deleted, or the daemon is
    /// stopping: each a reason to try later.
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
    ) -> Result<Begun, NotStarted> {
        let command = self
            .commands
            .get(project)
            .cloned()
            .ok_or(NotStarted::NotConfigured)?;
        let key = (project.to_path_buf(), location.clone());
        if self.stopping
            || self.deleting.contains(&key)
            || self.runs.get(&key).is_some_and(Entry::is_running)
        {
            return Err(NotStarted::Busy);
        }
        let id = RunId(self.next_id);
        self.next_id += 1;
        let (done, done_rx) = watch::channel(false);
        let (stop_tx, stop) = watch::channel(false);
        let entry = Entry {
            record: SetupRun::start(id, command.clone(), now),
            output: OutputBuffer::new(),
            terminator: None,
            stop_requested: false,
            log_first: 0,
            done: Some(done_rx),
            stop: stop_tx,
        };
        let persist = Persist {
            seq: issue(&mut self.issued),
            record: Some(entry.record.clone()),
            log: Some(LogWrite::Replace(SetupLog::default())),
        };
        out.push((To::All, changed(&key, Some(entry.summary()))));
        self.runs.insert(key.clone(), entry);
        // The new run's output replaces the last one's: each watcher is owed it from the start.
        if let Some(watchers) = self.watchers.get_mut(&key) {
            watchers.values_mut().for_each(|next| *next = 0);
        }
        Ok(Begun {
            id,
            command,
            next_id: self.next_id,
            persist,
            done,
            stop,
        })
    }

    /// The run's shell is spawned. `true` when **Stop setup** came first, or the run has already
    /// ended (an unwind recorded it interrupted: M4 review A-F1), so the caller ends it at once.
    pub fn spawned(&mut self, key: &Key, id: RunId, terminator: Terminator) -> bool {
        match self.current(key, id) {
            Some(entry) => {
                entry.terminator = Some(terminator);
                !entry.is_running() || entry.stop_requested
            }
            None => true,
        }
    }

    /// Keep what the run printed, and say what its log needs. Nothing once the run's entry is
    /// gone, replaced or ended, so a late line never re-creates a deleted log.
    pub fn push(&mut self, key: &Key, id: RunId, stream: Stream, bytes: &[u8]) -> Option<Persist> {
        let entry = self
            .runs
            .get_mut(key)
            .filter(|e| e.record.id() == id && e.is_running())?;
        let lines = entry.output.push(stream, bytes);
        let log = entry.log_write(lines)?;
        Some(Persist {
            seq: issue(&mut self.issued),
            record: None,
            log: Some(log),
        })
    }

    /// Run `id` waits for the daemon's setup turn (#1439, contract setup-turn S2): its output
    /// begins with [`WAITING_LINE`], kept and logged as a line it printed and sent to its watchers
    /// now. Says what its log needs.
    pub fn waiting(&mut self, key: &Key, id: RunId, out: &mut Outgoing) -> Option<Persist> {
        let line = format!("{WAITING_LINE}\n");
        let persist = self.push(key, id, Stream::Stdout, line.as_bytes());
        self.flush(key, id, out);
        persist
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
    /// watchers the rest and tell every client, and say what to write. Dropped when the
    /// location's record is gone, belongs to a newer run, or has already ended (an unwind
    /// recorded it interrupted).
    pub fn finish(
        &mut self,
        key: &Key,
        id: RunId,
        ended: Ended,
        now: u64,
        out: &mut Outgoing,
    ) -> Option<Persist> {
        let entry = self.current(key, id).filter(|e| e.is_running())?;
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
        let lines = entry.end_output(&state);
        let log = entry.log_write(lines);
        entry.terminator = None;
        match entry.record.clone().finish(state, now) {
            Ok(record) => entry.record = record,
            Err(refused) => {
                tracing::warn!(?refused, "a setup run's end was refused");
                return None;
            }
        }
        let record = entry.record.clone();
        self.flush(key, id, out);
        if let Some(entry) = self.runs.get(key) {
            out.push((To::All, changed(key, Some(entry.summary()))));
        }
        Some(Persist {
            seq: issue(&mut self.issued),
            record: Some(record),
            log,
        })
    }

    /// **Stop setup** (FR-023): mark the run stopped and end its tree. Does not wait. `false` when
    /// nothing runs there.
    pub fn stop(&mut self, project: &Path, location: &SessionLocation) -> bool {
        let key = (project.to_path_buf(), location.clone());
        let Some(entry) = self.runs.get_mut(&key).filter(|e| e.is_running()) else {
            return false;
        };
        entry.stop_requested = true;
        entry.stop.send_replace(true);
        if let Some(terminator) = &entry.terminator {
            terminator.terminate();
        }
        true
    }

    /// A delete of `location` begins (FR-024): no run starts there until [`Self::end_delete`]
    /// (M4 review A-F4), and the one running is stopped. Returns its task's end to await; `None`
    /// when nothing runs there.
    pub fn begin_delete(
        &mut self,
        project: &Path,
        location: &SessionLocation,
    ) -> Option<watch::Receiver<bool>> {
        let key = (project.to_path_buf(), location.clone());
        self.deleting.insert(key.clone());
        if !self.stop(project, location) {
            return None;
        }
        self.runs.get(&key).and_then(|entry| entry.done.clone())
    }

    /// The delete [`Self::begin_delete`] began has ended, whether or not it removed anything.
    pub fn end_delete(&mut self, project: &Path, location: &SessionLocation) {
        self.deleting
            .remove(&(project.to_path_buf(), location.clone()));
    }

    /// The daemon is stopping (FR-019): end every live run's tree and record it `Interrupted`,
    /// with the reason line last. Returns what each location needs written. A run task's own end,
    /// coming later, is dropped; a shell spawned later is ended at once, and no run starts after
    /// this (M4 review A-F1).
    pub fn interrupt_all(&mut self, now: u64, out: &mut Outgoing) -> Vec<(Key, Persist)> {
        self.stopping = true;
        let mut writes = Vec::new();
        for (key, entry) in self.runs.iter_mut().filter(|(_, e)| e.is_running()) {
            entry.stop_requested = true;
            entry.stop.send_replace(true);
            if let Some(terminator) = entry.terminator.take() {
                terminator.terminate();
            }
            let Some(persist) = interrupt(entry, now, issue(&mut self.issued)) else {
                continue;
            };
            out.push((To::All, changed(key, Some(entry.summary()))));
            writes.push((key.clone(), persist));
        }
        writes
    }

    /// The worktree at `location` was deleted: forget its run, ending its tree if it still runs
    /// (M4 review A-F4), and tell every client (`SetupChanged { run: None }`). The caller removes
    /// its files, as the write numbered by the returned [`Persist::seq`].
    pub fn forget(
        &mut self,
        project: &Path,
        location: &SessionLocation,
        out: &mut Outgoing,
    ) -> u64 {
        let key = (project.to_path_buf(), location.clone());
        if let Some(entry) = self.runs.remove(&key) {
            if let Some(terminator) = entry
                .terminator
                .filter(|_| !entry.record.state().is_ended())
            {
                terminator.terminate();
            }
            out.push((To::All, changed(&key, None)));
        }
        issue(&mut self.issued)
    }

    /// The project was removed: end its runs (not waiting) and forget them and its command.
    pub fn forget_project(&mut self, project: &Path, out: &mut Outgoing) {
        self.commands.remove(project);
        let keys: Vec<Key> = self
            .runs
            .keys()
            .filter(|(p, _)| p == project)
            .cloned()
            .collect();
        for key in keys {
            self.stop(&key.0, &key.1);
            self.forget(&key.0, &key.1, out);
        }
        self.watchers.retain(|(p, _), _| p != project);
    }

    /// What a client attaching to `project` is sent (FR-018): one `SetupChanged` per location
    /// with a record.
    #[must_use]
    pub fn on_attach(&self, project: &Path) -> Vec<DaemonMsg> {
        let mut found: Vec<(&Key, &Entry)> = self
            .runs
            .iter()
            .filter(|((p, _), _)| p == project)
            .collect();
        found.sort_by_key(|(key, _)| place(&key.1).to_owned());
        found
            .into_iter()
            .map(|(key, entry)| changed(key, Some(entry.summary())))
            .collect()
    }

    /// The runs a daemon found on disk at start (FR-018, FR-019): each project's records and logs,
    /// and the stored counter. A record left `Running` is `Interrupted` now, and its rewrite comes
    /// back to be written. The next id is past both the counter and every record (data-model
    /// §RunId).
    pub fn loaded(
        stored: Vec<(PathBuf, StoredSetup)>,
        counter: Option<u64>,
        now: u64,
    ) -> (Self, Vec<(Key, Persist)>) {
        let mut runs = Self::default();
        let mut writes = Vec::new();
        let highest = stored.iter().map(|(_, s)| s.run.id().0).max().unwrap_or(0);
        runs.next_id = counter.unwrap_or(1).max(highest + 1);
        for (project, setup) in stored {
            let key = (project, setup.location);
            let mut entry = Entry {
                record: setup.run,
                output: OutputBuffer::restored(setup.log.first_line, setup.log.lines),
                terminator: None,
                stop_requested: false,
                log_first: setup.log.first_line,
                done: None,
                stop: watch::channel(false).0,
            };
            if entry.is_running() {
                if let Some(persist) = interrupt(&mut entry, now, issue(&mut runs.issued)) {
                    writes.push((key.clone(), persist));
                }
            }
            runs.runs.insert(key, entry);
        }
        (runs, writes)
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

/// Record `entry`'s run `Interrupted` (FR-019) and end its output with the reason line, as write
/// `seq`. The log is rewritten whole, not appended to: a line write of the run issued before this
/// one and landing after it is dropped (M4 review A-F2), and this write holds its lines.
fn interrupt(entry: &mut Entry, now: u64, seq: u64) -> Option<Persist> {
    let state = SetupState::Interrupted;
    entry.end_output(&state);
    let log = entry.whole_log();
    entry.record = entry.record.clone().finish(state, now).ok()?;
    Some(Persist {
        seq,
        record: Some(entry.record.clone()),
        log: Some(log),
    })
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
        Ok(begun) => {
            let key = (project.to_path_buf(), location);
            if let Some(store) = state.setup_store() {
                // The counter before the record (data-model §RunId); the last run's log is replaced.
                store.write_counter(begun.next_id);
                store.write(&key, &begun.persist);
            }
            let id = begun.id;
            spawn_run(state, key, dir, id, begun.command, begun.done, begun.stop);
            SetupStart::Started(id)
        }
        Err(why) => why.into(),
    }
}

/// A delete's hold on a location: no run starts there while it is held (M4 review A-F4).
#[must_use = "dropping the hold lets a run start there again"]
pub struct DeleteHold {
    state: Arc<DaemonState>,
    key: Key,
}

impl Drop for DeleteHold {
    fn drop(&mut self) {
        let (project, location) = &self.key;
        self.state
            .setup(|runs, _| runs.end_delete(project, location));
    }
}

/// Hold `location` for a delete, stop the run there, if one runs, and wait for its task to write
/// its end, at most [`micold_core::process::RUN_OVERRUN`] (the kill-and-drain bound). At once when
/// nothing runs. A delete calls it before removing the worktree (FR-024) and keeps the hold until
/// it has forgotten the run, or failed.
pub async fn stop_and_wait(
    state: &Arc<DaemonState>,
    project: &Path,
    location: &SessionLocation,
) -> DeleteHold {
    let hold = DeleteHold {
        state: Arc::clone(state),
        key: (project.to_path_buf(), location.clone()),
    };
    let Some(mut done) = state.setup(|runs, _| runs.begin_delete(project, location)) else {
        return hold;
    };
    let ended = tokio::time::timeout(
        micold_core::process::RUN_OVERRUN,
        done.wait_for(|done| *done),
    )
    .await;
    if ended.is_err() {
        tracing::warn!(
            project = %project.display(),
            location = place(location),
            "a stopped setup run had not ended in time; it is forgotten anyway"
        );
    }
    hold
}

/// Forget the run at `location` and remove its files: the worktree was deleted (FR-016).
pub fn forget(state: &DaemonState, project: &Path, location: &SessionLocation) {
    let seq = state.setup(|runs, out| runs.forget(project, location, out));
    if let Some(store) = state.setup_store() {
        store.remove(project, location, seq);
    }
}

/// The daemon is stopping: record every live run `Interrupted` and end its tree (FR-019).
pub fn interrupt_all(state: &DaemonState) {
    let writes =
        state.setup(|runs, out| runs.interrupt_all(micold_core::unix_time::unix_secs(), out));
    if let Some(store) = state.setup_store() {
        for (key, persist) in &writes {
            store.write(key, persist);
        }
    }
}

/// Where setup's records, logs and the run id counter are written (M4). Absent for the ephemeral
/// catalog, which keeps the counter in memory.
pub struct SetupStore {
    files: crate::catalog::SetupFiles,
    /// The highest counter value written; held across the write, so two starts racing to the file
    /// never lower it.
    counter: Mutex<u64>,
    /// Each location's last [`Persist::seq`] written (or removed); held across the write, so a
    /// write issued earlier and landing later is dropped (M4 review A-F2).
    written: Mutex<HashMap<Key, u64>>,
}

impl SetupStore {
    #[must_use]
    pub fn new(files: crate::catalog::SetupFiles) -> Self {
        Self {
            files,
            counter: Mutex::new(0),
            written: Mutex::new(HashMap::new()),
        }
    }

    /// Every stored run of `projects`, and the stored counter, as a daemon finds them at start;
    /// the `Running` ones found are written back `Interrupted`.
    #[must_use]
    pub fn load(&self, projects: impl IntoIterator<Item = PathBuf>) -> SetupRuns {
        let files = self.files.get();
        let stored = projects
            .into_iter()
            .flat_map(|project| {
                files
                    .load_setup(&project)
                    .into_iter()
                    .map(move |setup| (project.clone(), setup))
            })
            .collect();
        let (runs, writes) = SetupRuns::loaded(
            stored,
            files.load_setup_counter(),
            micold_core::unix_time::unix_secs(),
        );
        for (key, persist) in &writes {
            self.write(key, persist);
        }
        runs
    }

    fn write_counter(&self, next: u64) {
        let mut written = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        if next <= *written {
            return;
        }
        match self.files.get().save_setup_counter(next) {
            Ok(()) => *written = next,
            Err(err) => tracing::warn!(%err, "the setup run id counter was not written"),
        }
    }

    /// Run `write` as `key`'s write `seq`, unless a later one was already written there.
    fn in_order(&self, key: &Key, seq: u64, write: impl FnOnce(&JsonFileStore)) {
        let mut written = self.written.lock().unwrap_or_else(|e| e.into_inner());
        if written.get(key).is_some_and(|&last| last >= seq) {
            tracing::debug!(
                location = place(&key.1),
                seq,
                "a stale setup write was dropped"
            );
            return;
        }
        write(self.files.get());
        written.insert(key.clone(), seq);
    }

    /// Write what `key` needs: the record and the log each, the one whatever became of the other
    /// (M4 review A-F3). A failure is logged: the run goes on, and memory keeps it.
    fn write(&self, key: &Key, persist: &Persist) {
        let (project, location) = key;
        let warn = |what: &str, err: std::io::Error| {
            tracing::warn!(
                project = %project.display(),
                location = place(location),
                %err,
                "a setup {what} was not written"
            );
        };
        self.in_order(key, persist.seq, |files| {
            if let Some(record) = &persist.record {
                if let Err(err) = files.save_setup_record(project, location, record) {
                    warn("record", err);
                }
            }
            let log = match &persist.log {
                None => Ok(()),
                Some(LogWrite::Append(lines)) => files.append_setup_log(project, location, lines),
                Some(LogWrite::Replace(log)) => files.write_setup_log(project, location, log),
            };
            if let Err(err) = log {
                warn("log", err);
            }
        });
    }

    /// Remove `location`'s files, as its write `seq`.
    fn remove(&self, project: &Path, location: &SessionLocation, seq: u64) {
        let key = (project.to_path_buf(), location.clone());
        self.in_order(&key, seq, |files| {
            if let Err(err) = files.remove_setup(project, Some(location)) {
                tracing::warn!(
                    project = %project.display(),
                    location = place(location),
                    %err,
                    "a deleted worktree's setup files were not removed"
                );
            }
        });
    }
}

/// Run `command` in `dir` for run `id`, sending its output to the watchers every
/// [`BATCH_EVERY`], and record how it ended.
fn spawn_run(
    state: &Arc<DaemonState>,
    key: Key,
    dir: PathBuf,
    id: RunId,
    command: SetupCommand,
    done: watch::Sender<bool>,
    stop: watch::Receiver<bool>,
) {
    let shell = {
        let state = Arc::clone(state);
        let key = key.clone();
        move || run_blocking(&state, &key, id, &dir, &command)
    };
    spawn_run_with(state, key, id, done, stop, shell);
}

/// [`spawn_run`] with the blocking part, `shell`, given: on a sandbox daemon it runs only while
/// this run holds the setup turn (#1439, contract setup-turn S2), which is let go once the run's
/// end is written, however it ended, or once the run is forgotten.
fn spawn_run_with(
    state: &Arc<DaemonState>,
    key: Key,
    id: RunId,
    done: watch::Sender<bool>,
    mut stop: watch::Receiver<bool>,
    shell: impl FnOnce() -> Ended + Send + 'static,
) {
    let state = Arc::clone(state);
    // Here, not in the task: runs join the turn's queue in the order they asked (review A-F3).
    let queued = join_turn(&state, &key, id);
    tokio::spawn(async move {
        let (ended, turn) = match take_turn(&state, &key, queued, &mut stop).await {
            Turn::Go(mut turn) => {
                let ended = run_shell(&state, &key, id, shell, &mut stop, &mut turn).await;
                (ended, turn)
            }
            // Recorded `Stopped`, or dropped when the run was interrupted or forgotten (S3).
            Turn::Stopped => (
                Ended::NotStarted("stopped while waiting for another setup".into()),
                None,
            ),
        };
        // The end is written before `done`, so a delete that waited finds the files to remove.
        let writer = Arc::clone(&state);
        let _ = tokio::task::spawn_blocking(move || {
            let persist = writer.setup(|runs, out| {
                runs.finish(&key, id, ended, micold_core::unix_time::unix_secs(), out)
            });
            if let (Some(persist), Some(store)) = (persist, writer.setup_store()) {
                store.write(&key, &persist);
            }
        })
        .await;
        drop(turn);
        let _ = done.send(true);
    });
}

/// The daemon's setup turn, owned.
type TurnGuard = tokio::sync::OwnedMutexGuard<()>;

/// A wait for the setup turn that has its place in the queue.
type TurnLock = Pin<Box<dyn Future<Output = TurnGuard> + Send>>;

/// Where a run stands with the daemon's setup turn once it has asked for it.
enum Queued {
    /// The daemon has no turn (S1): host and remote daemons run setups side by side.
    NoTurn,
    /// The turn was free and is this run's.
    Held(TurnGuard),
    /// In the turn's queue behind another setup, its place kept by the pending lock; the waiting
    /// line is in the run's output, and in its record to write when there is one.
    Waiting(TurnLock, Option<Persist>),
}

/// Ask for the daemon's setup turn for run `id`, when it has one (S1). The first poll of the lock
/// takes the run's place in the fair queue, so the order runs ask in is the order they run in.
/// When another setup holds the turn, record [`WAITING_LINE`] (S2).
fn join_turn(state: &DaemonState, key: &Key, id: RunId) -> Queued {
    let Some(turn) = state.setup_turn() else {
        return Queued::NoTurn;
    };
    let mut lock: TurnLock = Box::pin(turn.lock_owned());
    // The task's own poll later replaces the waker; the place in the queue stays.
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    match lock.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(guard) => Queued::Held(guard),
        std::task::Poll::Pending => {
            let persist = state.setup(|runs, out| runs.waiting(key, id, out));
            Queued::Waiting(lock, persist)
        }
    }
}

/// Whether a run may spawn its shell.
enum Turn {
    /// Yes, holding the daemon's setup turn when it has one.
    Go(Option<TurnGuard>),
    /// No: it was ended while it waited, or before its turn came (S3).
    Stopped,
}

/// Wait, when `queued` is in the queue, until the turn comes or a stop path ends the run.
async fn take_turn(
    state: &Arc<DaemonState>,
    key: &Key,
    queued: Queued,
    stop: &mut watch::Receiver<bool>,
) -> Turn {
    let guard = match queued {
        Queued::NoTurn => return Turn::Go(None),
        Queued::Held(guard) => guard,
        Queued::Waiting(lock, persist) => {
            if let Some(persist) = persist.filter(|_| state.setup_store().is_some()) {
                let writer = Arc::clone(state);
                let key = key.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Some(store) = writer.setup_store() {
                        store.write(&key, &persist);
                    }
                })
                .await;
            }
            tokio::select! {
                guard = lock => guard,
                _ = stop.wait_for(|stopped| *stopped) => return Turn::Stopped,
            }
        }
    };
    // Stopped, or forgotten (the sender gone), before the turn came: no shell (S3).
    if *stop.borrow() || stop.has_changed().is_err() {
        return Turn::Stopped;
    }
    Turn::Go(Some(guard))
}

/// Run `shell` on the blocking pool, sending the run's new lines to its watchers every
/// [`BATCH_EVERY`] until it ends. A run forgotten meanwhile (its stop sender gone: a delete that
/// stopped waiting for it, [`stop_and_wait`]) lets `turn` go at once: nothing can **Stop** it any
/// more, so a tree that escaped the kill must not hold every later setup (review A-F2).
async fn run_shell(
    state: &Arc<DaemonState>,
    key: &Key,
    id: RunId,
    shell: impl FnOnce() -> Ended + Send + 'static,
    stop: &mut watch::Receiver<bool>,
    turn: &mut Option<TurnGuard>,
) -> Ended {
    let mut run = tokio::task::spawn_blocking(shell);
    let mut tick = tokio::time::interval(BATCH_EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            ended = &mut run => {
                break ended.unwrap_or_else(|join| Ended::NotStarted(join.to_string()));
            }
            _ = tick.tick() => state.setup(|runs, out| runs.flush(key, id, out)),
            changed = stop.changed(), if turn.is_some() => {
                if changed.is_err() {
                    tracing::warn!(
                        project = %key.0.display(),
                        location = place(&key.1),
                        "a forgotten setup run still runs; the setup turn goes to the next run"
                    );
                    *turn = None;
                }
            }
        }
    }
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
        let persist = state.setup(|runs, _| runs.push(key, id, stream, bytes));
        if let (Some(persist), Some(store)) = (persist, state.setup_store()) {
            store.write(key, &persist);
        }
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
            runs.start(Path::new("/p"), &key("a").1, 1, &mut out).err(),
            Some(NotStarted::NotConfigured)
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_second_start_while_running_is_busy_and_ids_are_daemon_wide() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let a = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        assert_eq!(
            runs.start(Path::new("/p"), &key("a").1, 1, &mut out).err(),
            Some(NotStarted::Busy)
        );
        let b = runs
            .start(Path::new("/p"), &key("b").1, 1, &mut out)
            .unwrap()
            .id;
        assert_ne!(a, b, "ids are unique across locations");
    }

    #[test]
    fn a_stop_requested_run_is_stopped_whatever_it_exits_with() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        assert!(runs.stop(Path::new("/p"), &key("a").1));
        runs.finish(&key("a"), id, Ended::Exited(ExitKind::Code(0)), 2, &mut out);
        assert_eq!(runs.runs[&key("a")].record.state(), &SetupState::Stopped);
        assert!(!runs.stop(Path::new("/p"), &key("a").1), "nothing runs now");
    }

    #[test]
    fn a_finish_for_a_replaced_run_is_dropped() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let old = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        runs.finish(
            &key("a"),
            old,
            Ended::Exited(ExitKind::Code(1)),
            2,
            &mut out,
        );
        let new = runs
            .start(Path::new("/p"), &key("a").1, 3, &mut out)
            .unwrap()
            .id;
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
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
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
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
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

    /// A shell of `secs` seconds, for a test that needs a live tree.
    #[cfg(unix)]
    fn sleeper(secs: u32) -> StreamingChild {
        let mut cmd = Command::new("sleep");
        cmd.arg(secs.to_string());
        StreamingChild::spawn(cmd).unwrap()
    }

    /// The setup files of a store in `dir`.
    fn store_in(dir: &Path) -> SetupStore {
        let catalog = crate::catalog::Catalog::load(
            Box::new(micold_core::store::JsonFileStore::at(
                dir.join("projects.json"),
            )),
            Box::new(micold_core::settings::JsonFileSettingsStore::at(
                dir.join("settings.json"),
            )),
        );
        SetupStore::new(catalog.setup_files().expect("a file store has setup files"))
    }

    /// M4 review A-F1: a shell spawned after the unwind recorded its run `Interrupted` is ended at
    /// once, and no run starts once the daemon is stopping.
    #[cfg(unix)]
    #[test]
    fn a_shell_spawned_after_an_unwind_is_ended_and_nothing_starts_after_it() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        runs.interrupt_all(2, &mut out);
        let child = sleeper(10);
        assert!(
            runs.spawned(&key("a"), id, child.terminator()),
            "the caller is told to end it"
        );
        assert_eq!(
            runs.start(Path::new("/p"), &key("b").1, 3, &mut out).err(),
            Some(NotStarted::Busy)
        );
    }

    /// M4 review A-F4: forgetting a location whose run is still live ends its tree, so nothing is
    /// left running that **Stop setup** can no longer reach.
    #[cfg(unix)]
    #[test]
    fn forgetting_a_live_run_ends_its_tree() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        let child = sleeper(10);
        assert!(!runs.spawned(&key("a"), id, child.terminator()));
        runs.forget(Path::new("/p"), &key("a").1, &mut out);
        assert!(
            matches!(child.run(|_, _| {}), ExitKind::Signal(_)),
            "the forgotten run's tree is ended"
        );
    }

    /// M4 review A-F3: a record that cannot be written does not keep a new run's log from being
    /// replaced, so its lines never land after the last run's.
    #[test]
    fn a_record_write_that_fails_still_replaces_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let (project, location) = key("a");
        let files = store.files.get();
        files
            .append_setup_log(&project, &location, &["old".into()])
            .unwrap();
        std::fs::create_dir_all(files.setup_record_path(&project, &location)).unwrap();
        let mut runs = runs_with("true");
        let begun = runs.start(&project, &location, 1, &mut Vec::new()).unwrap();
        store.write(&key("a"), &begun.persist);
        let log = std::fs::read_to_string(files.setup_log_path(&project, &location)).unwrap();
        assert!(
            !log.lines().any(|line| line == "old"),
            "the last run's log is replaced: {log:?}"
        );
    }

    /// M4 review A-F4: while a delete holds a location no run starts there, whatever the gate.
    #[test]
    fn a_location_being_deleted_starts_no_run() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let (project, location) = key("a");
        assert!(runs.begin_delete(&project, &location).is_none());
        assert_eq!(
            runs.start(&project, &location, 1, &mut out).err(),
            Some(NotStarted::Busy)
        );
        runs.end_delete(&project, &location);
        assert!(runs.start(&project, &location, 2, &mut out).is_ok());
    }

    /// M4 review A-F2: writes reach the store in the order they were issued under the lock. Run
    /// N's end landing after run N+1's start is dropped, and so is a line write landing after the
    /// delete removed the files.
    #[test]
    fn a_write_issued_before_the_last_one_written_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(dir.path());
        let (project, location) = key("a");
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let first = runs.start(&project, &location, 1, &mut out).unwrap();
        store.write(&key("a"), &first.persist);
        let ended = runs
            .finish(
                &key("a"),
                first.id,
                Ended::Exited(ExitKind::Code(0)),
                2,
                &mut out,
            )
            .unwrap();
        let second = runs.start(&project, &location, 3, &mut out).unwrap();
        store.write(&key("a"), &second.persist);
        store.write(&key("a"), &ended);
        let files = store.files.get();
        let stored = files.load_setup(&project);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].run.id(), second.id, "run N+1's record stands");
        assert!(
            stored[0].log.lines.is_empty(),
            "and its log: {:?}",
            stored[0].log
        );

        let late = runs
            .push(&key("a"), second.id, Stream::Stdout, b"late\n")
            .unwrap();
        let removed = runs.forget(&project, &location, &mut out);
        store.remove(&project, &location, removed);
        store.write(&key("a"), &late);
        assert!(files.load_setup(&project).is_empty());
        assert!(!files.setup_log_path(&project, &location).exists());
    }

    /// FR-021: the log takes each line as it is kept, and once past [`LOG_COMPACT_AT`] lines it
    /// is replaced by the buffer's last [`KEEP_LINES`], numbered on from the lines dropped.
    #[test]
    fn a_log_past_its_bound_is_cut_back_to_the_buffer() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        let mut push = |bytes: &[u8]| {
            runs.push(&key("a"), id, Stream::Stdout, bytes)
                .and_then(|p| p.log)
        };
        assert_eq!(push(b"x\n"), Some(LogWrite::Append(vec!["x".into()])));
        assert_eq!(push(b"partial"), None);
        let mut last = None;
        for _ in 1..=LOG_COMPACT_AT {
            last = push(b"\n");
        }
        let Some(LogWrite::Replace(log)) = last else {
            panic!("the log is cut back: {last:?}");
        };
        assert_eq!(log.lines.len(), KEEP_LINES);
        assert_eq!(log.first_line, LOG_COMPACT_AT + 1 - KEEP_LINES as u64);
        assert!(matches!(push(b"y\n"), Some(LogWrite::Append(_))));
    }

    /// FR-019: an unwind records a live run `Interrupted` with the reason line last; the run
    /// task's own end and late output, coming after, write and send nothing.
    #[test]
    fn after_an_interrupt_the_runs_own_end_is_dropped() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        let writes = runs.interrupt_all(2, &mut out);
        assert_eq!(writes.len(), 1);
        let (_, persist) = &writes[0];
        assert_eq!(
            persist.record.as_ref().map(SetupRun::state),
            Some(&SetupState::Interrupted)
        );
        assert_eq!(
            persist.log,
            Some(LogWrite::Replace(SetupLog {
                first_line: 0,
                lines: vec!["Interrupted: the daemon stopped.".into()],
            })),
            "the log is rewritten whole, so a line write dropped as stale loses nothing"
        );
        out.clear();
        assert_eq!(runs.push(&key("a"), id, Stream::Stdout, b"late\n"), None);
        let late = runs.finish(
            &key("a"),
            id,
            Ended::Exited(ExitKind::Signal(9)),
            3,
            &mut out,
        );
        assert_eq!(late, None);
        assert!(out.is_empty());
        assert!(
            runs.interrupt_all(4, &mut out).is_empty(),
            "nothing runs now"
        );
    }

    /// 1439 setup-turn S2: the waiting line is kept, logged and sent like a line the run printed.
    #[test]
    fn setup_turn_the_waiting_line_goes_to_the_buffer_the_log_and_the_watchers() {
        let mut runs = runs_with("true");
        let mut out = Vec::new();
        let id = runs
            .start(Path::new("/p"), &key("a").1, 1, &mut out)
            .unwrap()
            .id;
        runs.watch(7, Path::new("/p"), &key("a").1, 0, &mut out);
        out.clear();
        let persist = runs.waiting(&key("a"), id, &mut out).unwrap();
        assert_eq!(
            persist.log,
            Some(LogWrite::Append(vec![WAITING_LINE.into()]))
        );
        let sent: Vec<String> = out
            .iter()
            .filter_map(|(to, m)| match m {
                DaemonMsg::SetupOutput { lines, .. } if *to == To::One(7) => Some(lines.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(sent, [WAITING_LINE]);
    }

    /// Setup runs in folders of one project, on a daemon of its own, for the setup-turn tests
    /// (#1439). Each run appends `start <folder>` and `end <folder>` to the project's `order`
    /// file around `body`, which reads its folder's name from `$d`.
    #[cfg(unix)]
    struct Turns {
        state: Arc<DaemonState>,
        root: tempfile::TempDir,
    }

    #[cfg(unix)]
    impl Turns {
        /// A sandbox daemon (in its container, an `offered.json` of its own) when `sandbox`, else
        /// a host one.
        fn new(sandbox: bool, body: &str) -> Self {
            let root = tempfile::tempdir().unwrap();
            let state = DaemonState::new(crate::catalog::Catalog::load(
                Box::new(JsonFileStore::at(root.path().join("projects.json"))),
                Box::new(micold_core::settings::JsonFileSettingsStore::at(
                    root.path().join("settings.json"),
                )),
            ));
            if sandbox {
                let clis = root.path().join("ai-clis");
                std::fs::create_dir_all(&clis).unwrap();
                std::fs::write(clis.join("offered.json"), r#"{"version":1,"clis":[]}"#).unwrap();
                state.set_ai_clis_dir(clis);
                state.set_in_sandbox(true);
            }
            state.set_setup_command(
                root.path(),
                SetupCommand::parse(&format!(
                    r#"d=$(basename "$PWD"); echo "start $d" >> ../order; {body}; echo "end $d" >> ../order"#
                )),
            );
            Self {
                state: Arc::new(state),
                root,
            }
        }

        fn key(&self, dir: &str) -> Key {
            (
                self.root.path().to_path_buf(),
                SessionLocation::Worktree(dir.into()),
            )
        }

        /// Start setup in the folder `dir`, made first.
        fn start(&self, dir: &str) -> Key {
            let path = self.root.path().join(dir);
            std::fs::create_dir_all(&path).unwrap();
            let key = self.key(dir);
            let started = start_in(&self.state, &key.0, key.1.clone(), path);
            assert!(matches!(started, SetupStart::Started(_)), "{started:?}");
            key
        }

        fn read<T>(&self, key: &Key, f: impl FnOnce(&Entry) -> T) -> T {
            self.state.setup(|runs, _| f(&runs.runs[key]))
        }

        fn state_of(&self, key: &Key) -> SetupState {
            self.read(key, |e| e.record.state().clone())
        }

        fn lines(&self, key: &Key) -> Vec<String> {
            self.read(key, |e| e.output.since(0).1)
        }

        /// What the runs wrote to `order`, in order.
        fn order(&self) -> Vec<String> {
            std::fs::read_to_string(self.root.path().join("order"))
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        }

        /// The run at `key` once its task has written its end, at most 10 s from now.
        async fn ended(&self, key: &Key) -> SetupState {
            let mut done = self.read(key, |e| e.done.clone().unwrap());
            tokio::time::timeout(Duration::from_secs(10), done.wait_for(|done| *done))
                .await
                .expect("the run ended in time")
                .unwrap();
            self.state_of(key)
        }

        /// Wait, at most 10 s, until `ready` holds.
        async fn until(&self, what: &str, ready: impl Fn(&Self) -> bool) {
            let wait = async {
                while !ready(self) {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            };
            tokio::time::timeout(Duration::from_secs(10), wait)
                .await
                .unwrap_or_else(|_| panic!("{what} in time"));
        }

        async fn started(&self, dir: &str) {
            let line = format!("start {dir}");
            self.until(&line, |t| t.order().contains(&line)).await;
        }

        async fn waits(&self, key: &Key) {
            self.until("the waiting line", |t| {
                t.lines(key).first().map(String::as_str) == Some(WAITING_LINE)
            })
            .await;
        }

        /// The turn is free.
        fn turn_is_free(&self) -> bool {
            self.state.setup_turn().unwrap().try_lock().is_ok()
        }
    }

    /// S2: on a sandbox daemon runs never overlap, and they take the turn in the order they asked.
    /// A waiting run's output begins with the waiting line, in its log too; a run that got the
    /// turn at once prints nothing extra.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_on_a_sandbox_daemon_runs_take_turns_in_the_order_they_asked() {
        let t = Turns::new(true, "sleep 0.3");
        let a = t.start("a");
        t.started("a").await;
        let b = t.start("b");
        t.waits(&b).await;
        let c = t.start("c");
        t.waits(&c).await;
        for key in [&a, &b, &c] {
            assert_eq!(t.ended(key).await, SetupState::Succeeded);
        }
        assert_eq!(
            t.order(),
            ["start a", "end a", "start b", "end b", "start c", "end c"]
        );
        assert_eq!(t.lines(&a), ["Exited with status 0."]);
        assert_eq!(t.lines(&b), [WAITING_LINE, "Exited with status 0."]);
        let stored = t.state.setup_store().unwrap().files.get().load_setup(&b.0);
        let logged = stored.iter().find(|s| s.location == b.1).unwrap();
        assert_eq!(
            logged.log.lines.first().map(String::as_str),
            Some(WAITING_LINE)
        );
        assert!(t.turn_is_free());
    }

    /// S1: a host daemon runs its setups concurrently, as 1289 specifies: each run here waits for
    /// both to have started.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_on_a_host_daemon_runs_overlap() {
        let both = "i=0; while [ $(grep -c start ../order) -lt 2 ] && [ $i -lt 200 ]; do sleep 0.05; i=$((i+1)); done";
        let t = Turns::new(false, both);
        let a = t.start("a");
        let b = t.start("b");
        assert_eq!(t.ended(&a).await, SetupState::Succeeded);
        assert_eq!(t.ended(&b).await, SetupState::Succeeded);
        let mut first_two = t.order()[..2].to_vec();
        first_two.sort();
        assert_eq!(first_two, ["start a", "start b"]);
        assert!(!t.lines(&b).iter().any(|line| line == WAITING_LINE));
    }

    /// S3: **Stop setup** on a waiting run ends it `Stopped` at once, while the holder still runs,
    /// and its shell never spawns, not even once the turn comes.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_stop_setup_on_a_waiting_run_ends_it_at_once_and_it_never_spawns() {
        let t = Turns::new(true, "sleep 30");
        let a = t.start("a");
        t.started("a").await;
        let b = t.start("b");
        t.waits(&b).await;
        assert!(t.state.setup(|runs, _| runs.stop(&b.0, &b.1)));
        assert_eq!(t.ended(&b).await, SetupState::Stopped);
        assert_eq!(t.lines(&b), [WAITING_LINE, "Stopped."]);
        assert_eq!(t.state_of(&a), SetupState::Running);
        assert!(t.state.setup(|runs, _| runs.stop(&a.0, &a.1)));
        assert_eq!(t.ended(&a).await, SetupState::Stopped);
        assert_eq!(t.order(), ["start a"], "b never spawned");
        assert!(t.turn_is_free());
    }

    /// S3: stopping the run that holds the turn lets the first waiting run spawn, then the next.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_stopping_the_holder_lets_the_first_waiting_run_spawn() {
        let t = Turns::new(true, r#"if [ "$d" = a ]; then sleep 30; fi"#);
        let a = t.start("a");
        t.started("a").await;
        let b = t.start("b");
        t.waits(&b).await;
        let c = t.start("c");
        t.waits(&c).await;
        assert!(t.state.setup(|runs, _| runs.stop(&a.0, &a.1)));
        assert_eq!(t.ended(&a).await, SetupState::Stopped);
        assert_eq!(t.ended(&b).await, SetupState::Succeeded);
        assert_eq!(t.ended(&c).await, SetupState::Succeeded);
        assert_eq!(
            t.order(),
            ["start a", "start b", "end b", "start c", "end c"]
        );
    }

    /// S3: a delete (`stop_and_wait`) ends a waiting run `Stopped` and a daemon stop ends one
    /// `Interrupted`, each at once and without its shell.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_a_delete_or_a_daemon_stop_ends_a_waiting_run_at_once() {
        let t = Turns::new(true, "sleep 30");
        let a = t.start("a");
        t.started("a").await;
        let b = t.start("b");
        t.waits(&b).await;
        let c = t.start("c");
        t.waits(&c).await;
        let hold =
            tokio::time::timeout(Duration::from_secs(10), stop_and_wait(&t.state, &b.0, &b.1))
                .await
                .expect("a waiting run ends at once");
        assert_eq!(t.state_of(&b), SetupState::Stopped);
        drop(hold);
        interrupt_all(&t.state);
        assert_eq!(t.ended(&c).await, SetupState::Interrupted);
        assert_eq!(t.ended(&a).await, SetupState::Interrupted);
        assert_eq!(t.order(), ["start a"], "neither waiting run spawned");
        assert!(t.turn_is_free());
    }

    /// S2: the turn is let go after a run that failed, one that could not start and one whose
    /// blocking part panicked, so the run waiting behind each goes on.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_the_turn_is_released_after_a_failed_a_not_started_and_a_panicking_run() {
        let t = Turns::new(true, r#"if [ "$d" = a ]; then sleep 0.3; exit 3; fi"#);
        let a = t.start("a");
        t.started("a").await;
        let b = t.start("b");
        t.waits(&b).await;
        assert_eq!(t.ended(&a).await, SetupState::Failed(Failure::Exit(3)));
        assert_eq!(t.ended(&b).await, SetupState::Succeeded);
        assert!(t.turn_is_free());

        let gone = t.key("gone");
        let started = start_in(
            &t.state,
            &gone.0,
            gone.1.clone(),
            t.root.path().join("gone"),
        );
        assert!(matches!(started, SetupStart::Started(_)));
        assert!(matches!(
            t.ended(&gone).await,
            SetupState::Failed(Failure::NotStarted(_))
        ));
        assert!(t.turn_is_free());

        let panicked = t.key("panicked");
        let begun = t
            .state
            .setup(|runs, out| runs.start(&panicked.0, &panicked.1, 1, out))
            .unwrap();
        spawn_run_with(
            &t.state,
            panicked.clone(),
            begun.id,
            begun.done,
            begun.stop,
            || panic!("a setup's blocking part panicked"),
        );
        assert!(matches!(
            t.ended(&panicked).await,
            SetupState::Failed(Failure::NotStarted(_))
        ));
        assert!(t.turn_is_free());
        let d = t.start("d");
        assert_eq!(t.ended(&d).await, SetupState::Succeeded);
        assert!(!t.lines(&d).iter().any(|line| line == WAITING_LINE));
    }

    /// Review A-F2: a run forgotten while its blocking part still runs (a delete that gave up
    /// waiting for a tree that escaped the kill) lets the turn go, so later setups are not held
    /// until the daemon restarts.
    #[cfg(unix)]
    #[tokio::test]
    async fn setup_turn_is_let_go_when_a_still_running_run_is_forgotten() {
        let t = Turns::new(true, "true");
        let escaped = t.key("escaped");
        let begun = t
            .state
            .setup(|runs, out| runs.start(&escaped.0, &escaped.1, 1, out))
            .unwrap();
        let mut done = begun.done.subscribe();
        let (release, held) = std::sync::mpsc::channel::<()>();
        let running = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shell_running = Arc::clone(&running);
        spawn_run_with(
            &t.state,
            escaped.clone(),
            begun.id,
            begun.done,
            begun.stop,
            move || {
                shell_running.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = held.recv();
                Ended::NotStarted("released".into())
            },
        );
        t.until("the shell runs", |_| {
            running.load(std::sync::atomic::Ordering::SeqCst)
        })
        .await;
        assert!(!t.turn_is_free(), "the run holds the turn");
        forget(&t.state, &escaped.0, &escaped.1);
        t.until("the turn is let go", Turns::turn_is_free).await;
        let d = t.start("d");
        assert_eq!(t.ended(&d).await, SetupState::Succeeded);
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), done.wait_for(|done| *done))
            .await
            .expect("the forgotten run ended in time")
            .unwrap();
        assert!(t.turn_is_free());
    }
}

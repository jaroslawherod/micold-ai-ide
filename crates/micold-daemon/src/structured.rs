//! A structured session (feature 938): the AI CLI over pipes rather than in a pseudo-terminal,
//! writing stream-json lines the daemon parses into typed events (contracts/claude-stream-json.md).
//!
//! The launch is a terminal session's — the same command, cwd, environment and argv, behind the
//! provider's [`structured_args`] (FR-005, FR-006) — and so is the lifecycle: the child and its
//! process tree are a [`SupervisedChild`], as a [`crate::supervisor::PtySession`]'s are (FR-009).
//!
//! [`structured_args`]: micold_core::provider::AiCliProvider::structured_args

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{self, BufRead, Read, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use micold_core::session::{KindRefusal, PermissionMode, SessionId};
use micold_core::structured::control::{encode_interrupt, encode_set_mode, next_request_id};
use micold_core::structured::input::Message;
use micold_core::structured::{
    encode_user_message, parse_line, ControlOutcome, EventHistory, InterruptAsk, LineSplitter,
    SequencedEvent, StructuredEvent, Turn, TurnId, TurnState, UserMessageRefusal,
};
use micold_core::terminal::{launch_args, LaunchSpec};
use tokio::sync::{broadcast, oneshot};

use crate::supervision::ExitOutcome;
use crate::supervisor::{apply_env_hygiene, ensure_cwd_exists, Supervised, SupervisedChild};

/// How many live events a viewer may fall behind before its receiver lags (R6).
const LIVE_CAPACITY: usize = 1024;

/// The bounds of an [`EventStream`]: its history's event count and line text (R5), and how many
/// live events a viewer may fall behind before its receiver lags (R6). Lowered only by tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamBounds {
    pub max_events: usize,
    pub max_bytes: usize,
    pub live_capacity: usize,
}

impl Default for StreamBounds {
    fn default() -> Self {
        Self {
            max_events: micold_core::structured::MAX_EVENTS,
            max_bytes: micold_core::structured::MAX_BYTES,
            live_capacity: LIVE_CAPACITY,
        }
    }
}

/// A structured session's events: the bounded history every viewer starts from, and the channel
/// each new event is broadcast on (R6, FR-016, FR-017). Kept by the daemon apart from the process,
/// so the events written before an exit are still there after it (FR-009).
pub struct EventStream {
    history: Mutex<EventHistory>,
    live: broadcast::Sender<SequencedEvent>,
}

impl Default for EventStream {
    fn default() -> Self {
        Self::with_bounds(StreamBounds::default())
    }
}

impl EventStream {
    /// An empty stream under `bounds`.
    pub fn with_bounds(bounds: StreamBounds) -> Self {
        Self {
            history: Mutex::new(EventHistory::with_bounds(
                bounds.max_events,
                bounds.max_bytes,
            )),
            live: broadcast::channel(bounds.live_capacity).0,
        }
    }

    /// A stream around a history restored from a saved one (research R14): `events` are the
    /// retained ones after `omitted` evicted, numbered from `omitted`, under `bounds`.
    pub fn restored(omitted: u64, events: Vec<StructuredEvent>, bounds: StreamBounds) -> Self {
        Self {
            history: Mutex::new(EventHistory::restored_with_bounds(
                omitted,
                events,
                bounds.max_events,
                bounds.max_bytes,
            )),
            live: broadcast::channel(bounds.live_capacity).0,
        }
    }

    /// The history as `(omitted, events, next_seq)` for a save, taken under the history lock so
    /// the three agree.
    pub fn saved_snapshot(&self) -> (u64, Vec<SequencedEvent>, u64) {
        let history = self.history.lock().unwrap_or_else(|e| e.into_inner());
        let (omitted, events) = history.snapshot();
        (omitted, events, history.next_seq())
    }

    /// The `seq` the next event gets: what the periodic saver counts (a save is due after a new
    /// event).
    pub fn next_seq(&self) -> u64 {
        self.history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .next_seq()
    }

    /// Append `event` to the history and broadcast it, under the history lock, so a viewer that
    /// [`Self::subscribe`]s sees each event exactly once: in its snapshot or on its receiver.
    pub fn push(&self, event: StructuredEvent) {
        let mut history = self.history.lock().unwrap_or_else(|e| e.into_inner());
        let sequenced = history.append(event);
        // No receiver is not an error: nobody is viewing the session.
        let _ = self.live.send(sequenced);
    }

    /// The history as `(omitted, events)` and a receiver of every later event, taken in one
    /// critical section (R6).
    pub fn subscribe(
        &self,
    ) -> (
        u64,
        Vec<SequencedEvent>,
        broadcast::Receiver<SequencedEvent>,
    ) {
        let history = self.history.lock().unwrap_or_else(|e| e.into_inner());
        let (omitted, events) = history.snapshot();
        (omitted, events, self.live.subscribe())
    }
}

/// Why a user message did not reach a structured session's CLI (FR-008). Its text is never in it.
#[derive(Debug)]
pub enum UserMessageUndelivered {
    /// Refused before anything was written.
    Refused(UserMessageRefusal),
    /// Accepted, and writing it to the CLI's stdin failed.
    WriteFailed(io::Error),
}

impl std::fmt::Display for UserMessageUndelivered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(why) => why.fmt(f),
            Self::WriteFailed(err) => write!(f, "The message could not be sent to the CLI: {err}"),
        }
    }
}

/// How long the daemon waits for the CLI's answer to a control request (#942 data-model
/// *ControlRequest*, FR-017).
pub const CONTROL_ANSWER_BOUND: Duration = Duration::from_secs(10);

/// How long an interrupt may go unanswered by the turn's end before every window is told it is
/// overdue (#941 FR-015). Lowered only by tests (`DaemonState::set_interrupt_overdue`).
pub const INTERRUPT_OVERDUE: Duration = Duration::from_secs(10);

/// Why a running session's CLI did not change its permission mode (FR-017): the user is told this.
#[derive(Debug)]
pub enum ModeChangeFailed {
    /// The CLI answered `error`: its text.
    Refused(String),
    /// No answer within [`CONTROL_ANSWER_BOUND`].
    NoAnswer,
    /// The process's output ended first.
    Stopped,
    /// Writing the request to the CLI's stdin failed.
    Unsent(io::Error),
}

impl std::fmt::Display for ModeChangeFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(text) => f.write_str(text),
            Self::NoAnswer => f.write_str("Claude did not answer the mode change."),
            Self::Stopped => f.write_str("Claude stopped before it answered."),
            Self::Unsent(err) => write!(f, "The mode change could not be sent to the CLI: {err}"),
        }
    }
}

/// The control requests of one process waiting for their answers (data-model *ControlRequest*): a
/// request id's `oneshot`, resolved by the stdout reader on the `control_response` for it. At the
/// end of the output every waiter is dropped, which ends its wait, and no new one is taken.
struct ControlWaiters(Mutex<Option<HashMap<String, oneshot::Sender<ControlOutcome>>>>);

impl ControlWaiters {
    fn new() -> Self {
        Self(Mutex::new(Some(HashMap::new())))
    }

    /// A receiver of `request_id`'s answer; `None` once the output ended.
    fn wait_for(&self, request_id: &str) -> Option<oneshot::Receiver<ControlOutcome>> {
        let (sender, receiver) = oneshot::channel();
        lock(&self.0)
            .as_mut()?
            .insert(request_id.to_string(), sender);
        Some(receiver)
    }

    /// Stop waiting for `request_id`.
    fn forget(&self, request_id: &str) {
        if let Some(waiting) = lock(&self.0).as_mut() {
            waiting.remove(request_id);
        }
    }

    /// The CLI answered `request_id`; an answer nobody waits for is dropped.
    fn resolve(&self, request_id: &str, outcome: &ControlOutcome) {
        let waiter = lock(&self.0).as_mut().and_then(|w| w.remove(request_id));
        if let Some(waiter) = waiter {
            let _ = waiter.send(outcome.clone());
        }
    }

    /// The output ended: every wait ends unanswered, and later ones at once.
    fn close(&self) {
        lock(&self.0).take();
    }
}

/// What ended a turn ([`TurnClosed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedBy {
    /// The CLI's `result`.
    Result,
    /// The end of the process's output; `TurnClosed::turn` is the turn it cut off, if one was
    /// open.
    Exit,
}

/// A turn the stdout reader ended, or the end of a process's output, on its way to the input
/// driver (#941 data-model *Daemon*, research R2). Named apart from the activity's turn end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnClosed {
    pub session: SessionId,
    /// The process's [`StructuredSession::serial`], so a late end of an earlier process is told
    /// from the current one's.
    pub process: u64,
    /// The turn that ended; `None` for an output that ended between turns.
    pub turn: Option<TurnId>,
    pub by: ClosedBy,
    /// Whether an interrupt was asked for during the turn.
    pub interrupted: bool,
}

/// The serial of the next structured process this daemon run spawns.
static NEXT_SERIAL: AtomicU64 = AtomicU64::new(1);

/// Where a structured session's events go, in order: the stdout reader's events and the
/// [`StructuredEvent::UserSent`] note of a message being written (#941 research R3). Its lock is
/// held by the reader across ending a turn and passing the event on, and by a writer only to set
/// or pass on its note, never across the write itself.
struct Emit {
    on_event: Box<dyn FnMut(StructuredEvent) + Send>,
    /// The note of the message being written: passed on before the next line the reader reads,
    /// or after the write, whichever is first; dropped when the write fails first.
    sending: Option<StructuredEvent>,
}

impl Emit {
    fn emit(&mut self, event: StructuredEvent) {
        self.flush();
        (self.on_event)(event);
    }

    fn flush(&mut self) {
        if let Some(note) = self.sending.take() {
            (self.on_event)(note);
        }
    }
}

/// One daemon-hosted structured session: the CLI's child process with piped stdin, stdout and
/// stderr. Stdout is read line by line into events, stderr into the daemon's log (FR-010).
pub struct StructuredSession {
    id: SessionId,
    process: SupervisedChild,
    /// The CLI's stdin, which user messages and control requests are written to (FR-008, #942
    /// R6). Behind a mutex so a write takes `&self` and two writes never interleave; `None` once
    /// teardown closed it.
    stdin: Mutex<Option<ChildStdin>>,
    /// The last turn whose user message was written to stdin. Set and read only under the stdin
    /// mutex, so an interrupt sees whether its turn's message is out yet (#941 R4).
    written: Mutex<Option<TurnId>>,
    /// Control requests waiting for the CLI's answer, shared with the stdout reader.
    control: Arc<ControlWaiters>,
    /// This process's turn (research R7): a new process starts `Idle`, so a restart resets it.
    /// Shared with the stdout reader, which ends the turn on a `result` and at end-of-file. Taken
    /// under the daemon's state lock by every decision on the turn (#941 R1), never the other way
    /// round.
    turn: Arc<Mutex<TurnState>>,
    /// Where events go, shared with the stdout reader (#941 R3).
    emit: Arc<Mutex<Emit>>,
    /// Set by the stdout reader at its end-of-file, before it reports the end.
    stdout_ended: Arc<AtomicBool>,
    /// This process's serial in the daemon run.
    serial: u64,
    /// The stdout and stderr reader threads, joined by [`Self::teardown`].
    readers: Mutex<Vec<JoinHandle<()>>>,
    /// Set when both readers have seen end-of-file.
    output_ended: Arc<ReadersDone>,
    /// Set by the first [`Self::teardown`]; a later call does nothing.
    torn_down: AtomicBool,
}

/// How many of a session's two readers have not yet seen end-of-file.
#[derive(Default)]
struct ReadersDone(std::sync::atomic::AtomicUsize);

impl ReadersDone {
    fn all(&self) -> bool {
        self.0.load(Ordering::Acquire) == 0
    }
    fn one_done(&self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl StructuredSession {
    /// Spawn `spec`'s AI CLI as a structured session: [`structured_args`], then `launch_args(spec)`,
    /// then `extra_args` (the activity wiring and, last, the tool server), in `spec.cwd` with
    /// `spec.env` over the provider's `structured_env` (feature 944) over the daemon's environment
    /// cleaned as a terminal session's is (R3). Each line
    /// the CLI writes to stdout reaches `on_event` as one event, in order, and a trailing partial
    /// line at end-of-file as one more (FR-014). A `result` ends the turn; at end-of-file a turn
    /// still in progress ends with a `TurnEndedWithoutResult` note, after the last line (R7), and
    /// then `on_end` runs: the process's output is over (#942 FR-011). `on_turn_end` is told of each
    /// turn a `result` ended, after the event, and of the output's end, before `on_end` (#941 R2).
    ///
    /// Refused if `spec.cwd` does not exist (FR-006c), and for a CLI with no structured mode
    /// (FR-003).
    ///
    /// [`structured_args`]: micold_core::provider::AiCliProvider::structured_args
    pub fn spawn(
        id: SessionId,
        spec: &LaunchSpec,
        extra_args: &[OsString],
        on_event: impl FnMut(StructuredEvent) + Send + 'static,
        on_turn_end: impl FnMut(TurnClosed) + Send + 'static,
        on_end: impl FnOnce() + Send + 'static,
    ) -> io::Result<Self> {
        ensure_cwd_exists(&spec.cwd)?;
        let provider = spec.provider.provider();
        let Some(structured) = provider.structured_args() else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                KindRefusal::NotSupported(spec.provider).to_string(),
            ));
        };
        let mut cmd = Command::new(provider.command());
        micold_core::process::no_window(&mut cmd);
        // Its own process group, as a PTY child is its own session: the group is what teardown
        // ends (FR-036).
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        apply_env_hygiene(&mut cmd);
        cmd.current_dir(&spec.cwd)
            .envs(provider.structured_env().iter().copied())
            .envs(spec.env.iter().map(|(k, v)| (k, v)))
            .args(structured)
            .args(launch_args(spec))
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        // Before anything else, as for a PTY child: on Windows this puts the child in its job.
        let tree = crate::platform::ProcessTree::adopt(child.id());
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let process = SupervisedChild::new(Supervised::new(Box::new(child), tree));
        let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
            let _ = process.kill();
            return Err(io::Error::other("the structured child has no output pipes"));
        };

        let output_ended = Arc::new(ReadersDone(2.into()));
        let turn = Arc::new(Mutex::new(TurnState::default()));
        let reader_turn = Arc::clone(&turn);
        let control = Arc::new(ControlWaiters::new());
        let reader_control = Arc::clone(&control);
        let mut on_event = on_event;
        let emit = Arc::new(Mutex::new(Emit {
            on_event: Box::new(move |event| {
                if let StructuredEvent::ControlResponse {
                    request_id,
                    outcome,
                    ..
                } = &event
                {
                    reader_control.resolve(request_id, outcome);
                }
                on_event(event);
            }),
            sending: None,
        }));
        let reader_emit = Arc::clone(&emit);
        let stdout_ended = Arc::new(AtomicBool::new(false));
        let reader_ended = Arc::clone(&stdout_ended);
        let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let reader_control = Arc::clone(&control);
        let readers = [
            spawn_reader(
                format!("stream-out-{id}"),
                Arc::clone(&output_ended),
                move || {
                    let mut on_turn_end = on_turn_end;
                    read_events(
                        stdout,
                        &reader_turn,
                        &reader_emit,
                        |turn, by, interrupted| {
                            if by == ClosedBy::Exit {
                                reader_ended.store(true, Ordering::Release);
                            }
                            on_turn_end(TurnClosed {
                                session: id,
                                process: serial,
                                turn,
                                by,
                                interrupted,
                            });
                        },
                    );
                    reader_control.close();
                    on_end();
                },
            ),
            spawn_reader(
                format!("stream-err-{id}"),
                Arc::clone(&output_ended),
                move || log_stderr(id, stderr),
            ),
        ];
        // A reader that could not start leaves no owner for the child: end it here.
        let readers = match readers.into_iter().collect::<io::Result<Vec<_>>>() {
            Ok(readers) => readers,
            Err(e) => {
                let _ = process.kill();
                return Err(e);
            }
        };
        Ok(Self {
            id,
            process,
            stdin: Mutex::new(stdin),
            written: Mutex::new(None),
            control,
            turn,
            emit,
            stdout_ended,
            serial,
            readers: Mutex::new(readers),
            output_ended,
            torn_down: AtomicBool::new(false),
        })
    }

    /// The session's stable identity.
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// Whether the child is still running: [`SupervisedChild::is_alive`].
    pub fn is_alive(&self) -> bool {
        self.process.is_alive()
    }

    /// The child's exit outcome once it has exited: [`SupervisedChild::exit_outcome`].
    pub fn exit_outcome(&self) -> Option<ExitOutcome> {
        self.process.exit_outcome()
    }

    /// The child's OS process id, if known.
    pub fn pid(&self) -> Option<u32> {
        self.process.pid()
    }

    /// Terminate and reap the child and its process tree: [`SupervisedChild::kill`].
    pub fn kill(&self) -> io::Result<()> {
        self.process.kill()
    }

    /// Whether both readers have seen end-of-file.
    pub fn output_ended(&self) -> bool {
        self.output_ended.all()
    }

    /// This process's serial in the daemon run: a later process of the session has a higher one.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// Whether the CLI can be sent a message: the child runs and its output has not ended.
    pub fn is_running(&self) -> bool {
        self.is_alive() && !self.stdout_ended.load(Ordering::Acquire)
    }

    /// This process's turn, locked. The daemon takes it under its state lock, so every decision
    /// on the turn (deliver, queue, interrupt) sees the turn as it is and opens one in the same
    /// section that read it idle (#941 R1).
    pub fn turn(&self) -> std::sync::MutexGuard<'_, TurnState> {
        lock(&self.turn)
    }

    /// The turn as the windows are shown it (#941 contracts/wire.md *StructuredInput*).
    pub fn phase(&self) -> micold_core::protocol::messages::TurnPhase {
        use micold_core::protocol::messages::TurnPhase;
        if !self.is_running() {
            return TurnPhase::NotRunning;
        }
        match self.turn().now() {
            Turn::Idle => TurnPhase::Idle,
            Turn::InTurn {
                turn, interrupt, ..
            } => TurnPhase::Busy { turn, interrupt },
        }
    }

    /// Write `message`, for which the daemon opened a turn, to the CLI as one stream-json user
    /// message (#941 FR-003, FR-027), and note it as [`StructuredEvent::UserSent`] with its
    /// pictures' names before any line of its turn (R3). On a failed write nothing was delivered:
    /// the note is dropped and the turn closed again. Never logs the text or the pictures.
    pub fn write_user_message(&self, message: &Message) -> io::Result<()> {
        lock(&self.emit).sending = Some(StructuredEvent::UserSent {
            text: message.text.clone(),
            attachments: message
                .images
                .iter()
                .map(|image| image.name().to_owned())
                .collect(),
        });
        let line = encode_user_message(&message.text, &message.images);
        let written = {
            let mut stdin = lock(&self.stdin);
            let written = write_whole(&mut stdin, &line);
            if written.is_ok() {
                // The daemon opened this message's turn before handing it here, and no other
                // message is written while it is open: the open turn is this message's.
                if let Turn::InTurn { turn, .. } = lock(&self.turn).now() {
                    *lock(&self.written) = Some(turn);
                }
            }
            written
        };
        {
            let mut emit = lock(&self.emit);
            if written.is_ok() {
                emit.flush();
            } else {
                emit.sending = None;
            }
        }
        if written.is_err() {
            lock(&self.turn).abandon();
        }
        written
    }

    /// Ask the CLI to interrupt `turn` with an `interrupt` control request (#941 R4, P-4), when
    /// `turn` is still this process's open turn, and note it as [`StructuredEvent::InterruptAsked`]
    /// before the request goes out, so the note lands inside the turn. The check, the note and the
    /// write are made under the stdin mutex: a message of a later turn is written only after it,
    /// so the request never reaches a turn it was not for. The answer is not waited on: the turn's
    /// `result` ends the turn (contracts/claude-input.md *Interrupt*), and an unanswered request
    /// is what the overdue notice covers. `Ok(false)`: the turn ended first, nothing written.
    pub fn write_interrupt(&self, turn: TurnId) -> io::Result<bool> {
        loop {
            let mut stdin = lock(&self.stdin);
            {
                let mut emit = lock(&self.emit);
                let open = matches!(
                    lock(&self.turn).now(),
                    Turn::InTurn { turn: open, .. } if open == turn
                );
                if !open {
                    return Ok(false);
                }
                if *lock(&self.written) != Some(turn) {
                    // The turn is open but its own message is not written yet (its write is
                    // spawned after the turn opens): let it go first, so the request follows the
                    // message it interrupts. A failed write closes the turn, which ends this wait.
                    drop(emit);
                    drop(stdin);
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                emit.emit(StructuredEvent::InterruptAsked);
            }
            return write_whole(&mut stdin, &encode_interrupt(&next_request_id())).map(|()| true);
        }
    }

    /// Write one whole line to the CLI's stdin, under its mutex.
    fn write_line(&self, line: &str) -> io::Result<()> {
        write_whole(&mut lock(&self.stdin), line)
    }

    /// Ask the CLI to switch to `mode` with a `set_permission_mode` control request, and wait for
    /// its answer up to [`CONTROL_ANSWER_BOUND`] (#942 R6, FR-017). A request already pending is
    /// not touched. The write may block briefly on the pipe, as a user message's does.
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<(), ModeChangeFailed> {
        let request_id = next_request_id();
        let answer = self
            .control
            .wait_for(&request_id)
            .ok_or(ModeChangeFailed::Stopped)?;
        if let Err(err) = self.write_line(&encode_set_mode(&request_id, mode)) {
            self.control.forget(&request_id);
            return Err(ModeChangeFailed::Unsent(err));
        }
        match tokio::time::timeout(CONTROL_ANSWER_BOUND, answer).await {
            Ok(Ok(ControlOutcome::Success(_))) => Ok(()),
            Ok(Ok(ControlOutcome::Error { message, .. })) => {
                Err(ModeChangeFailed::Refused(message))
            }
            Ok(Err(_)) => Err(ModeChangeFailed::Stopped),
            Err(_) => {
                self.control.forget(&request_id);
                Err(ModeChangeFailed::NoAnswer)
            }
        }
    }

    /// End the session: kill its process tree, close its stdin, wait up to `wait` for both readers'
    /// end-of-file — so every line the CLI wrote has become an event — and join them. A second call
    /// does nothing; [`Drop`] calls it.
    pub fn teardown(&self, wait: Duration) {
        if self.torn_down.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = self.kill();
        drop(self.stdin.lock().ok().and_then(|mut stdin| stdin.take()));
        let deadline = std::time::Instant::now() + wait;
        while !self.output_ended() {
            if std::time::Instant::now() >= deadline {
                tracing::warn!(
                    session = %self.id.0,
                    ?wait,
                    "the structured session's output did not end after the stop; keeping what was read"
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let readers = self
            .readers
            .lock()
            .map(|mut readers| std::mem::take(&mut *readers))
            .unwrap_or_default();
        for reader in readers {
            let _ = reader.join();
        }
    }
}

impl Drop for StructuredSession {
    fn drop(&mut self) {
        self.teardown(crate::supervisor::TEARDOWN_WAIT);
    }
}

/// Write one whole line to `stdin`, whose mutex the caller holds; a closed one is a broken pipe.
fn write_whole(stdin: &mut Option<ChildStdin>, line: &str) -> io::Result<()> {
    match stdin.as_mut() {
        Some(stdin) => stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush()),
        None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
    }
}

/// Run `read` on a thread named `name`, counting it done in `done` when it returns.
fn spawn_reader(
    name: String,
    done: Arc<ReadersDone>,
    read: impl FnOnce() + Send + 'static,
) -> io::Result<JoinHandle<()>> {
    std::thread::Builder::new().name(name).spawn(move || {
        // Counted on unwind too, so a panicking reader does not hold every teardown to its wait.
        let _done = CountsDone(done);
        read();
    })
}

/// Counts its reader done when dropped.
struct CountsDone(Arc<ReadersDone>);

impl Drop for CountsDone {
    fn drop(&mut self) {
        self.0.one_done();
    }
}

/// A mutex's guard, poison or not: the guarded state stays sound across a panic.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Read `stdout` to its end, one event per line in order, the trailing partial line last (R4,
/// FR-014), each followed by `turn` (R7) under `emit`'s lock, so a message written meanwhile is
/// noted after the event that ended the turn before it (#941 R3). No line, however unreadable,
/// stops the reading. Each turn a `result` ends is reported to `on_turn_end` after its event. At
/// the end, a turn still in progress ends with its note, the last event of this process, and the
/// end is reported with the turn it cut off.
fn read_events(
    stdout: impl Read,
    turn: &Mutex<TurnState>,
    emit: &Mutex<Emit>,
    mut on_turn_end: impl FnMut(Option<TurnId>, ClosedBy, bool),
) {
    read_lines(stdout, |event| {
        let ended = {
            let mut emit = lock(emit);
            // The turn's lock is released before the event goes on, which takes the history's.
            let ended = lock(turn).on_event(&event);
            emit.emit(event);
            ended
        };
        if let Some(ended) = ended {
            on_turn_end(Some(ended.turn), ClosedBy::Result, ended.interrupted);
        }
    });
    let cut_off = {
        let mut emit = lock(emit);
        let (cut_off, note) = {
            let mut turn = lock(turn);
            let cut_off = match turn.now() {
                Turn::InTurn {
                    turn, interrupt, ..
                } => Some((turn, interrupt != InterruptAsk::None)),
                Turn::Idle => None,
            };
            (cut_off, turn.on_exit())
        };
        // Passed on as it is: the note of a message whose write is still going on waits for that
        // write's own outcome.
        if let Some(note) = note {
            (emit.on_event)(note);
        }
        cut_off
    };
    on_turn_end(
        cut_off.map(|(turn, _)| turn),
        ClosedBy::Exit,
        cut_off.is_some_and(|(_, interrupted)| interrupted),
    );
}

/// Read `stdout` to its end, one event per line in order, the trailing partial line last.
fn read_lines(mut stdout: impl Read, mut on_event: impl FnMut(StructuredEvent)) {
    let mut splitter = LineSplitter::default();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for line in splitter.push(&buf[..n]) {
                    on_event(parse_line(&line));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if let Some(line) = splitter.finish() {
        on_event(parse_line(&line));
    }
}

/// Log each line the CLI writes to stderr with its session, never as an event (FR-010).
fn log_stderr(id: SessionId, stderr: impl Read) {
    for line in io::BufReader::new(stderr).lines() {
        let Ok(line) = line else { break };
        tracing::warn!(session = %id.0, "claude stderr: {line}");
    }
}

/// The most one `StructuredEvents` message's encoding may take (contracts/wire.md, R6): half the
/// frame limit, so a message never comes near it.
pub const MESSAGE_LIMIT: usize = micold_core::protocol::envelope::MAX_FRAME_LENGTH / 2;

/// Room left in each message for everything but its events: the variant and field names, the
/// session id, `omitted`, `live` and the brackets take about 120 bytes of JSON.
const MESSAGE_OVERHEAD: usize = 256;

/// The JSON length of `event`, as the wire encodes control messages, counted without a buffer.
fn encoded_len(event: &SequencedEvent) -> usize {
    struct Count(usize);
    impl Write for Count {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0 += buf.len();
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    // Serializing an event cannot fail: every field is a string, number, bool or JSON value.
    let _ = serde_json::to_writer(&mut count, event);
    count.0
}

/// `event` as `session`'s viewers are sent it, with its encoded length plus the comma that
/// separates it from the next: itself, or, when it alone would not fit in a message, an
/// `OversizedOnWire` stand-in under its `seq`, logged (R6, FR-014). The history keeps it whole.
fn on_wire_sized(session: SessionId, event: SequencedEvent) -> (SequencedEvent, usize) {
    let bytes = encoded_len(&event) + 1;
    if bytes <= MESSAGE_LIMIT - MESSAGE_OVERHEAD {
        return (event, bytes);
    }
    tracing::warn!(
        session = %session.0,
        seq = event.seq,
        bytes,
        "a structured event too large for one message is sent as OversizedOnWire"
    );
    let stand_in = SequencedEvent {
        seq: event.seq,
        event: StructuredEvent::OversizedOnWire {
            original_bytes: bytes as u64 - 1,
        },
    };
    let bytes = encoded_len(&stand_in) + 1;
    (stand_in, bytes)
}

/// A live event of `session` as its viewers are sent it: see [`on_wire_sized`].
pub fn on_wire(session: SessionId, event: SequencedEvent) -> SequencedEvent {
    on_wire_sized(session, event).0
}

/// `session`'s history cut into the events of consecutive messages, each under
/// [`MESSAGE_LIMIT`], in order; a stand-in for an oversized event in a message of its own (R6).
/// An empty history is one empty message, so a viewer is always sent where its log starts.
pub fn history_messages(
    session: SessionId,
    history: Vec<SequencedEvent>,
) -> Vec<Vec<SequencedEvent>> {
    let mut messages = Vec::new();
    let mut current = Vec::new();
    let mut size = 0;
    for event in history {
        let (event, bytes) = on_wire_sized(session, event);
        // Only the daemon makes a stand-in: no CLI line parses to one.
        let oversized = matches!(event.event, StructuredEvent::OversizedOnWire { .. });
        if !current.is_empty() && (oversized || size + bytes > MESSAGE_LIMIT - MESSAGE_OVERHEAD) {
            messages.push(std::mem::take(&mut current));
            size = 0;
        }
        current.push(event);
        size += bytes;
        if oversized {
            messages.push(std::mem::take(&mut current));
            size = 0;
        }
    }
    if !current.is_empty() || messages.is_empty() {
        messages.push(current);
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events_of(bytes: &[u8]) -> Vec<StructuredEvent> {
        let mut got = Vec::new();
        read_lines(bytes, |event| got.push(event));
        got
    }

    #[test]
    fn every_line_and_the_trailing_partial_one_become_events_in_order() {
        let got = events_of(b"not json\n\n{\"type\":\"res");
        let expected: Vec<StructuredEvent> = ["not json", "", "{\"type\":\"res"]
            .into_iter()
            .map(parse_line)
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn a_subscriber_gets_each_event_once_in_its_snapshot_or_live() {
        let stream = EventStream::default();
        stream.push(parse_line("one"));
        let (omitted, history, mut live) = stream.subscribe();
        stream.push(parse_line("two"));
        assert_eq!(omitted, 0);
        assert_eq!(history.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![0]);
        assert_eq!(live.try_recv().map(|e| e.seq), Ok(1));
        assert!(live.try_recv().is_err(), "nothing twice");
    }
}

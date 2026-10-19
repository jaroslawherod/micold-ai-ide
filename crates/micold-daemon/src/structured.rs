//! A structured session (feature 938): the AI CLI over pipes rather than in a pseudo-terminal,
//! writing stream-json lines the daemon parses into typed events (contracts/claude-stream-json.md).
//!
//! The launch is a terminal session's — the same command, cwd, environment and argv, behind the
//! provider's [`structured_args`] (FR-005, FR-006) — and so is the lifecycle: the child and its
//! process tree are a [`SupervisedChild`], as a [`crate::supervisor::PtySession`]'s are (FR-009).
//!
//! [`structured_args`]: micold_core::provider::AiCliProvider::structured_args

use std::ffi::OsString;
use std::io::{self, BufRead, Read, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use micold_core::session::{KindRefusal, SessionId};
use micold_core::structured::{
    encode_user_message, parse_line, EventHistory, LineSplitter, SequencedEvent, StructuredEvent,
    TurnState, UserMessageRefusal,
};
use micold_core::terminal::{launch_args, LaunchSpec};
use tokio::sync::broadcast;

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

/// One daemon-hosted structured session: the CLI's child process with piped stdin, stdout and
/// stderr. Stdout is read line by line into events, stderr into the daemon's log (FR-010).
pub struct StructuredSession {
    id: SessionId,
    process: SupervisedChild,
    /// The CLI's stdin, which user messages are written to (FR-008). Behind a mutex so a write
    /// takes `&self`; `None` once teardown closed it.
    stdin: Mutex<Option<ChildStdin>>,
    /// This process's turn (research R7): a new process starts `Idle`, so a restart resets it.
    /// Shared with the stdout reader, which ends the turn on a `result` and at end-of-file.
    turn: Arc<Mutex<TurnState>>,
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
    /// `spec.env` over the daemon's environment cleaned as a terminal session's is (R3). Each line
    /// the CLI writes to stdout reaches `on_event` as one event, in order, and a trailing partial
    /// line at end-of-file as one more (FR-014). A `result` ends the turn; at end-of-file a turn
    /// still in progress ends with a `TurnEndedWithoutResult` note, after the last line (R7).
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
        let turn = Arc::new(Mutex::new(TurnState::Idle));
        let reader_turn = Arc::clone(&turn);
        let readers = [
            spawn_reader(
                format!("stream-out-{id}"),
                Arc::clone(&output_ended),
                move || read_events(stdout, &reader_turn, on_event),
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
            turn,
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

    /// Accept `text` as the next user message, opening a turn, or say why not (FR-008). The
    /// daemon calls it under its state lock, so of two simultaneous messages the first wins; the
    /// write is [`Self::write_user_message`], off the lock.
    pub fn accept_user_message(&self, text: &str) -> Result<(), UserMessageRefusal> {
        let running = self.is_alive() && !self.output_ended();
        lock(&self.turn).accept(text, running)
    }

    /// Write an accepted `text` to the CLI as one stream-json user message. On a failed write
    /// nothing was delivered, so the turn it opened is closed again.
    pub fn write_user_message(&self, text: &str) -> io::Result<()> {
        let line = encode_user_message(text);
        let written = match lock(&self.stdin).as_mut() {
            Some(stdin) => stdin
                .write_all(line.as_bytes())
                .and_then(|()| stdin.flush()),
            None => Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        };
        if written.is_err() {
            *lock(&self.turn) = TurnState::Idle;
        }
        written
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
/// FR-014), each followed by `turn` (R7). No line, however unreadable, stops the reading. At the
/// end, a turn still in progress ends with its note, the last event of this process.
fn read_events(
    stdout: impl Read,
    turn: &Mutex<TurnState>,
    mut on_event: impl FnMut(StructuredEvent),
) {
    read_lines(stdout, |event| {
        lock(turn).on_event(&event);
        on_event(event);
    });
    // The turn's lock is released before `on_event`, which takes the history's.
    let note = lock(turn).on_exit();
    if let Some(note) = note {
        on_event(note);
    }
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

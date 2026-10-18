//! IMPURE: the one process-spawn shim the runtime trait is implemented over.
//!
//! Deliberately tiny and deliberately alone. Everything that decides anything lives in the pure
//! modules beside it; this is the sole place the sandbox layer touches the world, which is what
//! keeps the rest of the layer testable without a container runtime (Principle I).
//!
//! # Why the runner is injected rather than found on `PATH`
//!
//! The contract (`contracts/container-runtime.md`) describes the test harness as a fake runtime
//! binary placed first on `PATH`. That shape does not survive contact with `cargo test`: `PATH` is
//! process-global, cargo runs tests as parallel **threads** of one process, and a test that
//! rewrites `PATH` therefore rewrites it for every other test running at that moment. (Edition
//! 2024 makes the same point by marking `set_var` `unsafe`.)
//!
//! So the seam is one level in: [`CommandRunner`] is injected, [`SystemRunner`] spawns for real,
//! and [`RecordingRunner`] records argv and replays canned output in-process. Everything the
//! conformance suite asserts — argv construction, output parsing, error classification — is above
//! this seam and is exercised identically on all three platforms with nothing installed, which was
//! the property the fake binary existed to provide. What remains below it is the spawn itself,
//! covered by [`tests::system_runner_captures_a_real_process`].

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use crate::process::no_window;

/// What a runtime invocation produced. Mirrors `std::process::Output`, minus the platform-specific
/// `ExitStatus`, so a canned response is as constructible as a real one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// The process's exit code, or `None` if it was terminated by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    /// A successful run producing `stdout`.
    pub fn ok(stdout: impl Into<String>) -> Self {
        Self {
            code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// A failed run producing `stderr`. `code` is the runtime's own exit status, which the error
    /// classifier uses alongside the text.
    pub fn err(code: i32, stderr: impl Into<String>) -> Self {
        Self {
            code: Some(code),
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    /// Whether the process exited zero.
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// The longest a streaming run leaves its caller without news while the process is silent.
///
/// `docker load` without a terminal prints one line, when the import is over; a 2 GiB archive
/// on a loaded runner took 10.5 s, past SC-004's ten-second limit on a still indicator (#790).
/// One second keeps the indicator visibly alive with room to spare.
pub const QUIET_TICK: Duration = Duration::from_secs(1);

/// What a streaming run tells its caller while the process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEvent<'a> {
    /// A line of output, from either stream, as it arrived.
    Line(&'a str),
    /// The process has printed nothing for [`QUIET_TICK`] and is still running; `elapsed` is the
    /// time since it was spawned.
    Quiet { elapsed: Duration },
}

/// What may end a streaming run early: a deadline, and a cancel (feature 806, FR-019).
///
/// The run kills its child and fails with `io::ErrorKind::TimedOut` or `Interrupted`, which
/// [`CliRuntime`](super::cli::CliRuntime) turns into the matching [`RuntimeError`](super::runtime::RuntimeError).
#[derive(Debug, Clone, Default)]
pub struct RunLimits {
    /// The longest the process may run.
    pub deadline: Option<Duration>,
    /// Cancelling it kills the process.
    pub cancel: Option<crate::provision::CancelToken>,
}

/// Runs one container-runtime invocation. The only impure operation in the sandbox layer.
///
/// Implementors must not interpret the arguments: classification of what an invocation *meant*
/// belongs to `parse` and `runtime`, above this seam.
pub trait CommandRunner: Send + Sync {
    /// Run `program` with `args`, waiting for it and capturing both streams.
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput>;

    /// Run `program`, invoking `on_event` for each line of output **as it arrives**, and with
    /// [`StreamEvent::Quiet`] whenever the process has been silent for [`QUIET_TICK`].
    ///
    /// Image acquisition is the one operation that may take minutes, and obligation C-8 requires
    /// progress to move while it does — SC-004 gives first-time enable five minutes, and five
    /// silent minutes reads as a hang. Capturing the whole output first and replaying it
    /// afterwards would satisfy the type and defeat the point.
    ///
    /// The default implementation does exactly that replay, which is correct for a fake (the
    /// canned output is already complete) and wrong for a real process — so [`SystemRunner`]
    /// overrides it.
    fn run_streaming(
        &self,
        program: &OsStr,
        args: &[OsString],
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        let out = self.run(program, args)?;
        for line in out.stdout.lines().chain(out.stderr.lines()) {
            on_event(StreamEvent::Line(line));
        }
        Ok(out)
    }

    /// [`Self::run_streaming`] that `limits` can end early. The default only honours a cancel that
    /// came before the run (a fake's canned output is already complete); [`SystemRunner`] kills the
    /// process.
    fn run_limited(
        &self,
        program: &OsStr,
        args: &[OsString],
        limits: &RunLimits,
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        if limits.cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        self.run_streaming(program, args, on_event)
    }
}

/// Spawns the runtime for real.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput> {
        let out = no_window(&mut Command::new(program)).args(args).output()?;
        Ok(CommandOutput {
            code: out.status.code(),
            // Runtime output is UTF-8 in practice; a lossy conversion keeps a malformed byte from
            // becoming an error that hides the real one. `parse` classifies unusable text anyway.
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn run_streaming(
        &self,
        program: &OsStr,
        args: &[OsString],
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        self.run_limited(program, args, &RunLimits::default(), on_event)
    }

    fn run_limited(
        &self,
        program: &OsStr,
        args: &[OsString],
        limits: &RunLimits,
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        use std::io::{BufRead, BufReader, Read};
        use std::process::Stdio;
        use std::sync::mpsc::{self, RecvTimeoutError};

        /// Forward each line of `pipe` as it arrives, tagged with whether it is stderr.
        fn forward(
            pipe: impl Read + Send + 'static,
            is_stderr: bool,
            tx: mpsc::Sender<(bool, String)>,
        ) -> std::thread::JoinHandle<()> {
            std::thread::spawn(move || {
                let mut reader = BufReader::new(pipe);
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 0 {
                    if tx.send((is_stderr, std::mem::take(&mut line))).is_err() {
                        break;
                    }
                }
            })
        }

        let started = std::time::Instant::now();
        let mut child = no_window(&mut Command::new(program))
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Both streams are read on their own threads, because a runtime that fills one pipe while
        // we only drain the other deadlocks — and `docker pull` writes progress to whichever it
        // feels like. This thread waits on the lines with a timeout, so a process that prints
        // nothing for minutes still lets the caller move its indicator (C-8, #790).
        let (tx, rx) = mpsc::channel();
        let readers: Vec<_> = [
            child.stdout.take().map(|p| forward(p, false, tx.clone())),
            child.stderr.take().map(|p| forward(p, true, tx.clone())),
        ]
        .into_iter()
        .flatten()
        .collect();
        drop(tx);

        let (mut stdout, mut stderr) = (String::new(), String::new());
        let mut ended_early = None;
        loop {
            // Checked every line and every tick, so a cancel or the deadline is noticed within
            // `QUIET_TICK` even from a process that prints nothing.
            if ended_early.is_none() {
                let cause = if limits.cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
                    Some((io::ErrorKind::Interrupted, "cancelled"))
                } else if limits.deadline.is_some_and(|d| started.elapsed() >= d) {
                    Some((io::ErrorKind::TimedOut, "ran past its time limit"))
                } else {
                    None
                };
                if let Some(cause) = cause {
                    let _ = child.kill();
                    ended_early = Some(cause);
                }
            }
            match rx.recv_timeout(QUIET_TICK) {
                Ok((is_stderr, line)) => {
                    on_event(StreamEvent::Line(line.trim_end()));
                    if is_stderr { &mut stderr } else { &mut stdout }.push_str(&line);
                }
                Err(RecvTimeoutError::Timeout) => on_event(StreamEvent::Quiet {
                    elapsed: started.elapsed(),
                }),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        for reader in readers {
            let _ = reader.join();
        }

        let status = child.wait()?;
        if let Some((kind, why)) = ended_early {
            return Err(io::Error::new(kind, why));
        }
        Ok(CommandOutput {
            code: status.code(),
            stdout,
            stderr,
        })
    }
}

/// Records every invocation and replays canned responses, in-process.
///
/// This is the harness the conformance suite runs against. A test asserts on [`Self::calls`] to
/// check *what was asked for*, and seeds [`Self::push`] to control *what came back* — including
/// failures that are awkward to arrange with a real runtime (daemon down, disk full, an image
/// vanishing between inspect and create).
#[derive(Debug, Default)]
pub struct RecordingRunner {
    calls: Mutex<Vec<Invocation>>,
    responses: Mutex<VecDeque<io::Result<CommandOutput>>>,
}

/// One recorded invocation: the program and the exact argument vector it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: OsString,
    pub args: Vec<OsString>,
}

impl Invocation {
    /// The argument vector as UTF-8 strings, for readable assertions.
    pub fn args_lossy(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// Whether the argument vector contains `flag` as a whole argument.
    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|a| a == flag)
    }

    /// The value following `flag`, if the vector contains `flag` and something after it.
    pub fn value_of(&self, flag: &str) -> Option<String> {
        let idx = self.args.iter().position(|a| a == flag)?;
        self.args
            .get(idx + 1)
            .map(|v| v.to_string_lossy().into_owned())
    }
}

impl RecordingRunner {
    /// A runner with no canned responses. Any invocation yields an empty success, which is enough
    /// for tests that only assert on argv.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the next response. Responses are consumed in order, one per invocation.
    pub fn push(&self, output: io::Result<CommandOutput>) -> &Self {
        self.responses.lock().expect("responses").push_back(output);
        self
    }

    /// Queue a successful response producing `stdout`.
    pub fn push_ok(&self, stdout: impl Into<String>) -> &Self {
        self.push(Ok(CommandOutput::ok(stdout)))
    }

    /// Every invocation so far, in order.
    pub fn calls(&self) -> Vec<Invocation> {
        self.calls.lock().expect("calls").clone()
    }

    /// The most recent invocation, for the common single-call assertion.
    pub fn last(&self) -> Option<Invocation> {
        self.calls.lock().expect("calls").last().cloned()
    }
}

impl CommandRunner for RecordingRunner {
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput> {
        self.calls.lock().expect("calls").push(Invocation {
            program: program.to_os_string(),
            args: args.to_vec(),
        });
        self.responses
            .lock()
            .expect("responses")
            .pop_front()
            .unwrap_or_else(|| Ok(CommandOutput::ok("")))
    }
}

/// A borrowed runner is a runner.
///
/// [`CliRuntime::new`](super::cli::CliRuntime::new) takes its runner by value, which is right for
/// production — the runtime owns its spawn shim — and leaves a test holding nothing. The argv log
/// a [`RecordingRunner`] accumulates is the assertion in checks like "the runtime the user did not
/// select is never invoked", so the test has to keep the recorder and lend it out.
impl<R: CommandRunner + ?Sized> CommandRunner for &R {
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput> {
        (**self).run(program, args)
    }

    fn run_streaming(
        &self,
        program: &OsStr,
        args: &[OsString],
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        (**self).run_streaming(program, args, on_event)
    }

    fn run_limited(
        &self,
        program: &OsStr,
        args: &[OsString],
        limits: &RunLimits,
        on_event: &mut dyn FnMut(StreamEvent<'_>),
    ) -> io::Result<CommandOutput> {
        (**self).run_limited(program, args, limits, on_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn recording_runner_records_argv_verbatim() {
        let runner = RecordingRunner::new();
        runner
            .run(OsStr::new("docker"), &os(&["run", "--rm", "alpine"]))
            .unwrap();

        let call = runner.last().expect("one call recorded");
        assert_eq!(call.program, OsString::from("docker"));
        assert_eq!(call.args_lossy(), vec!["run", "--rm", "alpine"]);
        assert!(call.has_flag("--rm"));
    }

    #[test]
    fn value_of_reads_the_argument_after_a_flag() {
        let runner = RecordingRunner::new();
        runner
            .run(OsStr::new("docker"), &os(&["run", "--cpus", "2.0"]))
            .unwrap();
        let call = runner.last().unwrap();
        assert_eq!(call.value_of("--cpus").as_deref(), Some("2.0"));
        // A flag present but last has no value, and must not panic on the lookup.
        assert_eq!(call.value_of("2.0"), None);
    }

    #[test]
    fn responses_are_replayed_in_order_then_default_to_empty_success() {
        let runner = RecordingRunner::new();
        runner.push_ok("first").push_ok("second");

        assert_eq!(runner.run(OsStr::new("d"), &[]).unwrap().stdout, "first");
        assert_eq!(runner.run(OsStr::new("d"), &[]).unwrap().stdout, "second");
        // Beyond the queue: still a success, so a test that only cares about argv need not seed it.
        let third = runner.run(OsStr::new("d"), &[]).unwrap();
        assert!(third.success());
        assert_eq!(third.stdout, "");
        assert_eq!(runner.calls().len(), 3);
    }

    #[test]
    fn a_canned_failure_carries_its_code_and_stderr() {
        let runner = RecordingRunner::new();
        runner.push(Ok(CommandOutput::err(
            125,
            "Cannot connect to the Docker daemon",
        )));
        let out = runner.run(OsStr::new("docker"), &[]).unwrap();
        assert!(!out.success());
        assert_eq!(out.code, Some(125));
        assert!(out.stderr.contains("Cannot connect"));
    }

    /// The one thing the injected fake cannot cover: that [`SystemRunner`] actually spawns a
    /// process and captures its output. Uses a command every supported platform ships.
    #[test]
    fn system_runner_captures_a_real_process() {
        let (program, args) = if cfg!(windows) {
            ("cmd", os(&["/C", "echo micold"]))
        } else {
            ("echo", os(&["micold"]))
        };
        let out = SystemRunner
            .run(OsStr::new(program), &args)
            .expect("spawn a process every platform has");
        assert!(out.success());
        assert!(out.stdout.contains("micold"), "stdout was {:?}", out.stdout);
    }

    /// A process silent for longer than the progress indicator may stand still (SC-004, C-8).
    /// `docker load` without a terminal is one: it prints nothing until the import is over.
    fn silent_then_done() -> (&'static str, Vec<OsString>) {
        if cfg!(windows) {
            ("cmd", os(&["/C", "ping -n 4 127.0.0.1 >NUL & echo done"]))
        } else {
            ("sh", os(&["-c", "sleep 3; echo done"]))
        }
    }

    /// Issue #790: a 2 GiB import went 10.5 s with one report, at the end. Streaming must keep
    /// the caller hearing from the runner while the process says nothing.
    #[test]
    fn system_runner_streaming_never_goes_quiet_while_the_process_is_silent() {
        let (program, args) = silent_then_done();
        let started = std::time::Instant::now();
        let mut heard = Vec::new();
        let mut quiet = 0;
        let out = SystemRunner
            .run_streaming(OsStr::new(program), &args, &mut |event| {
                quiet += usize::from(matches!(event, StreamEvent::Quiet { .. }));
                heard.push(started.elapsed())
            })
            .expect("spawn a process every platform has");
        let finished = started.elapsed();

        assert!(out.success());
        assert!(out.stdout.contains("done"), "stdout was {:?}", out.stdout);
        let mut previous = std::time::Duration::ZERO;
        let mut longest = std::time::Duration::ZERO;
        for at in heard.iter().copied().chain([finished]) {
            longest = longest.max(at - previous);
            previous = at;
        }
        assert!(quiet >= 1, "a silent process produced no quiet events");
        // Twice the tick: a loaded runner may wake late, but never by a whole tick.
        assert!(
            longest <= 2 * QUIET_TICK,
            "the caller heard nothing for {}ms; heard at {heard:?}",
            longest.as_millis()
        );
    }

    #[test]
    fn system_runner_reports_a_missing_program_as_an_io_error() {
        // Classification of *what* a missing program means belongs above this seam; here it is
        // simply an io error rather than a panic or a silent empty success.
        let err = SystemRunner
            .run(OsStr::new("micold-no-such-runtime-binary"), &[])
            .expect_err("a missing program cannot succeed");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    /// A process that outlives its limits, as a `sh` script (every Linux and macOS runner has one).
    #[cfg(unix)]
    fn long_sleep() -> (OsString, Vec<OsString>) {
        (
            OsString::from("sh"),
            vec!["-c".into(), "exec sleep 30".into()],
        )
    }

    #[cfg(unix)]
    #[test]
    fn a_run_past_its_deadline_is_killed_and_fails_timed_out() {
        let (program, args) = long_sleep();
        let limits = RunLimits {
            deadline: Some(Duration::from_millis(300)),
            cancel: None,
        };
        let started = std::time::Instant::now();
        let err = SystemRunner
            .run_limited(&program, &args, &limits, &mut |_| {})
            .expect_err("a 30 s sleep cannot finish in 300 ms");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the child was waited out, not killed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_cancel_kills_the_child_of_a_run() {
        let (program, args) = long_sleep();
        let token = crate::provision::CancelToken::new();
        let limits = RunLimits {
            deadline: None,
            cancel: Some(token.clone()),
        };
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            token.cancel();
        });
        let started = std::time::Instant::now();
        let err = SystemRunner
            .run_limited(&program, &args, &limits, &mut |_| {})
            .expect_err("cancelled");
        canceller.join().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_run_cancelled_before_it_starts_never_spawns() {
        let token = crate::provision::CancelToken::new();
        token.cancel();
        let limits = RunLimits {
            deadline: None,
            cancel: Some(token),
        };
        let runner = RecordingRunner::new();
        let err = runner
            .run_limited(OsStr::new("docker"), &[], &limits, &mut |_| {})
            .expect_err("cancelled");
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(runner.calls().is_empty());
    }
}

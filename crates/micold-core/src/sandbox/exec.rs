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

/// Runs one container-runtime invocation. The only impure operation in the sandbox layer.
///
/// Implementors must not interpret the arguments: classification of what an invocation *meant*
/// belongs to `parse` and `runtime`, above this seam.
pub trait CommandRunner: Send + Sync {
    /// Run `program` with `args`, waiting for it and capturing both streams.
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput>;

    /// Run `program`, invoking `on_line` for each line of output **as it arrives**.
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
        on_line: &mut dyn FnMut(&str),
    ) -> io::Result<CommandOutput> {
        let out = self.run(program, args)?;
        for line in out.stdout.lines().chain(out.stderr.lines()) {
            on_line(line);
        }
        Ok(out)
    }
}

/// Spawns the runtime for real.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &OsStr, args: &[OsString]) -> io::Result<CommandOutput> {
        let out = Command::new(program).args(args).output()?;
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
        on_line: &mut dyn FnMut(&str),
    ) -> io::Result<CommandOutput> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;

        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // stdout is read on this thread and stderr on another, because a runtime that fills the
        // stderr pipe while we are only draining stdout deadlocks — and `docker pull` writes
        // progress to whichever it feels like.
        let mut stderr = child.stderr.take().map(BufReader::new);
        let stderr_thread = stderr.take().map(|mut r| {
            std::thread::spawn(move || {
                let mut collected = String::new();
                let mut line = String::new();
                while r.read_line(&mut line).unwrap_or(0) > 0 {
                    collected.push_str(&line);
                    line.clear();
                }
                collected
            })
        });

        let mut stdout = String::new();
        if let Some(out) = child.stdout.take() {
            let mut reader = BufReader::new(out);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                on_line(line.trim_end());
                stdout.push_str(&line);
                line.clear();
            }
        }

        let status = child.wait()?;
        let stderr = stderr_thread
            .and_then(|t| t.join().ok())
            .unwrap_or_default();
        for line in stderr.lines() {
            on_line(line);
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

    #[test]
    fn system_runner_reports_a_missing_program_as_an_io_error() {
        // Classification of *what* a missing program means belongs above this seam; here it is
        // simply an io error rather than a panic or a silent empty success.
        let err = SystemRunner
            .run(OsStr::new("micold-no-such-runtime-binary"), &[])
            .expect_err("a missing program cannot succeed");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}

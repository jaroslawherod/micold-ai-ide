//! Environment-include sourcing/diffing engine (feature 011).
//!
//! Resolves the user's normal shell environment by actually sourcing a configured rc-style
//! script in a real, disposable, timeout-bounded shell process and diffing its resulting
//! environment against a clean baseline — not by parsing the script's text (FR-005), since
//! conditionals, sourced sub-files, and version-manager init blocks can't be statically
//! evaluated. Needs only `std::process`, so this module is core (not `gui`-gated) and is
//! exercised by `cargo test --no-default-features` with real disposable subprocesses (research
//! R4, contracts/env-include-resolution.md).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The hardcoded terminal-emulation pair every launch always carries (`main.rs`, pre-existing).
const TERM_KEY: &str = "TERM";
const TERM_VALUE: &str = "xterm-256color";

/// How long to poll a spawned child's exit status before considering it hung (research R2).
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// The result of the most recent attempt to resolve the include script (data-model.md).
/// An enum (not a `bool` + `Option<String>`) so "failed but has no category" or "succeeded but
/// also has a failure category" cannot be constructed (Constitution Principle V).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvIncludeOutcome {
    /// The feature is off, or the configured path is empty/blank (spec Edge Cases). `resolve()`
    /// never returns this itself — it is the caller's short-circuit before invoking `resolve()`
    /// at all (contracts/env-include-resolution.md's Non-goals).
    Disabled,
    /// Resolved successfully (an empty/no-op script still counts as `Success`, spec Edge Cases).
    Success,
    /// The configured path did not exist at resolution time — checked before any subprocess is
    /// spawned (research R1).
    MissingScript,
    /// The script ran but its last command / an explicit `exit` produced a non-zero status.
    NonZeroExit { code: i32, diagnostic: String },
    /// Sourcing did not complete within the configured timeout; the subprocess was killed.
    TimedOut { diagnostic: String },
}

/// Parse a NUL-delimited `KEY=VALUE` environment dump (`env -0` on Unix, the PowerShell
/// equivalent on Windows — research R6) into a lookup map. Pure — no I/O.
pub fn parse_env_dump(bytes: &[u8]) -> HashMap<String, String> {
    bytes
        .split(|&b| b == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let text = String::from_utf8_lossy(entry);
            text.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

/// Everything `attempt` added or changed relative to `baseline` (research R3). Keys present in
/// `baseline` but absent from `attempt` (an `unset` by the script) are NOT reported — the merge
/// target (`CommandBuilder::env`) is itself additive/overwrite-only and has no "unset" to apply.
/// Pure — no I/O.
pub fn diff_env(
    baseline: &HashMap<String, String>,
    attempt: &HashMap<String, String>,
) -> Vec<(String, String)> {
    attempt
        .iter()
        .filter(|(k, v)| baseline.get(*k) != Some(*v))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Append the hardcoded `TERM` pair last, after removing any captured `TERM` entry, so exactly
/// one `TERM` entry survives and it is always the hardcoded value (FR-009). Pure — no I/O.
pub fn merge_with_term(vars: &[(String, String)]) -> Vec<(String, String)> {
    let mut merged: Vec<(String, String)> = vars
        .iter()
        .filter(|(k, _)| k != TERM_KEY)
        .cloned()
        .collect();
    merged.push((TERM_KEY.to_string(), TERM_VALUE.to_string()));
    merged
}

/// How a bounded subprocess run concluded — kept distinct from `EnvIncludeOutcome` so `resolve()`
/// decides the public category from an unambiguous fact (did it exit, or did we have to kill it)
/// rather than inferring "was this a timeout?" from elapsed time, which would be racy right at
/// the timeout boundary.
enum RunOutcome {
    /// The process exited on its own within `timeout`.
    Exited {
        code: i32,
        stdout: Vec<u8>,
        stderr: String,
    },
    /// The process was still running at `timeout` and was killed.
    TimedOut { stderr: String },
    /// The process could not even be spawned (e.g. the interpreter binary is missing).
    SpawnFailed(String),
}

/// Reported when the budget ran out before a baseline environment could be established (BUG-003).
///
/// A distinct message rather than an empty diagnostic, because the two timeouts mean different
/// things to whoever reads them: the script was slow, or the budget was never big enough to look at
/// the script at all. The second is a setting to raise, not a script to debug.
const BASELINE_UNAVAILABLE: &str =
    "the timeout expired while establishing the baseline environment, before the script was \
     sourced — raise the env-include timeout";

/// Kill every process in `pid`'s process group (Unix only — `cmd` is spawned with
/// `process_group(0)` below, so the group id equals the child's own pid) via a direct `kill(2)`
/// syscall. Deliberately NOT implemented by spawning the `kill(1)` *binary* as a subprocess —
/// that proved unreliable in sandboxed environments during development (the spawned `kill`
/// process reported success without the target group actually dying), whereas the direct syscall
/// from this same process is unambiguous.
#[cfg(unix)]
fn kill_process_group(pid: u32) {
    // Safety: `kill(2)` with a negative pid signals the process group; passing an invalid/already-
    // reaped group id is a documented, safe no-op (returns -1/ESRCH) rather than undefined
    // behavior.
    unsafe {
        libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
    }
}

/// A Windows Job Object owning the spawned process and everything it starts (BUG-003).
///
/// The counterpart to `process_group(0)` + `kill(-pid)` on Unix, and it exists for the same reason:
/// `read_to_end` below returns when the pipe has no *writers* left, not when the child this module
/// spawned has died. A profile that backgrounds a helper — a version manager, an agent daemon —
/// leaves that helper holding the pipe, and without a whole-tree kill the read waits for it. On
/// Windows that wait was unbounded: `Child::kill` terminates one process and none of its
/// descendants.
///
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` means closing the handle kills whatever is still in the
/// job, so the `Drop` below is the backstop even on a path that forgets to terminate explicitly.
///
/// **The assignment races the child.** `CreateProcessW` has already started the process by the time
/// `spawn()` returns, so a child that spawns a grandchild in its first instants could in principle
/// escape. Closing that window needs `CREATE_SUSPENDED` and a `ResumeThread`, which needs the main
/// thread handle — and `std`'s `Child` does not expose one. The window is microseconds against
/// interpreter startup measured in tens of milliseconds, and an escape degrades to exactly the
/// behaviour this replaces rather than to something worse.
#[cfg(windows)]
struct JobHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl JobHandle {
    /// Create a job and put `child` in it. `None` if either step fails — in which case the caller
    /// falls back to killing the top-level process alone, which is what it did before this existed.
    fn capture(child: &std::process::Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        // Safety: a null name and null security attributes are the documented "anonymous job"
        // arguments; the call returns a null handle on failure rather than misbehaving.
        let job: HANDLE = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return None;
        }
        let handle = Self(job);

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // Safety: `info` is a correctly-sized, fully-initialised value of the type the
        // `JobObjectExtendedLimitInformation` class requires, and its length is passed alongside.
        let set = unsafe {
            SetInformationJobObject(
                handle.0,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if set == 0 {
            return None;
        }
        // Safety: the handle comes from a live `Child` this function borrows, so it is valid for
        // the duration of the call.
        let assigned =
            unsafe { AssignProcessToJobObject(handle.0, child.as_raw_handle() as HANDLE) };
        if assigned == 0 {
            return None;
        }
        Some(handle)
    }

    /// Kill every process still in the job.
    fn terminate(&self) {
        // Safety: `self.0` is a job handle owned by this value and closed only in `Drop`.
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for JobHandle {
    fn drop(&mut self) {
        // Closing the handle kills whatever remains, per the limit flag set above.
        // Safety: closing a handle this value owns, exactly once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Run `cmd`, waiting up to `timeout` for it to exit (research R2: `spawn()` + `try_wait()`
/// poll, not the blocking `.output()`). On Unix, `cmd` is spawned in its own process group so
/// that on timeout — or even after a natural exit — the ENTIRE group (not just the top-level
/// process) is killed before reading its piped stdout/stderr: a sourced rc file MAY background a
/// process (an agent daemon, a version-manager helper); if such a grandchild inherits the pipe
/// and outlives its parent, reading to EOF would otherwise block forever (this exact deadlock was
/// observed with a `sleep`-under-`source`-under-command-substitution during development).
fn run_bounded(mut cmd: Command, timeout: Duration) -> RunOutcome {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return RunOutcome::SpawnFailed(err.to_string()),
    };
    // Only the Unix kill needs the pid; binding it unconditionally warns on Windows, and this
    // crate is built with warnings denied.
    #[cfg(unix)]
    let pid = child.id();
    // Windows' counterpart to the process group above, assigned as soon as the child exists.
    #[cfg(windows)]
    let job = JobHandle::capture(&child);

    let start = Instant::now();
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    break true;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(err) => return RunOutcome::SpawnFailed(err.to_string()),
        }
    };

    // Kill the whole group unconditionally (whether it exited on its own or timed out) so no
    // orphaned grandchild can keep the pipes open — see the doc comment above.
    #[cfg(unix)]
    kill_process_group(pid);
    #[cfg(windows)]
    {
        // The job first — it takes the whole tree, including anything holding the pipes read
        // below. `child.kill()` after it, for the case where the job could not be created at all.
        if let Some(job) = &job {
            job.terminate();
        }
        let _ = child.kill();
    }
    #[cfg(not(any(unix, windows)))]
    let _ = child.kill();

    let mut stdout = Vec::new();
    let mut stderr = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }

    if timed_out {
        RunOutcome::TimedOut { stderr }
    } else {
        match child.wait() {
            Ok(status) => RunOutcome::Exited {
                code: status.code().unwrap_or(-1),
                stdout,
                stderr,
            },
            Err(err) => RunOutcome::SpawnFailed(err.to_string()),
        }
    }
}

#[cfg(not(windows))]
fn baseline_env(cwd: &Path, budget: Duration) -> Option<HashMap<String, String>> {
    // Deliberately reuses `attempt_env`'s exact wrapper (sourcing `/dev/null`, a guaranteed-empty
    // no-op) rather than a bare `env -0`: bash tail-call-optimizes a `-c` script whose last
    // command is a single simple external command (execve()-replacing itself, no fork) but
    // cannot when anything follows (here, `exit $status`) — and the two paths report a
    // different `SHLVL` for reasons internal to bash, which would otherwise show up as a
    // spurious diff on *every* resolution regardless of the actual script's content. Keeping
    // baseline and attempt structurally identical (same fork/exec shape) eliminates this whole
    // class of bash-internal artifact, not just the one variable this was caught by.
    //
    // `cwd` (BUG-002) MUST be the same directory `attempt_env` is about to run in, not a fixed
    // neutral one: bash exports `PWD` (and `OLDPWD`) based on the shell's own working directory,
    // so a baseline computed from a different cwd than the attempt would make `PWD` show up as a
    // spurious diff on every resolution, exactly like the `SHLVL` artifact above. Since this
    // baseline sources `/dev/null` (never the real script), running it in `cwd` does not trigger
    // any directory-dependent hook the real script might contain — only the real `attempt_env`
    // call actually sources user content there.
    match attempt_env(Path::new("/dev/null"), cwd, budget) {
        RunOutcome::Exited { stdout, .. } => Some(parse_env_dump(&stdout)),
        // `None`, never an empty map (BUG-003). Diffing a real environment against nothing reports
        // every variable in it as newly set by the user's script — a wrong answer dressed as a
        // successful one, which is worse than the timeout it is hiding.
        _ => None,
    }
}

#[cfg(not(windows))]
fn attempt_env(path: &Path, cwd: &Path, timeout: Duration) -> RunOutcome {
    let mut cmd = Command::new("bash");
    cmd.current_dir(cwd);
    // `source "$1"` runs directly in THIS shell (never inside a `$(...)` subshell) so any
    // `export`s it makes land in the environment `env -0` dumps below — an earlier version
    // captured the script's own output via `out=$(source "$1" 2>&1)`, which silently discarded
    // every exported variable, since command substitution runs in a subshell whose environment
    // changes are lost when it exits.
    //
    // Because `source` is NOT subshelled, a sourced script calling `exit` directly would
    // otherwise kill this entire wrapper before it reaches `env -0` — an `EXIT` trap set before
    // sourcing catches that (an EXIT trap fires on ANY shell termination, explicit `exit` or
    // falling off the end alike) and does the env dump + diagnostic printing + final `exit
    // "$status"` there instead, so the sourced script's own exit code is still what this whole
    // process reports. Note: this trap does NOT run if the process is later SIGKILLed on timeout
    // (SIGKILL is unblockable) — a `TimedOut` outcome's diagnostic may therefore be empty even if
    // the script had already printed something; guaranteed cleanup was chosen over best-effort
    // diagnostics for that one narrow case.
    //
    // `-i` (BUG-001): without it, `$-` never contains `i`, so a script that itself gates on
    // shell-interactivity — Debian/Ubuntu's stock `~/.bashrc` (the FR-004 default) opens with
    // exactly such a guard — returns before running any of its own exports. `-i` makes this a
    // real interactive shell for `$-`'s purposes while `-c` still just runs the wrapper command
    // and exits (no prompt is ever read), so the EXIT-trap/timeout/kill/diffing behavior above is
    // unchanged; stray job-control warnings an interactive shell prints when there's no
    // controlling tty (e.g. "no job control in this shell") land on stderr, never on the `env -0`
    // stdout dump this wrapper parses.
    cmd.arg("--noprofile").arg("--norc").arg("-i").arg("-c").arg(
        r#"diag_file=$(mktemp); trap 'status=$?; printf "%s" "$(cat "$diag_file" 2>/dev/null)" >&2; rm -f "$diag_file"; env -0; exit "$status"' EXIT; source "$1" >"$diag_file" 2>&1"#,
    );
    cmd.arg("--").arg(path);
    run_bounded(cmd, timeout)
}

#[cfg(windows)]
fn baseline_env(cwd: &Path, budget: Duration) -> Option<HashMap<String, String>> {
    // `cwd` (BUG-002): matches `attempt_env`'s working directory for the same reason the Unix
    // branch does — even though `[System.Environment]::GetEnvironmentVariables()` (process env
    // vars) isn't expected to vary with the shell's cwd the way bash's `PWD` does, running both
    // subprocesses from the same directory keeps this branch structurally parallel and immune to
    // any such quirk.
    let mut cmd = Command::new("powershell.exe");
    cmd.current_dir(cwd);
    cmd.arg("-NoProfile").arg("-Command").arg(
        "[System.Environment]::GetEnvironmentVariables().GetEnumerator() | ForEach-Object { \
         [Console]::Out.Write(\"$($_.Key)=$($_.Value)`0\") }",
    );
    match run_bounded(cmd, budget) {
        RunOutcome::Exited { stdout, .. } => Some(parse_env_dump(&stdout)),
        // See the Unix branch: `None` rather than an empty map (BUG-003).
        _ => None,
    }
}

#[cfg(windows)]
fn attempt_env(path: &Path, cwd: &Path, timeout: Duration) -> RunOutcome {
    let script = format!(
        "$out = try {{ . '{}' 2>&1 | Out-String }} catch {{ $_.Exception.Message }}; \
         $status = if ($LASTEXITCODE) {{ $LASTEXITCODE }} else {{ 0 }}; \
         [Console]::Error.Write($out); \
         [System.Environment]::GetEnvironmentVariables().GetEnumerator() | ForEach-Object {{ \
         [Console]::Out.Write(\"$($_.Key)=$($_.Value)`0\") }}; \
         exit $status",
        path.display()
    );
    let mut cmd = Command::new("powershell.exe");
    cmd.current_dir(cwd);
    cmd.arg("-NoProfile").arg("-Command").arg(script);
    run_bounded(cmd, timeout)
}

/// Resolve `path`'s effect on the environment by actually sourcing it in a real, disposable
/// shell process bounded by `timeout`, diffed against a clean baseline (contracts/
/// env-include-resolution.md). `cwd` is the sourcing subprocess's working directory — the
/// session's own project/worktree directory — so directory-dependent `PATH` contributions from a
/// version manager (mise, asdf, nvm, pyenv, rbenv, etc.) resolve the same way they would in a
/// real interactive shell opened there (FR-020, BUG-002); the baseline is ALSO run against `cwd`
/// (see `baseline_env`'s doc comment) — it just never sources `path` itself, so a directory-
/// dependent hook in the real script still only fires for the attempt, never the baseline. Never
/// blocks longer than `timeout` for the sourcing attempt itself (research R2). Does not decide
/// whether to run at all (see the contract's Non-goals) — callers only invoke this when the
/// feature is enabled with a non-blank path.
pub fn resolve(
    path: &Path,
    cwd: &Path,
    timeout: Duration,
) -> (Vec<(String, String)>, EnvIncludeOutcome) {
    if !path.exists() {
        return (Vec::new(), EnvIncludeOutcome::MissingScript);
    }

    // ONE budget for the whole call (BUG-003). The baseline probe used to carry its own hardcoded
    // ten seconds, so a caller asking for 500ms could wait ten and a half — and `timeout` is the
    // user's `env_include_timeout_secs`, whose promise is about how long opening a project may
    // stall, not about one of the two subprocesses it takes to answer.
    let started = Instant::now();
    let Some(baseline) = baseline_env(cwd, timeout) else {
        return (
            Vec::new(),
            EnvIncludeOutcome::TimedOut {
                diagnostic: BASELINE_UNAVAILABLE.to_string(),
            },
        );
    };
    let remaining = timeout.saturating_sub(started.elapsed());
    if remaining.is_zero() {
        return (
            Vec::new(),
            EnvIncludeOutcome::TimedOut {
                diagnostic: BASELINE_UNAVAILABLE.to_string(),
            },
        );
    }

    match attempt_env(path, cwd, remaining) {
        RunOutcome::Exited {
            code: 0, stdout, ..
        } => {
            let attempt = parse_env_dump(&stdout);
            (diff_env(&baseline, &attempt), EnvIncludeOutcome::Success)
        }
        RunOutcome::Exited { code, stderr, .. } => (
            Vec::new(),
            EnvIncludeOutcome::NonZeroExit {
                code,
                diagnostic: stderr,
            },
        ),
        RunOutcome::TimedOut { stderr } => (
            Vec::new(),
            EnvIncludeOutcome::TimedOut { diagnostic: stderr },
        ),
        RunOutcome::SpawnFailed(diagnostic) => (
            Vec::new(),
            EnvIncludeOutcome::NonZeroExit {
                code: -1,
                diagnostic,
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// The capability (feature 021, T046) — declared here, beside the engine it wraps.
// ---------------------------------------------------------------------------

/// The result of resolving the include script for one directory: the variables it contributed and
/// how the attempt went.
///
/// Moved here from the client's `main.rs` at T046. It was already the shape both halves agreed on;
/// it lived in the shell only because the decision that produces it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvIncludeSnapshot {
    /// Resolved variables. Vestigial on the client now that the daemon resolves env at spawn time;
    /// kept so the Settings resolution path is unchanged.
    pub vars: Vec<(String, String)>,
    /// How the attempt went — including [`EnvIncludeOutcome::Disabled`] when none was made.
    pub outcome: EnvIncludeOutcome,
}

/// Sourcing an include script: the one I/O need this module has (FR-015).
///
/// **Deliberately narrower than the contract's sketch**, which was
/// `fn resolve(&self, cwd: &Path) -> EnvIncludeSnapshot`. That shape folds the *decision* — is the
/// feature on, is the path blank — into the port, which costs twice. It makes the real
/// implementation carry settings, so it cannot be a unit struct; and it makes a fake unable to
/// answer the question a test most wants to ask, which is *what was I asked to source, and was I
/// asked at all*. FR-016 wants a capability to be the I/O need and nothing more, so the decision
/// stays where it can be tested without a port at all: [`snapshot_for`], pure and beside this.
pub trait EnvIncludeResolver {
    /// Source `path` in a disposable shell rooted at `cwd`, bounded by `timeout`, and report the
    /// variables it contributed with the outcome of the attempt.
    fn resolve(
        &self,
        path: &Path,
        cwd: &Path,
        timeout: Duration,
    ) -> (Vec<(String, String)>, EnvIncludeOutcome);
}

/// The real resolver: a disposable, timeout-bounded subprocess (FR-005).
#[derive(Debug, Default, Clone, Copy)]
pub struct SubprocessResolver;

impl EnvIncludeResolver for SubprocessResolver {
    fn resolve(
        &self,
        path: &Path,
        cwd: &Path,
        timeout: Duration,
    ) -> (Vec<(String, String)>, EnvIncludeOutcome) {
        resolve(path, cwd, timeout)
    }
}

/// A resolver that spawns nothing and remembers what it was asked (FR-019).
///
/// Records every call so a test can assert the *absence* of one — which is the whole point for
/// this capability: the short-circuit's claim is not merely that the outcome is `Disabled`, it is
/// that no subprocess was spawned at all. An outcome can be right for the wrong reason; a call
/// count cannot.
#[derive(Debug, Default)]
pub struct FakeEnvIncludeResolver {
    inner: RefCell<FakeResolverState>,
}

#[derive(Debug, Default)]
struct FakeResolverState {
    calls: Vec<(PathBuf, PathBuf, Duration)>,
    answer: Option<(Vec<(String, String)>, EnvIncludeOutcome)>,
}

impl FakeEnvIncludeResolver {
    /// A resolver that answers every call with `vars` and `outcome`.
    pub fn answering(vars: Vec<(String, String)>, outcome: EnvIncludeOutcome) -> Self {
        Self {
            inner: RefCell::new(FakeResolverState {
                calls: Vec::new(),
                answer: Some((vars, outcome)),
            }),
        }
    }

    /// Every `(path, cwd, timeout)` this resolver was asked to source, in order (test assertions).
    pub fn calls(&self) -> Vec<(PathBuf, PathBuf, Duration)> {
        self.inner.borrow().calls.clone()
    }
}

impl EnvIncludeResolver for FakeEnvIncludeResolver {
    fn resolve(
        &self,
        path: &Path,
        cwd: &Path,
        timeout: Duration,
    ) -> (Vec<(String, String)>, EnvIncludeOutcome) {
        let mut state = self.inner.borrow_mut();
        state
            .calls
            .push((path.to_path_buf(), cwd.to_path_buf(), timeout));
        state
            .answer
            .clone()
            .unwrap_or((Vec::new(), EnvIncludeOutcome::Success))
    }
}

/// Whether to source at all, and the snapshot either way (feature 011 Edge Cases).
///
/// The engine never decides whether to run — that is this function, moved out of the client's
/// `main.rs` at T046 so every caller applies the same short-circuit and so it can be tested
/// without a shell. Off, or a blank/whitespace path, yields [`EnvIncludeOutcome::Disabled`]
/// **without touching `resolver`**: no subprocess is spawned, which is the part that matters.
pub fn snapshot_for(
    resolver: &dyn EnvIncludeResolver,
    enabled: bool,
    script_path: &str,
    timeout: Duration,
    cwd: &Path,
) -> EnvIncludeSnapshot {
    if !enabled || script_path.trim().is_empty() {
        return EnvIncludeSnapshot {
            vars: Vec::new(),
            outcome: EnvIncludeOutcome::Disabled,
        };
    }
    let (vars, outcome) = resolver.resolve(Path::new(script_path), cwd, timeout);
    EnvIncludeSnapshot { vars, outcome }
}

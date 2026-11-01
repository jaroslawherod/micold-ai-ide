//! Environment-include sourcing/diffing engine (feature 011).
//!
//! Resolves the user's normal shell environment by actually sourcing a configured rc-style
//! script in a real, disposable, timeout-bounded shell process and diffing its resulting
//! environment against a clean baseline — not by parsing the script's text (FR-005), since
//! conditionals, sourced sub-files, and version-manager init blocks can't be statically
//! evaluated. Needs only `std::process`, so this module is core (not `gui`-gated) and is
//! exercised by `cargo test --no-default-features` with real disposable subprocesses (research
//! R4, contracts/env-include-resolution.md).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::process::{run_bounded, ExitKind, RunOutcome, Stream, StreamingChild};

/// The hardcoded terminal-emulation pair every launch always carries (`main.rs`, pre-existing).
const TERM_KEY: &str = "TERM";
const TERM_VALUE: &str = "xterm-256color";

/// The result of the most recent attempt to resolve the include script (data-model.md).
/// An enum (not a `bool` + `Option<String>`) so "failed but has no category" or "succeeded but
/// also has a failure category" cannot be constructed (Constitution Principle V).
///
/// Serializable because the service reports a failed attempt per session directory to its clients
/// in the catalog snapshot (FR-022, BUG-454: `CatalogSnapshot::env_include_failures`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Whether an inherited `key=value` is another terminal's identity, which a session must not see
/// (feature 031, FR-006, contracts/session-terminal-identity.md §1).
///
/// Keys compare exactly when `keys_case_insensitive` is false and ASCII case-insensitively when it
/// is true; callers pass `cfg!(windows)`, so core never reads the host OS.
pub fn is_inherited_terminal_identity(key: &str, value: &str, keys_case_insensitive: bool) -> bool {
    let is = |name: &str| {
        if keys_case_insensitive {
            key.eq_ignore_ascii_case(name)
        } else {
            key == name
        }
    };
    if is("COLORTERM") {
        // A colour-depth value is a capability every session may keep; anything else names the
        // terminal that set it.
        return !(value.eq_ignore_ascii_case("truecolor") || value.eq_ignore_ascii_case("24bit"));
    }
    TERMINAL_IDENTITY_KEYS.iter().any(|name| is(name))
}

/// The keys that name another terminal whatever their value (contract §1).
const TERMINAL_IDENTITY_KEYS: [&str; 9] = [
    "FORCE_HYPERLINK",
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "VTE_VERSION",
    "WT_SESSION",
    "WT_PROFILE_ID",
    "TERMINAL_EMULATOR",
    "KONSOLE_VERSION",
    "DOMTERM",
];

/// Remove from `cmd` every variable of `inherited` that names another terminal (contract §3), so
/// the include shell's diff sees a script that sets one as adding it.
fn remove_inherited_terminal_identity(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) {
    for (key, value) in inherited {
        if is_inherited_terminal_identity(
            &key.to_string_lossy(),
            &value.to_string_lossy(),
            cfg!(windows),
        ) {
            cmd.env_remove(key);
        }
    }
}

/// Reported when the budget ran out before a baseline environment could be established (BUG-003).
///
/// A distinct message rather than an empty diagnostic, because the two timeouts mean different
/// things to whoever reads them: the script was slow, or the budget was never big enough to look at
/// the script at all. The second is a setting to raise, not a script to debug.
const BASELINE_UNAVAILABLE: &str =
    "the timeout expired while establishing the baseline environment, before the script was \
     sourced — raise the env-include timeout";

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
    run_bounded(bash_command(path, cwd, std::env::vars_os()), timeout)
}

/// The Unix include shell, shared by `baseline_env` and `attempt_env`, with every variable of
/// `inherited` that names another terminal removed (feature 031, contract
/// session-terminal-identity §3).
#[cfg(not(windows))]
fn bash_command(
    path: &Path,
    cwd: &Path,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let mut cmd = Command::new("bash");
    remove_inherited_terminal_identity(&mut cmd, inherited);
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
    cmd
}

#[cfg(windows)]
fn baseline_env(cwd: &Path, budget: Duration) -> Option<HashMap<String, String>> {
    match run_bounded(
        powershell_baseline_command(cwd, std::env::vars_os()),
        budget,
    ) {
        RunOutcome::Exited { stdout, .. } => Some(parse_env_dump(&stdout)),
        // See the Unix branch: `None` rather than an empty map (BUG-003).
        _ => None,
    }
}

/// The Windows baseline shell, with `inherited`'s terminal identity removed (contract §3). Built on
/// every OS so the removal is testable here; only Windows runs it.
#[cfg(any(windows, test))]
fn powershell_baseline_command(
    cwd: &Path,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    // `cwd` (BUG-002): matches `attempt_env`'s working directory for the same reason the Unix
    // branch does — even though `[System.Environment]::GetEnvironmentVariables()` (process env
    // vars) isn't expected to vary with the shell's cwd the way bash's `PWD` does, running both
    // subprocesses from the same directory keeps this branch structurally parallel and immune to
    // any such quirk.
    let mut cmd = Command::new("powershell.exe");
    remove_inherited_terminal_identity(&mut cmd, inherited);
    crate::process::no_window(&mut cmd);
    cmd.current_dir(cwd);
    cmd.arg("-NoProfile").arg("-Command").arg(
        "[System.Environment]::GetEnvironmentVariables().GetEnumerator() | ForEach-Object { \
         [Console]::Out.Write(\"$($_.Key)=$($_.Value)`0\") }",
    );
    cmd
}

#[cfg(windows)]
fn attempt_env(path: &Path, cwd: &Path, timeout: Duration) -> RunOutcome {
    run_bounded(
        powershell_attempt_command(path, cwd, std::env::vars_os()),
        timeout,
    )
}

/// The Windows include shell, with `inherited`'s terminal identity removed (contract §3). Built on
/// every OS so the removal is testable here; only Windows runs it.
#[cfg(any(windows, test))]
fn powershell_attempt_command(
    path: &Path,
    cwd: &Path,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
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
    remove_inherited_terminal_identity(&mut cmd, inherited);
    crate::process::no_window(&mut cmd);
    cmd.current_dir(cwd);
    cmd.arg("-NoProfile").arg("-Command").arg(script);
    cmd
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
    inner: Mutex<FakeResolverState>,
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
            inner: Mutex::new(FakeResolverState {
                calls: Vec::new(),
                answer: Some((vars, outcome)),
            }),
        }
    }

    /// Behind a `Mutex` rather than a `RefCell` so the fake is `Sync` and can stand in for the
    /// client's `Arc<dyn EnvIncludeResolver + Send + Sync>` capability (spec 035).
    fn state(&self) -> MutexGuard<'_, FakeResolverState> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Every `(path, cwd, timeout)` this resolver was asked to source, in order (test assertions).
    pub fn calls(&self) -> Vec<(PathBuf, PathBuf, Duration)> {
        self.state().calls.clone()
    }
}

impl EnvIncludeResolver for FakeEnvIncludeResolver {
    fn resolve(
        &self,
        path: &Path,
        cwd: &Path,
        timeout: Duration,
    ) -> (Vec<(String, String)>, EnvIncludeOutcome) {
        let mut state = self.state();
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

// ---------------------------------------------------------------------------
// The start script (#1533): a streaming variant of the include engine.
// ---------------------------------------------------------------------------

/// How a script run ended (#1533, contract startup-run S2 step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptEnd {
    /// The shell exited with this status.
    Exited(i32),
    /// A signal ended it (Unix).
    Signal(i32),
    /// It was still running at the timeout; its whole process tree was killed.
    TimedOut,
    /// The shell could not be started, or the run could not be set up.
    SpawnFailed(String),
}

/// What [`run_and_diff`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRun {
    pub end: ScriptEnd,
    /// What the script exported: variables added or changed against the environment the script
    /// started with. Empty unless it ended `Exited(0)`; a variable it unset is not reported.
    pub exports: Vec<(OsString, OsString)>,
}

/// File names, beside the working directory, that carry the two `env -0` dumps so that stdout and
/// stderr stay purely the script's output.
const DUMP_FILE: &str = "startup-env.dump";
const BEFORE_FILE: &str = "startup-env.before";

/// Whether `name` can be assigned by a shell as a variable.
fn is_shell_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse a NUL-delimited dump keeping the bytes of names and values as they are.
fn parse_env_dump_os(bytes: &[u8]) -> HashMap<OsString, OsString> {
    bytes
        .split(|&b| b == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let at = entry.iter().position(|&b| b == b'=')?;
            Some((os_from_bytes(&entry[..at]), os_from_bytes(&entry[at + 1..])))
        })
        .collect()
}

#[cfg(unix)]
fn os_from_bytes(bytes: &[u8]) -> OsString {
    std::os::unix::ffi::OsStringExt::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_from_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Hands whole lines to a callback, per stream, as the bytes arrive; the unfinished line of a
/// stream is handed over by [`Self::finish`].
struct LineSplitter<F: FnMut(Stream, &str)> {
    pending: [Vec<u8>; 2],
    on_line: F,
}

impl<F: FnMut(Stream, &str)> LineSplitter<F> {
    fn new(on_line: F) -> Self {
        Self {
            pending: [Vec::new(), Vec::new()],
            on_line,
        }
    }

    fn slot(stream: Stream) -> usize {
        match stream {
            Stream::Stdout => 0,
            Stream::Stderr => 1,
        }
    }

    fn push(&mut self, stream: Stream, chunk: &[u8]) {
        let slot = Self::slot(stream);
        self.pending[slot].extend_from_slice(chunk);
        while let Some(at) = self.pending[slot].iter().position(|&b| b == b'\n') {
            let rest = self.pending[slot].split_off(at + 1);
            let mut line = std::mem::replace(&mut self.pending[slot], rest);
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            (self.on_line)(stream, &String::from_utf8_lossy(&line));
        }
    }

    fn finish(mut self) {
        for (slot, stream) in [(0, Stream::Stdout), (1, Stream::Stderr)] {
            if !self.pending[slot].is_empty() {
                let line = std::mem::take(&mut self.pending[slot]);
                (self.on_line)(stream, &String::from_utf8_lossy(&line));
            }
        }
    }
}

/// The POSIX wrapper: turns each `extra_env` pair into an unexported shell variable (values reach
/// the shell as data in its environment and are assigned without evaluation), dumps the
/// environment, sources the script, and dumps it again from an `EXIT` trap so a script that calls
/// `exit` is still diffed. `$1` is the script, `$2` the after dump, `$3` the before dump.
#[cfg(not(windows))]
const POSIX_WRAPPER: &str = r#"_micold_script=$1; _micold_dump=$2; _micold_before=$3; set --
_micold_i=0
while :; do
  eval "_micold_n=\${MICOLD_XN$_micold_i-}"
  [ -n "$_micold_n" ] || break
  eval "_micold_v=\${MICOLD_XV$_micold_i-}"
  eval "$_micold_n=\$_micold_v"
  eval "unset MICOLD_XN$_micold_i MICOLD_XV$_micold_i"
  _micold_i=$((_micold_i + 1))
done
unset _micold_i _micold_n _micold_v
env -0 >"$_micold_before"
trap '_micold_status=$?; env -0 >"$_micold_dump"; exit "$_micold_status"' EXIT
. "$_micold_script""#;

#[cfg(not(windows))]
fn script_command(
    script: &Path,
    shell: &Path,
    dump: &Path,
    before: &Path,
    extra_env: &[(String, String)],
) -> Command {
    let mut cmd = Command::new(shell);
    remove_inherited_terminal_identity(&mut cmd, std::env::vars_os());
    cmd.arg("-c")
        .arg(POSIX_WRAPPER)
        .arg("sh")
        .arg(script)
        .arg(dump)
        .arg(before);
    for (i, (name, value)) in extra_env.iter().enumerate() {
        cmd.env(format!("MICOLD_XN{i}"), name)
            .env(format!("MICOLD_XV{i}"), value);
    }
    cmd
}

/// The PowerShell wrapper: the pairs become PowerShell variables (not environment variables), the
/// environment is written before and after dot-sourcing the script, and the script's exit status
/// is the run's.
#[cfg(windows)]
const POWERSHELL_WRAPPER: &str = r#"
$i = 0
while ($true) {
  $n = [Environment]::GetEnvironmentVariable("MICOLD_XN$i")
  if (-not $n) { break }
  Set-Variable -Name $n -Value ([Environment]::GetEnvironmentVariable("MICOLD_XV$i"))
  Remove-Item "Env:MICOLD_XN$i", "Env:MICOLD_XV$i"
  $i++
}
function Dump($path) {
  $text = ([Environment]::GetEnvironmentVariables().GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)`0" }) -join ''
  [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding($false)))
}
Dump $env:MICOLD_RUN_BEFORE
$global:LASTEXITCODE = 0
try { . $env:MICOLD_RUN_SCRIPT } catch { [Console]::Error.WriteLine($_.Exception.Message); Dump $env:MICOLD_RUN_DUMP; exit 1 }
$status = if ($LASTEXITCODE) { $LASTEXITCODE } else { 0 }
Dump $env:MICOLD_RUN_DUMP
exit $status
"#;

#[cfg(windows)]
fn script_command(
    script: &Path,
    shell: &Path,
    dump: &Path,
    before: &Path,
    extra_env: &[(String, String)],
) -> Command {
    let mut cmd = Command::new(shell);
    remove_inherited_terminal_identity(&mut cmd, std::env::vars_os());
    crate::process::no_window(&mut cmd);
    cmd.arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(POWERSHELL_WRAPPER)
        .env("MICOLD_RUN_SCRIPT", script)
        .env("MICOLD_RUN_DUMP", dump)
        .env("MICOLD_RUN_BEFORE", before);
    for (i, (name, value)) in extra_env.iter().enumerate() {
        cmd.env(format!("MICOLD_XN{i}"), name)
            .env(format!("MICOLD_XV{i}"), value);
    }
    cmd
}

/// An empty file only its owner can use, for a dump to be written into.
fn private_file(path: &Path) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path).map(drop)
}

/// Run `script` in `shell` from `cwd`, bounded by `timeout`, hand every line it prints to
/// `on_line` as it prints it, and report what it exported (#1533, contract startup-run S2).
///
/// The script is sourced, so its `export`s land in the shell's environment, which is dumped to a
/// file beside `cwd` before and after and diffed. `extra_env` pairs are visible to the script as
/// plain, unexported variables: they reach the diff only if the script exports them, and their
/// values are never evaluated (a name that is not a valid shell name is skipped). `timeout` ends
/// the whole process tree; a child that keeps the output open does not extend the run.
///
/// Unlike [`resolve`], this does not run `bash -i`, and leaves stdout and stderr to the script alone.
pub fn run_and_diff(
    script: &Path,
    shell: &Path,
    cwd: &Path,
    timeout: Duration,
    extra_env: &[(String, String)],
    on_line: impl FnMut(Stream, &str),
) -> ScriptRun {
    let failed = |message: String| ScriptRun {
        end: ScriptEnd::SpawnFailed(message),
        exports: Vec::new(),
    };
    let extra: Vec<(String, String)> = extra_env
        .iter()
        .filter(|(name, _)| is_shell_name(name))
        .cloned()
        .collect();
    let script = match std::path::absolute(script) {
        Ok(path) => path,
        Err(e) => return failed(e.to_string()),
    };
    let dump = cwd.join(DUMP_FILE);
    let before = cwd.join(BEFORE_FILE);
    for file in [&dump, &before] {
        if let Err(e) = private_file(file) {
            return failed(format!("{}: {e}", file.display()));
        }
    }
    let mut cmd = script_command(&script, shell, &dump, &before, &extra);
    cmd.current_dir(cwd);
    let child = match StreamingChild::spawn(cmd) {
        Ok(child) => child,
        Err(e) => {
            let _ = std::fs::remove_file(&dump);
            let _ = std::fs::remove_file(&before);
            return failed(format!("{}: {e}", shell.display()));
        }
    };

    // The watchdog: `done` is dropped when the run ends, which wakes it without a timeout.
    let terminator = child.terminator();
    let fired = Arc::new(AtomicBool::new(false));
    let (done, stopped) = std::sync::mpsc::channel::<()>();
    let watchdog = {
        let fired = Arc::clone(&fired);
        std::thread::spawn(move || {
            if matches!(
                stopped.recv_timeout(timeout),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ) {
                fired.store(true, Ordering::SeqCst);
                terminator.terminate();
            }
        })
    };
    let mut lines = LineSplitter::new(on_line);
    let kind = child.run(|stream, chunk| lines.push(stream, chunk));
    lines.finish();
    drop(done);
    let _ = watchdog.join();

    let end = if fired.load(Ordering::SeqCst) {
        ScriptEnd::TimedOut
    } else {
        match kind {
            ExitKind::Code(code) => ScriptEnd::Exited(code),
            ExitKind::Signal(signal) => ScriptEnd::Signal(signal),
            ExitKind::WaitFailed(message) => ScriptEnd::SpawnFailed(message),
        }
    };
    let exports = if end == ScriptEnd::Exited(0) {
        let base = parse_env_dump_os(&std::fs::read(&before).unwrap_or_default());
        let after = parse_env_dump_os(&std::fs::read(&dump).unwrap_or_default());
        let mut exports: Vec<(OsString, OsString)> = after
            .into_iter()
            .filter(|(k, v)| base.get(k) != Some(v))
            .collect();
        exports.sort();
        exports
    } else {
        Vec::new()
    };
    let _ = std::fs::remove_file(&dump);
    let _ = std::fs::remove_file(&before);
    ScriptRun { end, exports }
}

#[cfg(test)]
mod terminal_identity_tests {
    use super::*;

    /// The keys contract session-terminal-identity §1 matches whatever their value.
    const IDENTITY_KEYS: [&str; 9] = [
        "FORCE_HYPERLINK",
        "TERM_PROGRAM",
        "TERM_PROGRAM_VERSION",
        "VTE_VERSION",
        "WT_SESSION",
        "WT_PROFILE_ID",
        "TERMINAL_EMULATOR",
        "KONSOLE_VERSION",
        "DOMTERM",
    ];

    #[test]
    fn each_identity_key_is_matched_whatever_its_value() {
        for key in IDENTITY_KEYS {
            for value in ["", "1", "WezTerm", "truecolor"] {
                assert!(
                    is_inherited_terminal_identity(key, value, false),
                    "{key}={value:?} names another terminal, so a session must not inherit it"
                );
            }
        }
        assert!(
            !is_inherited_terminal_identity("HOME", "/h", false),
            "a key outside the table is never matched"
        );
    }

    #[test]
    fn colorterm_is_matched_unless_it_names_a_colour_depth() {
        for value in ["truecolor", "24bit", "TrueColor", "24BIT"] {
            assert!(
                !is_inherited_terminal_identity("COLORTERM", value, false),
                "COLORTERM={value} describes colour depth, which a session keeps"
            );
        }
        for value in ["gnome-terminal", "rxvt-xpm", "yes", ""] {
            assert!(
                is_inherited_terminal_identity("COLORTERM", value, false),
                "COLORTERM={value:?} names a terminal, so a session must not inherit it"
            );
        }
    }

    #[test]
    fn keys_compare_exactly_unless_the_platform_folds_their_case() {
        assert!(
            !is_inherited_terminal_identity("term_program", "WezTerm", false),
            "on a case-sensitive platform `term_program` is another variable"
        );
        assert!(
            is_inherited_terminal_identity("term_program", "WezTerm", true),
            "on a case-insensitive platform `term_program` is `TERM_PROGRAM`"
        );
        assert!(
            is_inherited_terminal_identity("ColorTerm", "gnome-terminal", true),
            "the COLORTERM rule applies whatever the key's case on a case-insensitive platform"
        );
        assert!(
            !is_inherited_terminal_identity("ColorTerm", "truecolor", true),
            "and still keeps a colour depth"
        );
    }

    /// A fixed inherited environment, so the test does not depend on the runner's own.
    fn inherited() -> Vec<(OsString, OsString)> {
        [
            ("TERM_PROGRAM", "WezTerm"),
            ("FORCE_HYPERLINK", "1"),
            ("COLORTERM", "truecolor"),
            ("HOME", "/h"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
    }

    /// Every variable `cmd` was told to change, as `(key, None)` for a removal.
    fn env_changes(cmd: &Command) -> Vec<(String, Option<String>)> {
        let mut changes: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        changes.sort();
        changes
    }

    #[test]
    fn each_include_shell_removes_exactly_the_inherited_identity() {
        let removed = vec![
            ("FORCE_HYPERLINK".to_string(), None),
            ("TERM_PROGRAM".to_string(), None),
        ];
        let cwd = Path::new(".");
        // The bash builder only exists where it runs; the PowerShell ones are built everywhere.
        #[cfg(not(windows))]
        let bash = Some((
            "bash",
            bash_command(Path::new("/dev/null"), cwd, inherited()),
        ));
        #[cfg(windows)]
        let bash = None;
        let builders = [
            (
                "powershell baseline",
                powershell_baseline_command(cwd, inherited()),
            ),
            (
                "powershell attempt",
                powershell_attempt_command(Path::new("x.ps1"), cwd, inherited()),
            ),
        ]
        .into_iter()
        .chain(bash);
        for (name, cmd) in builders {
            assert_eq!(
                env_changes(&cmd),
                removed,
                "the {name} include shell removes the identity variables it inherits, and only \
                 those, so a script that sets one is seen to add it (contract §3)"
            );
        }
    }
}

#[cfg(test)]
mod run_and_diff_tests {
    use super::*;
    use std::time::Instant;

    fn shell() -> PathBuf {
        PathBuf::from(if cfg!(windows) {
            "powershell.exe"
        } else {
            "/bin/sh"
        })
    }

    fn run(
        body: &str,
        timeout: Duration,
        extra: &[(String, String)],
    ) -> (ScriptRun, Vec<(Stream, String)>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let script = dir
            .path()
            .join(if cfg!(windows) { "s.ps1" } else { "s.sh" });
        std::fs::write(&script, body).unwrap();
        let mut seen = Vec::new();
        let result = run_and_diff(&script, &shell(), dir.path(), timeout, extra, |s, l| {
            seen.push((s, l.to_string()))
        });
        (result, seen, dir)
    }

    fn export_of(run: &ScriptRun, name: &str) -> Option<String> {
        run.exports
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.to_string_lossy().into_owned())
    }

    #[cfg(windows)]
    #[test]
    fn a_powershell_script_s_env_assignment_is_captured_and_the_timeout_ends_the_job() {
        let (result, _, _dir) = run("$env:FOO = 'bar'", Duration::from_secs(30), &[]);
        assert_eq!(result.end, ScriptEnd::Exited(0));
        assert_eq!(export_of(&result, "FOO").as_deref(), Some("bar"));
        let started = Instant::now();
        let (result, _, _dir) = run("Start-Sleep -Seconds 30", Duration::from_secs(1), &[]);
        assert_eq!(result.end, ScriptEnd::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[cfg(not(windows))]
    mod posix {
        use super::*;

        fn pair(n: &str, v: &str) -> (String, String) {
            (n.to_string(), v.to_string())
        }

        #[test]
        fn an_explicit_export_is_captured_and_an_unexported_assignment_is_not() {
            let (r, _, _d) = run("export FOO=bar\nHIDDEN=1\n", Duration::from_secs(30), &[]);
            assert_eq!(r.end, ScriptEnd::Exited(0));
            assert_eq!(export_of(&r, "FOO").as_deref(), Some("bar"));
            assert_eq!(export_of(&r, "HIDDEN"), None);
        }

        #[test]
        fn a_value_with_a_newline_is_captured_intact() {
            let (r, _, _d) = run("export MULTI=\"one\ntwo\"\n", Duration::from_secs(30), &[]);
            assert_eq!(export_of(&r, "MULTI").as_deref(), Some("one\ntwo"));
        }

        #[test]
        fn a_script_that_calls_exit_is_still_diffed_and_its_status_reported() {
            let (r, _, _d) = run("export A=1\nexit 3\n", Duration::from_secs(30), &[]);
            assert_eq!(r.end, ScriptEnd::Exited(3));
            assert!(r.exports.is_empty(), "a failed run exports nothing");
        }

        #[test]
        fn extra_env_is_readable_by_the_script_but_not_exported_unless_it_says_so() {
            let extra = [pair("TOKEN", "x"), pair("OTHER", "y")];
            let (r, seen, _d) = run(
                "echo \"token=$TOKEN\"\nexport OTHER\n",
                Duration::from_secs(30),
                &extra,
            );
            assert_eq!(r.end, ScriptEnd::Exited(0));
            assert!(seen.contains(&(Stream::Stdout, "token=x".to_string())));
            assert_eq!(export_of(&r, "TOKEN"), None, "read-only use leaks nothing");
            assert_eq!(export_of(&r, "OTHER").as_deref(), Some("y"));
            assert_eq!(export_of(&r, "MICOLD_XN0"), None);
        }

        #[test]
        fn extra_env_values_are_literal() {
            let value = "$(touch pwned) `id` \"q\" 'q' # not a comment\n\nlast";
            let (r, _, d) = run("export V\n", Duration::from_secs(30), &[pair("V", value)]);
            assert_eq!(export_of(&r, "V").as_deref(), Some(value));
            assert!(!d.path().join("pwned").exists());
        }

        #[test]
        fn a_name_that_is_not_a_shell_name_is_skipped() {
            let extra = [pair("A;touch pwned", "x"), pair("OK", "y")];
            let (r, _, d) = run("export OK\n", Duration::from_secs(30), &extra);
            assert_eq!(export_of(&r, "OK").as_deref(), Some("y"));
            assert!(!d.path().join("pwned").exists());
        }

        #[test]
        fn the_timeout_ends_the_process_group_and_a_backgrounded_child_does_not_extend_it() {
            let started = Instant::now();
            let (r, _, _d) = run("sleep 30 &\nsleep 30\n", Duration::from_secs(1), &[]);
            assert_eq!(r.end, ScriptEnd::TimedOut);
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "took {:?}",
                started.elapsed()
            );
            let started = Instant::now();
            let (r, _, _d) = run("sleep 30 &\nexport A=1\n", Duration::from_secs(20), &[]);
            assert_eq!(r.end, ScriptEnd::Exited(0));
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn output_is_streamed_line_by_line_with_its_stream() {
            let (r, seen, _d) = run(
                "echo one\necho two >&2\nprintf partial\n",
                Duration::from_secs(30),
                &[],
            );
            assert_eq!(r.end, ScriptEnd::Exited(0));
            assert!(seen.contains(&(Stream::Stdout, "one".to_string())));
            assert!(seen.contains(&(Stream::Stderr, "two".to_string())));
            assert!(seen.contains(&(Stream::Stdout, "partial".to_string())));
            assert!(seen.iter().all(|(_, l)| !l.contains("MICOLD")));
        }

        #[test]
        fn the_dump_files_are_removed_and_a_missing_shell_is_reported() {
            let (_, _, d) = run("true\n", Duration::from_secs(30), &[]);
            assert!(!d.path().join(DUMP_FILE).exists());
            assert!(!d.path().join(BEFORE_FILE).exists());
            let dir = tempfile::tempdir().unwrap();
            let r = run_and_diff(
                Path::new("/nonexistent"),
                Path::new("/no/such/shell"),
                dir.path(),
                Duration::from_secs(5),
                &[],
                |_, _| {},
            );
            assert!(matches!(r.end, ScriptEnd::SpawnFailed(_)));
        }

        #[test]
        fn a_script_that_does_not_exist_ends_nonzero() {
            let dir = tempfile::tempdir().unwrap();
            let r = run_and_diff(
                &dir.path().join("absent.sh"),
                &shell(),
                dir.path(),
                Duration::from_secs(30),
                &[],
                |_, _| {},
            );
            assert!(
                matches!(r.end, ScriptEnd::Exited(c) if c != 0),
                "{:?}",
                r.end
            );
        }
    }
}

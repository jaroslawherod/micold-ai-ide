# Phase 0 Research: Environment-Include Script

All Technical-Context unknowns are resolved below. Evidence was gathered by reading the current
implementation (`src/main.rs`, `src/app.rs`, `src/settings.rs`, `src/terminal.rs`,
`src/ui/terminal.rs`, `src/ui/settings_form.rs`, `docs/user-guide/settings.md`) and prior feature
plans (006, 010) for established patterns.

## R1 — Sourcing mechanism: `source` runs un-subshelled, an `EXIT` trap captures the diagnostic

**Decision** (revised during implementation — see "Implementation note" below): On Linux/macOS,
resolve in three steps:

1. **Existence check (Rust-side, no subprocess)**: if the configured path doesn't exist (or is
   blank/disabled — spec Edge Cases), short-circuit to `EnvIncludeOutcome::MissingScript` (or
   `Disabled`) without spawning anything. This gives a clean, testable category distinction
   instead of parsing bash's own English error text.
2. **Baseline capture**: spawn the *same wrapper* used for the attempt (step 3), sourcing
   `/dev/null` (a guaranteed-empty no-op) instead of the real path, and parse its stdout. Reusing
   the identical wrapper — rather than a bare `bash -c 'env -0'` — matters: bash tail-call-
   optimizes a `-c` script whose last command is a single simple external command (`execve()`-
   replacing itself, no fork) but cannot when anything follows, and the two paths report a
   different `SHLVL` for reasons internal to bash — a spurious diff on *every* resolution if
   baseline and attempt aren't structurally identical.
3. **Attempt capture**, a wrapper passed to `bash --noprofile --norc -c '<wrapper>' -- <path>`:
   ```bash
   diag_file=$(mktemp); trap 'status=$?; printf "%s" "$(cat "$diag_file" 2>/dev/null)" >&2; rm -f "$diag_file"; env -0; exit "$status"' EXIT; source "$1" >"$diag_file" 2>&1
   ```
   `source "$1"` runs directly in this shell (NOT inside a `$(...)` subshell) so any `export`s it
   makes land in the environment `env -0` dumps at the end. An `EXIT` trap set beforehand — which
   fires on any shell termination, an explicit `exit` inside the sourced script or falling off the
   end alike — does the diagnostic printing, cleanup, and env dump, and re-asserts the original
   `$status` as the final exit code.

**Implementation note (supersedes the original decision below)**: the first implementation
captured the sourced script's own output via `out=$(source "$1" 2>&1)` — command substitution —
on the theory that it isolates the diagnostic capture from the env dump with no temp file needed.
This was wrong: command substitution runs in a subshell, so every `export` the sourced script made
was silently discarded before `env -0` ran, and — separately — a sourced script calling `exit`
directly killed the whole subshell harmlessly but (once the subshell bug was fixed by removing it)
would otherwise kill the *entire* wrapper before reaching `env -0`. Both defects were caught by
`tests/env_include_resolve.rs`'s real-subprocess tests failing, not by inspection. The temp-file +
`EXIT`-trap design fixes both: `source` is never subshelled (exports persist), and the trap
guarantees `env -0` still runs and the original exit code still surfaces even when the sourced
script calls `exit` itself. One tradeoff: the trap does not run if the process is later SIGKILLed
on timeout (SIGKILL is unblockable) — a `TimedOut` outcome's diagnostic may therefore be empty
even if the script had already printed something before hanging; guaranteed cleanup (R2) was
chosen over best-effort diagnostics for that one narrow case.

**Rationale**: This gives exactly the three failure categories FR-013 asks for a UI to
distinguish (missing / non-zero exit / timeout) without ever guessing from free-text output, and
delivers the FR-013 diagnostic (the script's own captured stdout+stderr) on a separate stream from
the env dump — never mixed into the parsed variable list.

**Alternatives considered**: Parsing the script's text (rejected outright — the spec's FR-005
explicitly requires actually sourcing it, since `PATH`-mutating loops and version-manager
conditionals can't be statically evaluated). The original command-substitution approach (rejected
per the implementation note above — it silently discards every exported variable).

## R2 — Timeout enforcement: poll `try_wait`, kill the whole process group via `libc`

**Decision** (revised during implementation — see "Implementation note" below): Spawn the attempt
subprocess with `std::process::Command::spawn()` (not the blocking `.output()`) in its own process
group (`CommandExt::process_group(0)`, Unix), then loop calling `child.try_wait()` with a short
sleep (~20ms) between polls until either it exits or the configured timeout elapses. Whether the
process exited on its own or timed out, the ENTIRE process group is then killed via a direct
`libc::kill(-pid, SIGKILL)` syscall before reading its piped stdout/stderr, and only then are the
pipes read to completion.

**Implementation note (supersedes the original decision below)**: the first implementation called
`child.kill()` (signals only the single top-level process) and read the piped stderr *before*
killing on timeout. Both were bugs, caught by a real hang during test development (not by
inspection): a sourced script running under `source` inside `$(...)` forks a subshell (bash's own
command-substitution mechanics) which in turn runs whatever the script does — e.g. `sleep 999` —
as a further child; killing only the top-level `bash` left that grandchild running, and since it
inherited the piped stdout/stderr file descriptors, reading those pipes to EOF blocked forever
(the pipe only closes once *every* process holding its write end exits). The fix needed two parts:
(a) spawn in a new process group so the whole group can be killed together, not just the direct
child, and (b) kill before reading, not after. Killing the group was tried first via spawning the
`kill(1)` *binary* as a subprocess (`Command::new("kill").arg("-KILL").arg(format!("-{pid}"))`) to
avoid adding a dependency — this reported success (`ExitStatus` code 0) without the target group
actually dying, reproducibly, in this development sandbox. Switching to a direct in-process
`libc::kill()` syscall (no subprocess spawn at all) resolved it. `libc` was added as a small,
Unix-only (`[target.'cfg(unix)'.dependencies]`), already-transitively-present dependency —
correcting the original decision's claim that dependency count would stay at zero.

**Rationale**: `std::process` + `libc::kill()` is the minimal correct primitive for "kill a whole
process tree, not just its root" — no polling-crate or job-management library needed. This is the
same "spawn once, bound the wait" shape as the rest of the app's process-facing code
(`portable-pty`'s PTY handles), extended with process-group cleanup because a *sourced rc file*,
unlike a PTY-hosted interactive process, may legitimately background further processes (an agent
daemon, a version-manager helper) that must not outlive a killed/timed-out attempt.

**Alternatives considered**: A crate like `wait-timeout` — still unnecessary for the poll loop
itself. `tokio::process` with `tokio::time::timeout` — still rejected, `tokio` isn't compiled
under `--no-default-features`. Shelling out to `kill(1)` instead of `libc` — tried first,
rejected per the implementation note above (unreliable in this environment). SIGTERM-then-SIGKILL
(a grace period, allowing the `EXIT` trap in R1 to run and produce a diagnostic even on timeout) —
rejected as added complexity or a script that ignores SIGTERM; plain SIGKILL was chosen for
guaranteed cleanup, accepting a possibly-empty diagnostic on `TimedOut` as a known, narrow
tradeoff (documented in R1's implementation note).

## R3 — Diffing: additions/changes only, pure and separately testable

**Decision**: A pure function takes two already-parsed `HashMap<String, String>`s (baseline,
attempt) and returns a `Vec<(String, String)>` of every key that is new in `attempt` or whose
value differs from `baseline` — keys present in `baseline` but *absent* from `attempt` (a script
that `unset`s something) are not reported, since the merge target
(`cmd.env(k, v)` on `portable_pty::CommandBuilder`) is itself additive/overwrite-only and has no
"unset" operation to apply anyway.

**Rationale**: Matches the feature description's framing ("merge into the same `env:
Vec<(String, String)>`") — the mechanism this feature feeds into cannot express removal, so
diffing for removal would produce information the rest of the pipeline can't use. Keeping this as
a pure `HashMap, HashMap -> Vec` function (no subprocess, no I/O) makes it trivially unit-testable
under `--no-default-features` with hand-built maps, independent of R1/R2's process-spawning
machinery.

**Alternatives considered**: Also tracking removed keys for some future "unset" merge semantics —
rejected as speculative; nothing in the spec calls for it, and `Vec<(String,String)>` has no slot
to carry a removal today.

## R4 — Module placement: pure/core, not gui-gated, tested with real subprocesses

**Decision**: New module `src/env_include.rs`, added to `src/lib.rs` unconditionally (like
`settings.rs`, unlike `ui/terminal.rs`). It holds:
- `EnvIncludeOutcome` enum (`Disabled`, `Success`, `MissingScript`, `NonZeroExit { code: i32 }`,
  `TimedOut`), each failure variant carrying the captured diagnostic text.
- Pure: `parse_env_dump(bytes: &[u8]) -> HashMap<String, String>` (NUL-delimited `KEY=VALUE`
  parsing — see R6 for the Windows-side equivalent) and `diff_env(...)` (R3).
- Impure: `resolve(path: &Path, timeout: Duration) -> (Vec<(String, String)>, EnvIncludeOutcome)`
  orchestrating R1/R2/R3.

**Rationale**: `std::process::Command` needs no `iced`/`portable-pty`, so this can compile and be
tested under `cargo test --no-default-features` (the CI-matching `mise run test`), exactly like
`terminal.rs`'s `default_shell_command` — a pure/std-only platform-decision function that lives in
the core despite the actual PTY spawning it feeds being gui-side. The impure `resolve()` gets a
real (non-mocked) integration test that writes an actual disposable script via `tempfile` and
spawns real `bash`/`powershell.exe` — required by FR-005's "actually sourcing it, not parsing
text" mandate, and precedented by how `shell_command.rs` already asserts real per-platform branch
behavior rather than simulating `cfg!`.

**Alternatives considered**: Putting this gui-side in `ui/terminal.rs` next to `spawn_shell_pty` —
rejected: it would make the engine untestable under `--no-default-features`, and it has nothing to
do with PTYs (no `portable-pty` dependency needed at all).

## R5 — When resolution runs: synchronous, bounded by the configured timeout, at two triggers

**Decision**: Resolve once, synchronously, inside `boot()` — after settings load, before the
window's initial state is constructed — and store the result on `App`. Refresh it (also
synchronously, in the `update()` handler) at exactly the two triggers the spec's clarifications
name: a `Message::SettingsSaved` that touched the enabled/path/timeout fields, and
`Message::TerminalRestartRequested` (any session). No async task, no background thread beyond
R2's own poll loop.

**Rationale**: `boot()` already runs a synchronous, unbounded external-process call today —
`discover_worktrees(&repo)` shells out to `git worktree list --porcelain` via `GitCli` — and the
same function is called again synchronously from inside `update()` at three more call sites
(project switch, worktree creation). This feature's resolution is strictly *more* bounded than
that existing precedent (it has an explicit, user-configurable timeout; `discover_worktrees` has
none), so doing it synchronously does not introduce a new architectural pattern — it tightens an
existing one. SC-003's "never delayed by more than the configured timeout" is exactly what R2's
poll-and-kill loop guarantees; the timeout setting (FR-003/FR-004, default 10s) exists precisely to
put a known, user-adjustable ceiling on this already-accepted class of blocking call.

**Alternatives considered**: An `iced::Task`-driven async resolution so the window appears
immediately and the first session's launch races the resolution — rejected: it reopens exactly
the concurrency/race question the spec's clarification phase explicitly deferred as low-impact and
implementation-level, and it would need `tokio` (a `gui`-only dependency) reachable from the pure
engine, contradicting R4's core-module placement. Resolving on *every* individual session/terminal
launch instead of caching — explicitly rejected by the spec (FR-007) for the same reason: paying
rc-sourcing cost (and the associated timeout risk) on every launch instead of at most twice per
run.

## R6 — Windows: `powershell.exe` (built-in 5.1), profile default, NUL-delimited dump for parser reuse

**Decision**: On Windows, source via `powershell.exe` (Windows PowerShell 5.1, which ships on
every Windows install — not `pwsh.exe`/PowerShell 7, which is a separate optional install) dot-
sourcing the configured path: `. "<path>"`. The default path (R7) is the current-user,
current-host profile: `%USERPROFILE%\Documents\WindowsPowerShell\profile.ps1`. The env dump uses
the same NUL-delimited `KEY=VALUE` wire format as bash's `env -0`, built manually since PowerShell
has no `-0` flag:
```powershell
[System.Environment]::GetEnvironmentVariables().GetEnumerator() |
  ForEach-Object { [System.Text.Encoding]::UTF8.GetBytes("$($_.Key)=$($_.Value)`0") }
```
so `parse_env_dump` (R4) is a single platform-independent function with no `#[cfg]` branches of
its own — only the two subprocess-invocation call sites (R1 vs. this) differ.

**Rationale**: Targeting `powershell.exe` rather than `pwsh.exe` means the feature works out of the
box on a stock Windows install with no additional software required, mirroring how the Unix
default (`~/.bashrc`) requires nothing beyond what ships on Linux/macOS. Reusing one wire format
for both platforms keeps the parser single and pure (R3/R4), rather than two parsers to maintain
and test.

**Alternatives considered**: Defaulting to `pwsh.exe`/PowerShell 7 — rejected: not guaranteed
present on a stock Windows install, which would make the *default* silently no-op for users who
never installed it, undermining FR-004's "auto-includes the default script with no user action."
A newline-delimited (not NUL-delimited) dump — rejected: environment variable values can
legitimately contain newlines on both platforms, which would corrupt parsing; NUL is the one byte
POSIX environ values can never contain, hence `env -0`'s own choice, reused here for
consistency.

## R7 — Default script path: one function per OS, mirroring `default_shell_command`

**Decision**: `default_env_include_path(home: Option<&Path>) -> PathBuf` in `src/settings.rs`,
argument-driven like `terminal.rs`'s `default_shell_command` (the impure `std::env::var`/
`directories` home-dir lookup happens once at the call site, not inside this function):
- Non-Windows: `home.join(".bashrc")`.
- Windows: `home.join("Documents").join("WindowsPowerShell").join("profile.ps1")`.

**Rationale**: Matches the existing pure/argument-driven split already used for
`default_shell_command` (research R3, feature 010) — testable on any host by passing an explicit
`home` value, with the actual home-directory resolution (`directories::UserDirs` or equivalent)
read once at the boot()/settings-load call site.

**Alternatives considered**: Hardcoding a shell-out to `echo ~` or reading `$HOME` directly inside
the function — rejected, breaks the argument-driven testability pattern this codebase already
established.

## R8 — Settings persistence: three flat fields, version bump as documentation

**Decision**: Extend `Settings`/`StoredSettings` (`src/settings.rs`) with three new flat fields —
`env_include_enabled: bool` (`#[serde(default = "default_env_include_enabled")]`, defaulting
`true`), `env_include_script_path: String` (`#[serde(default = "default_env_include_script_path_string")]`,
defaulting to R7's platform path stringified), `env_include_timeout_secs: u64`
(`#[serde(default = "default_env_include_timeout_secs")]`, defaulting `10`) — sibling fields next
to `theme`/`scrollback_lines`, not a nested sub-struct. Bump `SETTINGS_VERSION` `2 → 3` and extend
its doc comment, exactly as it was bumped `1 → 2` for `scrollback_lines` (the constant is
documentation only; nothing branches on it).

**Rationale**: Matches the established flat-sibling-field style over introducing a nested
`EnvIncludeSettings` struct purely for these three fields — `Settings` already holds `theme` and
`scrollback_lines` as flat, unrelated-by-type fields; FR-015's "grouped together" requirement is a
**Settings-modal UI** concern (R9, `settings_form.rs`'s field ordering/visual clustering), not a
data-shape requirement, so no new type is warranted for it (avoids introducing an abstraction the
task doesn't need, matching this repo's general anti-over-engineering stance).

**Alternatives considered**: A nested `env_include: EnvIncludeSettingsDoc` struct, serialized as a
sub-object — rejected as an unnecessary shape change for three scalar fields with no independent
lifecycle from the rest of `Settings`; the existing flat style already scales fine (this is its
third addition after `theme` and `scrollback_lines`).

## R9 — Settings UI: grouped section with iced's own widgets, no new shared component

**Decision**: `settings_form.rs` gains one grouped block — a `text("Environment include")`
sub-heading, an iced built-in `checkbox("Enabled", draft.env_include_enabled)` (styled via a new
`style::checkbox(r)` helper alongside the existing `style::input`/`style::filled`/
`style::outlined`), and two `text_input`s (path, timeout) — all inside the same `column` the
scrollback field already lives in, visually separated by spacing/order from the scrollback
section (satisfies FR-015's "grouped... visually distinct" without a new container primitive). A
read-only diagnostic block (the failure category + captured stdout/stderr, FR-013) renders below
the group only when the last resolution outcome was a failure.

**Rationale**: `settings_form.rs` already composes its one existing field directly from iced's
built-ins (`text_input`, `button`, `column`, `container`, `row`) styled via the shared `style::`
module, not via bespoke Material-wrapped structs per field — the one Material component it uses
is `Modal`, for the dialog shell itself, which is reused unchanged here. Following that exact
existing precedent for the three new fields means no one-off widget is forked and no shared
primitive is duplicated (Principle VIII targets *this codebase's own* shared component reuse,
not "wrap every iced built-in in a custom struct").

**Alternatives considered**: A new `EnvIncludeSection` Material component with its own builder API
— rejected as premature abstraction for a single call site with three fields; if a second Settings
section later needs the same grouped-fields-plus-status shape, promoting it then would be the
right time (Principle VIII's "when a needed UI element does not yet exist as a shared primitive,
create it" — this one call site does not yet establish a *pattern* worth sharing).

# Contract: the Windows host daemon endpoint

**Feature**: [../spec.md](../spec.md) | Normative for:

- `crates/micold-core/src/endpoint.rs`
- `crates/micold-core/src/spawn.rs`
- `crates/micold-core/src/process.rs` (new)
- `crates/micold-daemon/src/singleton.rs`
- `crates/micold-daemon/src/server.rs`
- `crates/micold-daemon/src/platform/windows.rs`

Covers FR-020 to FR-025. Each row names the test that proves it. Every test must run on the
`windows-latest` CI leg (FR-025). A test that runs on Linux only does not satisfy a row.

## E1. Resolution

| # | Guarantee | Test |
|---|---|---|
| E1.1 | `endpoint::resolve()` returns `Ok` on Windows. `socket_path` is `\\.\pipe\Micold.Daemon.<SID>`, where `<SID>` matches `^S-1-[0-9-]+$` and equals the current token's user SID. | `endpoint::tests::resolve_creates_a_usable_endpoint_pair` (the `#[cfg(windows)]` arm is rewritten; the current stub assertion is deleted) |
| E1.2 | `lock_path` is `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid`, and its parent exists after `resolve()`. | same |
| E1.3 | Two calls in one process return equal endpoints. | `endpoint::tests::resolve_is_stable` |

## E2. Access control (FR-021, SC-009)

| # | Guarantee | Test |
|---|---|---|
| E2.1 | The bound pipe's DACL is protected and holds exactly one ACCESS_ALLOWED ACE, whose SID is the owner's. | `micold-daemon/tests/windows_pipe_acl.rs` (`#[cfg(windows)]`): bind, `GetSecurityInfo(DACL)`, walk the ACEs |
| E2.2 | Remote clients are rejected (`PIPE_REJECT_REMOTE_CLIENTS`). | Asserted by code review of the `ListenerOptions` builder. The flag is set by `interprocess` itself; no network test. |
| E2.3 | A connection attempt by another account fails with access denied before the handshake. | Manual quickstart step M7. CI runners have one account, and creating a user needs admin, which the per-user posture avoids. E2.1 is the automated proxy. |

## E3. Singleton (FR-022)

| # | Guarantee | Test |
|---|---|---|
| E3.1 | While one daemon is bound, a second `singleton::acquire` returns `Acquisition::AlreadyRunning` and does not disturb the first. | `micold-daemon/tests/daemon_singleton.rs`: the two-starters case is un-gated from `#![cfg(unix)]`; the stale-socket and directory-mode cases stay `#[cfg(unix)]` |
| E3.2 | After the bound listener drops (crash equivalent), the next acquire returns `Bound`, with no stale-state cleanup required. | `daemon_singleton.rs::acquire_after_drop_rebinds` (new, all platforms) |
| E3.3 | The client's spawn-or-attach connects to an already-running daemon instead of spawning a second one. | `micold-daemon/tests/autospawn.rs`, un-gated (a real two-process test) |

## E4. Stop (FR-023)

| # | Guarantee | Test |
|---|---|---|
| E4.1 | The daemon writes `lock_path` after binding. It leaves the file in place when it stops: it has no clean exit, and a leftover record is ignored (E4.4). | `micold-daemon/tests/daemon_stop.rs::pid_record_lifecycle` (new file, all platforms) |
| E4.2 | `spawn::stop_running_daemon(&endpoint)` terminates a live daemon, and the endpoint stops accepting connections within 5 s. | `daemon_stop.rs::stop_running_daemon_ends_endpoint`. It spawns the real daemon binary and calls the same `spawn::stop_running_daemon` that `micold-client/src/shell/service_control.rs:57` (Restart service) calls. |
| E4.3 | `terminate_daemon(pid)` refuses (`ErrorKind::InvalidData`) when the pid's image is not `micold-daemon.exe`, and the process survives. | `spawn::tests::terminate_refuses_foreign_image` (spawns a harmless `cmd /c timeout` child it owns, then kills it itself) |
| E4.4 | A stale pid record (the pipe is not live) is ignored. `stop_running_daemon` returns "not running", not an error. | `spawn::tests::stale_pid_record_is_ignored` |

## E5. Session process trees

| # | Guarantee | Test |
|---|---|---|
| E5.1 | Killing a session terminates its shell and a grandchild started from it. | `supervisor::tests::kill_reaps_grandchild` (`#[cfg(windows)]` arm: shell runs `cmd /c start /b ping -t 127.0.0.1`, then asserts the grandchild pid has exited within 5 s) |
| E5.2 | `env_include` keeps its current timeout-kills-tree behaviour after the Job Object code moves to `micold-core::win_job`. | The existing `env_include` tests (unchanged) |

## E6. No console windows (FR-005, FR-024)

| # | Guarantee | Test |
|---|---|---|
| E6.1 | Release builds of `micold-ai-ide.exe` and `micold-daemon.exe` have PE subsystem `WINDOWS_GUI` (2); debug builds keep `WINDOWS_CUI` (3). | CI step: read the PE optional-header `Subsystem` field of the built release exes (PowerShell, about 10 lines) |
| E6.2 | Every `std::process::Command::new` / `tokio::process::Command::new` in `crates/*/src/**/*.rs` goes through `process::no_window` unless the call site is allowlisted. The allowlist is the detached daemon spawn (`spawn.rs`), which uses DETACHED_PROCESS, and `#[cfg(test)]` code. | `micold-core/tests/background_spawns_hide_console.rs` (source scan; runs on every OS) |
| E6.3 | `process::no_window` sets `CREATE_NO_WINDOW` (0x08000000) on Windows and is a no-op elsewhere. | `process::tests::no_window_sets_flag` (`#[cfg(windows)]`, via `CommandExt` round-trip behaviour: spawned `cmd /c exit 0` succeeds; flag constant pinned) |
| E6.4 | Once the console is gone, a fatal daemon startup error is still written to the daemon log. | `micold-daemon/tests/fatal_startup_is_logged.rs` |
| E6.5 | An installed client spawns its daemon without creating a `conhost.exe` child. | CI install smoke (R14), step 5 |

## E7. Running-app marker

| # | Guarantee | Test |
|---|---|---|
| E7.1 | `process::announce_running()` creates `Local\MicoldAIIDE`. A second call in another process succeeds (it is shared, not exclusive), and `OpenMutexW` sees it while either process holds it. | `process::tests::announce_running_is_visible` (`#[cfg(windows)]`) |
| E7.2 | The client's `main` calls it before anything else can fail. The daemon's does not (revised after A6: a daemon holding it cancelled a silent repair at the prompt, before setup's `StopDaemon` ran). | Code review. The binaries are GUI glue under the constitution's Principle I exception, and E7.1 covers the logic. |

## Un-gating the daemon suite (FR-025)

Today 26 of the 60 files in `crates/micold-daemon/tests/` are `#![cfg(unix)]` as a whole. Most are
end-to-end tests over the real endpoint and a real PTY, and they were gated only because the Windows
endpoint was a stub. Each file ends in one of three states, in this order of preference:

1. **Un-gated.** The blanket `#![cfg(unix)]` is removed and the file passes on `windows-latest`.
   This is **required** for the files that prove FR-020 to FR-023:
   - `autospawn.rs`
   - `daemon_singleton.rs` (partly; see E3.1)
   - `stream_view.rs`
   - `session_start.rs`
   - `session_isolation.rs`
   - `session_survival.rs`

   The new `daemon_stop.rs` and `windows_pipe_acl.rs` never carry a gate.
2. **Partly gated.** The file runs on Windows, and individual Unix-only cases carry `#[cfg(unix)]`
   with a one-line reason. Examples: socket-file modes, systemd socket activation, `SHELL`-override
   crash shells, uid checks.
3. **Still whole-file gated.** This is allowed only with a reason comment naming the Unix-specific
   dependency. Every such file is listed in `docs/development/windows-packaging.md`, section "Tests
   not run on Windows", so the gap stays visible.

A guard test, `micold-core/tests/daemon_tests_gate_with_reason.rs`, is a text scan. It fails when a
`#![cfg(unix)]` in `crates/micold-daemon/tests/*.rs` is not immediately preceded by a `// unix-only:`
reason line.

# Phase 0 Research: Daemon-Backed Session Persistence

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

All findings below were gathered by parallel research agents on 2026-07-20 and verified against
crate sources, upstream repositories, and — for R6 — empirical testing against a live `claude` CLI.
Every item states what was verified and what was not. **Unverified items are marked; they are not
to be treated as settled.**

---

## R0. Codebase baseline — two premises in the feature request are false

Before any design decision, two "existing constraints to honor" from the feature request were
checked against the code. Both are wrong, and the plan must not be built on them.

### R0.1 The `TerminalBackend` / `TerminalHandle` / `SessionRouter` seam does not carry production traffic

The request states this seam "already model[s] this process boundary and should be the basis for
the split rather than being replaced." In fact:

- `PtyTerminalBackend` (`src/ui/terminal.rs:337`) is marked `#[allow(dead_code)]` and hardcodes
  `spawn_pty(&spec, 10_000)`, ignoring the user's scrollback setting.
- Production calls the free function `spawn_pty` directly at `src/main.rs:533`, `:563`, `:940`,
  then uses inherent `RuntimeTerminal` methods.
- The reason is structural: the GUI needs `renderable()`, `scroll()`, `selection_*()`,
  `display_offset()`, `history_size()`, `key_term_mode()`, `mouse_report_bytes()` and
  `selectable_content()`. **None of these exist on `TerminalHandle`**, whose entire surface is
  `write_input` / `resize` / `kill`.
- `SessionRouter` has **zero** production callers; only `tests/pty_routing.rs` references it.

**Decision**: Design the client/daemon boundary fresh, informed by but not constrained to the
existing three-method trait. Delete `SessionRouter` (and migrate `tests/pty_routing.rs`) rather
than preserve a struct that models nothing real.

**Rationale**: The traits are a sketch that was never load-bearing. Nothing depends on their
current shape, which makes this cheap; pretending they are a designed seam would import a
three-method vocabulary that cannot express the actual protocol.

### R0.2 The test count is 259, not 63

Verified: `cargo test --no-default-features` → **259 passing tests across 43 integration files**
(294 with the `gui` feature; the delta is in-binary `#[cfg(test)]` modules). The feature request's
"63 existing headless integration tests" understates the migration surface by ~4×.

**Decision**: Treat test migration as a first-class workstream sized against 259 tests, not 63.

### R0.3 Selection and scroll are currently mutations of the shared `Term`

`selection_start/update/clear` (`src/ui/terminal.rs:198/209/217`) write `term.selection` directly,
and `selectable_content()` (`:236`) reads it back by walking `display_iter`. `scroll()` (`:115`)
calls `term.grid_mut().scroll_display()`.

The spec (FR-010, FR-018) assigns selection and viewport to the client. These cannot both hold.

**Decision**: Selection becomes a purely client-side model over the wire grid, and text extraction
("copy") becomes client-side too. The daemon's `Term.selection` is never used. Viewport offset is
client-owned; the daemon serves scrollback by absolute line range (see R3.5).

**Rationale**: Required by the spec's ownership split, and independently correct — selection is
per-window presentation state, so two windows viewing one project (after takeover) must not fight
over one shared selection.

### R0.4 Other baseline facts that shape the design

| Fact | Location | Consequence |
|---|---|---|
| `portable-pty` + `alacritty_terminal` are `dep:`-gated behind the `gui` feature | `Cargo.toml` | The daemon cannot link them today. A new feature split is required (see R7). |
| `JsonFileStore` has no file locking; `.tmp` + rename, no fsync | `src/store.rs:249-262` | Two processes writing `projects.json` silently clobber each other — this is the concrete hazard that FR-008's single-writer rule removes. |
| `MAX_RESTART_ATTEMPTS = 3` yields **two** actual restarts | `src/session.rs:153-166` | Off-by-one against the natural reading of FR-005. Preserve behaviour, but document it. |
| The restart counter has no time window; `mark_running()` resets it | `src/session.rs:142` | A slow crash loop (crash every hour) **never** trips the guard. Pre-existing bug; FR-005 makes it worse by moving it to an unattended context. Flagged for the plan. |
| Session titles: full `read_to_string` + full JSONL scan per active session, every 120 ms | `src/provider.rs:375`, `src/main.rs:754` | Cost grows with transcript length, on the UI thread. Moving to the daemon fixes the UI-thread problem; R6 replaces the polling entirely. |
| Worktree cwd convention `<project>/.claude/worktrees/<dir>` is hardcoded at 4 sites | `src/main.rs:318,527,962,971` | Centralise in the daemon. |
| Git is shelled out to the `git` binary; errors are `io::Error::other(stderr)` | `src/git.rs:54-70` | Preserves FR-034 (underlying diagnostic detail) naturally, provided the string crosses the RPC boundary intact. |
| Worktree deletion errors are discarded (`let _ = ...`) | `src/main.rs:783-784` | Violates FR-031 today. The move to RPC is the opportunity to fix it. |

---

## R1. IPC transport

### Decision: `interprocess` 2.4.2 with the `tokio` feature, using `GenericFilePath` names

```toml
interprocess = { version = "2.4.2", features = ["tokio"] }
```

Released 2026-04-19, `0BSD OR Apache-2.0`, MSRV 1.75. Passively maintained but shipping steadily.

**Verified by reading the 2.4.2 source** (see the methodology note at the end of this section):

- `os::windows::local_socket::ListenerOptionsExt::security_descriptor()` is implemented **on the
  portable `ListenerOptions`**, mirroring the Unix `mode()` extension. Takes an SDDL string via
  `SecurityDescriptor::deserialize()`.
- `create_instance.rs` sets `FILE_FLAG_FIRST_PIPE_INSTANCE` automatically on the first instance and
  `PIPE_REJECT_REMOTE_CLIENTS` by default.
- `fn peer_creds(&self) -> io::Result<PeerCreds>` exists on the portable trait and all four
  sync/tokio × unix/windows backends.
- `impl From<OwnedFd> for Listener` (sync) and `impl TryFrom<OwnedFd> for Listener` (tokio) exist in
  `os/unix/uds_local_socket/` — this is the systemd fd adoption path.

**Rationale**: It is the only candidate covering Unix mode bits, Windows SDDL DACLs, automatic
first-instance anti-squatting, portable peer credentials, and systemd fd adoption behind one API.
This directly serves FR-029 (portable transport), FR-030 (owner-only access) and FR-037 (one binary,
both launch paths) without a platform-specific escape hatch that would violate FR-036's
"platform differences behind one abstraction".

**Caveat**: `ListenerOptionsExt::mode()` is Linux/FreeBSD≥14.3/OpenBSD only — **macOS returns
`Unsupported`** (Darwin cannot `fchmod` a socket fd). macOS access control therefore rests on the
parent directory mode, which is why the endpoint lives under a 0700 directory (R1.2).

**Alternatives considered**:

| Option | Verdict |
|---|---|
| Raw `tokio::net::UnixStream` + `tokio::net::windows::named_pipe` | Credible fallback — same primitives underneath. Rejected: reimplements the abstraction, and `ServerOptions` has no safe DACL helper (only `unsafe create_with_security_attributes_raw`). |
| `parity-tokio-ipc` | ⚠️ **UNVERIFIED** — version and maintenance never checked. |
| `tarpc` | ⚠️ **UNVERIFIED**. Solves RPC codegen, not transport; would layer on top of this decision rather than replace it. |
| TCP on 127.0.0.1 + token file | **Rejected outright.** Any local process can connect; discards kernel-enforced peer identity in favour of a bearer secret needing storage, rotation and constant-time comparison. Adds Windows firewall prompts and port collision. |

**Hard constraint confirmed**: AF_UNIX on Windows is not an escape hatch. `cfg_net_unix!` in
`tokio/src/macros/cfg.rs` gates on `#[cfg(all(unix, feature = "net"))]` — a target-family gate — so
`UnixListener` does not exist on Windows regardless of build number.

### R1.1 Name construction is a security decision

- `GenericFilePath` / `.to_fs_name()` — Linux+macOS: filesystem path, untransformed. Windows: only
  accepts paths already starting with `\\.\pipe\`.
- `GenericNamespaced` / `.to_ns_name()` — Windows: prepends `\\.\pipe\`. **Linux: the abstract
  namespace.** macOS: prepends `/tmp/`.

Per `unix(7)`, *"Socket permissions have no meaning for abstract sockets."* There is no access
control at all; the only boundary is the network namespace, shared by every desktop process.

**Decision: `GenericFilePath` with explicit per-OS paths.** `GenericNamespaced` would ship three
different security postures across three targets — unacceptable under FR-030.

### R1.2 Endpoint location policy — the spec's macOS guidance is wrong

The spec proposes "macOS: under the app's application-support or cache directory". **The
application-support half does not fit.**

macOS `sockaddr_un.sun_path` is **104 bytes (103 usable)** — verified verbatim from XNU
`bsd/sys/un.h`; it includes a `sun_len` byte that Linux lacks (Linux is 108). Measured budgets:

| Path | Length | Verdict |
|---|---|---|
| `~/Library/Application Support/<reverse-dns>/…` typical username | 88/103 | Unsafe |
| same, corporate AD username | 99/103 | Unsafe |
| same + session discriminator | >104 | **Fails** |
| `$TMPDIR` (fixed format) | 49/103 | Safe but auto-reaped |
| **`$HOME/.micold/run/d.sock`** | **55/103 worst case** | **48 chars headroom** |
| `_CS_DARWIN_USER_CACHE_DIR` | 63, constant | Fallback |

Overruns surface as an opaque `EINVAL`, not `ENAMETOOLONG`. Lima's documentation cites exactly this
as why it avoids Application Support; VS Code is a recurring real-world failure case. `$TMPDIR` is
per-user and 0700 but is reaped after 3 days without access by `dirhelper`/`110.clean-tmps` — a
silent-failure risk for a long-lived socket.

**Final policy**:

| OS | Endpoint | Notes |
|---|---|---|
| **Linux** | `$XDG_RUNTIME_DIR/micold/daemon.sock` | Spec mandates 0700, user-owned. **Set the sticky bit** — the XDG spec permits periodic cleanup of files not touched every 6h unless sticky. |
| **Linux, `$XDG_RUNTIME_DIR` unset** | `/tmp/micold-<uid>/`, created 0700 **and then verified** | Do **not** probe `/run/user/<uid>` — it is mounted by `pam_systemd` at login; if the var is unset the mount did not happen and the dir is root-owned. Do **not** use `~/.cache` — frequently NFS (where AF_UNIX misbehaves) and survives logout. This is what tmux does. |
| **macOS** | `$HOME/.micold/run/d.sock` | Assert path length at bind time so overruns are loud, not `EINVAL`. Fallback `_CS_DARWIN_USER_CACHE_DIR`. |
| **Windows** | `\\.\pipe\Micold.Daemon.<user-SID>` | ~69 of 256 chars. |

**`/tmp` fallback verification is load-bearing**: `/tmp` is world-writable and the path is
predictable. Use `symlink_metadata` (not `metadata`, to defeat a planted symlink), check
`uid() == geteuid()`, check mode is exactly 0700. **Bail loudly** — wrong ownership means an active
attack, not a mess to tidy. This is the FR-030 + Settled Decision 8 ("loud, early failure") case.

### R1.3 Windows still needs an explicit DACL

The pipe namespace is machine-global and flat with no per-user partitioning. Two RDP users would
otherwise contend for one name, and clients connect FIFO to whichever instance was created first —
so one user's client can land on another's daemon. Precedent: Google Omaha appends the user SID for
exactly this reason.

But the SID in the name buys **collision avoidance, not security** — the name is public and
guessable, and `PIPE_REJECT_REMOTE_CLIENTS` only blocks clients arriving over SMB. Neither stops a
*local* process. Microsoft documents that the default security descriptor grants "read access to
members of the Everyone group and the anonymous account."

```
D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x12019b;;;<sid>)
```

`D:P` = protected, blocking inherited ACEs — this is what strips the Everyone/anonymous grant. The
unusual mask implements a documented trap: `FILE_APPEND_DATA` and `FILE_CREATE_PIPE_INSTANCE` share
a bit, so granting plain `FW` would let clients create rival instances of our own pipe.

⚠️ **`0x12019b` is a derivation, not a quoted constant — verify in an integration test.**

For RDS / fast user switching, Microsoft recommends the **logon SID** in the DACL (session-scoped)
alongside the user SID in the name (stable, for discovery).

### R1.4 Stale endpoint and the single-instance startup race

**On Unix, socket existence proves nothing.** Per `unix(7)` a bound socket "must be deleted by the
caller"; the kernel never removes it. `bind()` returns `EADDRINUSE` for an existing filesystem
object regardless of liveness, with no `SO_REUSEADDR` escape. The discriminator is `connect()`:
`ECONNREFUSED` = stale, `ENOENT` = never existed, success = live. Guard the unlink with an
`S_ISSOCK` check, since a plain file also yields `ECONNREFUSED`.

**Connect-testing alone is racy, and adding a lock naively does not fix it.** Both starters see
`ECONNREFUSED`; A unlinks and binds; **B then unlinks A's live socket** — unlink operates on the
name, not the inode — and binds its own. Two daemons, one permanently unreachable. The window
cannot be narrowed away; check and mutation are simply not atomic.

**The correct sequence** (step 3 is the one commonly omitted):

```text
1. connect() -> Ok            => act as client. Fast path, no lock.
2. try_lock exclusive on <runtime>/daemon.lock
     WouldBlock               => someone is mid-recovery; back off, goto 1. Touch nothing.
3. RE-CHECK connect()         <-- without this, the lock LOSER unlinks the winner's live socket
     Ok                       => the other starter won; drop lock, act as client.
4. unlink(sock) (ignore ENOENT); bind; listen
5. HOLD THE LOCK FOR PROCESS LIFETIME
```

**Hold the lock for life.** Per `flock(2)` the lock releases "when all such file descriptors have
been closed", so process death drops it — including SIGKILL and OOM, where no cleanup code can run.
That is an unforgeable liveness beacon, strictly better than a PID file (no PID-reuse hazard), and
others can probe it non-destructively with `try_lock`. Keep the lockfile off NFS (since 2.6.12
`flock` there is emulated via `fcntl`).

**Windows needs none of this.** Named pipes live in NPFS, not on a volume; `CreateNamedPipeW`
guarantees deletion "when the last handle to the instance … is closed". No stale file, ever. And
because `interprocess` sets `FILE_FLAG_FIRST_PIPE_INSTANCE` automatically, check-and-become-daemon
collapses into **a single atomic kernel call with no TOCTOU gap** — the second creator fails with
`ERROR_ACCESS_DENIED`. Treat that as "a daemon exists, become a client". Distinguish
`ErrorKind::NotFound` (spawn one) from `ERROR_PIPE_BUSY` (exists, retry) — conflating them causes
duplicate launches. `WaitNamedPipe` is not a boot-wait primitive; it returns immediately when no
instance exists.

**Embrace the asymmetry rather than abstracting it**: Windows has an atomic primitive; Unix needs
the lockfile because unlink-then-bind cannot be made atomic.

### R1.5 ⚠️ MSRV decision required

`File::lock` / `try_lock` / `unlock` **stabilized in Rust 1.89.0** (2025-08-07); current stable is
1.97.1. `Cargo.toml` pins `rust-version = "1.80"`.

| Option | Trade-off |
|---|---|
| **Bump MSRV to 1.89** (recommended) | Zero dependencies, std-native. 1.89 is eleven releases old. |
| `fd-lock` 4.0.4 (2025-03-10) | RAII guard, Windows supported. Keeps MSRV 1.80. |
| `fs4` 1.1.0 (2026-04-28) | Maintained `fs2` fork, has async variants. |

`fs2` (2018) and `single-instance` (2021) are abandoned. Note `try_lock` returns
`Result<(), TryLockError>` — a distinct type, not `io::ErrorKind`.

**This is a project-policy call, surfaced in the plan rather than decided here.**

### R1.6 Detached spawn

**Unix**: `setsid()` via `pre_exec` is correct, and the double fork is **unnecessary** here.
`Command::spawn` supplies the first fork; the second fork in `daemon(7)` only guards against later
opening a TTY without `O_NOCTTY`, which this daemon never does, and a GUI parent has no controlling
terminal anyway. `pre_exec` is `unsafe` because the closure runs post-`fork` holding the parent's
locks in whatever state other threads left them — std warns that `malloc`, `std::env` and mutexes
are "not guaranteed to work". A bare `libc::setsid()` is async-signal-safe and allocation-free,
which is why it is among the few things safe to do there. Use `Stdio::null()` on all three streams;
an inherited pipe tethers the daemon to the parent and can kill it with SIGPIPE.

**Correction**: `process_group(0)` (stable 1.64) is **not** a substitute — a process group is not a
session and does not drop the controlling terminal. `CommandExt::setsid` is nightly-only. Also do
not test `getppid() == 1`: since Linux 3.4, `PR_SET_CHILD_SUBREAPER` (set by systemd user sessions)
reparents to the nearest subreaper.

**Windows**: `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` confirmed. The job-object issue is worse
than the spec assumed — with nested jobs (Win8+) a child joins *every* job in the parent's chain,
and "if the immediate job object does not allow breakaway, the child process does not break away
even if jobs in its parent job chain allow it."

**Do not set `CREATE_BREAKAWAY_FROM_JOB` unconditionally** — Chromium removed exactly that because
it broke launching from Task Scheduler (CL 1546313002). Detect first via `IsProcessInJob` +
`QueryInformationJobObject`, request breakaway only when `JOB_OBJECT_LIMIT_BREAKAWAY_OK` is set,
and keep retry-without-flag as a fallback.

**Honest limitation**: if confined with `KILL_ON_JOB_CLOSE` and no breakaway, creation flags
**cannot** save the daemon. The real escape hatches are OS-owned intermediaries (Windows service,
`schtasks`, WMI). Report that diagnostic rather than silently launching a daemon that will die.

**Crates**: cfg-gate it ourselves. `daemonize` 0.5.0 (2023, unmaintained) is Unix-only *and* solves
the wrong problem — it daemonizes the current process, not a spawned child. The only cross-platform
candidate, `daemon-kit`, has ~1,500 total downloads. Two ~25-line functions behind one `#[cfg]`
split.

---

## R2. Logging and diagnostics (FR-043 – FR-047)

### Decision: `tracing` + `tracing-subscriber`, with `file-rotate` for the bounded file sink

| Crate | Version | Released |
|---|---|---|
| `tracing` | 0.1.44 | 2025-12-18 |
| `tracing-subscriber` | 0.3.23 | 2026-03-13 |
| `tracing-appender` | 0.2.5 | 2026-04-17 |
| `file-rotate` | 0.8.0 | 2025-02-27 |
| `tracing-journald` | 0.3.2 | 2025-11-26 |

**Rationale for `tracing` over `log` + `env_logger`/`fern`**: the `Layer` trait *is* the composition
mechanism, so FR-043's "backend configurable, not hard-wired" is its design centre rather than a
bolt-on; spans survive `.await` points, which matters in a tokio daemon; and `log`-based crates in
the dependency tree are captured free via `tracing-log`.

Erase layer types with `.boxed()` and select at init — a `Vec<Box<dyn Layer<S> + Send + Sync>>`
itself implements `Layer`. For runtime verbosity changes (FR-043), wrap the `EnvFilter` in
`tracing_subscriber::reload::Layer` and expose a daemon RPC that calls `handle.reload(...)`.

⚠️ **Keep the `WorkerGuard` alive for the process lifetime** — dropping it flushes; losing it
silently drops buffered logs at exit. Classic bug.

### R2.1 `tracing-appender` cannot satisfy FR-044's hard size cap

It rotates by time only (`MINUTELY`/`HOURLY`/`DAILY`/`NEVER`) and **has no retention policy — it
never deletes old files**. Unbounded disk growth. Known upstream gap (tokio-rs/tracing#1940,
discussion #2823).

Useful split: `tracing_appender::non_blocking` (background writer) is orthogonal to
`tracing_appender::rolling` (the appender). **Keep `non_blocking`, replace `rolling`.**

```rust
FileRotate::new(
    log_dir.join("micold-daemon.log"),
    AppendCount::new(5),                       // keeps exactly 5 rotated files
    ContentLimit::Bytes(5 * 1024 * 1024),
    Compression::None,
    #[cfg(unix)] Some(0o600),                  // logs may contain paths/session metadata
)
```

Total bounded to `(N+1) × M` = 30 MiB arithmetically. Use `AppendCount` or
`FileLimit::MaxFiles(N)` — **not `FileLimit::Age`, which does not bound disk**.

Alternatives: `tracing-rolling-file` 0.1.3 is ergonomically nicer and does deliver the cap, but is
0.1.x with a single maintainer (⚠️ maturity not audited). `flexi_logger` 0.31.9 has excellent
rotation but is a `log`-ecosystem framework — adopting it imports its whole logger model for an
appender.

### R2.2 Detecting the launch context for FR-044's adaptive default

The correct check is **`JOURNAL_STREAM`**, *verified* rather than merely present: systemd sets it to
`dev:inode` of the stderr fd, so confirm stderr actually is that stream by comparing
`metadata().dev()`/`.ino()`.

Distinguish the three variables — they answer different questions:

- **`JOURNAL_STREAM`** — "is my stderr the journal?" → **this is the logging decision.**
- **`INVOCATION_ID`** — "am I a systemd unit at all?" Set even when `StandardError=file:`, so it
  does not answer the logging question.
- **`LISTEN_FDS`/`LISTEN_PID`** — socket activation only. Unrelated to logging.

Portable fallback: **`std::io::IsTerminal`**, stable since Rust 1.70. `atty` is unmaintained
(RUSTSEC-2024-0375) and its own maintainer points at `IsTerminal`; do not add `atty` or
`is-terminal`.

Ladder: journal → terminal → rotating file. On macOS/Windows the first rung compiles out, so a
GUI-spawned daemon with no tty correctly lands on the file backend — exactly FR-044.

`tracing-journald` is Linux-only and worth offering as an opt-in `sink` value (native structured
fields, queryable via `journalctl MICOLD_SESSION_ID=…`), but **not as the default** — it would add a
hard journald socket dependency to the default path, and journald rate-limits and silently drops
bursts.

### R2.3 Log directory per OS

**Revised 2026-08-27 (BUG-015).** The table below is what the daemon does; what it *said* until then
is kept underneath, because the reason it changed is not obvious.

| OS | Location | Note |
|---|---|---|
| all | `data_local_dir()` → `micold-ai-ide/micold-daemon.log` | One `ProjectDirs` lookup, no `cfg`. On Linux `~/.local/share/micold-ai-ide/`, on macOS `~/Library/Application Support/micold-ai-ide/`, on Windows `%LOCALAPPDATA%\micold-ai-ide\data\`. |

`data_local_dir()`, **not `data_dir()`**: on Windows the latter is the *roaming* profile, and the
original table's Windows row is still binding — logs must never sync to a roaming profile. On Linux
and macOS the two resolve to the same directory, so this distinction costs nothing there.

**Not `cache_dir()`**, which is the one rule from the original table that never moved: cache is
defined as safely deletable and cleaners *do* delete it, eating logs mid-investigation.

The original table specified `state_dir()` → `~/.local/state/micold-daemon/logs/` on Linux, on the
grounds that XDG basedir 0.8+ names logs as state data — which it does, and which is why that was
the right first answer. Two things happened after it was written. The implementation used
`data_dir()` and nothing noticed for a year, so the quickstart's `ls` command pointed at a directory
that had never existed on any install (BUG-015's second finding). And the sandbox (feature 027) began
mounting the daemon's data directory into the container as its entire state directory — the image
sets `XDG_DATA_HOME=/var/lib` and `MountSet::build` binds the host's data dir there — which is how a
sandboxed daemon's log is readable from the host at all. Moving the log to `state_dir()` now would
put it outside that mount, so it would take a second mount and a change to the rule-governed mount
set to restore what data-dir placement already gives. `state_dir()` is also `None` on two of three
targets, so it needs a `cfg`-matched helper that CI can only partly exercise.

Against that, `data_local_dir()` has no safety hazard of its own — the deletable-cache objection does
not apply to it — so the divergence was resolved in favour of the implementation, with the Windows
roaming rule carried across as a one-word fix.

---

## R3. Process supervision (FR-005, FR-036)

### R3.1 `portable-pty` 0.9.0 — stay on it, with two known defects

v0.9.0, published 2025-02-11, in the wezterm monorepo but published independently (9.09M downloads).
Maintained slowly; last `pty/`-touching commit 2026-06-07. Already pinned at `0.9`.

`Child: ChildKiller + Debug + Downcast + Send`, with blocking `wait()`, `try_wait()`,
`process_id()`. **No async support whatsoever.**
`ChildKiller::clone_killer() -> Box<dyn ChildKiller + Send + Sync>` is the cross-thread kill handle;
on Windows it dups the process HANDLE, which is also the handle needed for job assignment.

⚠️ **`kill()` returns inverted results on Windows in 0.9.0** — the source reads
`if res != 0 { Err(err) } else { Ok(()) }`, i.e. `Err` on success. Fixed in wezterm PR #7709 (merged
2026-06-07) but **unreleased**. Ignore the `Result` on Windows, or vendor the patch.

⚠️ **Windows `kill()` is a bare `TerminateProcess` on the direct handle** — no job object, no tree
kill. That is entirely on us (R3.3).

**ConPTY pitfalls, all verified:**

| Issue | Consequence |
|---|---|
| Reader never reaches EOF when clients die (terminal#4564; fixed only in 22H2) | **Never gate teardown on reader EOF — it will hang.** Wait on the process handle. |
| `ClosePseudoConsole` blocks indefinitely pre-24H2; deadlocks with `PSEUDOCONSOLE_INHERIT_CURSOR` (#17688) | Call it last, inside `spawn_blocking`, with a timeout. |
| It sends `CTRL_CLOSE_EVENT` to remaining clients | Close the output pipe first, or keep draining until it returns. |
| Resize ignored near client attach (#10400); truncation on grow (#16879) | Debounce resizes; do not resize during spawn. |

### R3.2 Exit detection: thread-per-child, and it is already paid for

**Thread-per-child is fine at 10–50 sessions. Unambiguously.** A thread blocked in
`waitpid`/`WaitForSingleObject` is fully descheduled — no timer, no wakeups, zero scheduler cost.
Rust's 2 MiB default stack is *reserved* address space, lazily committed; a parked thread touches
1–2 pages. 50 threads ≈ 100 MiB virtual, a few hundred KB resident. The pattern becomes pathological
around 10⁴, not 10¹.

**The decisive point**: `src/ui/terminal.rs:299` already spawns a reader thread per session, and the
`portable-pty` reader is a blocking `Read` — so a thread per session is required regardless. Fold
the blocking `wait()` into the thread that already exists and send the status over a channel into
tokio. What should be **deleted** is the UI-driven `try_wait()` poll at `src/ui/terminal.rs:159`.

**Tokio cannot adopt an externally-spawned process — definitively.** `tokio::process::Child` has no
`from_raw`/`from_pid`/`from_std`; only `impl From<StdCommand> for Command` exists, at the *Command*
level. PR #7388 proposing `TryFrom<std::process::Child>` was closed unmerged 2025-12-25.
`pidfd_reaper` is `pub(crate)`.

**No crate composes with `portable-pty`.** `command-group` 5.0.1 is **deprecated**, naming
`process-wrap` 9.1.0 as successor; but `process-wrap`'s escape hatch is
`spawn_with(impl FnOnce(&mut std::process::Command) -> Result<std::process::Child>)`, and
`portable-pty` returns `Box<dyn Child>` and on Windows never touches `std::process::Command` (it
calls `CreateProcessW` directly). Neither side of that signature can be satisfied.

**Decision**: ~200 lines of per-platform abstraction behind one trait. Skip
pidfd/kqueue/`RegisterWaitForSingleObject` entirely at this scale; revisit past a few hundred
sessions.

### R3.3 Process-tree termination

**Unix — nearly free.** Verified in `portable-pty-0.9.0/src/unix.rs:257`: the `pre_exec` closure
calls `libc::setsid()`, then `ioctl(TIOCSCTTY)` when `set_controlling_tty`. **The child is already a
session and process-group leader with pgid == pid, so `killpg(child_pid, sig)` works with no action
from us.** `CommandBuilder`'s missing `pre_exec` does not matter.

Nuance: `killpg` only reaches processes that *stayed* in the child's group. A job-control shell puts
each job in a new group, so `killpg` would miss them — which is why portable-pty's own `Child::kill()`
sends **SIGHUP** (`lib.rs:325`, with a 5×50 ms grace period). `claude` is not a job-control shell, so
`killpg` is sufficient.

Note `MasterPty::process_group_leader()` uses `tcgetpgrp()` — that is the *foreground* group, correct
for Ctrl-C targeting, wrong for teardown.

**Windows — one job object per session**, via `windows-sys` 0.61.2 (features
`Win32_System_JobObjects`, `Win32_System_Threading`, `Win32_Foundation`) rather than `win32job` 2.0.3,
which pulls the full `windows` crate for ~40 lines of unsafe. The job handle needs
`JOB_OBJECT_ASSIGN_PROCESS`; the process handle needs **both `PROCESS_SET_QUOTA` and
`PROCESS_TERMINATE`** — a common silent failure.

Three findings that shape the design:

1. **Verified in `win/psuedocon.rs:145`: creation flags are exactly
   `EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT`.** No job, no `CREATE_SUSPENDED`, and
   no `CREATE_NEW_PROCESS_GROUP` — **do not add that flag.** `CreatePseudoConsole` cannot pass it to
   its host, so child and pty host land in different groups and Ctrl-C delivery breaks *invisibly*.
   `ping.exe` ignores the signal while `cmd.exe` appears to work (because `ReadConsole` aborts on
   ETX), so testing against `cmd.exe` would not catch it.
2. **Keep conhost/OpenConsole out of the job.** It is a child of the *daemon*, not of the shell, so
   assigning only the shell child leaves it correctly outside. Microsoft added a `KILL_ON_JOB_CLOSE`
   job to Windows Terminal and then deliberately removed it (PR #2198) — orphaned conhost and
   post-mortem debugger popups. Never call `assign_current_process()`.
3. **Unavoidable race**: `AssignProcessToJobObject` is forward-looking only, so grandchildren spawned
   between `CreateProcess` returning and the assignment escape permanently. Closing it properly would
   require patching portable-pty for `CREATE_SUSPENDED`. The window is microseconds and `claude`
   spawns nothing before first input — **accepting it is reasonable**, and it must be documented.

Set neither `BREAKAWAY_OK` nor `SILENT_BREAKAWAY_OK`; never `JobObjectBasicUIRestrictions` (it
disables job nesting, which is needed if the daemon itself runs under a CI/container job). Use
`KILL_ON_JOB_CLOSE` as a crash-safety net, not the primary kill path.

**`GenerateConsoleCtrlEvent` is unusable from a daemon.** It requires sharing a console with the
target, and a daemon has none; with a nonzero `dwProcessGroupId` it *returns success while delivering
nothing*. The `AttachConsole`/`FreeConsole` workaround is process-global (not thread-local), so it
corrupts every tokio worker at once, rebinds stdio, and has a real suicide window before
`SetConsoleCtrlHandler(NULL, TRUE)` lands. **Do not implement it.**

**Escalation ladder (both platforms)**: write `0x03` to the master → wait 2–5 s → `"exit\r\n"` or
`0x04` → wait → `TerminateJobObject` / `killpg(SIGKILL)`. On Unix escalate SIGHUP → SIGTERM → SIGKILL
via `killpg`. Drain the master *before* the hard kill; close the ConPTY last.

### R3.4 Interrupting a session: write `0x03`, never a real signal

Claude Code is an Ink TUI that puts the tty in raw mode, disabling `ISIG` — so the line discipline
will **not** convert `0x03` into SIGINT; it arrives as a keystroke and Ink's own handler interprets
it. A real `SIGINT` would bypass the application's handler entirely.

Under ConPTY this is symmetric: ConPTY's input parser matches `0x03`, converts it to a
`KEY_EVENT_RECORD`, and drives the same kernel path underlying `GenerateConsoleCtrlEvent`, targeting
the foreground application. Given R3.3, it is also the *only* workable path on Windows.

⚠️ The `0x03`-to-ConPTY path is well-sourced but **not empirically tested**. Worth a spike.

### R3.5 Surviving logout — the asymmetry is the headline

**Linux is free; macOS and Windows both require a privileged install with no unprivileged
equivalent.** This validates the spec's Linux-only scoping (FR-038), and gives the reason.

**Linux.** `enable-linger` does three things (read from `logind-dbus.c`,
`method_set_user_linger`): touches `/var/lib/systemd/linger/<user>`, calls `user_start()`
immediately, and on disable sets `gc_mode = USER_GC_BY_PIN`.

- **Privilege: none needed for yourself.** The shipped polkit policy has
  `org.freedesktop.login1.set-self-linger` with `allow_any=yes` — no authentication. Setting it for
  *another* user hits `set-user-linger`, which does require admin. Hardened deployments override
  this, so **detect failure rather than assume success**.
- ⚠️ **Retroactivity is a split answer, and it matters.** The user manager starts *immediately*
  (synchronous `user_start()`, no re-login). But **already-running processes are NOT migrated** — a
  process already in `session-N.scope` stays there and still dies at logout. **So: enable linger,
  then spawn into the user manager. Enabling it afterwards does not rescue an existing daemon.**
- **`setsid`/double-fork alone is insufficient**: killing is by **cgroup**, not process group or
  session. `setsid(2)` changes POSIX session membership, which is irrelevant; the process stays in
  the same cgroup and dies with it. Linger and scope-escape are *both* necessary.
- **`KillUserProcesses`**: upstream default is **`yes`**, but Debian, Fedora rawhide and Arch all
  ship `-Ddefault-kill-user-processes=false` (verified in each packaging source). ⚠️ **Correcting a
  widespread claim**: the Fedora wiki change page asserts Fedora reverted to upstream `yes` — it did
  not; rawhide still ships `false`. Do not rely on the distro patch either way.

**macOS — genuinely expensive.** A plain detached process does *not* reliably survive GUI logout:
`loginwindow` "terminate[s] any open background processes by sending a `SIGKILL` signal, regardless
of any returned errors." (SSH session end is different — ordinary SIGHUP, so `setsid` genuinely
works there. Fast user switching is not a logout.) The defensible answer is a **LaunchDaemon in
`/Library/LaunchDaemons` with `UserName=<user>`** — requires root to install, no login-keychain
access, and TCC is the sharp edge (daemons have no session in which to prompt; **root does not
bypass TCC**). `SMAppService` (macOS 13+) requires the plist inside an app bundle — likely a blocker
for a bare Rust binary.

⚠️ **One open item could materially cheapen macOS**: folklore says a LaunchAgent with
`LimitLoadToSessionType=Background` survives logout. **No Apple documentation confirms this.** If
true, macOS collapses to roughly Linux-level cost. **Worth a half-day empirical test before
architecting around the expensive LaunchDaemon path.**

**Windows — killed unconditionally at logoff.** `DETACHED_PROCESS`/`CREATE_NO_WINDOW` control
console attachment only, never logon-session membership. Per *Logging Off*, "all processes in the
logon session are terminated"; per *HandlerRoutine*, `CTRL_LOGOFF_EVENT` "is received only by
services" — handling it is not a veto. The docs name the only exemption: running as a service. A
**Windows Service under the user's own account** beats a Scheduled Task (S4U tokens have no network
credentials and do not work with domain accounts). `windows-service` v0.8.1 (2026-05-08, Mullvad,
production-proven) covers both the SCM entry point and installation.

**ConPTY in session 0 works** — it is headless by design; Win32-OpenSSH's `sshd` runs as LocalSystem
in session 0 and hosts interactive terminals via ConPTY. The real limitation, terminal#11865
(ConPTY + `CreateProcessWithLogon` → `ERROR_INVALID_PARAMETER`), **evaporates if the service runs
under the user's own account**.

**To avoid foreclosing this (FR-038's "must not be foreclosed")**: model "install the persistent
daemon" as a distinct, possibly-elevated, possibly-failing step separate from "start the daemon" —
Linux's free path is the special case, not the baseline. Do not let the daemon ambiently inherit
session-scoped resources (keychain, DPAPI-in-user-context, mapped drives); pass them explicitly or
over IPC. Keep the IPC transport session-independent from day one.

---

## R4. Busy vs awaiting-input detection — FR-016b's mechanism is falsified

**This was tested empirically against a real `claude` v2.1.215 in a `pty.fork()` on Linux, not
reasoned from documentation.** Several mechanisms the spec and feature request contemplated are
measurably dead.

### R4.1 Output quiescence does not work, and fails in the direction that matters

| Condition | Chunks | **Max inter-chunk gap** |
|---|---|---|
| Idle at prompt | 12 | **6.02 s** |
| Working (`sleep 25` tool call) | 247 | **20.50 s** |

The spinner does **not** repaint continuously during tool calls. Any threshold safe against a
25-second sleep must exceed it — and a 10-minute build is silent for 10 minutes. **No workable
threshold exists.**

**FR-016b currently mandates exactly this mechanism** ("a defined, documented quiescence threshold"),
and SC-016 demands zero spurious transitions during multi-minute agent work plus detection within
5 s. Those cannot both be met by quiescence. **The spec requires amendment.**

### R4.2 Every other PTY-derived signal is also dead

- **Terminal bell is a trap**: 16 BEL bytes appeared in raw capture; after stripping OSC sequences,
  **zero real bells** — all 16 were OSC-title terminators (`ESC ] 0 ; … BEL`). A naive `\a` scan is
  100% false positives. Bells also only fire under `preferredNotifChannel: "terminal_bell"`, not the
  default.
- **OSC 133 semantic prompt marking: zero occurrences.** It is an open feature request
  (claude-code#1465, #22528). OSC 9/777/7: zero.
- **Bracketed paste**: `?2004h` sent **once at startup, never toggled** (`?2004l` count: 0).
- **Cursor hide/show**: 107/108 occurrences — pure repaint noise.
- **`tcgetpgrp`**: Linux-only and near-useless. Linux special-cases the pty master in
  `tty_jobctrl.c`; **macOS XNU returns `ENOTTY`** (source-verified in `bsd/kern/tty.c`). Windows has
  no equivalent concept. And `claude` never cedes the foreground group, so it would not discriminate
  "waiting on model" from "waiting on human" anyway.
- **Process state: zero information, confirmed.** Every process showed `state=S`, `wchan=ep_poll`
  whether mid-request or idle. `sysinfo` 0.39.6 hardcodes `ProcessStatus::Run` on Windows.
  ⚠️ **Also, a premise is outdated**: v2.1.215 ships as a **Bun-compiled native binary, not a Node
  process** — anything keyed to Node internals rests on a wrong assumption.

The one accidental signal is the OSC 0 title carrying an animated spinner glyph that switches to a
task description while working — real, but undocumented and fragile.

### R4.3 Decision: Claude Code hooks over HTTP to the daemon

The full lifecycle was **observed firing in interactive PTY mode**:

```text
SessionStart → UserPromptSubmit → PreToolUse(Bash) → PostToolUse(Bash) → Stop
```

Each payload carries `session_id` and `transcript_path` on stdin. `Stop` fired ~12 ms before the
turn-end result. Hooks support `type: "http"`, so they POST directly to the daemon — no scripts.
Configurable per-session via `--settings <file>`, so user config is never touched.

State machine: `UserPromptSubmit` / `PreToolUse` → **working**; `Stop` → **awaiting input**;
`Notification` with `notification_type` in `permission_prompt` / `idle_prompt` /
`agent_needs_input` → **awaiting input**.

**Rationale**: this is an authoritative application-level signal rather than a heuristic, and it is
identical on all three platforms — no PTY scraping, no `/proc`, no ConPTY-equivalence problem. **It
removes the hardest platform-divergence risk from the design entirely**, and notably means FR-016's
mechanism does *not* need to sit behind the FR-036 platform abstraction.

**Alternatives considered**:

- **`--input-format stream-json` / `--output-format stream-json`** — strictly better where the PTY
  can be dropped; an explicit `result` message ends each turn and the question becomes moot.
  Requires ≥ 2.1.208. ⚠️ Known bugs: missing/hung `result` (claude-code#8126, #25629) — needs a
  watchdog. Not viable for the interactive TUI sessions this feature is about, but worth keeping in
  view.
- **Transcript JSONL (degraded fallback)** — path confirmed as
  `~/.claude/projects/<slug>/<uuid>.jsonl`. **Take the path from the hook payload; do not compute
  the slug** — the current `/` and `.` → `-` transform (`src/provider.rs:361-373`) is lossy and
  collision-prone (claude-code#7009). Turn end is detectable (`stop_reason` `end_turn` vs
  `tool_use`), but the official hooks documentation states the transcript **"is written
  asynchronously and may lag the in-memory conversation"** — explicitly not a real-time signal. Scan
  backwards; the last line is often a `last-prompt`/`ai-title` record.

**How this can still be wrong** (must be reflected in the spec):

- Hooks are **config-dependent**. A user with `--bare` or conflicting settings silently loses them.
  **Treat missing hooks as *unknown*, never as *idle*.**
- `Stop` means the model turn ended, **not** that a human is required — auto-continuation or a
  blocking Stop hook can resume without input.
- Transcript tailing races the async writer; tolerate trailing partial lines.

⚠️ **Explicitly unverified**: the `Notification` hook subtypes did not fire during testing (no
permission prompt occurred) — **verify before relying on them**. macOS `tcgetpgrp` behaviour is
source-verified only, not runtime-tested. Exact slug character rules beyond `/` and `.`. Whether
Claude Code disables animation on a non-TTY.

---

## R5. Packaging and the systemd user unit (FR-037, FR-038)

### R5.1 `systemctl --user daemon-reload` in postinst is impossible; `--global enable` is the wrong fix

postinst runs as root with no user session, no `XDG_RUNTIME_DIR`, and no way to reach each user's
manager. `systemctl --global enable` *does* work — it symlinks into `/etc/systemd/user/<target>.wants/`
and, per systemctl(1), **reloads no daemon configuration**, so the root/no-session problem
evaporates. But its caveats are real:

1. **Future logins only.** Users logged in at install time are unaffected until re-login.
2. **All users**, including service accounts with a user manager. Over-broad for a developer IDE.
3. **`/etc/systemd/user/` symlinks are not dpkg-tracked** — postrm must `--global disable` or they
   leak.
4. **A user who runs `systemctl --user disable` finds it still starts**, because global enablement
   wins; they would need `mask`. systemd warns about exactly this.
5. Cannot be combined with `--now`.

**Decision: ship the units, do NOT enable at install, and have the GUI client enable and start the
unit in the user's own session on first run.** That is precisely the context postinst lacks —
`systemctl --user` works correctly there and needs no privilege. It sidesteps all five caveats.

This also sidesteps a tooling limitation: ⚠️ **`cargo-deb` 3.7.0's systemd support is written for
*system* units** — its documentation says nothing about user units, `/usr/lib/systemd/user`, or
`--global`, and its generated fragments use system-scope `systemctl`. **Unverified whether it
breaks, only that it is undocumented.** The manual-assets approach avoids the risk entirely:

```toml
[package.metadata.deb]
maintainer-scripts = "debian/"
assets = [
  ["target/release/micold-daemon", "usr/bin/", "755"],
  ["packaging/micold-daemon.socket",  "usr/lib/systemd/user/", "644"],
  ["packaging/micold-daemon.service", "usr/lib/systemd/user/", "644"],
]
```

Install path is `/usr/lib/systemd/user/` — Debian is merged-`/usr` since bookworm and
`dh_installsystemduser` targets it; `/lib/systemd/user` is a compat symlink. Never
`/etc/systemd/user/` (admin config).

### R5.2 Unit files

```ini
# /usr/lib/systemd/user/micold-daemon.socket
[Unit]
Description=Micold daemon socket

[Socket]
ListenStream=%t/micold/daemon.sock
SocketMode=0600
DirectoryMode=0700
Accept=no

[Install]
WantedBy=sockets.target
```

```ini
# /usr/lib/systemd/user/micold-daemon.service
[Unit]
Description=Micold daemon
Requires=micold-daemon.socket
After=micold-daemon.socket

[Service]
Type=notify
NotifyAccess=main
ExecStart=/usr/bin/micold-daemon --systemd
Restart=on-failure
RestartSec=2s
Environment=MICOLD_LOG=info

[Install]
WantedBy=default.target
```

- **`%t` = `$XDG_RUNTIME_DIR`** for user units — matches R1.2's Linux endpoint exactly.
- **`Accept=no`** is required: `Accept=yes` spawns a process per connection, wrong for a stateful
  session daemon.
- **Do not add `StopWhenUnneeded=`** to the socket — it would tear the service down.

### R5.3 FR-002 ("never exit while a session is alive") is confirmed unobstructed

**Nothing in systemd forces exit.** systemd has **no idle-exit mechanism** for socket-activated
services — the pattern (dbus-daemon, podman `--time=N`) is always implemented by the daemon itself.
Socket activation governs *start*, never *stop*. `RuntimeMaxSec=` defaults to `infinity` (do not set
it). `TimeoutStopSec=` only bounds shutdown once stop is requested.

If we later add idle-exit at zero sessions, exit **0** — `Restart=on-failure` then correctly leaves
it stopped and the socket re-activates on next connect. This is exactly why `on-failure` beats
`always` here; `Restart=always` would restart a clean idle exit and defeat activation.

⚠️ Asserted from documented defaults rather than a fetched man page in-session.

### R5.4 Do not make socket activation load-bearing

A desktop app launched from a `.desktop` file, Flatpak or AppImage does not go through a systemd
unit, and the classic benefits (boot-ordering parallelism, on-demand start, privilege separation) do
not apply to an unprivileged per-user helper. The one real win — race-free startup — we already get
from R1.4's lock pattern, which works unchanged on all three OSes.

**Build self-spawn-and-lock as the mechanism; add activation as a ~15-line opportunistic Linux
path.** The failure mode to avoid is activation becoming *required*, which breaks Flatpak/AppImage
users and local development. This is precisely FR-037's "identically whether launched by that
manager or spawned directly by a client, from a single binary".

### R5.5 systemd fd adoption

**`listenfd` 1.0.2** (maintainer mitsuhiko). Its `Cargo.toml` target-gates `libc` vs `winapi` and
`lib.rs` splits on `#[cfg(not(windows))]` — **it compiles on all three platforms**, so depend on it
unconditionally and gate only the Unix-only call sites. `sd-notify` 0.5.0 is pure Rust and adds
READY/WATCHDOG for `Type=notify` but has no verified non-Linux support — gate it.

Avoid `libsystemd` 0.7.2 (depends on `nix` with no target tables — **will not compile on Windows**)
and `systemd` 0.10.1 (FFI, needs `libsystemd-dev` on every dev machine and CI runner).

⚠️ **Cargo caveat**: `target.'cfg(...)'` cannot be combined with `feature = ...` — per the Cargo
reference those values "will not work as expected." Make the dep optional and gate via `[features]`.

```rust
#[cfg(target_os = "linux")]
pub fn activated() -> io::Result<Option<Listener>> {
    let Some(std_l) = listenfd::ListenFd::from_env().take_unix_listener(0)? else { return Ok(None) };
    std_l.set_nonblocking(true)?;   // REQUIRED — systemd does not guarantee it
    Ok(Some(UdsListener::try_from(OwnedFd::from(std_l))?.into()))
}

#[cfg(not(target_os = "linux"))]
pub fn activated() -> io::Result<Option<Listener>> { Ok(None) }
```

**Keeping the signature identical across both arms is what keeps macOS/Windows building** without
conditionals at every call site.

Protocol notes: fds start at `SD_LISTEN_FDS_START = 3`; check `LISTEN_PID == getpid()` because env
vars survive `fork`/`exec`; unset the vars after reading. Good detail from source: `interprocess`'s
`From<OwnedFd>` impls set `reclaim: ReclaimGuard::default()`, so an adopted listener **will not
unlink** the path on drop — exactly right for a systemd fd, which systemd reuses.

⚠️ `listenfd` uses a *modified* protocol that **skips the `LISTEN_PID` check** when the var is unset
(deliberate, for proxied binaries). If that check is treated as the safety basis for the `unsafe` fd
adoption, verify it independently.

### R5.6 `loginctl enable-linger` — document, do not automate

Covered mechanically in R3.5. The reasons to document rather than run from postinst:

1. postinst has no user context — it would have to guess which users to enable, and enumerating
   `/home` or `getent passwd` to flip a per-user persistence flag would be grounds for package
   rejection.
2. Linger is an **admin policy decision** with security and resource implications; some sites
   deliberately disable it by polkit rules. Silently enabling it subverts local policy.
3. dpkg has no clean uninstall story — we cannot know whether the user set it for other reasons.

This matches FR-038 exactly ("MUST NOT be enabled silently by installation").

---

## R6. Prior art for the streaming protocol

Two **negative** results that narrow the design space, plus one close analog.

### R6.1 kitty's remote-control protocol cannot push screen state

Framing is verified exact: `<ESC>P@kitty-cmd<JSON><ESC>\` (DCS, not APC; 12-byte prefix; ST
terminator), used identically over a `--listen-on` socket.

But it is **strictly request/response**. `get-text` is one-shot — no watch flag, no deltas, no
sequence numbers. There is no subscription or notification mechanism. **The `async` facility is a
trap**: `AsyncResponder` binds to a *prior client request* via `async_id` + `peer_id`, so it buys a
deferred response to a command already issued. The terminal cannot initiate. Streaming
(`stream`/`stream_id`) is **inbound only**.

**Consequence**: following kitty would mean client-driven polling of full snapshots with diffing on
our side — no dirty regions, no wakeup. Rejected. It is a control plane, not a screen-state
transport.

⚠️ Not a machine-exhaustive audit of all ~60 `kitty/rc/*.py`, but rests on the files where such a
mechanism would necessarily be plumbed.

### R6.2 GNU screen has no wire protocol at all

Layering is window (pty + `struct mline` cell buffer) → canvas (viewport) → display (one tty, own
`D_obuf`/termcap/cursor), and `RefreshLine`/`DisplayLine` diff old-vs-new rows to emit minimal
escapes per display. N terminals re-render independently from one shared buffer — architecturally
interesting.

**But there is no client process to carry state to.** `screen -x` hands its tty fd over the socket
and then does nothing; the server writes VT escapes into that fd. The socket carries only fixed-size
`struct msg` (`MSG_CREATE 0` … `MSG_QUERY 9`), and **no message type conveys cell data, attributes,
cursor position or scrollback.**

**Consequence**: adopting this shape makes the transport "a pty full of escape codes", forcing the
GUI to implement a terminal emulator to recover structured state the daemon already had — precisely
the split-brain that Settled Decision 1 exists to prevent. Rejected.

⚠️ `struct msg` / `MSG_*` are from screen 4.1.0 via a GitHub mirror (savannah timed out); screen 5.x
split the headers. The structural argument is unaffected. The `display.c` layering description *is*
current master.

### R6.3 wezterm mux — the closest analog, and the model to follow

- **Codec** (`codec/src/lib.rs`): leb128 varint framing, `tagged_len | serial | ident | data`, where
  the high bit of `tagged_len` (`COMPRESSED_MASK = 1 << 63`) flags zstd compression and `len` counts
  the encoded ident+serial bytes too. Payload codec is **varbincode** (a varint-integer bincode
  variant). `COMPRESS_THRESH = 32` bytes — above it, re-serialize through zstd and keep the
  compressed form only if smaller. `CODEC_VERSION = 45`. ⚠️ **No maximum PDU size cap exists** (an
  absence claim from grepping, not a positive confirmation).
- **Push vs poll**: hybrid, push-dominant. There is no `UnilateralPdu` type; **pushes are identified
  by `serial: 0`**. The server pushes `GetPaneRenderChangesResponse` on `MuxNotification::PaneOutput`;
  the client *also* polls `GetPaneRenderChanges` with exponential backoff from
  `BASE_POLL_INTERVAL = 20ms` to `MAX_POLL_INTERVAL = 30s`, **which doubles as the liveness probe**
  (`LivenessResponse`). Deltas are two-tier: `dirty_lines` row ranges plus inlined `bonus_lines` for
  the viewport and cursor row.
- **Seqno**: a single monotonic **per-terminal** counter stamped onto each `Line`, not per-line
  counters. **The client never sends a seqno.** The server holds a per-connection, per-pane
  `PerPane { seqno, .. }` cursor and diffs via `pane.get_changed_since(range, old_seqno)`, so each
  client gets "everything since I last told *you* anything", isolated from other clients.

**Adopt**: the server-side per-connection seqno cursor (keeps the client stateless, which Settled
Decision 6's takeover model assumes), and the poll-doubles-as-liveness-probe idea (one mechanism
satisfies both missed-push recovery and FR-026's half-open detection, instead of a separate
heartbeat).

**Diverge**: add an explicit maximum frame size. An unbounded frame from a buggy or hostile peer is
exactly the loud-early-failure case of Settled Decision 8.

⚠️ zstd level 3 is the crate default, not verified in zstd's source.

---

## R7. Crate topology

The daemon must link `portable-pty` and `alacritty_terminal` **without** iced. Today both are
`dep:`-gated behind the `gui` feature (`Cargo.toml`), so a daemon binary cannot reach them, and the
core lib must stay render-free so `cargo test --no-default-features` keeps working (FR-040).

**Decision**: split the feature flags so the PTY/VT stack is independently selectable:

```toml
[features]
default = ["gui"]
gui    = ["dep:iced", "dep:dark-light"]
daemon = ["dep:portable-pty", "dep:alacritty_terminal", "dep:tokio", "dep:interprocess", ...]
```

with a second `[[bin]]` target `micold-daemon` requiring `daemon`, and the shared protocol types
living in the render-free core lib so both binaries compile against one definition.

**Rationale**: this is the minimal change that satisfies FR-040 and Settled Decision "three units"
without introducing a workspace split. A full workspace (separate `core`/`daemon`/`client` crates)
is the cleaner long-term shape and should be considered in the plan, but is not required by any
functional requirement.

⚠️ Note the Cargo caveat from R5.5: optional deps gated by feature, never by
`target.'cfg(...)'` + `feature`.

---

## R8. Wire format and grid streaming

Benchmarked locally on rustc 1.97.0, not reasoned from documentation. Two measured results dominate
every other consideration here.

### R8.1 Representation beats format by ~15×; diff keying beats damage by ~11×

**(a) RLE style-runs + a per-frame interned style palette, vs naive per-cell structs, 80×24 full frame:**

| Format | Naive per-cell | RLE + interning | Ratio |
|---|---|---|---|
| JSON | 106,052 B | 6,813 B | 15.6× |
| MessagePack (compact) | 39,053 B | 2,624 B | 14.9× |
| CBOR (ciborium) | 66,495 B | 5,235 B | 12.7× |
| postcard | 14,042 B | 2,353 B | 6.0× |

**(b) Keying the diff by stable absolute line ID, vs damage tracking.** Simulating `cat` of a
500-line file into an 80×24 terminal:

```text
alacritty FULL-damage frames      : 477 / 500
lines/frame (alacritty damage)    : 22.99   <- effectively a full redraw
lines/frame (viewport-index diff) : 21.99
lines/frame (stable line id diff) :  2.00   <- 11x better
```

**Scrolling is the common terminal workload, and `alacritty_terminal` reports `TermDamage::Full` on
every scroll.** A design built only on `Term::damage()` degenerates to full-frame resends. This is
the single most important architectural decision in the streaming layer, and it is the conclusion
Mosh and wezterm reached independently.

Encode latency is a non-issue for every candidate (200×50 RLE frame): JSON 18.0 µs, msgpack 5.7 µs,
CBOR 12.1 µs, postcard 4.2 µs. Even JSON at 60 Hz is ~0.1% of one core. **Do not pick a format for
speed** — pick for debuggability and size.

### R8.2 ⚠️ `bincode` is dead — do not use it

`bincode` 3.0.0 (2025-12-16) is a deliberate tombstone. The published crate's `src/lib.rs` is, in
its entirety:

```rust
compile_error!("https://xkcd.com/2347/");
```

The README states development ceased after a doxxing and harassment incident, and no further
releases will be published. `bincode-org/bincode` is **archived** (last push 2025-08-15). Last
functional release is 2.0.1 (2025-03-10), unmaintained. Upstream recommends `wincode`, `postcard`,
`rkyv`. **Any scaffold reaching for bincode must be corrected.**

### R8.3 Decision: hybrid envelope, one framing layer

| Concern | Choice | Version |
|---|---|---|
| Framing | `tokio_util::codec::LengthDelimitedCodec`, u32 LE, `max_frame_length` 16 MiB | tokio-util 0.7.18 |
| Control plane | **`serde_json`** — low volume, high debugging value | 1.0.151 |
| Grid frames | **`postcard`**, with a `MICOLD_WIRE=json` debug override | 1.1.3 |
| Compression | **None initially.** `lz4_flex` behind a flag only if the transport ever goes remote. Never zstd. | 0.14.0 |

**One framed stream with a tag byte, not two channels** — ordering between control and grid messages
must be well-defined (a `Resize` has to be ordered against the frames around it):

```text
4-byte length prefix
u8  encoding : 0 = JSON, 1 = postcard, (2 = postcard+lz4)
u8  kind     : 0 = control/RPC, 1 = grid frame
u16 reserved
.. payload
```

**The debug switch is what makes the hybrid pay off.** Because both encodings are `serde`, the *same*
`GridFrame` type serializes either way, so `MICOLD_WIRE=json` yields a fully human-readable stream
with zero extra code — postcard's 2,353 bytes in production, JSON's 6,813 readable bytes when
something is wrong. This is the specific reason to reject rkyv/capnp: they would forfeit it.

`max_frame_length` must be **set explicitly**, not left at the 8 MiB default: a 100k-line scrollback
response at ~100 B/line is ~10 MB and would exceed it. Raise to 16 MiB *and* chunk scrollback
responses (R8.8) — both, not either. An unbounded frame is the hole in wezterm's codec (R6.3) and
the Settled-Decision-8 loud-failure case.

**Compression is not worth it locally.** A full 80×24 frame is 2,353 B; at 60 Hz that is 141 KB/s
uncompressed, and with the stable-ID diff it drops to ~15 KB/s. lz4 would cost 2.9 µs to save ~1 KB
on a Unix socket. Ship uncompressed.

**Defensible simplification**: `rmp-serde` (compact) for everything is 2,624 B vs postcard's 2,353 B
— 11% worse while staying self-describing and `tcpdump`-able. If the team prefers one format over
11%, take MessagePack throughout. **The hybrid is better only if the debug switch actually gets
built.**

Rejected: **Cap'n Proto / rkyv** — zero-copy pays off when you mmap large buffers or skip
deserialization, but frames are 2–7 KB and must be transformed into iced widgets anyway, so the win
cannot be collected. Adds a schema compiler or `unsafe` validation for nothing.

### R8.4 `alacritty_terminal` — version bump and four undocumented behaviours

**Current is 0.26.0** (2026-04-06); the repo pins **0.25**. `serde` is a **default feature**.

Four behaviours verified empirically that the documentation does not state:

1. **`damage()` takes `&mut self` and the returned `TermDamage` holds that borrow** — you cannot
   read the grid while iterating damage (`E0502`). Collect into a `Vec<LineDamageBounds>` first,
   then `reset_damage()`, then `grid()`.
2. **`damage()` never returns "nothing".** With zero input since the last `reset_damage()` it still
   returns `Partial` yielding `(0, 0, 0)`, because it unconditionally calls `damage_cursor()`.
   **Empty damage cannot be used as the "no changes" signal** — the daemon needs its own no-op test
   (per-line content hashes over the shadow copy).
3. **Scrolling ⇒ `TermDamage::Full`**, as does any write while `display_offset != 0`. Confirmed by
   probe and by a source comment.
4. **Damage is excellent for non-scrolling workloads**: typing 300 chars → 0 full frames, 1.01
   damaged lines/frame; cursor-addressed TUI repaint → 0 full frames, 2.00 lines/frame. So it is a
   genuinely good fast path that fails only on scroll.

`LineDamageBounds.line` is **viewport-relative**; `TermDamageIterator::next()` adds `display_offset`.
User-controlled elements (Vi cursor, `Selection`) are explicitly **not** in damage state.

**Line conventions confirmed by running it**: `Line(0)` is the top of the visible viewport,
**negative lines are history**, `Line(-1)` is the most recent scrolled-off line, `Line(-history_size)`
the oldest retained. Read history via `grid[Line(n)]` for `n` in
`topmost_line().0 ..= bottommost_line().0`.

**API churn is low in the grid area** — `damage()`, `reset_damage()`, `renderable_content()` have
identical signatures across 0.24.2, 0.25.x and 0.26.0. Breaking changes are concentrated in
`tty`/`event`, which is precisely the surface the daemon uses most: 0.26.0 changed
`ChildEvent::Exited`/`Event::ChildExit` to carry `ExitStatus` instead of `i32`; 0.25.0 renamed
`Options::hold` → `drain_on_exit`; 0.24.0 made `Term` unfocused by default. **Budget for tty/event
churn, not grid churn.**

**`Term<T>` is `Send` when `T: Send`, but not `Sync`.** Use `alacritty_terminal::sync::FairMutex`
per session — it exists precisely to stop a high-frequency PTY writer starving the renderer.

**`EventListener` events must be routed in two directions**, which is easy to get wrong:
`Title`/`ResetTitle`/`Bell`/`ChildExit`/`ClipboardStore` → forward to the client as control-plane
messages. `PtyWrite`/`ColorRequest`/`TextAreaSizeRequest` → **the daemon must answer these itself by
writing back to the PTY.** Routing them through the client would add a round trip to
terminal-internal handshakes.

### R8.5 Flow control: depth-1 dirty flag, fixed tick, server-held cursor

Mosh and wezterm converged independently on the same mechanism:

> **The server holds a per-client acknowledged position and always diffs from that position to
> *now*, never queuing intermediate frames.**

**Never queue frames; queue the *intent* to send a frame.** Per client, a depth-**one** dirty
signal (`tokio::sync::watch`, or `Notify` + an `AtomicBool`):

- The PTY reader sets `dirty = true`. It never allocates, never encodes, and cannot block.
- The framer wakes on the tick and diffs from the client's last-known state **to *now***, not to
  some intermediate state.
- If the client's socket is not writable, the framer **does not queue a second frame** — it leaves
  `dirty` set and returns. When the socket drains, the frame it then builds reflects the *current*
  screen, and every intermediate state is skipped for free.

**This delivers FR-015 structurally: a slow client's next frame is always the current screen.**
Latency is bounded by one frame-build, never by queue depth; memory by one in-flight frame per
client. It is the same reason Mosh delivers Ctrl-C in one RTT, and it is structurally unavailable
to byte-stream protocols like tmux control mode — whose only options when a client lags are to
discard data (`%pause`, corrupting the screen) or disconnect it at `CONTROL_MAXIMUM_AGE`.

**Fixed ~60 Hz tick aligned to the client's render cadence — not RTT-adaptive.** Mosh's
`send_interval() = clamp(SRTT/2, 20ms, 250ms)` exists because SRTT is 10–300 ms and unknown; on a
Unix socket RTT is ~50 µs and carries no information, so adapting to it is noise-fitting. The real
constraint is the GUI's frame budget. Keep Mosh's *sub-frame collection interval* idea: after
`dirty` fires, wait out the remainder of the tick rather than sending immediately, so a 3-byte write
does not cost a whole frame.

**No credit/ack accounting on the hot path.** For convergent state the correct backpressure is
*dropping stale frames*, not slowing the producer — which the depth-1 flag already does. Credit
schemes couple render latency to client scheduling and reintroduce the lag being avoided.

**Follow wezterm on seqno ownership: the client never sends one.** The server holds a per-client
cursor, so a slow client needs no protocol participation to get correct coalescing, and per-client
isolation means one slow client cannot degrade a fast one.

**Diverge from wezterm in two places**: its server→client channel is `unbounded` with `try_send`, so
a client that stops reading grows it without limit — bound it (the depth-1 flag does). And **skip
its 20 ms → 30 s backoff poll**: that exists because wezterm must work over TCP/TLS to a remote host
where liveness is genuinely in question. On a Unix socket, EOF/`ECONNRESET` is immediate and
definitive. Keep a much lazier **~30 s keepalive** purely to reap half-dead peers (suspend/resume,
container pause — FR-026), never as a render path.

⚠️ This supersedes the earlier ack-gated/8 ms-coalescing sketch: the two reports converge on the
same *convergence* property, but the refined recommendation is a fixed tick with no ack on the hot
path, which is simpler and has fewer failure modes.

**Resnapshot instead of diffing** when: attach or client-requested resync; **resize** (the grid
reflows, line identities change); **alt-screen switch** (`Term` swaps grids wholesale);
`|dirty| > 0.6 × viewport_rows`; or `acked_gen` older than retained shadow history. The threshold is
easy to set from the measurements: a full 80×24 RLE frame is 2,353 B and one line is 122 B, so a
full snapshot costs about 19 changed lines. **Full snapshots are cheap enough to be the safety valve
anywhere there is doubt** — 60 Hz of full 200×50 frames is only ~660 KB/s.

**GC**: steal Mosh's `throwaway_num` — each frame states the oldest generation the daemon will still
diff from; discard shadow state older than the minimum `acked_gen` across attached clients. Bounds
daemon memory deterministically rather than hoping clients keep up.

**One asymmetry that is expensive to fix later: screen state is lossy/convergent, but input must be
a lossless append-only log.** Never coalesce or drop keystrokes. Different direction, different
semantics, same transport. This is the FR-020 + edge-case ("input typed before detach must not be
lost or reordered") requirement.

### R8.6 Wire grid type

Three gotchas that shape it:

1. **`CursorShape`/`CursorStyle` do NOT derive `Serialize`** (unlike `Color`/`Rgb`/`NamedColor`/
   `Flags`/`Cell`). Define a mirror enum. Cursor *visibility* is encoded as `CursorShape::Hidden`,
   not a separate bool.
2. **`Grid<T>` derives `Serialize` but `cursor`/`saved_cursor` are `#[serde(skip)]`** — serializing
   the grid wholesale yields no usable cursor. Do not ship alacritty types directly.
3. **Wide chars occupy two cells** — the char is in the `WIDE_CHAR` cell and the next is a
   `WIDE_CHAR_SPACER` whose `c` is meaningless; `LEADING_WIDE_CHAR_SPACER` handles straddling the
   right margin. **Preserve the spacer convention** rather than stripping it, or the client loses
   column alignment the daemon already computed.

Compactness techniques in order of measured payoff: (1) RLE style runs with text packed into a
`String` — this is where the 15× lives; (2) intern styles per frame into a `Vec<Style>` palette with
`u16` indices; (3) hoist rare per-cell data (`zerowidth`, `underline_color`, `hyperlink`) behind
`skip_serializing_if`, mirroring alacritty's own `Cell::extra: Option<Arc<CellExtra>>`; (4) ship
`Flags::bits()` and `TermMode::bits()` as raw integers, no translation; (5) lz4 above a threshold.

**Do not intern strings across frames** — it couples daemon and client state, breaks
resnapshot-on-attach, and the measured win over per-frame interning + lz4 is small.

### R8.7 ⚠️ `alacritty_terminal` has no stable row index — the one real gap

Unlike wezterm's `wezterm-term` (which has `StableRowIndex` and `phys_to_stable_row_index` built in),
alacritty's `Line` is viewport-relative and shifts under scrolling. There is **no public counter of
lines ever scrolled off** — and R8.1(b) shows the entire streaming efficiency rests on having one.

Lines enter history at exactly **one** call site, `Grid::scroll_up()` in `grid/mod.rs`, guarded by
`if region.start == 0 { self.increase_scroll_limit(positions); }`.

| Option | Assessment |
|---|---|
| **(a) Vendor a ~3-line patch** (recommended) | Add `pub scrolled_total: u64` to `Grid`, increment beside `increase_scroll_limit`. Then `abs_id(Line(l)) = scrolled_total + history_size() + l`. Single choke point; surrounding code stable across 0.24–0.26. Use `[patch.crates-io]` with a git fork. |
| (b) Track `history_size()` deltas per parse batch | No fork, but **breaks permanently once scrollback saturates** — a few seconds of `cat` at the default 10,000 lines. Prototype only. |
| (c) Decorate `vte::ansi::Handler` | Reliable in principle but requires replicating scroll-region logic. More code, more risk. |

⚠️ **The agent flagged this as its main unverified area**: "cheap to rebase" is an inference from
code stability, not from having actually rebased such a patch across an alacritty release. **This is
a real project risk** — a vendored fork of the VT engine is a maintenance commitment, and it should
be an explicit, visible decision rather than an implementation detail.

### R8.8 Scrollback range requests

Use **monotonic absolute line IDs**, not relative indices, for three reasons: correctness under
concurrent trimming (a trimmed range becomes *detectably* gone rather than silently wrong content);
scrolling becomes free (the 11× above); and it is what the mature implementations do.

```rust
// client -> daemon
ScrollbackRequest { session, req: u64, ranges: Vec<Range<LineId>> }
// daemon -> client
ScrollbackResponse { session, req: u64, oldest_available: LineId, newest: LineId, lines: Vec<WireLine> }
```

**Every `Frame` also carries `oldest_available`**, so the client learns the trim watermark
continuously and can evict its cache and clamp its scrollbar without asking.

Requests are **advisory, never errors** — a range wholly below the watermark returns empty plus the
watermark, and the client clamps. No error path, no retry storm. Because history lines are
**immutable once scrolled off**, `LineId → content` is a permanent mapping the client can cache
indefinitely. That immutability is what makes the whole scheme simple, and it is only available
because IDs are absolute.

Two refinements worth taking from wezterm: **`bonus_lines`** (speculatively inline the cursor's line
and lines just outside the viewport in the scroll direction — kills a round trip for a few hundred
bytes), and **degrade-to-stale** (cache entries are a small state machine
`Line | Fetching | LineAndFetching(old, at) | Stale(old)` so an in-flight fetch keeps old content
renderable instead of blanking, with a fetch rate limit).

---

## Consolidated decisions

| # | Concern | Decision |
|---|---|---|
| 1 | Transport | `interprocess` 2.4.2 + `tokio` feature, `GenericFilePath` names |
| 2 | Endpoint | Linux `$XDG_RUNTIME_DIR/micold/` (verified `/tmp/micold-<uid>/` fallback); macOS `$HOME/.micold/run/d.sock`; Windows `\\.\pipe\Micold.Daemon.<sid>` + explicit protected DACL |
| 3 | Single instance | Unix: connect-test → lock → **re-check** → unlink → bind, lock held for life. Windows: none needed (`FILE_FLAG_FIRST_PIPE_INSTANCE` is atomic) |
| 4 | Daemon spawn | `setsid` via `pre_exec` (no double fork); `DETACHED_PROCESS \| CREATE_NEW_PROCESS_GROUP`; detect job posture before requesting breakaway |
| 5 | Socket activation | `listenfd` 1.0.2 unconditional dep, Linux-gated call path, `TryFrom<OwnedFd>`. **Opportunistic, never required** |
| 6 | PTY | Stay on `portable-pty` 0.9. Ignore `kill()`'s `Result` on Windows or vendor PR #7709 |
| 7 | Exit detection | Blocking `wait()` in the reader thread that already exists. Delete the `try_wait()` UI poll. No crate, no pidfd |
| 8 | Tree kill | Unix: free via portable-pty's `setsid` → `killpg`. Windows: one job object per session via `windows-sys` 0.61.2, assigning **only** the shell child |
| 9 | Interrupt | Write `0x03` to the master on both platforms. Never `GenerateConsoleCtrlEvent` |
| 10 | Busy/idle | **Claude Code hooks over HTTP.** Transcript JSONL as degraded fallback. Nothing PTY-derived. Missing hooks ⇒ *unknown*, never *idle* |
| 11 | Logging | `tracing` + `tracing-subscriber`, `file-rotate` 0.8 for the hard size cap (`tracing-appender` cannot bound disk), `JOURNAL_STREAM`-verified context detection |
| 12 | Packaging | Ship user units, do **not** enable at install; the GUI client enables in-session. Manual `cargo-deb` assets |
| 13 | Logout survival | Linux: `enable-linger` **then** spawn (not retroactive). macOS/Windows: out of scope, kept open by treating install as a separate fallible step |
| 14 | Streaming model | Depth-1 dirty flag, fixed ~60 Hz tick, diff last-known→now, never queue frames. Server-held per-client cursor; client sends no seqno. ~30 s keepalive only |
| 15 | Diff keying | **Stable absolute `LineId`** — 11× measured win under scroll. Requires a vendored ~3-line `alacritty_terminal` patch (R8.7) |
| 16 | Damage usage | `Term::damage()` as a *filter* to avoid hashing untouched rows, **never as truth** — it returns `Full` on every scroll and is never empty |
| 17 | Wire format | JSON control plane + `postcard` grid frames in one framed stream with an encoding tag byte; `MICOLD_WIRE=json` debug override. **Not bincode — it is a dead tombstone crate** |
| 18 | `alacritty_terminal` | Upgrade 0.25 → **0.26.0**; only child-exit handling changes. `Arc<FairMutex<Term<T>>>`, never hold the lock across `await` |

## Open items requiring a decision in the plan

1. **MSRV**: bump to 1.89 for std `File::lock`, or take `fd-lock` 4.0.4. (R1.5)
2. **FR-016b must be amended** — it mandates a quiescence threshold that measurement shows cannot
   work. Replacement is Claude Code hooks. (R4)
3. **FR-029's macOS endpoint guidance must be amended** — application-support does not fit in 103
   bytes. (R1.2)
4. **Restart FSM has no time window**, so a slow crash loop never trips the guard — pre-existing,
   but FR-005 moves it into an unattended context where it matters more. (R0.4)
5. **Workspace split vs feature split** for the crate topology. (R7)
6. **Vendoring an `alacritty_terminal` patch** for the stable line counter. This is a maintenance
   commitment on the VT engine and must be an explicit, visible decision, not an implementation
   detail. The no-fork alternative breaks permanently once scrollback saturates. (R8.7)
7. **`alacritty_terminal` 0.25 → 0.26.0 upgrade** — low risk (only child-exit handling changes) but
   it is a prerequisite, not incidental. (R8.4)

## Unverified — do not treat as settled

| # | Item |
|---|---|
| 1 | `parity-tokio-ipc` and `tarpc` versions/maintenance — never checked (R1) |
| 2 | Windows SDDL mask `0x12019b` — derived, not quoted (R1.3) |
| 3 | `ERROR_ACCESS_DENIED` as the breakaway failure code — absent from MSDN (R1.6) |
| 4 | Windows spawn/job code — compile-verified against `windows-sys` 0.61.2, **never executed on Windows** (R1.6, R3.3) |
| 5 | `0x03` → ConPTY interrupt path — well-sourced, not empirically tested (R3.4) |
| 6 | macOS LaunchAgent `LimitLoadToSessionType=Background` surviving logout — **worth a half-day test; would materially cheapen macOS** (R3.5) |
| 7 | Claude Code `Notification` hook subtypes — did not fire during testing (R4.3) |
| 8 | `cargo-deb` user-unit behaviour — undocumented, not proven broken (R5.1) |
| 9 | `RuntimeMaxSec` default / absence of systemd idle-exit — from documented defaults, no man page fetched (R5.3) |
| 10 | `listenfd`'s skipped `LISTEN_PID` check (R5.5) |
| 11 | wezterm "no max PDU size" — absence claim from grep (R6.3) |
| 12 | `windows`/`windows-sys` 0.62.2/0.61.2 are ~9 months stale — re-check before pinning (R1) |
| 13 | The `Grid::scroll_up` patch was **not written or rebased** — "cheap to rebase" is inferred from code stability across 0.24–0.26, not experience (R8.7) |
| 14 | Benchmarks are **single-machine, single-run**, rustc 1.97.0 `--release`, no statistical treatment. The workload modelled is mostly-ASCII with 3 coloured spans/line; heavy-emoji, full-RGB syntax highlighting, or box-drawing TUIs will shift absolute numbers. **Ratios should hold; re-measure before locking sizing decisions** (R8.1) |
| 15 | **iced-side rendering cost is entirely unexamined.** If the client cannot repaint at 60 Hz, the tick rate is the wrong knob and this analysis does not reveal that (R8.5) |
| 16 | The ~60% resnapshot threshold is derived from byte measurements, not tuned against a real client (R8.5) |
| 17 | **Alt-screen switching was not probed.** It almost certainly needs a generation bump and full resnapshot — worth a quick test (R8.5) |
| 18 | bincode's status is ambiguous *upstream*: crates.io says "no further releases", the archived GitHub README points at sourcehut. The `compile_error!` tombstone was verified firsthand; either way nothing further ships to crates.io (R8.2) |

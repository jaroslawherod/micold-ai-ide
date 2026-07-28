---
description: "Task list for Daemon-Backed Session Persistence"
---

# Tasks: Daemon-Backed Session Persistence

**Input**: Design documents from `/specs/010-daemon-session-persistence/`

**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅ (protocol, messages, hooks)

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE). Every user story writes
failing tests before implementation. The daemon is headless-testable (no iced), which is a strict
improvement over today's zero coverage of `spawn_pty`, the reader thread, and `handle_process_exits`.

**Cross-platform**: Principle VI. Platform-specific behaviour is confined behind two abstractions —
endpoint location (`endpoint.rs`) and process supervision (`platform/`). Core logic stays
platform-agnostic. Windows is compile-verified only until W5's CI gate (Risk 3).

## Structure (workspace split — plan Decision 3)

```
crates/micold-core/    # render-free; NO iced, NO portable-pty. FR-040 enforced by the compiler.
crates/micold-daemon/  # binary micold-daemon; PTY/VT stack; NO iced.
crates/micold-client/  # binary micold-ai-ide; iced; cannot name a PTY type.
```

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US7 for user-story phases only; Setup/Foundational/Polish carry no story label

---

## Phase 1: Setup (Shared Infrastructure) — plan W0

**Purpose**: Convert to the workspace, establish the compile-time boundaries, resolve the two settled
prerequisites (MSRV, stable line IDs). Everything downstream depends on this.

- [X] T001 Convert repo root to a Cargo workspace in `Cargo.toml` (`[workspace]`, `resolver = "2"`, members `crates/micold-core`, `crates/micold-daemon`, `crates/micold-client`); create the three crate manifests. **Done**: shared `[workspace.package]` (version/edition/MSRV) + `[workspace.dependencies]` as the single source of version truth; `[workspace.lints]` clippy. `deb` metadata moved to `micold-client`.
- [X] T002 Move existing render-free modules into `crates/micold-core/src/`; move `main.rs` + client-only modules + `ui/` into `crates/micold-client/src/`. **Done** via `git mv` (history preserved). **Boundary decided from the real import graph (user chose "lean core"):** core = the domain both binaries share — `session, store, workspace, git, worktree, settings, project, naming, metadata, fs_scan, provider, selector`, **plus `theme`** (needed by core `settings`) **and `terminal`** (the render-free `TerminalBackend`/`LaunchSpec` trait layer — its alacritty/PTY *impl* actually lives in `ui/terminal.rs` and migrates at T030). client = `main, app, keymap, icons, tokens, motion, ui/**`. `ClosingOverlay` relocated from the binary into `micold_client::app`. The 44 test files + `tests/support/` were redistributed to each crate's `tests/`; all import paths (`micold_ai_ide::` → `micold_core::`/`micold_client::`/`crate::`) and asset/`include_bytes!`/doctest paths rewritten.
- [X] T003 In `crates/micold-core/Cargo.toml` declare only render-free deps (`serde`, `serde_json`, `uuid`, `directories`, `postcard`) — **no iced, no portable-pty, no alacritty**; this manifest IS the FR-040 enforcement. **Verified**: `cargo tree -p micold-core` shows zero iced/PTY/alacritty in the dependency tree.
- [X] T004 Bump `rust-version` to **1.97** (latest stable, the installed toolchain) in the workspace manifest; deps pulled to current versions (Decision 1). `File::lock` confirmed available from std (no `fd-lock`); `clippy.toml` MSRV aligned to 1.97.
- [X] T005 **~~DEFERRED to T030.~~** Upgrade `alacritty_terminal` 0.25 → 0.26.0 and adapt child-exit handling (`ChildEvent::Exited(ExitStatus)`). Rationale: the alacritty/`ChildEvent` code still lives in `micold-client/src/ui/terminal.rs` (moved untouched, still on 0.25); the daemon has no terminal stack until T030. Bumping now would rewrite code that T030 relocates — the upgrade lands with that move. Client + workspace pin alacritty 0.25 in the interim (one version, no drift). **Done via T105 (2026-07-27)**: the deferral outlived its target — T030 closed and the upgrade did *not* land with it, leaving the plan (which pins 0.26.0) and the code (0.25) silently disagreeing until `/speckit-converge` caught it. The real break was `Event::ChildExit(i32)` → `Event::ChildExit(ExitStatus)` in the daemon, not `ChildEvent` in the client as predicted here. See T105 for the adaptation and verification.
- [X] T006 Define the stable-line-ID seam behind a `LineIdSource` trait in `crates/micold-daemon/src/terminal.rs`. **Done**: `trait LineIdSource { line_id(offset) -> LineId; oldest_available() -> LineId }` with the no-fork approximation `ApproxLineIds` (per plan Decision 2's mitigation — a line keeps its id as it scrolls, derived from a monotonic `total_lines` watermark minus `retained`, unit-tested for stability + monotonicity). The vendored VT fork (T005, still deferred) can swap in behind this trait without touching the framer. **T005 (alacritty 0.26 upgrade + vendored patch) remains deferred**: the daemon runs on 0.25 with the approximation, which is exactly the swappable-fallback the plan called for; the fork is a later spike gated on measured need (Risk 1). **Correction (T105, 2026-07-27)**: the version half of that sentence is now stale — the daemon runs on **0.26.0**. The *vendored patch* half still stands: the fork was never taken, `ApproxLineIds` is still what ships behind this trait, and it remains a later spike gated on measured need.
- [X] T007 Verify `cargo build --workspace` and `cargo test --workspace`. **Done, all green**: `cargo test --workspace` → **294 passed / 0 failed** (both crates' unit + integration + doctests); `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all --check` clean. **Baseline for T079**: pre-split render-free `--no-default-features` = 220; documented full suite = 259; post-split workspace total (incl. per-crate doctests) = 294. `mise.toml` + `.github/workflows/ci.yml` updated from the removed `--features gui`/`--no-default-features` model to the workspace model.

**Checkpoint**: ✅ Three crates compile; `micold-core`'s dependency tree contains no iced/PTY/alacritty — the render-free boundary is a compile error, not a convention. **T005/T006 deferred to T030** (their target code doesn't exist until the daemon terminal stack is carved out).

---

## Phase 2: Foundational (Blocking Prerequisites) — plan W1 + W2

**Purpose**: The wire protocol, transport, single-instance daemon, and durable-state ownership. No
user story can begin until a client can connect to a daemon and read catalog state. This is the
re-architecture's spine.

**⚠️ CRITICAL**: Blocks ALL user stories.

### Protocol types (micold-core) — contracts/protocol.md, messages.md

- [X] T008 [P] Define the framing envelope (length prefix, encoding tag, kind byte) in `crates/micold-core/src/protocol/envelope.rs` per protocol.md §3. **Done**: `EnvelopeHeader` (`encoding`/`kind`/reserved) with `to_bytes`/`parse`; `Encoding` (Json/Postcard/PostcardLz4), `Kind` (Control/Grid); `MAX_FRAME_LENGTH = 16 MiB` and `HEADER_LEN = 4`; a non-zero reserved field and unknown tags are rejected as specific `EnvelopeError`s, never silent defaults.
- [X] T009 [P] Define `ClientMsg` / `DaemonMsg` enums and all message structs in `crates/micold-core/src/protocol/messages.rs` per messages.md (Hello with `schema_hash`, Attach/Detach, session commands, mutating requests, Welcome/Refused/Displaced, operation results). **Done**: full surface incl. `SessionSummary`, `CatalogSnapshot`/`ProjectSnapshot`/`WorktreeSnapshot`, `DaemonSettings`, `RefusalReason`, `OperationResult`, `ErrorKind`, `LogSink`/`LogEntry`, `ExitStatus`, `ActivitySignal`. Wire lifecycle carried by a dedicated `WireLifecycle` (adds `InterruptedResumable` + `Failed{reason,attempts}` the in-process `SessionLifecycle` lacks — domain↔wire mapping deferred to T073). `SessionId`/`SessionLabel` gained `Serialize`/`Deserialize` (additive).
- [X] T010 [P] Define wire grid types (`GridFrame`, `WireLine`, `StyleRun`, `WireStyle`, `WireColor`, `WireCursor`, `CellExtras`) in `crates/micold-core/src/protocol/grid.rs` per messages.md, preserving the RLE + per-frame palette representation rules. **Done**, incl. `LineId(i64)` and `WireCursorShape`. **Correctness note**: `skip_serializing_if` is deliberately omitted — the same type serializes under both JSON and `postcard` (non-self-describing), so a skipped field would desync the decoder and break the byte-identical round-trip T012 asserts. Structural sparseness (empty `Vec` = 1 length byte, `None` = 1 tag byte) delivers the size win instead.
- [X] T011 Define `PROTOCOL_VERSION: u32` and add `build.rs` to `crates/micold-core/` producing `const SCHEMA_HASH: [u8; 32]` by hashing the canonical text of `protocol/{messages,grid,envelope}.rs` (Decision 4); expose both from `protocol/version.rs`. **Done**: `build.rs` `include!`s `protocol/hashing.rs` (dependency-free SHA-256 + canonicalisation) so the generator, the crate, and the guard test share **one** hash implementation — no build-deps, no drift. `version.rs` `include!`s the generated array; `cargo:rerun-if-changed` tracks all four source files.

### Protocol tests FIRST (Principle I)

- [X] T012 [P] Test in `crates/micold-core/tests/protocol_roundtrip.rs`: every `ClientMsg`/`DaemonMsg` round-trips under JSON and every grid type round-trips under `postcard`; a `GridFrame` survives encode→decode byte-identical (wide-char spacer + zerowidth preserved). **Done** (6 tests, all 25 `ClientMsg` + 17 `DaemonMsg` variants + envelope header exercised).
- [X] T013 [P] Test in `crates/micold-core/tests/schema_hash.rs`: editing a message struct changes `SCHEMA_HASH` (guards the guard); a version-only bump also changes the handshake tuple. **Done** (6 tests): baked hash matches a recompute over the real source; a struct edit changes it; a comment-only edit does not; a version bump changes the `(version, hash)` tuple.
- [X] T014 [P] Test in `crates/micold-core/tests/handshake.rs`: version mismatch OR schema-hash mismatch both refuse with both sides' version + hash named (FR-021/022). **Done** (4 tests) via `protocol::handshake::evaluate`.

**Sub-checkpoint (protocol types)**: ✅ T008–T014 complete. The wire surface, framing envelope,
schema-hash guard, and strict handshake are defined in `micold-core` and covered by 16 tests. The
single-implementation schema hash means two builds that disagree about the wire necessarily refuse
each other.

**Sub-checkpoint (transport + single-instance)**: ✅ T015–T020 complete. The shared framing codec
(`micold-core`), per-OS endpoint policy + single-instance startup (connect → lock → **re-check** →
bind → hold-for-life) + the tokio accept loop with the strict handshake (`micold-daemon`) are in and
covered by 11 more tests (workspace total 294 → 321, zero regressions; clippy + fmt clean; core still
iced/PTY-free). A client can now open a connection and complete the version+schema handshake against a
real daemon. **Two scoped deferrals**, both recorded on their tasks: the Windows named-pipe DACL →
T083/W5 (needs the Windows CI gate to validate), and systemd fd adoption is Linux-only + opportunistic
by design.

**Sub-checkpoint (state ownership + routing)**: ✅ T021–T024 complete. The daemon now owns the durable
catalog as its single writer (adopted in place — the on-disk shape is unchanged), routes
attach/detach/viewed-session with one-attachment-per-project + forced takeover, pushes
`SettingsChanged`/`CatalogChanged` to every connected client, and honours the never-exit-while-a-session-
lives rule. 12 more tests (workspace total 321 → 333, zero regressions; clippy + fmt clean; core still
iced/PTY-free). **Remaining in Phase 2: T025** (tracing/logging), **T026** (client connection layer),
**T026a/T026b** (client auto-spawn + its test) — after which a client auto-spawns a daemon, handshakes,
and sees live catalog state, and the user stories can begin.

### Transport + framing (micold-daemon, micold-client) — plan W1

- [X] T015 Implement the `interprocess` 2.4.2 (tokio) transport with `LengthDelimitedCodec` + explicit `max_frame_length` and the hybrid JSON-control / `postcard`-grid encoder honouring `MICOLD_WIRE=json`, shared via `crates/micold-core/src/protocol/codec.rs`. **Done**: role-parameterised `WireCodec<In, Out>` (aliases `DaemonCodec` reads `ClientMsg`/writes `DaemonMsg`, `ClientCodec` the mirror) implementing `tokio_util::codec::{Encoder, Decoder}` over a `LengthDelimitedCodec` (u32 LE, 16 MiB cap). Control is always JSON; grid is postcard unless `MICOLD_WIRE=json`. `CodecError` variants are all specific (Io/Envelope/ControlNotJson/Json/Postcard). Core gained `tokio-util` + `bytes` (still no iced/PTY — FR-040 verified).
- [X] T016 Implement per-OS endpoint policy with the macOS `sun_path` length assertion (FR-029a → `$HOME/.micold/run/d.sock`), `/tmp` fallback ownership verification, and Windows named-pipe DACL in `crates/micold-daemon/src/endpoint.rs`. **Done for Unix** (Linux XDG w/ sticky bit + `/tmp/micold-<uid>` fallback verified via `symlink_metadata` + `uid==geteuid` + mode `0o700`, bailing loudly; macOS `$HOME/.micold/run/d.sock` with the 103-byte `sun_path` assertion). **Windows DACL deferred to T083/W5**: `resolve()` returns `Unsupported` with a clear message — the protected-DACL construction needs `windows-sys` `LookupAccountName` and can only be validated on the Windows CI gate. Recorded as a known limitation.
- [X] T017 Implement the single-instance sequence — connect-test → `File::lock` → **RE-CHECK connect** → unlink → bind → hold lock for process lifetime (R1.4) — in `crates/micold-daemon/src/singleton.rs`. **Done** exactly per protocol.md §2: `std::fs::File::try_lock` (→ `TryLockError`), the mandatory re-check, `S_ISSOCK`-guarded unlink, and the lock held in `BoundListener` for the daemon's lifetime (`Drop` unlinks on clean shutdown). Windows uses the atomic `create_tokio` create-or-fail path.
- [X] T018 [P] Test in `crates/micold-daemon/tests/daemon_singleton.rs`: two simultaneous starters converge on one daemon; a stale socket is reclaimed; a wrong-owner parent dir causes a loud bail, not a silent bind (Edge: stale endpoint, startup race). **Done** (3 tests; the wrong-owner bail is covered by the `endpoint::verify_owned_0700` unit test).
- [X] T019 [P] Test in `crates/micold-daemon/tests/framing.rs`: a frame exceeding the cap is rejected loudly; JSON and postcard frames interleave on one stream in total order (messages.md §1). **Done** (4 tests; over-cap rejected on both encode and decode; `MICOLD_WIRE=json` grid path also covered).

### Daemon skeleton, state ownership, lifecycle (micold-daemon) — plan W2

- [X] T020 Implement daemon startup/bind/systemd-fd adoption (`listenfd`) and the tokio accept loop in `crates/micold-daemon/src/main.rs`. **Done**: daemon is now a lib+bin; `server::run` resolves the endpoint, runs `singleton::acquire` (or exits if a daemon already owns it), and serves each accepted connection via a stream-generic `serve_connection` that speaks the strict handshake (`Hello` → `Welcome`/`Refused`) and answers `Ping`/`Goodbye`. Linux systemd socket activation is adopted opportunistically (`listenfd`, `set_nonblocking(true)`), never required. End-to-end handshake covered by `tests/handshake_flow.rs` (2 tests). Catalog/attach/streaming layer on in T021–T022.
- [X] T021 Implement the Catalog as the single writer of durable state (projects, worktrees, sessions, settings), adopting existing `projects.json`/`settings.json` in place (FR-008, FR-012) in `crates/micold-daemon/src/catalog.rs`. (External-modification detection is out of scope — see spec Out of Scope.) **Done**: wraps the existing `micold-core` stores so the on-disk shape is unchanged — only the writer changes. Provides `snapshot()` → `CatalogSnapshot`, `sessions_for()`, `settings_wire()`, clamped `set_scrollback()`, atomic `persist()`, and surfaces `LoadStatus` (C4 `Recovered` is now reported rather than swallowed). Worktree entries in the snapshot are derived from the durable knowledge (display-name overrides + session bindings); live git branch/status arrives with the worktree RPCs (T053). **Main-sync (2026-07-23, main `93a0a08`/`7dc9c8a`)**: core `Session` gained an `archived` flag (anti-resurrection — a deleted worktree's / removed session's record is kept but marked, so reconciliation can't resurrect it). `sessions_for` now **filters out archived sessions**, since the catalog snapshot is the single source clients render; covered by `catalog_adoption::archived_sessions_are_excluded_from_the_snapshot`. The new per-project storage-fault isolation (main `93a0a08`, `project_state_path`) — and its `d88c7a1` refinement (write per-project state *before* the catalog, keeping a catalog fallback copy only for projects whose own write just failed, via `skip_serializing_if`-empty) — are inherited transparently through the `ProjectStore` seam (unchanged trait surface); `Catalog::persist()` calls `store.save()`, so this lossless-migration guarantee holds for the daemon writer for free. Covered by `micold-core`'s carried `store_fault_isolation::migrating_project_whose_state_write_fails_keeps_a_catalog_fallback`.
- [X] T022 Implement `Attach`/`Detach`/`SetViewedSession` routing and `CatalogChanged`/`SettingsChanged` push projection to all connected clients (FR-011) in `crates/micold-daemon/src/main.rs`. **Done** in `state.rs` + `server.rs` (the daemon is a lib+bin, so the routing lives in the library where it is testable). `DaemonState` holds the catalog, a client registry, and per-project attachments; each connection gets a writer task draining an unbounded channel, so a push from *another* connection reaches this one. Attach is exclusive with a `ProjectBusy` refusal naming the holder, `force` displaces (the displaced client is notified, never terminated), and disconnect releases every attachment the client held (T2). `broadcast`/`broadcast_catalog`/`set_scrollback` implement the push projection. The state mutex is never held across an `.await`.
- [X] T023 Implement the "never exit while a session is alive" lifecycle rule and the zero-sessions-zero-clients permitted-exit (FR-002) in `crates/micold-daemon/src/lifecycle.rs`. **Done**: pure `may_exit(live_sessions, connected_clients)` predicate plus an atomic `Lifecycle` counter tracker wired into client connect/disconnect. Session counters are driven by the supervisor at T031.
- [X] T024 [P] Test in `crates/micold-daemon/tests/daemon_lifecycle.rs`: daemon stays up with one live session and no clients; may exit at zero/zero; a catalog mutation reaches a second connected client without user action (FR-002, FR-011). **Done** (5 tests, end-to-end through the real `serve_connection` path): all three required assertions, plus attach exclusivity/forced-takeover and attachment release on disconnect. Catalog adoption itself is covered by `tests/catalog_adoption.rs` (5 tests).
- [X] T025 Configure `tracing` + `tracing-subscriber` with `JOURNAL_STREAM`/`IsTerminal` context detection and `file-rotate` hard disk cap in `crates/micold-daemon/src/logging.rs`; **no terminal content in logs** (FR-047). **Done**: sink is *detected*, never assumed — `JOURNAL_STREAM` → undecorated stderr (journald adds its own metadata), else `stderr.is_terminal()` → pretty+ANSI stderr, else (the detached auto-spawn case) → an owner-only (`0o600`) rotating file under the user data dir. Disk is hard-capped by construction (`MAX_LOG_BYTES × (LOG_FILES+1)`, ~15 MiB) since `file-rotate` bounds each file *and* the file count — `tracing-appender` was rejected for being unable to bound total disk. A `reload::Handle` backs runtime `SetLogLevel` (wired in T080). Structured events replaced the `eprintln!` scaffolding in `server.rs`. FR-047 (no terminal content) is a documented invariant asserted by T081.
- [X] T026 [P] Implement the thin client-side connection layer (connect, handshake, catalog cache, reconnect scaffolding). **Done**, placed in `crates/micold-core/src/connect.rs` rather than the client: the connect/handshake primitives are shared verbatim by the client *and* the auto-spawn integration test, so putting them in core makes the client and daemon physically unable to disagree about the dial/handshake contract (the same reasoning that moved `endpoint` into core). Provides `dial`/`handshake`/`connect`/`connect_or_spawn`, `DaemonConnection` (`Framed<Stream, ClientCodec>`), and a `Welcome` carrying the daemon build + catalog + settings snapshot. The client-side `daemon_conn.rs` that *replaces in-process session ownership* is the UI migration and lands with Phase 6 (T058+); this task delivered the reusable connection layer it will build on.
- [X] T026a Implement **client auto-spawn**: when connect finds no daemon listening, spawn a *detached* `micold-daemon` (survives the client process — `setsid`/double-fork on Unix, `DETACHED_PROCESS`/`CREATE_NEW_PROCESS_GROUP` on Windows) behind a per-OS spawn abstraction, then retry connect until the endpoint answers or a timeout elapses. **Done** in `crates/micold-core/src/spawn.rs` (`spawn_detached_daemon` → pid; Unix `setsid` via `pre_exec`, EPERM tolerated; Windows `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`) + `connect_or_spawn` in `connect.rs` (connect → spawn → poll with 10 ms→250 ms backoff up to a timeout). The daemon binary is located via `MICOLD_DAEMON_BIN` → sibling of `current_exe` → bare name. No install step, no external supervisor (FR-003). Closes the SC-003 cold-start path.
- [X] T026b [P] Test in `crates/micold-daemon/tests/autospawn.rs`: from a state with no daemon, a client reaches attached state (spawn → handshake) with no manual command; the spawned daemon outlives the spawning client (FR-003, SC-003). **Done**: a genuine two-process test running the Cargo-built `micold-daemon` binary. It isolates the endpoint by setting `XDG_RUNTIME_DIR` into a tempdir and deriving the path through the shared `endpoint::resolve()` so client and spawned daemon agree by construction; it asserts cold start reaches `Connected::Ready`, then that a *second* client connects after the first drops — proving the daemon outlived its spawner.

**Checkpoint**: ✅ A client starts a daemon, handshakes on version + schema hash, and sees live catalog state. Phase 2 complete — connect/handshake/auto-spawn primitives live in `micold-core`; the daemon detects its log sink and hard-caps disk. Workspace tests green, clippy + fmt clean, `micold-core` still iced/PTY-free (FR-040). User stories can begin.

---

## Phase 3: User Story 1 — Work continues without a window (Priority: P1) 🎯 MVP

**Goal**: Sessions and their processes outlive the UI — close, crash, or rebuild the client and the
session is found exactly where it got to, scrollback covering the whole detached interval.

**Independent Test**: Start a continuously-printing session, close the UI, wait ≥10 min, relaunch,
reattach — session still `Running`, screen current, scrollback gap-free and duplication-free (quickstart S1).

### Tests FIRST (Principle I) ⚠️

- [X] T027 [P] [US1] Integration test in `crates/micold-daemon/tests/session_survival.rs`: a spawned PTY child keeps running and producing output after its owning connection drops (FR-001). **Done**: drives a real PTY child that emits forever with **no client attached**, asserts the grid keeps receiving output and the child stays alive, then reaps it. Connection→session wiring lands in Phase 4; the test validates the FR-001 property at the supervisor layer where it originates (the session is connection-independent by construction).
- [X] T028 [P] [US1] Integration test in `crates/micold-daemon/tests/reattach_snapshot.rs`: on reattach the client receives a full-snapshot `GridFrame` (not a replay) whose scrollback covers the detached interval, bounded by the scrollback limit (FR-014, FR-017). **Done**: frames while "attached", stops framing while output arrives ("detached"), then `frame(force_full=true)` on reattach → asserts `full`, the whole screen, latest detached-interval output present, stale pre-detach top line gone, and a real `oldest_available` watermark with retained history behind the viewport.
- [X] T029 [P] [US1] Test in `crates/micold-daemon/tests/session_isolation.rs` (migrated): two sessions' grids never cross-contaminate (Principle II). **Done**: two real VT sessions each emit a distinct marker; each grid contains only its own. Replaces the deleted in-memory `SessionRouter` byte-routing approximation with an end-to-end check against separate `Term`s (isolation is now structural).

### Implementation — plan W3

- [X] T030 [US1] Split `terminal.rs`: delete `SessionRouter` (zero callers, R0.2); move `Term` ownership into the daemon as one `FairMutex<Term>` per session with a per-session reader thread absorbing the blocking read, in `crates/micold-daemon/src/terminal.rs`. **Done**: `SessionRouter` (only its own test referenced it) removed from `micold-core`; the daemon now owns each session's `alacritty_terminal::Term<DaemonListener>` behind `SharedTerm = Arc<FairMutex<Term>>`. A per-session reader thread absorbs the blocking PTY read and is the sole advancer of the VT parser (research R4). `terminal.rs` also holds the `LineIdSource` seam (T006), the `DaemonListener` (T034), and a `StandardPalette` for color replies. The client's `alacritty` stack stays on 0.25 (T005 upgrade deferred).
- [X] T031 [US1] Implement PTY spawn + child supervision handle (`portable-pty` 0.9, `setsid` free on Unix) in `crates/micold-daemon/src/supervisor.rs`. **Done**: `PtySession` opens a PTY, spawns the child (`spawn_claude`/`spawn_shell`/generic `spawn`), starts the reader thread, and owns child + writer + master + `SharedTerm` + signals. Exposes `write_input`/`resize`/`is_alive`/`kill`/`pid`/`term`/`signals`; `Drop` kills + joins the reader so no thread leaks. **Detached sizing**: the grid size is seeded at spawn and only changed by `resize`, valid with no client attached. **Deferred within this task**: persisting the session UUID (needs catalog wiring, folded into T053) and `SessionResize`-on-attach (needs the attach path, Phase 4).
- [X] T032 [US1] Implement the shadow-diff framer: depth-1 dirty flag, fixed ~60 Hz tick, stable-`LineId` diff via the `LineIdSource` trait, `oldest_available` watermark on every frame, snapshot-on-attach, resnapshot triggers, in `crates/micold-daemon/src/framer.rs`. **Done**: `Framer::frame(term, force_full, input_serial)` reads the live grid, keys each viewport line by a stable `LineId`, and emits either a full snapshot or a delta of only the changed lines. Stable ids come from a **history-only eviction alignment** (`eviction_count`): history lines are immutable once written, so aligning this tick's history hashes against last tick's yields the exact scroll-off count — the screen is excluded because its bottom line is edited in place. Styles are interned per frame; extras (zerowidth/hyperlinks) are hoisted sparse; `generation` bumps on resize / alt-screen toggle / unalignable redraw (forces a full frame); `oldest_available` + `viewport_top` ship on every frame. The fixed-tick/depth-1-dirty driver (a session ticks only when `VtSignals::take_dirty`) wires in with the server streaming path (Phase 4); the dirty flag + `frame()` primitive it drives are done here.
- [X] T033 [US1] Implement bounded scrollback retention at the daemon (service-owned limit FR-012a) with oldest-first discard, applied even while detached, in `crates/micold-daemon/src/framer.rs` (Edge: detached growth). **Done**: retention is a property of the VT `Term` (constructed with a fixed `scrolling_history`), so oldest-first discard happens in the emulator whether or not a client is framing; the framer reports the resulting `oldest_available`. Covered by `slow_client.rs` (`the_watermark_advances_as_evictions_are_observed`, `scrollback_retention_is_bounded_even_while_unframed`): retained lines stay within `cap + screen` under a 20k-line unframed burst, and the watermark advances as evictions are observed.
- [X] T033a [US1] Implement **scrollback-by-range** on the daemon side: `Framer::scrollback_range(term, from, count)` returns contiguous `WireLine`s by id, chunked, with a `more` flag, clamping `from` up to `oldest_available` rather than erroring and returning fewer than requested near the live edge (FR-017; quickstart S10). **Done** in `framer.rs`. **Deferred**: the `ScrollbackRequest`/`ScrollbackResponse` wire round-trip and the client range-fetch-on-scroll in `micold-client/src/ui/terminal.rs` land with the client retargeting (Phase 4, T041–T043) — the daemon-side range reader they call is complete.
- [X] T033b [P] [US1] Test in `crates/micold-daemon/tests/scrollback_range.rs`: a range request returns contiguous lines by `LineId`; a request past the retained watermark clamps; a selection anchored to line IDs is not corrupted by new output arriving mid-scroll (FR-017, FR-018). **Done** (3 tests): contiguous ids starting at the watermark; a request 1000 lines below the watermark clamps up (no error) and one past the live edge returns empty; an id anchored in history resolves to the same text before and after 50 more lines of output (immutability I2).
- [X] T034 [US1] Answer `EventListener` replies (`PtyWrite`, `ColorRequest`, `TextAreaSizeRequest`) daemon-side, never routed to the client (protocol.md §8). **Done**: `DaemonListener` (installed in every `Term`) answers `PtyWrite` by writing straight back to the PTY, `ColorRequest` via the daemon's `StandardPalette` (xterm 256), and `TextAreaSizeRequest` with the session's current `WindowSize` — all serialized on the one shared PTY writer alongside user input. It also captures OSC-0 `Title` (glyph-stripped, T047 groundwork), records `ChildExit`, and raises the depth-1 dirty flag on new content for the framer. None of these reach the client.
- [X] T035 [P] [US1] Regression test in `crates/micold-daemon/tests/scroll_cost.rs`: stable-ID diffing stays ~2 lines/frame under a scrolling workload (the 11× property is load-bearing and must not silently regress). **Done**: fills a 24-row screen, then scrolls 200 single lines asserting every delta is `<= 2` lines and never a full frame; a second test asserts an idle screen diffs to **0** lines. This is the guard that caught the original whole-buffer alignment bug (in-place bottom-line edits forced full frames) and locks in the fix.
- [X] T036 [P] [US1] Test in `crates/micold-daemon/tests/slow_client.rs`: a client that stops reading causes no unbounded daemon growth and converges to the true screen on resume (SC-006; Edge: slow consumer). **Done**: a 20k-line unframed burst then one frame carries **at most one screen** (no backlog replay) and reflects the live bottom; the following frame is empty (converged). Retention-bound tests confirm no unbounded growth. The full per-client channel back-pressure test lands with the server streaming path (Phase 4).
- [X] T037 [US1] User-guide doc: what persistence guarantees hold across close/crash/rebuild, in `docs/daemon.md` (Principle VII, FR-042). **Done**: `docs/daemon.md` explains the client/daemon split and states exactly what survives (close, crash, rebuild+relaunch), what does not (reboot/power loss), bounded scrollback + on-demand history fetch, and why reattach is instant (snapshot, not replay). Attach/detach (US2) and project/worktree ops (US3) sections are stubbed for T049/T057.

**Checkpoint**: ✅ Sessions survive the UI (US1). The daemon owns each `Term`+PTY, streams the grid via the shadow-diff framer with stable line ids (11× scroll property locked by test), serves bounded scrollback + ranges, and answers VT queries daemon-side. Client retargeting to consume these frames + drive input is US2 (Phase 4) — together they are the demonstrable MVP.

---

## Phase 4: User Story 2 — Attach, drive, detach (Priority: P1) 🎯 MVP

**Goal**: Attach to a waiting session, read the current screen, type an answer, watch it proceed,
detach — with local interactions (scroll, select, resize) never blocking on a round trip, and the
session list showing which sessions are working vs awaiting input.

**Independent Test**: With a session blocked on a prompt and no UI, launch UI, answer, observe effect,
close UI; cold start attaches in < 3 s (quickstart S2, S3).

### Tests FIRST (Principle I) ⚠️

- [X] T038 [P] [US2] Test in `crates/micold-client/tests/local_interactions.rs`: scroll, select, resize issue zero round trips (FR-020, SC-004/005). **Done**: 4 tests drive the local interaction stack directly — `GridCache` reads (screen/line/watermarks) for scroll, the pure `Selection` build+extract for select, and a resize generation bump absorbed by the cache — proving each is served from local state with no daemon handle in reach. Includes the FR-018 property (a selection survives new higher-`LineId` output with no re-fetch). Pane resize's single fire-and-forget `SessionResize` (returns `Task::none()`, never awaits) is noted as covered where the outbox is driven, not a round trip.
- [X] T039 [P] [US2] Test in `crates/micold-core/tests/input_ordering.rs`: `SessionInput.serial` is monotonic and input is never coalesced, dropped, or reordered across a detach/reattach boundary (G2; Edge: clock/ordering). **Done**: the contract is a single shared primitive in `micold-core::input` — the client's `InputSeq` (monotonic per-session stamper, never reset across reattach) and the daemon's `InputReceiver` (`accept(serial) -> Apply | Lost{missing} | Stale`). `input_ordering.rs` pins all of it: dense monotonic serials; every in-order serial applied exactly once; a duplicate/reordered serial is `Stale` and never re-applied (no coalescing); a gap is surfaced as `Lost` loudly then resynced; continuity holds across a simulated detach/reattach (same counter, per-session receiver); and a keystroke severed mid-flight at a drop is detected as loss on reattach, never papered over.
  **Reopened (BUG-006, 2026-07-27)**: "same counter" is the defect, not a detail of the fixture.
  `input_ordering.rs:80` states it outright — *"the reconnect touches only the transport: `seq` is the
  SAME counter (never re-created via …)"* — so the suite models a **transport** reconnect and never a
  **client-process restart**, which is the case that re-creates the counter and the ordinary one for a
  daemon designed to outlive its UI. The assumption under test was written into the test's own setup,
  so it passed while every pre-existing session went silently read-only after each UI restart. No test
  anywhere constructs a second `SessionInputStamper` against a surviving `InputReceiver`. The
  properties this task already pins (dense serials, no coalescing, loud `Lost`, in-flight loss at a
  drop) are correct and unaffected; the missing coverage is the restart boundary, added by T112.
  **Re-closed (⚠️ was reopened BUG-006, 2026-07-28)**: T112 added the three restart-boundary tests
  and corrected the `:80` comment, which now says explicitly that the same-counter premise is the
  *reconnect* case only and names the test that covers a client restart. The suite no longer
  certifies a boundary it does not exercise.
- [X] T040 [P] [US2] Test in `crates/micold-daemon/tests/activity_signal.rs`: hooks drive Working/AwaitingInput/Ended; absent hooks yield `Unknown` never AwaitingInput (H1); a spinner-glyph title yields Working only, never a move toward AwaitingInput (H1a). **Done** (as inline tests in `activity.rs` — 9 tests): the happy path (`UserPromptSubmit`/`PreToolUse`→Working, `PostToolUse` no-op, `Stop`/`Notification`→AwaitingInput, `Ended`), **H1** (no hooks → stays Unknown; spinner-only → Working, never AwaitingInput), **H1a** (`SpinnerObserved` acts only from Unknown; a no-op from Working/AwaitingInput/Ended), `Ended` absorbing, and `is_spinner_title` braille detection.

### Implementation — plan W4 + W5 (activity) + W3 (titles)

> **Daemon side of the render/drive switch is ready** (prereq for T041/T042, built ahead of the
> client retarget). The daemon now: (1) **streams grid frames** to a viewing client — `SetViewedSession(Some(id))`
> starts a per-view task that sends a full snapshot then coalesced deltas on VT-dirty, over a
> per-client channel generalised to `Frame<DaemonMsg>` (`server.rs::stream_view`, `state.rs::frame_sender`);
> (2) **accepts input** — `SessionInput` routes through the per-session `InputReceiver` to the PTY;
> (3) **spawns on request** — `SessionStart` (`state.rs::start_session`) resolves cwd/mode from the
> catalog and spawns claude/shell, idempotently. Proven end-to-end over `serve_connection` by
> `tests/stream_view.rs` (view→snapshot→drive→echo, incl. a cold-start `SessionStart`→view→drive)
> and `tests/session_start.rs`. What remains for the switch is purely the **client** consumption below.

- [X] T041 [US2] Retarget `App.terminals` to a daemon handle + per-session wire grid cache; split the ~25 `terminals.get_mut()` sites into fire-and-forget commands vs local-cache queries (FR-020) in `crates/micold-client/src/ui/terminal.rs`. **Daemon-ready** (see the note above): the client needs a connection actor (iced `Subscription` owning the `Framed` `DaemonConnection` + outgoing `mpsc<ClientMsg>`), then on startup `connect_or_spawn` → `Attach` → `SetViewedSession`; drive via the `SessionInputStamper`. **Connection actor landed**: `micold_client::daemon::connection()` is a single long-lived iced `Subscription` that `connect_or_spawn`s the daemon, handshakes, hands the App an `Outbox` (`Message::DaemonConnected`) and pumps both directions — outgoing `ClientMsg`s → socket, incoming `Frame::Control`/`Frame::Grid` → `Message::DaemonEvent`/`DaemonGridFrame`. `Outbox` is `Eq` by `Arc`-token identity so `Message` stays `Eq`. `App` stores `daemon: Option<Outbox>` + `daemon_catalog`; the actor is wired into `subscription()`. Non-breaking/additive — the local PTY still renders until the **render swap** (T042) consumes the grid cache and reroutes input. **Remaining for T041**: retarget `App.terminals` onto the cache, split the `get_mut` sites, and rip out the local PTY together with T042.
- [X] T042 [US2] Retarget `TerminalPane` from `&RuntimeTerminal` to the wire grid cache, keeping its builder shape (`TerminalPane::new(..).focused(..)`) in `crates/micold-client/src/ui/material/terminal_pane.rs`. **Cache core landed**: `micold_client::grid::{GridCache, CachedLine, CachedExtra}` applies `GridFrame`s (snapshot + `LineId` delta upsert), **resolves per-frame interned styles/hyperlinks at apply-time** into frame-independent lines (a later palette can't corrupt cached lines), honours `generation` reset + `seq` staleness + `oldest_available` eviction, and exposes `screen()` (rows × `viewport_top`, `None` for gaps) + `line(id)`/`cursor()`/`mode()`. 8 tests. **Remaining for T042**: feed frames from `Message::DaemonGridFrame` into a per-session `GridCache`, and rewrite `terminal_pane` to render from it (glyphs, styles via `WireStyle.flags`, cursor, selection highlight via `micold_client::selection`). '"'"'- [X] T043 [US2] Implement the client-side selection model + text extraction (new work, R0.3 — selection was a mutation of the shared `Term`) anchored to `LineId` so new output cannot corrupt it (FR-018; Edge: selection under new output). **Done**: `micold_client::selection` — pure logic (imports only `LineId`). `Selection` holds `Anchor { line: LineId, col }` endpoints + a `SelectGranularity { Char, Word, Line }`; `start`/`update` expand+normalize bounds, `contains(line,col)` drives render highlighting, `text(provider)` extracts (multi-line join, per-line `trim_end`). Text ops take an `impl Fn(LineId) -> Option<String>` line provider so the module never owns the grid. FR-018 invariance is structural (absolute-`LineId` anchors), proven by a test that appends new higher-`LineId` output and asserts `contains`/`text` are unchanged. 14 inline tests. **Wiring into the pane's mouse handlers + render highlight lands with T042.**
- [X] T044 [US2] Implement client-side keymap → byte translation sending `SessionInput` (FR-019); `Drop for App` sends `Goodbye`/disconnect, never a kill. **Foundation landed early (minimal drive loop)**: the daemon *receive* half is done — `DaemonState` now owns a live-session registry (`register_session`/`remove_session`/`live_session`) with a per-session `InputReceiver`, and `server.rs` routes `ClientMsg::SessionInput { serial, bytes }` through `session_input`, which classifies the serial (Apply→write to PTY; Lost→surface loudly then write; Stale→drop) and writes to the PTY *after* dropping the state lock. `PtySession.master` is now behind a `Mutex` so the session is `Sync` and can live in the shared registry. Covered by `crates/micold-daemon/tests/drive_loop.rs` (input reaches a live `cat` PTY; in-order batches preserve order; unknown-session input is a logged no-op). **Client stamper landed**: `micold_client::input::SessionInputStamper` holds one `InputSeq` per session (in the client's long-lived state, so it is never reset by a detach/reattach) and turns key-encoded bytes into a `ClientMsg::SessionInput` with a dense monotonic per-session serial; `forget(session)` clears a counter on session end. Covered by `crates/micold-client/tests/client_input.rs` incl. the real `keymap::encode` → `stamp` → `SessionInput` pipeline and the no-reset-across-reattach property. **Remaining for T044**: the *live wiring* — a daemon-connection actor (an iced `Subscription` owning the `Framed` `DaemonConnection` + an outgoing `mpsc` channel), rerouting `Message::TerminalBytes` through the stamper→channel instead of the local `RuntimeTerminal` PTY, and `Drop for App` → `Goodbye`. This must land **together with T042** (frame receive/render): switching input to the daemon without also rendering daemon frames would type blind, and keeping the local PTY alongside would violate the single-source-of-truth rule. That combined switch is the load-bearing T041 change.
  **Reopened (BUG-006, 2026-07-27)**: *"in the client's long-lived state, so it is never reset by a
  detach/reattach"* — long-lived here means **process**-lived, and against a daemon built to outlive
  the client process that is not long enough. `App.stamper` is rebuilt by `SessionInputStamper::new()`
  on every client start (`main.rs:352`), while the daemon's `InputReceiver` is created once per
  session at `register_session` (`state.rs:768`) and is never reset by `detach` (`state.rs:414`). So a
  restarted UI stamps serial `0` into a session the daemon expects at `N`, every serial classifies
  `Ordering::Less → Stale` (`micold-core/src/input.rs:98`), and `session_input` returns without
  writing to the PTY (`state.rs:1131-1134`) — total, silent input loss until the daemon restarts.
  The keymap translation, the wire shape, and the `Drop`→`Goodbye` behaviour are correct and
  unaffected; what is missing is adopting the daemon's position on connect (T111). Corroborating: the
  `forget(session)` hook this note describes has **no caller anywhere in the client** — the stamper's
  lifecycle was never tied to session lifecycle at all (T114).
  **Re-closed (⚠️ was reopened BUG-006, 2026-07-28)**: T111 seeds the stamper from the daemon's
  authoritative position on connect and on later catalog pushes, so a counter this process never had
  is adopted rather than invented; T114 gave `forget` its callers. The stamper's doc no longer claims
  its own lifetime bounds the session's — it now says the daemon's position is authoritative and
  names why.
- [X] T045 [US2] Implement the loopback HTTP hook receiver: bind `127.0.0.1`/`::1` only, ephemeral port, per-session bearer token, 403 on mismatch, bounded bodies, no body logging, in `crates/micold-daemon/src/hooks.rs` (contracts/hooks.md). **Done**: `HookReceiver` binds `127.0.0.1:0` (ephemeral, never `0.0.0.0`), holds a per-session token registry, and a hand-rolled bounded HTTP/1.1 handler enforces every listener rule — POST `/hook/<uuid>` only, per-session bearer token with a bare `403` on mismatch (never revealing session existence), `MAX_HEAD`/`MAX_BODY` bounds (431/413), and bodies are never logged. Recognised hooks drive `DaemonState::note_activity` and push one `CatalogChanged`. Pure parsing (`parse_head`/`session_id_from_path`/`classify_hook`/`settings_json`) is unit-tested (9); the bind→POST→activity path incl. 403 is in `tests/hooks_receiver.rs` (3). Bound + served from `server::run`; a bind failure is non-fatal (activity degrades to `Unknown`, H1). **Re-fixed (⚠️ was reopened BUG-001, 2026-07-27)**: `settings_json()` was rejected by `claude`'s settings validator for every event (missing `matcher`/`hooks` wrapper); fixed by T086. The receiver itself (bind, token auth, bounds, no-log) was unaffected throughout.
- [X] T046 [US2] Implement the activity FSM (hooks + `Event::Title` spinner as Working-only evidence, invariant H1a; transcript JSONL as explicitly-degraded fallback) in `crates/micold-daemon/src/activity.rs`; write the per-session `--settings` file so user config is never modified. **Done**: the FSM core (`Activity`/`ActivityEvent`/`HookKind`/`is_spinner_title`) is now wired end-to-end. Each `LiveSession` owns an `Activity`; hooks feed it via `note_activity` (T045); the `DaemonListener` captures a braille-spinner edge from the *raw* OSC title (before glyph-strip) into `VtSignals`, and `DaemonState::drain_signals` (on the supervisor cadence) applies it as `SpinnerObserved`. Activity is overlaid onto every `SessionSummary` at snapshot time (never persisted, H3), so a change broadcasts one `CatalogChanged`. `spawn_claude` gets a per-session `--settings` file (`HookReceiver::prepare_settings`) so user config is untouched. Covered by `tests/activity_pipeline.rs`. (Transcript-JSONL degraded fallback remains a later refinement.) **Re-fixed (⚠️ was reopened BUG-001, 2026-07-27)**: the settings file was rejected by `claude` before any hook could fire; fixed by T086. The FSM and per-session settings-file wiring were otherwise unaffected throughout.
- [X] T047 [US2] Adopt `Event::Title` (OSC 0) as the push-based session-title source, stripping the leading status glyph by codepoint range and treating the text as untrusted; retire the 120 ms transcript rescan (`src/main.rs:754`) and the lossy path-slug transform (`src/provider.rs:361-373`). **Done**: the `DaemonListener` captures each OSC-0 title into `VtSignals` (glyph-stripped via `strip_status_glyph`, treated as untrusted); `DaemonState::drain_signals` debounces it and overlays the live title onto each `SessionSummary` (a change → one `CatalogChanged`). The client adopts the daemon-overlaid title in `reconcile_catalog`. The 120 ms client-side transcript rescan and the lossy path-slug transform were already removed in the daemon re-architecture (the client no longer has a `provider.rs`; `main.rs:754` is now diagnostics handling).
- [X] T048 [P] [US2] Build the `activity_badge` shared primitive (builder-into-`Element` API, Principle VIII) in `crates/micold-client/src/ui/material/activity_badge.rs`; render it in the session list for every session (FR-016d). **Done**: `ActivityBadge::new(signal, roles).into()` renders a status dot — filled accent for `Working`, filled attention for `AwaitingInput`, hollow for `Ended`, and **nothing** for `Unknown` (ambient, H2 — never a "needs you" cue the app can't justify). The signal→emphasis decision is a pure, unit-tested `emphasis()` fn. Threaded through: a transient `activity` field on the core `Session`, set from each `SessionSummary` in `reconcile_catalog`, and rendered via a new `TreeItem::badge` slot in `session_tree_item` for every session. **Reopened
  (BUG-004, 2026-07-27)**: the dots never reach the user — the badge hardcodes `"\u{25CF}"`/`"\u{25CB}"`
  into `text(..)`, which draws in `iced::Font::DEFAULT` (Fira Sans). Verified with `ttf-parser`:
  neither Fira Sans nor the shipped `MaterialSymbolsOutlined.ttf` maps U+25CF or U+25CB, so every
  signalled session renders an identical blank box ("tofu") and the filled-vs-hollow shape encoding
  FR-016d relies on is lost. The pure `emphasis()` decision and the signal plumbing are correct and
  unaffected; only the glyph sourcing is wrong. **Re-fixed (⚠️ was reopened BUG-004, 2026-07-27)**:
  T101 moved the dots into the `Icon` vocabulary and renders them through `crate::ui::icon(..)` in
  the Material Symbols font. `emphasis()` and its unit test are byte-for-byte unchanged.
- [X] T049 [US2] User-guide doc: attach/detach flow and what the working / awaiting-input / unknown badges mean, in `docs/daemon.md` (Principle VII). **Done**: `docs/daemon.md` gained an "Attaching, driving, and the activity badges (User Story 2)" section — attach/detach semantics, the activity-dot table (Working / Awaiting input / Unknown-shows-nothing / Ended), why absent hooks report `Unknown` rather than guessing, the "awaiting input is a strong hint not a guarantee" caveat (H4), and how the loopback receiver + OSC-title source work (and what they never log/modify).

**Checkpoint**: US1 + US2 together deliver the MVP — persistent sessions you can drive.

---

## Phase 5: User Story 3 — Project and worktree management still works (Priority: P1)

**Goal**: Add/rename/remove projects and create/rename/delete worktrees through the daemon, with
specific actionable failures, visible pending state, and cross-client propagation.

**Independent Test**: Perform each op; confirm correct result, specific failure messages with git's
stderr preserved, and a second window observing the change (quickstart S8).

### Tests FIRST (Principle I) ⚠️

- [X] T050 [P] [US3] Test in `crates/micold-daemon/tests/mutation_semantics.rs`: worktree-create failures (branch exists, path collision, read-only parent) return a specific `GitFailed` error with git's stderr verbatim, no catalog entry, no leftover directory (FR-034; Edge cases).
- [X] T051 [P] [US3] Test in `crates/micold-daemon/tests/mutation_atomicity.rs`: a `req` lost to disconnect resolves client-side to explicit **unknown**, never success/failure, and settles by reading authoritative state on reconnect (FR-031/035).
- [X] T052 [P] [US3] Test: `WorktreeDelete` on a worktree with a live session and `stop_sessions:false` fails specifically rather than orphaning the process (W2; Edge: delete worktree with live session).

### Implementation

- [X] T053 [US3] Implement the correlated mutating-request handlers (`ProjectAdd/Remove/Rename`, `WorktreeCreate/Delete/Rename`, `SessionCreate/Delete`, `SettingsSet`) resolving to exactly one `OperationOk`/`OperationError` in `crates/micold-daemon/src/catalog.rs`; every git call a named RPC, no `GitRun` escape hatch (FR-009). **Main-sync**: `WorktreeDelete` and `SessionDelete` must **archive** the affected sessions (`Session::archive()`), not drop them — mirroring main `7dc9c8a` so a subsequent reconcile cannot resurrect them; env setup for spawned sessions must resolve the environment in the session's own directory (main `2862bab`, `env_include`). **Main-sync (2026-07-23, main `d88c7a1`)**: the kill + archive + record-drop for a `WorktreeDelete` must be **gated on the removal actually succeeding** — never done unconditionally before the git delete is attempted. On a failed delete (locked worktree, branch checked out elsewhere, permission error) the sessions must be left **untouched** (not killed, not archived, records intact); otherwise the durable archive marker turns an FR-023-recoverable failure into permanent session loss, since the marker also blocks reconciliation-based recovery. The `OperationError` reply carries git's stderr (T050); the catalog reconciles from git truth either way. **Re-closed by T098 (was reopened BUG-003, 2026-07-27)**: the `env_include` clause above was unimplemented — `micold_core::env_include` was referenced nowhere in `crates/micold-daemon/`, and all three PTY-spawn sites in `state.rs` hardcoded a `TERM`-only environment. Fixed by T096–T098 (Phase 15). The RPC handlers, archive-on-delete, and gated-removal behavior described elsewhere in this task were unaffected throughout.
- [X] T054 [US3] Fix the current dropped-error violation: worktree deletion errors must surface (was discarded at `src/main.rs:783-784`) (FR-032, Principle III).
- [X] T055 [US3] Implement client-side pending/disabled control state per in-flight `req` (no duplicate submission) and the unknown-outcome resolution on reconnect in `crates/micold-client/`.
- [X] T056 [US3] Implement empty-session pruning that runs **only** for a project with an attached client (FR-007a) in `crates/micold-daemon/src/catalog.rs`. **Main-sync**: pruning must treat already-`archived` sessions as gone (never revive or re-count them) and mark, not delete, so it composes with the anti-resurrection invariant (main `93a0a08`).
- [X] T057 [US3] User-guide doc: project/worktree operations now run through the daemon, in `docs/daemon.md` (Principle VII).

**Checkpoint**: All P1 stories complete — full MVP with everyday operations intact.

---

## Phase 6: User Story 4 — Unsupervised sessions are supervised anyway (Priority: P2)

**Goal**: With no client attached, an unexpectedly-exited session restarts under the same retry
policy; a crash-loop past the limit settles in a durable `Failed` state reported on next attach.

**Independent Test**: With no UI, kill a session's process → restart; force repeated failures past the
limit → give-up state on next attach (quickstart S4).

### Tests FIRST (Principle I) ⚠️

- [X] T058 [P] [US4] Test in `crates/micold-daemon/tests/supervision_restart.rs`: an unattended process exit triggers restart with the same retry policy as when attached (FR-005); normal `exit` marks stopped, no restart (FR-004 scenario 3).
- [X] T059 [P] [US4] Test: after `MAX_RESTART_ATTEMPTS` the session settles `Failed { reason, attempts }`, persisted, and is surfaced on the next attach (FR-005).

### Implementation

- [X] T060 [US4] Implement the restart FSM (retry counter, `MAX_RESTART_ATTEMPTS`, give-up → durable `Failed { reason, attempts }`) in `crates/micold-daemon/src/supervisor.rs`, identical whether or not a client is attached.
- [X] T061 [US4] Implement per-OS process-tree teardown behind the supervision abstraction — `killpg` on Unix, job object on Windows — in `crates/micold-daemon/src/platform/{unix,windows}.rs` (FR-036); teardown not gated on reader EOF (ConPTY).
- [X] T062 [US4] Surface the `Failed` state (reason + attempt count) in the session list via `SessionSummary` (FR-016a `Ended`).
- [X] T063 [P] [US4] Document the retry policy and the L5 caveat (counter has no time window) in `docs/daemon.md`.

**Checkpoint**: Attended and unattended failure handling are provably identical.

---

## Phase 7: User Story 5 — One viewer per project, with deliberate takeover (Priority: P2)

**Goal**: A second client on a held project is refused with a takeover offer; on takeover the first
drops to a disconnected-but-running state, sends zero further input, and can reconnect once free.

**Independent Test**: Two windows on one project → refusal + takeover affordance → take over → displaced
window visibly disconnected, sends no input, does not exit (quickstart S6).

### Tests FIRST (Principle I) ⚠️

- [X] T064 [P] [US5] Test in `crates/micold-daemon/tests/exclusivity.rs`: `Attach{force:false}` on a held project is refused with `ProjectBusy{holder,since}`; `force:true` displaces the holder (FR-023). **Done**: `a_second_attach_is_refused_then_force_displaces_the_holder` drives two clients over shared `serve_connection` duplexes.
- [X] T065 [P] [US5] Test: a `Displaced` client stops rendering/sending for that project, does not terminate; a crashed holder frees the project without a daemon restart (FR-024; Edge: holder dies). **Done**: `a_displaced_client_is_not_terminated` (post-displace Ping/Pong proves liveness) + `a_crashed_holder_frees_the_project_without_a_restart` (drop the socket → next attach succeeds by default).
- [X] T066 [P] [US5] Test in `crates/micold-daemon/tests/exclusivity.rs`: two clients on two different projects do not interfere (FR-024 scenario 4). **Done**: `two_clients_on_two_projects_do_not_interfere`.

### Implementation — plan W6

- [X] T067 [US5] Implement one-attachment-per-project with reject-by-default and force-takeover, emitting `Displaced` to the prior holder, in `crates/micold-daemon/src/main.rs` (FR-023–025). **Done** (in `state.rs::attach` + the `server.rs` `Attach` route, established during US1/US3): reject-by-default with `ProjectBusy{holder,since_secs}`, `force` pushes `Displaced{project,by}` to the prior holder without terminating it, and `deregister` frees the attachment on EOF (crash case). Now covered by T064–T066.
- [X] T068 [US5] Implement half-open detection via a **3 s-`Ping` / 9 s-deadline** keepalive (worst-case < 10 s per SC-011) and free the project on a crashed holder, in the client connection actor + daemon `Pong` responder (FR-026; Edge: half-open connection). **Done**: a pure, time-injected `micold_core::protocol::keepalive::Keepalive` (3 s `PING_INTERVAL`, 9 s `LIVENESS_DEADLINE`, 1 s `CHECK_INTERVAL` so worst-case detection = deadline + one tick ≤ 10 s) wired into `crates/micold-client/src/daemon.rs`, which now runs an auto-reconnecting outer loop; the daemon `Pong` responder already existed. (Path note: the client connection module is `daemon.rs`, not the tasks.md-hypothesised `daemon_conn.rs`.)
- [X] T068a [P] [US5] Test in `crates/micold-daemon/tests/liveness.rs`: a silently half-open connection (no FIN) is surfaced as `Disconnected` in **≤ 10 s**, and a healthy connection is never spuriously reaped (SC-011). **Done**: `a_responsive_daemon_is_never_reaped` (real Ping/Pong over a duplex, synthetic 15 s clock) + `a_half_open_connection_is_surfaced_within_10s`; plus 4 deterministic unit tests on `Keepalive` in `micold-core`.
- [X] T069 [US5] Build the `connection_banner` shared primitive (builder API, Principle VIII) in `crates/micold-client/src/ui/material/connection_banner.rs` for the disconnected/takeover states; wire the reconnect action (FR-028). **Done**: `ConnectionBanner` builder + a `ui::ConnectionStatus` the binary computes; the disconnected banner auto-reconnects, the displaced banner carries a "Take over" action (`ConnectionTakeoverRequested` → `Attach{force:true}`). A displaced window is read-only — `TerminalBytes` is suppressed before stamping so no input serial is consumed (G2).
- [X] T070 [P] [US5] User-guide doc: exclusivity, takeover, and the second-test-client workflow, in `docs/daemon.md` (Principle VII). **Done**: "One window per project, with deliberate takeover (User Story 5)" section.

**Checkpoint**: The daily two-window collision is handled without input fighting.

---

## Phase 8: User Story 6 — Contract mismatch fails loudly and recoverably (Priority: P2)

**Goal**: A rebuilt client against an older daemon is refused with a diagnostic naming both versions;
a one-click restart swaps the daemon; previously-live sessions come back interrupted-resumable, never
auto-relaunched.

**Independent Test**: Launch a client whose version/hash differs from the running daemon → refusal +
working restart action; a live session returns interrupted-resumable (quickstart S7, S5).

### Tests FIRST (Principle I) ⚠️

- [X] T071 [P] [US6] Test in `crates/micold-daemon/tests/version_recovery.rs`: on daemon restart, previously-running sessions load as `InterruptedResumable` — distinct from `Running` and from a deliberate `Idle` stop — and no process is auto-relaunched (FR-006a/b; Edge: identity exists, process does not). **Done**: `restart_presents_previously_running_sessions_as_interrupted_resumable` (was-running → InterruptedResumable, never-started → Idle, shell → Idle, none active) + `present_interrupted_resumable_never_overrides_a_running_or_failed_session`.
- [X] T072 [P] [US6] Test: `SessionStart` on an `InterruptedResumable` session is the single explicit action that resumes the prior conversation (FR-006a). **Done**: `session_start_is_the_single_explicit_resume_of_an_interrupted_session` (InterruptedResumable → `start()` → Starting); the daemon's `SessionStart` handler already drives `LaunchMode::Resume`.

### Implementation

- [X] T073 [US6] Implement the `InterruptedResumable` lifecycle state and its persistence/reload on daemon start (FR-006a/b) in `crates/micold-core/src/session.rs` + `crates/micold-daemon/src/catalog.rs`. **Done**: new `SessionLifecycle::InterruptedResumable` (+ `start()` accepts it, `mark_interrupted_resumable()` guards `Idle`); `Catalog::present_interrupted_resumable(predicate)` flips loaded-`Idle` AI-CLI sessions with a recorded conversation (S3-respecting: lifecycle not persisted, derived at startup from the provider's durable transcript store); `DaemonState::present_interrupted_resumable_at_startup` backs the predicate with `ClaudeProvider`; called once in `server::run` off the runtime before the accept loop (data-model L4 — the only lifecycle startup produces). Wire mapping in `catalog::wire_lifecycle` + the client's `wire_to_lifecycle`/sidebar/status now carry the state distinctly.
- [X] T074 [US6] Implement the client's version-mismatch diagnostic and "restart daemon" action (stop old, start matching, attach; warn live processes lost but sessions resumable) in `crates/micold-client/` (FR-022). **Done**: the daemon records its pid in the lock file; `micold_core::spawn::stop_running_daemon` terminates it by pid (the version-agnostic stop — a mismatched client can't handshake). The actor surfaces `Connected::Refused(VersionMismatch)` as `Message::DaemonVersionMismatch{client,daemon,daemon_build}`; a `ConnectionStatus::VersionMismatch` banner names both versions + the build and offers "Restart service" → `ConnectionRestartServiceRequested`, which stops the old daemon; the existing auto-reconnect loop then spawns a matching one and the sessions reload interrupted-resumable (T073). A toast warns live processes are lost but sessions are preserved.
- [X] T075 [P] [US6] User-guide doc: what a contract mismatch looks like and how restart-and-resume behaves, in `docs/daemon.md` (Principle VII). **Done**: "A version mismatch fails loudly, and recovers (User Story 6)" + "Interrupted-resumable sessions after any service restart" sections.

**Checkpoint**: The developer's own rebuild loop fails safe and recovers in one click.

---

## Phase 9: User Story 7 — Surviving logout on Linux (Priority: P3)

**Goal**: On Linux, documented `loginctl enable-linger` lets sessions survive logout; it is never
enabled silently; macOS/Windows docs state the limitation plainly.

**Independent Test**: On Linux with linger enabled, log out/in → session survived; without it → ends;
docs state per-platform support (quickstart S14).

### Implementation — plan W6 (packaging)

- [X] T076 [P] [US7] Ship systemd **user** units (`packaging/micold-daemon.socket`, `.service`) — shipped but NOT enabled at install; the client enables in-session. **Done**: both units created (`Type=simple` + `listenfd` fd adoption — the daemon has no sd_notify; `Accept=no`; `Restart=on-failure` so a future clean idle-exit re-activates). Deb assets now ship the `micold-daemon` binary + both units to `/usr/lib/systemd/user/`.
- [X] T077 [US7] Implement the in-session enable path (client offers to enable the user service / linger; never automated by install). **Done**: `micold_core::logout_survival::enable(endpoint)` (Linux-gated) runs the load-bearing sequence — `loginctl enable-linger` → stop the session-scoped daemon → `systemctl --user enable --now micold-daemon.socket` — detecting failure (never assuming), and returns a `SurvivalOutcome` with a user message. Surfaced as a Linux-only "Keep sessions after logout" overflow-menu item → `Message::LogoutSurvivalRequested`, run off-thread, result shown as a toast. (Client connection module is `daemon.rs`, not the tasks.md-hypothesised `daemon_conn.rs`.)
- [X] T078 [P] [US7] User-guide doc: `loginctl enable-linger` instructions with the "enable linger THEN start the daemon — not retroactive" ordering warning; explicit statement that logout survival is Linux-only (FR-038, Principle VI exception). **Done**: "Surviving logout (User Story 7)" section in `docs/daemon.md` — the ordering/retroactivity warning, the Linux-only statement + explicit macOS/Windows unsupported note, and the packaging note.

**Checkpoint**: All user stories functional. ✅ US1–US7 complete.

---

## Phase 10: Polish & Cross-Cutting Concerns — plan W6 (tests, docs) + validation

**Purpose**: Test redistribution accountability, diagnostics surfacing, and the cross-platform gate.

- [X] T079 Redistribute all 259 pre-split tests to owning crates (pure logic → `micold-core`, supervision/protocol/lifecycle → `micold-daemon`, render-coupled → `micold-client`) with a per-test disposition record (moved / rewritten / retired-with-reason); **silent deletion forbidden** (FR-041). Gate: `cargo test --workspace` green with the T007 baseline accounted for. **Done**: redistribution happened during the Phase 1 split; the accountability record is `specs/010-daemon-session-persistence/test-disposition.md` — crate/file-level disposition + count reconciliation (259 pre-split → 690 test fns / 98 groups now; the count rose, so nothing was silently deleted) + the two explicit retired-with-reason entries (`tests/pty_routing.rs`; the old crate-root unit tests). Gate green.
- [X] T080 Implement the diagnostics surface end to end: `LogLocationRequest`/`RecentErrorsRequest`/`SetLogLevel` (runtime `EnvFilter` reload) and the client UI that shows the log location and recent daemon errors (FR-043–046, SC-017). **Done**: a bounded `RecentErrors` ring captured by a dedicated tracing layer (sink-independent) in `logging.rs`; the diagnostics handle stored on `DaemonState` (`OnceLock`, set at startup); daemon handlers for all three RPCs in `server.rs` (SetLogLevel reloads the `EnvFilter` and refuses an invalid directive as `InvalidInput`). Client: a "Session service diagnostics" overflow-menu action requests location + recent errors and shows them as notices. Tested by `tests/diagnostics.rs` (RPCs end-to-end) + ring unit tests in `logging.rs`.
- [X] T080a Place the mandated FR-045 log-event call sites: startup/shutdown with reason; endpoint bind and bind failure; client attach/detach/refusal/takeover with reason; session start/exit/restart-attempt/give-up with reason; every mutating-operation failure with the underlying diagnostic preserved. Assert their presence in `crates/micold-daemon/tests/log_events.rs` (FR-045; T025 built the infra, this places the calls). **Done**: added the missing sites — `Detach` log, endpoint-resolve/bind-failure logs (`server::run`), session start (`start_session`), and session exit / restart-attempt / give-up / recovered (`supervise_exited_sessions`). `tests/log_events.rs` drives the connection subset (connect/attach/refusal/takeover/detach/disconnect) under a global in-memory subscriber and asserts each event with its reason; startup/bind + session-lifecycle sites are placed at their call sites and exercised by the daemon-lifecycle/autospawn/supervision suites (they need a second process / live PTY to assert here).
- [X] T081 [P] Verify no terminal content or user input appears in any log entry — grep a typed string against the log file in `crates/micold-daemon/tests/log_redaction.rs` (FR-047). **Done**: `tests/log_redaction.rs` installs a global in-memory subscriber, drives attach + `SessionInput` carrying a sentinel, and asserts the sentinel never appears in any captured log line (a regression guard — no site logs those bytes today).
- [X] T082 [P] Cross-cutting documentation review + index/navigation in `docs/` (Principle VII). **Done**: `docs/README.md` now indexes `docs/daemon.md` under a "The session service (daemon)" section (it was previously unlinked); added a "Finding the logs and recent errors" section to `daemon.md` so the diagnostics surface (T080) is documented and the index reference is accurate.
- [ ] T083 CI gate: build + full test suite pass on Linux, macOS, and Windows before merge; exercise Windows job-object teardown, `0x03` interrupt (NOT via `cmd.exe`), and the inverted `portable-pty` `kill()` result (Principle VI, Risk 3). **Blocked in this environment** (no macOS/Windows runners): Linux `cargo test --workspace` is green (98 groups); the Windows job-object teardown / `0x03` interrupt / inverted-`kill()` paths are the deliberately-deferred W5 stubs and need the Windows CI job to validate.
- [ ] T084 Run all quickstart.md scenarios S1–S15 and record outcomes. **Partially blocked**: the scenarios are GUI/interactive (`mise run run`, multi-window, logout). Their underlying behaviour is covered by automated tests — S5/S7 (`version_recovery`), S6 (`exclusivity`/`liveness`), S8 (`mutation_semantics`/`mutation_atomicity`), supervision (`supervision_restart`/`supervision_giveup`) — but the manual walkthrough + outcome log still needs a human at the GUI.
- [ ] T085 [P] Measure retargeted `TerminalPane` repaint cost at 60 Hz on the client (Risk 2 — all streaming measurements were daemon-side); record whether the tick rate is the right knob. **Blocked in this environment**: needs a running GUI + a frame profiler; can't be measured headlessly here.

---

## Phase 11: Bugfix BUG-001 — hook settings JSON missing the matcher/hooks wrapper

**Goal**: `claude`'s settings validator currently rejects the per-session `--settings` file for
every lifecycle event (`Expected array, but received undefined` on `hooks.<Event>.0.hooks`), so no
session's activity hooks ever reach the daemon. T045/T046 (Phase 4, above) are reopened for this.
See `bugs/BUG-001.md`.

- [X] T086 [US2] Fix `settings_json()` in `crates/micold-daemon/src/hooks.rs` to wrap each event's
  hook entry in a matcher-group object (`{"matcher": "", "hooks": [{"type": "http", "url": ...,
  "headers": {...}}]}`) instead of the flattened `{"type": "http", ...}` placed directly in the
  event array (BUG-001). Update `settings_json_embeds_the_url_and_bearer_token` (~line 433), which
  currently asserts `parsed["hooks"]["UserPromptSubmit"][0]` fields directly against the flattened
  shape, to assert against the matcher-group wrapper and its nested `hooks[0]` entry instead.
  Depends on T045/T046 (reopened above). **Done**: rewrote `settings_json()` from untyped
  `serde_json::json!` macros to typed structs (`HttpHook`/`MatcherGroup`/`HooksMap`/`SettingsDoc`,
  `#[derive(serde::Serialize)]`, `serde = { workspace = true }` added to
  `crates/micold-daemon/Cargo.toml`) emitting `[{"matcher": "", "hooks": [{"type": "http", "url",
  "headers"}]}]` for every event — a key typo is now a compile error, not a second BUG-001 (found by
  `/code-review high --fix` on this diff). Same pass also wired up the previously-missing
  `SubagentStop` entry: `activity.rs::classify_hook` already grouped `"Stop" | "SubagentStop"` into
  one transition, but nothing in `settings_json()` told `claude` to ever send a `SubagentStop` hook,
  so that branch was dead code (also found by the same review). `contracts/hooks.md`'s Configuration
  example and `settings_json_embeds_the_url_and_bearer_token` both updated to include it. Re-closes
  T045/T046. `cargo test --workspace` (98 groups), `cargo clippy -D warnings`, `cargo fmt --check`
  all clean.
- [X] T087 [P] [US2] Add a regression test, co-located with `settings_json()` or as a new
  integration test (whichever fits once T086 lands), that parses the generated settings JSON and
  asserts every event's array entries are matcher-group objects with a non-empty `hooks` array (the
  shape a real Claude Code settings loader requires), so a future flattening regression fails the
  suite instead of only surfacing at runtime against a live `claude` binary (BUG-001). **Done**:
  landed co-located, folded into the existing
  `settings_json_embeds_the_url_and_bearer_token` unit test (`crates/micold-daemon/src/hooks.rs`) —
  the pure-function shape assertion belongs beside the function it tests rather than in a new
  integration-level file, since no live HTTP/`claude` process is needed to catch a flattening
  regression. Now loops over all six events asserting a `matcher` field is present and the nested
  `hooks` array is non-empty. `cargo test --workspace` (98 groups) green.

---

## Phase 12: Bugfix BUG-002 — daemon not restarted (or flagged stale) on a same-contract `.deb` upgrade

**Goal**: the FR-021 handshake only refuses on a `PROTOCOL_VERSION`/`SCHEMA_HASH` mismatch, so a
`.deb` upgrade whose daemon-side fix doesn't touch the wire schema — the common case, e.g. the
BUG-001 fix — never trips it: the new client silently attaches to the old, already-running daemon
via the `AlreadyRunning` singleton path, and the fix stays inert until something unrelated restarts
the daemon. Adds FR-022a (build-staleness detection independent of wire-contract compatibility). See
`bugs/BUG-002.md`.

- [X] T088 [P] [US6] Test in `crates/micold-core/tests/handshake.rs`: same `protocol_version`/
  `schema_hash` but differing `client_build`/`daemon_build` returns a `RefusalReason::BuildMismatch`
  distinct from `VersionMismatch`; matching builds still return `Ok(())` (FR-022a, BUG-002). **Done**:
  discovered mid-implementation that `client_build`/`daemon_build` are free-form diagnostic strings
  with different program-name prefixes (`"micold-ai-ide/…"` vs `"micold-daemon …"`) that can never be
  equal even on a matching release, so comparing them directly was never viable — confirmed with the
  user before proceeding (per the project's "verify or ask, don't silently assume" convention).
  Landed a dedicated `PACKAGE_VERSION` constant instead (below). 6 tests in `handshake.rs`: the two
  new ones (`build_mismatch_is_refused_distinctly_when_contract_still_matches`,
  `matching_package_version_is_accepted_even_with_differing_build_strings`) plus the 4 existing tests
  updated for the new `evaluate()` signature.
- [X] T089 [US6] Implement build-mismatch detection: add `RefusalReason::BuildMismatch{client_build,
  daemon_build}` in `crates/micold-core/src/protocol/messages.rs`; thread `client_build` into
  `handshake::evaluate` (`crates/micold-core/src/protocol/handshake.rs`) and compare it against
  `daemon_build()` (`crates/micold-daemon/src/server.rs`) whenever `protocol_version`/`schema_hash`
  already match. Depends on T088. (FR-022a, BUG-002) **Done**: added `PACKAGE_VERSION: &str =
  env!("CARGO_PKG_VERSION")` to `crates/micold-core/src/protocol/version.rs` — every workspace member
  shares one version (`version.workspace = true`), and `release-please` bumps it on every release
  (confirmed against `CHANGELOG.md`/`git log` — 0.2.0 → 0.3.0 → 0.4.0, one bump per release), so it
  changes on every `.deb` build whether or not the wire contract moved. Added `client_package_version`
  to `ClientMsg::Hello` (a wire-visible change — bumped `PROTOCOL_VERSION` 1→2 per this file's own
  "MUST be bumped" convention; the release that ships this fix will itself trip the *existing*
  `VersionMismatch` path once via the schema-hash change, then `BuildMismatch` covers every
  same-contract release after that). `handshake::evaluate` now takes `client_package_version` and
  `client_build` and checks contract first (unchanged), then package version, returning
  `RefusalReason::BuildMismatch{client_build, daemon_build}` on a same-contract difference.
  `server.rs::serve_connection` destructures the new Hello field and passes it through; its refusal
  log line gained `client_package_version`/`daemon_package_version` fields. Updated all 16
  `ClientMsg::Hello` construction sites across the daemon/core integration tests for the new field.
  `cargo test --workspace` (103 groups), `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check` all clean.
- [X] T090 [US6] Client: extend the existing version-mismatch restart-action UI (`crates/micold-client/`,
  T074) to also handle `Connected::Refused(BuildMismatch)` — same "Restart service" action and
  stop/spawn mechanics, but presented as a distinct, lower-severity notice without the "live
  processes will be lost" warning (the contract still matches). Depends on T089. (FR-022a, BUG-002)
  **Done**: `daemon.rs`'s `connect_and_pump` matches `RefusalReason::BuildMismatch` alongside
  `VersionMismatch` and sends a new `Message::DaemonBuildMismatch{client_build, daemon_build}`
  (`app.rs`). `main.rs`'s `App` gained a sibling `build_mismatch: Option<(String, String)>` field
  (cleared on every successful connect and on "Restart service", alongside `version_mismatch`);
  `connection_status()` checks it with the same precedence tier as (but after) `version_mismatch`.
  `ui/mod.rs` gained `ConnectionStatus::BuildMismatch` and its banner: "A newer session service is
  installed" / "…your sessions are unaffected either way and remain resumable", reusing
  `Message::ConnectionRestartServiceRequested` — the mechanics (stop old, auto-reconnect spawns
  matching) are identical to the contract-mismatch case, only the wording differs.
- [X] T091 [P] [US6] User-guide doc: extend `docs/daemon.md`'s "A version mismatch fails loudly, and
  recovers (User Story 6)" section to distinguish build-staleness (same contract, different binary —
  most bugfix/feature releases) from a wire-contract mismatch, so users understand why an upgrade did
  or didn't restart the daemon. (FR-022a, FR-042, BUG-002) **Done**: new "After installing an update"
  subsection between the existing "Your sessions survive the restart" bullet and "Interrupted-resumable
  sessions after any service restart".

**Bugfix**: 2026-07-27 — BUG-002 Added Phase 12 (T088–T091) for build-staleness detection (FR-022a).
See `bugs/BUG-002.md`.

---

## Phase 13: Convergence

Produced by `/speckit-converge` after T088–T091 (BUG-002) landed. Both items trace to the
version/build-mismatch banner surface US6/FR-021/022/022a touch; other user stories were not
re-derived from source (see the run's findings summary).

- [X] T092 Add a unit test asserting `connection_status()`'s precedence order (`crates/micold-client/src/main.rs`) — `VersionMismatch` > `BuildMismatch` > `Displaced` > `Disconnected`/`Connected` — since the function contains decision/branching logic the GUI-wiring test exception explicitly excludes, and it currently has zero coverage anywhere in the crate per Constitution I (contradicts) **Done**: `connection_status_orders_mismatch_over_displaced_over_disconnected` in `main.rs`'s existing `#[cfg(test)] mod tests` — one `App`, mutated field-by-field through all five states, asserting each precedence step in turn.
- [X] T093 Add an end-to-end test in `crates/micold-daemon/tests/handshake_flow.rs`, mirroring `mismatched_handshake_is_refused_naming_both_sides`, that drives `server::serve_connection` with a matching `protocol_version`/`schema_hash` but a differing `client_package_version` and asserts a `Refused{reason: RefusalReason::BuildMismatch{..}}` frame is received, closing the gap between FR-022a's pure-function unit coverage (T088) and its wire-level round-trip per FR-022a / SC-009a (partial) **Done**: `build_mismatch_is_refused_distinctly_when_contract_still_matches` — real `serve_connection` over an in-memory duplex, matching contract + stale `client_package_version`, asserts `RefusalReason::BuildMismatch` naming both builds and that the daemon closes the connection, mirroring the existing contract-mismatch test. `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` all clean.

---

## Phase 14: Convergence (retrofit session, 2026-07-27)

Produced by a separate, broader `/speckit-converge` retrofit sweep across all 21 features in
`specs/`, run against this feature's state before T088–T093 (BUG-002 and its follow-up) landed.

- [X] T094 This feature's own distinctive requirements were verified directly and hold: FR-016a/b
  (the `activity.rs` FSM is purely hook-driven — grepped for zero `Instant`/`elapsed`/timing
  logic anywhere near it, confirming no quiescence inference survived the FR-016b rewrite),
  FR-031/FR-035 (`Message::DaemonDisconnected` resolves every in-flight `pending_op` to an
  explicit "unknown outcome" notification, never silent success/failure, per the reconnect-reads-
  authoritative-state contract), and FR-007a (`prune_empty_off_runtime` is called only from the
  `Attach` handler, never a background timer, so pruning never runs unattended).
- **Not new findings, but tracked back here for a single point of reference**: this retrofit
  sweep found and fixed four separate cases where this feature's migration of session/worktree
  lifecycle logic from the client into the daemon silently dropped wiring that a pre-existing
  feature's own spec required, each verified red→green with a regression test:
  - `specs/005-worktree-session-terminal/tasks.md` T081 — the FR-020c durable archive marker
    (`AiCliProvider::mark_archived`) was never called from the daemon's `archive_session`/
    `archive_worktree_sessions`/`archive_session_ids`.
  - `specs/006-real-terminal-emulator/tasks.md` T058/T059 — `app::route_key`/`KeyRouting`
    (FR-009's focus gate) and `app::should_write_to` (FR-012a's write gate) both became
    orphaned pure functions once the daemon took over process liveness.
  - `specs/008-background-project-switching/tasks.md` T035 — `State::note_background_restart`
    (FR-011) was never called from `reconcile_catalog`, so the daemon-driven snapshot
    reconciliation never detected a background session transitioning into `Restarting`.
  - `specs/013-create-worktree-refinement/tasks.md` T030 — `ClientMsg::WorktreeDelete` shipped
    with no wire field for the keep/delete-branch choice (FR-011/FR-012/FR-014), so the daemon
    hardcoded "always keep the branch" regardless of the confirm dialog's checkbox.
  No further instances were found in this feature's own scope. Recorded here in case it's useful
  context for reviewing future migrations of similar scope.

---

## Phase 15: Bugfix BUG-003 — daemon never resolves `env_include`, so no daemon-spawned session sees `~/.bashrc`-exported variables

**Goal**: All three of the daemon's own PTY-spawn sites (`start_session`, `respawn_primary`,
`open_shell` in `crates/micold-daemon/src/state.rs`) resolve the environment-include setting
(feature 011) in the session's own directory and merge it into the spawned process's environment,
exactly as `micold-client`'s launch path already does — and the three `env_include_*` settings
fields become readable/settable through the daemon, mirroring the existing `scrollback_lines`
precedent (FR-012a/FR-012b). Reopens T053's env-include clause (see above).

**Independent Test**: Configure `env_include` (enabled, default `~/.bashrc` path) with a plain
`export SOME_TOKEN=abc123` line; start a session so the daemon spawns it (fresh `SessionCreate`,
a crash respawn, or a regular-terminal `open_shell` instance); confirm the spawned process's
environment contains `SOME_TOKEN=abc123`, matching what `micold-client`'s own spawn path already
produces for the same script.

### Tests for BUG-003 (MANDATORY — Constitution Principle I) ⚠️

> Written FIRST; confirmed to FAIL before implementation.

- [X] T095 [P] [BUG-003] Add a failing test in `crates/micold-daemon/tests/session_start.rs` (or a
  new `env_include_spawn.rs`, whichever fits): construct a `Catalog`/`DaemonState` test harness
  directly against a `Settings` with `env_include_enabled: true` and `env_include_script_path`
  pointing at a real disposable script containing a plain unconditional `export` (mirroring the
  real-subprocess pattern `tests/env_include_resolve.rs` established for BUG-001/BUG-002 in
  `specs/011-env-include-script/`) — no dependency on T096/T097, since `Settings` already carries
  the three fields today (`crates/micold-core/src/settings.rs:94-102`); only the spawn path (T098)
  needs to change for this to pass. Call `start_session`, and assert the spawned `PtySession`'s
  child process actually has that variable set. Confirmed failing today: the hardcoded
  `vec![("TERM", ...)]` means no such variable can ever appear (`bugs/BUG-003.md`). **Done**: landed
  co-located in `session_start.rs` as `a_daemon_spawned_session_sees_env_include_resolved_variables`
  — writes a disposable script exporting `BUG003_MARKER`, starts a Regular (shell) session via
  `start_session`, drives it with `session_input` to `echo` the variable back, and asserts it
  appears in the live PTY's rendered grid (mirroring `drive_loop.rs`'s `visible_text` pattern) —
  proving the variable is actually in the spawned process's own environment, not just resolvable in
  the abstract.
- [X] T096 [P] [BUG-003] Extend `DaemonSettings` (`crates/micold-core/src/protocol/messages.rs:
  620-623`) with `env_include_enabled: bool`, `env_include_script_path: String`,
  `env_include_timeout_secs: u64`; update `Catalog::settings_wire()`
  (`crates/micold-daemon/src/catalog.rs:87-91`) to populate them from `self.settings` (mirroring
  `scrollback_lines`, FR-012b). Read-side only (what the daemon reports to a client); no dependency
  on T097/T098. **Done**: fields added; `settings_wire()` populates them. `DaemonSettings` dropped
  its `Copy` derive (a `String` field can't be `Copy`) — no call site relied on copy semantics
  (every use already constructs or clones it fresh). **Discovered while implementing**: the added
  `String` field tipped `Connected::Ready(DaemonConnection, Welcome)` (`crates/micold-core/src/
  connect.rs`) — which embeds a `Welcome` carrying this `DaemonSettings` — over clippy's
  `large_enum_variant` threshold against the much-smaller `Refused(RefusalReason)` variant; fixed by
  boxing the connection (`Ready(Box<DaemonConnection>, Welcome)`, matching clippy's own suggested
  fix), updating the one construction site (`connect.rs::handshake`) and confirming the two
  destructuring call sites (`micold-client/src/daemon.rs`, `micold-daemon/tests/autospawn.rs`)
  still compile — they do, since `Box`'s special move-out support lets `conn.split()`/`drop(conn)`
  work unchanged through the box.
- [X] T097 [BUG-003] Add `env_include_enabled: Option<bool>`, `env_include_script_path:
  Option<String>`, `env_include_timeout_secs: Option<u64>` to the `ClientMsg::SettingsSet` variant
  itself (`crates/micold-core/src/protocol/messages.rs:261-266`, alongside the existing
  `scrollback_lines: Option<usize>`), and extend the daemon's handler
  (`crates/micold-daemon/src/server.rs:553-558`) so a client can request a change to any of the
  three env-include settings, persisting via the same `Catalog`/`SettingsStore` path
  `set_scrollback` already uses. Write-side counterpart to T096; no dependency on T096/T098 (touches
  a different message variant and a different settings-mutation path). **Done**: added the three
  optional fields to `SettingsSet`; `Catalog::set_env_include` (mirroring `set_scrollback`, clamping
  the timeout via `clamp_env_include_timeout`) and `DaemonState::set_env_include` (mirroring
  `set_scrollback`'s broadcast-`SettingsChanged` shape) added; `server.rs`'s handler now applies
  scrollback and/or env-include depending on which fields are present, short-circuiting to a no-op
  `Ok(())` when neither settings kind is present. `DaemonState::set_env_include` calls
  `invalidate_env_include_all()` (T098) after persisting, closing the T098/T097 cache-invalidation
  handoff this task's own text left open. Updated the two other `ClientMsg::SettingsSet`
  construction sites the new required fields broke (`crates/micold-daemon/tests/
  daemon_lifecycle.rs`, `crates/micold-core/tests/protocol_roundtrip.rs`) and the `DaemonSettings`
  struct literals in `protocol_roundtrip.rs`'s round-trip fixtures (T096).
- [X] T098 [BUG-003] Add a per-directory `env_include` resolution cache owned by
  `DaemonState`/`Catalog` (there is no `App` in the daemon to hold one, unlike `micold-client`'s
  `env_include_cache: HashMap<PathBuf, EnvIncludeSnapshot>`); call `micold_core::env_include::
  resolve` (with `merge_with_term`) against the session's own `cwd` at all three spawn sites in
  `crates/micold-daemon/src/state.rs` — `start_session` (replacing the hardcoded `env` at line 633),
  `respawn_primary` (line 820), `open_shell` (line 952) — gated on `env_include_enabled` and
  short-circuiting to `TERM`-only when disabled or the path is blank, mirroring `micold-client`'s
  existing caller-side convention, reading straight from `Catalog`'s already-loaded `Settings` (no
  dependency on T096/T097 — those only add a *wire* projection/mutation path for clients, which
  this task's spawn-time read does not go through). Invalidate the cache entry for a path on
  `WorktreeDelete` (mirroring the follow-on fix recorded in
  `specs/011-env-include-script/bugs/BUG-002.md`'s Resolution). If T097 lands first, also invalidate
  every cached directory on a `SettingsSet` that changes any of the three fields; if T098 lands
  first, T097 must add that invalidation call when it wires up the handler — whichever task lands
  second closes this loop. Makes T095 pass; re-closes T053's env-include clause. **Done**: added
  `env_include_cache: HashMap<PathBuf, Vec<(String, String)>>` to `Inner` (caches the already-
  `merge_with_term`-merged vars, ready to hand straight to a spawn site's `env`);
  `DaemonState::env_include_vars_for(cwd)` reads the enabled/path/timeout settings and checks the
  cache under one short lock, drops the lock before calling `env_include::resolve` (which spawns a
  real subprocess and may block up to the configured timeout — the module's existing invariant that
  blocking work never happens under the state lock, same reason PTY spawning itself is off-lock),
  then re-locks briefly to cache the result. All three spawn sites now call
  `self.env_include_vars_for(&cwd)` instead of the hardcoded `vec![("TERM", ...)]`.
  `DaemonState::invalidate_env_include`/`invalidate_env_include_all` added; the `WorktreeDelete`
  handler (`server.rs`) now calls `invalidate_env_include` with the deleted worktree's path
  (computed before `repo` moves into the delete's `spawn_blocking` closure) once the git delete
  actually succeeds — mirroring BUG-002 (011)'s equivalent fix for the exact same "path gets reused
  by the same branch name" hazard.
- [X] T099 [P] [BUG-003] User-guide doc: note in `docs/daemon.md` that daemon-spawned sessions
  (including crash respawns and regular-terminal instances) resolve `env_include` identically to a
  client-initiated launch, per-directory (Principle VII). Depends on T098 (describes its finished
  behavior). **Done**: added to the "Project and worktree operations run through the daemon (User
  Story 3)" section, ahead of the existing "current limitations" list.

**Checkpoint**: A daemon-spawned session (fresh, respawned after a crash, or a regular-terminal
instance) sees the same `env_include`-resolved variables a `micold-client`-spawned session in the
same directory would; T095's regression test passes; `mise run test` stays green throughout. **Met**
— verified: `cargo test --workspace` green (103 test groups, 0 failures, including T095's new test);
`cargo clippy --workspace --all-targets -- -D warnings` clean (after the `Connected::Ready` boxing
fix above); `cargo fmt --check` clean.

**Known follow-up, out of scope for this bugfix**: `micold-client`'s own `Message::SettingsSaved`
handler (`crates/micold-client/src/main.rs`) persists settings by writing `settings.json` directly
via its own local `JsonFileSettingsStore`, for every field including scrollback — it never sends
`ClientMsg::SettingsSet` to a *running* daemon at all (confirmed: zero references to `SettingsSet`
anywhere in `crates/micold-client/`). A settings change made while a daemon is already running only
takes effect for daemon-spawned sessions once the daemon restarts and re-reads the file (the
"Deferred within this task" pattern already flagged for scrollback in T067's docs note is this same
gap, pre-existing and not introduced by BUG-003). This does not block BUG-003's own fix — the
default configuration (env-include enabled, default `~/.bashrc` path) never goes through this path
at all, only a live-edit-while-running scenario does — but is worth its own follow-up task if a user
needs to see an env-include settings change apply without restarting the daemon.

**Bugfix**: 2026-07-27 — BUG-003 Added Phase 15 (T095–T099); reopened T053's env-include clause.
FR-012b added to spec.md; plan.md's W2 gained a design-correction note on the daemon's settings
projection and per-directory cache. See `bugs/BUG-003.md`. Resolved 2026-07-27: T096–T098 landed
the daemon-side `env_include` wiring (settings projection/mutation + the per-directory cache and
its use at all three spawn sites); T095's regression test passes; T099 documents the behavior;
T053 re-closed. `cargo test --workspace` (103 groups) green; `cargo clippy --workspace --all-targets
-- -D warnings` and `cargo fmt --check` both clean.

---
## Dependencies & Execution Order

### Phase dependencies

- **Setup (P1)** → no deps; the workspace conversion gates everything.
- **Foundational (P2)** → after Setup; **blocks all user stories**. This is large — it is the re-architecture.
- **US1, US2, US3 (all P1)** → after Foundational. US1+US2 are the joint MVP (spec: "Together with Story 1 this is the MVP"). US2 depends on US1's grid framer (T032) for streaming; US3 is independent of US1/US2 beyond Foundational.
- **US4, US5, US6 (P2)** → after Foundational; US4 builds on US1's supervisor; US6 builds on US1's session identity + Foundational handshake.
- **US7 (P3)** → after Foundational; packaging-only, independent.
- **Polish (P10)** → after all targeted stories.

### Reality note on independence

This is a re-architecture, so the Foundational phase is unusually heavy and US1 is not testable until
the daemon owns a `Term` and streams a grid. The stories remain *independently testable* once
Foundational is done, but they are not independently *deliverable* the way a greenfield feature's
stories would be. That is inherent to replacing the process boundary, not a task-breakdown flaw.

### Parallel opportunities

- Setup: T003 after T001/T002; T004/T005/T006 are largely parallel once crates exist.
- Foundational: protocol types T008/T009/T010 in parallel; their tests T012/T013/T014 in parallel; transport T015–T019 partly parallel with catalog T020–T024.
- Within each story, all `[P]` test tasks run together before implementation.
- With staff: once Foundational closes, US3 (mutations), US5 (exclusivity), US7 (packaging) can proceed alongside the US1→US2 spine.

---

## Implementation Strategy

### MVP (the three P1 stories)

1. Phase 1 Setup → three crates compile, boundaries enforced.
2. Phase 2 Foundational → client connects to daemon, reads catalog. **Largest single investment.**
3. Phase 3 US1 → sessions survive the UI.
4. Phase 4 US2 → attach and drive them.
5. Phase 5 US3 → project/worktree ops through the daemon.
6. **STOP and VALIDATE** against quickstart S1–S3, S8.

### Incremental delivery after MVP

- US4 unattended supervision → US5 exclusivity → US6 contract recovery → US7 logout survival, each
  validated against its quickstart scenario, then Polish (test redistribution accountability + the
  three-platform CI gate).

---

**Bugfix**: 2026-07-27 — BUG-001 Added Phase 11 (T086–T087); reopened then re-closed T045/T046. See
`bugs/BUG-001.md` and `contracts/hooks.md`'s "Configuration" section. Resolved 2026-07-27: T086
landed the matcher/hooks wrapper fix in `settings_json()`; T087's regression coverage folded into
T045's existing unit test; `cargo test --workspace` (98 groups) green.

**Bugfix**: 2026-07-27 — BUG-002 Added Phase 12 (T088–T091) for build-staleness detection (FR-022a).
See `bugs/BUG-002.md`.

**Bugfix**: 2026-07-27 — BUG-004 Added Phase 17 (T101–T104); reopened then re-closed T048. The
activity badge drew raw `●`/`○` literals in iced's default font, which maps neither codepoint, so it
rendered as tofu. Resolved 2026-07-27: the dots are now `Icon::ActivityWorking`/`ActivityEnded`
(`U+E837`/`U+E836`) drawn through `crate::ui::icon(..)`, pinned in `tests/icons.rs` +
`PROVENANCE.md`, and a new `tests/ui_glyph_literals.rs` guard makes a hardcoded glyph a test failure
rather than a runtime blank box. `mise run test` green (104 groups), `cargo clippy -D warnings` and
`cargo fmt --check` clean. See `bugs/BUG-004.md`.

**Bugfix**: 2026-07-27 — BUG-005 Added Phase 19 (T106–T109) to remove the unconditional
`check_circle` from session rows and make the badge slot constant-width. **No task reopened**: T048's
scope was building and rendering the badge, which it did; nothing owned reconciling it with the icon
the row already drew. Implemented 2026-07-27: T106–T108 done (icon dropped; the badge slot is a
`Fixed(sidebar::TAG)` box in every state including `Unknown`; two red-first tests pin both
properties). `mise run test` green (114 groups), `cargo clippy --workspace --all-targets -- -D
warnings` and `cargo fmt --check` clean. **T109 remains open** — the app was launched and connected
without any FR-022a mismatch or daemon restart, but a fresh client opens with worktrees collapsed and
this environment has no input-injection tool to expand one, so the session rows were never seen. See
`bugs/BUG-005.md`.

**Bugfix**: 2026-07-27 — BUG-006 Added Phase 20 (T110–T115) to realign the input-ordering counters
across a client restart, and **reopened T039 and T044**. The client's per-session stamper is
process-lived while the daemon's receiver is session-lived, so every UI restart left pre-existing
sessions silently discarding all input as `Stale`; the contract test missed it by reusing one
counter object across its simulated reattach. Fix makes the daemon's position authoritative
(FR-028a) and makes any discard visible at the shipped log level (FR-045a). Implemented 2026-07-28:
T110–T114 done and **T039/T044 re-closed** — `SessionSummary::input_serial` is published from the
live registry in `overlay_live_summaries` (not the catalog, which cannot see the receiver), the
client adopts it absent-only on `DaemonConnected` and each `CatalogChanged`, the stale drop is a
`warn!` carrying both serials and no bytes (FR-047 intact), and `forget` gained its callers. The
wire moved as anticipated: `PROTOCOL_VERSION` 2 → 3, `SCHEMA_HASH` follows automatically.
`mise run test` green (119 groups), `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo fmt --check` clean. **T115 remains open** — SC-020 is an interactive check and needs a human
at the GUI; note also that this build's own upgrade requires one daemon restart because the contract
moved, so SC-020's package-upgrade path must be observed on the *next* `.deb`. See
`bugs/BUG-006.md`.

---

## Notes

- `[P]` = different files, no incomplete-task dependency.
- Every user-facing story ships its own user-guide doc in the same change (Principle VII).
- Tests are written to FAIL before implementation (Principle I).
- Two brief premises were falsified in planning and are baked into the tasks above: the
  `SessionRouter`/`TerminalBackend` seam carries no production traffic (deleted in T030), and the test
  count is 259 not 63 (T007/T079). `bincode` is dead — grid frames use `postcard` (T010, T015).

---

## Phase 16: Convergence

Produced by `/speckit-converge` after T095–T099 (BUG-003) landed, scoped to the settings/env-include
area (see the run's findings summary — not a full re-audit of every already-`[X]` task).

- [X] T100 Wire `micold-client`'s `Message::SettingsSaved` handler (`crates/micold-client/src/main.rs`, ~L1674-1739) to send `ClientMsg::SettingsSet` (scrollback + the three env-include fields) to the daemon when connected, instead of relying solely on its own direct `JsonFileSettingsStore` write — a change currently only reaches a running daemon after that daemon's next restart (`Catalog::load`/`load_default` read `settings.json` exactly once at boot, `crates/micold-daemon/src/catalog.rs:42-67`), contradicting FR-012a's "a requested change MUST take effect for all sessions" and FR-012b's identical mirror (missing). **Done**: `SettingsSaved` now sends `ClientMsg::SettingsSet` with all four fields as `Some(...)` (matching the existing all-or-nothing local-save semantics — no per-field diffing) whenever `app.daemon` is connected, added a `PendingOp::SettingsSet` so a failure surfaces via the existing generic `notify_error` fallback and a disconnect-before-reply resolves to "unknown" like every other mutating RPC (T055); silently skipped when disconnected (no `notify_error`), since settings-saving already has a fully working local-only path that every other `send_op` caller lacks. **Discovered while implementing**: two more spots needed the identical fix to actually deliver "takes effect for all sessions/clients" (FR-011) rather than just scrollback — `DaemonMsg::SettingsChanged`'s handler (only synced `scrollback_lines`, never the three env-include fields, so another window's — or this client's own echoed-back — change was silently dropped) and `Message::DaemonConnected`'s handler (same gap, for the one-time welcome snapshot). Both now sync all four fields and re-source env-include (`env_include_cache.clear()` + `refresh_env_include`), mirroring the local-save path's own post-save behavior. `daemon::Outbox::new` changed from private to `pub` (not `pub(crate)` — `main.rs` is a separate binary crate from the `micold_client` library, so `pub(crate)` would not reach it) so tests can build a real `Outbox` over a manually-created channel. Added 4 tests in `main.rs`'s `mod tests` covering: the RPC is sent with the right fields; it's a silent no-op while disconnected; `DaemonConnected` adopts the daemon's authoritative env-include settings over a stale local read; `SettingsChanged` syncs all four fields. Verified: `cargo test --workspace` (103 groups) green; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --check` clean.


---

## Phase 17: Bugfix BUG-004 — the activity badge renders as tofu

**Goal**: the session-list activity dot is drawn from raw `●`/`○` literals in iced's default font,
which maps neither codepoint, so every signalled session shows an identical blank box instead of the
filled/hollow dot FR-016d promises. T048 (Phase 4, above) is reopened for this. Fixing it also closes
the gap that let it ship: the build-time tofu guard only covers `Icon::ALL`, so any surface that skips
the enum escapes it (FR-016e, SC-018). See `bugs/BUG-004.md`.

- [X] T101 [US2] Add the activity-dot glyphs to the closed `Icon` enum in
  `crates/micold-client/src/icons.rs` (`Icon::ActivityWorking` filled, `Icon::ActivityEnded` hollow),
  add them to `Icon::ALL`, and render the badge through the existing `icon(..)` helper instead of
  `text("\u{25CF}")` in `crates/micold-client/src/ui/material/activity_badge.rs` (FR-016d, FR-016e).
  Keep the pure `emphasis()` fn and its unit test unchanged — the signal→emphasis decision was never
  at fault. Depends on T048 (reopened above). **Done**: `Icon::ActivityWorking` → `U+E837`
  (`radio_button_checked`), `Icon::ActivityEnded` → `U+E836` (`radio_button_unchecked`); the badge's
  `From<ActivityBadge> for Element` now returns `icon(glyph, badge.size, color)` and no longer
  imports `text`/`style`. `emphasis()` and its test are byte-for-byte unchanged.
  **Correction to this task's own premise**: the codepoints it proposed are wrong. The shipped font
  is a static instance pinned at **FILL=0**, and at that axis value `circle` (`EF4A`), `lens`
  (`E3FA`) and `fiber_manual_record` (`E061`) all rasterize as *rings*, not solid discs — verified
  by rendering them out of the shipped file. Using them would have replaced one bug with a subtler
  one: two identical-looking rings, failing SC-018's shape-distinctness while passing every glyph
  presence check. `radio_button_checked` is the only same-diameter glyph in this file with a
  genuinely filled centre. Rasterized at the real badge size (`sidebar::TAG` = 10px): Working =
  ring with filled centre (ink 21.6), Ended = empty ring (ink 13.2) — visibly distinct by shape.
- [X] T102 [P] [US2] Record the two new glyphs in `assets/fonts/PROVENANCE.md`'s "Curated icon →
  glyph name → codepoint" table and lock their codepoints in `crates/micold-client/tests/icons.rs`,
  matching the existing per-variant rows (feature 004 FR-003 pinning convention). The existing
  `tests/icons_font.rs::every_icon_codepoint_has_a_glyph` then covers them automatically, since it
  iterates `Icon::ALL`. Depends on T101. **Done**: both rows added to the PROVENANCE table plus a
  note recording *why* they are radio-button glyphs (the FILL=0 finding above), so a future
  maintainer picking by icon name is warned. `tests/icons.rs` pins both codepoints and the curated
  set size moves 26 → 28. `every_icon_codepoint_has_a_glyph` passes over the widened `Icon::ALL`.
- [X] T103 [P] [US2] Close the guard gap that let BUG-004 ship: add a source-level test asserting no
  client UI source file hardcodes an icon glyph — the check that would have failed at build time on
  T048's `"\u{25CF}"` (SC-018, feature 004 SC-005). Scope it to `crates/micold-client/src/ui/` so it
  states the real invariant ("every glyph the UI draws comes from `Icon`") without tripping on
  ordinary non-ASCII copy elsewhere. Independent of T101/T102 — it must FAIL against the current
  tree and pass after T101 (Principle I). **Done**: `crates/micold-client/tests/ui_glyph_literals.rs`
  scans every `src/ui/**/*.rs`, extracts string/char literals (skipping comments, decoding `\u{..}`
  escapes) and flags any character in the icon-like Unicode blocks — Geometric Shapes, Dingbats,
  Braille, Arrows, Box Drawing, Private Use Area, Misc Symbols/Pictographs. Confirmed RED first on
  exactly the three real offenders (`activity_badge.rs:77/78/79`), GREEN after T101.
  **Two rejected earlier framings**, both worth recording: (a) scoping the check to "a literal
  passed as the first argument of `text(`" *missed the actual bug*, because the badge passed a
  variable, not a literal — the guard must scan literals wherever they appear; (b) flagging any
  non-ASCII character over-fired on seven pieces of ordinary UI prose (em dashes, ellipses), which
  would have got the test muted rather than obeyed. Naming the symbol blocks is what makes it both
  sound and quiet. A companion `the_guard_actually_works` test asserts the scanner still finds
  `"\u{25CF}"`, still ignores prose and comments, and still sees `activity_badge.rs`, so the guard
  cannot rot into vacuous green.
- [X] T104 [US2] Verify the fix in the running app (`mise run run`): confirm the session list shows a
  filled dot for working / awaiting-input and a hollow one for ended, with zero blank boxes, and that
  the three states are told apart by shape with colour ignored (SC-018). Update `docs/daemon.md`'s
  activity-dot table only if the rendered shapes differ from what T049 documented. Depends on T101.
  **Done, with the GUI walkthrough deferred** (same disposition as T084): the *glyph* half is
  verified deterministically rather than by eye — both codepoints rasterized out of the shipped font
  at the exact badge size (`sidebar::TAG` = 10px) produce real ink in visibly different shapes
  (filled centre vs empty ring), which is the property SC-018 asks for and the one that was broken.
  `docs/daemon.md` needs no change: its table says "Filled" / "Hollow", which still describes what
  renders. The remaining live-app step — seeing a real session transition Working → Awaiting input →
  Ended in the sidebar — needs a human at the GUI with an active `claude` session; it is not
  reproducible headlessly here, and spawning a real agent session in the user's environment to force
  the transitions would be a side effect well beyond the fix.

---

## Phase 18: Convergence

Produced by `/speckit-converge` (2026-07-27), scoped to a bounded sample of the requirement set plus
the plan's technical decisions — not a full re-audit of every already-`[X]` task.

- [X] T105 Upgrade `alacritty_terminal` 0.25 → 0.26.0 across the workspace (`Cargo.toml:54`,
  inherited by `micold-daemon` and `micold-client`) and adapt child-exit handling
  (`ChildEvent::Exited(ExitStatus)`), per `plan: Technical Context` — whose dependency table pins
  **0.26.0** ("upgrade from 0.25") and whose step 3 lists the upgrade — (partial). The code is still
  on 0.25. This restates T005, which cannot be relied on to carry the work: T005 is prefixed
  `**DEFERRED to T030.**`, and T030 closed long ago, so its stated blocker no longer exists and an
  implement pass would reasonably skip it as deferred. Converge is append-only and may not edit T005
  in place, hence this task. **Two things to settle while doing it**, both of which may make "stay on
  0.25" the right answer — record the decision either way rather than leaving the plan and the code
  disagreeing: (a) the wire format sends `alacritty_terminal` enum discriminants verbatim
  (`crates/micold-client/src/ui/terminal.rs:210` — "both processes link the same
  `alacritty_terminal`, so decoding against …"), so the bump must land in both binaries together;
  FR-022a's package-version handshake already refuses a mixed-build pair, so this is a build-ordering
  concern rather than a correctness hole. (b) Plan Decision 2's vendored patch for stable line IDs
  was never taken — `ApproxLineIds` (the plan's own sanctioned no-fork mitigation) is what ships
  behind the `LineIdSource` trait, so the upgrade carries no vendored-patch rebase cost that Risk 1
  anticipated.

  **Done**: workspace pin `Cargo.toml:54` moved `"0.25"` → `"0.26"`; `cargo update -p
  alacritty_terminal` resolved 0.25.1 → 0.26.0 (and pulled `signal-hook` 0.3.18 → 0.4.4
  transitively). Exactly one call site broke, which is the child-exit adaptation T005 predicted —
  though not where it expected it: there is no `ChildEvent` in this tree, the change is
  `Event::ChildExit(i32)` → `Event::ChildExit(ExitStatus)` at
  `crates/micold-daemon/src/terminal.rs:147`. Adapted by storing `code.code()` and documenting the
  resulting ambiguity on `VtSignals::child_exit`: `None` now means "no ChildExit seen **or** signal-
  terminated". That is acceptable here and only here — the accessor has **no callers anywhere in the
  workspace**, and the supervisor's PTY `wait()` is what decides restart policy (FR-005/FR-022), not
  this in-band value. Both settling questions from the task body resolved as anticipated: (a) the
  bump is workspace-wide so both binaries move together, and FR-022a's package-version handshake
  already refuses a mixed-build pair; (b) no vendored-patch rebase cost, since Decision 2's patch was
  never taken and `ApproxLineIds` is what ships. Verified: `mise run test` green (105 groups),
  `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` both clean. This
  also re-closes **T005**, whose `DEFERRED to T030` rationale had expired.

---

## Phase 19: Bugfix BUG-005 — the obsolete `check_circle` icon on every session row

**Goal**: every session row draws an unconditional `Icon::ActiveMarker` (`check_circle`, `U+F0BE`)
immediately left of the activity dot (`crates/micold-client/src/ui/sidebar.rs:489`). It does not vary
with state, so it carries no information while reading as "done / OK" on failed and
interrupted-but-resumable sessions alike, and it competes with the one indicator that *does* vary
(FR-016c, FR-016d). It is a leftover: feature 004 defined `ActiveMarker` as the **active
known-project** marker (`contracts/icon-api.md:70`), and feature 005 (`6aff29b`) reused the variant
as a generic session bullet. Feature 008 removed the equivalent leading icon from *worktree* rows
(its FR-010/T044) but was scoped not to touch session rows, leaving the sidebar half-migrated.

**T048 is not reopened**: its stated scope was building the badge and rendering it, which it did.
The defect is that the pre-existing icon was never reconciled with the new one — no task owned that.

**Validated before patching** (see `bugs/BUG-005.md` §Validation): removing the icon loses no
information. The row's lifecycle tint is applied to the label as well as the icon
(`tree_view.rs:234`), so `Failed`/`Idle`/`InterruptedResumable` colouring still reaches the user via
the session name. The badge is *not* asked to absorb lifecycle — `SessionLifecycle` (process
supervision) and `ActivitySignal` (hooks) are different domains and the badge cannot express
`Failed` or `InterruptedResumable`, nor should it (H1/H2). The one real function the icon performs
is incidental: it is the row's fixed-width leading anchor, which the badge is not, hence T107.

- [X] T106 [US2] Drop `.with_icon(Icon::ActiveMarker)` from `session_tree_item` in
  `crates/micold-client/src/ui/sidebar.rs:489` so the activity badge is the row's sole leading
  indicator (FR-016f). **Keep the `Icon::ActiveMarker` variant and its two legitimate call sites** —
  `src/ui/shell.rs:163` and `src/ui/material/project_switcher.rs:138`, where "active" is a real
  varying property of the row; only the sidebar use is obsolete. No change to `TreeItem`'s optional
  `icon` field: worktree rows and other consumers may still set one. Depends on T107 landing in the
  same change (removing the icon before the slot is constant-width regresses alignment).
  **Done**: the `.with_icon(..)` call is gone from `session_tree_item`
  (`crates/micold-client/src/ui/sidebar.rs`); the comment in its place records *why* the glyph was
  there (feature 005 leftover from the active-known-project marker) so it is not re-added by someone
  reading the row as under-decorated. `Icon::ActiveMarker` and both legitimate call sites
  (`shell.rs:163`, `project_switcher.rs:138`) are untouched, as is `TreeItem`'s optional `icon`
  field — worktree rows and other consumers are unaffected. Landed together with T107.
- [X] T107 [US2] Make the badge slot constant-width in
  `crates/micold-client/src/ui/material/activity_badge.rs:88`: the `Unknown` arm returns a `Space` of
  fixed width `sidebar::TAG` (the badge's own size) instead of `Length::Shrink`, so nothing is drawn
  for `Unknown` (H2 — unchanged) but the slot is still reserved (FR-016f, SC-019). `Unknown` is not
  an edge case: it is the default for a project whose hooks are unconfigured and the state every
  session occupies before its first signal, so without this a hook-less project shows a ragged list
  and a signalled row's name shifts horizontally as it moves unknown → working → ended. This also
  fixes an existing minor defect — a `Shrink` spacer between two elements in a `.spacing(spacing::XS)`
  row contributes *two* XS gaps instead of one, so `Unknown` rows already have a wider leading gutter
  than signalled ones.
  **Done, with a correction to this task's own premise**: fixing only the `Unknown` arm would *not*
  have satisfied SC-019. The drawn arms return `icon(..)`, which is a `Text` widget whose width is
  `Length::Shrink` — so the slot's width would still have been "10.0 for Unknown, whatever the glyph
  advance happens to be otherwise". Material Symbols glyphs advance 1em, so it would have *looked*
  right at `sidebar::TAG` = 10.0 while remaining a font-metric coincidence rather than a guarantee,
  and untestable without pixel measurement. Implemented instead as a `container(inner)` with
  `.center_x(Length::Fixed(badge.size))` wrapping **every** state, so the declared width is
  `Fixed(10.0)` for all four signals; `Unknown` still draws nothing (an empty `Space` inside the
  box), preserving H2. Centring also keeps a future icon inside the box if its advance differs.
- [X] T108 [P] [US2] Assert the invariant so it cannot silently regress (plan Risk 7): a test that
  `session_tree_item` produces a row with no leading icon, and that the label's leading offset is
  identical across all four `ActivitySignal` variants (`Unknown`, `Working`, `AwaitingInput`,
  `Ended`) — the check that would have caught both the stacked glyph and the variable-width slot.
  Must FAIL against the current tree and pass after T106+T107 (Principle I). Note `TreeItem` fields
  are `pub`, so the leading-icon half is assertable without a renderer; prefer asserting the
  constructed `TreeItem`/badge geometry over pixel measurement.
  **Done**: three tests, all confirmed RED against the pre-fix tree and GREEN after T106+T107.
  (a) `ui::sidebar::tests::a_session_row_has_no_leading_icon` — `item.icon.is_none()` and
  `item.badge.is_some()` across all six `SessionLifecycle` variants (RED with "session row for Idle
  still carries a leading icon"). (b)
  `ui::material::activity_badge::tests::the_slot_is_constant_width_in_every_state_including_unknown`
  — `element.as_widget().size().width == Length::Fixed(sidebar::TAG)` for all four `ActivitySignal`
  variants (RED with `left: Shrink, right: Fixed(10.0)`). (c) an added
  `lifecycle_still_reaches_the_row_through_the_tint` pins the property that makes the removal
  safe — `Failed`→`error`, `Idle`→`on_surface_variant`, `InterruptedResumable`→`primary`,
  `Running`→`on_surface` — so a future change that moves lifecycle back onto a glyph, or drops the
  tint, fails here. This one passed before the fix by design: it documents an invariant the fix
  *relies on*, it is not a regression test for the bug. Both new tests live inline
  (`#[cfg(test)] mod tests`) next to the private items they assert, matching `activity_badge.rs`'s
  existing pattern, so no API was widened to make the code testable.
- [ ] T109 [US2] Verify in the running app (`mise run run`): session rows show exactly one leading
  indicator, names in the list all begin at the same horizontal offset, and a row does not shift as
  its signal changes (SC-019). Update `docs/daemon.md`'s activity-dot section only if it describes
  the removed icon. Depends on T106–T107. Expect the same disposition as T104/T084 if driving a real
  session through the transitions needs a human at the GUI — say so explicitly rather than implying a
  walkthrough happened.
  **Left open — the app was launched, but the session rows were never seen.** The build was run
  (`./target/debug/micold-ai-ide`) against the live daemon and it connected cleanly: **no FR-022a
  contract mismatch and no daemon restart**, contrary to what this task anticipated — the debug
  client simply attached and, because the installed client already held the project, displayed the
  US5 takeover banner and went read-only. What blocked the check was mundane: a freshly started
  client opens with every worktree **collapsed**, so no session rows render, and this environment has
  no way to click one open — GNOME Shell's `Eval` is disabled and no input-injection tool exists
  (`ydotool`, `wtype`, `xdotool`, `dotool` all absent under Wayland). Screenshots themselves work via
  `xdg-desktop-portal`; the GNOME `Screenshot` D-Bus method is `AccessDenied` and PIL's X11 grab
  fails `BadMatch` on rootless XWayland.
  **What the screenshots did establish**: the *installed* (pre-fix) client was captured showing the
  bug exactly as reported — a `check_circle` on every session row, including one beside a red
  attention dot on `Modify Rust config to use 4 CPUs max`. That is the "before"; the "after" is the
  missing half.
  **Discharged without it**: the doc half — `grep` over `docs/` finds no reference to `check_circle`,
  `ActiveMarker` or a session-row leading icon, so `docs/daemon.md` needs no change (its activity-dot
  table describes the dot only, which is unchanged). The geometry half is covered deterministically
  by T108, which asserts the two properties SC-019 names (one leading indicator; identical slot width
  across all four signals) at the widget level rather than by eye. What remains is purely visual
  confirmation — expand a worktree in a client running this build and look at the session rows.

---

## Phase 20: Bugfix BUG-006 — sessions that predate a client restart go silently read-only

**Goal**: after the UI is restarted while the daemon keeps running — an upgrade, or a plain quit and
reopen — every pre-existing session stops accepting input. The terminal still renders and streams,
the session still reports `running`, and keystrokes simply vanish. Sessions created *after* the
restart are fine. There is no banner, no notification, and no log entry at the shipped verbosity.

**Root cause**: the input-ordering contract (G2, protocol.md §7) is held by two counters that must
agree, with mismatched lifetimes. The client's `SessionInputStamper` lives in the client **process**
(`App.stamper`, `main.rs:117`, rebuilt by `SessionInputStamper::new()` at `main.rs:352`); the
daemon's `InputReceiver` lives in the **`LiveSession`** (`state.rs:99`), created once at
`register_session` (`state.rs:768`) and never touched by `detach` (`state.rs:414`), because sessions
are deliberately kept alive with no client attached (G4/FR-002). A restarted UI therefore stamps
serial `0` into a session the daemon expects at `N`; `0.cmp(&N)` is `Ordering::Less` →
`InputOutcome::Stale` (`micold-core/src/input.rs:98`) → `session_input` returns without writing to
the PTY (`state.rs:1131-1134`). Every following serial is also `< N`, so the session stays read-only
until the client burns through `N` batches or the daemon restarts. Confirmed against
`journalctl --user -u micold-daemon` for 2026-07-27: the daemon has run since 19:10:48; sessions
driven by client generation 3 are read-only under generation 9, and only `c946c166` — created by
generation 9 at 20:43:37 — accepts input.

**Why it was invisible**: `Stale` is the only branch of the classifier that logs below WARN
(`debug!`, `state.rs:1132`); its three siblings are `warn!` (`state.rs:1117`/`1124`/`1138`). The
shipped unit sets `Environment=MICOLD_LOG=info` (`packaging/micold-daemon.service:20`), so the one
branch that silently discards user keystrokes is the one filtered out — and it never reaches the
FR-046 recent-errors ring either, which captures WARN and above (`logging.rs:89`).

**T039 and T044 are both reopened** (Phase 4, above) — see their notes. T039's fixture reused one
counter object across the simulated reattach, writing the assumption under test into the test's own
setup; T044's "long-lived state" is process-lived, which is not long enough here.

**Design note**: the serial exists to prove no input was lost across a reconnect, which is worth
keeping — so realign the counters rather than removing them. Make the daemon authoritative: publish
each session's expected serial and have a fresh client resume from it. Loss detection still holds
within a client's lifetime, and a new client process starts in step.

- [X] T110 [P] [US2] Carry each session's expected input serial in the catalog snapshot: add the
  field to `SessionSummary` (`crates/micold-core/src/protocol/messages.rs:527`), populated from
  `InputReceiver::expected()` (`crates/micold-core/src/input.rs:103`) in
  `DaemonState::overlay_live_summaries` (`crates/micold-daemon/src/state.rs:307`), beside the
  existing `activity` / `last_title` overlay. **Not in `catalog.rs`**: the catalog is projected from
  the durable `Workspace` and cannot see the live-session registry — `session_summary`
  (`catalog.rs:504`) takes a persisted `&Session`, which is why it hardcodes
  `ActivitySignal::Unknown`. The input position is a runtime-only field and belongs in the overlay
  that already exists for exactly that class of value. This also settles what sessions the daemon is
  not hosting report: the overlay's existing rule is unchanged — no live entry means the catalog's
  default stands, exactly as activity falls back to `Unknown` — so `catalog.rs` supplies a
  documented default (serial `0`) and the overlay replaces it wherever a receiver exists. T111's
  seed-only-when-absent rule keeps that default harmless for a session the client has already
  driven. Additive to the wire contract — note whether `PROTOCOL_VERSION`/`SCHEMA_HASH` must move,
  and if so that FR-021/FR-022a will make stale peers fail loudly rather than silently (the correct
  outcome). Closes part of FR-028a.
  **Done**: `SessionSummary` gained `input_serial: u64`
  (`crates/micold-core/src/protocol/messages.rs`), overlaid from `live.input.expected()` in
  `DaemonState::overlay_live_summaries` (`crates/micold-daemon/src/state.rs`) — the one place both
  snapshot paths (`snapshot_locked` and the per-project projection) already run through, beside
  `activity` and `last_title`. `catalog.rs`'s `session_summary` supplies the documented default `0`
  for a session with no live entry, which is the correct answer rather than a placeholder: the
  daemon has accepted no input for it, so a client starting at `0` is exactly in step. The wire did
  move, as anticipated: `PROTOCOL_VERSION` 2 → 3, and `SCHEMA_HASH` regenerates automatically since
  `build.rs` hashes `messages.rs`. Both peers of a mismatched pair now refuse each other loudly per
  FR-021/FR-022a instead of silently mis-parsing. Covered by
  `crates/micold-daemon/tests/session_start.rs::the_snapshot_publishes_a_live_sessions_expected_input_serial`
  (0 before the session is hosted; the receiver's mark after 7 batches; a stale serial does not move
  the published mark) and by `protocol_roundtrip.rs`, whose fixture carries a non-zero serial so a
  dropped field cannot round-trip.
- [X] T111 [US2] Seed `SessionInputStamper` from that value in the `Message::DaemonConnected` arm
  (`crates/micold-client/src/main.rs:525`), beside the existing resync of `disconnected` /
  `displaced` / settings, and on every later `CatalogChanged` that introduces a session this client
  has no counter for. Seed only when the client holds no counter for that session — an existing
  counter is ahead of the daemon by any input still in flight, and overwriting it would manufacture
  the duplicate-serial case `Stale` exists to reject. Depends on T110. Closes FR-028a.
  **Done**: `SessionInputStamper::seed` (`crates/micold-client/src/input.rs`) adopts a serial via
  `InputSeq::resume_from` through `entry().or_insert_with()` — absent-only by construction, so there
  is no path that rewinds a live counter. A free `seed_input_serials(&mut stamper, &catalog)` in
  `main.rs` walks the snapshot and is called from both catalog-adoption sites: the
  `Message::DaemonConnected` arm (beside the `disconnected`/`displaced`/settings resync) and
  `DaemonMsg::CatalogChanged`. Deliberately **seed-only, never prune** — `reconcile_catalog` does not
  remove sessions either, and for the same reason: a snapshot that predates an in-flight local
  mutation, or an ephemeral daemon reporting an empty catalog, is not evidence of deletion, and
  dropping a counter on it would rebuild it at `0` on the next keystroke and reintroduce this very
  bug. Counters are released explicitly instead (T114). Covered by three tests in `main.rs`'s test
  module: adopting the daemon's mark for a session this client never drove; refusing to rewind a
  counter already ahead of a stale snapshot; and a non-hosted session seeding at `0`.
- [X] T112 [P] [US2] Red-first, in `crates/micold-core/tests/input_ordering.rs` (extend, do not add a
  file — the contract stays in one place): a **fresh** `SessionInputStamper`/`InputSeq` against a
  **surviving** `InputReceiver` applies input rather than discarding it, once seeded per T110/T111.
  Assert the unseeded case is the observed bug (a fresh counter at `0` against a receiver at `N`
  yields `Stale` for `N` consecutive batches) so the regression is pinned from both sides, and
  correct the `input_ordering.rs:80` comment that asserts the reconnect only touches the transport.
  Reopens and closes T039.
  **Done**: red confirmed first — the new tests failed to compile against `InputSeq` until
  `resume_from` existed (`E0599`), and `protocol_roundtrip.rs` failed on the missing field
  (`E0063`). Three tests added: `an_unseeded_restarted_client_has_every_keystroke_discarded` (40
  batches from a fresh counter against a receiver at 40 are all `Stale` and the log does not advance
  once — then the 41st applies, which is the "read-only until I restart the daemon" symptom exactly);
  `a_restarted_client_seeded_from_the_daemon_drives_a_session_it_did_not_start` (the contract); and
  `seeding_preserves_loss_detection_within_the_new_clients_lifetime` (a resumed counter is still
  dense and monotonic, so an in-flight severed keystroke is still reported as `Lost` — the fix does
  not weaken what the serial is for). The `:80` comment is corrected: it now states that reusing one
  counter is the *reconnect* case only, is not the binding case for this feature, and names the test
  that covers a client restart. Whole workspace green (119 test groups).
- [X] T113 [P] Raise the stale-input drop from `debug!` to `warn!` in
  `crates/micold-daemon/src/state.rs:1132` so it is visible at the shipped `MICOLD_LOG=info` level
  and reaches the FR-046 ring. Log the session id and the two serials only, never the bytes —
  `crates/micold-daemon/tests/log_redaction.rs` (T081) must stay green. Closes FR-045a.
  **Done**: now `tracing::warn!` with the session id, the rejected `serial`, and the `expected` one,
  and a message that says the keystrokes are discarded. `expected` is read *after* `accept` — a
  stale serial leaves the high-water mark unmoved, so that is the serial the client should have
  sent, which is the number a reader needs to diagnose a seeding failure. No bytes are logged
  (FR-047); `log_redaction.rs` stays green. The drop is now visible at the shipped
  `MICOLD_LOG=info` and reaches the FR-046 recent-errors ring, which captures WARN and above.
- [X] T114 [P] [US2] Resolve `SessionInputStamper::forget` (`crates/micold-client/src/input.rs:49`):
  it has no caller anywhere in the client, so a counter is never released when a session ends. Either
  call it from the session-close/delete path or delete it. An uncalled lifecycle hook is what let the
  lifecycle mismatch go unnoticed; leaving it uncalled leaves the same trap set.
  **Done — called, not deleted**: `app.stamper.forget(id)` now runs in both session-teardown arms of
  `main.rs`, beside the existing `app.grids.remove(&id)` hygiene — `Message::SessionCloseRequested`
  and `Message::SessionRemoveConfirmed`. Kept rather than deleted because the alternative to an
  explicit release is pruning against the catalog snapshot, and that is the unsafe option here: a
  snapshot is not evidence of deletion (see T111), so a diff-based prune could drop a counter for a
  live session and rebuild it at `0`. Deliberately *not* called on detach — the counter must survive
  a reconnect for loss detection to hold.
- [ ] T115 [US2] Verify in the running app (SC-020): with sessions live, restart **only** the UI
  (quit and reopen, and separately install a `.deb`), then type into a session that predates the
  restart and confirm the keystrokes land on the first try, with no daemon restart. Confirm
  `journalctl --user -u micold-daemon` shows no stale-input warnings during the check. Depends on
  T110–T113. If a human at the GUI is required, say so explicitly rather than implying a walkthrough
  happened (same disposition as T104/T109).
  **Open — a human at the GUI is required (2026-07-28).** Nothing here was walked through. The check
  is by construction interactive (restart the UI, type into a pre-existing session), and the
  environment has the user's own app and daemon running, which are not ours to stop. What *is*
  verified automatically: the daemon publishes the mark and a stale serial never moves it
  (`session_start.rs`), a seeded client resumes at it and an unseeded one loses everything
  (`input_ordering.rs`), and the client seeds absent-only from both catalog paths (`main.rs` tests).
  **Caveat for whoever runs this**: T110 moved the wire (`PROTOCOL_VERSION` 2 → 3, and a new
  `SCHEMA_HASH`), so on *this particular* upgrade the already-running old daemon will refuse the new
  client — loudly, per FR-021/FR-022a — and must be restarted once. That is the designed behaviour,
  not a regression, but it means SC-020's **package-upgrade** path cannot be observed on this build's
  own install; check it on the next `.deb` that does not move the contract. SC-020's
  **quit-and-reopen** path is checkable immediately once the daemon is running this build.

---

## Phase 21: Bugfix BUG-007 — a window that reacquires a project stays read-only

**Goal**: make the client's read-only state follow current attachment ownership as the daemon reports
it, so any accepted attach clears it (FR-024a, SC-021, US5 acceptance scenario 5).

**Root cause**: `App.displaced` decides whether a window suppresses its own input
(`main.rs:1465-1468`) and whether it shows the takeover banner (`connection_status`). It is written
on `DaemonMsg::Displaced` (`main.rs:796`) and on a `ProjectBusy` refusal (`main.rs:808`), but cleared
only by a full reconnect (`main.rs:534`) or the banner's explicit "Take over" button
(`main.rs:869`). `DaemonMsg::Attached` — the daemon confirming this client now holds the project
(`messages.rs:335`, sent from `server.rs:337`) — falls into the catch-all arm (`main.rs:811`) under
a comment calling it uninteresting. It is the one event that falsifies the flag, and the only one of
the three attach outcomes the client ignores. So a window refused project P, that later reacquires P
by an ordinary project switch (`switch_daemon_attachment` → `Attach { force: false }`) once the
holder has gone, renders P and refuses to type into it, above a banner naming a window that may no
longer be running.

**Why it survived**: `crates/micold-daemon/tests/exclusivity.rs` covers US5 with four tests, and all
four assert what the *daemon* sends. The client's only `displaced` coverage
(`connection_status_orders_mismatch_over_displaced_over_disconnected`, `main.rs:2952`) pins the
precedence of connection states given a populated map and never exercises its lifecycle. The daemon
tests prove the project is freed; nothing proves the client notices.

**No task is reopened**: T069's stated scope was the `ConnectionBanner` primitive, the
`ConnectionStatus` computation and the read-only input suppression, and it delivered all three.
Nothing in Phase 8 owned the map's full lifecycle — which is how the gap survived. (Same disposition
as BUG-005's T048.)

**Design note**: the fix is to stop treating "am I displaced?" as state accumulated from events and
start treating it as a fact read off the daemon's replies. The daemon already sends the deciding
answer on every attach; the client need only stop ignoring the positive case. See plan Risk 8, whose
generalisation this bug widened.

- [X] T116 [US5] Clear the project's `displaced` entry on `DaemonMsg::Attached` in
  `crates/micold-client/src/main.rs:811` — give it a real arm instead of leaving it to the
  `_ => {}` catch-all, and update the comment that calls it uninteresting. `Attached` also carries
  `sessions: Vec<SessionSummary>`, which the client currently discards; decide explicitly whether
  that is a second omission or is correctly left to the `CatalogChanged` push, and record which.
  Closes FR-024a.
  **Done**: `DaemonMsg::Attached { project, .. }` now has a real arm that removes the project's
  `displaced` entry, and the comment that called it uninteresting is gone. **Decision on `sessions`:
  correctly ignored, and adopting it would be a bug.** `Attached`'s list is built from
  `DaemonState::sessions_for`, which is `catalog.sessions_for` — the raw durable projection with
  **no** `overlay_live_summaries` pass. Every summary in it therefore carries `activity: Unknown`,
  the persisted label rather than the live OSC-0 title, and `input_serial: 0` even for a session the
  daemon has been driving for hours; a client seeding from it would re-create BUG-006 exactly. The
  authoritative view arrives immediately anyway: `refresh_worktrees_and_send` (`server.rs:1014`)
  sends a `CatalogChanged` built from `catalog_snapshot()` — which *is* overlaid — on the heels of
  every `Attached`. Recorded as a doc comment on `sessions_for` so the next reader does not
  "helpfully" start consuming it.
- [X] T117 [P] [US5] Red-first, in `crates/micold-client/src/main.rs`'s test module (beside the
  existing `connection_status` test): drive `update_inner` with `Refused { ProjectBusy }` for a
  project and then `Attached` for the same project, and assert `active_project_displaced` is false
  and `connection_status` is `Connected`. Assert the pre-fix sequence reproduces the bug so the
  regression is pinned from both sides (as T112 does for BUG-006). Cover the `Displaced` → released
  → `Attached` ordering too, since that is the takeover-shaped route to the same state. Depends on
  T116. Closes SC-021.
  **Done**: four tests, red confirmed by neutralising the `Attached` arm and re-running — three
  failed, and the fourth (`a_refusal_after_an_attach_makes_the_window_read_only_again`) passed, which
  is correct: it guards the direction that already worked, so the fix must not break it.
  `an_accepted_attach_ends_the_read_only_state_a_refusal_started` is the reported sequence;
  `..._a_takeover_started` is the same fix reached via a real `Displaced` rather than a refusal;
  `an_accepted_attach_clears_only_that_project` pins that the map stays per-project, so being handed
  back one project says nothing about another; and the fourth pins that the flag moves in *both*
  directions — clearing on `Attached` must not make it sticky the other way. Driven through
  `update_inner` with real `DaemonMsg` values, so the arm under test is the one the runtime uses.
- [X] T118 [P] [US5] Resolve the stale holder label: the banner's `by` string is the holder as of the
  refusal (`main.rs:796`/`808`) and is never refreshed, so a window can be told it is blocked by a
  window that has since exited. Either re-derive it when shown or state in the type's docs that it is
  a point-in-time label. Not a correctness bug once T116 lands — a window holding the project no
  longer shows the banner at all — but it is the same stale-shadow habit and is worth closing while
  it is in view.
  **Done — documented, not re-derived.** With T116 in place a window holding the project no longer
  shows the banner at all, so the only way to see the label is to genuinely be blocked, and then it
  is accurate as of the event that blocked us. Re-deriving it would mean asking the daemon who holds
  a project we were just refused — a round trip to restate a refusal we already have. The docs on
  `ui::ConnectionStatus::Displaced::by` now say plainly that it is a point-in-time label, never
  refreshed, and must not be read as a claim that the named window is running now.
- [X] T119 [P] [US5] Decide whether `ClientMsg::Detach` should also drop the project's `displaced`
  entry: a window that deliberately leaves a project has no live claim to be displaced from it. With
  T116 in place this is belt-and-braces rather than load-bearing, so the acceptable outcome is a
  recorded decision either way, not necessarily a code change.
  **Done — decided against, no code change.** A `displaced` entry for a project that is not the
  active one is inert: `active_project_displaced` (`main.rs`) reads only the active project, so a
  stale entry can affect nothing until the window returns to that project — and returning sends an
  `Attach`, whose outcome now sets the flag correctly either way (accepted → T116 clears it; refused
  → it is re-inserted with a current holder). Clearing on `Detach` would add a second write path to a
  value whose correctness now rests on a single rule — *the flag is whatever the last attach outcome
  said* — and a second path that happens to agree is how the invariant gets blurred. The map is
  bounded by project count, so there is no leak to argue from either.

**Bugfix**: 2026-07-28 — BUG-007 Added Phase 21 (T116–T119) to make the client's read-only state
follow attachment ownership rather than the history of refusals. **No task reopened**: T069 built the
banner, the status computation and the input suppression as specified; nothing owned the `displaced`
map's exit condition. Same defect shape as BUG-006 in a second pair of values, which is why plan
Risk 8's generalisation was widened rather than a new risk added. Implemented 2026-07-28: T116–T119
all done, **no task reopened and none left open**. `DaemonMsg::Attached` clears the project's
`displaced` entry; its `sessions` payload is deliberately ignored and now documented as unsafe to
adopt (unoverlaid durable projection — seeding from it would re-create BUG-006); the stale `by` label
is documented as point-in-time rather than re-derived; and clearing on `Detach` was decided against
so the flag keeps a single write rule. Four tests, red confirmed by neutralising the arm.
`mise run test` green (119 groups), `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo fmt --check` clean. See `bugs/BUG-007.md`.

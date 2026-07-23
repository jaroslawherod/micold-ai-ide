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
- [ ] T005 **DEFERRED to T030.** Upgrade `alacritty_terminal` 0.25 → 0.26.0 and adapt child-exit handling (`ChildEvent::Exited(ExitStatus)`). Rationale: the alacritty/`ChildEvent` code still lives in `micold-client/src/ui/terminal.rs` (moved untouched, still on 0.25); the daemon has no terminal stack until T030. Bumping now would rewrite code that T030 relocates — the upgrade lands with that move. Client + workspace pin alacritty 0.25 in the interim (one version, no drift).
- [X] T006 Define the stable-line-ID seam behind a `LineIdSource` trait in `crates/micold-daemon/src/terminal.rs`. **Done**: `trait LineIdSource { line_id(offset) -> LineId; oldest_available() -> LineId }` with the no-fork approximation `ApproxLineIds` (per plan Decision 2's mitigation — a line keeps its id as it scrolls, derived from a monotonic `total_lines` watermark minus `retained`, unit-tested for stability + monotonicity). The vendored VT fork (T005, still deferred) can swap in behind this trait without touching the framer. **T005 (alacritty 0.26 upgrade + vendored patch) remains deferred**: the daemon runs on 0.25 with the approximation, which is exactly the swappable-fallback the plan called for; the fork is a later spike gated on measured need (Risk 1).
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
- [X] T021 Implement the Catalog as the single writer of durable state (projects, worktrees, sessions, settings), adopting existing `projects.json`/`settings.json` in place (FR-008, FR-012) in `crates/micold-daemon/src/catalog.rs`. (External-modification detection is out of scope — see spec Out of Scope.) **Done**: wraps the existing `micold-core` stores so the on-disk shape is unchanged — only the writer changes. Provides `snapshot()` → `CatalogSnapshot`, `sessions_for()`, `settings_wire()`, clamped `set_scrollback()`, atomic `persist()`, and surfaces `LoadStatus` (C4 `Recovered` is now reported rather than swallowed). Worktree entries in the snapshot are derived from the durable knowledge (display-name overrides + session bindings); live git branch/status arrives with the worktree RPCs (T053).
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
- [ ] T028 [P] [US1] Integration test in `crates/micold-daemon/tests/reattach_snapshot.rs`: on reattach the client receives a full-snapshot `GridFrame` (not a replay) whose scrollback covers the detached interval, bounded by the scrollback limit (FR-014, FR-017).
- [X] T029 [P] [US1] Test in `crates/micold-daemon/tests/session_isolation.rs` (migrated): two sessions' grids never cross-contaminate (Principle II). **Done**: two real VT sessions each emit a distinct marker; each grid contains only its own. Replaces the deleted in-memory `SessionRouter` byte-routing approximation with an end-to-end check against separate `Term`s (isolation is now structural).

### Implementation — plan W3

- [X] T030 [US1] Split `terminal.rs`: delete `SessionRouter` (zero callers, R0.2); move `Term` ownership into the daemon as one `FairMutex<Term>` per session with a per-session reader thread absorbing the blocking read, in `crates/micold-daemon/src/terminal.rs`. **Done**: `SessionRouter` (only its own test referenced it) removed from `micold-core`; the daemon now owns each session's `alacritty_terminal::Term<DaemonListener>` behind `SharedTerm = Arc<FairMutex<Term>>`. A per-session reader thread absorbs the blocking PTY read and is the sole advancer of the VT parser (research R4). `terminal.rs` also holds the `LineIdSource` seam (T006), the `DaemonListener` (T034), and a `StandardPalette` for color replies. The client's `alacritty` stack stays on 0.25 (T005 upgrade deferred).
- [X] T031 [US1] Implement PTY spawn + child supervision handle (`portable-pty` 0.9, `setsid` free on Unix) in `crates/micold-daemon/src/supervisor.rs`. **Done**: `PtySession` opens a PTY, spawns the child (`spawn_claude`/`spawn_shell`/generic `spawn`), starts the reader thread, and owns child + writer + master + `SharedTerm` + signals. Exposes `write_input`/`resize`/`is_alive`/`kill`/`pid`/`term`/`signals`; `Drop` kills + joins the reader so no thread leaks. **Detached sizing**: the grid size is seeded at spawn and only changed by `resize`, valid with no client attached. **Deferred within this task**: persisting the session UUID (needs catalog wiring, folded into T053) and `SessionResize`-on-attach (needs the attach path, Phase 4).
- [ ] T032 [US1] Implement the shadow-diff framer: depth-1 dirty flag, fixed ~60 Hz tick, stable-`LineId` diff via the `LineIdSource` trait, `oldest_available` watermark on every frame, snapshot-on-attach, resnapshot triggers, in `crates/micold-daemon/src/framer.rs`.
- [ ] T033 [US1] Implement bounded scrollback retention at the daemon (service-owned limit FR-012a) with oldest-first discard, applied even while detached, in `crates/micold-daemon/src/framer.rs` (Edge: detached growth).
- [ ] T033a [US1] Implement **scrollback-by-range**: daemon `ScrollbackRequest` → `ScrollbackResponse` handler (chunked, `more` flag, may return fewer lines than requested as advisory, clamp past `oldest_available` rather than erroring) in `crates/micold-daemon/src/framer.rs`, and client range-fetch on scroll into the grid cache in `crates/micold-client/src/ui/terminal.rs` — so the client scrolls without holding all history (FR-017; quickstart S10).
- [ ] T033b [P] [US1] Test in `crates/micold-daemon/tests/scrollback_range.rs`: a range request returns contiguous lines by `LineId`; a request past the retained watermark clamps; a selection anchored to line IDs is not corrupted by new output arriving mid-scroll (FR-017, FR-018).
- [X] T034 [US1] Answer `EventListener` replies (`PtyWrite`, `ColorRequest`, `TextAreaSizeRequest`) daemon-side, never routed to the client (protocol.md §8). **Done**: `DaemonListener` (installed in every `Term`) answers `PtyWrite` by writing straight back to the PTY, `ColorRequest` via the daemon's `StandardPalette` (xterm 256), and `TextAreaSizeRequest` with the session's current `WindowSize` — all serialized on the one shared PTY writer alongside user input. It also captures OSC-0 `Title` (glyph-stripped, T047 groundwork), records `ChildExit`, and raises the depth-1 dirty flag on new content for the framer. None of these reach the client.
- [ ] T035 [P] [US1] Regression test in `crates/micold-daemon/tests/scroll_cost.rs`: stable-ID diffing stays ~2 lines/frame under a scrolling workload (the 11× property is load-bearing and must not silently regress).
- [ ] T036 [P] [US1] Test in `crates/micold-daemon/tests/slow_client.rs`: a client that stops reading causes no unbounded daemon growth and converges to the true screen on resume (SC-006; Edge: slow consumer).
- [ ] T037 [US1] User-guide doc: what persistence guarantees hold across close/crash/rebuild, in `docs/daemon.md` (Principle VII, FR-042).

**Checkpoint**: Sessions survive the UI. This plus US2 is the demonstrable MVP.

---

## Phase 4: User Story 2 — Attach, drive, detach (Priority: P1) 🎯 MVP

**Goal**: Attach to a waiting session, read the current screen, type an answer, watch it proceed,
detach — with local interactions (scroll, select, resize) never blocking on a round trip, and the
session list showing which sessions are working vs awaiting input.

**Independent Test**: With a session blocked on a prompt and no UI, launch UI, answer, observe effect,
close UI; cold start attaches in < 3 s (quickstart S2, S3).

### Tests FIRST (Principle I) ⚠️

- [ ] T038 [P] [US2] Test in `crates/micold-client/tests/local_interactions.rs`: scroll, select, resize issue zero round trips (FR-020, SC-004/005).
- [ ] T039 [P] [US2] Test in `crates/micold-core/tests/input_ordering.rs`: `SessionInput.serial` is monotonic and input is never coalesced, dropped, or reordered across a detach/reattach boundary (G2; Edge: clock/ordering).
- [ ] T040 [P] [US2] Test in `crates/micold-daemon/tests/activity_signal.rs`: hooks drive Working/AwaitingInput/Ended; absent hooks yield `Unknown` never AwaitingInput (H1); a spinner-glyph title yields Working only, never a move toward AwaitingInput (H1a).

### Implementation — plan W4 + W5 (activity) + W3 (titles)

- [ ] T041 [US2] Retarget `App.terminals` to a daemon handle + per-session wire grid cache; split the ~25 `terminals.get_mut()` sites into fire-and-forget commands vs local-cache queries (FR-020) in `crates/micold-client/src/ui/terminal.rs`.
- [ ] T042 [US2] Retarget `TerminalPane` from `&RuntimeTerminal` to the wire grid cache, keeping its builder shape (`TerminalPane::new(..).focused(..)`) in `crates/micold-client/src/ui/material/terminal_pane.rs`.
- [ ] T043 [US2] Implement the client-side selection model + text extraction (new work, R0.3 — selection was a mutation of the shared `Term`) anchored to `LineId` so new output cannot corrupt it (FR-018; Edge: selection under new output).
- [ ] T044 [US2] Implement client-side keymap → byte translation sending `SessionInput` (FR-019); `Drop for App` sends `Goodbye`/disconnect, never a kill.
- [ ] T045 [US2] Implement the loopback HTTP hook receiver: bind `127.0.0.1`/`::1` only, ephemeral port, per-session bearer token, 403 on mismatch, bounded bodies, no body logging, in `crates/micold-daemon/src/hooks.rs` (contracts/hooks.md).
- [ ] T046 [US2] Implement the activity FSM (hooks + `Event::Title` spinner as Working-only evidence, invariant H1a; transcript JSONL as explicitly-degraded fallback) in `crates/micold-daemon/src/activity.rs`; write the per-session `--settings` file so user config is never modified.
- [ ] T047 [US2] Adopt `Event::Title` (OSC 0) as the push-based session-title source, stripping the leading status glyph by codepoint range and treating the text as untrusted; retire the 120 ms transcript rescan (`src/main.rs:754`) and the lossy path-slug transform (`src/provider.rs:361-373`).
- [ ] T048 [P] [US2] Build the `activity_badge` shared primitive (builder-into-`Element` API, Principle VIII) in `crates/micold-client/src/ui/material/activity_badge.rs`; render it in the session list for every session (FR-016d).
- [ ] T049 [US2] User-guide doc: attach/detach flow and what the working / awaiting-input / unknown badges mean, in `docs/daemon.md` (Principle VII).

**Checkpoint**: US1 + US2 together deliver the MVP — persistent sessions you can drive.

---

## Phase 5: User Story 3 — Project and worktree management still works (Priority: P1)

**Goal**: Add/rename/remove projects and create/rename/delete worktrees through the daemon, with
specific actionable failures, visible pending state, and cross-client propagation.

**Independent Test**: Perform each op; confirm correct result, specific failure messages with git's
stderr preserved, and a second window observing the change (quickstart S8).

### Tests FIRST (Principle I) ⚠️

- [ ] T050 [P] [US3] Test in `crates/micold-daemon/tests/mutation_semantics.rs`: worktree-create failures (branch exists, path collision, read-only parent) return a specific `GitFailed` error with git's stderr verbatim, no catalog entry, no leftover directory (FR-034; Edge cases).
- [ ] T051 [P] [US3] Test in `crates/micold-daemon/tests/mutation_atomicity.rs`: a `req` lost to disconnect resolves client-side to explicit **unknown**, never success/failure, and settles by reading authoritative state on reconnect (FR-031/035).
- [ ] T052 [P] [US3] Test: `WorktreeDelete` on a worktree with a live session and `stop_sessions:false` fails specifically rather than orphaning the process (W2; Edge: delete worktree with live session).

### Implementation

- [ ] T053 [US3] Implement the correlated mutating-request handlers (`ProjectAdd/Remove/Rename`, `WorktreeCreate/Delete/Rename`, `SessionCreate/Delete`, `SettingsSet`) resolving to exactly one `OperationOk`/`OperationError` in `crates/micold-daemon/src/catalog.rs`; every git call a named RPC, no `GitRun` escape hatch (FR-009).
- [ ] T054 [US3] Fix the current dropped-error violation: worktree deletion errors must surface (was discarded at `src/main.rs:783-784`) (FR-032, Principle III).
- [ ] T055 [US3] Implement client-side pending/disabled control state per in-flight `req` (no duplicate submission) and the unknown-outcome resolution on reconnect in `crates/micold-client/`.
- [ ] T056 [US3] Implement empty-session pruning that runs **only** for a project with an attached client (FR-007a) in `crates/micold-daemon/src/catalog.rs`.
- [ ] T057 [US3] User-guide doc: project/worktree operations now run through the daemon, in `docs/daemon.md` (Principle VII).

**Checkpoint**: All P1 stories complete — full MVP with everyday operations intact.

---

## Phase 6: User Story 4 — Unsupervised sessions are supervised anyway (Priority: P2)

**Goal**: With no client attached, an unexpectedly-exited session restarts under the same retry
policy; a crash-loop past the limit settles in a durable `Failed` state reported on next attach.

**Independent Test**: With no UI, kill a session's process → restart; force repeated failures past the
limit → give-up state on next attach (quickstart S4).

### Tests FIRST (Principle I) ⚠️

- [ ] T058 [P] [US4] Test in `crates/micold-daemon/tests/supervision_restart.rs`: an unattended process exit triggers restart with the same retry policy as when attached (FR-005); normal `exit` marks stopped, no restart (FR-004 scenario 3).
- [ ] T059 [P] [US4] Test: after `MAX_RESTART_ATTEMPTS` the session settles `Failed { reason, attempts }`, persisted, and is surfaced on the next attach (FR-005).

### Implementation

- [ ] T060 [US4] Implement the restart FSM (retry counter, `MAX_RESTART_ATTEMPTS`, give-up → durable `Failed { reason, attempts }`) in `crates/micold-daemon/src/supervisor.rs`, identical whether or not a client is attached.
- [ ] T061 [US4] Implement per-OS process-tree teardown behind the supervision abstraction — `killpg` on Unix, job object on Windows — in `crates/micold-daemon/src/platform/{unix,windows}.rs` (FR-036); teardown not gated on reader EOF (ConPTY).
- [ ] T062 [US4] Surface the `Failed` state (reason + attempt count) in the session list via `SessionSummary` (FR-016a `Ended`).
- [ ] T063 [P] [US4] Document the retry policy and the L5 caveat (counter has no time window) in `docs/daemon.md`.

**Checkpoint**: Attended and unattended failure handling are provably identical.

---

## Phase 7: User Story 5 — One viewer per project, with deliberate takeover (Priority: P2)

**Goal**: A second client on a held project is refused with a takeover offer; on takeover the first
drops to a disconnected-but-running state, sends zero further input, and can reconnect once free.

**Independent Test**: Two windows on one project → refusal + takeover affordance → take over → displaced
window visibly disconnected, sends no input, does not exit (quickstart S6).

### Tests FIRST (Principle I) ⚠️

- [ ] T064 [P] [US5] Test in `crates/micold-daemon/tests/exclusivity.rs`: `Attach{force:false}` on a held project is refused with `ProjectBusy{holder,since}`; `force:true` displaces the holder (FR-023).
- [ ] T065 [P] [US5] Test: a `Displaced` client stops rendering/sending for that project, does not terminate; a crashed holder frees the project without a daemon restart (FR-024; Edge: holder dies).
- [ ] T066 [P] [US5] Test in `crates/micold-daemon/tests/exclusivity.rs`: two clients on two different projects do not interfere (FR-024 scenario 4).

### Implementation — plan W6

- [ ] T067 [US5] Implement one-attachment-per-project with reject-by-default and force-takeover, emitting `Displaced` to the prior holder, in `crates/micold-daemon/src/main.rs` (FR-023–025).
- [ ] T068 [US5] Implement half-open detection via a **3 s-`Ping` / 9 s-deadline** keepalive (worst-case < 10 s per SC-011) and free the project on a crashed holder, in `crates/micold-client/src/daemon_conn.rs` + daemon `Pong` responder (FR-026; Edge: half-open connection).
- [ ] T068a [P] [US5] Test in `crates/micold-daemon/tests/liveness.rs`: a silently half-open connection (no FIN) is surfaced as `Disconnected` in **≤ 10 s**, and a healthy connection is never spuriously reaped (SC-011).
- [ ] T069 [US5] Build the `connection_banner` shared primitive (builder API, Principle VIII) in `crates/micold-client/src/ui/material/connection_banner.rs` for the disconnected/takeover states; wire the reconnect action (FR-028).
- [ ] T070 [P] [US5] User-guide doc: exclusivity, takeover, and the second-test-client workflow, in `docs/daemon.md` (Principle VII).

**Checkpoint**: The daily two-window collision is handled without input fighting.

---

## Phase 8: User Story 6 — Contract mismatch fails loudly and recoverably (Priority: P2)

**Goal**: A rebuilt client against an older daemon is refused with a diagnostic naming both versions;
a one-click restart swaps the daemon; previously-live sessions come back interrupted-resumable, never
auto-relaunched.

**Independent Test**: Launch a client whose version/hash differs from the running daemon → refusal +
working restart action; a live session returns interrupted-resumable (quickstart S7, S5).

### Tests FIRST (Principle I) ⚠️

- [ ] T071 [P] [US6] Test in `crates/micold-daemon/tests/version_recovery.rs`: on daemon restart, previously-running sessions load as `InterruptedResumable` — distinct from `Running` and from a deliberate `Idle` stop — and no process is auto-relaunched (FR-006a/b; Edge: identity exists, process does not).
- [ ] T072 [P] [US6] Test: `SessionStart` on an `InterruptedResumable` session is the single explicit action that resumes the prior conversation (FR-006a).

### Implementation

- [ ] T073 [US6] Implement the `InterruptedResumable` lifecycle state and its persistence/reload on daemon start (FR-006a/b) in `crates/micold-core/src/session.rs` + `crates/micold-daemon/src/catalog.rs`.
- [ ] T074 [US6] Implement the client's version-mismatch diagnostic and "restart daemon" action (stop old, start matching, attach; warn live processes lost but sessions resumable) in `crates/micold-client/` (FR-022).
- [ ] T075 [P] [US6] User-guide doc: what a contract mismatch looks like and how restart-and-resume behaves, in `docs/daemon.md` (Principle VII).

**Checkpoint**: The developer's own rebuild loop fails safe and recovers in one click.

---

## Phase 9: User Story 7 — Surviving logout on Linux (Priority: P3)

**Goal**: On Linux, documented `loginctl enable-linger` lets sessions survive logout; it is never
enabled silently; macOS/Windows docs state the limitation plainly.

**Independent Test**: On Linux with linger enabled, log out/in → session survived; without it → ends;
docs state per-platform support (quickstart S14).

### Implementation — plan W6 (packaging)

- [ ] T076 [P] [US7] Ship systemd **user** units (`packaging/micold-daemon.socket`, `.service`) — shipped but NOT enabled at install; the client enables in-session.
- [ ] T077 [US7] Implement the in-session enable path (client offers to enable the user service / linger; never automated by install) in `crates/micold-client/src/daemon_conn.rs`.
- [ ] T078 [P] [US7] User-guide doc: `loginctl enable-linger` instructions with the "enable linger THEN start the daemon — not retroactive" ordering warning; explicit statement that logout survival is Linux-only (FR-038, Principle VI exception).

**Checkpoint**: All user stories functional.

---

## Phase 10: Polish & Cross-Cutting Concerns — plan W6 (tests, docs) + validation

**Purpose**: Test redistribution accountability, diagnostics surfacing, and the cross-platform gate.

- [ ] T079 Redistribute all 259 pre-split tests to owning crates (pure logic → `micold-core`, supervision/protocol/lifecycle → `micold-daemon`, render-coupled → `micold-client`) with a per-test disposition record (moved / rewritten / retired-with-reason); **silent deletion forbidden** (FR-041). Gate: `cargo test --workspace` green with the T007 baseline accounted for.
- [ ] T080 Implement the diagnostics surface end to end: `LogLocationRequest`/`RecentErrorsRequest`/`SetLogLevel` (runtime `EnvFilter` reload) and the client UI that shows the log location and recent daemon errors (FR-043–046, SC-017).
- [ ] T080a Place the mandated FR-045 log-event call sites: startup/shutdown with reason; endpoint bind and bind failure; client attach/detach/refusal/takeover with reason; session start/exit/restart-attempt/give-up with reason; every mutating-operation failure with the underlying diagnostic preserved. Assert their presence in `crates/micold-daemon/tests/log_events.rs` (FR-045; T025 built the infra, this places the calls).
- [ ] T081 [P] Verify no terminal content or user input appears in any log entry — grep a typed string against the log file in `crates/micold-daemon/tests/log_redaction.rs` (FR-047).
- [ ] T082 [P] Cross-cutting documentation review + index/navigation in `docs/` (Principle VII).
- [ ] T083 CI gate: build + full test suite pass on Linux, macOS, and Windows before merge; exercise Windows job-object teardown, `0x03` interrupt (NOT via `cmd.exe`), and the inverted `portable-pty` `kill()` result (Principle VI, Risk 3).
- [ ] T084 Run all quickstart.md scenarios S1–S15 and record outcomes.
- [ ] T085 [P] Measure retargeted `TerminalPane` repaint cost at 60 Hz on the client (Risk 2 — all streaming measurements were daemon-side); record whether the tick rate is the right knob.

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

## Notes

- `[P]` = different files, no incomplete-task dependency.
- Every user-facing story ships its own user-guide doc in the same change (Principle VII).
- Tests are written to FAIL before implementation (Principle I).
- Two brief premises were falsified in planning and are baked into the tasks above: the
  `SessionRouter`/`TerminalBackend` seam carries no production traffic (deleted in T030), and the test
  count is 259 not 63 (T007/T079). `bincode` is dead — grid frames use `postcard` (T010, T015).

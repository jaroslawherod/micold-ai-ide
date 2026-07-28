# Implementation Plan: Daemon-Backed Session Persistence

**Branch**: `feat/micold-daemon` | **Date**: 2026-07-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/010-daemon-session-persistence/spec.md`

**Phase 0**: [research.md](./research.md) · **Phase 1**: [data-model.md](./data-model.md),
[contracts/](./contracts/), [quickstart.md](./quickstart.md)

---

## Summary

Split the application into a headless user-space daemon that owns every PTY, the VT emulation, all
durable state and all git operations, and a thin iced client that attaches to one project at a time
to view and drive sessions. Sessions survive closing, crashing and rebuilding the UI on all three
platforms; on Linux they additionally survive logout via user-enabled lingering.

The technical shape that fell out of Phase 0:

- **Transport**: `interprocess` 2.4.2 (tokio feature) with explicit per-OS filesystem paths, one
  framed stream carrying a JSON control plane and `postcard` grid frames behind an encoding tag.
- **Streaming**: the daemon holds a per-client cursor and diffs *last-known → now* on a fixed
  ~60 Hz tick with a depth-one dirty flag. Frames are never queued, so a slow client converges to
  the current screen instead of lagging.
- **Diff keying**: stable absolute line IDs, not viewport indices — measured 11× fewer lines per
  frame under scrolling, which is the dominant terminal workload.
- **Activity signal**: Claude Code hooks posted to a loopback endpoint, replacing the output-
  quiescence heuristic the spec assumed, which measurement falsified. The agent's OSC 0 terminal
  title is adopted separately as a push-based **session-title** source, replacing a 120 ms UI-thread
  poll — but is explicitly barred from the activity decision (it fails the same way).
- **Supervision**: the reader thread that already exists per session absorbs the blocking `wait()`;
  process-tree kill is free on Unix via `setsid`+`killpg` and needs a job object on Windows.

Three spec requirements needed amendment before implementation. All three are now
[resolved](#spec-amendments--resolved-2026-07-21). Two engineering decisions remain open — see
[Open decisions](#open-decisions).

---

## Technical Context

**Language/Version**: Rust, **latest stable** — `rust-version` bumped from 1.80. *(Decision 1,
settled 2026-07-21: bump rather than add `fd-lock`; prefer current versions of dependencies
generally.)* std `File::lock`, used for the single-instance lock, stabilized in 1.89.0 and is
therefore available without a dependency.

**Primary Dependencies**:

| Purpose | Crate | Version | Notes |
|---|---|---|---|
| GUI (client only) | `iced` | 0.13 | unchanged; Principle V |
| IPC transport | `interprocess` | 2.4.2 | `tokio` feature |
| Async runtime | `tokio` | 1.53 | daemon; client already has it via iced |
| Framing | `tokio-util` | 0.7.18 | `LengthDelimitedCodec` |
| Control plane | `serde_json` | 1.0.151 | already a dependency |
| Grid frames | `postcard` | 1.1.3 | **not bincode — dead crate** |
| VT emulation | `alacritty_terminal` | **0.26.0** | upgrade from 0.25; ⚠️ vendored patch, see R8.7 |
| PTY | `portable-pty` | 0.9 | unchanged; ⚠️ Windows `kill()` returns inverted results |
| Logging | `tracing`, `tracing-subscriber` | 0.1.44 / 0.3.23 | |
| Log rotation | `file-rotate` | 0.8.0 | `tracing-appender` cannot bound total disk |
| systemd fd adoption | `listenfd` | 1.0.2 | compiles on all three platforms |
| Windows job objects | `windows-sys` | 0.61.2 | ⚠️ ~9 months stale, re-check before pinning |
| Single-instance lock | std `File::lock` | 1.89+ | no dependency; MSRV settled |

**Storage**: Local files only (Principle IV). `projects.json` and `settings.json` in the existing
`directories`-derived data dir, adopted **in place**. The daemon becomes the single writer,
eliminating the current silent-clobber hazard (`src/store.rs` has no locking).

**Testing**: `cargo test --workspace` — the headless suite runs without the client crate
(`-p micold-core -p micold-daemon`), so nothing under test pulls in iced (Principle I, FR-040).
Current baseline: **259 tests across 43 files** (not the 63 the brief assumed), redistributed to
owning crates in W6.

**Target Platform**: Linux, macOS, Windows desktop (Principle VI). Daemon runs headless with no
graphical environment (FR-039).

**Project Type**: Desktop application, now a Cargo workspace — two binary crates
(`micold-daemon`, `micold-client`) plus a shared render-free `micold-core` crate.

**Performance Goals**: Steady-state output visible without perceptible lag (SC-004); session/project
switch presenting correct screen within 200 ms (SC-005); attach-from-cold under 3 s (SC-003);
activity-state transition within 5 s with zero spurious transitions during multi-minute agent work
(SC-016). Measured headroom: a full 80×24 frame is 2,353 B / 4.2 µs, ~15 KB/s with stable-ID
diffing — the wire is not the constraint.

**Constraints**: No unbounded memory in either process under sustained high-volume output (SC-006);
the PTY reader must never block on a client; daemon must never exit while a session is alive
(FR-002); log disk usage hard-capped (FR-044); endpoint reachable only by the owning user (FR-030).

**Scale/Scope**: ~10–50 concurrent sessions across several projects, one attached client per project.
Thread-per-session is comfortably within budget at this scale and is required anyway because
`portable-pty`'s reader is a blocking `Read`.

---

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design — result unchanged.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every workstream below is ordered test-first. The protocol,
      exclusivity, reconnect, unattended-supervision and error-semantics behaviours get failing
      integration tests before implementation. The daemon is testable headlessly (no iced), which
      *improves* on today's position where `spawn_pty`, the reader thread, `pump`, `has_exited` and
      `handle_process_exits` have **no automated coverage at all** and rely on manual quickstarts.
- [x] **II. Multi-Session Support**: Sessions remain independently addressable and isolated; the
      daemon strengthens this by making persistence real rather than best-effort. Per-session state
      is per-session in the daemon (`FairMutex<Term>` each), and cross-session leakage is covered by
      the migrated `session_isolation.rs`.
- [x] **III. Worktree Integration**: Worktree lifecycle moves wholesale into the daemon and stays
      app-owned; the user still never runs git by hand. FR-032 additionally *fixes* a current
      violation — worktree deletion errors are presently discarded (`src/main.rs:783-784`).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: All state stays on the local filesystem. The IPC
      endpoint is a Unix socket / named pipe, never a network socket. **One qualification**: the
      Claude Code hook receiver is an HTTP listener — it MUST bind `127.0.0.1` only, carry a
      per-session bearer token, and expose no capability beyond reporting activity. Recorded in
      [Complexity Tracking](#complexity-tracking).
- [x] **V. Rust + iced Stack**: Rust throughout; iced remains the only GUI framework and is confined
      to the client binary. The daemon has no iced dependency. Type-system work: session lifecycle
      and attachment state become explicit enums so invalid states (attached-but-disconnected,
      running-without-process) are unrepresentable.
- [~] **VI. Cross-Platform Parity**: All *functional* behaviour is equivalent on the three
      platforms, with platform differences confined behind two narrow abstractions (endpoint
      location; process supervision). **One deliberate, spec-sanctioned exception**: surviving user
      logout is Linux-only (FR-038). Recorded in [Complexity Tracking](#complexity-tracking).
- [x] **VII. Documentation First-Class**: FR-042 requires user-guide documentation in the same change
      covering the daemon, its lifecycle, takeover behaviour, per-platform persistence guarantees,
      and the `loginctl enable-linger` instructions.
- [x] **VIII. Reusable UI Component Foundation**: The client gains disconnected-state, takeover-
      prompt and activity-indicator affordances. These MUST be added as shared primitives in
      `src/ui/material/` using the chainable builder-into-`Element` API, not feature-local one-offs.
      `TerminalPane` is retargeted from `&RuntimeTerminal` to a wire grid cache but keeps its
      existing builder shape (`TerminalPane::new(..).focused(..)`).
      **Bugfix 2026-07-27 (BUG-004)**: "shared primitive" is not satisfied by a new `material/`
      module alone — a primitive that draws a glyph MUST source it from the shared `Icon` vocabulary
      (feature 004 FR-002/FR-003) and render it through the `icon(..)` helper in the Material Symbols
      font. `activity_badge.rs` satisfied the module rule but hardcoded `"\u{25CF}"`/`"\u{25CB}"`
      into `text(..)`, which draws in iced's default font (Fira Sans); neither that font nor the
      shipped Material font maps those codepoints, so the badge rendered as tofu. Reaching outside
      `Icon` also escapes the `tests/icons_font.rs` build-time guard entirely (FR-016e, SC-018).
      **Bugfix 2026-07-27 (BUG-005)**: adding a shared primitive to a row does not discharge the
      obligation to *reconcile it with what the row already renders*. `activity_badge.rs` was pushed
      into `TreeItem`'s badge slot beside a pre-existing, unconditional `Icon::ActiveMarker`
      (`check_circle`) that feature 005 had installed as a generic session bullet — leaving two
      leading glyphs, one carrying no state. A new indicator primitive MUST subsume or displace the
      affordance it duplicates, and a primitive that renders nothing in one of its states MUST still
      occupy a constant-width slot so the row does not reflow around it (FR-016f, SC-019).

---

## Project Structure

### Documentation (this feature)

```text
specs/010-daemon-session-persistence/
├── plan.md              # This file
├── research.md          # Phase 0 — R0..R8, decisions + unverified items
├── data-model.md        # Phase 1 — entities, states, invariants
├── quickstart.md        # Phase 1 — runnable validation scenarios
├── contracts/
│   ├── protocol.md      # Wire protocol: framing, envelope, handshake, errors
│   ├── messages.md      # Full client↔daemon message surface
│   └── hooks.md         # Claude Code hook receiver contract
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
Cargo.toml               # [workspace] — three members, resolver = "2"

crates/
├── micold-core/         # render-free — no iced, no portable-pty. The compiler enforces FR-040.
│   ├── Cargo.toml       #   deps: serde, postcard, serde_json only; NO iced, NO portable-pty
│   └── src/
│       ├── lib.rs
│       ├── protocol/    #   shared wire types; the ONLY crate both binaries depend on
│       │   ├── envelope.rs  #   framing header, encoding/kind tags
│       │   ├── messages.rs  #   ClientMsg / DaemonMsg enums
│       │   ├── grid.rs      #   WireLine, WireStyle, GridFrame, WireCursor
│       │   └── version.rs   #   PROTOCOL_VERSION + build-time SCHEMA_HASH, exact-match handshake
│       ├── session.rs   #   interrupted-resumable state, activity signal
│       ├── store.rs     #   catalog types; daemon becomes sole writer
│       ├── workspace.rs
│       ├── git.rs       #   invoked only by the daemon crate
│       ├── worktree.rs
│       └── settings.rs  #   scrollback_lines is daemon-owned (FR-012a)
│
├── micold-daemon/       # binary `micold-daemon` — CANNOT name an iced type (no iced dep)
│   ├── Cargo.toml       #   deps: micold-core, tokio, interprocess, portable-pty, alacritty_terminal
│   └── src/
│       ├── main.rs      #   startup: endpoint bind / lock / systemd fd adoption
│       ├── endpoint.rs  #   per-OS path policy + permissions + FR-029a length assertion (R1.2/1.3)
│       ├── singleton.rs #   connect → lock → RE-CHECK → bind (R1.4)
│       ├── supervisor.rs #  spawn, restart FSM, exit detection
│       ├── platform/    #   the ONE process-supervision abstraction (FR-036)
│       │   ├── unix.rs  #     killpg; setsid comes free from portable-pty
│       │   └── windows.rs #   job object per session
│       ├── terminal.rs  #   Term ownership, FairMutex, stable line IDs, Event::Title
│       ├── framer.rs    #   shadow diff, depth-1 dirty flag, tick
│       ├── catalog.rs   #   projects/worktrees/sessions; single writer
│       ├── activity.rs  #   FSM over hooks + Event::Title (Working-only), invariant H1a
│       ├── hooks.rs     #   loopback HTTP receiver for activity signals
│       └── logging.rs   #   tracing init, context detection, bounded rotation
│
└── micold-client/       # binary `micold-ai-ide` — iced; CANNOT name a portable-pty type
    ├── Cargo.toml       #   deps: micold-core, iced, tokio, interprocess (client half)
    └── src/
        ├── main.rs
        ├── keymap.rs    #   client-side (daemon is keyboard-agnostic, FR-019)
        └── ui/
            ├── terminal.rs   # RuntimeTerminal REPLACED by a daemon handle + grid cache
            └── material/
                ├── terminal_pane.rs      # retargeted to the wire grid; client-side selection
                ├── connection_banner.rs  # NEW shared primitive (builder API)
                └── activity_badge.rs     # NEW shared primitive (builder API); glyphs MUST come
                                          #   from `icons::Icon`, never raw literals (BUG-004);
                                          #   sole leading indicator on a session row, and its slot
                                          #   is constant-width incl. Unknown (BUG-005)

packaging/
├── micold-daemon.socket # systemd user unit
└── micold-daemon.service

tests/                   # 259 existing tests — redistribute to the owning crate, never delete silently
                         # core logic → micold-core; supervision/protocol → micold-daemon;
                         # each crate's tests/ compiles without the other binary's deps.
```

**Structure Decision**: **Cargo workspace, three crates** (`micold-core`, `micold-daemon`,
`micold-client`). `micold-core` is the render-free shared crate carrying the protocol types; it
declares neither iced nor portable-pty, so FR-040 becomes a compile error rather than a convention.
`micold-daemon` links the PTY/VT stack and has no iced dependency; `micold-client` links iced and
cannot name a PTY type. The dependency graph *is* the architecture boundary.

*Rationale*: chosen over the cheaper single-crate feature split on Settled Decision 8's criterion —
prefer a structural guarantee (the boundary cannot silently drift) over the low-cost path (the
boundary held only by CI convention). The cost is redistributing 259 tests across 43 files into their
owning crate, tracked as its own workstream (W6) with per-test disposition recorded (FR-041). This
supersedes the earlier feature-split recommendation.

---

## Implementation Workstreams

Ordered by dependency. Each is test-first per Principle I. This is *not* the task breakdown —
`/speckit-tasks` produces that — but it fixes the sequence, because two of these are prerequisites
that gate everything downstream.

### W0. Prerequisites (gates everything)

1. Convert the repo to a **Cargo workspace**: create `micold-core`, `micold-daemon`,
   `micold-client`; move today's `src/` into the owning crate. `micold-core`'s manifest declares
   neither iced nor portable-pty — this is the compile-time enforcement of FR-040. Confirm the full
   suite still passes (259) after redistribution (see W6 for the per-test disposition).
2. Bump `rust-version` to **latest stable** and pull dependencies to current versions (Decision 1).
   The single-instance lock uses std `File::lock`; no `fd-lock`.
3. Upgrade `alacritty_terminal` 0.25 → 0.26.0. Only child-exit handling changes
   (`ChildEvent::Exited` now carries `ExitStatus`, not `i32`).
4. Vendor the **stable line ID** patch (Decision 2), behind a trait so the no-fork approximation
   stays a swappable fallback. Everything in W3 depends on it, and the approximation breaks
   permanently once scrollback saturates — a few seconds of `cat` at the default 10,000-line history.

### W1. Protocol and transport

Shared `protocol` module in `micold-core`; `LengthDelimitedCodec` with an explicit
`max_frame_length`; the encoding/kind tag envelope; strict exact-match handshake on **both
`PROTOCOL_VERSION` and a build-time `SCHEMA_HASH`** (FR-021, Decision 4), the diagnostic naming both
sides' version and hash. The hybrid encoding (JSON control, `postcard` grid) with the
`MICOLD_WIRE=json` switch built. Endpoint policy per OS with the macOS length assertion (FR-029a),
the `/tmp` fallback ownership verification, and the Windows DACL. The singleton dance with its
**re-check after lock acquisition** (R1.4).

**Schema hash**: a `build.rs` in `micold-core` hashes the canonical text of the protocol type
definitions (`protocol/messages.rs`, `grid.rs`, `envelope.rs`) into a `const SCHEMA_HASH`. Editing a
message struct without bumping `PROTOCOL_VERSION` changes the hash, so the handshake refuses two
builds that disagree about the bytes — the exact failure gRPC's tolerant schema would hide. It is
strictly stricter than a version integer, not a replacement for it.

*Tests first*: version **or** schema-hash mismatch is refused with both sides named; a struct edit
without a version bump changes `SCHEMA_HASH` (guards the guard); a frame exceeding the cap is
rejected loudly; a `postcard` grid frame and a JSON control frame round-trip under both `MICOLD_WIRE`
settings; two simultaneous starters converge on one daemon; a stale socket is reclaimed; a socket
whose parent directory has wrong ownership causes a loud bail, not a silent bind.

### W2. Daemon skeleton, state ownership, lifecycle

Move `JsonFileStore`, `Workspace`, the project/worktree catalog, `git.rs` and `worktree.rs` behind
daemon RPCs. Adopt `projects.json` in place. Implement never-exit-while-a-session-lives (FR-002),
logging with context detection and the bounded rotating sink, and the client-side spawn-and-retry.

*Tests first*: mutations resolve to exactly one of success / specific failure / explicit unknown
(FR-031); a failed worktree creation leaves no catalog entry and no directory (FR-032); git stderr
survives the RPC boundary intact (FR-034); the daemon refuses to exit while a session is alive.

**Design correction (BUG-003)**: T053's original scope named `env_include` resolution as part of
this workstream ("main-sync" note added once feature 011 landed on `main`), but no corresponding
code was ever written in `micold-daemon` — all three of the daemon's own PTY-spawn call sites
(`start_session`, `respawn_primary`, `open_shell` in `state.rs`) hardcode a `TERM`-only environment.
Fixing this (FR-012b) needs two things W2 did not originally account for: (1) `DaemonSettings`
(the wire projection `Catalog::settings_wire()` produces) must gain the three `env_include_*`
fields already present on `micold_core::settings::Settings`, and `ClientMsg::SettingsSet` must
accept changes to them — mirroring the existing `scrollback_lines` precedent (FR-012a) exactly, not
a new mechanism; (2) each of the three spawn sites must call `micold_core::env_include::resolve`
(with `merge_with_term`) against the session's own directory before building its `env`, with a
per-directory cache owned by `DaemonState`/`Catalog` (there is no `App` in the daemon to hold one,
unlike `micold-client`'s existing `env_include_cache: HashMap<PathBuf, EnvIncludeSnapshot>`),
invalidated on a `SettingsSet` that changes any of the three fields and on a worktree's deletion.
See `bugs/BUG-003.md`.

### W3. Terminal ownership and grid streaming

`Term` per session behind `FairMutex`; the reader thread absorbs the blocking `wait()`; shadow
line-hash diff keyed by stable IDs; depth-1 dirty flag and fixed tick; snapshot-on-attach;
resnapshot triggers; scrollback range requests with `oldest_available` on every frame.

Also here: **session titles move to `Event::Title`** (OSC 0), which the VT parser already produces,
retiring the 120 ms transcript-JSONL rescan on the UI thread (`src/main.rs:754`) and the lossy
path-slug computation (`src/provider.rs:361-373`). Strip the leading status glyph by codepoint range;
treat the text as untrusted PTY input.

*Tests first*: `scrollcost`-style regression asserting stable-ID diffing stays at ~2 lines/frame
under scrolling (the 11× property is load-bearing and must not silently regress); a client that
stops reading causes no unbounded growth and converges on resume; `EventListener` replies
(`PtyWrite`, `ColorRequest`, `TextAreaSizeRequest`) are answered daemon-side and never routed to the
client; a title carrying a spinner glyph yields the stripped name **and never** an activity
transition toward awaiting-input (H1a).

### W4. Client retargeting

`App.terminals` becomes a daemon handle plus a per-session grid cache. The ~25 `terminals.get_mut()`
sites split into fire-and-forget commands and local-cache queries so no interaction blocks on a
round trip (FR-020). **Client-side selection model and text extraction** — this is new work the
brief did not anticipate (R0.3). `Drop for App` disconnects instead of killing. Disconnected-state
and takeover affordances as shared builder-API primitives.

*Tests first*: rendering, scrolling and selection never issue a round trip; `Drop` sends no kill;
input is never coalesced or reordered across a detach/reattach boundary.

**Bugfix 2026-07-27 (BUG-006)**: "across a detach/reattach boundary" is not the binding case and
must not be the only one tested. This feature's premise is that the daemon outlives the UI, so the
ordinary event is a **new client process** attaching to a session it did not start — which destroys
any client-process-lifetime state the ordering contract leans on, while the daemon's side of that
contract survives untouched. Client retargeting therefore owns a resync step: on `DaemonConnected`,
seed the per-session input counters from the daemon's authoritative position (FR-028a) rather than
starting them empty. Test the *restart* boundary explicitly — a second stamper against a surviving
receiver — not just a reconnect that reuses the same counter object.

### W5. Supervision and activity signal

Restart FSM in the daemon with identical attended/unattended behaviour (FR-005); process-tree kill
behind the platform abstraction; the `0x03` interrupt path; the Claude Code hook receiver and the
working/awaiting-input/ended state machine, with **missing hooks reported as unknown, never idle**.

*Tests first*: unattended restart is indistinguishable from attended in attempt count and give-up
state (SC-012); give-up state persists and is reported to the next attaching client; a session with
no hooks configured reports unknown rather than a wrong answer.

### W6. Exclusivity, reconnect, packaging, docs, test redistribution

One-client-per-project with force-takeover (FR-023–025); half-open detection via a 3 s-ping / 9 s-deadline keepalive (SC-011 ≤10 s; analysis I1)
(FR-026); resync-by-reading-current-state on reconnect (FR-028); systemd units shipped but not
enabled at install, with the client enabling in-session; user-guide documentation (FR-042).

**Build-staleness detection (FR-022a, BUG-002)**: the FR-021 handshake refuses only on a
`PROTOCOL_VERSION`/`SCHEMA_HASH` mismatch, so a `.deb` upgrade whose daemon-side change doesn't touch
the wire schema — the common case — never trips it, and the new client silently attaches to the old,
already-running daemon via the `AlreadyRunning` singleton path. The original plan was to compare the
existing `client_build`/`daemon_build` diagnostic strings directly, but those turned out non-viable:
they carry different program-name prefixes (`"micold-ai-ide/…"` vs `"micold-daemon …"`) that can
never be equal even on an identical release. Landed instead: a dedicated `PACKAGE_VERSION` constant
(`crates/micold-core/src/protocol/version.rs`, backed by the workspace-shared `CARGO_PKG_VERSION`,
which `release-please` bumps on every release regardless of wire-schema changes) exchanged via a new
`client_package_version` field on `Hello`. `PROTOCOL_VERSION` bumped 1→2 accordingly (that new field
is itself wire-visible). A same-contract package-version difference refuses with a new
`RefusalReason::BuildMismatch` variant, distinct from `VersionMismatch`, reusing FR-022's client-side
restart action — without the "live processes will be lost" warning, since a matching contract means
nothing is actually at risk. `client_build`/`daemon_build` remain diagnostic-only, named in both
refusal kinds' banners.

**Bugfix**: 2026-07-27 — BUG-002 Added build-staleness detection to W6 (FR-022a). Resolved
2026-07-27: `PACKAGE_VERSION` + `RefusalReason::BuildMismatch` landed (T088–T091); see
`bugs/BUG-002.md`.

**Test redistribution** (FR-041): move each of the 259 tests to its owning crate — pure-logic tests
to `micold-core`, supervision/protocol/lifecycle to `micold-daemon`, any render-coupled tests to
`micold-client`. Record a per-test disposition (moved / rewritten against the daemon / retired with
reason); **silent deletion is forbidden**. The gate is `cargo test --workspace` green with the
pre-split count accounted for.

*Tests first*: a second attach is refused with an actionable error; after takeover the displaced
client sends zero further input and does not exit; a project held by a crashed client becomes
attachable without restarting the daemon.

---

## Spec amendments — RESOLVED 2026-07-21

Phase 0 falsified or contradicted three things the approved spec asserted. All three were reviewed by
the user and are now applied to `spec.md` (see its *Amendments from planning* section).

### A1. FR-016b mandated a mechanism that cannot work — ✅ REWRITTEN

> Original: "'Awaiting input' MUST be determined from observable session behavior using a defined,
> documented **quiescence threshold**…"

Measured against live `claude` v2.1.215/2.1.216: idle-at-prompt max output gap **6.02 s**; *working*
on a 25 s tool call max gap **20.50 s**. The working case is quieter than the idle case. The agent's
own OSC 0 title — investigated at the user's suggestion — reproduces the same failure: mid-`sleep 30`
the glyph reverts to `✳`, byte-identical to the idle glyph, then goes dark for **26.03 s**. Every
other PTY-derived signal also measured dead (OSC 133 absent, bracketed paste never toggled, all BEL
bytes were OSC-title terminators, `tcgetpgrp` returns `ENOTTY` on macOS, process state identical
whether busy or idle).

**Applied**: FR-016b now requires an authoritative agent-emitted signal and *prohibits* quiescence
inference; **unknown** is a first-class state in FR-016a. Mechanism: Claude Code hooks to a
loopback-only, token-authenticated endpoint (contracts/hooks.md). Invariant **H1a** makes the
falsified inference class structurally impossible — no PTY-derived evidence may ever move a session
toward `AwaitingInput`.

**Adopted separately from the same investigation**: OSC 0 carries the agent's generated session name,
pushed on change. It replaces the current transcript-JSONL rescan **every 120 ms on the UI thread**
(`src/main.rs:754`) with an event the daemon's VT parser already produces, and removes the lossy
path-slug computation (`src/provider.rs:361-373`) from the title path. Folded into W3.

### A2. FR-029's macOS endpoint guidance does not fit — ✅ FR-029a ADDED

macOS `sun_path` is **104 bytes**; Application Support measures 88/103 for a typical username and
99 for a corporate AD username, exceeding the limit once any discriminator is added and failing with
an opaque `EINVAL`. **Applied**: FR-029a requires the endpoint to fit the platform limit under
realistic usernames and to assert it at bind time with a named error. Chosen path:
`$HOME/.micold/run/d.sock` (55/103), fallback `_CS_DARWIN_USER_CACHE_DIR`.

### A3. FR-010 scrollback-limit ownership — ✅ NO CHANGE NEEDED

Re-read after the clarify pass: FR-010 already states "The scrollback limit is NOT client-owned (see
FR-012a)" and Settled Decision 4 already restricts the client to per-window presentation state. The
inconsistency I flagged had been fixed during `/speckit-clarify`. Recorded rather than silently
dropped.

---

## Open decisions

### Settled 2026-07-21

| # | Decision | Outcome |
|---|---|---|
| 1 | MSRV | **Bump to latest stable**, and prefer current versions of dependencies generally. Removes the `fd-lock` dependency in favour of std `File::lock`. |
| 2 | Vendored `alacritty_terminal` patch for stable line IDs | **Yes.** ~3 lines (`pub scrolled_total: u64` on `Grid`, incremented at `Grid::scroll_up`). Accepted as a VT-engine maintenance commitment. ⚠️ Risk 1 stands: "cheap to rebase" is inferred from code stability, **not tested** — W0 must implement it behind a trait so the no-fork approximation stays swappable. |
| 3 | Feature split vs **workspace split** | **Workspace split.** Three crates — `micold-core`, `micold-daemon`, `micold-client` — so the render-free boundary (FR-040) and the iced-free daemon (Principle V) are enforced by the compiler, not by convention. Chosen on Settled Decision 8's criterion (structural guarantee over cheap path), which supersedes the migration-cost argument for the feature split. |
| 4 | Wire format / transport | **Hand-rolled framed protocol, kept.** No gRPC/protobuf — its tolerant-evolution model works against the exact-match contract (Settled Decision 3), HTTP/2 fits local IPC poorly, independent streams break total ordering, and protobuf is larger/slower on the grid hot path. **Added: a build-time schema hash** over the protocol type definitions, exchanged in the handshake beside `PROTOCOL_VERSION`; mismatch refuses, naming both hashes. This closes the real gap (struct edited, version not bumped) and is strictly stricter than protobuf. **Hybrid JSON-control / `postcard`-grid split retained**, justified by building the `MICOLD_WIRE=json` debug switch. |

**On decision 3 — settled: workspace split.** My original recommendation (feature split first) rested
on migration cost across 43 test files. Settled Decision 8 explicitly excludes implementation cost as
a criterion and prefers structural guarantees over convention. Applying the user's own stated
criterion instead, the **workspace split wins**: three crates (`micold-core`, `micold-daemon`,
`micold-client`) make the render-free boundary a compile error rather than a code-review habit —
the client crate cannot name a PTY type, the daemon cannot name an iced type, and FR-040 stops being
something a future change can quietly violate. Recorded as a correction to my own reasoning, not a
new finding.

**On decision 4 — settled: hand-rolled protocol + schema-hash handshake, no gRPC.** For one
structural reason plus three practical ones.

The structural reason: protobuf's central design property is *tolerant evolution* — unknown fields are
silently ignored, absent fields silently take defaults. That is a deliberate accommodation for systems
where the two ends are deployed independently and must interoperate across versions. Here they are
built and shipped together, and Settled Decision 3 already chose the opposite contract: exact-match
version, no negotiation, refuse on mismatch. Adopting a framework whose schema layer is engineered to
paper over mismatches, inside a design whose stated priority is loud early failure over silent drift,
would work against the requirement rather than for it.

Practically: gRPC is HTTP/2, so it wants a network-ish transport — `tonic` over a Unix socket needs a
custom connector and Windows named pipes need more glue than they're worth. It also models RPC as
independent streams, which breaks the total-ordering guarantee in `contracts/messages.md` §1 (a
`SessionResize` must be ordered against the grid frames around it — they cannot live on separate
streams). And protobuf on the grid hot path is both larger and slower than `postcard`, on a wire whose
representation rules carry a measured 15× size win.

**What the concern is actually pointing at is real, though, and cheaper to address directly**: the
failure mode worth defending against is not "old client meets new daemon" — the handshake already
refuses that — it is *someone edits a message struct and forgets to bump `PROTOCOL_VERSION`*, so both
ends claim compatibility while disagreeing about the bytes. Proposed mitigation: derive a **schema
hash** over the protocol type definitions at build time and exchange it in the handshake alongside the
version. Mismatch refuses with both hashes named. That closes the actual gap with a few dozen lines and
no framework, and it is strictly stricter than protobuf, not looser.

**Settled**: keep the hand-rolled framed protocol; add the schema-hash handshake; build the
`MICOLD_WIRE=json` debug switch and therefore keep the hybrid JSON-control/`postcard`-grid split — a
control plane you can read by eye is the justification for carrying two encodings.

---

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| **Principle VI** — logout survival is Linux-only | FR-038 scopes it deliberately. Research confirms the asymmetry is real: Linux needs an unprivileged `loginctl enable-linger`; macOS requires a root-installed LaunchDaemon with TCC complications; Windows requires a service. | Achieving parity means a privileged installer on two platforms — a substantially larger investment for a secondary guarantee. Mitigation: the daemon's internal design keeps it *open* (install modelled as a separate, fallible, possibly-elevated step; no ambient session-scoped resources; session-independent transport). ⚠️ A macOS LaunchAgent with `LimitLoadToSessionType=Background` **might** collapse this to Linux-level cost — undocumented folklore, worth a half-day test. |
| **Principle IV** — an HTTP listener in a local-first app | The only reliable busy/idle signal is Claude Code hooks, which deliver over HTTP (A1). | PTY-derived detection was measured and does not work. Mitigation: bind `127.0.0.1` only, per-session bearer token, no capability beyond reporting activity, never a route to project state. This is loopback IPC, not network access — nothing leaves the device. |
| **Vendored fork of `alacritty_terminal`** | Stable line IDs are unavailable through the public API and the entire streaming efficiency rests on them. | The no-fork approximation is exact only until scrollback saturates, then silently wrong. See Open decision 2. |
| **Three-crate workspace instead of one crate** | The render-free boundary (FR-040) and iced-free daemon (Principle V) must be *enforced*, not merely intended. Separate crates make a violation a compile error. | A single crate — even with a `--daemon` flag or `gui`/`daemon` features — holds the boundary only by convention and CI; a stray `use` reintroduces iced into the daemon graph silently. Settled Decision 8 prefers the structural guarantee; the cost is redistributing 259 tests (W6). |

---

## Risks

1. **The vendored VT patch is the highest-consequence unknown.** It gates W3, and its rebase cost is
   inferred rather than measured. *Mitigation*: implement it in W0 behind a trait so the no-fork
   approximation remains a swappable fallback for a spike.
2. **iced-side rendering cost is entirely unexamined.** All the streaming measurements are
   daemon-side. If the client cannot repaint at 60 Hz, the tick rate is the wrong knob and this
   research does not reveal it. *Mitigation*: measure the retargeted `TerminalPane` early in W4.
3. **Windows is unexercised.** The job-object and detached-spawn code is compile-verified against
   `windows-sys` 0.61.2 but **never executed on Windows**; `portable-pty`'s `kill()` is known-broken
   there; the `0x03`→ConPTY interrupt path is well-sourced but untested. *Mitigation*: CI must cover
   all three platforms before W5 closes (Principle VI's gate already requires this).
4. **Claude Code hook behaviour is a moving target.** The `Notification` subtypes did not fire during
   testing. *Mitigation*: the unknown state is a first-class value, so an unfired signal degrades
   rather than lies.
5. **259 tests, not 63 — and the workspace split now *forces* every one to move to an owning crate.**
   *Mitigation*: W6 tracks redistribution with a per-test disposition record (FR-041 forbids silent
   deletion); W0 gates on `cargo test --workspace` staying green through the crate move.
6. **The build-time tofu guard is only as wide as `Icon::ALL`.** `tests/icons_font.rs` (feature 004
   T005) proves every enum variant resolves to a glyph, but it cannot see a surface that skips the
   enum and passes a literal to `text(..)` — exactly how BUG-004 shipped. *Mitigation*: a
   source-level guard test that fails on non-ASCII glyph literals in client UI code (T103), so the
   invariant is defended rather than merely documented.
7. **A new row indicator can silently stack on an inherited one.** Nothing forces a feature adding an
   indicator to audit what the row already draws, so feature 010's activity badge landed beside
   feature 005's unconditional `check_circle` and shipped two leading glyphs — one of them
   meaningless — for the life of the feature (BUG-005). *Mitigation*: FR-016f makes "sole leading
   indicator" an explicit requirement, and T108 asserts the label offset is identical across all
   four `ActivitySignal` variants, so a re-added constant glyph or a variable-width slot fails a
   test rather than waiting to be noticed by eye.
8. **State split across the client/daemon boundary can have mismatched lifetimes, and the test that
   should catch it can encode the mismatch as its premise.** The input-ordering contract is held by
   two counters that must agree: the client's is process-lived, the daemon's is session-lived. The
   daemon is *designed* to outlive the client, so the two diverge on every UI restart and the daemon
   then discards the client's input as out-of-order — silently, since only that branch of the
   classifier logs below the shipped verbosity (BUG-006). The contract test missed it because it
   simulated a reconnect by reusing the same counter object, making the assumption under test into
   the test's own setup. *Mitigation*: FR-028a makes the daemon's position authoritative and requires
   the client to adopt it on connect; FR-045a forces any discard to be visible at the shipped log
   level; and the T039 extension exercises a genuinely fresh stamper against a surviving receiver.
   *Generalisation*: for any pair of values that must agree across the boundary, name which side is
   authoritative and re-read it on connect — do not assume symmetry of lifetimes.

---

**Bugfix**: 2026-07-27 — BUG-003 Updated from bugfix patch.

**Bugfix**: 2026-07-27 — BUG-004 Updated from bugfix patch: annotated Principle VIII with the
shared-`Icon` sourcing rule, marked the `activity_badge.rs` structure entry, and added Risk 6 (the
tofu guard's blind spot). See `bugs/BUG-004.md`.

**Bugfix**: 2026-07-27 — BUG-005 Updated from bugfix patch: annotated Principle VIII with the
subsume-what-you-duplicate rule and the constant-width-slot rule, extended the `activity_badge.rs`
structure entry, and added Risk 7 (a new indicator stacking on an inherited one). See
`bugs/BUG-005.md`.

**Bugfix**: 2026-07-27 — BUG-006 Updated from bugfix patch: annotated W4's *Tests first* line with
the client-restart resync step (the binding boundary is a new client process, not a reconnect), and
added Risk 8 (mismatched lifetimes across the client/daemon boundary, and a contract test that
encodes the mismatch as its premise). See `bugs/BUG-006.md`.

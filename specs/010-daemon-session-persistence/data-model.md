# Phase 1 Data Model: Daemon-Backed Session Persistence

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

Entities, their ownership side, state machines, and the invariants that must hold. Wire message
shapes live in [contracts/messages.md](./contracts/messages.md); this file is about *what exists and
what must remain true*.

---

## Ownership map

The dividing line from the spec: **the daemon owns anything that must remain true when no UI is
running, plus every mutation of project state. The client owns only how this particular window
looks.**

| Entity | Owner | Durable | Notes |
|---|---|---|---|
| Project catalog | Daemon | Yes — `projects.json` | Single writer (FR-008) |
| Project | Daemon | Yes | |
| Worktree | Daemon | Yes | Created/removed only by daemon RPC |
| Session (identity, title, lifecycle) | Daemon | Yes | Identity assigned at creation (FR-006) |
| Session process + PTY | Daemon | No | Dies with the daemon; identity survives |
| `Term` / VT state / scrollback | Daemon | No | Authoritative; the client never parses VT |
| Activity signal | Daemon | No | Derived from hooks; `Unknown` on restart |
| Scrollback limit | Daemon | Yes — `settings.json` | **FR-012a** — moved from client during clarify |
| Attachment (who holds a project) | Daemon | No | Rebuilt on connect |
| Theme preference | Client | Yes | Per-window |
| Window geometry | Client | Yes | Per-window |
| Viewport offset | Client | No | Per-window (FR-018) |
| Text selection | Client | No | Per-window; **new client-side model** (see R0.3) |
| Grid cache | Client | No | A projection; discarded on disconnect |

---

## Daemon-side entities

### Catalog

The single durable aggregate, adopted in place from the existing `projects.json`
(`SCHEMA_VERSION = 1`). Shape is unchanged from `src/store.rs`; what changes is that exactly one
process writes it.

```text
Catalog
├── schema_version : u32
├── last_active    : Option<PathBuf>
└── projects       : Vec<Project>
```

**Invariants**

- **C1** — Exactly one process holds write access for the lifetime of the daemon. Enforced by the
  single-instance lock (R1.4), not by file locking on the catalog itself.
- **C2** — Writes are atomic: temp file + rename, as today. A partially written catalog is never
  observable.
- **C3** — A failed mutation leaves the catalog byte-identical to its pre-mutation state (FR-032).
  Catalog mutation is therefore the **last** step of any compound operation, after filesystem and
  git side effects have succeeded.
- **C4** — On unparseable input, the file is preserved as `.json.bak` and an empty catalog is loaded
  with status `Recovered` — existing behaviour, retained, and now surfaced to the client rather than
  swallowed.

### Project

```text
Project
├── path                  : PathBuf     -- identity; lexically canonicalised, never fs-resolved
├── display_name          : String
├── is_git_repo           : bool
├── availability          : Available | Unavailable    -- recomputed, never persisted
├── sessions              : Vec<Session>
└── worktree_display_names: BTreeMap<String, String>
```

**Invariants**

- **P1** — `path` is the identity. Canonicalisation stays **lexical** (`src/project.rs:115`); making
  it filesystem-resolving would make identity machine-dependent.
- **P2** — At most one `Attachment` per project at any instant (FR-023).
- **P3** — `availability` is derived state and is never written to disk.

### Worktree

```text
Worktree
├── dir_name  : String      -- the identity a Session binds to
├── branch    : Option<String>
├── status    : Clean | Missing | Locked | Prunable
└── path      : <project>/.claude/worktrees/<dir_name>    -- convention, daemon-owned
```

**Invariants**

- **W1** — The path convention is constructed in exactly one place in the daemon. It is currently
  hardcoded at four sites in `src/main.rs`; that duplication does not survive the move.
- **W2** — A worktree with any live session cannot be deleted without an explicit confirmed stop of
  those sessions first. Never silently orphaned.
- **W3** — Creation is all-or-nothing. On failure the rollback plan runs (`worktree::rollback_plan()`)
  and no catalog entry is written (C3).

### Session

The central entity. **Identity and process are separate lifetimes** — this is the whole feature.

```text
Session
├── id            : SessionId (Uuid)   -- assigned at creation, persisted, never reused
├── worktree_dir  : String             -- binds to a Worktree by directory name
├── label         : Pending | Named(String)
├── lifecycle     : SessionLifecycle   -- see state machine below
├── restart_count : u8
└── activity      : ActivitySignal     -- see below; NOT persisted
```

**Invariants**

- **S1** — `id` is assigned before the process starts and persisted immediately, so a session is
  resumable even if the process never starts (FR-006). This is what makes protocol-version
  restarts acceptable.
- **S2** — A session is bound to exactly one worktree for its whole life.
- **S3** — `lifecycle` and `activity` are runtime state and are **not** persisted, with one
  exception: `Failed` (crash-loop give-up) and its reason **must** persist, because FR-005 requires
  reporting it to a client that attaches much later.
- **S4** — Killing the client never changes any session's lifecycle (FR-007).

#### SessionLifecycle state machine

```text
                     ┌──────────────────────────────────────────┐
                     │                                          │
   [create] ──► Idle ──start──► Starting ──proc up──► Running ───┤
                 ▲                  │                    │       │
                 │                  │ spawn fails        │ clean exit
                 │                  ▼                    ▼       │
                 └───────────────  Failed ◄──give up── Restarting│
                                     │      (attempts   ▲   │    │
                                     │       exhausted) │   │    │
                                     └──start (manual)──┘   │    │
                                                            │    │
                        unexpected exit ────────────────────┘    │
                                                                 │
   [daemon restart] ──► InterruptedResumable ──resume──► Starting┘
```

- `Idle` — identity exists, no process. Reachable at creation and after a clean stop.
- `Starting` — spawn requested, process not yet confirmed up.
- `Running` — process alive.
- `Restarting { attempts }` — unexpected exit, retry in progress. **The counter lives in the
  variant**, which is why `mark_running()` resets it.
- `Failed` — retries exhausted, or spawn failed. Manually restartable. **Persisted** (S3).
- `InterruptedResumable` — **new** (FR-006a). The daemon restarted and found a durable record of a
  session that was running. Visually distinct from both `Running` and a deliberately stopped
  session, and **never auto-relaunched** (FR-006b).

**Transition invariants**

- **L1** — `MAX_RESTART_ATTEMPTS = 3` yields **two** actual restarts (exit 1 → `Restarting{1}`,
  exit 2 → `Restarting{2}`, exit 3 → `Failed`). Preserved from `src/session.rs:153-166`. The
  off-by-one against the natural reading of FR-005 is deliberate and documented.
- **L2** — Behaviour is **identical whether or not a client is attached** (FR-005, SC-012). No
  branch anywhere in the FSM may consult attachment state.
- **L3** — A clean exit (user typed `exit`) transitions to `Idle`, never `Restarting`.
- **L4** — Daemon startup may only produce `InterruptedResumable`, never `Starting` (FR-006b) — a
  service restart can never cause an agent to act unasked.
- ⚠️ **L5 — known gap**: the restart counter has **no time window**, and `mark_running()` resets it.
  A session that crashes once an hour never trips the guard. Pre-existing
  (`src/session.rs:142`), but FR-005 moves it into an unattended context where it matters more.
  Flagged for an explicit decision during implementation.

#### ActivitySignal

```text
ActivitySignal = Unknown | Working | AwaitingInput | Ended { reason }
```

Derived from Claude Code hooks (research R4.3), **not** from output quiescence.

| Source | Result |
|---|---|
| `UserPromptSubmit`, `PreToolUse` | `Working` |
| `Stop` | `AwaitingInput` |
| `Notification` (`permission_prompt`, `idle_prompt`, `agent_needs_input`) | `AwaitingInput` ⚠️ subtypes unverified |
| Process exit / give-up | `Ended { reason }` |
| OSC 0 title with a spinner glyph (`Event::Title`) | `Unknown → Working` **only** — never a transition toward `AwaitingInput` |
| Hooks unconfigured, or no signal yet | **`Unknown`** |

**Invariants**

- **A1** — `Unknown` is a first-class value and **must never be rendered as `AwaitingInput`**. Hooks
  are config-dependent; a user with `--bare` or conflicting settings silently loses them, and
  guessing "idle" would produce exactly the false attention signal FR-016c forbids.
- **A1a** — Terminal-derived evidence is monotone toward `Working` only. The OSC 0 spinner is
  positive evidence of work; its absence and the idle glyph carry no information (measured: the glyph
  reverts to `✳` mid-tool-call). Nothing observed on the PTY may move a session toward
  `AwaitingInput` — only hooks and process exit may. Mirrors hooks.md **H1a**.
- **A2** — `AwaitingInput` and `Ended` are notification-grade; `Working` and `Unknown` are ambient
  (FR-016c).
- **A3** — Reported for every session regardless of whether it is being viewed (FR-016a, FR-016d).
- **A4** — Not persisted; resets to `Unknown` on daemon restart.

### Attachment

```text
Attachment
├── project    : PathBuf
├── client_id  : ClientId       -- per-connection, ephemeral
└── since      : Instant
```

**Invariants**

- **T1** — At most one attachment per project (P2). A second attach is refused with an actionable
  error offering takeover (FR-023).
- **T2** — Released on disconnect **for any reason including crash**, without restarting the daemon
  (FR-025). Since the connection owns the attachment, EOF is the release signal.
- **T3** — Takeover is atomic: the displaced client is notified and the new one attached with no
  window in which both or neither hold it.
- **T4** — A displaced client stops rendering and sending input for that project but **does not
  exit** (FR-024).

### SessionShadow (streaming state)

Per-session daemon state supporting the diff. Not durable.

```text
SessionShadow
├── generation   : u64                  -- bumped on resize / alt-screen / reset
├── seq          : u64                  -- monotonic frame sequence
├── line_hashes  : HashMap<LineId, u64> -- stable id -> content hash
├── viewport_top : LineId
├── oldest_id    : LineId               -- trim watermark
├── cursor       : WireCursor
└── dims         : (u16, u16)
```

### ClientCursor (per attached client, server-held)

```text
ClientCursor
├── acked_seq  : u64
├── generation : u64            -- mismatch forces resnapshot
├── viewport   : Range<LineId>
└── dirty      : bool           -- DEPTH ONE
```

**Invariants**

- **F1** — `dirty` is depth-one. **Frames are never queued** — only the intent to send one. This is
  what makes a slow client converge to the current screen rather than lag (FR-015).
- **F2** — The client never sends a sequence number; the daemon holds the cursor (wezterm model).
- **F3** — Memory is `O(viewport + shadow)` per client, never `O(output rate)` (SC-006).
- **F4** — A `generation` mismatch forces a full snapshot; diffing across a resize or alt-screen
  switch is meaningless because line identities change.

---

## LineId — the load-bearing identifier

```text
LineId : i64 = scrolled_total + history_size + line.0
```

Monotonic over session lifetime, never reused. `scrolled_total` requires the vendored
`alacritty_terminal` counter (research R8.7) — **this is the one place the plan needs a patched
dependency**, and the 11× scroll efficiency rests on it.

**Invariants**

- **I1** — Absolute and stable: a line's ID never changes once assigned, even as the viewport moves.
- **I2** — **History lines are immutable once scrolled off.** This is what makes client-side caching
  sound and scrollback requests idempotent.
- **I3** — `oldest_id` advances monotonically as the daemon trims to the configured limit, and is
  advertised on **every** frame so the client can evict and clamp without asking.
- **I4** — A scrollback request below `oldest_id` is **advisory, not an error** — it returns the
  available intersection plus the watermark. No error path, no retry storm.

---

## Client-side entities

### GridCache

A projection, never a source of truth. Discarded on disconnect.

```text
GridCache
├── lines        : BTreeMap<LineId, WireLine>
├── viewport_top : LineId
├── cursor       : WireCursor
├── generation   : u64
└── oldest_id    : LineId
```

Per wezterm's degrade-to-stale model, a cache entry is a small state machine —
`Line | Fetching | LineAndFetching(old, at) | Stale(old)` — so an in-flight fetch keeps old content
renderable rather than blanking.

### Selection — new client-side model

**This is work the original brief did not anticipate.** Today selection is a mutation of the shared
`Term` (`src/ui/terminal.rs:198-236`) and `selectable_content()` reads it back from alacritty. The
spec assigns selection to the client (FR-010, FR-018), so:

```text
Selection
├── anchor : (LineId, u16)
├── head   : (LineId, u16)
└── mode   : Simple | Semantic | Lines
```

**Invariants**

- **X1** — Anchored to `LineId`, not viewport rows, so it survives scrolling and new output.
- **X2** — Text extraction is client-side over the `GridCache`; the daemon's `Term.selection` is
  never used.
- **X3** — A selection extending below `oldest_id` is clamped, not invalidated.

### ConnectionState

```text
ConnectionState = Connecting
                | Attached { project }
                | Refused { reason: VersionMismatch | ProjectBusy }
                | Disconnected { since, last_error }
```

**Invariants**

- **N1** — While `Disconnected`, displayed content is explicitly marked possibly-stale, actions
  requiring the daemon are disabled, and reconnection is offered (FR-027).
- **N2** — A dead or half-open connection reaches `Disconnected` within a bounded time (SC-011: 10 s)
  via the ~30 s keepalive plus immediate EOF detection. **Stale content is never presented as live**
  (FR-026).
- **N3** — On reconnect the client **re-reads current authoritative state**; it never replays missed
  events (FR-028).

---

## Cross-cutting invariants

- **G1 — Single writer.** No process other than the daemon writes `projects.json` or `settings.json`,
  and no process other than the daemon invokes git (FR-008, FR-009). This removes the current
  silent-clobber hazard, since `src/store.rs` has no file locking.
- **G2 — Input is a lossless append-only log; screen state is lossy and convergent.** Keystrokes are
  never coalesced, dropped or reordered — including across a detach/reattach boundary. Grid frames
  are freely collapsed. Same transport, opposite semantics; getting this backwards loses user input.
- **G3 — Every mutation resolves to exactly one of success, specific failure, or explicit unknown**
  (FR-031). "Unknown" is a real state that must be resolved by reading authoritative state on
  reconnect, not a synonym for failure.
- **G4 — The daemon never exits while any session is alive**, regardless of connected clients
  (FR-002). Exit requires live sessions **and** connected clients both zero.
- **G5 — The client holds no durable state beyond per-window presentation.** Displacing or crashing
  a client destroys nothing, which is what makes takeover safe (Settled Decision 6).

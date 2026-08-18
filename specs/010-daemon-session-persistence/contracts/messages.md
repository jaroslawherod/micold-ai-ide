# Contract: Client ↔ Daemon Message Surface

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

The full message surface. Framing, handshake and error semantics are in
[protocol.md](./protocol.md). Types are Rust-shaped for clarity; both binaries compile against one
definition in the core lib's `protocol` module.

Two categories, distinguished by the envelope `kind` byte:

- **Commands** — fire-and-forget. The client never blocks on them (FR-020).
- **Requests** — correlated by `req: u64`, resolve to exactly one outcome (FR-031).

---

## Client → Daemon

### Connection

```rust
Hello { protocol_version: u32, schema_hash: [u8; 32], client_build: String }  // both must match (Decision 4)
Attach { project: PathBuf, force: bool }      // force = confirmed takeover (FR-023)
Detach { project: PathBuf }
Goodbye                                        // clean disconnect; does NOT stop sessions
```

`Attach { force: false }` on an occupied project is **refused**, not queued. `force: true` is only
ever sent after explicit user confirmation.

### Session commands (fire-and-forget)

```rust
SessionInput   { session: SessionId, serial: u64, bytes: Vec<u8> }
SessionResize  { session: SessionId, cols: u16, rows: u16 }
SessionStart   { session: SessionId }          // Idle | Failed | InterruptedResumable -> Starting
SessionStop    { session: SessionId }          // graceful; -> Idle, no restart
SessionKill    { session: SessionId }          // escalation ladder, force
SessionInterrupt { session: SessionId }        // writes 0x03 to the PTY master
```

`SessionInput.serial` is monotonic per session and **exists to detect loss, not to enable
coalescing**. Input is an append-only log: never coalesced, dropped or reordered, including across
detach/reattach (G2). `SessionStart` on an `InterruptedResumable` session is the single explicit
user action that resumes the prior conversation (FR-006a).

### View commands

```rust
SetViewedSession { project: PathBuf, session: Option<SessionId> }
ScrollbackRequest { session: SessionId, req: u64, ranges: Vec<Range<LineId>> }
```

`SetViewedSession` tells the daemon which session gets full grid streaming; all others report status,
title and activity only (FR-016). `None` means no session is being viewed.

### Mutating requests (correlated, resolve to one outcome)

```rust
ProjectAdd    { req: u64, path: PathBuf }
ProjectRemove { req: u64, path: PathBuf }
ProjectRename { req: u64, path: PathBuf, display_name: String }

WorktreeCreate { req: u64, project: PathBuf, branch: String, dir_name: String }
WorktreeDelete { req: u64, project: PathBuf, dir_name: String,
                 stop_sessions: bool }          // must be true if sessions are live (W2)
WorktreeRename { req: u64, project: PathBuf, dir_name: String, display_name: String }

SessionCreate { req: u64, project: PathBuf, worktree_dir: String }
SessionDelete { req: u64, session: SessionId }

SettingsSet   { req: u64, scrollback_lines: Option<usize> }   // FR-012a
```

**`WorktreeDelete` with live sessions and `stop_sessions: false` MUST fail** with a specific error
rather than silently orphaning processes (W2). The client turns that into a confirmation prompt and
retries with `true`.

There is **no** `GitRun` escape hatch. Every git invocation is a named RPC; the client never shells
out (FR-009).

### Diagnostics

```rust
LogLocationRequest { req: u64 }
RecentErrorsRequest { req: u64, limit: u32 }
SetLogLevel { req: u64, directives: String }    // runtime EnvFilter reload (FR-043)
```

### Keepalive

```rust
Ping { nonce: u64 }
```

---

## Daemon → Client

### Connection

```rust
Welcome { daemon_build: String, catalog: CatalogSnapshot, settings: DaemonSettings }
Refused { reason: RefusalReason }
Attached { project: PathBuf, sessions: Vec<SessionSummary> }
Displaced { project: PathBuf, by: String }     // this client lost the project to a takeover
Pong { nonce: u64 }

enum RefusalReason {
    VersionMismatch { client: u32, daemon: u32,
                      client_hash: [u8; 32], daemon_hash: [u8; 32],   // Decision 4
                      daemon_build: String },
    ProjectBusy     { project: PathBuf, holder: String, since_secs: u64 },
    NotPermitted    { detail: String },
}
```

`Displaced` **MUST NOT** terminate the client. It stops rendering and sending input for that project,
shows a disconnected state with a reconnect affordance, and keeps running (FR-024).

### State projection (pushed, unsolicited)

```rust
CatalogChanged { catalog: CatalogSnapshot }
SessionChanged { session: SessionId, summary: SessionSummary }
SettingsChanged { settings: DaemonSettings }

struct SessionSummary {
    id: SessionId,
    worktree_dir: String,
    title: SessionLabel,
    lifecycle: SessionLifecycle,     // incl. InterruptedResumable, Failed { reason, attempts }
    activity: ActivitySignal,        // Unknown | Working | AwaitingInput | Ended
    input_serial: u64,               // FR-028a, BUG-006 — the serial the service expects next
    live_shells: Vec<ShellInstanceId>, // `012` FR-008, BUG-003 — which instances have a process
}
```

The last three are **runtime-only and overlaid** from the live registry
(`DaemonState::overlay_live_summaries`), not projected from the durable catalog, which cannot see it:
`activity`, `input_serial`, `live_shells`, and the live OSC-0 title. A session the service is not
hosting reports each one's default, and those defaults are correct answers rather than placeholders —
no receiver means no input accepted, and no live entry means no live shell instances.

`live_shells` is asymmetric on purpose. An id present means that instance's process exists; an id
absent means the service is not hosting it, which covers a spawn still in flight as much as a shell
that exited. A client MUST NOT read a first absence as death. It is still the only way to observe an
exit at all — no frames is a quiet shell as much as a dead one.

Every connected client affected by a mutation receives the update without further user action
(FR-011). `SessionSummary` is sent for **every** session, viewed or not, so the client can render the
activity indicator in the session list (FR-016d).

### Grid stream (envelope `kind = 1`)

```rust
struct GridFrame {
    session: SessionId,
    seq: u64,
    generation: u64,
    full: bool,                      // true = snapshot, false = delta
    viewport_top: LineId,
    oldest_available: LineId,        // trim watermark, on EVERY frame
    cols: u16, rows: u16,
    cursor: WireCursor,
    styles: Vec<WireStyle>,          // per-frame interned palette
    hyperlinks: Vec<String>,         // per-frame interned URIs
    lines: Vec<WireLine>,            // all lines if full, changed only if delta
    mode: u32,                       // TermMode::bits()
    input_serial: Option<u64>,       // echo of last applied input, for local-echo correlation
}

struct WireLine {
    id: LineId,
    text: String,                    // one char per CELL; wide-char spacers keep a sentinel
    runs: Vec<StyleRun>,             // RLE; sum(len) == cell count
    extras: Vec<CellExtras>,         // usually empty — zerowidth/hyperlink
    wrapped: bool,                   // WRAPLINE on the last cell
}

struct StyleRun { len: u16, style: u16 }        // style = index into frame palette

struct WireStyle {
    fg: WireColor, bg: WireColor,
    flags: u16,                      // alacritty Flags::bits() verbatim, no translation
    underline_color: Option<WireColor>,
}

enum WireColor { Named(u8), Indexed(u8), Rgb(u8, u8, u8) }

struct CellExtras { col: u16, zerowidth: Vec<char>, hyperlink: Option<u16> }

struct WireCursor {
    line: LineId, col: u16,
    shape: WireCursorShape,          // own enum — vte's CursorShape does NOT derive Serialize
    visible: bool, blinking: bool,
}

enum WireCursorShape { Block, Underline, Beam, HollowBlock, Hidden }
```

**Representation rules that carry the 15× size win** — these are not optional polish:

1. Text is one `String`, style is RLE runs. Per-cell structs are ~15× larger.
2. Styles are interned per frame (`u16` index). Typically fewer than 8 distinct styles per frame.
3. Rare per-cell data (`zerowidth`, `underline_color`, `hyperlink`) is hoisted into sparse side
   tables with `skip_serializing_if`, mirroring alacritty's own `Cell::extra: Option<Arc<_>>`.
4. `Flags::bits()` and `TermMode::bits()` ship as raw integers.
5. **Do not intern across frames** — it couples both ends' state and breaks resnapshot-on-attach.

**Wide characters occupy two cells.** The char lives in the `WIDE_CHAR` cell; the next is a
`WIDE_CHAR_SPACER` whose content is meaningless. **The spacer convention MUST be preserved on the
wire** — stripping it loses the column alignment the daemon already computed. `zerowidth` carries
combining marks and ZWJ sequences; dropping it silently mangles emoji.

```rust
ScrollbackResponse {
    session: SessionId, req: u64,
    oldest_available: LineId, newest: LineId,
    lines: Vec<WireLine>,            // may be fewer than requested — advisory, not an error
    more: bool,                      // chunked; client requests the next chunk
}
```

### Terminal-originated notifications

```rust
SessionTitleChanged { session: SessionId, title: Option<String> }   // OSC title
SessionBell         { session: SessionId }
SessionExited       { session: SessionId, status: ExitStatus, restarting: bool }
ClipboardStore      { session: SessionId, content: String }
```

Only these cross to the client. `PtyWrite`, `ColorRequest` and `TextAreaSizeRequest` are answered by
the daemon writing back to the PTY (protocol.md §8).

### Operation results

```rust
OperationOk    { req: u64, result: OperationResult }
OperationError { req: u64, kind: ErrorKind, message: String, detail: Option<String> }

enum ErrorKind {
    NotFound, AlreadyExists, InvalidInput, Busy,
    GitFailed,                        // `detail` carries git's stderr VERBATIM (FR-034)
    IoFailed, Refused, Internal,
}
```

The client resolves a `req` with no response before disconnect into an **explicit unknown** state,
never success and never failure (FR-031/FR-035), and settles it by reading authoritative state on
reconnect.

### Diagnostics

```rust
LogLocation { req: u64, path: Option<PathBuf>, sink: LogSink }
RecentErrors { req: u64, entries: Vec<LogEntry> }

enum LogSink { Stderr, Journald, File }
```

Log entries **MUST NOT** contain terminal content or user input, which may hold source code and
secrets (FR-047). They reference sessions by identity and state only.

---

## Ordering guarantees

1. **One stream, total order.** Control and grid messages share the framed connection so their
   relative order is well-defined — a `SessionResize` is ordered against the frames around it. They
   MUST NOT be split across two connections.
2. **Input is ordered and lossless** per session (G2).
3. **Grid frames are lossy and convergent.** Any prefix may be collapsed; only the latest matters.
   A client that misses frames converges on the next one it receives.
4. **`CatalogChanged` is idempotent** — it carries a full snapshot, not a delta, so a missed update
   is self-healing.

---

## Message surface ↔ requirement map

| Requirement | Messages |
|---|---|
| FR-011 catalog propagation | `CatalogChanged`, `SessionChanged`, `SettingsChanged` |
| FR-012a service-owned scrollback | `SettingsSet`, `SettingsChanged` |
| FR-014 snapshot on attach | `GridFrame { full: true }` |
| FR-016 viewed vs background | `SetViewedSession`, `SessionSummary.activity` |
| FR-016d list-level indicator | `SessionChanged` for every session |
| FR-017 scrollback by range | `ScrollbackRequest` / `ScrollbackResponse` |
| FR-019 client-side keymap | `SessionInput { bytes }` |
| FR-021/022 version handshake | `Hello`, `Refused::VersionMismatch` |
| FR-023/024 exclusivity, takeover | `Attach { force }`, `Refused::ProjectBusy`, `Displaced` |
| FR-031/034 error semantics | `OperationError`, `ErrorKind::GitFailed.detail` |
| FR-043/046 diagnostics | `SetLogLevel`, `LogLocation`, `RecentErrors` |

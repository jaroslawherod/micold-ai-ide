# Contract: Wire Protocol

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

Transport, framing, handshake and error semantics for the client↔daemon connection. The message
surface is in [messages.md](./messages.md); the hook receiver is a separate listener documented in
[hooks.md](./hooks.md).

---

## 1. Transport

`interprocess` 2.4.2 local sockets with the `tokio` feature, addressed with **`GenericFilePath`**
and explicit per-OS paths.

> **`GenericNamespaced` MUST NOT be used.** On Linux it maps to the abstract namespace, where per
> `unix(7)` "socket permissions have no meaning" — no access control at all. It would ship three
> different security postures across three targets.

### Endpoint location

| OS | Path | Protection |
|---|---|---|
| Linux | `$XDG_RUNTIME_DIR/micold/daemon.sock` | dir 0700 (XDG-mandated), **sticky bit set** so periodic cleanup does not reap it |
| Linux (`$XDG_RUNTIME_DIR` unset) | `/tmp/micold-<uid>/daemon.sock` | dir created 0700 **and verified** — see below |
| macOS | `$HOME/.micold/run/d.sock` | dir 0700. `mode()` is unsupported on Darwin, so directory mode is the boundary |
| Windows | `\\.\pipe\Micold.Daemon.<user-SID>` | explicit protected DACL — see below |

**macOS**: `sun_path` is 104 bytes (103 usable). The chosen path is 55/103 worst case. The
implementation **MUST assert the length at bind time** — an overrun surfaces as an opaque `EINVAL`,
not `ENAMETOOLONG`. Fallback is `_CS_DARWIN_USER_CACHE_DIR` (63 chars, constant).

**Linux `/tmp` fallback verification is mandatory**, because `/tmp` is world-writable and the path
is predictable:

1. `symlink_metadata` — **not** `metadata`, to defeat a planted symlink.
2. `uid() == geteuid()`.
3. mode is exactly `0o700`.

Any failure **MUST bail loudly**. Wrong ownership means an active attack, not a mess to tidy.

**Windows DACL** is required even though the SID appears in the pipe name — the name buys collision
avoidance, not security, and the default descriptor grants read access to Everyone and the anonymous
account:

```text
D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x12019b;;;<sid>)
```

`D:P` (protected) is what strips the inherited Everyone grant. The mask avoids a documented trap:
`FILE_APPEND_DATA` shares a bit with `FILE_CREATE_PIPE_INSTANCE`, so a plain `FW` grant would let
clients create rival instances of our own pipe. ⚠️ **`0x12019b` is derived, not quoted — verify in
an integration test.**

`PIPE_REJECT_REMOTE_CLIENTS` and `FILE_FLAG_FIRST_PIPE_INSTANCE` are applied automatically by
`interprocess`; no action required.

---

## 2. Startup: connect, else become the daemon

Two clients starting simultaneously **MUST** converge on exactly one daemon.

### Unix

Socket existence proves nothing — the kernel never removes a bound socket. `connect()` is the
liveness discriminator: `ECONNREFUSED` = stale, `ENOENT` = never existed, success = live.

```text
1. connect()  -> Ok            => act as client. Fast path, no lock.
2. try_lock exclusive on <runtime>/daemon.lock
     WouldBlock                => another starter is mid-recovery; back off, goto 1.
                                  Touch nothing.
3. RE-CHECK connect()          <-- MANDATORY
     Ok                        => the other starter won; drop lock, act as client.
4. unlink(sock) if S_ISSOCK (ignore ENOENT); bind; listen
5. HOLD THE LOCK FOR PROCESS LIFETIME
```

**Step 3 is not optional.** Without it the lock *loser* acquires after the winner has bound, and
unlinks a live socket — unlink operates on the name, not the inode. Result: two daemons, one
permanently unreachable.

**Step 5**: the lock releases when the fd closes, which includes SIGKILL and OOM where no cleanup
code runs. That makes it an unforgeable liveness beacon, strictly better than a PID file (no
PID-reuse hazard), probeable non-destructively by others via `try_lock`. Keep the lockfile off NFS.

### Windows

None of the above applies. Named pipes live in NPFS with no filesystem object, so there is no stale
endpoint, and `FILE_FLAG_FIRST_PIPE_INSTANCE` makes create-or-fail a **single atomic kernel call
with no TOCTOU gap**.

| Result | Meaning |
|---|---|
| Created | Become the daemon |
| `ERROR_ACCESS_DENIED` | A daemon exists → become a client |
| `ErrorKind::NotFound` on connect | No daemon → spawn one |
| `ERROR_PIPE_BUSY` | Exists but busy → retry |

**Do not conflate `NotFound` with `PIPE_BUSY`** — that causes duplicate launches. `WaitNamedPipe` is
not a boot-wait primitive; it returns immediately when no instance exists.

### systemd socket activation (Linux, opportunistic)

If `LISTEN_FDS` is present and `LISTEN_PID == getpid()`, adopt fd 3 via `listenfd` and skip the bind
entirely. `set_nonblocking(true)` is **mandatory** — systemd does not guarantee it, and a blocking
`accept()` stalls the runtime. Activation **MUST NOT** be required: a `.desktop`/Flatpak/AppImage
launch never goes through a unit, and the race-free-startup benefit is already provided by the lock.

---

## 3. Framing

`tokio_util::codec::LengthDelimitedCodec`, configured explicitly rather than defaulted:

```rust
LengthDelimitedCodec::builder()
    .length_field_type::<u32>()
    .little_endian()                      // both ends are the same machine
    .max_frame_length(16 * 1024 * 1024)   // NOT the 8 MiB default
    .new_codec()
```

`max_frame_length` **MUST** be set explicitly. A corrupt length must not trigger a huge allocation,
and a large scrollback response must not be silently truncated — hence 16 MiB *and* response
chunking (§6). This is the loud-early-failure requirement of Settled Decision 8.

### Envelope

One framed stream carries both planes. **Do not split the transport** — ordering between control and
grid messages must be well-defined (a `Resize` must be ordered against the frames around it).

```text
| u32 length (LE) | u8 encoding | u8 kind | u16 reserved | payload |
```

| Field | Values |
|---|---|
| `encoding` | `0` = JSON, `1` = postcard, `2` = postcard+lz4 (reserved, unused locally) |
| `kind` | `0` = control/RPC, `1` = grid frame |
| `reserved` | MUST be zero; receiver MUST reject non-zero |

Control messages use JSON (low volume, high debugging value, `#[serde(default)]`-evolvable). Grid
frames use postcard. **`MICOLD_WIRE=json` forces `encoding = 0` for grid frames too** — the same
`GridFrame` type serializes either way, giving a fully human-readable stream for debugging at zero
code cost. This debug switch is the entire justification for the hybrid; if it is not built, use one
format throughout.

> **Never `bincode`.** `bincode` 3.0.0 is a tombstone whose `src/lib.rs` is
> `compile_error!("https://xkcd.com/2347/")`; the repo is archived.

---

## 4. Handshake

Strict exact-match, no negotiation, no compatibility range (FR-021).

```text
client ──► Hello { protocol_version: u32, schema_hash: [u8; 32], client_build: String }
daemon ──► Welcome { daemon_build: String, catalog, settings }
       or  Refused { reason: VersionMismatch { client: u32, daemon: u32,
                                               client_hash, daemon_hash,
                                               daemon_build: String }, .. }
```

Both `protocol_version` **and** `schema_hash` **MUST** match. On either mismatch the daemon **MUST**
refuse the connection and **MUST** name both sides' version and hash plus its own build, so the client
can render an actionable diagnostic and offer the restart action (FR-022). The client's restart action
**MUST** warn that live processes are lost while sessions remain resumable — true because session
UUIDs are owned up front and persisted (S1).

**`PROTOCOL_VERSION`** lives in `micold-core` so both binaries compile against one definition, and
**MUST** be bumped on any wire-visible change. **`SCHEMA_HASH`** is a `const [u8; 32]` produced by
`micold-core`'s `build.rs`, hashing the canonical text of the protocol type definitions
(`messages.rs`, `grid.rs`, `envelope.rs`). It exists to catch the case the version integer cannot: a
message struct edited without a version bump. Because both ends compile the same `micold-core`, two
builds that disagree about the wire necessarily disagree on the hash and the handshake refuses them —
loud early failure over silent drift (Settled Decision 8). This is strictly stricter than a version
check, never a substitute for it; the version is still the human-facing number in diagnostics.

---

## 5. Grid streaming

### Model

The daemon holds a per-client cursor and diffs **last-known → now**. Frames are never queued; only
the *intent* to send one is (a depth-one dirty flag).

```text
PTY output  ──► set dirty = true          (never allocates, never blocks)
tick (~60Hz) ──► if !dirty || in_flight: return
                 lock Term, collect damage, reset_damage, unlock
                 hash candidate lines, diff vs shadow by LineId
                 build frame, encode, write
```

**Guarantees**

- **A slow client converges rather than lags.** If the socket is not writable the framer leaves
  `dirty` set and returns; the next frame it builds reflects the *current* screen, and every
  intermediate state is skipped for free (FR-015).
- **Bounded memory**: one in-flight frame per client, `O(viewport + shadow)` (SC-006).
- **The PTY reader never blocks on a client.** Nothing in the send path may hold the terminal lock —
  build into an owned buffer, drop the guard, *then* encode and write. Use
  `alacritty_terminal::sync::FairMutex`, which exists precisely to stop the framer starving the
  reader.

### Damage is a filter, not truth

`Term::damage()` **MUST NOT** be treated as the change set:

- It returns `TermDamage::Full` on **every scroll** (measured: 477 of 500 frames under a `cat`
  workload), and on any write while `display_offset != 0`.
- It is **never empty** — it unconditionally damages the cursor cell, so a no-op frame yields
  `Partial([(0,0,0)])`. "Nothing changed" must be determined by content hashing, not by alacritty.
- It takes `&mut self` and the returned iterator holds that borrow, so it **must** be collected
  before the grid can be read.

Use it only to avoid hashing untouched rows — a genuinely good fast path for typing (1.01 lines per
frame, zero full frames).

### Diff keying

Diffs are keyed by **stable absolute `LineId`**, never viewport index. Measured on a scrolling
workload at 80×24: 22.99 lines/frame by damage, 21.99 by viewport index, **2.00 by stable ID**.
Viewport indexing does not help because scrolling by one shifts every row.

### Resnapshot triggers

Send a full snapshot on: attach or requested resync; **generation change** (resize, alt-screen
enter/exit, reset — line identities change and diffing is meaningless); changed lines ≥ ~60% of the
viewport; or when the client's position predates retained shadow state. Because the framer always
diffs from last-known to now, "too far behind" is rare by construction — resnapshot is an
attach/resize path, not a congestion path.

### Liveness

EOF/`ECONNRESET` on a local socket is immediate and definitive, so the common case needs no polling.
The residual case is a *silently* half-open peer (suspend/resume, container pause) that never sends
FIN. To meet SC-011, the client **MUST** send a `Ping` every **3 s** and declare the connection dead
if no `Pong` (nor any other frame) arrives for **9 s** (three missed intervals). Worst-case detection
is thus **< 10 s** (SC-011). On that deadline the client **MUST** reach `Disconnected` and **MUST
NOT** present stale content as live (FR-026).

⚠️ A ~30 s cadence was the earlier sketch; it cannot satisfy SC-011's 10 s bound and was corrected
here (analysis I1). The 3 s/9 s figures are the contract — the interval and the deadline must stay
coupled so their sum stays under the SC-011 bound.

#### What the deadline assumes of the daemon (BUG-009, T124, FR-026a)

The rule above infers death from silence. That inference is only sound while **the daemon is silent
only when it is dead** — which is a constraint on the daemon, not on the client, and one that every
newly added operation can violate without touching a line of this section.

The obligation, stated as a property of the connection rather than of any operation:

> A connection **MUST** keep serving its own protocol — at minimum answering `Ping` — for the entire
> duration of any operation the client asked it for, however long that operation runs. Progress
> reporting **MAY** reset the deadline as a side effect but **MUST NOT** be what liveness rests on:
> an operation that legitimately emits nothing for longer than 9 s must still keep its link alive.

Concretely, in `route()` (`crates/micold-daemon/src/server.rs`) — the one sequential loop per
connection, and the only place that client's `Ping` is answered:

- **`spawn_blocking(..).await` in the loop does not satisfy this.** It frees the runtime; it does not
  free the loop. That is exactly how BUG-009 shipped.
- An operation that can outlast the deadline **MUST** be `tokio::spawn`ed, replying through the
  client's ordered frame channel (`state.frame_sender(id)` / `state.send(id, ..)`), which a departed
  client drops harmlessly.
- Spawning removes the incidental serialization the inline `.await` provided. Where two concurrent
  runs could corrupt each other, take an explicit gate — `DaemonState::worktree_gate(project)` for
  worktree mutations — inside the spawned task, never on the loop.

Audit of the arms, as of 2026-08-06:

| Arm | Can exceed 9 s? | Status |
|---|---|---|
| `WorktreeCreate` | **Yes** — submodule fetch is network-bound and routinely minutes | Spawned + per-project gate (T120) |
| `WorktreeDelete` | **Yes** — `remove_dir_all` over dependency trees/build output; worse on a network FS | Spawned + per-project gate (T124) |
| `SessionCreate` / `SessionStart` | **Yes** — `start_session` resolves the user's environment-include script, whose timeout is user-configurable up to **60 s** (`MAX_ENV_INCLUDE_TIMEOUT_SECS`), plus a PTY fork | Spawned + per-session gate, with held input and a view hand-back (T125) |
| `BranchPreflight`, `BranchList` | Unlikely — local `git for-each-ref` / `worktree list --porcelain`, no remote | Left inline; revisit if a pathological ref count is ever reported |
| `SettingsSet`, `WorktreeRename`, `Project*`, `SessionDelete` | No — catalog + a small atomic file write | Left inline |
| `Ping`, `Attach`, `Detach`, `SetViewedSession`, `SessionInput`, `Scrollback*` | No — lock-only or bounded | Left inline |

#### Spawning a session start owes two more things (T125)

The worktree arms only had to preserve their own reply. A session start is different: other messages
are *about* the thing it is creating, and the inline version made them safe by accident.

- **Input must be held, not dropped.** With the start spawned, `SessionInput` can arrive before the
  session exists — where it would have hit the "input for a session the daemon is not hosting" path
  and been discarded. §7 forbids that. `DaemonState::begin_start` marks the session, `session_input`
  holds arriving batches in order, and `finish_start` replays them through the ordinary path the
  moment the session is live, so classification happens exactly as it would have had the start been
  instant. `finish_start` runs on **every** outcome, including a failed start — held keystrokes must
  never be stranded. It drains repeatedly and only clears the marker on an empty buffer observed
  *under the lock*, so no input can take the direct path and overtake a held one.
- **A view asked for too early must still be built.** The client sends `SessionStart` and
  `SetViewedSession` back to back; the second used to find the session live. It no longer does, and
  a view request that quietly resolves to nothing is a permanently blank terminal. The connection
  loop records what the client asked to view and builds the stream when its own spawned start
  reports back over an internal channel (`Internal::SessionStarted`). The loop owns the view stream,
  so work finishing elsewhere reports to it rather than reaching in — the general shape for any
  future spawned work that the loop's own state depends on.

Both are covered by `crates/micold-daemon/tests/busy_session_start.rs`, whose slow operation is a
real environment-include script that sleeps.

---

## 6. Scrollback

```text
client ──► ScrollbackRequest { session, req, ranges: [Range<LineId>] }
daemon ──► ScrollbackResponse { session, req, oldest_available, newest, lines }
```

- Requests are **advisory, never errors**. A range wholly below `oldest_available` returns empty plus
  the watermark; the client clamps. No error path, no retry storm.
- **Every** `GridFrame` also carries `oldest_available`, so the client learns the trim watermark
  continuously and can evict its cache and size its scrollbar without asking.
- History lines are **immutable once scrolled off**, so `LineId → content` is a permanent mapping the
  client may cache indefinitely. This is only sound because IDs are absolute.
- Responses are **chunked** (~500 lines or ~256 KB) so a scrollback fetch cannot monopolise the
  socket or approach the frame cap.
- The daemon **SHOULD** speculatively include the cursor's line and a screenful either side of the
  requested range, which removes a round trip on scroll for a few hundred bytes.

---

## 7. Input

**Input is a lossless, append-only, ordered log. It MUST NEVER be coalesced, dropped or reordered** —
including across a detach/reattach boundary. Screen state is lossy and convergent; input is not.
Same transport, opposite semantics, and getting it backwards loses user keystrokes.

The client encodes keys to VT bytes (FR-019); the daemon stays keyboard-agnostic and never
interprets keymaps. Interrupt is `0x03` written to the PTY master, **never** a real signal —
`claude` is an Ink TUI in raw mode with `ISIG` disabled, so a real `SIGINT` would bypass its
handler. `GenerateConsoleCtrlEvent` **MUST NOT** be used on Windows: it requires a shared console
the daemon does not have, and silently delivers nothing.

---

## 8. Terminal-internal replies stay daemon-side

`alacritty_terminal` `EventListener` events split in two directions, and confusing them adds a round
trip to terminal-internal handshakes:

| Event | Destination |
|---|---|
| `Title`, `ResetTitle`, `Bell`, `ChildExit`, `ClipboardStore` | Forward to client as control messages |
| `PtyWrite`, `ColorRequest`, `TextAreaSizeRequest` | **Daemon answers by writing to the PTY itself** |

---

## 9. Error semantics

Every mutating RPC resolves to exactly one of three outcomes (FR-031):

```text
Ok(result)
Err(OperationError { kind, message, detail })   -- specific and actionable
<connection lost>                               -- client enters Unknown for that operation
```

- **Underlying diagnostics survive intact.** Git failures carry the actual stderr; the daemon
  **MUST NOT** substitute a generic message (FR-034).
- **No partial artifacts.** A failed operation leaves no catalog entry, no directory, no git state
  (FR-032). Catalog mutation is the last step, after side effects succeed.
- **Unknown is a real state**, not a synonym for failure. It is resolved by reading authoritative
  state on reconnect, and the resolved outcome is shown to the user (FR-035).
- While a mutation is pending the client shows it as pending and prevents duplicate submission
  (FR-033).

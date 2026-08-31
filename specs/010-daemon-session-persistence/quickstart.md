# Quickstart: Validating Daemon-Backed Session Persistence

**Feature**: `specs/010-daemon-session-persistence` | **Date**: 2026-07-20

Runnable scenarios that prove the feature works end to end. Each maps to spec success criteria and
states its expected outcome. Implementation detail belongs in `tasks.md`, not here.

---

## Prerequisites

```bash
mise run build                      # or: cargo build --features gui,daemon
cargo test --no-default-features    # baseline: 259 tests must pass (FR-040, FR-041)
```

- `claude` CLI on `PATH` (v2.1.208+ for stream-json; hooks need a version that supports
  `type: "http"`).
- A git repository to add as a project.
- Linux: `systemd --user` for the socket-activation scenarios. macOS/Windows: portable path only.

**Useful during validation**

```bash
export MICOLD_LOG=debug        # verbosity (FR-043)
export MICOLD_WIRE=json        # human-readable grid frames — the reason for the hybrid format
```

---

## S1 — Sessions outlive the UI *(SC-001, SC-002; User Story 1)*

The core promise. Run this first; nothing else matters if it fails.

```bash
# 1. Launch, add a project, start a session, give it long-running work.
mise run run
#    In the session:  ask the agent to run something that prints continuously for ~10 min.

# 2. Close the window. Confirm the daemon and the child survive.
pgrep -af micold-daemon
pgrep -af claude

# 3. Wait ≥10 minutes.

# 4. Relaunch and reattach.
mise run run
```

**Expected**: the daemon and `claude` processes are still running in step 2. In step 4 the session is
still `Running`, its screen shows current output, and scrollback covers the **entire** closed
interval with **zero gaps and zero duplication**, bounded only by the configured scrollback limit.

**Also verify** — crash and rebuild survival:

```bash
kill -9 $(pgrep -f 'micold-ai-ide$')   # crash the CLIENT only, never the daemon
mise run build && mise run run          # rebuild without a protocol change
```

Both must leave sessions running. ⚠️ Never `pkill -f micold` — that would take the daemon with it.

---

## S2 — Attach, drive, detach *(SC-003, SC-004, SC-005; User Story 2)*

```bash
pkill -f micold-daemon      # ensure nothing is listening (test daemon only)
time mise run run           # cold start
```

**Expected**: usable and attached in **under 3 s** with no install step or manual command. Then, with
a session blocked on a prompt: type a response, and it reaches the process with no perceptible lag.
Scrolling, selecting and resizing respond immediately — none may block on a round trip. Switching
session or project presents the correct screen within **200 ms**.

---

## S3 — Activity signal *(SC-015, SC-016; User Story 2 scenario 5)*

The mechanism most likely to be wrong; test it deliberately.

1. Start three sessions. Leave one at a prompt, give one a **multi-minute** build or test run, and
   stop the third.
2. Look only at the session list — do not open any session.

**Expected**: the waiting session reads *awaiting input*, the busy one reads *working* for the
**entire** run with **zero** spurious flips to awaiting-input, and the stopped one reads *ended*.
A blocked session reaches awaiting-input within 5 s.

3. Now launch a session with hooks disabled.

**Expected**: activity reads **unknown**, never *awaiting input* (H1). This is the failure mode that
matters — a wrong "ready for you" is worse than no answer.

---

## S4 — Unattended supervision *(SC-012; User Story 4)*

```bash
# With NO client running:
pkill -f 'claude.*--session-id <uuid>'     # kill one session's process
sleep 5
pgrep -af claude                            # daemon should have restarted it
```

**Expected**: restarted with the same retry policy as when attached. Then force repeated failures
past the limit:

**Expected**: after `MAX_RESTART_ATTEMPTS` the session settles in `Failed`, and on the next attach it
shows the failed state **with the reason and attempt count** — proving give-up state persisted
across a period with no observer.

**Note**: `MAX_RESTART_ATTEMPTS = 3` yields **two** actual restarts (L1). ⚠️ The counter has no time
window, so a session crashing once an hour never trips the guard (L5) — verify current behaviour and
record whether it is being fixed.

---

## S5 — Interrupted-resumable after a daemon restart *(FR-006a/b; User Story 6)*

```bash
# With sessions running:
pkill -f micold-daemon
mise run run
```

**Expected**: previously running sessions appear in a **distinct interrupted-resumable state** — not
`Running`, not indistinguishable from a deliberate stop. **The service relaunches nothing**: every
session it recovered is presented, none is respawned.

The client then restores the session you were last on in the project it reopens, and restoring a
session resumes it (`025-last-session-memory` FR-004a) — so **that one** leaves the interrupted-
resumable state because you opened the project, exactly as clicking it would have. Every other session
— its neighbours in the same project, and every session in every other project — MUST still be
interrupted-resumable with no process behind it, and MUST take one explicit action to resume,
continuing the prior conversation.

So this is a check on the **count**, not on silence: after the relaunch there is at most **one** new
agent process and it belongs to the session on screen. Count them rather than judging by the pane in
front of you — `pgrep -fa 'claude|copilot'` before the relaunch and after — since the failure worth
catching is a restart that quietly wakes the sessions nobody opened.

*(Amended 2026-08-27 — BUG-016. Until then this row asked for no agent process at all and called that
"the safety property"; the client had resumed the displayed session since `025`'s BUG-002, so the row
described something the build had stopped doing. What is true, and is what `025` says it traded for,
is the bound on scope above.)*

---

## S6 — Exclusivity and takeover *(SC-010; User Story 5)*

```bash
mise run run          # window A, attach to project P
mise run run          # window B, attach to project P
```

**Expected**: B is **refused** with a message naming the conflict and offering takeover. On
confirming takeover: B attaches; A stops rendering P, shows a disconnected state with a reconnect
affordance, sends **zero** further input, and **does not exit**. A's reconnect works once P is free.

Also: with A on project P and B on project Q, neither interferes with the other.

```bash
kill -9 <window A pid>       # holder crashes without releasing
```

**Expected**: P becomes attachable again **without restarting the daemon**.

---

## S7 — Version mismatch *(SC-009; User Story 6)*

```bash
# Bump PROTOCOL_VERSION in the core lib, rebuild the CLIENT only, leave the old daemon running.
mise run run
```

**Expected**: connection refused with a message naming **both** versions and the daemon build. The
offered "restart daemon" action stops the old daemon, starts a matching one, and attaches — with no
command typed by the user, and with a warning that live processes are lost while sessions remain
resumable. Afterwards, previously live sessions appear as interrupted-resumable (S5).

---

## S8 — Mutation error semantics *(SC-007, SC-008; User Story 3)*

Force each failure and check the outcome is exactly one of success / specific failure / explicit
unknown:

| Trigger | Expected |
|---|---|
| Create a worktree on an existing branch | Specific git error, **git's own stderr preserved**; no catalog entry; no leftover directory |
| Create a worktree at a colliding path | Same |
| Create a worktree in a read-only parent | Same |
| Delete a worktree containing a live session | **Refused** or explicit confirm-and-stop-first; never a silently orphaned process |
| `pkill -f micold-daemon` mid-worktree-creation | Client shows **outcome unknown**, then the actual state after reconnect — never a stale list or a silent success |

While any mutation is pending the control must show pending state and reject duplicate submission.

---

## S9 — Slow-client convergence and bounded memory *(SC-006)*

```bash
# In a session, generate sustained high-volume output for ~10 minutes:
#   yes "$(head -c 200 /dev/zero | tr '\0' 'x')" | head -n 5000000
```

While it runs, watch RSS of both processes:

```bash
watch -n5 'ps -o rss=,comm= -p $(pgrep -d, -f "micold")'
```

**Expected**: neither process grows without bound; the session's process is never blocked by a slow
client; and when output stops, the displayed screen **matches the true screen exactly**. Convergence,
not catch-up: the client must not be replaying a backlog.

Suspend the client briefly (`SIGSTOP`, then `SIGCONT`) — on resume it must jump to current state, not
crawl through intermediate frames.

---

## S10 — Scrollback across a detached interval *(FR-017, FR-018, I2/I4)*

After S1's 10-minute detached run, scroll back through the closed interval.

**Expected**: content is continuous; scrolling stays responsive (history is fetched by range, not
held whole); the scrollbar is sized from the advertised watermark; and scrolling past the retained
limit **clamps** rather than erroring. With a selection active, new output must not move or corrupt
it — selection is anchored to line IDs, not viewport rows.

---

## S11 — Startup race and stale endpoint *(FR-004; protocol.md §2)*

```bash
pkill -f micold-daemon
mise run run & mise run run & wait          # two clients start simultaneously
pgrep -c -f micold-daemon                   # MUST be exactly 1
```

Stale endpoint (Unix):

```bash
pkill -9 -f micold-daemon                   # leaves the socket file behind
ls -l "$XDG_RUNTIME_DIR/micold/daemon.sock" # still present
mise run run                                # must reclaim it and start cleanly
```

Hostile directory (Linux `/tmp` fallback) — with `XDG_RUNTIME_DIR` unset and a wrong-owner or
wrong-mode `/tmp/micold-<uid>`, startup **MUST bail loudly**, not bind anyway — and **MUST leave the
directory as it found it**, so that what it refused is still there for a human to look at. Silently
repairing the mode and continuing is the failure this block exists to catch (BUG-019).

---

## S12 — Daemon lifetime *(FR-002, SC-006)*

```bash
# With one session RUNNING and no client attached:
pgrep -af micold-daemon      # must still be running after several minutes

# With ZERO sessions and no client attached:
#   the daemon MAY exit; if it does, the next client start must re-spawn it transparently.
```

The daemon must **never** exit while any session is alive, regardless of connected clients — this
deliberately overrides the usual socket-activation idle-exit pattern.

---

## S13 — Diagnostics *(SC-017, FR-043–047)*

For each failure in S7, S8 and S11, confirm the cause is determinable from logs reachable **through
the UI**, without rebuilding or reading source.

```bash
ls -l ~/.local/share/micold-ai-ide/micold-daemon.log*   # Linux; see research R2.3 for macOS/Windows
```

The daemon reports its own choice on its first line, which is the quicker check when the file is not
where you expect: `micold-daemon starting … sink=File log_path=Some("…")`. A `sink=Journald` there
from an auto-spawned daemon means the log is being discarded — see BUG-015.

**Expected**: total log size is hard-capped (rotation count × size limit) and does not grow without
bound. **No terminal content or user input appears in any log entry** (FR-047) — grep for a string
you typed into a session; it must not be there.

Under systemd, logs go to the journal instead:

```bash
journalctl --user -u micold-daemon -f
```

---

## S14 — Linux logout survival *(User Story 7)*

```bash
loginctl enable-linger "$USER"     # documented, user-run, NEVER automated by install
# start a session, log out fully, log back in
```

**Expected**: the session survived. Without linger, it does not — and the documentation must say so
plainly.

⚠️ **Order matters**: enable linger **then** start the daemon. Linger is not retroactive — a process
already in `session-N.scope` stays there and still dies at logout.

**macOS/Windows**: confirm the documentation states explicitly that surviving logout is unsupported,
rather than leaving the user to discover it.

---

## S15 — Cross-platform parity *(Principle VI)*

S1–S13 must pass on **Linux, macOS and Windows**. Windows carries the most risk, since the job-object
and detached-spawn code is compile-verified but was never executed there:

- Process-tree kill leaves no orphaned grandchildren.
- Interrupt (`0x03`) reaches the agent. ⚠️ Do **not** validate with `cmd.exe` — it appears to work for
  the wrong reason. Use a program that ignores console control events.
- `portable-pty` 0.9's `kill()` returns inverted results on Windows; confirm teardown succeeds
  regardless.
- Teardown must **not** be gated on reader EOF — ConPTY may never deliver it.

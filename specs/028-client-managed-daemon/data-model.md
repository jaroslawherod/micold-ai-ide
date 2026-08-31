# Phase 1 Data Model: Client-Managed Session Service Lifecycle

**Feature**: 028-client-managed-daemon | **Date**: 2026-08-27

Nothing here is persisted. Every entity below is process-lifetime state in the daemon, or a value
computed from it — which is the point: an idle rule that survived a restart would be a rule about a
service that no longer exists. The one durable thing this feature touches is the session lifecycle
it hands off to, and that mechanism already exists (L4 below).

---

## G1 — `Presence`: how many clients are connected, and since when

Replaces the `Lifecycle` counter pair in `crates/micold-daemon/src/lifecycle.rs`, which counted
sessions as well.

| field | type | meaning |
|-------|------|---------|
| `connected` | `usize` | Clients past the handshake and not yet deregistered. |
| `alone_since` | `Option<Uptime>` | When `connected` last fell to zero. `None` whenever `connected > 0`. |

Transitions (the only two, and both already have exactly one call site — R1):

- `client_connected()` — `connected += 1`; `alone_since = None`.
- `client_disconnected()` — `connected -= 1`; if it reaches zero, `alone_since = Some(now)`.

Invariants:

- `alone_since.is_some() ⟺ connected == 0`. Making this a pair of fields updated in one guarded
  method — rather than a count plus a separately-armed timer — is what stops the two disagreeing
  after a reconnect storm.
- A daemon that has never had a client is idle from **startup**, not from `None`: `alone_since` is
  initialised at construction. Otherwise a daemon spawned by a client that died before handshaking
  would live forever.

## G2 — `IdleWindow`: the rule

A pure function over `Presence` and the clock, tested without a runtime:

```
expired(presence, now) = presence.connected == 0
                      && now - presence.alone_since >= IDLE_WINDOW
```

`IDLE_WINDOW` is a constant, 30 minutes (FR-008), one value for both placements and all three
platforms (FR-017). Live sessions are **not** an input — that is the clarified rule (FR-006a), and
it is the whole difference from the `may_exit(live_sessions, connected_clients)` predicate this
replaces.

Evaluated on a 30-second tick, not slept on once (R3): the tick is what makes waking from suspend
prompt, and it bounds the overshoot at 30 s — inside SC-004's 30-to-31-minute band.

## G3 — `Uptime`: the reading the window is measured in

A newtype over nanoseconds from a monotonic, suspend-inclusive platform clock (`micold_core::clock`,
R3). Two operations only — `now()` and `saturating_sub` — because those are all the rule needs, and
a type that cannot be added to a wall-clock time cannot accidentally be compared with one.

- Monotonic: never moves backwards, so a clock correction cannot expire the window early.
- Suspend-inclusive: eight hours asleep counts as eight hours, so a resumed machine stops promptly.
- Not persisted, not sent on the wire, not rendered.

## G4 — `StopReason`: why the service ended

| variant | produced by | seen by the user as |
|---------|-------------|---------------------|
| `Idle` | the rule in G2 | a diagnostics line naming inactivity (FR-024) |
| `Requested` | the existing stop path (`spawn::stop_running_daemon`) | nothing new |
| *(absent)* | a crash, an OOM kill, a `SIGKILL` | no line at all — which is what distinguishes it |

The distinguishing property is structural rather than textual: an idle stop writes its line **before**
teardown begins, so the presence of the line is itself the evidence, and no crash can forge one.

## G5 — Shutdown sequence

Ordered, and the order is the contract (R4, FR-012–FR-014):

1. Record `StopReason::Idle` in the diagnostics.
2. Stop accepting: drop the accept loop, so no client can attach into a dying daemon.
3. Mark every live session `InterruptedResumable` and persist the catalog (L4).
4. Drop the session table — `PtySession::Drop` terminates each process tree.
5. Drop the bound listener — unlinks the socket, releases the `flock`.
6. Return from `run()`; the process exits 0 by unwinding, never via `process::exit`.

Step 5 last is deliberate: the `flock` is the liveness beacon `singleton::acquire` tests, so it must
outlive everything a next start could observe as still-in-use.

## L4 — Session lifecycle at the handoff (existing, unchanged)

`SessionLifecycle::InterruptedResumable` already exists and is already what daemon startup presents
for sessions that were live when the service last stopped —
`state.present_interrupted_resumable_at_startup()`, called before the accept loop
(`crates/micold-daemon/src/server.rs:136`), and the daemon's own comment names it "the ONLY
lifecycle daemon startup may produce". An idle stop therefore introduces **no new session state**:
it produces the same durable situation as any other service restart, and FR-006c's "never
auto-resume" is a property of that existing path.

## Sandbox placement mapping

| host-process concept | sandboxed equivalent |
|----------------------|----------------------|
| `Presence.connected` | identical — the same `register`/`deregister`, over loopback TCP |
| G5 shutdown | identical, and because the daemon is PID 1 the container exits with it |
| "no process remains" | container status `exited`, `RestartCount` unchanged |
| the idle rule applying at all | **conditional** — suppressed while the survive-reboot opt-in is on (R2) |

That last row is the amendment R2 forces; every other row is the same code on both sides, which is
what makes FR-018 cheap to hold everywhere else.

# Phase 0 Research: Client-Managed Session Service Lifecycle

**Feature**: 028-client-managed-daemon | **Date**: 2026-08-27

Everything below was resolved against this repository or measured on this machine. Where a claim is
measured, the command and its output are recorded so the next reader does not have to re-run it —
and where a measurement **falsified** a requirement as written, that is called out as an amendment
the spec needs before `/speckit-tasks`.

---

## R1 — Where the idle rule attaches

**Decision**: Count connections in `DaemonState::register` / `deregister`, drive the rule from a
single new module `micold-daemon/src/idle.rs`, and evaluate it on a periodic tick in the same style
as the restart supervisor.

**Rationale**: The daemon already has exactly one register/deregister pair, used by every transport.
`server::serve_connection` calls `state.register(client_build)` after the handshake and
`state.deregister(id)` when `route` returns (`crates/micold-daemon/src/server.rs:373`, `:397`), and
all three accept loops — interprocess, systemd-unix, loopback-TCP — funnel into that one function.
So a connection count taken there is automatically correct in both placements, which is what FR-018
asks for.

`crates/micold-daemon/src/lifecycle.rs` already holds the *predicate* this feature revises —
`may_exit(live_sessions, connected_clients)` with its "never exit while a session is alive" rule —
and **it has no call sites**: `grep -rn "may_exit" --include=*.rs crates/` returns only the module
and its own unit tests. The daemon has therefore never exited on its own. That makes this feature
net-new behaviour wearing an existing name, and it means the clarified rule (connections only)
can be landed by rewriting a tested pure function rather than by unpicking live behaviour.

**Alternatives considered**: A timer reset from inside the accept loops (three call sites, three
chances to drift, and the TCP path would be the one that silently missed it). Counting attachments
rather than connections (an attachment is per-project and a client may hold none, so a connected
client with nothing open would have looked idle).

---

## R2 — A self-exiting container versus the runtime's restart policy *(measured — forces a spec amendment)*

**Question**: FR-019/FR-020 require the sandbox to be *stopped*, and to stay stopped, when the idle
window expires. The daemon is PID 1 in the container (`packaging/sandbox/Containerfile:64`,
`ENTRYPOINT ["/usr/local/bin/micold-daemon"]`), so an idle exit ends the container. Does the restart
policy the survive-logout opt-in selects (`argv::restart_policy` → `unless-stopped`, else `no`)
bring it straight back?

**Measured on this machine** (Docker 29.5.1), one throwaway container per policy, each exiting after
2 s, inspected 7 s later:

| policy | exit code | final status | RestartCount |
|--------|-----------|--------------|--------------|
| `unless-stopped` | 0 | **running** | 3 |
| `unless-stopped` | 1 | running | 3 |
| `on-failure` | 0 | **exited** | 0 |
| `on-failure` | 1 | running | 3 |
| `no` | 0 | exited | 0 |

And from the Docker documentation on daemon start (i.e. after a host reboot): `on-failure`
"doesn't restart the container if the daemon restarts"; `always` and `unless-stopped` do.

**Consequence**: the two behaviours are mutually exclusive under Docker's own policies.

- With the opt-in **off** (`--restart no`), an idle exit stops the container and it stays stopped.
  FR-019 and FR-020 hold as written, with no new machinery. ✅
- With the opt-in **on** (`--restart unless-stopped`), an idle exit is restarted — measured, three
  times in seven seconds — and the restarted daemon is immediately idle again. The steady state is
  not "stopped": it is a container restarting forever at Docker's capped backoff. ❌
- `on-failure` would ignore a clean exit, but it also forfeits the restart-at-boot the opt-in exists
  to deliver, so it cannot serve as the opt-in's policy.
- Nothing inside the container can mark itself "stopped" the way `docker stop` does, and feature
  027's FR-005 forbids mounting the runtime's control socket into the sandbox — deliberately, since
  that socket is root on the host. Nor is there a host-side process to issue the stop: the client is
  closed, which is the entire premise of the idle window.

**Decision**: **the survive-logout/reboot opt-in and the idle stop are mutually exclusive in the
sandboxed placement, and the opt-in wins.** Off (the default): `--restart no`, and the sandbox is
idle-stopped like the host process. On: `--restart unless-stopped`, and the sandbox is not
idle-stopped, because the user has asked the platform to keep it. The setting's copy says so at the
point of choosing it.

**This amended the spec, and the user approved it on 2026-08-27.** FR-018's "identically", FR-019,
FR-022 and User Story 4 scenario 5 were written as though both could hold at once; the spec now
carries the exception (FR-022, FR-022a, US4 scenarios 5–6, Clarifications). See `plan.md` →
Complexity Tracking.

**Alternatives considered and rejected**: a host-side one-shot timer left behind at quit
(`systemd-run --on-active=30min`, `launchd`, Task Scheduler — three implementations, and it dies
with a client that crashed rather than quit, and it reintroduces exactly the platform-registered
job this feature removes); keeping PID 1 alive as a hibernating supervisor while the daemon exits
(FR-019 forbids a container left running with no service in it, and it is the orphan shape 027
already fought); `docker update --restart=no` at quit time (the container would then also not come
back after a reboot during the very window the opt-in is about).

---

## R2a — How the daemon inside the container learns it must not idle-stop *(follows the approved amendment)*

FR-022a was added with the amendment: turning the opt-in off must return the sandbox to the idle
rule without the user stopping or restarting anything by hand. That needs a mechanism, and there are
only two shapes.

**Decision — creation-time, and a toggle makes the container stale.** The suppression rides in the
container's environment at creation, `MICOLD_IDLE_STOP=off`, immediately beside the
`MICOLD_IMAGE_REFERENCE` that `sandbox/argv.rs:114` already passes for exactly this reason: the
daemon inside a container cannot see how its container was created. The restart policy is fixed at
creation too, so both halves of the opt-in have the same lifetime, and flipping the setting on a
running sandbox moves it to `SandboxState::Stale` through the existing `mount_set_changed` shape —
"the mount set is fixed at creation … the only question is who decides when to take the
interruption, and the answer is the user" (`sandbox/lifecycle.rs:36`). The recreation path a stale
sandbox already has then applies the new policy and the new environment together.

**Rejected — syncing it live over the protocol.** The daemon could learn the current value from the
settings it already serves and suppress the rule accordingly. It is a smaller code change and it is
wrong: it fixes only half the problem. The container's `--restart unless-stopped` is baked in at
creation, so a daemon that obeyed a live "off" would exit cleanly and be restarted by the runtime —
the measured restart loop of R2, reached by a different road. Any correct answer has to change the
restart policy as well, which only the host side can do, which is the recreation this decision
already uses.

**Consequence for the tasks**: the opt-in must be part of what makes a sandbox stale, and a test has
to assert that, or FR-022a silently does not hold. Nothing else about the sandbox changes.

---

## R3 — A window that survives suspend

**Decision**: A new `micold_core::clock` exposing a monotonic-and-suspend-inclusive reading, with
three thin platform impls behind one function, plus a 30-second tick that compares *now* against a
recorded deadline rather than a single long sleep.

**Rationale**: FR-011 wants both halves: suspended time counts, and a wall-clock change must not
move the deadline. `Instant`/`tokio::time::sleep` give the second and not the first — on Linux
`Instant` is `CLOCK_MONOTONIC`, which does not advance across suspend, so a laptop closed for eight
hours would still owe the full thirty minutes on waking (the spec's own edge case). `SystemTime`
gives the first and not the second — a clock correction would move the deadline, and FR-011 forbids
that in both directions. The suspend-inclusive monotonic clocks are exactly the right primitive:

| platform | call | crate already present |
|----------|------|----------------------|
| Linux | `clock_gettime(CLOCK_BOOTTIME)` | `libc` (already a `cfg(unix)` dep of core) |
| macOS | `mach_continuous_time()` | `libc` |
| Windows | `GetTickCount64()` | `windows-sys` (already a `cfg(windows)` dep of core) |

No new dependency, and it lands in `micold-core` where Principle VI's "platform differences behind a
clear abstraction" belongs — the daemon then never names an OS. The ticking evaluation (rather than
one `sleep(30min)`) is what makes waking from suspend prompt: the first tick after resume sees the
deadline already passed and stops within the tick interval.

**Alternatives considered**: the `boot-time` crate (a real dependency for ~30 lines of `cfg`
code, and thinly adopted — the constitution's dependency rule points the other way); `max(monotonic,
wall)` (a forward clock jump would shorten the window to zero, which FR-011 names explicitly);
accepting plain `Instant` and dropping the suspend clause (cheap, but it makes a laptop that wakes
at 09:00 hold the service until 09:30 for no reason a user could explain).

---

## R4 — Leaving nothing behind

**Decision**: Idle stop unwinds — drop the state so `PtySession::Drop` runs, then return from
`run()` — never `std::process::exit`.

**Rationale**: FR-014 ("nothing the service owned remains resident") and FR-006b (sessions end with
the service) are already implementable by existing code, but only on the unwinding path.
`PtySession::Drop` calls `self.kill()`, which calls `platform::terminate_process_tree(pid)`
(`crates/micold-daemon/src/supervisor.rs:366-382`) — the whole process group, not just the direct
child, and there is a test for that (`kill_reaps_the_whole_process_group`, `:483`). `process::exit`
skips every destructor, which would leave both the session process tree and the endpoint behind
(the endpoint is unlinked by `BoundListener::Drop`, `crates/micold-daemon/src/singleton.rs:52`).

Order matters and is the contract: stop accepting → mark the catalog's live sessions as
interrupted-resumable and persist → drop sessions (killing process trees) → drop the bound listener
(unlinking the socket and releasing the `flock`) → return. The `flock` is the liveness beacon the
next start tests, so releasing it last is what makes FR-013 true.

**Alternatives considered**: `process::exit(0)` after a manual teardown (duplicates what `Drop`
already does and drifts the moment a new owned resource appears); SIGTERM to self (the same
destructor problem, plus it would look like a kill in the diagnostics FR-024 asks to keep
distinguishable).

---

## R5 — The connection that arrives as the window expires

**Decision**: Close the accept side first and treat "accepted but the daemon is going" as an
ordinary disconnect the client's existing reconnect loop absorbs; the client's connect path already
spawns a fresh daemon when nothing is listening.

**Rationale**: FR-016 asks for no user-visible failure, and the machinery is already there —
`crates/micold-client/src/daemon.rs:126` has a 1-second `RECONNECT_BACKOFF` and an outer loop that
reconnects, and `micold_core::connect` spawns a detached daemon when the endpoint does not answer
(its module docs call this the cold-start path). The single-instance dance in `singleton.rs` makes
the follow-up safe: `connect()` is the liveness discriminator and the `flock` arbitrates the race,
so a client that arrives mid-shutdown either connects to the still-listening daemon or finds nothing
and becomes the starter. What must be checked, not assumed, is that one transient failure does not
flash the connection banner — the banner is fed by `Message::DaemonConnectFailed`, and the quickstart
records a run that watches for it.

**Alternatives considered**: a grace period where the daemon refuses new connections but stays up
(adds a state to the handshake for a window measured in milliseconds); an "I am stopping" wire
message (a new protocol frame for a case the reconnect loop already handles).

---

## R6 — Noticing that a client is gone

**Decision**: Rely on stream EOF, which is what `deregister` already keys on; no keepalive.

**Rationale**: FR-010 gives one minute to notice an unclean disconnect. On both transports the
kernel closes the peer's socket when the client process dies — a Unix socket immediately, loopback
TCP with a FIN/RST — and `serve_connection` returns from `route` and deregisters at
`server.rs:397`. The writer task independently detects a failed push and releases attachments
(`server.rs:387`). The residual case is a peer that is neither closed nor readable (a paused VM, a
severed loopback), which no local IPC realistically produces; the protocol already has
`Ping`/`Pong` (`protocol/messages.rs:403`) if it ever does.

**Alternatives considered**: a mandatory client heartbeat with a server-side deadline (a new
protocol obligation, and a client that is merely busy would look dead).

---

## R7 — Un-registering the systemd units

**Decision**: Two halves. The package stops shipping the units (delete both asset lines from
`crates/micold-client/Cargo.toml`, delete `packaging/micold-daemon.{service,socket}`), and the
**client** disables any enabled per-user units once, at startup, before it connects.

**Rationale**: dpkg removes a file that the old version of a package shipped and the new one does
not, so the unit files themselves need no maintainer script. What dpkg cannot touch is the per-user
enablement symlink under `~/.config/systemd/user/sockets.target.wants/`: a root `postinst` has no
route to a per-user manager — the finding feature 010 recorded as research R5.1 and the reason
`logout_survival` lives in the client at all. Left alone, that symlink points at a unit file that
no longer exists and the user's manager logs a failure on every login. So FR-003's "must not require
the user to run a command" resolves to: on Linux, the client runs `systemctl --user disable --now
micold-daemon.socket micold-daemon.service` when either is enabled, ignores every failure (a machine
with no user manager is not a problem to report), and proceeds to its ordinary auto-spawn. The
existing `logout_survival` module is the model for shelling out to `systemctl` off the update
thread; its enable path is deleted and this is what replaces it.

Note the ordering hazard: the migration must run *before* the connect/auto-spawn attempt, or the
first launch after an upgrade attaches to the socket-activated daemon it is about to orphan.

**Alternatives considered**: a `postrm` running `systemctl --global disable` (wrong scope — it
edits `/etc/systemd/user`, not the user's own enablement, and still needs root at a moment when no
user session exists); leaving the enablement in place (a login-time failure for every user who ever
used the opt-in).

---

## R8 — The blast radius of removing logout survival

**Decision**: Delete the capability from the host-process placement in one change: the core module,
its one caller, the menu entry, the outcome message plumbing, and the docs sentence.

Touched, from `grep`:

- `crates/micold-core/src/logout_survival.rs` — the module; `enable`/`enable_for`, `SurvivalOutcome`
  and its four variants. `PendingSandboxRestart` and the sandbox arm are the part that survives, so
  what is deleted is the Linux host-process path, not the file wholesale.
- `crates/micold-client/src/shell/service_control.rs:70` — `on_logout_survival_requested` and
  `on_logout_survival_outcome`, plus the overflow-menu item and `Message::LogoutSurvival*`.
- `crates/micold-daemon/src/server.rs` — `systemd_listener()` and `serve_unix`, the
  `LISTEN_FDS` adoption they exist for, and the `listenfd` dependency in
  `crates/micold-daemon/Cargo.toml`.
- `docs/daemon.md:309` — the `systemctl --user enable --now micold-daemon.socket` instruction, and
  the surrounding claim that the session service can be moved under the user manager.
- `docs/user-guide/settings.md`, `docs/user-guide/sandboxed-daemon.md` — the sandbox toggle's copy,
  which now also has to say that a kept sandbox is not idle-stopped (R2).

**What stays**: the sandbox's `survive_logout` profile field, its settings toggle
(`crates/micold-client/src/ui/settings/daemon.rs:49`), and `argv::restart_policy` — the promise the
spec's FR-005b keeps, because a container runtime's restart policy is not a session-scoped service
registration. The field's *user-facing* name and copy change; the field itself does not, which keeps
the change out of the persisted settings schema.

**Alternatives considered**: leaving the module in place unused (dead code that reads as a supported
feature); a deprecation period (there is no external consumer — it is a menu item in a desktop app).

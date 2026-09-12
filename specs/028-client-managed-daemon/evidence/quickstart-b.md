# Quickstart Part B, executed (T066)

**Date**: 2026-09-12 · **Machine**: this workstation, kernel 7.0.0-30-generic, Docker 29.5.1
**Binaries**: a matched `micold-ai-ide` / `micold-daemon` pair built from this tree at `d6d995e9`
and pinned outside `target-shared/` — the pair `evidence/t065-pinned-pair.md` verifies
**Image** (B5, B6): `micold-daemon:dev`, rebuilt from this tree by `mise run image`

Part B is the half of the quickstart no test suite reaches: it is made of real clocks. Every
window below is a **real thirty minutes** — nothing here shortened one with `MICOLD_IDLE_STOP`,
because a rule that only holds at 30 seconds is not the rule the spec states.

## Outcome

| | scenario | outcome |
|---|---|---|
| **B1** | a real thirty minutes on the host | **passed**; one half of step 5 shown in B4 instead (see below) |
| **B2** | the window is about connections, not activity | **passed** |
| **B3** | suspend | **not run** — needs the machine to sleep; reasoned about, not ticked |
| **B4** | a live session does not save the service | **passed**, all four steps |
| **B5** | the sandbox, opt-in off | **passed** |
| **B6** | the sandbox, opt-in on | **passed**, all three steps |
| **B7** | upgrade migration on a real package | **not run** — needs a fresh VM and a root install |
| **B8** | nothing is registered on a clean install | **not run** — same |

## How it was run

Not by hand at a desk. Each scenario got a private Xvfb (1600x1400x24, lavapipe), a private
`HOME`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and a short `XDG_RUNTIME_DIR` under `/tmp`, and its own
seeded `projects.json` with a git worktree at `<proj>/.claude/worktrees/feature`. That isolation is
also the *"is this process mine?"* predicate the `visual-pass` skill insists on: every check below
reads `XDG_RUNTIME_DIR` out of `/proc/<pid>/environ` before believing a pid belongs to this run.

Three details cost a round each and are worth writing down.

- **`XDG_RUNTIME_DIR` must be short.** The session scratchpad path alone is longer than
  `sun_path`'s 108 bytes, so every attach failed with *"local socket name length exceeds capacity
  of sun_path of sockaddr_un"* until the runtime dirs moved to `/tmp/vp<tag>`.
- **A worktree only appears if it is under `<repo>/.claude/worktrees`.** `worktree::discover`
  surfaces nothing else, so a worktree created beside the project showed the sidebar's
  *"No worktrees yet."* and there was no route to a session at all.
- **The sandbox container's name is a constant** (`micold-sandbox`, `shell/sandbox.rs`). B5 and B6
  therefore cannot run at the same time on one machine however well isolated their data
  directories are; they ran one after the other.

## B1 — a real thirty minutes on the host

**Passed, except for one half of step 5** — shown in B4 instead, for the reason below.
Display `:81`, runtime dir `/tmp/vpb1`, daemon pid **835791**.

| step | what happened |
|---|---|
| 1–2 | session started in the `feature` worktree, client quit at **08:37:32Z**; the daemon logged `client disconnected client=1` at `08:37:33.099579Z` and kept running |
| 3 | +29 min (sample `09:06:40Z`): `b1=835791` with `daemon.lock` **and** `daemon.sock` present. +31 min (sample `09:08:41Z`): `b1=gone` |
| 4 | `/tmp/vpb1/micold/` holds `daemon.lock` only — no socket. No process left descended from it |
| 5 | reopened and attached in **0.266 s** (`launch_at=09:08:55.435855193Z`, `attached_at=09:08:55.703061290Z`), well inside SC-006's 3 s. No banner, no error |

The daemon named its own way out rather than simply vanishing:

```
2026-09-12T09:07:38.937448Z  INFO micold_daemon::server: stopping: nothing has been connected for the whole idle window reason=Idle
2026-09-12T09:07:38.988640Z  INFO micold_daemon::server: sessions handed off as interrupted-resumable sessions_marked=1 sessions_stopped=1
```

That is **30 min 05.8 s** after the disconnect — the window plus one 30 s tick, which is the
granularity `idle.rs` is built to have.

### The one part of step 5 this run could not show, and why

Step 5 also asks that the reopened client show *"the session marked resumable"*. It did not: the
daemon logged `pruned empty sessions … count=1` and then `session start failed … err=no such
session in the catalog`, and the sidebar came back with no session row.

That is not this feature. The stand-in `claude` on this harness's `PATH` is a bare
`bash --norc --noprofile -i`, and both the restore path and the pruning path key off the **real**
CLI's transcript file: `present_interrupted_resumable` marks only sessions
`has_recorded_conversation` is true for, and `prune_empty_sessions` (pre-existing, feature 026)
archives exactly the ones it is false for. No transcript is written, so the handed-off session was
archived on restart. A real `claude` given no prompt behaves the same way — the container in B5
finished its session with a `~/.claude` containing only `backups/`, no `projects/` at all.

What B1 step 5 is really asking about was therefore checked in **B4** below, with the transcript
present. The handoff itself is proved here regardless: `sessions_marked=1` is the daemon writing the
durable record before anything was killed, which is the G5 order.

## B2 — the window is about connections, not activity

**Passed.** Display `:82`, runtime dir `/tmp/vpb2`, daemon pid **835924**.

The client opened at `08:36:11.099692Z` (`client attached to daemon`) and was then left strictly
alone — no keystroke, no mouse, no session activity of any kind. Its daemon log still contains
**zero** `client disconnected` lines and **zero** `stopping:` lines, and at `11:29:50Z` the client
(pid 835888) and the daemon (pid 835924) are both still there with `daemon.lock` *and* `daemon.sock`
in `/tmp/vpb2/micold/` — **2 h 53 min** of an idle-but-connected client, almost six times the
window. It passed the scenario's 35 minutes at `09:11:43Z` and was simply left running.

Its two neighbours, whose clients had quit, both stopped inside that same span (B1 at `09:07:38Z`,
B4 at `09:07:43Z`) on the same binaries and the same 30 s tick. The rule counts connections, not
activity.

## B3 — suspend

**Not run**, and it is the one scenario on this list that a private display cannot reach: it asks
for the *machine* to sleep. This workstation runs other people's work — other Claude sessions hold
build locks, containers and half-finished branches on it — so suspending it is not a step this pass
gets to take on its own authority, and a suspend that only put this session to sleep would not be
the test.

What B3 exists to catch is whether the idle window is **inclusive of suspended time** — a window
measured by a monotonic clock that stops while the machine sleeps would keep the service alive
across a two-hour nap, which is the bug. `data-model.md` G3 states that it must be inclusive, and
`evidence/us2-presence.md` already records that a unit test cannot reach the question. The code
answers it by reading `CLOCK_BOOTTIME` rather than `Instant` (`micold-core/src/clock.rs`), which is
monotonic *and* keeps counting while the machine sleeps — and the newtype around it exists so that
no wall-clock value can reach the rule at all. That is the design; it is **not** the same as having
watched a machine wake up.

Nor could this pass approximate it by comparing the two clocks: this machine has not suspended
since boot, so `CLOCK_BOOTTIME` and `CLOCK_MONOTONIC` read identically here (6808.628 s apiece,
difference 0.000 s) and the divergence B3 turns on has nothing to show. Left unticked.

## B4 — a live session does not save the service

**Passed, all four steps.** Display `:84`, runtime dir `/tmp/vpb4`, daemon pid **836069**, session
`4247e158-88e6-4ae8-9cb4-405834869775`.

Steps 1–3: `sleep 3000` started in the session (pid **836460**), client quit; `client disconnected`
at `08:37:33.452593Z`. At `09:07:40Z` both were still there; by the `09:08:11Z` sample both were
gone — `b4=gone` and `sleep3000=gone` in the same sample, not one and then the other. The daemon
said why:

```
2026-09-12T09:07:43.259426Z  INFO micold_daemon::server: stopping: nothing has been connected for the whole idle window reason=Idle
2026-09-12T09:07:43.310990Z  INFO micold_daemon::server: sessions handed off as interrupted-resumable sessions_marked=1 sessions_stopped=1
```

A live session did **not** hold the service open. This is the clarified rule, chosen over "never
exit while a session is alive", working as chosen.

Step 4 — reopened at `09:13:51.879806775Z`, attached in **0.262 s**:

```
09:13:51.912963Z  INFO …server: presented interrupted-resumable sessions after restart count=1
09:13:51.923598Z  INFO …server: client attached to daemon client_build=micold-ai-ide/0.12.1
09:13:52.082360Z  INFO …state: session started session=4247e158-… mode=AiCli launch=Resume
```

Read the order: the daemon presented the session as interrupted-resumable **before any client was
connected**, and relaunched nothing of its own. The start 170 ms later is the client's single
explicit `SessionStart` for the one session it restores — feature `025` FR-004a, bounded by
`connecting_starts_only_the_session_it_restores`. And the interrupted work did not come back:
the resumed session's only child is a fresh shell, with no `sleep 3000` anywhere. Screenshot
`evidence/quickstart-b4-reopened.png`: the session row is back under **Feature**, no banner, no
error.

**Disclosed**: to get past the transcript problem B1 hit, this reopen was done with a transcript
seeded by hand at the path the real provider derives —
`$HOME/.claude/projects/<encoded-cwd>/<session-id>.jsonl`, two JSONL lines. Nothing about the
daemon was changed or stubbed; the file is simply what a real `claude` writes and the stand-in does
not, and it is the input `has_recorded_conversation` reads. Every timestamp above is the daemon's
own.

## B5 — the sandbox, opt-in off

**Passed on every step the harness could reach; one half of step 4 hit B1's transcript problem.**
Display `:85`, runtime dir `/tmp/vpb5`, settings written by the GUI (not by hand):

```json
"daemon": { "placement": "local_sandbox",
  "sandbox": { "image": {"kind":"local_build","path":null,"reference":"micold-daemon:dev"},
               "network":"no_outbound", "runtime":"docker", "survive_logout": false } }
```

`local_build` is the deliberate choice over a registry reference: `acquire_image` inspects the local
image store first for *every* source, so a locally-present image never touches the network, and
`LocalBuild` is the only kind whose `refuses_fingerprint_mismatch()` is true — the stricter
configuration, not the convenient one.

| step | what happened |
|---|---|
| 1–2 | container `micold-sandbox` created at `08:45:56.125782207Z` from `micold-daemon:dev` (`sha256:b7917005…`), daemon listening `(sandboxed) addr=0.0.0.0:7727`, `idle stop armed window=1800s`; session `c76d14f6-…` started `08:47:31Z`; client quit, `client disconnected` at `08:50:16.431454Z` |
| 3 | the container ran on for the whole window — 60 samples at 30 s, every one `running exited=0 restarts=0` — and then **exited on its own** at `09:20:26.386262324Z`: `status=exited exit=0 restarts=0 policy=no`. `RestartCount` is unchanged from the `restartcount=0 policy=no` recorded before the quit |
| 4 | reopened `09:20:47.013797084Z`, attached `09:20:48.067394444Z` — **1.054 s**, container creation included, inside SC-006's 3 s. A *new* container (`90beded127f4…`, created `09:20:47.703Z`) on the same mounts; the daemon logged `catalog adopted load_status=Loaded` and the client `attach: connected projects=1 sessions=1` |

The stop was the daemon's own, named:

```
2026-09-12T09:20:26.332653Z  INFO micold_daemon::server: stopping: nothing has been connected for the whole idle window reason=Idle
2026-09-12T09:20:26.383301Z  INFO micold_daemon::server: sessions handed off as interrupted-resumable sessions_marked=1 sessions_stopped=1
```

`09:20:26.332` is **30 min 09.9 s** after the disconnect, and the container's `FinishedAt` is 54 ms
later — the daemon is pid 1, so the container exits because the daemon returned from `run()`, not
because anything stopped it. Restart policy `no` is what makes that an exit rather than the restart
loop R2 measured.

Two things this scenario showed that were not on its list:

- **303 session crash-restart cycles did not postpone the stop by a second.** The real `claude` in
  the container exited immediately on every launch (no terminal, no credentials), and the daemon's
  supervision restarted it 303 times across the window — `session crashed; restarting` /
  `session recovered; running`, the last pair 7 s before the idle stop fired. Session churn is not
  connection, so the window never reset. That is B2's claim again, arrived at by accident.
- **Step 4's "sessions … as they were" is only half shown.** The catalog *was* adopted intact, with
  `c76d14f6-…` present and `archived: false`. The daemon then logged `pruned empty sessions … count=1`
  and archived it — the same pre-existing rule B1 hit, and for the same reason: the crash-looping
  `claude` never wrote a transcript, so `has_recorded_conversation` was false. B4 above is where the
  restore path is shown working, with a transcript present. Nothing about the container half is
  affected: it went away, it came back, and the store it came back to was the one it left.

**Harness note, disclosed**: the sandboxed `claude` first refused to start with *"Temp directory
/tmp/claude-1000 is owned by uid 0, expected 1000"*. The harness `HOME` lives under
`/tmp/claude-1000/…`, Docker creates a bind mount's missing ancestors as root, and that path is
coincidentally also the temp dir `claude` picks for uid 1000. `docker exec -u 0 micold-sandbox chown
1000:1000 /tmp/claude-1000` clears it, and it recurs whenever the container is recreated. It is an
artefact of running the app out of a session scratchpad, not of this feature.

## B6 — the sandbox, opt-in on

**Passed, all three steps.** Display `:86`, runtime dir `/tmp/vpb6`, same image and same
`local_build` source as B5, with one field different — `"survive_logout": true`, set by clicking the
checkbox in the GUI.

That one field is visible at all three layers, which is the whole of US4:

| layer | B5 (off) | B6 (on) |
|---|---|---|
| `settings.json` | `"survive_logout": false` | `"survive_logout": true` |
| container env | *(no `MICOLD_IDLE_STOP`)* | `MICOLD_IDLE_STOP=off` |
| `docker inspect .HostConfig.RestartPolicy.Name` | `no` | `unless-stopped` |
| daemon's first words | `idle stop armed window=1800s` | `the idle stop is disabled; this daemon runs until it is asked to stop` |

Steps 1–2: container created `09:21:49.239352972Z`, session `56f41d4a-…` started in it, client quit
— `client disconnected client=1` at `09:23:25.839758Z`. Two hours and six minutes later, at
`11:29:50Z`:

```
status=running restarts=0 policy=unless-stopped
started=2026-09-12T09:21:49.239352972Z  finished=0001-01-01T00:00:00Z
```

`StartedAt` is the original creation, `FinishedAt` has never been set and `RestartCount` is still 0
— so the container did not stop and get restarted by the policy; it simply never stopped. The
daemon log contains **zero** `stopping:` lines. B5's identical container, one field different, was
already `exited` 30 minutes after its client left.

Step 3 — the copy at the toggle, captured *before* the box was ticked, with the container placement
already selected (`evidence/quickstart-b6-survive-logout.png`):

> **Keep the service running when I'm signed out or away**
> The container is created with a restart policy, and is not stopped for being unused, so this takes
> effect the next time the sandbox starts.

Both halves of the consequence — the restart policy, and *"is not stopped for being unused"* — are
stated before the choice is made, and so is the fact that it applies at the next sandbox start
rather than immediately. That is what step 3 asks for.

The `claude` inside crash-restarted continuously here too (the same no-terminal exit as B5). With
the opt-in on it changes nothing either way: nothing was going to stop this daemon.

## B7, B8 — the package scenarios

**Not run here**, for the reason they were not run at T028 either: both are written for a *fresh
VM* and a *root install of a real package*, and installing this feature's `.deb` over the release on
the development machine would replace the copy the user actually runs.

`evidence/us1-packaging.md` records what could be established without that VM — the built package's
complete file list (no path under `usr/lib/systemd`), its control tarball (no maintainer scripts at
all, so nothing can register or start anything), and the same facts about the *previous* release for
contrast. What is still owed to those two boxes is the measurement rather than the inference:
`systemctl --user is-enabled micold-daemon.socket` before and after an upgrade on a machine where
the old opt-in was really enabled, and `systemctl --user list-unit-files | wc -l` unchanged across a
clean install. Both remain unticked.


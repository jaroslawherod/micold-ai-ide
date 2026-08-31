# FR-024: a first run against the published image

**Date**: 2026-08-27 · **Host**: Linux 7.0.0-30-generic, Docker 29.5.1 (build 2518b52) ·
**Client/daemon**: built from `7b9c0fa8`, version `0.12.0` · **Image**:
`ghcr.io/jaroslawherod/micold-daemon:0.12.0`, pulled from GHCR (`sha256:5001ee01dbbb…`)

`evidence/image-publishing.md` established that the release publishes an image and that the
manifest is pullable anonymously. Neither of those is FR-024's claim. FR-024 says a *first run*
requires no manual image preparation, and the only way to check that is to have a client do it.

This is that run. Nothing here was simulated: a real client binary, on a real X server, against the
real Docker daemon, pulling from the real registry.

## What made it a first run rather than a re-run of the test suite

The real-runtime suite (`mise run test-sandbox`, 23 tests) builds `micold-daemon:dev` locally and
asserts against that, so it passes with no registry in existence — which is exactly how the wrong
namespace survived to a release. Two things had to be different here:

- **The client resolves the reference, not the test.** The settings file seeded for the run carries
  `daemon: { placement: "local_sandbox" }` and **no `sandbox` block at all**, so rule S-2 applies and
  `DEFAULT_IMAGE` is what resolves. A run that names the image in its settings proves the plumbing
  and not the default.
- **The binary was pinned before it was launched.** `strings` on the copied binary found exactly one
  `ghcr.io/jaroslawherod/micold-daemon:0.12.0` and zero `ghcr.io/micold/micold-daemon`. The shared
  target directory is whatever branch built last (CLAUDE.md), and a pass that launches from it can
  screenshot another branch's code with a clear conscience.

Isolated on `DISPLAY=:83` with `XDG_RUNTIME_DIR=/tmp/vp83` and `XDG_DATA_HOME=/tmp/vp83/data`, per
the `visual-pass` skill.

## The chain, in order

| step | what was observed |
|---|---|
| The client compiles the reference the release publishes | one `ghcr.io/jaroslawherod/micold-daemon:0.12.0` in the binary, no retired namespace |
| Settings omit the image, so the default resolves | seeded document has `placement` only |
| A container is created from the pulled image | `micold-sandbox  ghcr.io/jaroslawherod/micold-daemon:0.12.0  Up 34 seconds`; `docker inspect`'s image id matches the pulled `sha256:5001ee01dbbb…` |
| The daemon runs *inside* it | `listening (sandboxed) addr=0.0.0.0:7727` — the sandboxed bind, not the host socket |
| The client attaches over the published control channel | `client attached to daemon client_build=micold-ai-ide/0.12.0 client_window=1505784`, then `project attached client=1 project=/tmp/vp83/proj` |
| A session starts | `session started session=3f17f4d0-… mode=AiCli launch=Fresh` |
| The AI CLI runs in the container, not on the host (FR-023a) | `docker exec micold-sandbox ps` → PID 17, `claude --session-id 3f17f4d0-… --settings /var/lib/micold-ai-ide/hooks/….json` |
| The client renders its output | "Welcome to Claude Code v2.1.247" in the terminal pane |
| The network posture is the one configured | network `micold-sandbox-net`, `com.docker.network.bridge.enable_ip_masquerade: false`; container `restart=no`, matching `survive_logout: false` |

The client log's first line is `attach: failed reason=no sandboxed daemon is listening on
127.0.0.1:7727`, followed by `attach: connected`. That is the intended shape — the client polls
while the container comes up — and it is worth recording that the failure is *visible* in the log of
a successful run, so a reader triaging a real failure does not mistake it for the cause.

## Two things this run found that no test had

### The AI CLI cannot reach Anthropic, and that is the feature working

The terminal showed "Unable to connect to Anthropic services". `network: no_outbound` is the default
posture and the bridge has masquerading disabled, so this is US1's claim demonstrated from the user's
side rather than from `docker network inspect`. It also means **this run did not exercise a working
AI session** — it exercised a session that starts, runs, and cannot reach the network. Whether the
CLI works with `network: outbound` is not settled here.

### The session restarted 24 times and never gave up (010 FR-022a, not 027)

`claude` exited about 10 s after each start, the supervisor restarted it, and it did that 24 times
across 13 minutes with no `gave up` and no `Failed`. `MAX_RESTART_ATTEMPTS` is 3, so the budget
should have been spent in under a minute.

It is not spent because of the survivor reset in `state.rs`: a session still alive on the *next
supervision tick* is marked `Running`, which clears the counter. A tick is ~250 ms, so the guard
only fires against a process that dies inside a quarter of a second. Anything that starts, runs,
and dies — the shape of every configuration and connectivity failure — restarts forever. The log
reads `session crashed; restarting` / `session recovered; running` at 10-second intervals, which
describes a healthy session recovering rather than a loop.

This is feature 010's crash-loop guard, not 027's, and it is recorded rather than fixed here. It is
worth recording because it is a defect that only appears when something *else* is wrong, which is
why 010's own tests do not catch it: they model a process that exits immediately, and every one of
them passes.

## Cleanup

Container and network removed, `Xvfb :83` killed by PID after confirming ownership via
`/proc/<pid>/environ`. The user's own client and daemon, and nine other agents' daemons, were
identified and left running. **The container outlived the client** — it was still `Up` after the
client process was gone. The client was killed rather than quit through its own UI, so this says
nothing about the normal quit path; it does say that an abnormally terminated client leaves a
container behind, which the next run adopts rather than recreates.

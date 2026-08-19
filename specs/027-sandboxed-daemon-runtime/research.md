# Phase 0 Research: The Session Daemon in a Sandbox

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-19

Ten questions from the plan, answered as Decision / Rationale / Alternatives considered. Three (R1,
R2, R5) were blocking. Findings marked **measured** were produced by running the command against
Docker 29.5.1, `overlayfs` storage driver, cgroup v2, x86_64 Linux; they are recorded verbatim so a
reviewer can re-run them rather than trust the prose.

## Prior art: three published patterns, and where we land

Before deciding anything, the three containerization patterns published at
`pi.dev/docs/latest/containerization` were checked, because they solve a visibly similar problem
(an AI coding agent that must touch a workspace without handing over the host).

| Pattern | Split | Our equivalent | Adopted |
|---|---|---|---|
| **Gondolin extension** | agent process on the host, *tool execution* delegated to a per-workspace micro-VM; host cwd appears at `/workspace` | daemon stays a host process, individual sessions' PTYs run sandboxed | **No** |
| **Plain Docker** | the whole agent process in a container; cwd bind-mounted to `/workspace`; named volume for agent settings so host auth files are not exposed | the whole daemon in a container; projects bind-mounted; named volume for daemon state | **Yes — this is the design** |
| **OpenShell** | a gateway brokers a sandbox that may be local or remote, over docker, podman, a VM, or Kubernetes, with filesystem/process/network/credential policy | `Placement` (local host / local sandbox / remote, FR-003a) over `ContainerRuntime` (FR-020) | **Shape only, not the dependency** |

**Gondolin is rejected** on scope, not merit. It puts the isolation boundary *inside* the daemon
rather than around it, which contradicts the feature request as written ("daemon should run ... as
docker container") and would multiply sandboxes per workspace rather than the single sandbox settled
during `/speckit-specify`. It is, however, the shape that would give **per-session** isolation, which
Principle II values and which one-sandbox-for-all-projects deliberately does not provide — noted in
Out of Scope as the natural successor, not a competitor.

**Plain Docker is the design**, and two of its specifics are taken directly: daemon state lives in a
runtime-managed **named volume** rather than a host bind mount (FR-011), and its stated trade-off —
"provider API keys enter the container" — is treated as the thing to avoid rather than accept.
FR-004a's default-off, per-item credential opt-in is the stricter version of Pi's
"mount a named volume so host auth files are not exposed".

**OpenShell confirms the two axes** this plan already separates: *where* the daemon runs (placement,
local or remote) and *what runs it* (runtime, docker/podman/other). That an independent design
arrived at the same decomposition is the main reason FR-003a's "must accommodate remote later" is
treated as a structural requirement now rather than a promise. Its credential model — brokering at
the boundary instead of passing secrets inside — is recorded as the **direction** for FR-004a beyond
v1, not as v1 itself. No dependency on OpenShell is taken; it is a gateway product, and Principle IV
forbids making a working setup contingent on one.

**Where we deliberately diverge from both Pi container patterns: the mount path.** Both mount the
workspace at a fixed `/workspace`. They can, because nothing on the host runs git against those
paths — the whole agent is inside. Our client does (`micold-client/src/shell/workspace.rs`,
`shell/capabilities.rs`). See R2.

## R1 — How does a host client talk to a containerised daemon on all three platforms? *(blocking)*

**Decision.** **TCP on the loopback interface, authenticated by a shared secret** that the client
generates and the runtime mounts into the container as a file. The daemon reads its token from the
mounted path; the client presents it in the handshake. The existing Unix-socket / named-pipe
transport is kept unchanged for the host-process placement, so nothing regresses for users who never
enable the sandbox.

**Rationale.** A bind-mounted Unix socket is the obvious first choice and the one that would need no
protocol change, because the current design authenticates by filesystem permission alone: a `0700`
directory owning the socket (`micold-core/src/endpoint.rs`, FR-030). It fails on the platform matrix.
On macOS and Windows the project directory reaches the container through Docker Desktop's file-sharing
layer (virtiofs / gRPC-FUSE), which passes file *contents*, not socket semantics — a Unix socket
bind-mounted through it is not connectable. Socket-only therefore means Linux-only, which violates
Principle VI outright. TCP on loopback works identically on all three because it is the transport
Docker Desktop is built to forward (`-p 127.0.0.1:<port>:<port>`).

Loopback TCP has no filesystem-permission property: any local process can connect to
`127.0.0.1:<port>`. Shipping that inside a *security* feature would be a regression, so the token is
not optional — it is why the protocol moves 5 → 6 (see `contracts/protocol-delta.md`). The token file
is created `0600` in the existing per-user state directory and mounted read-only, so the same
filesystem permission that protected the socket now protects the secret.

**Alternatives considered.**
- *Unix socket bind-mounted into the container.* Rejected: Linux-only, per above. This is the single
  decision that forces the protocol bump, and it is recorded in plan.md's Complexity Tracking for
  that reason.
- *Per-platform transport — socket on Linux, TCP elsewhere.* Rejected: two transports to maintain and
  test forever, on exactly the platform axis where test depth is already thinnest. A single transport
  that is slightly more expensive everywhere beats two that are cheap in one place each.
- *`docker exec` with the daemon's stdio as the channel.* Rejected: it makes the client the container's
  parent for connection purposes, so reconnecting a running daemon after a client restart — the whole
  point of the daemon (FR-014) — has no mechanism.
- *vsock.* Rejected: not exposed by Docker Desktop to host clients in a supported way, and podman's
  support differs; it would put us back on per-runtime transports.

## R2 — Can projects be mounted at identical absolute paths, and what happens on Windows? *(blocking)*

**Decision, two parts.**

1. **For this feature (local sandbox): mount each registered project at its own absolute host path.**
   `/home/u/p` is mounted at `/home/u/p`. Free on Linux and macOS, and it removes the problem rather
   than managing it.
2. **For Windows, and for the remote placement later: route git through the daemon** rather than
   translating paths. The client gains a second `micold_core::git::Git` implementation that issues
   the call over the existing daemon connection instead of spawning `git` locally.

**Rationale.** Git records **absolute** paths in worktree metadata — in
`.git/worktrees/<name>/gitdir` and in each worktree's own `.git` file. Both processes run git: the
daemon (`micold-daemon/src/server.rs`, `state.rs`) and the client
(`micold-client/src/shell/workspace.rs`, `shell/capabilities.rs`). If the container sees a project at
a different path than the host does, the two disagree about `git worktree list`, and a worktree
created by one is broken for the other. That is a Principle III failure, not a cosmetic mismatch,
and `<project>/.claude/worktrees/` is where this feature's worktrees live
(`micold-core/src/worktree.rs:92`).

`C:\Users\u\p` has no Linux-container equivalent, so on Windows the mapping is unavoidable — this is
the one place the feature does not fall out of the existing design for free, and plan.md records it
as a conditional on Principle VI.

Part 2 resolves that conditional, and the cost is far lower than it looks because **the client already
funnels every git call through one injected capability**: `shell/capabilities.rs:64` holds
`git: Arc<dyn Git + Send + Sync>`, constructed once. Adding a daemon-backed implementation is a new
impl of an existing trait plus the RPC messages, not a refactor of call sites — the capability seam
was introduced precisely to replace eleven scattered `GitCli::new()` constructions, and this is that
seam paying out.

It is also the **only** answer that survives the feature's own stated future. FR-003a requires the
placement model to accommodate a remote daemon; when the daemon is remote there is no host filesystem
to run git against at all, at any path. So "client runs git locally" is a local-only assumption that
has to end regardless, and identical-path mounting is best understood as the cheap way to defer it by
exactly one release, not as the permanent answer.

**Alternatives considered.**
- *Mount everything at a fixed `/workspace`, as both Pi container patterns do.* Rejected **for now,
  and only because the client runs git** — Pi can do this precisely because its whole agent is inside
  the container. Once part 2 lands, this becomes viable and arguably preferable, since it stops the
  container layout depending on host usernames.
- *`git worktree repair` after every placement switch.* Rejected: host and container would each repair
  the metadata *away* from the other, converting a static, diagnosable mismatch into an oscillation.
- *Windows containers, preserving `C:\` paths.* Rejected: needs a Windows base image, a Windows-built
  daemon, and Docker Desktop switched out of Linux-container mode — which is mutually exclusive with
  every other container the user runs. Vastly larger deviation than routing git.
- *Dropping Windows support.* Rejected: violates Principle VI.

## R5 — Can a writable-storage limit be enforced portably? *(blocking)*

**Decision.** No — so the runtime **declares** whether it can, and the Settings view shows an
unsupported limit as unavailable **with the reason** rather than accepting a number it will ignore.
FR-015 is thereby honoured as "enforced where the selected runtime can, visibly unavailable where it
cannot" (recorded in plan.md's Complexity Tracking as a softening of the requirement).

**Measured.** On this machine, `--storage-opt size=` **is** accepted:

```
$ docker run --rm --storage-opt size=1G alpine:latest true ; echo $?
0                                     # Docker 29.5.1, storage driver: overlayfs
```

**Rationale.** That exit code is exactly why the capability must be probed rather than assumed in
either direction. The same flag is rejected outright on the older `overlay2` driver unless it is
backed by xfs with `pquota`, and podman's behaviour differs again. Hard-coding "supported" would
break users on common configurations; hard-coding "unsupported" would deny the limit to users like
this one, whose runtime enforces it fine. The probe runs once per runtime, its result is cached with
the runtime's version, and it is the same mechanism R10 uses for every other limit — so storage is
not a special case in the code, only the most likely to come back false.

Silently accepting a limit the runtime drops is the "silent drift" the codebase's endpoint module
already refuses to tolerate; reproducing it in a security feature would be worse, because the user
would believe a bound exists.

**Alternatives considered.**
- *Quota the named volume instead of the container filesystem.* Rejected as the primary mechanism —
  it bounds only daemon state, not what a session writes into `/tmp` or an unmounted path, so it
  under-delivers on the requirement while looking like it delivers.
- *Refuse to start when the limit cannot be enforced.* Rejected: turns a common driver configuration
  into a hard failure, and Principle IV's spirit is that the local path keeps working.
- *Omit the setting from the UI entirely on unsupporting runtimes.* Rejected: a silently missing
  control reads as a bug. Shown-and-disabled-with-a-reason is the accessible form and is what makes
  SC-009 measurable.

## R3 — How do files created in the container end up owned by the host user?

**Decision.** Run the container process as the host user's uid/gid: `--user <uid>:<gid>` on Docker,
`--userns=keep-id` on podman (which maps the invoking user to the same uid inside a rootless
container). The uid/gid are read at sandbox-start time, not baked into the image, so the same
published image works for every user.

**Rationale.** Anything else leaves root-owned files in the user's project after a session writes,
which the user then cannot edit without `sudo` — a worse outcome than not sandboxing. On macOS and
Windows the Desktop file-sharing layer already re-owns written files to the host user, so the flag is
a no-op there rather than a conflict; specifying it uniformly avoids a per-platform branch.

**Alternatives considered.** A fixed non-root uid baked into the image (rejected: only correct for
users whose uid happens to match, and 1000 is a coin flip); post-hoc `chown` of the project after each
session (rejected: racy, slow on large trees, and needs privileges we are trying to drop).

## R4 — How is "network off" expressed without cutting the control channel?

**Decision.** Put the sandbox on a **user-defined bridge network created with IP masquerade
disabled**, and publish the daemon port to loopback as normal:

```
docker network create --driver bridge \
  -o com.docker.network.bridge.enable_ip_masquerade=false <net>
docker run --network <net> -p 127.0.0.1:<port>:<port> ...
```

Outbound connections have no NAT and therefore fail; inbound port publishing is host-side DNAT and
keeps working. The transport from R1 is unchanged whether the network posture is open or closed,
which is the property that matters.

**Measured.** Two configurations were tested. The obvious one does not work:

```
# --internal network + published port
$ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18099/
000                                   # host cannot reach the container at all
egress: BLOCKED   dns: BLOCKED
```

An `--internal` network makes the published port inert, so "network off" expressed the obvious way
severs the control channel — the failure this question exists to avoid. Reaching the container
*directly on the internal bridge's subnet* does work (`http://172.19.0.2:8080/` → `200`, egress still
blocked), but it is Linux-only: on macOS and Windows that subnet lives inside the Desktop VM and is
not routable from the host. It was rejected for that reason.

Disabling masquerade gets both properties with a host-uniform address:

```
# user-defined bridge, enable_ip_masquerade=false, -p 127.0.0.1:18099:80
PUBLISHED_PORT=200                    # control channel works
EGRESS=BLOCKED                        # wget http://1.1.1.1/ fails
EGRESS_DNS=RESOLVES                   # see caveat
```

**Caveat, recorded deliberately.** DNS *lookups* still succeed, because Docker's embedded resolver
forwards them from the host side. Names resolve; connections to the resolved addresses do not. This
is a small metadata channel — a process in the sandbox can exfiltrate bits by choosing what to look
up — and it must be stated in `docs/user-guide/sandboxed-daemon.md` rather than left for a user to
discover. Closing it requires `--dns` pointed at a sink or a network with no DNS at all, which also
breaks any legitimate in-sandbox name resolution; the posture is therefore documented as "no outbound
connections", not "no outbound traffic of any kind".

**Alternatives considered.** `--network none` (rejected: no control channel at all); attaching the
container to two networks, one internal and one bridged (rejected: the bridged one restores egress,
so the isolation is nominal); host firewall rules (rejected: not portable, and needs privileges).

## R6 — How does the sandbox honour the existing session-survival opt-in on all three platforms?

**Decision.** Map the existing opt-in onto the runtime's own restart policy:
`--restart unless-stopped` when survival is enabled, `--restart no` when it is not. The container is
started detached in both cases, so client exit never stops the daemon.

**Rationale.** FR-014b raises the bar on purpose: the host-process mechanism
(`micold-core/src/logout_survival.rs`) manages survival only on Linux, via `loginctl enable-linger`,
and reports `SurvivalOutcome::Unsupported` elsewhere. A container runtime's restart policy is
implemented by a daemon/service that the platform already keeps running across logout and reboot, on
all three platforms — so the sandboxed placement can offer on macOS and Windows what the host
placement cannot. The setting keeps one name and one meaning; only the mechanism behind it differs by
placement, which is the same shape `logout_survival.rs` already has.

`unless-stopped` rather than `always` so that a user who explicitly stops the sandbox stays stopped
across a reboot.

**Alternatives considered.** A separate "keep the sandbox running" toggle (rejected by the user's own
clarification — survival is governed by the existing opt-in); host-side supervision of the container
(rejected: reimplements what the runtime already does, per platform).

## R7 — CLI-and-parse, or an API client, for the runtime abstraction?

**Decision.** Drive each runtime through **its own CLI** with `--format '{{json .}}'`, parsing typed
structures out of the JSON. No `bollard` or other API-client dependency.

**Rationale.** It mirrors `micold-core/src/git.rs`, which drives git porcelain the same way, so the
codebase gains no new integration idiom. It keeps the pure/impure split the constitution's Test-First
principle depends on: argv construction and output parsing are pure functions (`sandbox/argv.rs`,
`sandbox/parse.rs`) with one process-spawn shim behind them (`sandbox/exec.rs`), which is what lets
the entire adapter layer be tested with a **fake runtime binary on `PATH`** — no Docker in CI, on all
three platforms. An API client would move that boundary inside a dependency and make the same tests
require a live daemon. It is also what makes FR-020's "replaceable runtime" cheap: podman's CLI is
near-identical, so a dialect is a table of argument differences rather than a second client.

**Alternatives considered.** `bollard` over the Docker socket (rejected: Docker-specific, so the
abstraction would be a shim around one runtime's API shape; and it puts the socket itself in reach,
which is the thing sandboxing exists to prevent). A vendored OCI/containerd client (rejected:
enormously larger surface for no user-visible gain).

## R8 — How is a stale *development* image detected?

**Decision.** Add a **build fingerprint** to the handshake alongside the existing three identity
constants, and refuse a connection whose fingerprint does not match, with a message naming the image
tag and telling the user to rebuild.

**Rationale.** FR-024d requires refusing a stale image, and the existing constants cannot detect it.
Within a single released version a rebuilt daemon and client share identical `PROTOCOL_VERSION`,
`SCHEMA_HASH` (a build.rs hash over `messages.rs`/`grid.rs`/`envelope.rs`) and `PACKAGE_VERSION`
(`env!("CARGO_PKG_VERSION")`) — see `micold-core/src/protocol/version.rs:16,23,28`. So a `:dev` image
built yesterday against today's client presents three matching numbers and connects, then misbehaves
in ways that look like bugs in the new code. FR-024c makes the maintainers' own rebuild loop a
requirement precisely so this path is exercised, and it is the loop where the failure is most likely.

The fingerprint therefore has to change on **every build**, not every release: a hash of the daemon
binary, or a value stamped in at link time. `contracts/protocol-delta.md` fixes the exact form. This
also explains a previously observed and unexplained symptom — a mixed client/daemon pair from the
shared target directory refusing to connect while printing matching version numbers.

**Alternatives considered.** Comparing image build timestamps (rejected: clock-dependent, and says
nothing about what is *in* the image); comparing the daemon binary's mtime (rejected: not visible
across the container boundary without another mechanism); doing nothing and documenting "rebuild
both" (rejected: FR-024d requires refusal, and the whole point is that this failure does not announce
itself).

## R9 — What happens when the registered project set changes while the sandbox runs?

**Decision.** The mount set is fixed at container creation. Adding or removing a project marks the
sandbox **stale** and surfaces an explicit "restart the sandbox to apply" action; the client does not
restart it silently. Sessions already running are unaffected until the user acts.

**Rationale.** Neither Docker nor podman can add a bind mount to a running container — that is a
runtime constraint, not a design choice, so the only real question is who decides when to take the
interruption. Restarting silently would kill live sessions to service a background settings change,
which is the opposite of what the daemon exists for. Making staleness visible and the restart explicit
keeps the user in control and is consistent with FR-035b's "persistently visible" treatment of a
degraded state.

**Alternatives considered.** Mounting the parent of all projects, or the user's home directory
(rejected: defeats the feature — the sandbox would see everything, including credentials FR-004a
excludes by default). Restarting the sandbox automatically (rejected per above). Refusing to register
a project while the sandbox runs (rejected: unnecessarily harsh, and forces a stop/start anyway).

## R10 — Which limits does each runtime support, and how is that surfaced?

**Decision.** Each dialect declares a `RuntimeCapabilities` set, refined by a **probe** executed once
per runtime and cached against the runtime's version string. The Settings view renders each limit
from that set: supported limits are editable, unsupported ones are shown disabled with the reason.
The same mechanism carries R5's storage answer, so there is one code path, not a special case.

**Rationale.** The alternative to a probe is a table of runtime versions and their behaviours, which
goes stale the first time either project ships a release. Probing is a handful of milliseconds once,
and it turns "does this work here" from a guess into a fact — which is what SC-009 needs to be
measurable at all. Caching against the version string means the probe re-runs exactly when the
runtime changes underneath us.

CPU (`--cpus`), memory (`--memory`), and process count (`--pids-limit`) are supported by both Docker
and podman on Linux and through Docker Desktop's VM on macOS/Windows, so in practice storage is the
limit that varies — but that is an observation, not something the code assumes.

**Alternatives considered.** A static per-runtime capability table (rejected: goes stale, and lies
confidently). Attempting each limit and interpreting the failure (rejected: a failed container start
is an awful place to discover a settings problem, and the error text is not stable enough to parse).

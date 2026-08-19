# Implementation Plan: The Session Daemon in a Sandbox

**Branch**: `feat/run-daemon-inside-an-container-sandbox` | **Date**: 2026-08-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/027-sandboxed-daemon-runtime/spec.md`

## Summary

Give the session daemon a second **placement**: instead of a detached host process, it can run inside
a container that sees only the registered project directories, under a resource budget the user
sets. The client stops assuming where the daemon is and resolves a configured placement at connect
time — which is what makes both this feature and the later remote-host one possible. Settings stops
being a 420-point modal and becomes a full-surface view with a navigation rail, and every daemon
setting moves into a section of its own.

The technical approach, in one paragraph: a `ContainerRuntime` trait in `micold-core` whose
implementations are **argument dialects over a runtime CLI** (`docker`, `podman`) rather than an API
client, keeping the pure/impure split the codebase already uses for `git`; a **loopback TCP
transport with a mounted shared secret** replacing the Unix-socket assumption, because bind-mounted
Unix sockets do not work through Docker Desktop's file sharing on macOS or Windows; **identical
absolute-path mounts** so that host-side and container-side git agree about worktree metadata
without translation; and a **capability probe** per runtime so that limits a runtime cannot enforce
are shown as unavailable rather than silently ignored.

Three things carry most of the risk and are researched in Phase 0 before any code: the transport
(R1), Windows path identity (R2), and the writable-storage limit (R5). Phase 0 also checks this
design against three published containerization patterns (pi.dev): ours is their *Plain Docker*
shape, their *OpenShell* gateway independently confirms the placement/runtime split, and their
*Gondolin* micro-VM pattern is rejected on scope — see research.md's prior-art section.

## Technical Context

**Language/Version**: Rust, stable, pinned for both entry points by `rust-toolchain.toml`.

**Primary Dependencies**: existing — `iced` (GUI), `tokio`/`tokio-util`, `interprocess` (current
transport), `alacritty_terminal` + `portable-pty` (session hosting), `serde`/`serde_json`,
`directories`. **New**: none required. The container runtime is driven through its own CLI with
`--format '{{json .}}'`, mirroring `micold-core/src/git.rs`'s use of git porcelain, so no
Docker-API crate (`bollard`) enters the dependency graph. One new *transport* dependency may be
needed if `interprocess` cannot carry loopback TCP — resolved in R1; the fallback is `tokio::net::TcpStream`,
already in the tree via `tokio`.

**Storage**: unchanged and local-first. Settings gain a sandbox profile in the existing
`settings.json` (schema version 3 → 4, missing-field-defaults contract preserved). Daemon state
(`projects.json`, per-project state, logs) moves into a runtime-managed named volume when sandboxed,
so it survives container recreation (FR-011). No new store, no database.

**Testing**: `cargo test --workspace` via `mise run test`. The runtime adapters are tested against a
**fake runtime binary** placed on `PATH` — a script that records its argv and replays canned JSON —
so the whole adapter layer, including failure modes, is exercised in CI on all three platforms
**without Docker installed**. Real-runtime coverage runs in a Linux-only CI job and in
`quickstart.md`'s manual pass.

**Target Platform**: Linux, macOS, Windows (Constitution VI). Linux containers on all three;
Docker Desktop's Linux VM on macOS/Windows.

**Project Type**: Desktop application, existing three-crate workspace (`micold-core`,
`micold-client`, `micold-daemon`).

**Performance Goals**: sandboxed session start no more than 2s slower than unsandboxed (SC-003);
first-time enable under 5 minutes with continuous progress (SC-004); the new Settings view must not
regress `crates/micold-client/tests/idle_requests_no_frames.rs` — no animation at rest.

**Constraints**: fully functional offline once the image is present, and *reachable* offline via
image import (Principle IV, FR-024a); no GUI framework beyond iced (Principle V); no new
one-off widgets (Principle VIII) — the view composes `NavigationDrawer`, `Select`, `TextField`,
`Checkbox`, `StageProgress`, `ConnectionBanner`, `Accordion`, all of which already exist.

**Scale/Scope**: one sandbox, N registered projects, existing session counts. Roughly: ~8 new
`micold-core` modules (pure, fully unit-tested), 1 new client view plus section modules, 1
Containerfile, 1 `mise` task, protocol version 5 → 6.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Re-checked after Phase 1 (2026-08-19): all eight PASS.** The only gate that was conditional —
VI — is resolved below by R2. Phase 1 added no new deviation; the two remaining entries in
Complexity Tracking are unchanged in substance.

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. The design deliberately puts every decision in pure
      code: runtime argv construction, JSON output parsing, limit validation, capability
      reconciliation, path mapping, placement resolution and settings migration are all
      argument-driven functions in `micold-core` with no process spawn, tested Red-Green-Refactor.
      The impure remainder is one `exec` shim and the client's render glue, which fall under the
      constitution's named GUI/process-spawn exception and are covered by `quickstart.md`.
- [x] **II. Multi-Session Support**: PASS. No new per-session state and no change to session
      identity, persistence, or isolation — the sessions already shared one daemon process, and they
      now share one daemon process that happens to be in a container. The sandbox boundary is
      *additional* containment around the whole set, never a substitute for the existing per-session
      guarantees.
- [x] **III. Worktree Integration**: PASS **conditional on R2**. Worktrees live under
      `<project>/.claude/worktrees/`, inside the mounted directory, so the lifecycle is unchanged —
      *provided the project is mounted at its own absolute path*, because git records absolute paths
      in `.git/worktrees/<name>/gitdir` and in each worktree's `.git` file, and both the client and
      the daemon run git (`micold-core/src/git.rs` is used from
      `micold-client/src/shell/workspace.rs` **and** `micold-daemon/src/server.rs`). Identical-path
      mounting is free on Linux and macOS and impossible on Windows; see Complexity Tracking.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing leaves the device. The container
      is local; the daemon's state stays on the local filesystem in a local volume. FR-024a's
      offline image import is what keeps "works fully offline" true rather than nearly true — a
      registry-only design would have failed this gate, which is why the clarification asked.
      Credential sharing is opt-in and default-off (FR-004a), matching "nothing leaves the device
      without explicit opt-in".
- [x] **V. Rust + iced Stack**: PASS. No new GUI framework. Placement, runtime kind and limit units
      are modelled as enums and newtypes so an unsupported limit or an unresolvable placement is a
      type-level fact, not a runtime string comparison.
- [x] **VI. Cross-Platform Parity**: **PASS — resolved by R2 after Phase 1.** Identical-path
      mounting still cannot hold on Windows, but the resolution is no longer path translation: the
      client already routes every git call through one injected capability
      (`shell/capabilities.rs:64`, `git: Arc<dyn Git + Send + Sync>`), so Windows gets a
      daemon-backed `Git` implementation and the mount path stops mattering. That is also the only
      answer that survives the remote placement FR-003a promises, since a remote daemon leaves no
      host filesystem to run git against at any path. Identical-path mounting ships for the local
      sandbox because it is free on Linux and macOS. CI covers all three platforms for the adapter
      layer via the fake runtime; real-runtime coverage is Linux-only, which is a *test-depth*
      asymmetry, not a behaviour one.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/settings.md` is restructured for
      the sectioned view and a new `docs/user-guide/sandboxed-daemon.md` covers enabling, limits,
      the credential opt-ins, offline import and the failure catalogue. `docs/daemon.md` gains the
      placement model. Ships in the same change.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The Settings view reuses
      `NavigationDrawer` (already the sidebar's rail-and-panel widget), `Select`, `TextField`,
      `Checkbox`, `StageProgress` (image acquisition), `ConnectionBanner` (the persistent
      unsandboxed-state indicator FR-035b needs) and `Accordion`. One genuinely new primitive is
      expected — a **settings section list** — and it is built in `ui/material/` with the mandated
      chainable-builder-into-`Element` API, not privately in the feature.

## Project Structure

### Documentation (this feature)

```text
specs/027-sandboxed-daemon-runtime/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── container-runtime.md      # The runtime abstraction's contract
│   ├── sandbox-settings-schema.md# Persisted settings, v3 → v4
│   └── protocol-delta.md         # Handshake auth + build fingerprint, v5 → v6
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks output — NOT created here
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── sandbox/
│   ├── mod.rs           # SandboxProfile, ResourceBudget, NetworkPosture, CredentialShare
│   ├── placement.rs     # Placement enum + resolution; generalises endpoint.rs
│   ├── runtime.rs       # ContainerRuntime trait, RuntimeKind, RuntimeCapabilities, probe results
│   ├── argv.rs          # PURE: profile + capabilities -> argv. The heart of the test surface.
│   ├── parse.rs         # PURE: `--format {{json .}}` output -> typed inspect/probe results
│   ├── dialect/
│   │   ├── docker.rs    # Docker's argv dialect + capability set
│   │   └── podman.rs    # Podman's argv dialect (rootless, --userns=keep-id)
│   ├── exec.rs          # IMPURE: the one process-spawn shim the trait is implemented over
│   ├── image.rs         # Reference parsing, moving-tag detection, import/build/pull decisions
│   └── pathmap.rs       # Host <-> sandbox path identity (identity on Linux/macOS; R2 on Windows)
├── protocol/
│   ├── auth.rs          # Shared-secret token: generate, mount, present, verify
│   └── version.rs       # PROTOCOL_VERSION 5 -> 6; build fingerprint constant
├── endpoint.rs          # Gains a TCP-loopback endpoint alongside socket/pipe
├── connect.rs           # connect_or_spawn -> connect_or_start(placement)
└── settings.rs          # Settings v3 -> v4, sandbox profile embedded

crates/micold-daemon/src/
├── server.rs            # Accepts the loopback listener + token verification
└── main.rs              # Binds per placement; in-container mode reads its mounted secret

crates/micold-client/src/
├── features/
│   ├── settings.rs      # SettingsDraft grows sections; validation moves in beside the type
│   └── sandbox.rs       # Sandbox lifecycle state: probing, acquiring, starting, degraded
├── ui/
│   ├── settings_view.rs # NEW full-surface view (replaces settings_form.rs's modal)
│   ├── settings/        # One module per section: appearance, terminal, environment, daemon
│   └── material/
│       └── section_list.rs  # NEW shared primitive, builder API
└── shell/
    └── sandbox.rs       # Off-thread runtime calls, progress, failure -> Message

packaging/sandbox/
├── Containerfile        # The published image: daemon + shell + git + AI CLI
└── README.md            # Build, publish, offline export/import

mise.toml                # [tasks.image] — build a :dev image from the working tree (FR-024c)
docs/user-guide/
├── settings.md          # Restructured for sections
└── sandboxed-daemon.md  # NEW
```

**Structure Decision**: The existing three-crate split is kept and leaned on hard. Everything that
decides anything lands in `micold-core` (render-free, iced-free, fully testable — the same property
that let feature 010 test auto-spawn headlessly); `micold-daemon` only learns to be bound by
something other than itself; `micold-client` gains a view and a lifecycle module. No fourth crate:
the sandbox logic has no consumer outside these three and a new crate would buy nothing but a
manifest.

## Phase 0 → research.md

Ten questions, of which three are load-bearing enough to block design:

| # | Question | Blocking |
|---|---|---|
| R1 | How does a host client talk to a containerised daemon on all three platforms? | **Yes** |
| R2 | Can projects be mounted at identical absolute paths, and what happens on Windows? | **Yes** |
| R5 | Can a writable-storage limit be enforced portably? | **Yes** |
| R3 | How do files created in the container end up owned by the host user? | No |
| R4 | How is "network off" expressed without cutting the control channel? | No |
| R6 | How does the sandbox honour the existing session-survival opt-in on all three platforms? | No |
| R7 | CLI-and-parse vs. an API client for the runtime abstraction | No |
| R8 | How is a stale *development* image detected, given PACKAGE_VERSION does not move between rebuilds? | No |
| R9 | What happens when the set of registered projects changes while the sandbox runs? | No |
| R10 | Which limits does each runtime actually support, and how is that surfaced? | No |

See [research.md](./research.md) for decisions, rationale and rejected alternatives.

## Phase 1 → design artifacts

- [data-model.md](./data-model.md) — the seven entities from the spec as concrete Rust shapes, with
  validation rules traced to FR numbers and the sandbox lifecycle state machine.
- [contracts/container-runtime.md](./contracts/container-runtime.md) — the `ContainerRuntime`
  contract every dialect must satisfy, including the capability probe, and the conformance suite a
  new runtime must pass to claim support (this is what makes SC-009 measurable).
- [contracts/sandbox-settings-schema.md](./contracts/sandbox-settings-schema.md) — the durable
  on-disk shape, v3 → v4, following the existing settings-schema contract's missing-field-defaults
  rule.
- [contracts/protocol-delta.md](./contracts/protocol-delta.md) — protocol 5 → 6: the authentication
  token and the build fingerprint, with the refusal reasons FR-023/FR-024d require.
- [quickstart.md](./quickstart.md) — Part A automated gates, Part B the manual visual/behavioural
  pass (runnable with the repo's `visual-pass` skill for the view work).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| **Principle VI, resolved after Phase 1**: identical-path mounting cannot hold on Windows, so Windows routes git through the daemon rather than mounting at a matching path | Both the client and the daemon run git, and git stores **absolute** paths in worktree metadata. If the container sees the project at a different path than the host does, `git worktree list` disagrees between them and worktrees created in one are broken in the other — which breaks Principle III, not just tidiness. On Linux and macOS mounting `/home/u/p` at `/home/u/p` costs nothing and removes the problem entirely. On Windows `C:\Users\u\p` has no Linux equivalent, so the mapping is unavoidable | **Windows containers** would preserve the path but require a Windows base image, a Windows-built daemon, and Docker Desktop switched out of Linux-container mode (mutually exclusive with every other container the user runs) — a far larger deviation. **`git worktree repair` on every switch** was rejected because the host and container would each repair the metadata away from the other, turning a static mismatch into a fight. **Dropping Windows support** violates VI outright. **Path translation** was the assumed answer until R2 found the client already funnels git through one injected capability, making a daemon-backed `Git` impl cheaper than a translation layer — and the only option that also works for the remote placement |
| **A writable-storage limit that is not uniformly enforceable** (FR-015 names it as a MUST) | The spec requires the user be able to bound writable storage. Docker's `--storage-opt size=` works only on specific storage-driver/filesystem combinations (overlay2 needs xfs with project quotas); podman differs again | Rather than silently ignoring the setting — the exact "silent drift" the codebase's endpoint module rejects — the runtime declares its capabilities and the view shows unsupported limits as unavailable **with the reason**. Honest, testable, and it generalises to future runtimes. Recorded here because it does soften FR-015 from "always enforced" to "enforced where the selected runtime can, and visibly unavailable where it cannot" |
| **Protocol version bump (5 → 6) plus a new authentication step** on a transport that previously needed none | The current design authenticates by filesystem permission — a `0700` directory owning a Unix socket (`endpoint.rs`, FR-030). Loopback TCP has no such property: any local process can connect. Moving to TCP without a token would be a security **regression** shipped inside a security feature | Keeping Unix sockets was the first choice and fails R1: bind-mounted Unix sockets do not work through Docker Desktop's file sharing on macOS or Windows, so socket-only means Linux-only, violating VI. Per-runtime transports (socket on Linux, TCP elsewhere) were rejected as two transports to test forever, on the platform matrix where testing is already thinnest |

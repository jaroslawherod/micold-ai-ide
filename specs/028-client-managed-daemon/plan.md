# Implementation Plan: Client-Managed Session Service Lifecycle

**Branch**: `feat/the-daemon-should-not-longer-be-a-system-service` | **Date**: 2026-08-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/028-client-managed-daemon/spec.md`

## Summary

Make the application the only thing that starts the session service, and make the service end
itself when nobody is using it. Three strands:

1. **Remove the system-service path.** Stop shipping the systemd user units, delete the socket
   activation the daemon adopts them through, delete the Linux logout-survival opt-in that enabled
   them, and have the client un-enable, once, whatever a previous release left enabled.
2. **Add the idle rule.** Count connections where the daemon already registers and deregisters
   them; when the count has been zero for 30 minutes of suspend-inclusive monotonic time, unwind —
   persisting live sessions as interrupted-resumable, killing their process trees, releasing the
   endpoint last. The predicate this replaces (`may_exit`) exists and is tested but has never had a
   call site, so the rule is net-new behaviour landing in a tested pure function.
3. **Carry both into the sandbox.** The daemon is PID 1 in the container, so the same unwind stops
   it — with one measured exception that needs the user's approval (Complexity Tracking below).

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: tokio, interprocess, tokio-util (existing); `libc` / `windows-sys` for the
suspend-inclusive clock — both already `cfg`-scoped dependencies of `micold-core`. **Removed**:
`listenfd` from `micold-daemon`. No dependency is added.

**Storage**: unchanged — the local catalog and settings. This feature persists nothing new; it hands
off to the existing `SessionLifecycle::InterruptedResumable` path.

**Testing**: `cargo test --workspace` (`mise run test`), plus the opt-in `sandbox-real-runtime`
feature for the Docker-backed checks, plus `quickstart.md` §B for the parts that need real time.

**Target Platform**: Linux, macOS, Windows desktops; the session service additionally runs inside a
Docker container (feature 027's sandboxed placement).

**Project Type**: Desktop application — three-crate Rust workspace (`micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**: the idle evaluation is a 30-second tick over two integers — no measurable
cost. Cold start after an idle stop stays inside the existing 3-second budget (SC-006), which it
does by construction: it *is* the existing cold start.

**Constraints**: the stop must overshoot 30 minutes by no more than a minute (SC-004), and must
leave zero residue (SC-005). Both fall out of the tick interval and the unwind order.

**Scale/Scope**: one new daemon module, one new core module, three deletions (socket activation,
logout survival, unit packaging), one client startup migration, one creation-time sandbox flag
(R2a), and the docs that describe them.

## Constitution Check

*GATE: passed before Phase 0, re-checked after Phase 1 design. Result: **PASS**, one deviation
recorded below.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The rule (G2), the presence counter (G1) and the clock
      (G3) are pure and land test-first. The shutdown order, the teardown residue and the
      connect/expiry race get failing integration tests before the code. The two pieces relying on
      the GUI exception are the menu-item deletion and the client's one-shot migration call site —
      and the migration's *decision* (is anything enabled, what to run, swallow failures) lives in a
      tested render-free module, with only the invocation in glue.
- [x] **II. Multi-Session Support**: Sessions stay independently addressable and persisted. The idle
      stop ends them all at once, but through the existing per-session teardown, and each returns as
      its own interrupted-resumable entry. No session state crosses into another.
- [x] **III. Worktree Integration**: Untouched. No worktree is created, moved or removed by this
      feature; sessions resume against the same worktree or project root as before.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Everything is local and offline. The feature
      *removes* a platform integration rather than adding one; nothing leaves the device.
- [x] **V. Rust + iced Stack**: Rust only, no new framework. `Presence` makes "a zero count with no
      armed deadline" unrepresentable by construction (one guarded transition, not two fields a
      caller keeps in step), and `Uptime` cannot be compared with a wall-clock instant.
- [x] **VI. Cross-Platform Parity**: The rule is platform-agnostic; only the clock reading has three
      implementations, behind one function in `micold-core`, and the daemon never names an OS. The
      migration in strand 1 is Linux-only because the artefact it removes only ever existed there —
      a removal, not a behavioural difference. CI covers all three.
- [x] **VII. Documentation First-Class**: `docs/daemon.md` (the systemctl instruction and the
      lifetime claim), `docs/user-guide/settings.md` and `docs/user-guide/sandboxed-daemon.md` (the
      opt-in's new meaning) ship in the same change, per FR-025 and packaging contract §4.
- [x] **VIII. Reusable UI Component Foundation**: The only UI change is the removal of a menu item
      and a copy change on an existing toggle. No widget is added or forked.

## Project Structure

### Documentation (this feature)

```text
specs/028-client-managed-daemon/
├── plan.md              # This file
├── spec.md              # Feature specification (with Clarifications)
├── research.md          # Phase 0 — R1..R8, including the measured R2
├── data-model.md        # Phase 1 — G1..G5, L4, placement mapping
├── quickstart.md        # Phase 1 — Part A automated, Part B recorded by hand
├── contracts/
│   ├── lifecycle.md     # Start / live / stop / race / placement / diagnostics
│   └── packaging.md     # What an install, upgrade and uninstall may leave behind
├── checklists/
│   └── requirements.md  # Spec quality checklist (passing)
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── clock.rs                     # NEW — suspend-inclusive monotonic reading (R3, G3)
├── logout_survival.rs           # TRIMMED — host-process enable path deleted, sandbox arm kept
├── sandbox/argv.rs              # restart_policy: amended meaning (R2); passes MICOLD_IDLE_STOP
│                                #   and the test window override at creation (R2a)
├── sandbox/lifecycle.rs         # survive_logout joins what makes a sandbox stale (R2a, FR-022a)
└── connect.rs                   # unchanged — already the only start path

crates/micold-daemon/src/
├── idle.rs                      # NEW — Presence, the rule, the tick, StopReason (G1, G2, G4)
├── lifecycle.rs                 # DELETED — may_exit never had a call site; Presence supersedes it
├── server.rs                    # register/deregister feed Presence; systemd_listener + serve_unix
│                                #   deleted; run() returns through the ordered unwind (G5)
└── state.rs                     # the one place the counters move

crates/micold-client/src/
├── shell/service_control.rs     # logout-survival handlers deleted
├── shell/legacy_units.rs        # NEW — the one-shot un-enable migration (R7)
├── ui/settings/daemon.rs        # copy change on the sandbox opt-in
└── main.rs / app.rs             # menu item removed; migration invoked before connect

packaging/
├── micold-daemon.service        # DELETED
└── micold-daemon.socket         # DELETED

crates/micold-client/Cargo.toml  # both systemd assets removed from [package.metadata.deb]
crates/micold-daemon/Cargo.toml  # listenfd removed

docs/
├── daemon.md                    # the systemctl instruction, the lifetime claim, the idle window
└── user-guide/{settings,sandboxed-daemon,worktrees-and-sessions}.md
```

**Structure Decision**: The existing three-crate split already puts each piece where it belongs and
this feature does not move that line. Platform-specific reading goes to `micold-core` (Principle
VI); the rule and the unwind go to `micold-daemon`, which is the only process that can act on them;
the migration goes to `micold-client`, because — as feature 010 recorded — a per-user service
manager is reachable only from the user's own session, which is precisely why the installer cannot
do it.

## Complexity Tracking

> One deviation, forced by measurement. It amended the spec, and the user **approved it on
> 2026-08-27**: the opt-in wins. `spec.md` now carries it as FR-022/FR-022a, US4 scenarios 5–6 and a
> third Clarifications entry.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| The idle stop and the sandbox's keep-it-running opt-in cannot both hold. The idle rule is suppressed while that opt-in is on (spec FR-022). | Measured on Docker 29.5.1: a container under `--restart unless-stopped` that exits **cleanly** is restarted — three times in seven seconds. The opt-in exists to deliver restart-at-boot, and only `always`/`unless-stopped` do that; `on-failure` ignores a clean exit but, per Docker's documentation, "doesn't restart the container if the daemon restarts", so it cannot serve the opt-in. Nothing inside the container may mark itself stopped: feature 027's FR-005 forbids mounting the runtime socket, and there is no host-side process left running to issue `docker stop`. | **`on-failure` for the opt-in** — forfeits the reboot survival the opt-in is for. **A host-side one-shot timer at quit** (`systemd-run --on-active`, `launchd`, Task Scheduler) — three platform implementations, dies with a client that crashed instead of quitting, and reinstates the platform-registered job this feature exists to remove. **A hibernating PID 1** — FR-019 forbids a container left running with no service in it, and it is the orphan shape 027 already fought. **Letting it restart-loop** — worse than not stopping: constant churn at Docker's capped backoff, forever. |

**The user-facing shape of the deviation**: the toggle stops meaning "survive logout" and starts
meaning "keep the sandbox running — it survives logout and reboot, and is not stopped when idle".
One sentence at the point of choosing (FR-004b of feature 027 already requires that shape), and the
default is off, so the idle rule applies unless the user has explicitly asked otherwise.

**The alternative the user declined** was deleting the sandbox opt-in as well — matching what strand
1 does to the host-process one — and making the idle rule unconditional. That is a smaller product,
not a smaller change; the opt-in stays.

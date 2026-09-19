# Implementation Plan: Run a session on the Pi coding agent

**Branch**: `feat/support-of-pi-agent` | **Date**: 2026-09-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-pi-cli-provider/spec.md`

## Summary

Add Pi — the `pi` coding agent — as the third AI CLI behind feature 026's provider seam, and treat
that as the seam's first real test: a CLI the seam was not designed around.

Research (Phase 0) settled every mechanism the spec left open and found the seam holds in all but one
place. Pi accepts `--session-id <id>`, *"creating it if missing"*, for both fresh and resumed
launches, so a session's identity is carried by where the conversation is stored in Pi's own per-cwd
store and FR-005b's correspondence-store fallback is never reached. Conversation storage, name
extraction, discovery, the durable close marker and the `PATH` availability check all fit the
existing trait methods unchanged. *(BUG-001: the availability check fits the seam, but the `PATH` it
walked was the wrong one — see [Bugfix increment: BUG-001](#bugfix-increment-bug-001).)*

The one place it does not fit is activity. Pi reports busy/idle only to code loaded into it, and that
code has to be supplied at launch and its log tailed — two spawn-time obligations
`ActivitySource::EventLog` does not imply and that must not fire for Copilot. So the **seam is what
changes** (FR-020): one new `ActivitySource` variant, `Extension { log: PathBuf }`, available to
every provider, with the daemon reusing `EventLogTail` unchanged behind it. Nothing above the seam
learns that Pi exists.

## Technical Context

**Language/Version**: Rust, toolchain pinned to `stable` by `rust-toolchain.toml`. One non-Rust
artifact: the ~100-line activity component, a TypeScript module executed by `jiti`, which ships as a
dependency of `pi` itself (see Complexity Tracking).

**Primary Dependencies**: no new Rust crates. Existing: `iced` (client), `notify` (daemon watch,
already used by `EventLogTail`), `uuid`, `serde`. External runtime dependency: the `pi` CLI —
`@earendil-works/pi-coding-agent`, pinned at **0.85.1** in the published image, with **no version
floor gated at runtime** (FR-003a).

**Storage**: local filesystem only. Pi's own store, read best-effort and never relocated:
`$PI_CODING_AGENT_DIR` or `~/.pi/agent`, with conversations at
`sessions/--<encoded cwd>--/<timestamp>_<session-id>.jsonl`. Two application-owned artifacts inside
it: an empty `<session-id>.archived` marker (the existing pattern) and a per-session activity log at
`micold-activity/<session-id>.jsonl`, beside `sessions/` and never inside it.

**Testing**: `cargo test` via the repo's mise tasks — `mise run test-core` for the seam, `mise run
test` for the workspace, `mise run image` + `mise run test-sandbox` for the image-side obligation
(`crates/micold-daemon/tests/sandbox_real_ai_cli.rs`, feature `sandbox-real-runtime`, iterates
`AiCli::ALL` and so covers `pi` the moment the variant exists).

**Target Platform**: Linux, macOS and Windows desktop. `pi` declares no `os`/`cpu` restriction and
resolves its base directory home-relative on all three, so no `cfg` arm is added anywhere.

**Performance Goals**: badge reflects a reported turn boundary within 1 s (SC-005, inherited —
`EventLogTail` already meets it). Project open cost proportional to the number of locations, not to
the number or the **length** of conversations in them (FR-015, SC-006b). Zero scheduled work for an
idle or merely-discovered session (FR-014).

**Constraints**: fully offline — `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1`, `PI_TELEMETRY=0` are set
per launch and in the image (Principle IV, research R9). No polling timer. No user configuration of
Pi's is modified (FR-007). No concrete provider type named outside `provider.rs`
(`micold-client/tests/no_concrete_implementations.rs`). No second enumeration of the supported CLIs
(FR-021).

**Scale/Scope**: three providers where there were two. One new provider impl, one new
`ActivitySource` variant, one new daemon event mapping, one spawn-preparation branch, one
application-wide settings switch, one image pin, and the user-guide/README additions FR-022 requires.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Confirm the plan satisfies each principle (mark each PASS, or record a justified
deviation in Complexity Tracking):

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every Rust unit lands behind a failing test first:
  `crates/micold-core/tests/pi_provider.rs` mirrors `copilot_provider.rs` for the whole trait
  surface against a fixture store; `activity.rs`'s `pi_event` mapping is a pure-function table test;
  the image obligation is red the moment `AiCli::Pi` exists, because `sandbox_real_ai_cli.rs`
  iterates `AiCli::ALL`. The activity component itself is the constitution's narrow process-spawn
  wiring case — it has no decision logic beyond "emit this line for this event" — and is validated
  by the recorded procedure in `quickstart.md` §C plus the end-to-end daemon test that asserts the
  badge moves for a real `pi` launch.
- [x] **II. Multi-Session Support**: PASS. Everything added is per-session: the id is the
  application's own session id, the activity log path is derived from it, the archived marker is
  named by it. Pi sessions run concurrently with the other CLIs' with no shared state (FR-006), and
  the application refuses to run two of its own sessions on one conversation (FR-006a).
- [x] **III. Worktree Integration**: PASS. A Pi session's cwd is its worktree (or the sanctioned
  project-root "Default"), exactly as the other two CLIs'. Pi's per-cwd store keys off that path, so
  worktree scoping is preserved by Pi's own layout with no manual git steps.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS, and this is where the plan adds something
  rather than inheriting it. Pi's documented startup behaviour includes an update check, a `pi.dev`
  version request and install telemetry; the launch environment disables all three (research R9),
  by the same judgement that made `--no-remote` deliberate for Copilot. Conversations stay on disk
  in Pi's own store; nothing is uploaded; the feature works offline.
- [x] **V. Rust + iced Stack**: PASS for the application, with one recorded deviation for the
  component (Complexity Tracking). The invalid-state work is real rather than nominal: the new
  `ActivitySource::Extension { log }` variant is what makes "this provider needs a component
  injected at spawn" a fact the type system carries, instead of a `match` on which CLI a session
  runs — which is precisely what FR-019/FR-020 forbid.
- [x] **VI. Cross-Platform Parity**: PASS. No `cfg` arm is added. `pi` is a JS bundle with no
  platform restriction; its base directory is home-relative on all three platforms, so
  `config_dir()` has the same single shape `CopilotProvider` already proved out on Windows; the
  `PATH` lookup goes through the existing `resolves_on_path`, which answers the `.exe`/`.cmd`
  question via `PATHEXT` with no branch. CI covers all three. *(BUG-001: the lookup keeps that
  shape but takes the `PATH` to walk as an argument instead of reading the process's own.)*
- [x] **VII. Documentation First-Class**: PASS. FR-022 is part of this change, not a follow-up:
  `docs/user-guide/settings.md` and `docs/user-guide/worktrees-and-sessions.md` gain Pi, and three
  things are stated outright — that the conversation lives in Pi's store and is resumable outside
  the application, that a Pi session loads a component of this application into Pi and how to
  decline it, and that the in-use warning is advisory. `README.md` lists Pi as supported.
- [x] **VIII. Reusable UI Component Foundation**: PASS. No new widget. The FR-012e switch is one more
  row in the existing Settings surface, built from the shared primitives the default-CLI setting
  already uses; the sidebar label, the activity badge and the terminal bar's CLI name are the
  existing components rendering a third value.

**Post-Phase-1 re-evaluation**: unchanged — all eight still PASS, with the same single deviation.
Phase 1 introduced no new entity, contract or procedure that touches a principle differently: the
data model adds one enum variant and one `ActivitySource` variant, the contract is a profile of the
existing seam, and the quickstart is a validation procedure rather than production code.

## Project Structure

### Documentation (this feature)

```text
specs/029-pi-cli-provider/
├── plan.md              # This file (/speckit-plan command output)
├── spec.md              # Feature specification
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/
│   └── pi-cli.md        # Phase 1 output — the Pi profile of the provider seam
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/session.rs                       # AiCli::Pi; ALL becomes [AiCli; 3]
├── src/provider.rs                      # PiProvider impl; ActivitySource::Extension { log };
│                                        #   AiCli::provider() gains one arm
├── src/settings.rs                      # the FR-012e application-wide switch
└── tests/
    ├── pi_provider.rs                   # NEW — the whole trait surface, fixture-backed
    ├── ai_cli_provider_seam.rs          # extended: the new variant round-trips the seam
    └── settings_ai_cli.rs               # extended: third value, and the new switch

crates/micold-daemon/
├── assets/pi-activity.ts                # NEW — the activity component, FR-012b's "one place"
├── src/activity.rs                      # pi_event(), beside copilot_event()
├── src/state.rs                         # spawn prep keyed on ActivitySource::Extension
├── src/event_log.rs                     # unchanged — reused as-is
└── tests/
    ├── activity_pipeline.rs             # extended: the Pi mapping end to end
    └── sandbox_real_ai_cli.rs           # unchanged — covers pi via AiCli::ALL

crates/micold-client/
├── src/features/settings.rs             # the switch's row and message
└── tests/                               # extended: menus/labels carry the third CLI

packaging/sandbox/Containerfile          # ARG PI_CLI_VERSION + the existing npm layer

docs/
├── README.md                            # Pi listed as supported
└── user-guide/
    ├── settings.md                      # selecting Pi; the activity-component switch
    └── worktrees-and-sessions.md        # Pi's store, resumability outside the app, the
                                         #   advisory in-use warning
```

**Structure Decision**: the existing three-crate Rust workspace, unchanged. `micold-core` holds the
seam and therefore every Pi-specific detail; `micold-daemon` holds the spawn and the activity tail;
`micold-client` gains one settings row and no knowledge of Pi. The only new source file outside a
test is `crates/micold-daemon/assets/pi-activity.ts`, placed in the daemon because FR-012a requires
the component to travel with the session service rather than with the image. No new crate, no new
module tree, and no directory outside the ones the two existing providers already use — which is
itself the evidence for the spec's second purpose.

## Bugfix increment: BUG-001

**Defect**: `ClientMsg::AiCliAvailabilityRequest` is answered by `provider::available_here()`,
which walks the daemon process's own `PATH` (`resolves_on_path` reads `std::env::var_os("PATH")`).
A desktop-launched daemon inherits the login session's `PATH`, which lacks version-manager
directories. Sessions are spawned with `SharedState::env_include_vars_for(cwd)` applied, which has
them. So a `pi` installed with `npm install -g` under mise or nvm runs fine in a session but is never
offered. FR-003b now says availability follows the spawn environment.

**Design** (every provider; nothing names Pi — FR-019, FR-021):

1. **Core**: `resolves_on_path` takes the `PATH` value to walk (`&OsStr`) rather than reading the
   process environment, and `available_here()` gains a sibling that takes that value, for example
   `available_in(path: &OsStr) -> Vec<AiCli>`. `PATHEXT` handling is unchanged, so no `cfg` arm is
   added (Principle VI). The provider trait's `is_available` takes the same argument; where a
   provider's check is a pure command lookup it forwards it. This is a seam change available to
   every provider, recorded as such (FR-020).
2. **Protocol**: `AiCliAvailabilityRequest` gains `cwd: Option<PathBuf>` — the directory the choice
   is being made for. Wire-visible, so `PROTOCOL_VERSION` goes from 13 to 14 and the schema hash is
   regenerated.
3. **Daemon**: the handler resolves the spawn environment for `cwd.unwrap_or(home)` through the
   existing `env_include_vars_for`, so it shares the per-directory `env_include_cache` with spawns
   and is invalidated by the same `SettingsSet` and `WorktreeDelete` paths. It takes the `PATH`
   from that list, or the process's own when env-include is off or supplies none, and answers from
   `available_in`. Resolution can block up to the env-include timeout, and the state lock is never
   held across it. So the handler runs it on `spawn_blocking` and sends `AiCliAvailability` when
   it completes, leaving the connection loop free (FR-003b's last sentence). `start_session`'s
   launch gate (`if !provider.is_available()`) checks against the same spawn-environment `PATH`
   for the session's own directory; otherwise a CLI offered by the handler is refused at launch.
4. **Client**: `ask_cli_availability` passes the directory when the per-session override or the
   missing-CLI list is opened for a project or worktree, and `None` from Settings.
5. **Docs**: `docs/user-guide/settings.md` states that which CLIs are offered follows the session
   environment, and what to do when a version-manager install is not offered: keep env-include on,
   or put the CLI on the login `PATH`.

**Cost** (SC-006a as amended): no CLI is spawned and no version is read. The one process is the
env-include resolution for that directory, which the next spawn there would run anyway, and later
requests and spawns are served from the cache. With env-include off, the cost is one `PATH` walk
per provider, as before.

**Sandboxed placement**: unchanged in shape. The daemon runs inside the sandbox, so the same code
resolves the same environment its sessions get there.

**Bugfix**: 2026-09-18 — BUG-001 Updated from bugfix patch.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| A TypeScript source file (`crates/micold-daemon/assets/pi-activity.ts`) in a Rust + iced project (Principle V) | FR-012 requires the busy/idle signal to be **reported by Pi**, and Pi reports it only to an extension loaded into its own process. Pi's extension API accepts `.ts`/`.js` and nothing else; there is no Rust entry point into it. | Inferring activity from Pi's session file is forbidden by FR-012 outright (a session thinking without writing would read as idle, which is the failure the requirement exists to prevent). Shipping Pi with no badge was rejected in the spec's own iteration 2. Pi's RPC mode would replace the PTY session model rather than observe it. The deviation is bounded: one file, ~100 lines, no decision logic, no dependency of its own — `jiti` already ships inside `pi` — and FR-012b caps it permanently at "report activity, nothing else, reviewable in one place". |

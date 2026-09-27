# Implementation Plan: The start affordance answers for its own directory

**Branch**: `fix/a-project-local-cli-stays-unreachable-in-that-project` | **Date**: 2026-09-27 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/033-directory-aware-start-affordance/spec.md`

## Summary

Each sidebar row's start affordance (chevron presence and primary press) reads the AI-CLI
availability answer **for that row's own directory**. Today it reads one window-wide answer, which
Settings and reconnects overwrite with the home directory's answer.

The design is a per-directory answer cache in the client, filled only on the spec's named events.
It reuses the existing per-directory request `ClientMsg::AiCliAvailabilityRequest { req, cwd }`,
which the start list already sends (research R1). The protocol does not change, and neither does
the daemon.

1. **One render-free type**, `AvailabilityAnswers` in `features/session.rs`, replaces the
   window-wide `available_providers`. It holds a home answer plus at most one answer per directory,
   and files each reply under the directory its request named, newest request per directory winning
   (R4, FR-009). This fixes the second half of the defect too: today's single
   `cli_availability_asked` drops answers for *other* directories.
2. **Readers take a directory.** `start_affordance_offers_a_choice`, `start_intent`,
   `offered_providers` and `default_ai_cli_is_available` read the row's answer, falling back to home
   while it is pending (FR-005). Settings reads home only (FR-008).
3. **Asks happen on events, never on draw.** A single idempotent `sync_cli_availability` derives the
   wanted directories from state (active project root plus startable worktrees, R3). It prunes
   answers for directories that are gone and asks for new ones. It runs after project
   open/reopen/switch/forget, worktree-list changes and connect. Environment-include changes and
   start-list opens refresh (R6, R7). A source tripwire pins the closed list of askers (R10,
   SC-003).

## Technical Context

**Language/Version**: Rust, toolchain pinned by `rust-toolchain.toml`.

**Primary Dependencies**: no new crates. `iced` (client), `std::collections::{HashMap, BTreeSet}`.

**Storage**: none. Answers are in memory only and never persisted (FR-003, 026 R11).

**Testing**: `mise run gate` (fmt, clippy, `cargo test --workspace`, script tests). Render-free state
tests in `crates/micold-client/tests/`, shell event tests in `crates/micold-client/src/main_tests.rs`
(which has an outbox-capturing harness: `connected_with_outbox`, `availability_asked_for`), and a
source tripwire. §B runs through the `visual-pass` skill.

**Target Platform**: Linux, macOS, Windows desktop. No `cfg` arm is added.

**Project Type**: desktop application (client + session service), Cargo workspace.

**Performance Goals**: a redraw performs zero availability requests and zero environment
resolutions, and its added cost is one path join plus one hash lookup per row (FR-006). Opening a
project sends one request per distinct startable directory (SC-004).

**Constraints**: no timer, no `Subscription`, no polling (SC-003). No protocol or daemon change
(spec Assumptions). Principle II: answers keyed by directory never overwrite each other.

**Scale/Scope**: about six client source files touched (`features/session.rs`,
`shell/daemon_sync.rs`, `shell/workspace.rs`, `shell/persist.rs`, `main.rs`, `ui/sidebar.rs`,
`ui/mod.rs`). One new state test file, one new tripwire test, updates to existing availability tests
and guards, and two user-guide pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Each behaviour lands behind a failing test.
  `AvailabilityAnswers` is pure and covered by `tests/directory_availability.rs` (filing,
  fallback, pruning, staleness). The readers are covered in `tests/features_session.rs`. The event →
  request wiring is covered by `main_tests.rs` through the existing outbox harness. "Only on these
  events" is covered by the tripwire (R10). The sidebar's per-row chevron is a render-free decision
  read by the view, so no GUI-only logic is added.
- [x] **II. Multi-Session Support**: PASS, and this is the principle the defect breaks. Answers are
  keyed by directory, one answer never replaces another's (FR-002), and replies are filed by the
  request that asked (R4). Two clients each hold their own cache and ask the same daemon, which
  answers per directory, so the same directory gets the same answer.
- [x] **III. Worktree Integration**: PASS. A worktree row is answered for its own directory through
  `SessionLocation::cwd`, the function the spawn uses (FR-007, R2). The Default row is answered for
  the project root.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is persisted and nothing leaves the
  machine. The requests go to the local session service (or its local container) exactly as today.
- [x] **V. Rust + iced Stack**: PASS. `AvailabilityKey { Home, Dir(PathBuf) }` makes "home" a
  distinct case rather than a `None` directory. Filing is by construction: an answer can only land
  under the key its request recorded.
- [x] **VI. Cross-Platform Parity**: PASS. Path keys come from `SessionLocation::cwd`, which is
  platform-neutral. The presence check and the environment-include resolution are the existing
  per-platform ones, unchanged. CI runs all three platforms.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` ("If only
  one CLI is installed, the chevron is not there") and `docs/user-guide/settings.md` ("The per-session
  list asks about…") are updated in the milestone that ships the behaviour: each row offers what its
  own directory provides, and the list refreshes when environment-include changes.
- [x] **VIII. Reusable UI Component Foundation**: PASS. No widget changes. `SplitAction` and the
  start list render the same way and are given different inputs.

**Post-Phase-1 re-evaluation**: unchanged, all eight PASS. The data model adds one client-only
type. The contract fixes the client's asking discipline over an unchanged wire message. There is no
deviation, so Complexity Tracking is empty.

## Requirement map

| Requirement | Where | Test layer |
|---|---|---|
| FR-001 row reads its own directory | `session::State` readers take a directory; `ui/sidebar.rs` passes `location_dir` (data-model) | client state test; §B 1–2 |
| FR-002 no cross-directory replacement | `AvailabilityAnswers::answered` (C2) | state test; `main_tests.rs` (Settings open, start list) |
| FR-003 at most one per directory, memory only | `AvailabilityAnswers::dirs` + `retain` | state test |
| FR-004 refresh only on named events | contract C1, R3, R6, R7 | `main_tests.rs` per event; tripwire |
| FR-005 home fallback while pending | `for_dir` fallback | state test |
| FR-006 no check on draw; one per distinct directory; eager | `sync_cli_availability` + `unasked`; tripwire forbids `ui/` askers | `main_tests.rs`; tripwire |
| FR-007 worktree answered for its own directory | `location_dir` via `SessionLocation::cwd` | `main_tests.rs` (cwd asked) |
| FR-008 Settings home; list/missing-CLI per directory | `ui/mod.rs` Settings reads `home()`; `session_start_menu_items` and `start_menu_toggled` read the list's directory | `provider_choice_surfaces.rs`, `missing_cli_is_reported_where_it_is_chosen.rs` |
| FR-009 stale same-directory answer discarded | `latest` / `in_flight` (R4) | state test |
| FR-010 primary press never substitutes | `start_intent(target, dir)`, same branches as today | `session_start_press.rs`, `unavailable_default_says_so.rs` |
| FR-011 reconnect discards and re-asks | `on_connected` → `clear` + home + sync (R5) | `main_tests.rs` |
| FR-012 closing drops answers | sync on switch/forget → `retain` | `main_tests.rs` |
| SC-003 zero timers, zero checks per redraw | tripwire (R10) + `idle_subscriptions.rs` | structural |
| SC-004 ≤ D resolutions | one request per distinct directory (R11) | `main_tests.rs` (count of requests) |
| SC-005 env-include change reflected | `SettingsChanged` diff (R6) | `main_tests.rs`; §B 7 |

## Project Structure

### Documentation (this feature)

```text
specs/033-directory-aware-start-affordance/
├── plan.md              # This file
├── spec.md
├── research.md          # Phase 0: R1–R11
├── data-model.md        # Phase 1: AvailabilityKey, AvailabilityAnswers, reader signatures
├── quickstart.md        # Phase 1: §A automated, §B real GUI
├── contracts/
│   └── availability-asks.md   # C1–C4: who asks, how answers are filed and read
├── checklists/
│   └── requirements.md
├── autopilot.md
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-client/
├── src/features/session.rs      # AvailabilityKey, AvailabilityAnswers (replaces available_providers);
│                                #   readers take a directory; wanted_availability_dirs;
│                                #   start_menu_toggled reads the list's directory
├── src/app.rs                   # State::location_dir
├── src/main.rs                  # drop App::cli_availability_asked; StartMenuOpened arm keys its ask;
│                                #   ForgetConfirmed arm runs the sync
├── src/shell/daemon_sync.rs     # ask_cli_availability(app, key) records in_flight;
│                                #   sync_cli_availability / refresh_cli_availability;
│                                #   AiCliAvailability arm files by req; on_connected clear+ask;
│                                #   SettingsChanged env-include diff; CatalogChanged sync
├── src/shell/workspace.rs       # sync after open / reopen
├── src/shell/persist.rs         # Settings(Opened) asks Home (unchanged call, new key)
├── src/shell/startup.rs         # App construction without cli_availability_asked
├── src/ui/sidebar.rs            # rows pass their directory to start_press / offers_a_choice
├── src/ui/mod.rs                # Settings reads home(); start list reads its directory
├── src/main_tests.rs            # event → request tests (extended)
└── tests/
    ├── directory_availability.rs                     # NEW: AvailabilityAnswers + readers
    ├── availability_is_asked_only_on_named_events.rs # NEW: tripwire (SC-003, FR-004)
    ├── features_session.rs                           # readers take a directory
    ├── provider_choice_surfaces.rs                   # home vs directory sources
    ├── session_start_press.rs, unavailable_default_says_so.rs,
    │   missing_cli_is_reported_where_it_is_chosen.rs # seeded through the new type
    ├── cli_availability_comes_from_the_service.rs    # vacuity spelling updated
    └── support/state_scan.rs                         # READERS vocabulary for the new readers

docs/user-guide/
├── worktrees-and-sessions.md    # the chevron reflects the row's own directory
└── settings.md                  # the Settings field answers for home; rows for their own directory
```

**Structure Decision**: the existing three-crate workspace. All changes are in `micold-client`.
`micold-core` (protocol, `SessionLocation::cwd`) and `micold-daemon` are read, not changed.

## Test strategy by layer

- **Render-free state** (`crates/micold-client/tests/`): `AvailabilityAnswers` operations and the
  readers. This is where FR-002/003/005/009/010 are proven.
- **Shell events** (`src/main_tests.rs`, outbox harness): each C1 event sends exactly the requests
  it names and no others. Answers arriving out of order land correctly. Connect clears. Covers
  FR-004/006/007/008/011/012 and SC-004/005.
- **Structural** (`tests/availability_is_asked_only_on_named_events.rs`, `idle_subscriptions.rs`):
  SC-003 and FR-006's "no check on draw".
- **Geometry gates**: none needed. The affordance's layout is unchanged. The chevron's presence per
  row is a boolean input the gates already cover in both states.
- **Quickstart §B** via `visual-pass`: US1–US3 end to end against a real service with an
  environment-include script.

## Complexity Tracking

No constitution violations to justify.

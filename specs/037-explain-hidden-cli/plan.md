# Implementation Plan: Explain Why an AI CLI Is Not Offered

**Branch**: `fix/github-issues` | **Date**: 2026-10-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/037-explain-hidden-cli/spec.md`

## Summary

An AI CLI a session would not find is hidden everywhere a CLI is chosen, and the only sentence
about it says it "isn't installed" (issue #434). That is false when the CLI is on a `PATH` only
the environment-include script adds and the script was not applied. This feature gives each
surface that names a missing CLI one reason, chosen from six states of the environment a session
in that directory gets, and the action that would change it.

Technical approach (research.md R1–R9): the session service already resolves each directory's
environment once and caches the variables. It now keeps the attempt's outcome in the same cache
cell and classifies it into a six-variant `SpawnEnv`. The availability answer carries that state
in one new field (`PROTOCOL_VERSION` 17 → 18). A new pure module `micold_core::cli_reason` writes
every sentence, and both the service (start failure, reply to an AI session) and the client
(Settings notes, missing-default message, a note in a row's CLI list) call it, so the surfaces
cannot disagree. The client stamps each answer with the key it was asked for, which gives the
directory a reason names. The row list's note is a new builder method on the shared
`material::MenuOverlay`. No script run is added, nothing is persisted, and what is offered does not
change.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise` (workspace edition as in `Cargo.toml`)

**Primary Dependencies**: iced 0.14 (client GUI), serde (wire types, already used), tokio (service
and client, already used). No new crates.

**Storage**: None added. `settings.json` and the session store are unchanged (spec Key Entities:
the reason "is not stored").

**Testing**: `cargo test` via `mise run test-core` (classification and wording) and
`mise run gate` (protocol pin, service answers and failures, client reducers and notices, menu
anatomy). Rendering is quickstart §B's visual pass (Constitution I exception).

**Target Platform**: Desktop client and session service on Linux, macOS and Windows; the service
also runs in a Linux container (027).

**Project Type**: Desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`).

**Performance Goals**: No added work on the availability path: the state is read from the cell the
`PATH` walk already fills. Zero additional script runs (SC-006).

**Constraints**: One reason per answer, from the same attempt that decided the offer (FR-012). No
search outside the session environment (FR-014). No change to what is offered or to any setting
(FR-015). The chevron rule of 026 FR-006 is unchanged (D5).

**Scale/Scope**: Three supported CLIs, six states, two placements, five surfaces. About 20 files
touched, 1 new core module, 1 new builder method on a shared component, 4 user guide pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How the plan satisfies it |
|---|---|---|
| **I. Test-First** | PASS | Every decision is in tested code written test-first: classification and all sentences in core (`crates/micold-core/tests/cli_reason.rs`), the wire pin (`tests/schema_hash.rs`, `protocol_roundtrip.rs`), the service's answers, failures and run count (`crates/micold-daemon/tests/ai_cli_availability.rs`, `session_start.rs`, `mcp_create_session.rs`), and the client's rules for each surface in render-free `features/` (`tests/missing_cli_is_reported_where_it_is_chosen.rs`, `unavailable_default_says_so.rs`, `directory_availability.rs`). The menu note's layout is held by the `menu_anatomy` gate. Only the lines that pass an already-computed string to a widget fall under the GUI-glue exception, and quickstart §B validates them. |
| **II. Multi-Session** | PASS | A reason starts, stops and changes no session (FR-015). Each directory has its own cell and each row its own answer, so two rows opened together cannot exchange reasons (Story 3 scenario 3). An answer older than the newest request for its key is dropped (033 contract C2), so two windows saving settings cannot leave a stale reason. The service's cache keeps its existing rule that an invalidation wins over a resolve in progress. |
| **III. Worktree Integration** | PASS | A row reasons from its own worktree directory's attempt. A worktree's cell is removed when the worktree is deleted, as today. No VCS operation is added. |
| **IV. Local-First** | PASS | The state comes from the local session service's own cache. No network, no remote service, nothing persisted or transmitted beyond the existing local socket. |
| **V. Rust + iced** | PASS | Rust and iced only. `SpawnEnv` is a closed enum, so a state without an action cannot be built. `Option<SpawnEnv>` separates "the service could not say" from every state. `ResolvedEnv` holds variables and state together, so one cannot be cached without the other. |
| **VI. Cross-Platform** | PASS | The classification and the sentences do not branch on the OS. No sentence names a platform's startup file (contract W2e). The service tests that source a script run on all three CI platforms with the fixtures `ai_cli_availability.rs` already has for bash and PowerShell. `mcp_create_session.rs` is `#![cfg(unix)]`, so U5's wiring is tested on unix only, and its wording on every platform by `cli_reason.rs`. No `cfg(target_os)` arm is added. |
| **VII. Documentation** | PASS | The user guide changes in the milestone that ships each behaviour (FR-017): `settings.md` (Default AI CLI note) and `sandboxed-daemon.md` with the Settings note; `worktrees-and-sessions.md` ("When a CLI isn't installed") and `agent-tools.md` with the failure and the reply; the row list in `worktrees-and-sessions.md` with the list note. `docs/daemon.md`'s protocol paragraph, which stops at version 10, is brought up to version 18. |
| **VIII. Reusable UI** | PASS | The Settings notes keep the shared `field_note` primitive. The row list's note is a builder method on the shared `material::MenuOverlay`, shown in the component showcase and covered by `menu_anatomy`. No feature-level widget styling. `menu_panel_size_with_note(items, note)` is not a component constructor: it is the measuring function beside the existing free `menu_panel_size(items)` (`ui/material/menu.rs`), which the anchor clamp calls before any overlay is built, so it keeps that function's form. |

No violations. Complexity Tracking is empty.

**Post-design re-check (after Phase 1)**: unchanged. The contracts add one core module, one wire
field, one cached field, two client fields and one builder method. No persisted field, no
dependency and no new widget type.

## Project Structure

### Documentation (this feature)

```text
specs/037-explain-hidden-cli/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R1–R9 with rejected alternatives
├── data-model.md        # Phase 1: SpawnEnv, ResolvedEnv, the changed answer types
├── quickstart.md        # Phase 1: §A automated checks, §B visual pass
├── contracts/
│   ├── availability-answer.md   # wire field, what the service answers, what the client files
│   └── reason-wording.md        # every sentence, every surface, when nothing is said
├── tdd/                 # test-list.md and cycle-log.md (speckit-tdd-plan)
├── evidence/            # visual pass screenshots
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/lib.rs                          # + pub mod cli_reason
├── src/cli_reason.rs                   # NEW: SpawnEnv, classify, Place, AttemptDir, Explanation,
│                                       #      explain, start_refusal, start_refusal_unknown, name_list,
│                                       #      the 3 label consts
├── src/protocol/messages.rs            # DaemonMsg::AiCliAvailability + env: Option<SpawnEnv>
├── src/protocol/version.rs             # PROTOCOL_VERSION 17 → 18, with its paragraph
├── tests/cli_reason.rs                 # NEW: classification table, W1–W3 over state × place
├── tests/schema_hash.rs                # the pinned version
└── tests/protocol_roundtrip.rs         # the answer with and without env

crates/micold-daemon/
├── src/state.rs                        # ResolvedEnv in EnvIncludeCell; spawn_env_for;
│                                       #   availability_in; launch gate uses start_refusal
│                                       #   (missing_cli_reason removed)
├── src/server.rs                       # the AiCliAvailabilityRequest arm sends env
├── src/mcp/tools.rs                    # create_session's refusal uses explain
├── tests/ai_cli_availability.rs        # S1–S7, no extra script run
├── tests/session_start.rs              # U4, Fresh and Resume, host and image
└── tests/mcp_create_session.rs         # U5

crates/micold-client/
├── src/features/session.rs             # CliAvailability + env, asked_for; answered() stamps it;
│                                       #   start_menu_toggled uses start_refusal; start_menu_note
├── src/features/settings.rs            # missing_cli_notice uses explain; name_list moves to core
├── src/shell/daemon_sync.rs            # the AiCliAvailability arm passes env
├── src/ui/settings/environment.rs      # labels from the core constants
├── src/ui/material/menu.rs             # MenuOverlay::note, menu_panel_size_with_note
├── src/ui/material/menu_anatomy.rs     # the note's layout and the size estimate
├── src/ui/material/mod.rs              # re-export menu_panel_size_with_note
├── src/ui/mod.rs                       # the start list passes its note and clamps with it
├── src/showcase/sections/floating.rs   # a MenuOverlay with a note
├── src/main_tests.rs                   # the shell arm files env
├── tests/missing_cli_is_reported_where_it_is_chosen.rs   # U1, U2, W5
├── tests/unavailable_default_says_so.rs                  # U3
├── tests/directory_availability.rs                       # C2, C3, U6's rule
├── tests/start_failure_notice.rs, tests/features_settings.rs   # fixtures that quote the old text
└── tests/layout_snapshot.rs            # regenerated if the showcase entry changes a snapshot

docs/user-guide/settings.md, sandboxed-daemon.md, worktrees-and-sessions.md, agent-tools.md   # FR-017
docs/daemon.md                          # the stale "version 10 today" sentence names version 18
                                        #   (the record of the version is version.rs's paragraph
                                        #   and the pin in tests/schema_hash.rs)
```

Paths marked NEW do not exist yet. `spawn_env_for`, `availability_in`, `ResolvedEnv`,
`start_menu_note`, `MenuOverlay::note` and `menu_panel_size_with_note` are new names in existing
files. Every other path and type exists today.

**Structure Decision**: Existing workspace layout. Classification and wording go to `micold-core`
because the service and the client both say the sentences. The service owns the state, because it
runs the attempt. The client's rules for each surface stay in render-free `features/`, and `ui/`
only draws what they return (028's encapsulation). The one new UI capability is added to the
shared menu component.

## Test strategy (which layer tests each requirement)

| Requirement | Layer | Where |
|---|---|---|
| FR-001 (six states, one reason and action each), FR-004 (several CLIs, one reason) | core unit | `micold-core/tests/cli_reason.rs`: `classify` per row of R2; `explain` per row of W2 for one, two and three CLIs |
| FR-002, SC-003 (no "not installed" where no script was applied) | core unit | `cli_reason.rs`: rule W2a over every state × place × surface function |
| FR-003 (display names, setting labels) | core unit + client | `cli_reason.rs`: W2b. `ui/settings/environment.rs` reads the same constants, so there is no second copy to test |
| FR-004a (the directory named) | core unit + client unit | `cli_reason.rs`: W2c. `tests/directory_availability.rs`: a row reading the home answer has `asked_for == Home` |
| FR-005 (container placement) | core unit + client unit | `cli_reason.rs`: image rows, W2d. `tests/missing_cli_is_reported_where_it_is_chosen.rs`: both notes in each state |
| FR-006, FR-007 (Settings note, complete by itself) | client unit (pure) | `tests/missing_cli_is_reported_where_it_is_chosen.rs`: U1 per state; W2f |
| FR-008 (missing-default message) | client reducer + core unit | `tests/unavailable_default_says_so.rs`: U3 per state, the list still opens, nothing starts, and `start_refusal_unknown`'s sentence (R8) when `env` is `None`. `cli_reason.rs`: W3e |
| FR-009 (start and restart failure) | service integration | `micold-daemon/tests/session_start.rs`: U4 for Fresh and Resume in an off, a failed and an applied state; W3a–W3c |
| FR-009a (reply to an AI session) | service integration (unix) + core unit | `micold-daemon/tests/mcp_create_session.rs`: U5, by rewriting `a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record`, which asserts the binary name today. The sentence itself is `explain`'s, tested on every platform in `cli_reason.rs` |
| FR-010 (row list note; nothing on a row with fewer than two) | client unit + geometry gate | `tests/directory_availability.rs`: `start_menu_note` for 3/2/1/0 available. `ui/material/menu_anatomy.rs`: the note is laid out, is not pressable, and the size estimate matches |
| FR-011, SC-005 (silence) | client unit | `missing_cli_is_reported_where_it_is_chosen.rs` and `directory_availability.rs`: no answer, nothing missing, `env` `None` (W5) |
| FR-012, SC-004 (one answer, all surfaces agree; event messages not rewritten) | core unit + client reducer | one function writes all sentences (R4), and `cli_reason.rs` asserts W3d: in every state and place except `Applied`/image, `start_refusal` begins with `explain`'s reason and action, and the `Applied`/image row equals today's `missing_cli_reason` text (W3b). `directory_availability.rs`: a newer answer changes U1 and U6's value and posts no second notification |
| FR-013 (follows a save or a new resolution, no restart) | service integration + shell | `ai_cli_availability.rs`: after `set_env_include` the next answer carries the new state. `main_tests.rs`: the `AiCliAvailability` arm files `env` on the occasions 033 contract C1 lists |
| FR-014, SC-006 (no extra script run) | service integration | `ai_cli_availability.rs`: the run-counting script is run once across an answer, a start check and a second answer |
| FR-015 (changes nothing) | client reducer | the notice functions take `&State` and return a string; `unavailable_default_says_so.rs` keeps asserting the stored default is unchanged |
| FR-016 (three OSes) | CI | core tests and `ai_cli_availability.rs` run on the Linux, macOS and Windows jobs; `mcp_create_session.rs` on unix only; W2e |
| FR-017 | docs | the four user guide pages; CI's user-guide gate |
| A1 (wire) | core unit | `tests/schema_hash.rs` (version 18), `tests/protocol_roundtrip.rs` |
| SC-001 (six of six, both placements), SC-002, SC-007, rendering | quickstart §B | `visual-pass` skill, steps B1–B14 |

## Complexity Tracking

No Constitution Check violations.

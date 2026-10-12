---
feature: 551-session-spawn-tree
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 12 # US1 1-4, US2 1-3, US3 1-2, US4 1-3
planned_at: f602c37c
updated_at: f602c37c
suite_baseline: unknown # not re-run at planning; profile records green at cdc473ab
---

# Test List: Session tree for agent-spawned sessions

Outer loop is headless integration tests: daemon (`crates/micold-daemon/tests/`) and client reducer
and layout gates (`crates/micold-client/tests/`). The rendered pass is quickstart Part B.

## Outer loop: acceptance behaviors

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1 | A session created by an agent in the same worktree is a child of the caller and is not a top-level sidebar session | US1-1, FR-003, FR-004, FR-019 | example | PENDING | |
| A2 | A child of a child nests at depth 3 while the first child stays under the origin | US1-2, FR-003 | example | PENDING | |
| A3 | A session the user started has no origin and renders top-level as before | US1-3, FR-005 | example | PENDING | |
| A4 | A child opens and takes actions like any session and its row carries the child marker | US1-4, FR-007, SC-004 | example | PENDING | |
| A5 | After a daemon restart the origin, its two children and their nesting are unchanged | US2-1, FR-006, SC-002 | example | PENDING | |
| A6 | A child running at daemon stop is resumable after restart and still nested | US2-2, FR-006 | example | PENDING | |
| A7 | A child whose origin was deleted stays visible top-level marked "origin removed", once | US2-3, FR-008, FR-018 | example | PENDING | |
| A8 | A session from a provider with no signal is top-level and nothing is nested by guesswork | US3-1, FR-002, SC-003 | example | PENDING | |
| A9 | The user guide lists every provider with its reporting status | US3-2, FR-014, SC-005 | example | PENDING | |
| A10 | Collapsing an origin hides its children and shows their count | US4-1, FR-011 | example | PENDING | |
| A11 | A collapsed origin shows that a hidden child needs attention | US4-2, FR-011 | example | PENDING | |
| A12 | A fold persists across restart and a never-folded origin is expanded | US4-3, FR-017 | example | PENDING | |

## Inner loop: unit behaviors

### `crates/micold-core/src/origin.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1 | A self-referencing origin is rejected | FR-009 | example | PENDING | |
| U2 | An origin in another project or worktree is rejected; same location accepted | FR-009 | example | PENDING | |
| U3 | An origin whose chain reaches the child is rejected; a chain of the bounded length that does not is accepted | FR-009 | example | PENDING | |
| U4 | A second, different origin does not replace the first (`Ok(false)`); the same origin is idempotent | FR-010 | example | PENDING | |
| U12 | `SessionTree` shows each session once, nests to any depth, breaks a cycle at the first repeat, keeps catalog order, and classifies an absent origin Pending or Removed by status | FR-003, FR-008, FR-016, A2 | example | PENDING | |

### `crates/micold-core/src/store.rs`, `protocol`, `mcp/tools.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U5 | A catalog without `origin` loads top-level; a stored origin and `origin_rejected` survive a round trip | FR-006, FR-015 | example | PENDING | |
| U6 | `SessionSummary.origin` round-trips, decodes to `None` from an older payload, protocol is 41 | FR-013 | contract | PENDING | |
| U9 | `list_sessions`/`get_session`/`create_session` rows carry `origin` and `origin_state` and omit them when top-level | FR-013 | contract | PENDING | |

### `crates/micold-daemon/src/catalog.rs`, `mcp/tools.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U7 | `create_session` for the caller's own worktree records the caller as origin in the same persist | FR-019 | example | PENDING | |
| U8 | `create_session` for another worktree records no origin | FR-009 | example | PENDING | |
| U10 | `Catalog::load` keeps origins; a live child restores interrupted-resumable under its origin | FR-006 | example | PENDING | |
| U11 | Each archive path emits a `removed` `SessionChanged` per direct child; children are not deleted or stopped; stopping an origin changes nothing | FR-012, FR-018 | example | PENDING | |

### `crates/micold-client/src/features/sidebar.rs`, `ui/`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U13 | `worktree_tree` and the Default group nest from `SessionTree`; orphans are top-level with pending/removed annotation; a pending child nests when its origin appears | FR-003, FR-004, FR-005, FR-016 | example | PENDING | |
| U14 | Child rows, orphans included, carry the child marker; user-started top-level rows do not | FR-007, SC-004 | approval | PENDING | |
| U19 | `ToggleOrigin` folds and expands; folded origin shows count and descendant attention rollup; fold survives a snapshot after restart without pruning | FR-011, FR-017 | example | PENDING | |

### `crates/micold-core/src/provider.rs`, `settings.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U15 | Default `reported_origin` is `None`; each confirmed fixture yields its parent and the same fixture without the field yields `None`; name/path/time never yields one | FR-001, FR-002 | example | PENDING | |
| U16 | Discovery records a reported origin once, keeps an unknown one pending, clears a rejected one once and never re-records it | FR-001, FR-010, FR-016 | example | PENDING | |
| U17 | The provider table lists every `AiCli` variant | FR-014, SC-005 | example | PENDING | |
| U18 | `folded_origins` round-trips, absent loads empty, `settings_version` unchanged | FR-017 | example | PENDING | |

## Invariants and edge cases still to place

- Dozens of children under one origin keep catalog order (U12); frame time is a plan goal, not a requirement, and has no test.

## Out of scope

- Name-based hiding of agent worktrees: companion issue #552.
- Attaching existing provider sessions (582), changing how sessions start, manual reparenting.
- Nesting Pi forks and Copilot in-session sub-agents (R2: not spawns of separate sessions).

## Verification commands

From `.specify/memory/tdd-profile.md`: single test `scripts/build-lock.sh cargo test -p <crate> --test <binary> <name> -- --exact`; fast subset `mise run test-core`; full suite `mise run gate`.

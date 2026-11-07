# Tasks: `[orchestrator]` config section and `config check` rules

**Input**: [spec.md](spec.md), [plan.md](plan.md), [data-model.md](data-model.md), [contracts/config-check.md](contracts/config-check.md)
**Tests**: required (Constitution I): each test task comes before the code it covers.

Paths: core `crates/micold-core/src/daemon_config/` (`mod.rs`, `validate.rs`, `files.rs`, `resolved.rs`, `tests.rs`); daemon tests `crates/micold-daemon/tests/`.

## Phase 1: Setup

- [x] T001 Read `validate.rs` (`Checker::at`, `at_key`, `paths`, `locate`) and `files.rs` (`secret_entries`, V15) and note the reuse points in the PR body; no code change.

## Phase 2: User Story 1 + 2 — table, rules and precise errors (P1) 🎯 MVP

- [x] T002 [US1] Failing tests in `daemon_config/tests.rs`: complete table parses and resolved view shows every key as path and state (US1.1); unknown key refused with `file:line:column` (US1.2); no table gives the same result and view as an existing fixture (US1.3, SC-003); and view-state cases: `path (readable, <mode>)`, `path (missing)` for all four file keys (US3 scenarios 2-3).
- [x] T003 [US2] Failing tests in `daemon_config/tests.rs` for rules O1–O5, V4 and type error, each asserting the reported `file:line:column`, both cert/key directions, empty table (two errors), header-position helper for an empty table and a table with keys (SC-001).
- [x] T003a [US1][US2] Failing `config check` exit-code test (0 valid, 2 invalid, `file:line:column` on stderr) in `crates/micold-daemon/tests/config_check.rs` (US1.1, US2, FR-006).
- [x] T004 [US1] Add `Orchestrator` struct (`deny_unknown_fields`, `SecretPath` file fields, `url: Option<String>`) and `DaemonConfig.orchestrator: Option<Orchestrator>` in `daemon_config/mod.rs`.
- [x] T005 [US2] Add the `[orchestrator]` header line:column helper and `Checker::orchestrator()` rules O1–O5 plus V4 entries in `Checker::paths` in `daemon_config/validate.rs` (with the hand-written https URL check, research D2/D7).
- [x] T006 [US1] Add `OrchestratorView` and its `Display` block to `daemon_config/resolved.rs`, `None` when the table is absent.
- [x] T007 [US1][US2] Add the `[orchestrator]` section, keys, rules O1–O5/V4 and example errors to `docs/user-guide/daemon-configuration.md` (FR-008, SC-004).

## Phase 3: User Story 3 — secrets never leak, file checks (P1)

- [x] T009 [US3] Failing sentinel leak test in `daemon_config/tests.rs` over `Debug`, `Display` and the resolved view (SC-002).
- [x] T010 [US3] Failing `--files` tests in `daemon_config/tests.rs`: missing file and directory are errors (V15), `key_file`/token mode warning on host and error in container/pod (V14).
- [x] T011 [US3] Add V15 entries for the four files in `files.rs` and `key_file` + token file to `secret_entries()` (V14).
- [x] T012 [US3] Add a regression reload test (expected green once T004–T005 land, FR-011 uses the existing `config_watch.rs` reject path) in `crates/micold-daemon/tests/config_reload.rs`: an invalid `[orchestrator]` table keeps the last configuration (FR-011).
- [x] T013 [US3] Document `--files` behaviour, severities and the no-leak guarantee in `docs/user-guide/daemon-configuration.md`.

## Dependencies

T002, T003 and T003a before T004–T006; T009–T010 before T011; T013 after T011.

## Implementation Strategy

MVP is Phase 2 (accepted table with exact errors). Phase 3 adds the file checks and the leak guarantees on top.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — `[orchestrator]` table and rules 🎯 MVP

- **Tasks**: T001–T007, T003a
- **Deliverable**: A daemon.toml with an `[orchestrator]` table parses, shows in the resolved view, and `config check` reports O1–O5, V4 and type errors at `file:line:column`; a file without the table is unchanged; the user guide documents it.
- **Satisfies**: US1 acceptance scenarios 1–3; US2; FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-008, FR-010; US3 scenarios 2–3 (view state)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test config_check`; `scripts/check-user-guide-updated.sh` and a hand check that each rule id appears in the guide (SC-004)
- **Depends on**: —
- **Tier**: full

### M2 — Secret-safe file checks

- **Tasks**: T009–T013
- **Deliverable**: `config check --files` errors on missing or non-regular files and flags permissive key/token modes; no secret content appears in `Debug`, `Display` or the resolved view; an invalid reload keeps the last configuration.
- **Satisfies**: US3 acceptance scenarios 1, 4 (leak, file checks); FR-007, FR-009, FR-011; SC-002
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test main config_reload`
- **Depends on**: M1
- **Tier**: full

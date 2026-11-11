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

### M3 — Refuse credentials and fragments in `url`; close test gaps

- **Tasks**: T014–T020
- **Deliverable**: `[orchestrator] url` with userinfo or a fragment is refused as an invalid URL at `file:line:column` (no password held in the file); the O4 rule, empty host, `--files` and no-leak behaviour are pinned by tests through both core and the `config check` binary.
- **Satisfies**: FR-002, spec edge case (url credentials/fragment); US3 (no leak) through the binary; `tdd/verification.md` findings 1–7, 10
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test config_check`; `scripts/check-user-guide-updated.sh`
- **Depends on**: M2
- **Tier**: full

## Phase 4: Close-unit remediation (M3)

The feature is not done until the HIGH findings (T014–T016) are cleared. Source: `tdd/verification.md`.

- [x] T014 [Finding 2] Add a core test for O4 with `key_file` plus `enrollment_token_file` and no cert (error at `orchestrator.enrollment_token_file`), `crates/micold-core/src/daemon_config/tests.rs` near line 1180. Prove: `scripts/build-lock.sh cargo test -p micold-core --lib daemon_config::tests::o4`; mutant `token && cert` at `validate.rs:1041` must fail it.
- [x] T015 [Finding 3] Refuse (FR-002, spec edge case) a `url` with userinfo or a fragment as an invalid URL in `check_https_url` (`validate.rs:330`), with tests asserting `file:line:column` at the url value for `https://u:p@h` and `https://h/#f`. Prove: `scripts/build-lock.sh cargo test -p micold-core --lib daemon_config`.
- [x] T016 [Finding 1] Record the TDD evidence gap: add `tdd/test-list.md` and `tdd/cycle-log.md` for M1/M2 (or note the waiver in `autopilot.md`); future milestones start with a recorded red.
- [x] T017 [Finding 4] Test `https://:8443` as an invalid URL at the url value. Prove: mutant dropping `host.starts_with(':')` fails.
- [x] T018 [Findings 5, 6] Extend `crates/micold-daemon/tests/config_check.rs` with an `[orchestrator]` O3 case and a `--files` case (missing file error exit 2, sentinel absent from stdout/stderr). Prove: `scripts/build-lock.sh cargo test -p micold-daemon --test main config_check`.
- [x] T019 [Findings 7, 10] Assert each key's own path in `tests.rs:1073-1078`; add empty-string url and duplicate-table cases.
- [x] T020 Update `docs/user-guide/daemon-configuration.md`: `url` with userinfo or a fragment is refused as an invalid URL (CI user-guide gate fires on this feat PR).

# Implementation Plan: `[orchestrator]` config section and `config check` rules

**Branch**: `feat/1728_feat-orchestrator-config-section-and-config-check` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1728-orchestrator-config-section/spec.md` (issue #1728, part of #1722)

## Summary

Add an optional `[orchestrator]` table to `micold_core::daemon_config` (`url`, `ca_file`,
`cert_file`, `key_file`, `enrollment_token_file`; all four file keys are `SecretPath`, so no
secret content can reach `Debug`/`Display`). New rules in the existing checker (`validate.rs`,
`files.rs`) give `file:line:column` errors; the resolved view gains an orchestrator block; the user
guide documents the table. Technical approach is in [research.md](research.md); no new crate.

## Technical Context

**Language/Version**: Rust (workspace toolchain). **Primary Dependencies**: `serde`, `toml`
(already used by `micold-core`). No `url` crate: the scheme/host/userinfo/fragment check is a small
hand-written function (research D2). **Storage**: none new; secrets stay in files named by path.
**Testing**: `mise run test-core` (core unit tests in `daemon_config/tests.rs`), a daemon
`config check` test in `micold-daemon`. **Target Platform**: all three desktop OSes; the mode check
is skipped on Windows as today. **Project Type**: shared core library + daemon CLI. **Constraints**:
a file without `[orchestrator]` behaves byte-for-byte as before (FR-010, SC-003). **Scale/Scope**:
one table, five keys, seven error rows.

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I. Test-First | Pass | One failing test per anchor row and per leak check before code ([tests below](#test-strategy)). |
| II. Native Multi-Session | Pass | Table is per daemon; no shared state between daemons. |
| III. Native Worktree Integration | Pass (n/a) | No worktree behaviour. |
| IV. Local-First Storage | Pass | Config and secrets are local files by path; the URL is only stored, never contacted here. |
| V. Rust + iced Stack | Pass | Core and daemon only; no UI. |
| VI. Cross-Platform Parity | Pass | Absolute-path check reuses `is_absolute` (Windows paths as literal strings); mode state shows the existing `readable, ACL not checked` on Windows (the spec's "`readable`" is shorthand). |
| VII. Documentation | Pass | `docs/user-guide/daemon-configuration.md` updated in the same change (FR-008). |
| VIII. Reusable UI Components | Pass (n/a) | No UI; rendering from Settings is another issue. |

No violations; Complexity Tracking is empty. Re-checked after design: unchanged.

## Project Structure

```text
specs/1728-orchestrator-config-section/
  plan.md  research.md  data-model.md  quickstart.md  contracts/config-check.md
crates/micold-core/src/daemon_config/
  mod.rs        # new `Orchestrator` struct, `orchestrator: Option<Orchestrator>` on DaemonConfig
  validate.rs   # Checker::orchestrator(): rules O1-O5; paths(): V4 entries
  files.rs      # V15 entries for the four files; secret_entries(): key_file + token file
  resolved.rs   # OrchestratorView + Display block
  tests.rs      # unit tests
crates/micold-daemon/src/config_load.rs   # unchanged
crates/micold-daemon/src/config_watch.rs  # unchanged; FR-011 reload test lives with it (`reject(.. "keeping the last configuration")`)
docs/user-guide/daemon-configuration.md   # [orchestrator] section, rules, errors
```

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 (table, unknown key refused) | `Orchestrator` with `#[serde(deny_unknown_fields)]`; existing unknown-key path gives `file:line:column` |
| FR-002 (absolute paths, no secret value) | `SecretPath` fields; V4 list in `Checker::paths` |
| FR-003 (`url` required, https) | rule O1/O2 in `Checker::orchestrator` |
| FR-004 (cert/key together) | rule O3 |
| FR-005 (token xor pair) | rules O4, O5 |
| FR-006 (`file:line:column`) | `Checker::at` / `at_key` with the anchors of the spec table |
| FR-007 (no leak) | `SecretPath` Debug/Display; `OrchestratorView` holds `SecretStatus` only |
| FR-008 (docs) | user guide |
| FR-009 (`--files`) | `check_files` V15 (missing or not a regular file: error) and V14 (mode, via `secret_entries`) |
| FR-010 (unchanged without table) | field is `Option`, every new code path is guarded; golden/fixture tests untouched |
| FR-011 (reload keeps last valid) | existing `config_watch.rs` path (`validate_text`, then `reject` keeps the last configuration); covered by a daemon test |

Anchors: missing `url` and "neither credential" anchor at the `[orchestrator]` header (`locate` of
`orchestrator`, value span); invalid `url`/path/type at the value; cert/key at the present key; both
credentials at the `enrollment_token_file` key. The header position is not derived by `locate` today (it returns the value span): the task adds a helper that finds the `[orchestrator]` header line:column (research D7), tested for an empty table and a table with keys. A wrong value type is a serde error, already
anchored at the value by `parse`.

## Test strategy

- **Core unit** (`mise run test-core`): one test per anchor row incl. both cert/key directions
  (SC-001); unknown key; empty table (two errors); no-table equivalence against an existing fixture
  (SC-003); sentinel leak test over `Debug`, `Display` and the resolved view (SC-002);
  `--files` missing/directory/mode cases with kind-dependent severity.
- **Daemon**: a `config check` exit-code test (0 valid, 2 invalid) and a reload test that an invalid
  table keeps the last configuration (FR-011).
- **Client / geometry gates / quickstart §B**: not applicable, no UI.
- **Docs** (SC-004): a test or script check that each rule id appears in the guide, if the existing
  guide has such a check; otherwise reviewed by hand.

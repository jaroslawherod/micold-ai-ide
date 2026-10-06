---
feature: 011-env-include-script
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 2 # scoped to BUG-454: FR-013, FR-022 (the rest of 011 closed before this list existed)
planned_at: 8f5afc15
updated_at: 8f5afc15
suite_baseline: see cycle-log.md Baseline
---

# Test List: Environment include script — BUG-454 (a per-directory failure reaches Settings)

**Scope.** Feature 011 closed before this extension was installed, and this list was written for
bugfix BUG-454 only (`bugs/BUG-454.md`, tasks T041–T046). It covers `FR-022` and the part of
`FR-013` it extends (the captured output stays off disk). The feature's other criteria shipped
before the list existed and are not re-derived here.

Derived from `spec.md` (FR-013, FR-022), `contracts/settings-ui.md` step 4 and `plan.md`'s BUG-454
design correction.

## Outer loop: acceptance behaviors

**Entry point.** The service's public `DaemonState` (`availability_in`, `catalog_snapshot`,
`invalidate_env_include`, `set_env_include`, `register`) with a real include script, in
`crates/micold-daemon/tests/env_include_failure_report.rs`. The rendered page is the client's pure
`directory_failure_lines`, fed by the snapshot the client applies.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | A directory whose resolution failed is listed in the catalog snapshot with its category and captured output; a directory that resolved is not; the entry goes with an invalidation or a settings change and returns with a new failed resolve; each change reaches a registered client as a `CatalogChanged` | FR-022 | example | DONE | `crates/micold-daemon/tests/env_include_failure_report.rs::a_directory_whose_resolution_failed_is_reported_while_its_failure_is_cached` |
| A2  | The service's log line for a failed resolve names the directory and holds none of the captured output | FR-013, FR-022 | example | DONE | `crates/micold-daemon/tests/env_include_failure_report.rs::the_log_names_the_failed_directory_without_the_scripts_output` |

## Inner loop: unit behaviors

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | `CatalogSnapshot::env_include_failures` round-trips through JSON | FR-022 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs::per_directory_env_include_failures_round_trip_in_the_catalog_snapshot` |
| U2  | A snapshot encoded without the field reads back with an empty list | FR-022 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs::a_catalog_snapshot_without_env_include_failures_reads_back_with_none` |
| U3  | `directory_failure_lines`: one caution per failing directory naming it and the category label, then a note with the captured output (none when empty); an entry equal to the representative outcome is folded away; empty in, empty out | FR-022, FR-013 | example | DONE | `crates/micold-client/tests/features_settings.rs::directory_failure_lines` |
| U4  | The client keeps the latest applied snapshot's failure list (welcome and `CatalogChanged`) | FR-022 | example | PENDING | |

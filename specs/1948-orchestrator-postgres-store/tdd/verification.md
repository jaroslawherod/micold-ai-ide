---
feature: 1948-orchestrator-postgres-store
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override, no preset)
verified_at: 40309293a # HEAD audited; feature range f5f8e812a~1..8005293fe (M1-M10 on main)
behaviors: 27 # 25 graded + 2 NOT_APPLICABLE; no tdd/test-list.md exists, so a "behavior" is one cycle-log entry / task group
proven: 11
likely: 0
test_after: 14
no_test: 0
high_smells: 1 # F3, plus 2 weakened-existing-test findings (F1, F2) that fail on their own
criteria_total: 17 # acceptance scenarios US1.1-US6.2
criteria_covered: 17 # 3 without a start-level entry-point test (US4.4, US5.2, US5.4); PostgreSQL-dependent ones not executed here
mutation_score: unmeasured # cargo-mutants not installed; 8 deliberate mutants, 8 caught (sample, not a score)
mutants_survived: 0 # of the 8 sampled
suite: "daemon `--test orchestrator` 183 passed, 0 failed, 17s run (4m10s cold build); core `--lib orchestrator:: daemon_config::` 244 passed, 0 failed; PostgreSQL cases skipped"
---

# TDD Verification: PostgreSQL store for the orchestrator (#1948)

**Verdict: FAIL.** 14 of 25 graded behaviors have no recorded red (the cycle log admits test-after;
M6 has no entry at all), two existing tests were weakened (an FR-008 scan exclusion, a dropped stderr
assertion), and every PostgreSQL case passes vacuously whenever no server is configured.

What holds: the tests that exist are mostly good. All 8 deliberate mutants on persistence, auth,
migration, import and config behaviors were caught, the shared suite pins one contract on four
backends, and every acceptance scenario reaches a test. The failure is about discipline and two
weakened guards, not about weak assertions.

Independence: this audit read the files cold in a session that did not write them. No
fresh-context subagent was available (the tool is not exposed to this agent), so the smell pass
was done by hand, not delegated; every cited line was opened.

## Test-first evidence

There is no `tdd/test-list.md`; the feature predates the tdd extension's plan step, so there are no
behavior ids and no `traces` column. Behaviors below are the cycle-log entries, mapped to tasks.
History: each milestone squash-merged to `main`, but the branch commits survive. Every feature
commit adds tests and source together, and no commit holds a test alone ahead of its source. So the
ordering evidence is "same commit" plus the log's recorded red. `PROVEN` below means exactly that,
and every red is a compile error or a named failing test taken from the log, not re-run by me.

| # | Behavior (tasks) | Class | Evidence |
| - | ---------------- | ----- | -------- |
| 1 | sqlx spike (T001) | NOT_APPLICABLE | probe deleted, not a kept test |
| 2 | `StoreError` variants, per-record defaults (T002, T004) | PROVEN | M1 c1 red: 13 compile errors; `6d4d88cad` |
| 3 | token, id, role, serial, detail types (T005, T006, T009, T010) | PROVEN | M1 c2 red: 30 compile errors |
| 4 | shared suite on memory and JSON (T003, T007, T008, T011-T013) | TEST_AFTER | M1 c3: "red: none ... test-after admission" |
| 5 | SQLite handle, schema, registry (T014-T020) | PROVEN | M2 c1 red: `unresolved import ...::db`; `a58f782bf` |
| 6 | URL encoding and mode tighten (M2 review A) | TEST_AFTER | M2 c2: tests written after the code, red shown by revert |
| 7 | seven error-path tests (M2 c3) | TEST_AFTER | M2 c3: "none was shown red" |
| 8 | SQL auth and audit on SQLite (T021, T023-T025) | PROVEN | M3 c1 red: `unresolved imports SqlAuditStore, SqlAuthStore` |
| 9 | audit log append-only triggers (T022, FR-011) | TEST_AFTER | M3 c1: green on first run; the triggers shipped in M2 with no test |
| 10 | `redeem_invite` refuses an unknown user | PROVEN | M3 c2 red recorded with output |
| 11 | nine failure-arm tests; `put_refresh` keeps `revoked_at` (M3 c3) | TEST_AFTER | M3 c3: "test-after admission, no red run" |
| 12 | PostgreSQL store (T026-T029) | PROVEN | M4 c1 red: `no function open_postgres` (4 sites) |
| 13 | URL-parse and CA-file error arms (M4) | TEST_AFTER | M4 c1: "written with the code (test-after)" |
| 14 | newer, edited, half-way, concurrent-starter migrations (T033-T035) | PROVEN | M5 c1: 4 reds with output |
| 15 | older-database upgrade, version chain (T033, FR-013) | TEST_AFTER | M5 c1: "passed at once", pins sqlx behavior |
| 16 | M5 review A fixes (retry bound, re-check newer) | TEST_AFTER | M5 c2: retry bound and re-check "test-after" |
| 17 | no private key in any table (T036-T038, FR-006) | TEST_AFTER | **no M6 entry in the cycle log**; `c19f093a6` adds tests only (`auth.rs` +24 lines, all in `mod tests`), so the source already satisfied them |
| 18 | `[database]` config and `choose_store` (T039-T043) | TEST_AFTER | M7: "test-after ... red was not observed" |
| 19 | orchestrator starts on the chosen store (T045-T050) | TEST_AFTER | M8 c1: red recovered by reverting each change |
| 20 | refuse an unimported `registry.json` (M8 F1) | PROVEN | M8 c2 red at `store_config.rs:172`; superseded by M9 |
| 21 | lost save reported on stderr (M8 F3) | PROVEN | M8 c2 red recorded |
| 22 | import `registry.json` once (T051-T053) | PROVEN | M9 red: suite did not compile / M8 refused |
| 23 | `OwnerId`, `DaemonRecord.owner` (T055, T058) | PROVEN | M10 red: types did not exist |
| 24 | owner survives relink; `reassign_unowned` (T056, T058, T059) | TEST_AFTER | M10: "red by revert" |
| 25 | every table has `tenant_id`, owned ones `owner_user_id` (T057) | TEST_AFTER | M10: "green on arrival" |
| 26 | import keeps a file's owner (T059) | TEST_AFTER | M10: "green with the insert-only bind" |
| 27 | CI service, docs, quickstart (T030, T031, T044, T050, T054, T060) | NOT_APPLICABLE | not behavior; `ci.yml:236-240` sets URL and `MICOLD_REQUIRE_POSTGRES=1` |

Log versus history: no entry contradicts the history. Two log entries name tests that no longer
exist: `a_database_from_a_newer_build_is_a_failed_schema_error_naming_the_version` (replaced in M5
by a stronger test) and `an_unimported_registry_json_refuses_the_start_naming_the_file` (the refusal
was reversed on purpose by M9). Baseline: "suite not run locally before M1", so no pre-feature green
count is recorded; the profile's 5071-pass baseline is at `cdc473ab`, not at `f5f8e812a~1`.

### Changes to tests that already existed

Checked every feature commit's removed assertion, test and exclusion lines.

| Where | Before | After | Judgment |
| ----- | ------ | ----- | -------- |
| `tests/orchestrator/no_outbound.rs:44-55,231-245` (FR-008 scan) | scanned every file under `orchestrator/` except `dial.rs` | `952b8a818`/`2deff3756` skipped all of `db/`; `c3944869a` narrowed it to blanking `.connect_with(`; `4d31eb32b` (a test-only commit after the source) added `AnyConnection::connect(` | exclusion added to an existing guard: F1 |
| `tests/orchestrator/restart.rs:147-165` (`2243de02d`) | `a_failed_save_is_logged_to_stderr_and_links_keep_working`: real process, waited for the line `orchestrator registry <path> could not be saved` | `a_failed_save_leaves_links_working`: in process on the JSON store, no log assertion | assertion removed: F2 |
| `restart.rs` `a_corrupt_registry_fails_the_start...` | corrupt `registry.json` fails the process start | corrupt `orchestrator.db` fails it | legitimate: the process now runs on SQLite; the JSON case moved to `store_import.rs:130` and `store_config.rs:150` |
| `store_config.rs:150-200` (M8 test, reworked in M9) | exact `assert_eq!` on the error text | `contains(path)` | LOW, same-feature test, see F8 |
| M5 `a_database_from_a_newer_build...` | `reason.contains("999")` | replaced by a test that pins both versions and the file fingerprint | stronger |
| M6 `store_suite.rs` (`3d5485c30`) | private helper fns and cases | made `pub(super)` for `store_scan.rs`; all 17 case names still present | not weakened |

No test was marked ignored or renamed out of a filter, and no coverage or mutation threshold was
lowered (`scripts/guardrails-scope.conf` and `mise.toml` untouched by the feature).

`tasks.md` against evidence: every T001-T060 is `[X]` and every behavior above has a test, so no
completion claim lacks a test. There are no unticked behavioral tasks. The 14 `TEST_AFTER` rows are
the tasks whose "failing tests first" wording (T014, T021, T022, T033, T036, T039-T041, T046-T047,
T051, T055-T057) the log says was not followed.

## Findings

Ordered by severity. F1 and F2 are weakened existing tests, each a `FAIL` on its own.

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| F1 | HIGH | The FR-008 no-outbound source scan now blanks every `.connect_with(` and `AnyConnection::connect(` under `orchestrator/db/`, whatever they connect to. Each was added after the source, to make the scan pass. A non-database dial spelled that way in `db/` is invisible; only `netns.rs` still covers it at runtime. | `no_outbound.rs:44-55,231-245`; history above |
| F2 | HIGH | The process-level assertion that a failed save logs `a registry change could not be saved` was removed. It survives only in `store_slow.rs:199`, which returns early without PostgreSQL. A default local-mode (SQLite) or JSON run has no test for the log line `save.rs:91` prints. | `restart.rs:147`; `store_slow.rs:201-207`; `save.rs:91` |
| F3 | HIGH | Conditional logic that asserts nothing: 28 tests with `postgres` in the name, plus the PostgreSQL arm of others, `return` when no server is configured and report `ok`. I ran them: 27 `skipped:` lines, 0 assertions. `MICOLD_REQUIRE_POSTGRES=1` turns the skip into a failure in CI (`store_support.rs:152`, `ci.yml:236-237`), so the CI run is the only evidence for FR-012, SC-004, US1.1 and FR-016's lost-database case. I did not audit that run. | `store_suite.rs:246,583,676`; `store_migrations.rs:326`; `store_scan.rs:43,199,226,233,248,271`; `store_slow.rs:55,75,201`; `store_import.rs:77` |
| F4 | HIGH (process) | 14 of 25 behaviors are `TEST_AFTER`; M6 (US3, FR-006, SC-003) has no cycle-log entry and no source change, so its tests could not have been red. Mutation of `CertSerial::new` is caught (mutant 5 below), which is the only after-the-fact red for it. | cycle log; `c19f093a6` |
| F5 | MED | US5.2 (second start imports nothing) and US5.4 (refuse with rows, naming file and count) are tested on `import_registry_json` directly, not through the start. `store_config.rs` covers only the first import and the corrupt file. US4.4 (non-loopback, no `[database]`) is a unit test of `choose_store`; `config check` always passes `LoopbackOnly`. The spec accepts US4.4 (#1732 wires it). | `store_import.rs:118,148`; `backend.rs:95`; `store_config.rs:150` |
| F6 | MED | Hand-rolled copies of helpers the suite already shares: `T`, `uid`, `hash`, `user`, `token`, `invite` duplicate `store_suite.rs:255-300`; `record` and `keys` duplicate `daemon` and `keys` (`store_suite.rs:27,41`). (Bypassed test utility, duplicated setup.) | `store_sql_errors.rs:14-68`; `store_import.rs:18-34` |
| F7 | MED | Vacuous or near-vacuous assertions: `!reason.is_empty()` where the neighbours pin target and no URL leak (acceptable for driver-owned text, but it cannot fail on a wrong cause); `target().rsplit_once(':').map(\|(_, p)\| p.is_empty()) == Some(false)` says nothing about a port. | `store_migrations.rs:186,292,340,667` |
| F8 | MED | Error-text coupling and a loosened exact assertion: refusals pinned by substrings (`"no bootstrap"`, `"does not match"`, `"already"`, `"user"`), and `contains(path)` where M8 had the exact message. | `store_sql_errors.rs:269-307`; `store_import.rs:159`; `store_config.rs:172-175` |
| F9 | MED | Eager tests: one test per store area walks 8-15 behaviors, so a failure names the case, not the rule. | `store_suite.rs:309,398,448,498`; `store_sql_errors.rs:85,173` |
| F10 | LOW | The FR-014 test asserts a real-clock `elapsed < 9 s` for a deadline the test clock drives, so it cannot fail for the reason it names; the stalled-store case is a parked double, not a slow database. | `store_slow.rs:175-191` |
| F11 | LOW | With one shipped migration, "an older database applies the missing migrations in order" and the version-chain test cannot fail on ordering; they pin sqlx. The cycle log says so. | `store_migrations.rs` M5 cases; cycle log M5 c1 |
| F12 | LOW | Internal-API coupling: `handle.map_error(&sqlx::Error::PoolTimedOut)` and raw positional `INSERT INTO daemons VALUES (...)` literals. | `store_migrations.rs:215-229,218,238` |

Other properties: tests are isolated (a tempdir or a database per case), the orchestrator target
runs in 17 s, and the race cases (two writers, two starters, six redeemers) assert an invariant, not
a timing. The log claims the two-starter case passed "5 of 5 runs", so the race is a known flake
risk to watch. Poll loops wait on a condition with a 10 s bound (`store_suite.rs:729`,
`store_config.rs:128`), not a fixed sleep.

## Mutation results

Tool: none. `cargo-mutants` is not installed here and `mise run mutants-diff` needs it, plus a
branch diff over a range this size is too costly for a cloud container with 20 GB free. Mutation
score is **unmeasured**. Instead, 8 deliberate mutants, one at a time, each restored with
`git checkout -- <file>` and verified (`git status` clean, then `orchestrator` 183 passed and
`micold-core orchestrator::` 104 passed). This is a sample of high-risk behaviors, not a score.

| # | Mutant | Behavior | Survived | Caught by |
| - | ------ | -------- | -------- | --------- |
| 1 | `migrate.rs:56` `database > build` to `>=` | FR-004 newer schema | No | 20 tests (every SQLite open at equal version fails; the "newer by one" edge is not isolated) |
| 2 | `db/auth.rs:347` drop `AND redeemed_at IS NULL` | invite redeemed once | No | `an_invite_is_redeemed_once`, `of_many_callers_redeeming_one_invite_exactly_one_wins`, 2 more |
| 3 | `db/registry.rs` `save` never deletes stale rows | whole-list `save` replaces | No | `save_replaces_the_whole_set_across_tenants`, `sqlite_holds_no_key_material` |
| 4 | `db/import.rs:82` import into a non-empty registry | FR-010 data-loss guard | No | `rows_without_a_matching_marker_refuse_...` |
| 5 | `core auth.rs:149` drop the hex check in `CertSerial::new` | FR-006 | No | 2 core unit tests |
| 6 | `backend.rs` network bind with no database returns SQLite | FR-009 | No | `no_table_and_a_network_bind_fails_naming_the_key` |
| 7 | `db/auth.rs:489` reuse of a revoked token no longer revokes its family | refresh-token reuse | No | `refresh_tokens_rotate_and_a_reused_one_revokes_its_family` |
| 8 | `db/mod.rs:384,388` file mode `0600` to `0644` | owner-only SQLite file | No | 3 tests |

No survivor inside a `DONE` behavior. Not sampled: `daemon_config` validation, the `[database]`
view, the migration retry bound, `reassign_unowned` on projects and sessions, every PostgreSQL code
path (no server), and the start-up wiring in `orchestrator/mod.rs`.

Coverage: not run (`mise run coverage-diff` needs `cargo-llvm-cov`). Not used as corroboration.

## Traceability

Real entry points: the three store traits through each backend, `db::open`, `run_configured`, the
`micold-daemon orchestrator` process, and `config check`. PostgreSQL rows are "yes in CI, not run
here".

| Criterion | Tests | Entry point |
| --------- | ----- | ----------- |
| US1.1 PG restart | `store_suite::postgres::*` (every case ends in `reopen`) | yes, PostgreSQL only |
| US1.2 SQLite restart | `store_suite::sqlite::*`; `store_config::only_a_data_directory_starts_on_sqlite_and_keeps_the_registry_across_a_restart` | yes (process-level for the registry only; users and audit are not wired into the orchestrator yet) |
| US1.3 in-memory parity | `store_suite::memory::*`, `json::*` | yes |
| US2.1 newer refused, unchanged | `store_migrations::a_newer_database_is_refused_...` (both dialects, SQLite fingerprint) | yes |
| US2.2 older applies in order | `an_older_database_applies_the_missing_migrations_in_order` | yes, but see F11 |
| US3.1, US3.2 | `store_scan::key_scan::*` (negative control present); core `a_cert_serial_rejects_...`, `a_token_hash_rejects_a_pem_block` | yes |
| US4.1 view never shows URL | `store_config::a_readable_url_file_passes_...`; `daemon_config::tests::database_the_view_shows_...` | yes (`config check`) |
| US4.2 missing or empty file | `store_config::a_missing_empty_or_foreign_url_file_...`, `a_missing_ca_file_or_url_key_...` | yes |
| US4.3 loopback uses SQLite | `only_a_data_directory_starts_on_sqlite_...` | yes |
| US4.4 network, no table fails | `backend::tests::no_table_and_a_network_bind_fails_naming_the_key` | unit only (spec accepts; F5) |
| US5.1 import and rename | `store_import::an_unimported_file_imports_...`; `store_config::the_start_imports_...` | yes |
| US5.2 second start | `store_import::a_second_start_imports_nothing_...` | `import_registry_json` only (F5) |
| US5.3 corrupt file | `store_import::a_corrupt_file_...`; `store_config::the_start_imports_...` | yes |
| US5.4 rows refuse | `store_import::rows_without_a_matching_marker_...` | `import_registry_json` only (F5) |
| US6.1 tenant and owner columns | `store_scan::key_scan::*_tables_carry_tenant_and_owner` | yes |
| US6.2 tenants disjoint | `two_tenants_on_one_backend_see_disjoint_rows`, `audit_entries_are_per_tenant_...` | yes |

Functional requirements with no scenario of their own: FR-012 (CI service container, `ci.yml:236-240`,
checked by reading it, not by running it), FR-013 (`every_shipped_version_upgrades_...`), FR-014
(`a_save_parked_in_a_store_call_...`, F10), FR-015 (registry cases), FR-016 (unreachable host:
`an_unreachable_host_fails_the_start_...`, runs here; lost-after-start: PostgreSQL only).

Untested criteria: none. Tests tracing to nothing: none found; all `store_*` files name FRs or US
scenarios in their headers. Every test name in the cycle log was checked against the tree:
two are gone and explained above.

## What was not audited

- Any PostgreSQL behavior: the local cluster on port 5432 needs a password I do not have, and I did
  not alter its auth. All PostgreSQL cases ran as skips. The CI run that executes them with
  `MICOLD_REQUIRE_POSTGRES=1` was not inspected.
- Mutation testing proper: unmeasured (no `cargo-mutants`); only the 8-mutant sample above.
- Coverage: not run.
- The full workspace suite (`mise run gate`): not run, because of its disk and I/O cost (see
  CLAUDE.md). I ran `micold-daemon --test orchestrator` (183) and `micold-core --lib
  orchestrator:: daemon_config::` (244 passed; it includes other features' orchestrator tests).
- #1725's tests in the same commit range (announce, dial, route, hub, list, link): out of scope,
  except the pre-existing `no_outbound`, `restart`, `not_hosted` and `support` files.
- The client crate and the docs pages: no test, no criterion.
- Performance, load, and a real slow-database test: no criterion beyond the 9 s liveness deadline.
- Red-phase claims in the cycle log were not re-run; they are taken as written and corroborated
  only by same-commit evidence in git.

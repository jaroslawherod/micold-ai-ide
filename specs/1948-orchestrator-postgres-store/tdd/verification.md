---
feature: 1948-orchestrator-postgres-store
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override, no preset)
verified_at: cae46cf # HEAD audited (origin/main incl. M11 squash c5c184a03); re-verify of the FAIL at 40309293a
behaviors: 27 # 25 graded + 2 NOT_APPLICABLE; no tdd/test-list.md exists, so a "behavior" is one cycle-log entry / task group
proven: 11
likely: 0
test_after: 14 # unchanged: M11 added a mutant red after the fact to each, which the rubric does not count as test-first
no_test: 0
high_smells: 0 # catalogue HIGH smells; plus 1 HIGH survivor (N1) and the 14 TEST_AFTER rows
criteria_total: 17 # acceptance scenarios US1.1-US6.2
criteria_covered: 17 # US5.2 and US5.4 now start-level; US4.4 still unit-level (spec accepts)
mutation_score: unmeasured # cargo-mutants not installed; 8 deliberate mutants this run, 7 caught, 1 caught about half the runs (sample, not a score)
mutants_survived: 1 # N1, intermittent
suite: "daemon `--test orchestrator` without PostgreSQL 204 passed, 0 failed, 55 ignored, 19 s; with PostgreSQL 16 (MICOLD_REQUIRE_POSTGRES=1) 259 passed, 0 failed, 0 ignored, 24 s; core `--lib orchestrator::` 104 passed, `--lib daemon_config::` 140 passed"
---

# TDD Verification: PostgreSQL store for the orchestrator (#1948), re-verify after M11

**Verdict: FAIL.** 14 of 25 graded behaviors are still `TEST_AFTER` by the rubric (history cannot
change; M11's mutant reds are corroboration, not test-first evidence), and the migration retry bound
survives its mutant on about half the runs.

What changed: 8 of the 12 earlier findings are resolved, including both weakened existing tests (F1,
F2) and the silent PostgreSQL skip (F3). F4, F8, F9, F10 and F12 are only partly cleared (below). The
suite is green on both backends, and I ran the PostgreSQL cases against a real server this time.

Independence: files read cold in a session that did not write them. No fresh-context subagent is
available to this agent, so the smell pass was done by hand; every cited line was opened.

## F1 to F12 status

| # | Was | Now | Evidence |
| - | --- | --- | -------- |
| F1 | HIGH, FR-008 scan blanks every `.connect_with(` in `db/` | **Resolved** | `no_outbound.rs:41-91` blanks only `.connect_with(` on an `AnyPoolOptions::new()` chain and the full path `sqlx::AnyConnection::connect(`; `:363-379` pins both sides. Mutant: `std::net::TcpStream::connect("1.1.1.1:1")` added to `db/mod.rs` fails `the_orchestrator_source_opens_no_outbound_connection` |
| F2 | HIGH, process-level failed-save stderr assertion removed | **Resolved** | `restart.rs:169` `a_failed_local_save_is_reported_on_stderr`: real process on SQLite, a trigger aborts daemon inserts, waits for `a registry change could not be saved`. Mutant: that text changed at `save.rs:91` fails it, no PostgreSQL needed |
| F3 | HIGH, PostgreSQL cases `return` and report `ok` | **Resolved** | `build.rs` sets `cfg(micold_postgres)`; every PostgreSQL case is `#[cfg_attr(not(micold_postgres), ignore = ...)]` (`store_suite.rs:230,747,870`, `store_import.rs:83`, `store_migrations.rs:337`, `store_scan.rs:42,357-390`, `store_slow.rs:54,74,197`). Without a URL: 55 reported `ignored`; with one: 259 ran, 0 ignored. `store_support.rs:149-161` fails a case that asks for a server it cannot find when built with the cfg or `MICOLD_REQUIRE_POSTGRES=1`. The leftover `let Some(..) else { return }` lines are unreachable in a CI build. CI run 43963469440 is claimed in the log, not inspected here |
| F4 | HIGH (process), 14 TEST_AFTER, no M6 entry | **Not resolved by the rubric** | M6 entry added (`cycle-log.md` T064); rows 4, 6, 7, 9, 11, 13, 15 to 19, 24 to 26 each have a mutant, command and failing test name. The history still shows source before test and no red at the time, so the class stays `TEST_AFTER`. Row 16 recorded as surviving (see N1) |
| F5 | MED, US5.2 and US5.4 only on `import_registry_json` | **Resolved** | `store_config.rs:182,202` call `run_configured`. Mutant `import.rs:82` `rows > 0` to `rows > 1000000` fails both PostgreSQL and SQLite `store_import` cases and the start-level case |
| F6 | MED, hand-rolled fixtures | **Resolved** | `store_sql_errors.rs:14` and `store_import.rs:13` import `T, uid, hash, new_user, refresh_token, invite, daemon, keys` from `store_suite`. `jscpd`/`similarity-rs` equality is claimed in the log, not re-run |
| F7 | MED, vacuous reason and port checks | **Resolved** | `store_migrations.rs:191,251,304,683` pin the reason text and `127.0.0.1:1`; mutant forcing the PostgreSQL target port to 5432 fails 4 tests. The remaining `!sqlite.is_empty()` (`:281`) guards a non-empty comparison, fine |
| F8 | MED, substring-pinned refusals, loosened exact message | **Partly** | The corrupt-file start error is exact again (`store_config.rs:163-169`, but it embeds serde_json's text `missing field \`version\` at line 1 column 2`). The auth refusals are still substrings: N3 |
| F9 | MED, eager cases | **Partly** | `store_suite.rs` cases are one rule each (e.g. `:316-342`, `:479-504`, `:566-614`, `:645-702`) and the lost-table and bad-row cases in `store_sql_errors.rs` are split. Two eager cases remain: N3 |
| F10 | LOW, real-clock `elapsed < 9 s` | **Not resolved** | The wall-clock check became `waited < 9` on the test clock (`store_slow.rs:188-189`), but `waited` is exactly `PING_INTERVAL` (3 s) because the test advances the clock once: N2 |
| F11 | LOW, one-migration ordering | **Resolved** | `store_migrations.rs:617-641` runs a three-migration chain and asserts `0003` saw two recorded rows; the cycle log records the swapped-SQL red |
| F12 | LOW, `map_error` and positional inserts | **Mostly** | `store_migrations.rs` no longer calls `map_error` and inserts name their columns (`:70,244`). Positional `INSERT INTO ... VALUES` remain in `store_sql_errors.rs:182-188,231` |

## Test-first evidence

Same 27 rows and classes as the previous report (no `tdd/test-list.md`; behaviors are cycle-log
entries mapped to tasks). `PROVEN` rows (2, 3, 5, 8, 10, 12, 14, 20, 21, 22, 23): red recorded and
test and source land in one commit. `TEST_AFTER` rows (4, 6, 7, 9, 11, 13, 15 to 19, 24 to 26): no
red at the time. M11 gives each a mutant red after the fact; 12 name a failing test (rows 4, 6, 7, 9,
11, 13, 17, 18, 19, 24, 25, 26), row 15 is covered by the T069 swapped-migration red, and row 16
survived its mutant. Rows 1 and
27 are `NOT_APPLICABLE`.

Log versus history: no entry contradicts the history. The M11 log's CI claim (run 43963469440,
`micold-daemon::orchestrator` 247 passed, none skipped) is not corroborated here; this box ran 259
with a local server because the tree has since grown.

### Changes to tests that already existed (M11 range `a5ebdc7^..HEAD`)

Assertion-bearing lines per test file are equal or higher after M11 (`store_suite` 76 to 82, the
others unchanged), and every removed assertion was either moved into a split case or replaced by a
stricter one: `!reason.is_empty()` to pinned reasons (F7), `contains(path)` to the exact message
(F8), real-clock `elapsed` to test clock (F10, but see N2). No test was ignored except through the
`micold_postgres` cfg (reported, intended), no filter or name moved a test out of reach, and
`scripts/guardrails-scope.conf` and `mise.toml` changes in the range belong to #1735, not this
feature. `tasks.md`: T061 to T070 are all `[x]`, each backed by a cycle-log entry and a commit; no
behavioral task is unticked.

## Findings

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| N1 | HIGH | The migration retry bound has no deterministic test. With `retries = migrator.migrations.len() + 1` replaced by `0`, `two_starters_on_one_database_apply_each_migration_once` failed on 2 of 4 runs here and survived on the other 2; the M11 log recorded it as surviving (row 16). A survivor inside a `DONE` behavior (M5 review A). | `db/migrate.rs:71`; `store_migrations.rs:699`; `cycle-log.md` T064 row 16 |
| F4 | HIGH (process) | 14 behaviors are `TEST_AFTER`; the rubric fails any `TEST_AFTER`. Not fixable retroactively, only by accepting the mutant reds as the recorded exception or by a spec-level waiver. | table above |
| N2 | MED | `assert!(waited < 9)` asserts a value the test set: the clock only moves by the `advance(PING_INTERVAL)` on the line before, so `waited` is always 3 and the claim "inside the 9 s liveness deadline" is not tested. The test's real assertion is that `record_when` sees `b` online while the save thread is parked, which does hold. Compare against `LIVENESS_DEADLINE` through the orchestrator's own timer, or drop the line. | `store_slow.rs:179-189` |
| N3 | MED | Two eager, substring-coupled cases survive T068: `the_bootstrap_states_each_have_their_own_refusal` walks 5 refusals and `an_invite_and_an_enrollment_name_why_they_were_refused` walks 8, each pinned by `contains("no bootstrap")`, `"does not match"`, `"consumed"`, `"unknown"`, `"expired"`, `"user"`, `"already"`. A reworded reason fails with no rule named; the sentence `contains("user")` cannot tell a wrong cause. | `store_sql_errors.rs:287-331` |
| N4 | MED | `starts` lets a start run for a fixed 2 s then drops it (`timeout(2 s, start)` expecting `Err`). On a slow box the import may still be mid-way when the future is dropped, so the following `stored(..)` count can read 0 rows. Sleepy test; wait on the renamed file or the row instead. | `store_config.rs:245-249`, used at `:173,185,192,205` |
| N5 | LOW | `a_save_lost_with_the_database_is_reported_on_stderr_without_the_password` panics (`the test URL carries a password`) when the server is reached with a passwordless URL. It failed in my first run until the URL carried a dummy password; the requirement is not documented in `store_support.rs:145`. | `store_slow.rs:218` |
| N6 | LOW | Positional `INSERT INTO users VALUES (...)` literals in the bad-row seed (the rest of F12). | `store_sql_errors.rs:182-188,231` |
| N7 | LOW | The exact corrupt-file assertion embeds serde_json's wording (framework under test); a serde upgrade can break it. | `store_config.rs:163-169` |

Other properties: tests are isolated (a tempdir or a database per case), the orchestrator target runs
in 19 to 25 s, and the race cases assert an invariant. The two-starter case is race-dependent for the
mutant above (N1), stable on unmutated code across 6 runs here.

## Mutation results

Tool: none (`cargo-mutants` absent). Score **unmeasured**. 8 deliberate mutants this run, one at a
time, each restored with `git checkout -- <file>` and verified (`git status` shows only the
pre-existing `autopilot.md` edit; the final full run was 259 passed, 0 failed). PostgreSQL ran for all.

| # | Mutant | Behavior | Survived | Caught by |
| - | ------ | -------- | -------- | --------- |
| 1 | `save.rs:91` text `could not be saved` to `was lost` | F2, FR-016 | No | `restart::a_failed_local_save_is_reported_on_stderr`, `store_slow::a_save_lost_...`, `restart::the_orchestrator_hosts_no_session_...` |
| 2 | `std::net::TcpStream::connect("1.1.1.1:1")` added in `db/mod.rs` | F1, FR-008 | No | `no_outbound::the_orchestrator_source_opens_no_outbound_connection` |
| 3 | `db/import.rs:82` `rows > 0` to `rows > 1000000` | US5.4, FR-010 | No | 3 tests, both backends and the start-level case |
| 4 | `db/migrate.rs:71` retry bound to `0` | M5 retry bound | **Yes, 2 of 4 runs** | `two_starters_...` only when the race is hit (N1) |
| 5 | `db/mod.rs:94` PostgreSQL target port forced to 5432 | F7 | No | 4 tests |
| 6 | `db/auth.rs:646` drop `AND redeemed_at IS NULL` (enrollment) | redeem once | No | 5 tests incl. `an_enrollment_is_redeemed_once_across_reopen` on both backends |
| 7 | `db/auth.rs:347` drop `AND redeemed_at IS NULL` (invite) | redeem once | No | 6 tests incl. `an_invite_is_redeemed_once`, `of_many_callers_...` |
| 8 | (restore check) full suite after the last restore | none | n/a | 259 passed |

Not sampled this run (carried from the previous report): `daemon_config` validation, the `[database]`
view, `reassign_unowned`, `choose_store`, owner preservation; the M11 log's own 17 mutants are taken
as written.

Coverage: not run (`cargo-llvm-cov` absent).

## Traceability

Unchanged except where noted. Every criterion US1.1 to US6.2 reaches a test; every cycle-log test name
was found in the tree except the two renamed or removed earlier. Start-level now: US5.2
(`store_config::a_second_start_imports_nothing_and_leaves_the_imported_file`) and US5.4
(`an_unimported_file_beside_rows_fails_the_start_naming_file_and_row_count`). US4.4 (network bind, no
`[database]`) is a `choose_store` unit test (`backend.rs:95`), accepted by the spec until #1732.
PostgreSQL rows (US1.1, FR-012, SC-004, FR-016 lost-database) executed here against a local server.
Untested criteria: none. Tests tracing to nothing: none.

## What was not audited

- The CI run that executes the PostgreSQL cases (run 43963469440) and the `ci.yml` service container
  beyond reading `ci.yml:303-320`; I ran the cases on a local PostgreSQL 16 instead.
- Mutation testing proper (no `cargo-mutants`); the 17 mutants in the M11 log were not re-run, only
  7 of my own plus a handful of repeats.
- Coverage; the full workspace suite (`mise run gate`); `mise run duplication` equality claims (F6).
- #1725 and other features' tests in the range; the client crate and docs.
- Performance, load, and a real slow database.
- Red-phase claims in the cycle log, including the M11 reds, taken as written except where re-run above.

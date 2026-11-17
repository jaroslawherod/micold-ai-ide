---
feature: 1948-orchestrator-postgres-store
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override, no preset)
verified_at: cc8816d9f # HEAD audited (M12 squash 97c75faea plus the ledger commit); re-verify of the FAIL at cae46cf
behaviors: 27 # 25 graded + 2 NOT_APPLICABLE; no tdd/test-list.md exists, so a "behavior" is one cycle-log entry / task group
proven: 11
likely: 0
test_after: 14 # class unchanged (history cannot change); F4 accepted by the owner on the recorded mutant reds, see "F4"
no_test: 0
high_smells: 0
criteria_total: 17 # acceptance scenarios US1.1-US6.2
criteria_covered: 17 # US4.4 unit-level only (spec accepts until #1732)
mutation_score: unmeasured # cargo-mutants not installed; 4 deliberate mutants this run, all caught (sample, not a score)
mutants_survived: 0
suite: "daemon `--test orchestrator` with PostgreSQL 16 (MICOLD_REQUIRE_POSTGRES=1): 271 passed, 0 failed, 0 ignored, 26 s on the first full run; 9 of the 10 full runs after it, under libtest parallelism, had 1 to 3 failures (N8), each passing alone. nextest absent."
---

# TDD Verification: PostgreSQL store for the orchestrator (#1948), re-verify after M12 (#2027, 97c75faea)

**Verdict: PASS_WITH_GAPS.** N1 to N7 are all resolved and every mutant caught, but the 14 `TEST_AFTER`
rows stay test-after by history: the verdict holds only because the owner accepted the recorded mutant
reds as the resolution of F4. Read literally, the rubric fails any `TEST_AFTER`.

Independence: files read cold in a session that did not write them. No fresh-context subagent is
available to this agent, so the smell pass was done by hand; every cited line was opened.

## F4: accepted exception

The 14 `TEST_AFTER` rows (4, 6, 7, 9, 11, 13, 15 to 19, 24 to 26) have no red at the time and the
history shows source before test. The re-verify instruction was to accept the after-the-fact mutant
reds recorded in `cycle-log.md` T064 and T068 as the resolution of F4. I applied that: F4 is closed, the
rows keep the class `TEST_AFTER` in the table (this audit does not rewrite history), and the verdict is
`PASS_WITH_GAPS` instead of `FAIL` on that basis only. The mutant reds are taken as written, not re-run
(except as in "Mutation results"). Row 16's survivor (the retry bound) is now killed (N1).

## N1 to N7 status

| # | Was | Now | Evidence |
| - | --- | --- | -------- |
| N1 | HIGH, retry bound survives its mutant on about half the runs | **Resolved** | `migrate::run_forward_with(migrator, attempt)` (`db/migrate.rs:78-95`); `run_forward` is `run_forward_with(.., \|\| migrator.run(pool))` (`:68`). Three cases in `store_migrations.rs:720-767` drive an attempt that loses the race `migrations + extra` times: finishes at `migrations + 2` attempts, gives up after `migrations + 2`, and a non-race error is tried once. Both bounds are pinned. Mutants: `retries = 0` fails the two retry cases on 3 of 3 runs (the two-starters case on 1 of 3), `retries = len()` fails both, `retries = len() + 2` fails the give-up case |
| N2 | MED, `waited < 9` asserts a value the test set | **Resolved** | the line and `waited` are gone (`store_slow.rs:175-189`); `record_when(b online, last_seen > before)` while the save is parked is the assertion and can fail if the heartbeat path blocks |
| N3 | MED, two eager substring cases | **Resolved** | 10 one-refusal cases in `store_sql_errors.rs:297-420` (the log says 11; I count 4 bootstrap, 4 invite, 2 enrollment), each `assert_eq!` against `db::refusal` (`db/mod.rs:31-61`). Mutant `auth.rs:443` `BOOTSTRAP_MISMATCH` to `BOOTSTRAP_CONSUMED` fails `consuming_another_code_than_the_one_set_is_refused_as_a_mismatch`. See N9 for the residual |
| N4 | MED, fixed 2 s then drop | **Resolved** | `starts` (`store_config.rs:245-262`) removes a stale socket, then selects the start against the socket appearing (20 ms poll, 60 s ceiling that only fails). `run` opens the store, importing, before `serve` binds (`orchestrator/mod.rs:141-154`), so the socket means the import is done. `store_config` passed in every full run here |
| N5 | LOW, undocumented password requirement | **Resolved** | documented at `store_support.rs:145-148`; the case itself still panics on a passwordless URL (the behavior was documented, not removed) |
| N6 | LOW, positional inserts | **Resolved** | all inserts in `store_sql_errors.rs:182-194,239-241` name their columns; no positional `INSERT INTO .. VALUES` left in the file |
| N7 | LOW, serde wording pinned | **Resolved** | `store_config.rs:163-169` pins the file name prefix and a non-empty cause |

## Test-first evidence

Same 27 rows and classes as the previous report. `PROVEN` rows (2, 3, 5, 8, 10, 12, 14, 20, 21, 22, 23):
red recorded and test and source in one commit. `TEST_AFTER` rows: see F4. Rows 1 and 27 are
`NOT_APPLICABLE`. M12 behaviors (T071 to T075) each have a cycle-log entry with a mutant red
(T071 `retries = 0`, 5 of 5 per the log, 3 of 3 here); the retry cases land with the source change in
one commit (`97c75faea`), so T071 is `PROVEN` by the same rule as the others.

Log versus history: no contradiction. One small one: the log and commit say 11 refusal cases, the file
holds 10.

### Changes to tests that already existed (M12 range `97c75faea^..97c75faea`)

- `store_slow.rs:188-189`: `waited < 9` removed. It asserted a constant (see N2), so nothing real was
  lost.
- `store_config.rs:163-169`: exact error string loosened to prefix plus non-empty cause. Justified
  (framework text, N7); the file-name part is still exact.
- `store_config.rs:245-262`: fixed timeout replaced by a socket wait. Strictly stricter (a start that
  returns early fails).
- `store_sql_errors.rs`: `contains(..)` replaced by `assert_eq!` on exact reasons, split into cases.
  Stricter.
- No test ignored, renamed out of a filter, or excluded. `.config/nextest.toml` adds a cap of 8 threads
  for the `orchestrator` binary (a limit on parallelism, not a skip). No coverage or mutation threshold
  touched.
- `tasks.md`: T071 to T075 are `[x]`, each with a cycle-log entry. No behavioral task unticked.

## Findings

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| N8 | MED | Under plain `cargo test` (the profile's suite command) the orchestrator binary is not reliably green on this box. At default parallelism 1 full run passed (271) and the next 4 failed with 1 to 3 PostgreSQL cases: `pool timed out while waiting for an open connection` (`store_support.rs:195`, `store_suite.rs:56,136`). At `--test-threads=8`, 5 of 6 runs failed, 4 on `restart::the_orchestrator_hosts_no_session_and_writes_only_its_registry` (`child_count()` read 1, expected 0) and 1 on the pool timeout. Each failing case passes alone. CI runs nextest with the M12 cap (`.config/nextest.toml`) and one process per test, which this box cannot run (no `cargo-nextest`), so whether CI is affected is not shown here. Cause not diagnosed: the child count is process-global and only tests that take `children_quiet()` are protected (`support.rs:420`); a test that spawns a child without the guard breaks it. | `restart.rs:84`; `store_support.rs:195`; `support.rs:420-442`; `.config/nextest.toml` |
| N9 | LOW | The refusal cases assert against the constants the source uses, so a reworded reason changes source and test together and nothing pins the wording. Intended by N3's remediation and distinct reasons are still told apart (mutant above), but a wrong wording ships unnoticed. | `store_sql_errors.rs:297-420`; `db/mod.rs:31-61` |
| N10 | LOW | The M12 log's "11 cases" does not match the 10 in the file. | `cycle-log.md` T073; `store_sql_errors.rs` |

Other properties: tests are isolated (a tempdir or a database per case), the orchestrator target runs in
20 to 30 s, race cases now assert an invariant on demand (N1). No HIGH smell found in the M12 files:
no assertion-free, tautological, vacuous or conditional-assert test. The `attempts_after` helper stubs the
migrator attempt, but the subject under test is the retry loop, not the migrator, so it is not a doubled
subject; the real migrator path stays covered by `two_starters_...` and the migration cases.

## Mutation results

Tool: none (`cargo-mutants` absent). Score **unmeasured**. 4 deliberate mutants this run on the M12
source, one at a time, each restored with `git checkout -- <file>`; `git status` was clean after each and
after the last, and the full orchestrator suite ran green (271 passed) on the restored tree.

| # | Mutant | Behavior | Survived | Caught by |
| - | ------ | -------- | -------- | --------- |
| 1 | `db/migrate.rs:83` `migrations.len() + 1` to `0` (3 runs) | N1 retry bound | No | `a_starter_that_loses_..._still_finishes`, `a_starter_that_keeps_losing_..._gives_up_...` on 3 of 3, plus `two_starters_...` on 1 of 3 |
| 2 | same line to `migrations.len()` | N1 lower bound | No | both retry cases |
| 3 | same line to `migrations.len() + 2` | N1 upper bound | No | `a_starter_that_keeps_losing_..._gives_up_...` |
| 4 | `db/auth.rs:443` `BOOTSTRAP_MISMATCH` to `BOOTSTRAP_CONSUMED` | N3 refusal | No | `consuming_another_code_than_the_one_set_is_refused_as_a_mismatch` (a `postgres_holds_no_key_material` failure in that run was the N8 pool timeout, not the mutant) |
| 5 | (restore check) full suite on the restored tree | none | n/a | 271 passed |

Not
sampled: `daemon_config` validation, the `[database]` view, `reassign_unowned`, `choose_store`, owner
preservation, the `starts` socket wait (N4), and the M11 log's 17 mutants, taken as written. Coverage not
run (`cargo-llvm-cov` absent).

## Traceability

Unchanged from the previous report: every criterion US1.1 to US6.2 reaches a test, US5.2 and US5.4 at
start level (`store_config`), US4.4 as a `choose_store` unit test (`backend.rs:95`, accepted by the spec
until #1732). The 10 refusal cases trace to FR-016 and the auth store contract. PostgreSQL rows executed
here against a local PostgreSQL 16 server. Untested criteria: none. Tests tracing to nothing: none.

## What was not audited

- CI: the run that executes the PostgreSQL cases under nextest, and whether N8 shows there. This box has
  no `gh`, `mise` or `cargo-nextest`; I ran PostgreSQL 16 locally (`postgres` role password set to a dummy
  value for the run).
- Mutation testing proper (no `cargo-mutants`), coverage, the full workspace suite (`mise run gate`) and
  `mise run duplication`; the 14 `TEST_AFTER` mutant reds and the M11 17 mutants as written.
- The other crates, the client, docs, and other features' tests in the range (#1735, #1981).
- Performance, load, and a real slow database.
- Red-phase claims in the cycle log, taken as written except the N1 mutant re-run above.

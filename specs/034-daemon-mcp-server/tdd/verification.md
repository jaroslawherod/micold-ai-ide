---
feature: 034-daemon-mcp-server
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override, no preset)
verified_at: 68e48320
behaviors: 262 # test-list rows: 256 DONE, 5 BASELINE (not applicable), 1 BLOCKED (U127, known)
proven: 97
likely: 139
test_after: 20
no_test: 1 # U127, BLOCKED and already recorded
high_smells: 8
criteria_total: 22
criteria_covered: 22
mutation_score: unmeasured # no cargo-mutants; 4 deliberate mutants, 4 caught (not a score)
mutants_survived: 0
suite: 361 passed, 0 failed, 0 ignored, 22 test binaries run one at a time; wall 1257 s (almost all queueing on the shared build lock; summed test time ~35 s)
---

# TDD Verification: The session service exposes an MCP server

**Verdict: FAIL.** Rubric fails on any `TEST_AFTER` behavior (20, admitted in the cycle log) and any `HIGH` smell (8, all vacuous or conditional assertions). Strength is good (4 of 4 mutants caught) and every acceptance scenario has a real-entry-point test. The audit was independent of the session that wrote the tests; the smell pass was done by two fresh-context subagents and every cited line was opened and checked.

## Test-first evidence

Rebase-merge kept commit order. Class was taken from the cycle log red plus `git log origin/main` order (cycle-to-id mapping parsed from cycle headers; cycle 35 ids from its `red:` commit list).

| Behaviors | Class | Evidence |
|---|---|---|
| 97 (cycles 27-32, 35-38: M4 core+daemon, M5, M6) | PROVEN | red recorded; a `test(034): ... red` commit precedes the source commit (e.g. `f136feef` then `47d58f4e`, `c66c5c06` then `746cfa6c`, `5fb65ff8` then `49f56a52`, `624f3e77` then `cdf18f5a`, `4961e682` then `6070a3ed`) |
| 139 (cycles 1-7, 9-26, 40: M1-M3, M2 toggle) | LIKELY | red recorded (several against stubs of a drafted implementation, cycles 1, 2, 5); test and source land together in one `feat(034)` commit, so order is not verifiable |
| 20: A2, A3, U124-U126, U128-U140 (cycle 8), U206 (cycle 39), U147 (cycle 41) | TEST_AFTER | log says "red: none observed. Test-after admission" (cycle 8) and "no red" (39, 41). Cycle 8 is compensated by 4 recorded mutants; 39 and 41 are not |
| 1 | NO_TEST | U127 `locked` BLOCKED (known) |
| 5 (U1-U3, U34, U141) | NOT_APPLICABLE | characterization baselines |

Also test-after, outside the id table: `a_collision_in_the_copilot_home_the_session_environment_sets_is_found` (cycle 10, mutant recorded). Cycles 33 and 34 have no separate red; red is "changed assertions" and "same commit" (`be70ca3f`, `8e2052f9`), which is LIKELY at best.

Existing tests: none weakened by feature 034. Changes found in the range: `protocol_auth.rs` literal pin `PROTOCOL_VERSION == 17` removed in `1ef91637` and consolidated into `schema_hash.rs` (a pin still exists; INV-2 holds); `features_settings.rs`, `icons.rs`, `tooltip_clears_its_row.rs` edits belong to features 037 and others. No `#[ignore]`, exclusion, or threshold change added.

tasks.md vs list: T015 and T021 stay `[ ]` only because U127 is BLOCKED (known). No `[X]` task with a non-DONE behavior.

## Findings

| # | Sev | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Only assertion for host-loopback isolation is inside `if connect().is_ok()`; in a normal run (connection refused) nothing is asserted. Should assert the connect fails | `crates/micold-daemon/tests/sandbox_real_mcp.rs:262-275` (off by default) |
| 2 | HIGH | `field(line,"target").is_some()` is vacuous; should equal the expected worktree or session | `mcp_audit_log.rs:309` |
| 3 | HIGH | Loop over `mutating_tools` x `failing_call` has no non-empty guard (the success test has one at 293); the six-category test accepts `outcome == "ok"` for a failed call and discards the expected category | `mcp_audit_log.rs:317-328`, `403-418` |
| 4 | HIGH | `tools/list` asserted only non-empty; should pin the tool-name set | `mcp_binding_spawn.rs:242` |
| 5 | HIGH | `!target_label.is_empty()` for the confirmation prompt; should equal the target's label | `mcp_lifecycle_tools.rs:741` |
| 6 | HIGH | `initialize` instructions asserted only non-empty | `crates/micold-core/tests/mcp_jsonrpc.rs:108-113` |
| 7 | HIGH | every tool description asserted only non-empty | `crates/micold-core/tests/mcp_tools_catalog.rs:111-114` |
| 8 | HIGH | `if DESTRUCTIVE.contains(..)` inside a loop decides the assertion; redundant with 115-119 | `mcp_tools_catalog.rs:335-340` |
| 9 | MED | 20 TEST_AFTER behaviors; U206 and U147 have no mutant evidence | cycle log 8, 39, 41 |
| 10 | MED | Duplicate ids: U220-U222 and U230-U232 each name two different behaviors, so a trace to those ids is ambiguous | `tdd/test-list.md:198-200` and `440-442`, `450-455` |
| 11 | MED | Fixed sleeps and wall-clock bounds: `sleep(1000ms)` before interrupt; 3 s negative sleep; `elapsed() < N` asserts; 1 s latency bound on a loaded machine | `mcp_lifecycle_tools.rs:515`, `mcp_create_session.rs:415,419-423,559-565`, `mcp_cross_session.rs:232`, `mcp_read_latency.rs` |
| 12 | MED | `Env` guard hand-copied in three files instead of one helper in `tests/support/` | `mcp_binding_spawn.rs:43`, `mcp_create_session.rs:49`, `mcp_cross_session.rs:65` |
| 13 | MED | `if cfg!(windows)` branch is dead on unix (`assert_eq` before it is the real check) | `mcp_read_tools.rs:436` |
| 14 | MED | Policy tests compare `decide()` to `decide()` for another caller and pin no absolute `Proceed` for `GetSession`/`Whoami`; allow and deny confirm tests assert the same state | `mcp_policy.rs:181-215,405-447`; `features_agent_confirm.rs:67-91` |
| 15 | LOW | Commit-tag collision: another feature also committed as `(034)` (GitHub issues, own U-ids), so history greps by tag mix two features | `git log --grep='(034)'` |
| 16 | LOW | Foreign style: most assertions in core mcp tests have no rule message; `core/src/mcp/*.rs` has no `#[cfg(test)]` block (all in `tests/`) | profile conventions |

Pre-feature smells in `settings_roundtrip.rs:108,193` and `features_settings.rs:674` (blame: before 034 or other features) are out of scope and not counted.

## Mutation results

Sample of 4 behaviors, one mutant each, no tool; restored with `git checkout` and `git status --short` clean afterwards.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `policy.rs` `if caller.is_default()` to `if false && ..` (Principle III refusal) | A18, U104 | No | 3 core tests (`mcp_policy`) and 2 daemon tests (`mcp_lifecycle_tools`) fail |
| `server.rs` unknown or missing bearer falls back to a nil session | U18-U20, U27, U28 (SC-006) | No | 7 of 17 `mcp_endpoint` tests fail |
| `confirm.rs` deadline `+ 1 s` | A23, U172-U185 | No | 2 `mcp_confirmations` tests fail (prompt's time-left); the expiry tests at 229/249 did not fail, so the expiry itself rides on a separate timer |
| `tools.rs` `resolve_session_target` searches every project | A21, FR-010 | No | `another_projects_session_or_worktree_is_not_found...` fails; only that one test |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 AS1-AS6 | A1-A6 | Yes (real `POST /mcp`, stand-in CLI spawn) |
| US2 AS1-AS5 | A7-A11 | Yes |
| US3 AS1-AS6 | A12-A18 | Yes |
| US4 AS1-AS5 | A19-A23 | Yes |

22 of 22 scenarios covered (23 behaviors, US3-AS2 has two). All `test` names in `traces` resolve to existing functions or files (heuristic name check over 256 DONE rows: 0 dangling). Real-CLI half is quickstart §B (not rerun). Untested criteria: none, except U127 (`status: locked`, known). Tests tracing to nothing: not exhaustively checked.

## What was not audited

- No mutation tool: strength is a 4-mutant sample, not a score; U206, U147, and the client dialog tests were not mutated.
- `sandbox_real_mcp.rs` was not run (needs a container runtime, off by default); read only.
- Whole workspace suite not run (instruction); only the 22 feature binaries plus no `#[cfg(test)]` lib blocks (`http.rs` was read, not run).
- Unix-only spawn and lifecycle tests were run on Linux only; Windows and macOS paths unverified.
- Coverage and performance: no tool, not assessed; the 1 s latency bound ran once.
- Per-behavior git ordering for LIKELY rows could not be corroborated (combined commits).

---
feature: 1813-supervisor-skill
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset copy found; extension copy graded against
verified_at: 2097a8eae
behaviors: 15 # derived from tasks.md test tasks and review-fix rounds; no tdd/test-list.md exists
proven: 7
likely: 0
test_after: 8
no_test: 0
high_smells: 1
criteria_total: 16 # 5 stories' acceptance groups (US1-US5) + FR-001..FR-011 grouped, see Traceability
criteria_covered: 16 # all have at least one test; 6 only by skill-text grep, see finding 3
mutation_score: 100 # 16 of 16 deliberate mutants caught (one by suite hang); NOT a tool score, sample only
mutants_survived: 0
suite: 179 passed, 0 failed, 9s (bash scripts/tests/supervisor.test.sh)
---

# TDD Verification: Autopilot supervisor skill (#1813)

**Verdict: FAIL.** Eight behaviors (all of M1, and the M1/M2/M3 review-fix rounds) have no recorded red run, so they are `TEST_AFTER` by the fail-closed rule; the suite itself is strong (16 of 16 deliberate mutants caught).

Audit independence: this audit did not write the tests; it read every file cold. No fresh-context subagent was available in this environment, so the smell pass was done directly.

## Preflight notes

- `.specify/feature.json` points at `specs/1857-worktree-parent`; the feature directory was taken from the explicit input (`specs/1813-supervisor-skill`), not from the resolver.
- `tdd/test-list.md` and `tdd/cycle-log.md` do not exist: the feature was not planned through the TDD extension. Behaviors were derived from the `tasks.md` test tasks (T002-T022) and the review-fix commits; red evidence comes from the ledger's "Red runs" section (`specs/1813-supervisor-skill/autopilot.md`), which is self-reported.
- Git cannot show ordering: every commit adds test and source together (`f8d79470a`, `a6b47dde3`, `3ea92d58d`, `ca82abd84`), so history neither proves nor contradicts the ledger. No ledger claim is contradicted by history.
- Profile (`.specify/memory/tdd-profile.md`) describes a Rust stack only. It has no shell test kind, so `Foreign style` and `Bypassed test utility` were judged against the shell convention in the repo: `scripts/tests/lib/autopilot-fixtures.sh` (`check`, `tmp`, counters), which the file reuses. No finding.
- Suite: `bash scripts/tests/supervisor.test.sh`, 179 cases, 0 failures, 9 s, at start and after all mutants.

## Test-first evidence

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| B1 gitignore case (T002) | TEST_AFTER | no M1 red recorded in the ledger; `f8d79470a` adds test and source together |
| B2 ledger init/owner/set/get/table (T003) | TEST_AFTER | same, M1 |
| B3 `next` under a cap (T005) | TEST_AFTER | same, M1 |
| B4 gate (T011) | TEST_AFTER | same, M1 |
| B5 skill-text supervised marker (T007) | TEST_AFTER | same, M1 |
| B6 M1 review round 1 hardening (non-numeric gate input, option-without-value, duplicate issues, partial ledger) | TEST_AFTER | `1b47d969d`; no red recorded |
| B7 map, retry, check-owner (T014) | PROVEN | ledger: 45 failures, `unknown subcommand map\|retry\|check-owner`; test and source in `a6b47dde3` |
| B8 watcher (T016) | PROVEN | ledger: exit 127 `supervisor-watch.sh: No such file or directory` |
| B9 skill-text restart/report/watcher (T018) | PROVEN | ledger: grep exit 1 on the listed phrases |
| B10 M2 review round 1 (recovery order, watcher interval validation) | TEST_AFTER | `92ed01eab`; no red recorded |
| B11 touches/overlap/next-holds (T019) | PROVEN | ledger: 29 failures, `unknown subcommand touches\|overlap` |
| B12 M3 review round 1 (CRLF, globs, bulleted key) | TEST_AFTER | `b3feeb311`; no red recorded |
| B13 small-context skill-text (T022) | PROVEN | ledger: 2 failures, grep exit 1 |
| B14 empty `Touches:` line skipped | PROVEN | ledger records the before-fix output `-`/`none` |
| B15 M4 review round 1 (4 skill-text cases) | PROVEN | ledger: 4 failures before the fix |

`NO_TEST`: none. Caveat on `PROVEN`: the red records are one paragraph per milestone in a hand-kept ledger, with counts but without per-case output; they were not independently reproducible here.

## Existing tests changed by the feature

Every removed assertion line in `scripts/tests/supervisor.test.sh` history was read. None was weakened, skipped, filtered out or renamed out of reach; no threshold or exclusion exists in this suite.

| Commit | Removed | Verdict |
| --- | --- | --- |
| `d7dea746e` | `every run-status.sh read is --json` (vacuous), `read is written to ... j.json` | replaced by `:244-248`, stricter (per-issue file, once-named count) |
| `1b47d969d` | `grep 'gate --running'` | replaced by an order check, gate before `create_worktree` (`:164`), stronger |
| `4e24482ca` | `Supervised: yes` line asserted in `speckit-autopilot/SKILL.md` | moved to `rules/supervised.md` (`:160`); the literal is also checked against `supervised.sh` (`:161`) |
| `92ed01eab` | stub `chg` fixed `sleep 0.5`; case name `... dead-no-ledger` | replaced by a condition-based wait (`:143`); same grep kept under a new name (`:175`) |

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Eight behaviors have no red evidence (B1-B6, B10, B12): test-after under the fail-closed rule. M1 is the foundation (ledger, `next`, gate); the ledger's Red runs section starts at M2. | `autopilot.md` "Red runs"; commits `f8d79470a`, `1b47d969d`, `92ed01eab`, `b3feeb311` |
| 2 | HIGH | Vacuous assertion: "the machine is read without overrides" accepts `PASS` or any `FAIL memory\|load`. An `unreadable` reading (broken `/proc/meminfo` or `uptime` parse) prints `FAIL memory: unreadable` and passes. It asserts the real readers return numbers in no way. Should assert the exit and that output does not contain `unreadable` (or test `mem_mib`/`load1` output is numeric). | `scripts/tests/supervisor.test.sh:81`; `scripts/autopilot/supervisor.sh:174-190, 214` |
| 3 | MED | Six acceptance groups are verified only by grepping `SKILL.md` for phrases: US1.3 (`autopilot:queue`), US1.4 (`create_worktree`/`create_session` calls), US4.3 (`list_sessions` liveness), US5.2, FR-001, FR-003, FR-009. No test exercises the skill's actual MCP flow, and quickstart Part B (live MCP calls, "run by the close unit") has no recorded result. Text greps prove the words exist, not that the instructions work. | `:159-178, 236-257`; `tasks.md` Implementation Strategy; no Part B record in `autopilot.md` |
| 4 | MED | Re-implemented expectation plus conditional logic: expected count `3 - occ`, expected gate outcome and exit code are computed in the test with the same cap-minus-occupied rule `next` uses. | `:260-266` |
| 5 | MED | Timing-dependent watcher cases: real 1 s intervals and 1-5 s ceilings with a background flipper; `CEILING` cases depend on wall clock. Slow or loaded CI can flip the result. Also the background `chg` loop (`:143`) polls forever if the stub is never called twice, so a regression in the watcher makes the suite hang, not fail (observed: mutant 16 below hung the run past 300 s). Needs a bounded wait. | `:141-152`, `:143` |
| 6 | MED | Skill-text greps pin exact phrases (`without reading the session`, `never read a session`, `only through ...`, `Finish with ...`); a harmless rewording breaks them (implementation coupled), and several (`:173` `starting.*empty`) would match unrelated text. | `:244-258`, `:173` |
| 7 | MED | Untested watcher/ledger edges: an unreadable run-status (`unreadable` state in `reading`), a starting/running row with no Worktree (skipped by `watched`), `ledger init --grace/--source`, `touches --labels` with a non-`flow:` label. | `supervisor-watch.sh:324-327, 322`; `supervisor.sh:43-48, 142` |
| 8 | LOW | `:49` wraps a pipeline in `echo "$(...)"` for no reason, duplicating the `nx` helper (`:18`) and the `bash -c` pipe on `:51`; `:190` re-checks CR absence with `od` inside the same `check`. Duplicated setup; helper `nx` unused in the early cases. | `:18, 49, 51, 190` |

Smells checked and not found: assertion free (every `check` asserts exit and output), tautological or doubled subject (no stubs of `supervisor.sh`; the only stub is `run-status.sh`, a collaborator), self-approving snapshot (none), empty or skipped tests (none), mystery guest (fixtures built per case in `mktemp`), order dependence (cases build their own ledger with `mk`).
Properties: isolated per-case temp ledgers; specific failure names; 9 s total; determinism weakened only by finding 5.

## Mutation results

No mutation tool (profile: `mutation: null`). Deliberate mutants, 16 sampled on the highest-risk behaviors (cap and slot counting, gate boundaries, duplicate-start protection, retry limit, overlap sequencing, state mapping, watcher). One at a time, restored from a byte-exact copy and verified with `cmp` plus a green full run afterwards.

| Mutant (`supervisor.sh` unless noted) | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `next` ignores occupied rows (`:151`) | B3, SC-001 | No (4 failures) | pinned |
| gate `-ge` to `-gt` for cap (`:212`) | B4 | No (5) | boundary pinned |
| gate memory `-lt` to `-le` (`:215`) | B4 | No (1) | boundary pinned |
| gate load `>` to `>=` (`:218`) | B4 | No (1) | boundary pinned |
| gate default min memory 4096 to 1024 (`:193`) | B4 | No (1) | default pinned |
| `map` grace `-le` to `-lt` (`:250`) | B7, FR-011 | No (1) | boundary pinned |
| `map` `blocked` to `lost` (`:251`) | B7 | No (1) | pinned |
| `retry` limit 3 to 4 (`:261`) | B7 | No (2) | pinned |
| `occupied` drops `starting` (`:33`) | B3, FR-008 | No (4) | pinned |
| `check-owner` OWN to FREE (`:278`) | B7, SC-002 | No (1) | pinned |
| `overlap` containment one-way (`:115`) | B11, FR-004 | No (5) | pinned |
| `next` queued hold `-lt` to `-gt` (`:164`) | B11 | No (3) | pinned |
| `ledger set` State validation dropped (`:76`) | B2 | No (1) | pinned |
| `touches` declared line ignored (`:140`) | B11 | No (6) | pinned |
| watcher change test inverted (`supervisor-watch.sh:338`) | B8 | No (4) | pinned |
| `ledger set` writes column +2 (`:80`) | B2 | Caught by hang | the suite did not fail, it hung (finding 5); counted as caught only because the run never completed |

Score: 16 of 16 caught (100 %), sample, scope `scripts/autopilot/supervisor.sh` and `supervisor-watch.sh`, hand mutants (no tool version). Not a measured mutation score. Survivors: none; so no survivor inside a `DONE` behavior.

## Traceability

| Criterion | Tests | Real entry point |
| --- | --- | --- |
| US1.1-2, SC-001, FR-008 (cap, ascending, slots) | `:48-63`, `:259-266` | Yes (`supervisor.sh next`) |
| US1.3 (label queue), FR-001 | `:159-165` grep only | Text only (finding 3) |
| US1.4, FR-003 (worktree then session) | `:162-164` grep only | Text only |
| US2.1-4, SC-003, FR-002 (gate) | `:66-89` | Yes |
| US3.1-3, FR-004 (overlap) | `:181-233`, `:236-240` | Yes (CLI); skill step text only |
| US4.1-2, FR-005, FR-006, FR-011, SC-002 | `:97-127`, `:167-176` | Yes (CLI); recovery flow text only |
| US4.3 (worktree gone, `list_sessions`) | `--exit 2\|3` `:109-110`; `list_sessions` grep `:170` | Partial: mapping tested, liveness check is text |
| US4.4 (dead-no-ledger within Grace) | `:101-104` | Yes |
| US5.1, FR-007, SC-004 | `:244-258`, `:163` | Text only (the property is about skill text) |
| US5.2, FR-009 (report asking runs) | `:175-176`, `:256` | Text only |
| FR-005 (ledger records), FR-010 (reproducible decisions) | `:24-63`, `:259-266` | Yes |

Untested criteria: none by count; four are text-only. Tests tracing to nothing: none (watcher and `map --exit` serve FR-006/FR-009/FR-011). No test-list `traces` column exists to verify.

## What was not audited

- No `tdd/test-list.md` or `tdd/cycle-log.md`: behavior list is derived, red evidence is the ledger's self-reported paragraph.
- Git cannot show test-before-source (test and source always share a commit).
- Mutation: 16 hand mutants on two files, not a tool run; `touches` inference by flow label, `gate` Darwin readers (`vm_stat`, `sysctl`), `ledger table` formatting and `supervise.sh` argument edges were not mutated.
- `.claude/skills/autopilot-supervisor/SKILL.md` content correctness and the MCP flow (`create_worktree`, `create_session`, `list_sessions`) were not exercised; no `mise`, `gh` or MCP available; quickstart Part B not run.
- Coverage: profile has none for shell.
- Performance beyond the 9 s run, and macOS (bash 3.2) behavior: not assessed.
- `.claude/skills/speckit-autopilot` edits from T008 were only checked through the greps in the suite.

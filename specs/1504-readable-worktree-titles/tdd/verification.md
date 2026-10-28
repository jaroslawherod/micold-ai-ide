---
feature: 1504-readable-worktree-titles
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: f210a3933
behaviors: 47 # task-level: T002-T046 and T049 test/impl tasks; there is no tdd/test-list.md
proven: 0 # no cycle-log exists, so PROVEN (red command in a cycle log) cannot be reached
likely: 23 # M2 (T017-T020, T049), M3 (T025-T027), M4 (T033-T036 + F1-F3), M5 (T042-T043): red recorded in the ledger, history cannot corroborate (test and source in one commit per milestone)
test_after: 24 # M1 (T002-T005, T009-T012; 8 test tasks) have no red recorded; plus round-1 fix tests without a separate red (see findings 1-2)
no_test: 0
high_smells: 0 # smell pass NOT exhaustive, see "What was not audited"
criteria_total: 14 # US1 x4, US2 x7, US3 x3 scenarios (spec.md); FRs/SCs traced through tasks.md
criteria_covered: 14
mutation_score: unmeasured # no mutation tool in the profile; 3 deliberate mutants, 3 caught
mutants_survived: 0
suite: feature subset only, 0 failed (core lib 72, review_git/git_fake/jira_parse 87, client 11 integration targets 358, bin title/fingerprint 19), about 7 min including cold build; full workspace suite not re-run (ledger: scripts/gate.sh exit 0 at 1e4021a18)
---

# TDD Verification: Readable worktree titles (#1504)

**Verdict: FAIL.** M1 (the chooser, `title_ticket`, `worktrees_fingerprint`, T002-T016) has no red run recorded anywhere in the repository (no cycle log, no ledger section), and its tests landed in the same commit as the code, so by the fail-closed rule those behaviors are `TEST_AFTER`. The tests themselves are strong: every sampled mutant was caught and no existing test was weakened.

Audit independence: this audit was run by a fresh agent that did not write the feature; files were re-read as they stand.

## Test-first evidence

Evidence sources: no `tdd/cycle-log.md` and no `tdd/test-list.md` (feature not planned through the tdd extension). Substitute: the "Red runs" sections of `autopilot.md`, the per-milestone commits, and `tasks.md` ("tests first" per phase). Git cannot corroborate order: each milestone is one `feat` commit holding tests and source together (`c29eaa82a` M1, `452902b24` M2, `a2212956c` M3, `b7c7163ca` M4, `87da4516b` M5), so nothing here is `PROVEN`.

| Milestone / tasks | Class | Evidence |
| --- | --- | --- |
| M1 T002-T005, T009-T012 (chooser, `first_line`, `title_ticket`, fingerprint, card/row agreement) | TEST_AFTER | No red recorded. Ledger line 65 says "red runs named in the PR body"; the PR body is outside the repo and could not be read (`gh pr view 1566` returned nothing) |
| M1 review A fixes F1-F3 (`00825d3e0`) | TEST_AFTER | "L2 rows and tests added", no red recorded |
| M2 T017-T020 | LIKELY | Ledger M2: compile reds (E0061, E0599, E0609), not assertion reds; T019 `fields` assertion and T018 gate test never reached an assertion red |
| M2 T049 | LIKELY | Ledger: three named exact-text tests red, exit 101 |
| M3 T025-T027 and review fixes | LIKELY | Ledger M3: exit 101 with left/right values per test; review-fix reds recorded |
| M4 T033-T036 and review A F1-F3 | LIKELY | Ledger M4: exit 101, failing assertions with values, stubs named |
| M5 T042, T043 | LIKELY | Ledger M5: exit 101, left/right values |
| M5 review round 1 tests (A1 switch-off, A2 clock back, A3 seed, A4 outage, B2 NotOpenIssue/overflow) | TEST_AFTER | Ledger line 104-106 states they were written with the fix, no red. B1 has an after-the-fact red by stubbing (LIKELY) |
| T001, T006-T008, T013-T016, T021-T024, T028-T032, T037-T041, T044-T046 | n/a | Implementation and doc tasks, graded through their test tasks |

Existing tests changed by the feature (checked with `git log -p` over the 1504 commits): none weakened.
- `tests/overview.rs` `TitleLookup::Unavailable` assertions replaced by `NoIssue`/`Failed` (87da4516b): required by the spec change (FR-009), stricter, not looser.
- `tests/jira_http_adapter.rs`: `["status"]` to `["status","summary"]`, `got[0].0/.1` to `.key/.status` (633795c1f): type and contract change (L6).
- `tests/features_overview.rs` `facts_round == 0` after a project switch to `1` (7bc13d200): documented review fix (M4 F2), with a new test for the stale-answer case.
- `features_sidebar.rs` Branch-title `with_title_line` assertions (dd7d3e2d0): behavior change from M3 review B F1, test rewritten, not removed.
- M5 `a clock that went back asks nothing` replaced by `... is due, not stuck` (a18889f77): a test of this same feature, changed together with the A2 fix; no pre-existing test.
No skips, `#[ignore]`, exclusions or lowered thresholds found in the 1504 diff. The `layout_snapshot` regeneration (M2, add-worktree-dialog-jira states, 16 px taller) is recorded in the ledger with its reason and affects only those states.

`tasks.md` against the ledger: every task is `[X]` except T050. The ledger header says T050 is done and the quickstart Results say M3-M5 visual passes are "run by the orchestrator", with none recorded in the repo (ledger lines 74, 78 still `pending`).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | M1 behaviors have no recorded red; classed `TEST_AFTER` (fail closed). Cleared by pasting the M1 red commands and failures from PR #1566 into `autopilot.md`, if they exist | `autopilot.md:65`, no "Red runs (M1)" section; `c29eaa82a` |
| 2 | HIGH | M5 round-1 fix tests (A1-A4, B2) and M1 review-fix tests (F1-F3) were written with the fix, no red | `autopilot.md:104-106`; `00825d3e0` |
| 3 | MED | No cycle log or test list; the evidence is self-reported prose in the ledger, and milestone commits bundle tests with source, so no behavior can be `PROVEN` | `specs/1504-readable-worktree-titles/` has no `tdd/` files before this report |
| 4 | MED | M2 reds are compile errors only; T018's "nothing sent with the switch off" and T019's `fields` assertion never failed on an assertion | `autopilot.md:196-206` |
| 5 | MED | T050 unticked, and the visual passes for M3 (E, G, I, folder-titled, renamed), M4 (F, H, Refresh) and M5 (Retry) are not recorded as done; the ledger header and `tasks.md` disagree | `tasks.md:297`, `autopilot.md:12,74,78`, `quickstart.md:59-60` |
| 6 | LOW | SC-005 (no read on the UI thread) has no test of its own; only the design (`spawn_blocking` with `READ_DEADLINE`) backs it | `tasks.md` T040; no test task names SC-005 |

## Mutation results

No mutation tool in the profile (`mutation: null`). Three deliberate mutants, one at a time, each restored with `git checkout` and the tree confirmed clean, then the affected tests re-run green.

| Mutant | Behavior | Caught | By |
| --- | --- | --- | --- |
| `location_title.rs:165` drop the session source from `choose` | T003/T026 source order | Yes | `each_source_wins_over_every_lower_one_in_order` |
| `features/overview.rs:705` retry `>=` to `>` (boundary, FR-009) | T042 | Yes | `titles_to_fetch_re_asks_a_failed_read_only_after_the_retry_interval` |
| `review/git.rs:532` drop `--no-merges` (own commit, L7) | T034, M4 F1 | Yes | `a_merge_from_the_default_branch_is_not_the_own_commit_subject` |

Sample of 3 behaviors out of about 25 distinct logic behaviors, so strength is only spot-checked, not measured.

## Traceability

Mapped from the task text in `tasks.md` and confirmed by running the named test targets (all green); individual test bodies were read only where noted under "not audited".

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1, US1.3 (GitHub ticket in branch) | T009-T012: `title_ticket` table, `titles_to_fetch`, fingerprint, card and row agree | Yes, `State`-level in `tests/overview.rs` and `features_sidebar.rs` |
| US1.2, US1.4 (Jira summary, fall-through) | T017-T020, T049 | Yes, `jira_http_adapter` and `main_tests` jira driver |
| US2.1, 2.3, 2.5, 2.7 (PR, session, branch/folder, follows sources) | T025-T027 | Yes, `features_overview.rs` |
| US2.2, 2.4 (spec heading, own commit) | T033-T036 | Yes, `title_facts_*` through `update`, plus `GitCli` on a temp repo |
| US2.6 (source in tooltips) | T027 | Unit on three surfaces; layout/visual pass for M3 not recorded (finding 5) |
| US3.1-3.3 (retry after 5 min) | T042, T043 | Yes, `title_answer_*` through the shell mapping |
| SC-004 (no new network occasions) | T047: the two `*_requested_only_on_named_events` guards | Yes, pass |
| FR-011 (no title text in `Debug`) | T004, T017, T035 | Unit |

Untested criteria: SC-005 (finding 6). Tests tracing to nothing: none seen.

## What was not audited

- The smell pass is not exhaustive: about 1,700 lines of new tests across ten files were not read line by line against the 21-item catalogue. No fresh-context subagent was used. Zero HIGH smells is "none found while reading the changed assertions and the mutated paths", not a clearance.
- `helpers` and exemplar conformance (Foreign style, Bypassed test utility) were not checked file by file.
- Mutation: 3 deliberate mutants only, no tool, no coverage (`coverage: null`).
- The full workspace suite, clippy and fmt were not re-run (ledger: `scripts/gate.sh` exit 0 at `1e4021a18`; the close commit `f210a3933` changes docs and specs only).
- The PR bodies (#1566 onward) were not readable here; they may hold the M1 red runs.
- Performance, flakiness and Windows/macOS legs. The ledger records unrelated flakes on the shared target during the M5 gate (`capture-harness`, `guide-own-image`, `structured_permission`), each passing alone.
- Remediation tasks were not appended to `tasks.md`: the requester asked for the report and commit only.

---
feature: 552-native-agent-session-signals
verdict: PASS_WITH_GAPS
verified_at: 0d0655eb
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
independent: true
test_list: absent (no tdd/test-list.md, no cycle-log.md; judged from spec.md, tasks.md, git history)
behaviors_tracked: 0 (no list); criteria_audited: 14 FR + 5 SC + 4 US
mutation: no tool in profile; 4 deliberate mutants on crates/micold-core/src/agent_run.rs, 2 survived
suite: feature-scoped run only (core agent_run/store_roundtrip/schema_hash, daemon agent_runs/hooks_receiver/attention_claims, client worktree_visibility/sidebar_tree): all green; full suite not re-run
---

# TDD verification: 552 native agent-session signals

**PASS_WITH_GAPS.** Tests exist for every requirement and kill most deliberate mutants, but test-first ordering is unprovable (no cycle log, no test list, milestone commits squashed tests and source together) and two behaviors of `apply_signal` are unguarded.

## Test-first evidence

No `tdd/` artifacts exist, so every behavior is `LIKELY` at best, never `PROVEN` (Hard Rule 3).

| Area | Evidence | Class |
|---|---|---|
| M1 hiding removal (T002-T006) | `b604a496` is one "wip ... (unverified)" commit holding test and src changes together; tasks.md T002 claims red-first | LIKELY (history cannot order) |
| M2 agent runs (T010-T019) | `e73908e0` "wip ... unverified, recovered from killed unit" contains `tests/agent_run.rs` and `src/agent_run.rs` in one commit | TEST_AFTER-indistinguishable, LIKELY |
| M3 provenance/attach removal | `cf035dfd`: one commit, tests and src deleted/rewritten together | LIKELY |
| M4 Copilot gap, docs check | `c29ea4cf`: test `a_copilot_session_never_gains_runs...` and `superseded-552.test.sh` land with the docs they check | LIKELY |

Discrepancies: tasks.md ticks every task `[x]` and states tests were written red first, with no red output recorded anywhere. The commit subjects "unverified" on both WIP commits contradict nothing but show no per-task verification record.

### Existing tests changed
Deletions dominate (worktree_visibility.rs 462 -> 165 lines, sidebar_tree.rs -206, store/provenance tests removed). These are deletions of tests for removed behavior (hide toggle, provenance, attach_worktree), as the spec requires, not weakening. Checked: no `#[ignore]` added, no threshold lowered. Residual concern, not a FAIL: the hiding tests were replaced by weaker count-only assertions (see S-2).

### tasks.md vs evidence
All T001-T033 ticked; no test list to cross-check. Each ticked code task has a matching test file on disk (agent_run.rs, agent_runs.rs, hooks_receiver.rs, worktree_visibility.rs, store_roundtrip.rs legacy-keys test, superseded-552.test.sh). No false completion found.

## Findings

**HIGH**
- none.

**MEDIUM**
- **M-1 Test-first unverifiable.** No cycle log or red evidence; WIP commits squashed. Evidence: `git show --stat b604a496 e73908e0`.
- **M-2 Surviving mutants in `apply_signal`** (`crates/micold-core/src/agent_run.rs`):
  - line 110-114: `unwrap_or(0)` changed to `unwrap_or(last)` survived. The cap test (`tests/agent_run.rs` `the_list_is_capped...`) only fills with Stopped runs, so the "no stopped run, evict oldest Running" branch is unasserted.
  - line 86-88: `if run.agent_type.is_none()` changed to `if false` survived. No test that a stop carrying an `agent_type` fills a start that lacked one.
- **M-3 Dead `Tag::Agent` variant remains.** `crates/micold-core/src/naming.rs:320` (doc: "belongs to an AI assistant", feature 014), rendered at `crates/micold-client/src/ui/sidebar.rs:423`, matched at `features/sidebar.rs:1220`. Nothing produces it; only negative assertions use it (`tests/worktree_visibility.rs:85`, `tests/sidebar_tree.rs:523-525`). It keeps an "agent" classification concept in the model that FR-001 removes, and the tests guard a value that cannot be built.

**LOW**
- **S-1 Source-text test** `crates/micold-client/tests/worktree_visibility.rs:the_sidebar_view_has_no_reveal_control` greps `include_str!` of two source files for strings. It breaks on a comment mentioning the phrase and cannot detect a differently-named control (framework-under-test / implementation-coupled). Better: assert on the view model's exposed controls.
- **S-2 Count-only assertions** `worktree_visibility.rs`: `every_worktree_is_listed` and `an_agent_named_worktree_is_listed_like_an_ordinary_one` assert `len() == 3`, not which rows; `renaming_to_or_from...` compares `len()` of two lists then `listed(&before).len()` vs `listed(&after).len()` (same quantity twice, vacuous). A bug that dropped one row and duplicated another passes. Assert the row names/order.
- **S-3 `provider` field unasserted** in `tests/agent_run.rs` and daemon `agent_runs.rs`: `AgentRun.provider` is set from the signal but no test reads it.
- **S-4 Copilot test depends on a negative research result** (`agent_runs.rs:a_copilot_session_never_gains_runs...`): it pins invented `subagent.*` lines, not captured ones. If the CLI gains an event it will keep passing while the docs say "gap"; acceptable but note it.
- **S-5 Unrelated working-tree change** `crates/micold-daemon/src/runs.rs` was modified by another process during the audit (not by this audit; mutants were all restored in `agent_run.rs`, `git diff` on it is empty).

Smells not found: tautological assertions, doubled subject, re-implemented expectations in the agent tests; the daemon tests reuse `support/attention.rs` helpers and `Fake*` doubles, matching the profile exemplars.

## Mutation (deliberate, 4 on one file; not exhaustive)

| Mutant | Result |
|---|---|
| `return *run != before` -> `return true` | killed (2 tests) |
| stop no longer sets `Stopped` | killed (2 tests) |
| cap eviction fallback `unwrap_or(0)` -> `unwrap_or(last)` | SURVIVED (M-2) |
| `agent_type` back-fill disabled | SURVIVED (M-2) |

All restored via `git checkout`; suite re-run green. No mutation tool is configured, so there is no score.

## Traceability

| Criterion | Test(s) | Entry point |
|---|---|---|
| FR-001/FR-002, SC-001/002, US1 | `worktree_visibility.rs` (listing, rename invariance, no control), `sidebar_tree.rs` | client State view model (real) |
| FR-003/004, US2.1-2 | `hooks.rs` classify tests, `hooks_receiver.rs`, `agent_runs.rs` | real hook receiver + DaemonState |
| FR-005, SC-004 | `agent_runs.rs:runs_survive_a_restart...`, `an_origin_exit...` | DaemonState with real store file |
| FR-006 | `agent_runs.rs` (b has empty agents) | yes |
| FR-007 | `agent_runs.rs` activity unchanged, `attention_claims.rs` | yes |
| FR-008 | `hooks.rs:a_sub_agent_report_without_a_usable_agent_id_is_ignored`, unknown-session test | yes |
| FR-009/010 | `store_roundtrip.rs:legacy_provenance_keys_load...` + hide-key test | store load |
| FR-011 | `sidebar_tree.rs` agents-summary test | client |
| FR-012/013, SC-005, US4 | `scripts/tests/superseded-552.test.sh` (greps docs) | script |
| FR-014 | `agent_runs.rs` first test | yes |
| US3 | Copilot gap test; provider table is docs only | partial (S-4) |
| SC-003 | covered by FR-003/004 tests, Claude only | yes |

Tests tracing to nothing: none found. Criteria with no test: FR-001 "no code classifies by name" is guarded only by the source-grep test (S-1) and `scripts/tests/superseded-552.test.sh` covers docs, not code; Tag::Agent (M-3) shows the guard is incomplete.

## What was not audited

- Full workspace suite, wall time, flakiness, isolation (only the feature's 8 test targets were run).
- Test-first ordering beyond commit-level (no red logs exist).
- Mutation beyond one file; hooks.rs, state.rs, catalog.rs, store.rs, and all client changes unmutated.
- The ~40 client/core/daemon test files edited for compile fixes in M3 were checked by diff stat only, not line by line.
- Docs content accuracy (user guide), visual-pass evidence, `sandbox_real_*` acceptance tests.
- Coverage (no tool).

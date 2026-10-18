---
feature: 551-session-spawn-tree
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 1fae1becb # short SHA audited (all four milestones on main; range 8e1cdcce2..HEAD, feature commits only)
behaviors: 31
proven: 0
likely: 1
test_after: 30
no_test: 0
high_smells: 0
criteria_total: 12
criteria_covered: 11 # US1-4 is covered for the marker only; "takes actions like any session" has no test
mutation_score: 60 # deliberate mutants, not a tool: 5 sampled, 3 caught, 2 survived; scope = 5 hand-picked lines in 5 changed files
mutants_survived: 2 # 1 inside a GREEN behavior (A12/U19), 1 boundary slack in U3
suite: 7713 passed, 0 failed, 13 ignored (557 binaries), 31m57s wall (almost all of it waiting on the shared build lock behind other worktrees; compute time not separable)
---

# TDD Verification: Session tree for agent-spawned sessions (#551)

**Verdict: FAIL.** No behavior has a recorded red (the cycle log says so for every milestone), a
`GREEN` behavior (A12/U19, the fold restored at app start) survives a deliberate mutant that
deletes its restore line, and `tasks.md` ticks T018 for a FR-007 test that does not exist.

The audit was independent: written from a cold read by a session that did not write the tests. No
subagent was used for the smell pass; every cited line was opened directly.

Stage that failed first, per the rubric's order: **Q1 (did the tests come first)**. Q2 (assert
behavior) is mostly good: no tautological, vacuous, assertion-free, conditional or skipped test
was found in the files read. Q3 (catch a bug) found one real hole. Q4 (coverage) has one gap.

## Test-first evidence

Git cannot corroborate order: every `feat(551)` commit changes the tests and their source
together (`554b4c4ad` M1, `3d8d6ae05` M2, `4769c1e71` M3, `8a2c11455` M4), and M1 was squashed onto
main. The cycle log is explicit that no red output exists:

- M1: "no per-behaviour red output was captured ... red by absence, not by a recorded run".
- M2, M3: "Tests written with the code in one pass ... no per-behaviour red output captured".
- M4: same, plus "the settings test failed to compile before `folded_origins` existed" (a claim, no output).

Per the rubric, no recorded red is `TEST_AFTER`. Compile-failure-by-absence is a plausible red
for the M1 tests, but it was never run, so it is not credited.

| Behaviors | Milestone | Class | Evidence |
| --- | --- | --- | --- |
| A5, A6, A7, U1-U11 (14) | M1 | TEST_AFTER | log admits no red; `554b4c4ad` adds tests and source together; squashed |
| A1, A2, A3, A4, U12, U13, U14 (7) | M2 | TEST_AFTER | "one pass", no red; `3d8d6ae05` adds tests and source together |
| A8, A9, U15, U16, U17 (5) | M3 | TEST_AFTER | "one pass", no red; `4769c1e71` adds tests and source together |
| A10, A11, A12, U19 (4) | M4 | TEST_AFTER | "one pass", no red; `8a2c11455` adds tests and source together |
| U18 | M4 | LIKELY | log claims a compile failure before `folded_origins` existed; no command or output; same commit |

NOT_APPLICABLE: none (no characterization baseline was recorded; `suite_baseline: unknown`).

### Existing tests

Diffed every removed line in `crates/*/tests/` across the ten feature commits (`554b4c4ad`,
`3d8d6ae05`, `4be744a7c`, `7c24cf833`, `4f1437187`, `25e1a4b05`, `4769c1e71`, `128f4351b`,
`8a2c11455`, `24db1743c`). No pre-existing test was weakened, skipped, renamed out of a filter,
or excluded, and no threshold was lowered. What the removed lines were:

- `mcp_create_session.rs` (`554b4c4ad`): the seeded session list moved from `Sandbox::new` into
  `Sandbox::build(child)`; the two seeded sessions and every assertion are preserved.
- `schema_hash.rs` (`554b4c4ad`): the pinned protocol constant follows the bump (46 to the new value),
  which is the pin doing its job.
- `layout_snapshot.rs` (`7c24cf833`, `25e1a4b05`): edits to the feature's own new test; see finding 6.
- `sidebar_spawn_tree.rs` (`24db1743c`): rustfmt wrapping only.
- `provider_origin_docs.rs` (`128f4351b`): an unused import.

The only edit to `tests/features_sidebar.rs`, `settings_sections.rs` and `settings_issue_mapping.rs`
for this feature is adding the new `folded` / `folded_origins` field to fixtures or expectations.
(Other changes in those files in the audited range belong to features 776 and others.)

### tasks.md against the test list

The test list was last updated at planning (`updated_at: f602c37c`) and never reconciled:

- A1-A4, U12, U13, U14 are still `PENDING` while T017-T019 and T039 are `[x]` and the M2 tests exist and pass.
- A8-A12, U15-U19 are `GREEN`, never `DONE`, although T026-T041 are `[x]`.

That is a ticked task whose behavior is not `DONE` on the list: finding 4.

## Findings

Ordered by severity. The audit changed no test or source file.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | No behavior has a recorded red; 30 of 31 are `TEST_AFTER`, one `LIKELY`. The loop wrote tests and code in one pass for M1-M4. | `tdd/cycle-log.md` M1 (lines 8-18), M2, M3, M4 entries; commits `554b4c4ad`, `3d8d6ae05`, `4769c1e71`, `8a2c11455` |
| 2 | HIGH | Surviving mutant in a `GREEN` behavior (A12/U19). Replacing the app-start restore with `Default::default()` leaves the whole workspace suite for `micold-client --lib`, `sidebar_spawn_tree` and `features_sidebar` green. Nothing proves the saved fold is read back at boot. `a_fold_restored_at_boot_survives_the_snapshot_without_pruning` seeds `state.sidebar.folded` by hand (`sidebar_spawn_tree.rs:204-206`), bypassing `startup.rs`; `persist_folded_origins` is unit tested (`shell/persist.rs:1170-1180`) and the settings round trip is tested (`settings_roundtrip.rs:774`), but the join between them is not. Should assert: with `Settings.folded_origins = {u}` on disk, the startup path leaves `u` folded. | mutant `crates/micold-client/src/shell/startup.rs:233`; test `crates/micold-client/tests/sidebar_spawn_tree.rs:198-212` |
| 3 | HIGH | T018 is `[x]` and claims "a child's select, stop, resume and delete messages are the same as a top-level session's (FR-007)". No such test exists in `sidebar_spawn_tree.rs` or any other test file (searched for the messages and for `FR-007` tied to 551). US1-4's "takes actions like any session" and FR-007's action half are untested; only the marker half (T019, `layout_snapshot.rs`) is. The test list overstates A4 and U13 coverage. | `specs/551-session-spawn-tree/tasks.md:69`; absence in `crates/micold-client/tests/sidebar_spawn_tree.rs` |
| 4 | HIGH | Ticked tasks with behaviors not `DONE` on the list: T017-T019, T039 (A1-A4, U12-U14 `PENDING`) and T026-T041 (`GREEN`). `/speckit.implement` or the next audit reads the list as unfinished. | `tdd/test-list.md` rows A1-A4, U12-U14 (PENDING); A8-A12, U15-U19 (GREEN) |
| 5 | MED | Boundary of the origin-chain walk is not pinned. Mutating `0..MAX_ORIGIN_DEPTH` to `0..MAX_ORIGIN_DEPTH - 1` survives `cargo test -p micold-core --lib origin::`. The accept test builds a chain of `MAX_ORIGIN_DEPTH - 1` nodes (`MAX-2` links plus a terminal), two short of the limit; the reject test is a short loop. Judged not equivalent (a real chain at the limit would be misreported as a cycle) but low-risk: the constant has no spec requirement. Should assert a chain at exactly the longest accepted length, and one link longer rejected as `Cycle`. U3 is `DONE`, so a strict reading of the rubric makes this HIGH; recorded MED on the judgment above. | `crates/micold-core/src/origin.rs:100` (mutant); `crates/micold-core/src/origin.rs:297-305` (test) |
| 6 | MED | Implementation-coupled assertion after a loosening: `25e1a4b05` changed the exact annotation text `"agent, origin pending, claude"` to `starts_with("agent, origin p")` / `"agent, origin r"` because the marker is now ellipsized to the row width. The test now depends on the painted width and distinguishes pending from removed by one letter. This was the feature's own test (not pre-existing), so it is not a weakened-existing-test failure. Should assert on the annotation the view model produces before ellipsis. | `crates/micold-client/tests/layout_snapshot.rs:~846-848` (`every_child_row_carries_the_child_marker_and_no_user_started_row_does`) |
| 7 | MED | Provider readers are proven only against synthetic fixtures (Codex, OpenCode) and a stand-in `opencode` shell script on `PATH`; Claude Code, Copilot, Pi are documented gaps. The tests pin the reader to the author's reading of the schema, not to a recorded run, so a wrong schema reading is green. The spec and docs say so (R2); recorded as residual risk, not a defect of the tests. | `crates/micold-core/tests/provider_origin.rs:1-4,152-190`; `crates/micold-core/tests/fixtures/*/README.md` |
| 8 | MED | FR-012 "two subscribers see the same summary" (named in T011) is covered only as event-equals-snapshot for one catalog; no two-observer test. `a_snapshot_after_the_archive_agrees_with_the_event...` asserts agreement, which is the substance, but the named behavior is narrower than the test. | `crates/micold-daemon/tests/session_origin_removal.rs:164-196`; `tasks.md:46` |
| 9 | LOW | Assertion roulette: several tests chain unlabeled `assert_eq!` on shapes (`sidebar_spawn_tree.rs:31-39,67-85,145-151`; `session_origin_removal.rs:104-109,174-178`). A failure does not say which rule broke. | files and lines as cited |
| 10 | LOW | `mcp_tools_catalog.rs:960` asserts that two tool descriptions `contain("origin")`; this pins documentation wording, not behavior (the row fields are pinned in `mcp_create_session.rs`). | `crates/micold-core/tests/mcp_tools_catalog.rs:960-971` |

No `HIGH` smell from the catalogue (tautological, re-implemented expectation, doubled subject,
over-mocked, vacuous, assertion-free, conditional, skipped) was found in the files read. The
`if child { ... }` in `mcp_create_session.rs` is fixture construction, not an assertion branch.
The new tests use the repository's own style (`mod support;` with `support::spawn_tree` in the
client crate; `tempfile` and the real `Catalog` in the daemon; `Fake`-style and
`AiCliProvider` types in core) and no foreign framework.

Properties: the 551 tests are fast (sub-second binaries as run), deterministic (tempdirs; the
`PATH`-mutating OpenCode tests and the `CODEX_HOME` discovery scenario are serialised by a mutex
and one-test-per-process respectively), and mostly specific about what broke. One flake in the
log (a real-sshd timing flake in CI, `89a39f17f`) is unrelated to this feature.

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, applied and restored inside a
single build-lock hold, with a green re-run of the touched targets after (core `origin::`, daemon
`catalog_adoption` and `session_origin_removal`, client `sidebar_spawn_tree`: all exit 0, working
tree source files unchanged afterwards). **5 mutants sampled across 5 files; this is a sample, not
exhaustive.**

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `micold-client/src/shell/startup.rs:233` restore replaced by `Default::default()` | A12, U19 | **Yes** | Real gap, finding 2. Ran `-p micold-client --lib --test sidebar_spawn_tree --test features_sidebar`: all green. |
| `micold-daemon/src/catalog.rs:713` `session.origin.is_some() \|\|` removed from the write-once guard | U4 (FR-010) | No | Caught by `an_origin_is_recorded_once_and_the_same_origin_again_is_idempotent` and `a_reported_origin_not_yet_known_is_kept_pending_and_stands_once_it_appears` |
| `micold-daemon/src/catalog.rs:814` children filter `!s.archived` replaced by `true` | U11 (FR-018) | No | Caught by `deleting_a_worktree_archives_origin_and_children_together_and_reports_no_child` |
| `micold-core/src/origin.rs:100` `0..MAX_ORIGIN_DEPTH` to `0..MAX_ORIGIN_DEPTH - 1` | U3 | **Yes** | Boundary slack, finding 5 |
| `micold-client/src/features/sidebar.rs:489` `counts_as_unread(s, in_view)` to `(s, None)` | U19, A11 | No | Caught by `a_folded_origin_shows_that_a_hidden_descendant_needs_attention` |

Score 3 of 5 caught (60%), scope: hand-picked lines in the five files above. Not comparable to a
tool score. Coverage: the profile has no coverage tool, so none was run.

## Traceability

12 acceptance scenarios (`spec_criteria: 12`). "Real entry point" here means the reducer
(`update`, `worktree_tree`), the `Catalog` load/archive/record API and the layout gate; no
criterion is exercised through a running daemon process or the GUI (the quickstart Part B visual
pass is manual and was not re-run).

| Criterion | Tests | Real entry point |
| --- | --- | --- |
| US1-1 (child nested, not top-level) | `children_nest_under_their_origin_at_any_depth`, `origin::tests::a_tree_nests_...`, `session_rows_carry_the_origin...` | Yes (reducer) |
| US1-2 (depth 3) | same nesting test, grandchild at depth 2 | Yes |
| US1-3 (user-started unchanged) | `a_session_without_an_origin_is_listed_as_it_was` | Yes |
| US1-4 (opens, acts, carries marker) | `every_child_row_carries_the_child_marker...` (marker only) | **Partial: actions untested (finding 3)** |
| US2-1 (restart keeps tree) | `origins_and_nesting_survive_a_restart...` (`catalog_adoption.rs:688`), `a_stored_origin_survives_save_and_load...` | Yes (`Catalog::load`) |
| US2-2 (live child resumable) | same restart test | Yes |
| US2-3 (origin deleted, once) | `session_origin_removal.rs` x4, `an_orphan_is_top_level...` | Yes |
| US3-1 (no signal, top-level) | `a_provider_without_a_reader_reports_nothing...`, `discovery_records_what_codex_reports_and_leaves_a_gap_provider_top_level` | Yes |
| US3-2 (guide lists providers) | `the_provider_table_lists_every_provider`, `every_row_says_whether_it_reports...` | Yes (reads the real guide) |
| US4-1 (fold hides, shows count) | `an_origin_never_folded_is_expanded...`, `a_folded_origin_counts_direct_children...` | Yes (reducer) |
| US4-2 (hidden child attention) | `a_folded_origin_shows_that_a_hidden_descendant_needs_attention` | Yes |
| US4-3 (fold persists across restart) | `folded_origins_round_trip...`, persist unit test, snapshot test | **Partial: boot restore unproven (finding 2)** |

Claimed tests that do not exist: the FR-007 action-message test named in T018 (finding 3). All
other `test` entries in the list resolve to a test that exists and, in this run, passes.
Tests tracing to nothing: none found.

## What was not audited

- Mutation was a 5-mutant hand sample, not a tool run; most behaviors (provider readers, the
  `SessionTree` cycle cut, `settle_pending_origins`, the three archive paths beyond one, MCP row
  serialisation, the tree view and sidebar UI code) had no mutant.
- Test files read in full: `origin.rs` tests, `sidebar_spawn_tree.rs`, `session_origin_removal.rs`,
  `provider_origin.rs`, `provider_origin_discovery.rs`, `provider_origin_docs.rs`,
  `support/spawn_tree.rs`, and the 551 tests in `mcp_create_session.rs`, `layout_snapshot.rs`
  (the child-marker test). `catalog_adoption.rs`, `store_roundtrip.rs`, `protocol_roundtrip.rs`,
  `settings_roundtrip.rs`, `mcp_tools_catalog.rs` and the `shell/persist.rs` unit test were checked
  by test name, diff and targeted lines, not read end to end for smells.
- No fresh-context subagent was used for the smell pass.
- Coverage: no tool in the profile.
- The quickstart Part B visual pass and the recorded screenshots; no GUI was run.
- Real providers: no real recording of any provider's native parent signal was audited (none exists).
- Performance and frame time (a plan goal with no requirement and no test, per the test list) and
  load with dozens of children.
- The wall time of 31m57s is dominated by waiting on the shared build lock behind other worktrees;
  the suite's own run time was not measured separately.
- At audit time the worktree had uncommitted edits to `specs/551-session-spawn-tree/{autopilot,quickstart,spec}.md`
  made by another session; they were not part of the audit and were not touched.
- Remediation tasks were not appended to `tasks.md`: the audit request limited writes to this file.
  The feature is not done until findings 1-4 are cleared. Suggested verifiable changes: (2) add a
  test that loads `Settings.folded_origins` through the startup path and re-run mutant `startup.rs:233`;
  (3) add the FR-007 reducer test T018 names and run `cargo test -p micold-client --test sidebar_spawn_tree`;
  (4) reconcile `tdd/test-list.md` states with `tasks.md`; (1) either re-derive a red per behavior by
  stashing the implementation and running each test (record command and output in `cycle-log.md`) or
  record the waiver explicitly.

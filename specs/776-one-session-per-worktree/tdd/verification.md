---
feature: 776-one-session-per-worktree
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved; no override or preset copy exists
verified_at: 3ef301a48
behaviors: 25 # no tdd/test-list.md; the 25 test tasks of tasks.md stand in (T002 T003 T006 T007 T008 T012 T014 T015 T017 T018 T023 T024 T025 T027 T031 T032 T037 T038 T039 T040 T043 T046 T050 T051 T055)
proven: 0
likely: 0
test_after: 25
no_test: 0
high_smells: 0
criteria_total: 37 # US1-US7 acceptance scenarios counted by spec.md line (6+5+4+3+5+4+5=32) plus SC-001..SC-008 partly overlapping; FR/SC mapped at the story level only
criteria_covered: 37 # at least one test per criterion by name or task; not every one through the real entry point (see Traceability)
mutation_score: null # no mutation tool in the profile; 4 deliberate mutants, 4 caught (sample, not a score)
mutants_survived: 0
suite: 7492 passed, 0 failed, 12 ignored, 541 binaries, 13m44s wall (cold build included)
---

# TDD Verification: One session per worktree, chosen at start

**Verdict: FAIL.** No red phase is recorded for any behavior (no `tdd/cycle-log.md`, no `tdd/test-list.md`), and history shows M1's daemon tests landing after the code they cover. The tests themselves are in good shape: the suite is green, four deliberate mutants were all caught, and no HIGH smell was found.

The audit was not independent of the repository's own history: it is a cold read by an auditor that did not write these tests.

## Test-first evidence

No cycle log exists, so nothing can be `PROVEN` or `LIKELY` (Hard Rule 3: fail closed). The test tasks are classified per milestone; the history is the only evidence.

| Milestone | Behaviors (test tasks) | Class | Evidence |
| --- | --- | --- | --- |
| M1 | T002 T003 T006 T007 T008 T012 | TEST_AFTER | `952e6e159` (10-12 23:35) lands occupancy, wire and daemon refusal together with 13 test-file edits; the daemon tests `one_session_per_worktree.rs`, `review_send.rs` arrive in `cb4c0650c` (10-13 00:02), 27 minutes later. Both commits are "wip ... (uncompiled)", so no test could have been run red. Ledger "Follow-ups" confirms: "TDD red phase was not recorded for M1 (tests written with the implementation; no tdd/ dir)" |
| M2 | T037 T038 T039 T040 | TEST_AFTER | `df57188e6` "wip ... (uncompiled)" changes 25 test files and 13 source files in one commit; no red recorded |
| M3 | T014 T015 T050 T051 | TEST_AFTER | `8946eb0b9` (uncompiled), `9d9cc3f80` put tests and source together; no red recorded |
| M4 | T017 T018 T043 | TEST_AFTER | `64d57e3b7` "(WIP M4)" source plus tests together; no red recorded |
| M5 | T046 T055 | TEST_AFTER | `817d05fb8` one commit, 8 test files plus 7 source files; conformance review added missing menu tests afterwards (ledger, "Conformance B M5", round 1 MAJOR) |
| M6 | T023 T024 T025 T027 | TEST_AFTER | `7ef1db4ae` "wip ... (handover)"; ledger "Declined review findings" B M6 F1 records the waiver: "red-before-green cannot be reconstructed honestly after the fact" |
| M7 | T031 T032 | TEST_AFTER | `d5c9c978b` "(WIP, uncompiled)" tests and source together |

The ledger declined the red-phase finding for M6 as a "recorded waiver". A waiver is not evidence; this rubric still grades it `TEST_AFTER`. The user had said M1's missing evidence is known; M2-M7 are the same.

The ledger's claim for M6 that the tests "fail without the change by construction" was spot-checked for the daemon side by the mutants below: the sampled core and daemon tests do fail against a broken implementation.

### Changes to tests that already existed

Scoped to the 69 non-merge `(776)` commits (not the merged-in main history). Most removed assertions are deliberate behavior changes the spec requires (one session per location, hide toggle removal is not this feature). Two are weakenings:

| # | File:line (at the commit) | Before | After | Judgment |
| --- | --- | --- | --- | --- |
| W1 | `crates/micold-daemon/tests/review_send.rs`, test `us3_s1_with_no_session_running_one_starts_and_its_first_input_is_the_prompt` (`ca70d2dc5`) | `assert_eq!(new, sid(1), "the entry's own session is started, not a second one (FR-001)")` and `assert_eq!(sessions_for(&project).len(), 3, "no session was added to the project")` | both deleted; helper `started_new_session` folded into `started_session`, which no longer states that the session reused is the entry's own | Weakened. The R10 test (`r10_an_ended_session_is_started_again_and_no_second_one_is_created`, same file) still pins the reuse, so the behavior is covered once, but this test no longer does and its `s.state.sessions_for` count check is gone |
| W2 | `crates/micold-daemon/tests/one_session_per_worktree.rs`, `deleting_an_extra_keeps_the_other_session` (`195195b52`) | `kept.iter().find(\|s\| !s.archived).map(\|s\| s.id) == Some(sid(1))` | `kept.first().map(\|s\| s.id) == Some(sid(1))` | Loosened. It now depends on `sessions_for` omitting archived sessions; the `unarchived_in(...) == 1` line above it keeps the count pinned, so the practical gap is small |

Other changed assertions checked and judged legitimate (behavior changed by the spec, new assertions replace the old): `session_discovery.rs` (`ec81fd525`: 2 and 3 become 1, and the "later conversation surfaced on every open" check becomes "not added beside it"; the R15 known-ids-before-probe check is kept in a form that still discriminates), `untitled_session_labels.rs` (`db25f1545`: three transcripts split into three single-transcript cases), `catalog_adoption.rs`, `occupancy.rs` unit tests (`default_session` replaced by `main_session`), protocol version asserts 41/43 to 42/44. No `#[ignore]` was added, no gate or threshold changed (no coverage or mutation threshold exists).

### tasks.md against the work

All of T001-T057 are `[x]`. There is no test list, so a `DONE` comparison is impossible; the claim "seen failing for the right reason first" in the header of `tasks.md` ("Every test task precedes the implementation it covers and must be seen failing for the right reason first") is not supported by any artifact. The header promise is the only evidence for ordering, which is why this is graded as missing, not as a HIGH completion claim.

## Findings

Ordered by severity.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (verdict-deciding) | No red-phase evidence for any of the 25 test tasks; M1's daemon tests provably follow the source in history; every milestone commit mixes tests and source and several are "uncompiled" WIP, so a test could not have been run before the code | `git log` `952e6e159`, `cb4c0650c`, `df57188e6`, `8946eb0b9`, `64d57e3b7`, `7ef1db4ae`, `d5c9c978b`; `specs/776-one-session-per-worktree/autopilot.md` "Follow-ups not done" and "Declined review findings" |
| 2 | MED | W1: a pre-existing assertion that the entry's own session is reused (not a second one) was deleted from the review-send test | `crates/micold-daemon/tests/review_send.rs` (`ca70d2dc5`) |
| 3 | MED | Sleepy tests: fixed sleeps stand in for waiting on a condition. `sleep(300ms)` before the restart wait, and `sleep(500ms)` then asserts a session is not live (a negative assertion that can pass before the start is processed) | `crates/micold-daemon/tests/one_session_per_worktree.rs:923`, `:1048` |
| 4 | MED | Vacuous refusal checks: `matches!(answer(...), DaemonMsg::OperationError { .. })` does not check `kind == Refused` nor the text naming the existing session, so any error would satisfy it | `crates/micold-daemon/tests/one_session_per_worktree.rs:1087`, `:1101` (and the same shape in the interleaving test around `:1080-1120`) |
| 5 | MED | W2: `kept.first()` replaces a find over non-archived sessions | `crates/micold-daemon/tests/one_session_per_worktree.rs:1130` |
| 6 | MED | The client-side criteria (US2 s1, US5 s1-s5, US6, US7 s2-s3) are tested at the reducer or `State` level; the row-press wiring and menu rendering are not exercised at the widget level (the M4 review declined this: "iced TreeItem messages are not inspectable here"). The visual pass covers them by screenshot only | `ledger: Declined review findings, M4 review B F2`; `specs/776-one-session-per-worktree/tdd/visual-pass/` |
| 7 | LOW | Traceability is by task, not by tag: US5 and US7 tests carry no `US5`/`US7` or `FR-0xx` marker (0 files match `US5 s`, 0 match `US7 s`), unlike US1-US4 (9-12 files each), so a criterion-to-test map has to be rebuilt by name | `crates/micold-client/tests/features_session.rs:1230-1335`, `features_sidebar.rs:667,679` |
| 8 | LOW | `tasks.md` header promises red-first; nothing records it (see finding 1) | `specs/776-one-session-per-worktree/tasks.md` header |

No HIGH smell from the catalogue: no assertion-free test, no tautology (daemon tests drive the real `SessionCreate` over a socket and read the catalog back), no doubled subject, no conditional assertions (the loops in `a_hundred_concurrent_creates_for_an_empty_worktree_make_exactly_one` drive requests and always reach `assert_eq!(refused, 99)`), nothing skipped. The 100-create test's `Err(_) => break` on timeout cannot hide a failure: `refused == 99` would not hold.

Style and helper use: the daemon tests reuse `state_with`, `init_repo`, `add_worktree`, `connect` from the local test module; `crates/micold-daemon/tests/one_session_per_worktree.rs` defines its own `ai_session`, `legacy_pair`, `shell` fixtures rather than `support/mod.rs` builders. This matches the exemplar's pattern (local fixtures atop `Fake*` types) and is not graded as a bypass.

### Properties

- Isolation and determinism: each daemon test uses its own tempdirs; a process-wide `PATH_LOCK` serializes the `PATH` tests. The sleeps above are the only timing dependence.
- Speed: `one_session_per_worktree` runs 25 tests in about 2 s. The workspace suite was 13m44s including a cold compile.
- Specificity: names state the behavior; the 4 refusal checks in finding 4 would not say which part broke.
- Restore note: after the mutants, the first re-run of `one_session_per_worktree` failed one test (`main_session_a_flagged_create_on_an_occupied_worktree_is_a_helper`) with the tree clean. Cause: the script restored files via `mv` of a copy whose mtime was older than the mutated build, so cargo reused the mutant artifact. After `touch`ing the two files the test passed (25 passed). The source was never left mutated (`git status` clean throughout). Anyone repeating this should `touch` restored files.

## Mutation results

No mutation tool in the profile (`mutation: null`), so deliberate mutants on four high-risk behaviors, one at a time, each restored and re-verified green (core `occupancy`: 12 passed; daemon `one_session_per_worktree`: 25 passed after the touch above).

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `crates/micold-core/src/occupancy.rs:93` drop `!s.archived` from `main_of` (archived sessions occupy) | FR-010 archived does not occupy; `T003`, `T050` | No | Caught by `archived_sessions_do_not_occupy`, `a_location_without_an_unarchived_session_has_no_main`, `the_main_session_is_the_marked_one_else_the_first_unarchived` |
| `occupancy.rs:118` `s.location == target.location` to `!=` in `start_block` | FR-013 start blocked by a live sibling; `T031`-`T033` | No | Caught by `another_live_session_of_the_location_blocks_a_start`, `stopped_archived_and_foreign_sessions_do_not_block` |
| `crates/micold-daemon/src/catalog.rs:~655` invert `!additional.is_yes()` in `Catalog::create_session` | FR-001/FR-002 refusal; `T006`, `T009` | No | Caught by 3+ refusal tests (idle, running, project root) |
| `catalog.rs:1819` `claim_main_if_free` sets `main` always | FR-020 exactly one main; `T051`, `T053` | No | Caught by `main_session_a_flagged_create_on_an_occupied_worktree_is_a_helper` (the only test that caught it: single point of protection) |

Sample of 4 behaviors of roughly 25; this is not exhaustive and gives no score. Not sampled: `Catalog::set_main` rollback on a failed write, `Catalog::replace_session` (kill the old session only once the new is durable), `cap_per_location`, the `adopt_discovered_sessions` cap, `ops::create_worktree_with_session` (no session created if the CLI is missing), and every client reducer (`start_offered`, expander, confirmation).

## Traceability

Mapped by test task and by test names (no `traces` column exists). Each file was opened or its test names listed; the 37 in the front matter are the 32 acceptance scenarios plus the 5 success criteria not repeated in them.

| Criterion | Tests | Real entry point |
| --- | --- | --- |
| US1 s1-s6 (refusal, concurrency, flag) | `one_session_per_worktree.rs` refusal tests, `a_hundred_concurrent_creates...`, `soft_limit_*`; `mcp_create_session.rs`; `review_send.rs` r10 | Yes: real daemon over its socket |
| US2 s2, s4, s5 (`create_worktree` with a definition) | `mcp_create_worktree.rs`, `one_session_per_worktree.rs` (`WorktreeCreate`) | Yes |
| US2 s1, s3 (dialog, no start action on a started row) | `features_worktree_form.rs`, `features_session.rs` (`start_offered`), visual pass B1-B2, B4 | No: reducer and screenshot only (finding 6) |
| US3 s1-s4 (restart/start keep; resume replaces or keeps both) | `one_session_per_worktree.rs`, `attach_apply.rs`, `features_attach.rs`, `attach_dialog.rs`, visual pass B3 | Daemon: yes; confirmation dialog: reducer only |
| US4 s1-s3 (several sessions intact) | `legacy_pair_opens_untouched_and_refuses_a_third`, start-block tests, `features_session.rs:1335` | Daemon: yes; client: reducer only |
| US5 s1-s5 (worktree rows, main opens) | `features_sidebar.rs:667,679`, `features_session.rs:1315` | No: reducer level (finding 6) |
| US6 s1-s4 (helper on purpose) | `features_session.rs:1246,1266,1275`, visual pass B5 | No: reducer level; daemon half covered by US1 s5-s6 |
| US7 s1-s5 (main session) | `main_session_*` in `one_session_per_worktree.rs` (e.g. `:764`), `features_session.rs:1289,1302,1309`, visual pass B6 | Daemon: yes; menu: reducer only |
| SC-001, SC-002, SC-003, SC-005 | the interleaving test, refusal tests, 100-create test, legacy-pair test | Yes |
| SC-004, SC-006, SC-007, SC-008 | reducer tests plus visual pass | No |

Untested criteria: none found. Tests tracing to nothing: none found. No claimed test was found missing.

## What was not audited

- `tdd/test-list.md` and `tdd/cycle-log.md` do not exist, so the 25 "behaviors" are the test tasks and nothing was compared against a recorded list; `traces` values could not be checked.
- Mutation: 4 deliberate mutants only; no tool, no score. Persistence rollback, replace, adoption cap, `create_worktree_with_session` and all client reducers were not mutated.
- Coverage: the profile has no coverage tool.
- Sandbox real-runtime tests (`sandbox_real_*`, `--features sandbox-real-runtime`) were not run; `858b511e4` edits them and they were not read.
- Test-smell pass covered `crates/micold-daemon/tests/one_session_per_worktree.rs` in depth, and `occupancy.rs` unit tests, the changed assertions of the 69 `(776)` commits, and the test names of `features_session.rs` / `features_sidebar.rs`. The other roughly 40 changed test files (e.g. `attach_apply.rs`, `mcp_create_session.rs`, `features_attach.rs`, the many "on purpose" fixture edits) were not read line by line.
- The visual-pass screenshots were not re-examined; they are not tests in the rubric's sense.
- Performance and load: only the wall time above.
- No independent subagent was used for the smell pass.

## Remediation

At close, findings 2 to 5 and 7 are fixed (T059-T062, T064), finding 6 is recorded in quickstart.md (T063), and finding 1 is waived (T058): the red phase stays missing, recorded in autopilot.md.

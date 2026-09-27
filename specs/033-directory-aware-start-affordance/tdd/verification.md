---
feature: 033-directory-aware-start-affordance
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 449fb2f7
behaviors: 48
proven: 38
likely: 9
test_after: 0
no_test: 0
not_applicable: 1
high_smells: 0
criteria_total: 10
criteria_covered: 10
mutation_score: not measured (cargo-mutants absent, profile mutation: null; scope was checked with 3 sampled deliberate mutants, all caught)
mutants_survived: 0
suite: scoped runs green — bin `micold-ai-ide` availability filter 23 passed; directory_availability 16 passed;
  features_session 33 passed; provider_choice_surfaces 6 passed; availability_is_asked_only_on_named_events
  3 passed; session_start_press 7 passed; unavailable_default_says_so 8 passed;
  cli_availability_comes_from_the_service 8 passed; missing_cli_is_reported_where_it_is_chosen 8 passed;
  a_field_note_shares_its_fields_column 16 passed; idle_subscriptions 6 passed. Full `cargo test --workspace`
  was attempted but the background process was killed under memory pressure (1.1G free, 10G swapped) before
  finishing; 141 test binaries had completed with 0 failures at the point it died. Not re-run in full (see
  "What was not audited").
---

# TDD Verification: The start affordance answers for its own directory

**Verdict: PASS_WITH_GAPS.** No HIGH smells, every acceptance scenario has an end-to-end test, and three
sampled deliberate mutants were all caught — but test-first evidence rests on cycle-log self-report for
most behaviors because M1's and M2's commits each squash 2-3 logged cycles into one commit, so git alone
cannot corroborate ordering within them (9 behaviors LIKELY rather than PROVEN), and no mutation tool
exists in this repo so test strength is sampled, not exhaustive.

## Test-first evidence

Git history for `84d61968..813322a6` (M1+M2, the range with source changes) shows every feature-033
commit bundling its cycle's test file(s) and source together (`297def16`, `fea2814d`, `d7241ad6`,
`a8b0f346`) — normal for a per-cycle commit per the rubric, and `PROVEN` when the cycle log records the
red. Two commits (`fea2814d`, `a8b0f346`) each squash more than one cycle-log entry, so while the cycle
log documents each cycle's own red/green pair with concrete failure output (specific `left`/`right`
values, specific panic strings), git history cannot independently show which cycle happened when inside
that one commit. Per the rubric ("A squashed history... means LIKELY, not PROVEN"), the behaviors from
the *first* of two cycles squashed into one commit (cycle 2, whose own log entry admits a resumed WIP
commit `420e9def` "already carried the green reader body") are graded LIKELY; the cycle that follows and
is the commit's "final" state is graded PROVEN where its own red is fully documented.

| Behavior | Class | Evidence |
|---|---|---|
| U1–U10, U16–U21 (16) | PROVEN | Cycle 1, commit `297def16`: red `3 passed; 13 failed` with per-test failure lines quoted in cycle-log.md; green same commit, `16 passed` |
| U11–U15 (5) | LIKELY | Cycle 2: red `28 passed; 5 failed` recorded, but cycle-log itself notes the session resumed from a WIP commit that already carried the green reader body, and the commit (`fea2814d`) squashes this cycle with cycle 3 — order not independently verifiable |
| A1–A7, U22–U27, U35–U36 (15) | PROVEN | Cycle 3, same commit `fea2814d`: red `212 passed; 13 failed` with specific per-test failure values quoted; a genuine red found and fixed a vacuous assertion in U35/U36 before green (menu string vs. `display_name()`) — a good discipline signal, not a smell |
| A8, U28–U33 (7) | PROVEN | Cycle 5, commit `a8b0f346`: red `16 passed; 7 failed` with specific failure lines per behavior |
| U34, A9, A10 (3) | LIKELY | Cycle 5 records these "passed on first run" (no natural red) because the behavior was already true from earlier design; the team then verified them with a deliberate mutant (documented) that failed as expected — real red evidence exists, but via mutation after the fact, not TDD red-first |
| U37 (1) | LIKELY | Tripwire test: cycle-log explicitly says its red evidence is the mutant runs, not a pre-change run, because the red build happened after the implementation in this resumed session |
| U38 (1) | NOT_APPLICABLE | Pre-existing test (`idle_subscriptions.rs`), re-run unchanged, not authored for this feature |

Two rendered tests in cycle 4 (`session_start_press.rs::the_default_row_draws_its_chevron_from_its_own_directorys_answer`,
`..._starts_a_default_its_own_directory_provides`, commit `d7241ad6`) are not on the behavior list at
all. Cycle-log honestly says their red was a deliberate mutant against wiring that "already existed from
T040" — i.e., for these two specific tests, the implementation predated the test. This is a legitimate,
disclosed test-after case for extra tests beyond the plan's own list, not a hidden one; it does not
change the verdict but is recorded here since the rubric asks history vs. log discrepancies to be
reported (finding #3 below).

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | LOW | `hold_home`/`hold`-style fixture (build `AvailabilityAnswers`, `asked(0,..)`, `answered(0,..)`) is copy-pasted near-verbatim across 5 test files instead of one `support::hold_home` helper | `crates/micold-client/tests/features_session.rs:485-494`, `provider_choice_surfaces.rs:51-54`, `session_start_press.rs:38-41`, `unavailable_default_says_so.rs:35-38`, `a_field_note_shares_its_fields_column.rs:41-44` |
| 2 | LOW | Two names for the same literal `"/repo"` (`PROJECT` and `P`) in one file | `crates/micold-client/tests/directory_availability.rs:20-21` |
| 3 | INFO | Two rendered tests (review-B follow-up, cycle 4) exist with no behavior id in `tdd/test-list.md`; their red was a deliberate mutant against already-existing wiring, i.e., test-after for those two tests specifically, honestly disclosed in cycle-log | `specs/033-directory-aware-start-affordance/tdd/cycle-log.md` Cycle 4; `crates/micold-client/tests/session_start_press.rs::the_default_row_draws_its_chevron_from_its_own_directorys_answer` and `..._starts_a_default_its_own_directory_provides` |

No HIGH or MED smells (assertion-free, tautological, doubled-subject, over-mocked, vacuous,
self-approving-snapshot, conditional-logic, empty/skipped, redundant, foreign-style, bypassed-utility,
framework-under-test) found in any of the 11 new/changed test files, per an independent fresh-context
smell pass (delegated, findings verified against the cited lines). No existing assertion was weakened,
loosened, or excluded in the `84d61968..813322a6` range; the one behavior-changing rewrite
(`an_answer_to_an_earlier_question_does_not_replace_a_later_one` in `main_tests.rs`, T007) reverses its
premise deliberately per FR-002/FR-009 and is documented as such in both `tasks.md` T007 and cycle-log,
not a silent weakening — confirmed by reading the test body: it still asserts a specific value
(`for_dir(worktree)` = `[Pi]`, `home()` = `[ClaudeCode]`), not a truthiness check.

`tasks.md` cross-check: every task is `[X]`, every behavior in `tdd/test-list.md` is `DONE`, and the two
line up — no ticked task pointing at a non-`DONE` behavior, no `DONE` behavior with an unticked task.

## Mutation results

No mutation tool is installed (`cargo-mutants` absent; profile records `mutation: null`). Per the
rubric's fallback, three deliberate mutants were sampled on the highest-risk logic in
`features/session.rs::AvailabilityAnswers` and re-verified restored:

| Mutant | File:line | Behavior | Caught | Restored & suite re-verified |
|---|---|---|---|---|
| Disable the same-key staleness guard in `answered()` (`if false && …`) | `features/session.rs:358` | U2 (FR-009, stale-answer discard) | Yes — `a_late_answer_to_an_older_request_for_the_same_directory_is_dropped` FAILED | Yes, `git diff` empty after restore, 16/16 green |
| Boundary `>= 2` → `>= 1` in `start_affordance_offers_a_choice` | `features/session.rs:1994` | U11 (chevron boundary) | Yes — `the_chevron_follows_the_rows_own_answer` and an unrelated pre-existing test `a_single_installed_cli_has_no_secondary_half_at_all` both FAILED | Yes, restored, 49/49 green across both files |
| Disable `retain`'s pruning (no-op the three `.retain(...)` calls) | `features/session.rs:387-389` | U7, U28–U31 (FR-003/FR-012 pruning on project switch/forget/hide/delete) | Yes — 4 of the `availability_*` bin tests FAILED (`switching_projects_drops_and_reasks`, `forgetting_a_project_drops_its_answers`, `revealing_agent_worktrees_asks_hiding_drops`, `a_deleted_then_recreated_worktree_is_asked_again`) | Yes, `git diff --stat` on the file empty after restore, re-run green |

Only 3 of 48 behaviors were sampled (about 6%); this is not exhaustive. It targets the module the spec's
own risk assessment centers on (`AvailabilityAnswers`, the one new stateful type) and each mutant maps to
a `DONE` behavior with no survivor.

## Traceability

All ten acceptance scenarios (US1 1–5, US2 1–2, US3 1–3) map 1:1 to A1–A10 in `tdd/test-list.md`, each
run through the real entry point (`update_inner` via the outbox harness in `main_tests.rs`, not a unit
with doubles at the boundary), and are corroborated by real quickstart §B evidence
(`specs/033-directory-aware-start-affordance/evidence/quickstart-b.md` plus 12 screenshots and 2 log
excerpts, T028).

| Criterion | Tests | End to end |
|---|---|---|
| US1-1..5 (FR-001,002,005,006,007,008,009,010,011) | A1–A5 | Yes — `main_tests.rs` + quickstart §B 1–5 |
| US2-1,2 (FR-002) | A6, A7 | Yes — `main_tests.rs` + quickstart §B 6 |
| US3-1,2,3 (FR-003,004,012, SC-003, SC-005) | A8, A9, A10 + tripwire U37 + `idle_subscriptions.rs` (U38) | Yes — `main_tests.rs` + structural scan + quickstart §B 7–8 |

All twelve functional requirements (FR-001–FR-012) and all five success criteria (SC-001–SC-005) have an
entry in `plan.md`'s requirement map pointing at a real test, and every test named there was confirmed to
exist and pass in this audit's runs. Untested criteria: none. Tests tracing to nothing: the two cycle-4
rendered tests noted in finding #3 (not a spec gap — they reinforce US1-1/FR-007 rendering, just
undocumented in the list).

## What was not audited

- **The full `cargo test --workspace` run was not completed.** The background process was killed (host
  had 1.1G free RAM, 10G swapped) after 141 of ~337 test binaries had passed with 0 failures. Scoped runs
  covering every file the feature's test list names (bin `availability` filter, and each of the 10 new/
  changed integration test files by name) all passed green afterward. `mise run gate` (fmt, clippy, full
  workspace, script tests) was not run in this audit; CI is the last word on that.
- **Mutation was not tool-measured.** No `cargo-mutants` in the environment; not installed for this audit
  (disk/build-lock cost, and the rubric's explicit fallback covers this). Only 3 of 48 behaviors got a
  deliberate mutant.
- **Coverage** is unavailable in this stack (`cargo-llvm-cov` not installed); not assessed.
- **Windows/macOS-only behavior**: none applies here (plan states no `cfg` arm is added), so nothing was
  skipped for platform reasons.
- **The real-GUI (quickstart §B) evidence was read, not re-run** in this audit; its screenshots and log
  excerpts were taken as given (T028, `evidence/quickstart-b.md`).
- **Performance** (SC-004's "at most D resolutions"): covered by request-count assertions in
  `main_tests.rs`, not independently re-measured here beyond reading those assertions.

---
feature: 040-worktree-pr-ci-status
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: caa6ef54
behaviors: 191 # A1-A42 and U1-U149, from tdd/test-list.md
proven: 85
likely: 64
test_after: 27 # 20 of them are the accepted M3 deviation (T024/T025); 7 are not
no_test: 15
high_smells: 0
high_findings: 3
criteria_total: 42
criteria_covered: 27 # a test at the level the list names; 15 have none or inner-loop only
mutation_score: n/a # no mutation tool in the profile; 12 deliberate mutants, 12 caught, scope below
mutants_survived: 0
suite: 6011 passed, 1 failed (host environment, not the feature), 9 ignored, 464 binaries, 220 s warm
---

# TDD Verification: Pull Request and Check Status for Each Worktree

**Verdict: FAIL.** Ten acceptance scenarios have no test at the level the test list names (A1-A9 and A12), and the test list is not
updated for M3-M7: 120 rows are still `PENDING`/`GREEN` with no test named while tasks T023-T065 are ticked. No weakened or skipped
existing test, no `HIGH` smell, and all 12 deliberate mutants were caught, so the tests that exist are strong; the gap is in what was
never written and in the evidence trail.

Not an independent audit of authorship: this session did not write the tests. Everything below was re-read from the files as they
stand. No fresh-context subagent was used for the smell pass; the cited lines were opened by the auditor.

## Preflight

- Profile `.specify/memory/tdd-profile.md` read (conventions, exemplars, helpers). Feature resolved from the explicit directory in the
  request, because `check-prerequisites.sh` errors without `.specify/feature.json`.
- Suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 6011 passed, 1 failed, 9 ignored, 464 binaries, 220 s
  (warm). The one failure, `crates/micold-core/tests/github_locate_desktop_launch.rs:58` (`a_desktop_launch_finds_a_working_gh`), is
  host-specific (`gh` is installed under `~/.local/share/mise/...`, outside a desktop launch's `PATH`); that file is not touched by any
  040 commit. Reported as a failure that predates the feature, not a feature failure. A first run without `--no-fail-fast` stopped at
  it after 202 binaries.
- After the mutants, the whole suite was run again: identical result (6011 passed, 1 failed, the same test). `git status` shows only
  the `autopilot.md` change that was there at the start.
- Accepted deviation, not re-counted as a finding: M3 tests T024/T025 were written after the wiring (Constitution I), recorded in
  `cycle-log.md` (M3 cycle: U85-U97, A10-A14, A31, A40-A42; and U85-U88). They stay in the table as `TEST_AFTER` so the counts add up.

## Test-first evidence

History: M1 and M2 land test and source in one commit per cycle (`git log --grep='(040)'`); M3's reducer has a test-only commit
(`370ea5d9`) before its source (`57e69d13`); M4 has a red-test commit (`94f97126`) before its source (`c288de0b`); M5, M6 and M7 are one
commit each (`ec81aa47`, `f994058b`, `044a0579`) so order is not provable from history.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| U1-U47 (M1) | PROVEN | red per cycle with output (stub-based batches, cycles 1-9), test and source in the same commit |
| U48-U69 (M2) | PROVEN | red per cycle; U61, U63, U65, U66, U69 are red by mutant (the code already held), which the log states |
| U70-U84 (M3 reducer) | PROVEN | `370ea5d9` tests, `57e69d13` source; red "3 passed, 13 failed" recorded |
| U120 | PROVEN | "1 of 3 failed" on an assertion, same commit |
| U85-U97, A10, A11, A14, A31, A40-A42 | TEST_AFTER (accepted) | "written after `shell/pr_status.rs` and the wiring ... passed on their first run"; two mutants afterwards |
| U116, U117, U118, A19, A21 | TEST_AFTER | log: `pr_status_open_*` "not seen red on their own: deviation"; the `OpenInBrowser` icon "not seen red"; the A21 scan "passed on first run". U116 was later mutant-checked (review B F2), and mutant M12 below kills it again |
| U119, U121 | TEST_AFTER | log: "already held because M3 stored and cleared `removable`", no mutant recorded |
| U98-U110, A13 | LIKELY | tests in `94f97126` before source, but the log records no red command or output ("none compiled until the code existed"); U108-U110 are a regenerated snapshot plus geometry gates |
| U111-U115, A15-A18, A20, A23, A24 | LIKELY | "5 of the 8 tooltip tests failed" on a stub, which 5 is not named; menu tests "not reached" |
| U122-U128, A25-A30 | LIKELY | log: compile-red only, "not seen failing on an assertion" |
| U129-U149, A22, A32, A34, A36 | LIKELY | log: compile-red only; the shell tests' one assertion-red was a test bug |
| A1-A9, A12, A33, A35, A37, A38, A39 | NO_TEST | see F2, F3, F4 |

## Existing tests: what the change did to them

No assertion was weakened, loosened, skipped or excluded, and no threshold lowered. Changed existing assertions, all required by the spec
or by an added item:

- `crates/micold-client/tests/features_sidebar.rs`/`icons.rs` `Icon::ALL.len()` 39 -> 46 (`94f97126`), comment lists the seven icons.
- `crates/micold-client/tests/settings_sections.rs` label `"GitHub issues"` -> `"GitHub"` (`94f97126`, FR-029/U107) and the `DEFERRED` entry
  `("pr_status_enabled", "040 T038")` removed once the checkbox exists.
- `root_vocabulary` scan count 18 -> 19 (`64da0d0b`, one new feature wrapper).
- `layout_snapshot.txt` regenerated in `c288de0b`, `f994058b`, `044a0579`, `599def40` (see F9).
- `57e69d13` changed a helper in `features_pr_status.rs` (saved the statuses before the switch-off cleared them): a test bug, not a
  loosening.

## Findings

| ID | Severity | Category | Finding | Evidence |
| --- | --- | --- | --- | --- |
| F1 | HIGH | docs | The test list is not updated after M2. Rows U70-U149 and A1-A42 are `PENDING` (or `GREEN`) with an empty `test` column while tasks T023-T065 are `[x]`; header `updated_at: 52ccc184`. Per the rubric a ticked task whose behavior is not `DONE` is a completion claim with no evidence, and the list cannot be used to confirm any `traces` value for M3-M7 (they were checked by hand here). | `tdd/test-list.md:48-60,105-116,290-520`; `tasks.md:142-275` |
| F2 | HIGH | unbuilt behaviour | A1-A9 (US1-1 to US1-9) are specified as one test that feeds `parse_status` on the recorded fixtures through selection, check reduction and the row projection. No client test reads a fixture or calls `parse_status` (grep of `crates/micold-client/tests` and `src`). `features_sidebar.rs:911-1010` projects hand-built `pull_request(n)` values, always `Open { Failing }`, so no test shows that a draft, merged, closed, or passing/pending/no-check status reaches the row. Core parse tests (U9-U15) and the showcase poses (U103) cover the ends, nothing covers the join. T030/T037 are ticked citing A1-A9. | `crates/micold-client/tests/features_sidebar.rs:889-1010`; `test-list.md:46-56`; `tasks.md:181,191` |
| F3 | HIGH | unbuilt behaviour | A12 (US1-12, FR-021, SC-005) has no test: nothing shows a selection message is applied while a reading is outstanding, or that the update call returns without the source being called. The code does hand the read to `spawn_blocking` (`shell/pr_status.rs:212-214`), but a change that read on the update loop passes every test (the fake answers instantly). T028 is ticked with `[A12]`. The comment `// A12, U97` at `main_tests.rs:8971` labels the "finished reading changes nothing else" test, which is A33/FR-020. | `crates/micold-client/src/main_tests.rs:8971`; `src/shell/pr_status.rs:212`; `tasks.md:164` |
| F4 | MED | unbuilt behaviour | A33, A35, A37, A38, A39 have no shell-level test (only the reducer's U132-U139 cover the logic). No shell test sends a tick after `RateLimited` (the only `RateLimited` in `main_tests.rs` is `until: u64::MAX` with no tick, line 8979), and none shows the next tick adds a call after `Passing` or `Unavailable`. The `A` labels in the comments do not match the list: `main_tests.rs:8626` says "A32, A33" (refresh control, which is A36), `:8673` "A34", `:8683` "A35 (FR-027)" (A35 is coalescing), `:8725` "A36 (S3)" (S3 is A32). A reader tracing by id lands on the wrong test. | `main_tests.rs:8626,8673,8683,8725,8971-8999`; `test-list.md:107-113` |
| F5 | MED | test-strength | Seven further `TEST_AFTER` behaviors beyond the accepted M3 deviation: U116, U117, U118, A19, A21 (M5) and U119, U121 (M6). U116 has since been killed by a mutant (review B, and M12 below). A21, U117, U118, U119, U121 have no recorded mutant check. | `cycle-log.md:535-542,548-549` |
| F6 | MED | docs | M4-M7 reds are compile-failures with no recorded command or output (`cycle-log.md:520-521,548-549,558-564`). Every one of those cycles says "compile-red, not seen failing on an assertion: deviation". Fail-closed, these are `LIKELY`; the log also says "commit: the commit that adds this entry" instead of a hash, so entries cannot be tied to a commit. | `cycle-log.md:518-569` |
| F7 | MED | test-strength | `no_sequence_yields_two_reads_without_an_end_between_them` (U83) asserts only inside `if let Effect::Read { .. }`, never that a read happened, so it passes if the walk never reads; and `ends` re-derives from the message which ones end a reading, the rule the reducer implements. It is deterministic and the 12 mutants show the reducer is otherwise well pinned. Add an assertion that at least one read occurred, and take `ends` from the pre/post phase instead of the message. | `crates/micold-client/tests/features_pr_status.rs:349-374` |
| F8 | MED | test-strength | The source gate for U88 scans only `shell/pr_status.rs` and `features/pr_status.rs` (the `for file in [SHELL, FEATURE]` loop) while U88 and FR-032/SC-011 say no line of `crates/micold-client/src` logs a title or address; the logging check is a 5-line text window. The start gate pins the exact source line `Effect::Read { seq } => start(app, seq),` (implementation-coupled; a rename fails it with no behavior change). Acceptable for this repository's guard-test style, but the title claim is wider than the check. | `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs:159-163,290-322` |
| F9 | MED | test-strength | U108-U110, U128 and U148 rest on a committed layout snapshot regenerated in the same commits as the code (`c288de0b`, `f994058b`, `044a0579`, `599def40`). The covered-state geometry gates (`tests/support/covered_states.rs:1632-1670`) are independent assertions, so this is not a self-approving snapshot, but nothing in the log records the diff of the snapshot being reviewed. | `crates/micold-client/tests/fixtures/layout_snapshot.txt`; `cycle-log.md:524,553,569` |
| F10 | MED | unbuilt behaviour | The recorded visual pass is partial: B3-B11 and B13-B17 NOT RUN, B12 passed with a caveat. Not run: the indicator in both themes (US1-13), the 100 ms response (SC-005), Open pull request in a browser (SC-009), the delete confirmation (SC-010), a real `gh` and rate-limit, a second window (SC-007). T067 is correctly unticked. | `evidence/visual-pass.md:1-18`; `tasks.md:284` |
| F11 | LOW | test-strength | U42 and U55 assert that `FakePullRequestSource` and `FakeGit` return what they were scripted to. The fakes are shipped helpers and are the subject, so this is not a doubled subject, but it pins a double rather than the feature. | `crates/micold-core/tests/pull_request_source.rs:55-80` |
| F12 | LOW | test-strength | The shell tests script the fake source with `statuses()` and assert `state.statuses == statuses()`: the answer passes straight through. The wiring (listing order, deduplication, detached branches left out, clearing on release) is what they pin, and mutants confirm that, but a wrong parser behind the seam would not show here (see F2). | `main_tests.rs:8530-8577` |

No `HIGH` smell from the catalogue: no assertion-free, tautological, vacuous, skipped, or empty tests found in the files read. Smells
that are not findings: `Foreign style` and `Bypassed test utility` were checked against the profile and the `Fake*` helpers; the 040 tests
use `FakePullRequestSource` and the existing `rig`/`snapshot_with`/`quiet_settings` helpers of `main_tests.rs`, and the guard test copies
`issues_are_requested_only_on_named_events.rs`, as the profile prescribes.

Properties: isolation, determinism and speed are fine (no clocks, no network, no sleeps; the one random walk is a seeded LCG at
`features_pr_status.rs:352`). The core test files run in well under a second each. Failures name the rule in their messages.

## Mutation results

Tool: none (`mutation: null` in the profile; `cargo-mutants` absent). Method: deliberate mutants, one at a time, each restored with
`git checkout` and confirmed by `git diff --quiet`, then the whole suite re-run (6011 passed, same single host failure). 12 mutants were
sampled across the feature's own files; this is not a score and is not exhaustive.

| Mutant | Behavior | Caught | By |
| --- | --- | --- | --- |
| M1 `pull_request.rs` `is_stale`: `>` to `>=` | U129-U130 | Yes | `a_reading_exactly_ten_minutes_old_is_not_stale` |
| M2 `rate_limit_pause`: drop `.max(now+1)` | U40, U41 | Yes | `a_reset_in_the_past_pauses_one_second`, `retry_after_zero_pauses_one_second` |
| M3 `select_pull_request`: `max_by` to `min_by` | U17, U18 | Yes | two select tests |
| M4 `select_pull_request`: drop the cross-repository filter | U15, U19, U20 | Yes | `parse_status_only_cross_repository_is_empty` (the select tests did not catch it alone) |
| M5 `reduce_checks`: `failed > 0` to `failed > 1` | U23, U26 | Yes | `a_failed_count_beside_pending_and_passing_is_failing`, `each_state_name_alone_reduces_to_its_group` |
| M6 `git.rs` `containment`: `Some(false)` to `Contained` | U51, U60 | Yes | `a_tip_that_is_not_an_ancestor_of_the_head_is_beyond` |
| M7 `BRANCHES_PER_REQUEST` 50 to 51 | U44 | Yes | `branches_are_read_in_chunks_of_fifty` (the query tests alone passed) |
| M8 `features/pr_status.rs` `paused`: `<` to `<=` | U137, U138 | Yes | two rate-limit tests |
| M9 removable filter: `Merged` to any state | U120 | Yes | `a_removable_branch_that_is_not_merged_or_not_listed_is_dropped` |
| M10 abandon rule: `>=` to `>` at 60 s | U139 | Yes | `a_trigger_sixty_seconds_after_a_reading_started_abandons_it` |
| M11 `sidebar.rs`: `if row.stale` to `if true` | U146, A22 | Yes | three tooltip tests |
| M12 `pull_request_address_to_open`: drop the `https://github.com/` guard (via `.or(Some(..))`) | U116, FR-014 | Yes | `pr_status_open_opens_nothing_for_an_address_that_is_not_github` |

Survivors: 0. Not sampled: the shell reading wiring beyond what the log recorded (dedupe, `Displaced`), `Cleanup:` text, the daemon
`MergedBranchCheck` arm and its 50-query bound, the settings switch, the layout and the stale indicator. M4 and M7 show the file
named in the list is not always the file that catches a bug (the cross-repository filter and the chunk size each had one test file that
did not catch it).

## Traceability

Criteria are the 42 acceptance scenarios (US1 1-14, US2 1-10, US3 1-6, US4 1-12), as the test list's `spec_criteria`. FR and SC ids are
checked through the test list's `traces`, which for M3-M7 had to be verified by hand (F1).

| Criterion | Tests | Level the list names |
| --- | --- | --- |
| US1-1 to US1-9 (A1-A9) | none client-side; core U9-U15, `features_sidebar.rs:911` (hand-built values only) | No (F2) |
| US1-10, US1-11, US1-14 (A10, A11, A14) | `pr_status_without_gh_...:8749`, `pr_status_without_a_github_remote_...:8593`, `pr_status_reads_the_listed_branches_...:8530` | Yes (after the wiring) |
| US1-12 (A12) | none | No (F3) |
| US1-13 (A13) | `icons_font.rs:30` | Yes |
| US2-1 to US2-4, US2-6, US2-8, US2-10 (A15-A18, A20, A22, A24) | `features_sidebar.rs:1038-1210,1540-1610` | Yes (reader level) |
| US2-5, US2-7, US2-9 (A19, A21, A23) | `main_tests.rs:9801`, `pr_status_is_read_only_on_named_events.rs:347`, `worktree_menu_pull_request.rs:34` | Yes |
| US3-1 to US3-6 (A25-A30) | `main_tests.rs:9139-9300`, `features_sidebar.rs:1241-1290`, `worktree_menu_pull_request.rs:59` | Yes |
| US4-1, US4-2, US4-4, US4-6, US4-10 to US4-12 (A31, A32, A34, A36, A40-A42) | `main_tests.rs:8530,8727,8675,8629,8874-8956` | Yes |
| US4-3, US4-5, US4-7, US4-8, US4-9 (A33, A35, A37, A38, A39) | reducer only: `features_pr_status.rs:496,529,218-250` | No at shell level (F4) |

Untested criteria: A1-A9 and A12 (10); reducer-only: A33, A35, A37, A38, A39 (5). Tests tracing to nothing: none; the tests beyond the
list (`pr_status_open_*` extras, the `read_outcome` pair in `pull_request_source.rs`, the `pr_status_displaced_from_another_project_*`
case) trace to FRs and are recorded in the log. Not covered by any automated test, by design (list "Held by no automated behavior"):
FR-034, FR-035. FR-029's note wording and SC-005's 100 ms are quickstart-only (F10).

## What was not audited

- No mutation tool: nothing here is a mutation score. 12 deliberate mutants; the rest of the feature's source is unmutated.
- Coverage: the profile has none; no branch coverage was run.
- Files read in full: `features_pr_status.rs` (the property walk, trigger tests), the `pr_status_*` blocks of `main_tests.rs`
  (8400-9000, 9780-9900), `features_sidebar.rs:885-1010`, the guard test's main assertions. Files sampled or checked by name only:
  `pull_request_{parse,failure,checks,select,query,source,stale}.rs`, `git_containment.rs`, `merged_branch_check.rs`,
  `pr_status_setting.rs`, `idle_subscriptions.rs`, `features_settings.rs`, `settings_sections.rs`, `showcase_completeness.rs`,
  `material_builder_api.rs`, `icons_font.rs`, `covered_states.rs`, and the tooltip/removable/stale tests of `features_sidebar.rs`
  beyond 1038-1290 and 1540-1610. Their smells were not exhaustively graded.
- Actual `gh`, the network, the browser, both themes, a second window and the delete confirmation: quickstart §B, partial (F10).
- The sandbox acceptance suite (`sandbox_real_*`) and the Windows/macOS legs: not run here; the feature adds no `cfg` arm per T068.
- Performance and load behavior: SC-005 and SC-006 numbers are not measured by any test.
- `mise run gate` (fmt, clippy, `scripts/tests/*.test.sh`) was not re-run; only `cargo test --workspace` was.
- `tasks.md` was not touched: the request restricted writes to this file, so no `TDD remediation` phase was appended (skill Phase 7
  skipped by instruction). The remediation, in order, is: F2, F3 (write the missing tests), F1 (update the list rows and `updated_at`),
  F4 (shell tests for A33/A35/A37/A38/A39 and the comment labels), then F5-F12.

## Resolution at close (2026-10-09)

- F2, F3, F4, F7, F8: fixed with tests only (A1-A9 as `recorded_*` tests in `features_sidebar.rs`; A12, A33, A35, A37, A38, A39 and corrected A-id labels in `main_tests.rs`; U83 and U88 strengthened). `cargo test -p micold-client --all-targets`: 2953 passed, 0 failed.
- F1: test-list.md carries a close note; F6: cycle-log gaps accepted, not reconstructed.
- F5, F9, F11, F12 (LOW/MED test-strength, TEST_AFTER beyond the accepted M3 deviation, snapshot review, fake pass-through): accepted, recorded in the ledger.
- F10: T067 stays open: quickstart B3-B11 and B13-B17 need a GitHub scratch repo with pull requests (not created on the user's account). Documented follow-up.

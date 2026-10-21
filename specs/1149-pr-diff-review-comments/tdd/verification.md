---
feature: 1149-pr-diff-review-comments
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 1e82b7dd6 # short SHA audited (M1-M3 merged on the branch)
behaviors: 14 # test tasks of tasks.md (no tdd/test-list.md exists); T015 is a characterization test
proven: 0
likely: 0
test_after: 13
no_test: 0
not_applicable: 1
high_smells: 0
criteria_total: 29 # US1 s1-5, US2 s1-6, US3 s1-4, US4 s1-3 and FR-001..FR-017 grouped under them; see Traceability
criteria_covered: 28
mutation_score: n/a # no mutation tool in the profile; 6 deliberate mutants, 6 caught (changed files only)
mutants_survived: 0
suite: 8770 passed, 0 failed, 13 ignored, 15m wall (includes ~8 min waiting on another worktree's build lock)
---

# TDD Verification: Review comments on a pull request's diff (#1149)

**Verdict: FAIL.** The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, so no behavior has a recorded red; every test task is `TEST_AFTER` by the rubric's fail-closed rule (git shows test and source landing in the same commit, and the ledger admits the client tests were written after the implementation). Test quality itself is good: 0 HIGH smells, 6 of 6 deliberate mutants caught, no existing test weakened, suite green.

This audit is not independent of the autopilot run only in the sense that the auditor did not write the tests; every test and source file cited was re-read cold.

## Test-first evidence

No test list exists, so "behavior" here is one test task of `tasks.md`. Evidence: `autopilot.md` (self-report), git history (`git log --stat 0ebf14ba..HEAD`), files as they stand. History is squashed per milestone (`d6d1156f7` wip M1 core/wire/daemon, `afe309857` M1 client, `12dbbf30f` M2, `cf3eb0f5e` M3): each commit adds tests and source together, so order is not verifiable.

| Task | Class | Evidence |
| --- | --- | --- |
| T001 pull_request serde test | TEST_AFTER | no red recorded; `d6d1156f7` carries test and source together |
| T002, T003, T004 (core comment, store, protocol) | TEST_AFTER | no red recorded; same commit `d6d1156f7`; ledger says "written first" but gives no output |
| T006 (daemon review_edit W18/W19/W21) | TEST_AFTER | no red recorded; same commit `d6d1156f7` |
| T008 (client features_pr_changes M1) | TEST_AFTER | `autopilot.md`: "client tests written after the implementation"; `afe309857` |
| T012 (build_pr bytes), T014 (daemon review_send) | TEST_AFTER | no red recorded; `12dbbf30f` |
| T015 (review_reports FR-011) | NOT_APPLICABLE | declared characterization in tasks.md |
| T017 (client send_action) | TEST_AFTER | `12dbbf30f`; no red |
| T019, T020 (git reads, US3 daemon) | TEST_AFTER | no red; `12dbbf30f` |
| T023 (anchor/diff HunkLines) | TEST_AFTER | no red; `cf3eb0f5e` |
| T025 (client M3) | TEST_AFTER | no red; `cf3eb0f5e` |

Existing tests changed by the feature (range `0ebf14ba..HEAD`, 1149 files only): `protocol_roundtrip.rs` (version 56 to 57, `ClearSent`/`DiscardPending` gain `scope`), `schema_hash.rs` (56 to 57), `store.rs` unit tests (version-3-quarantined becomes version-4-quarantined, as T003 mandates), `review_edit.rs`, `review_reports.rs`, `mcp_audit_log.rs` (extra `begin_send`/`review_send` arguments). All are signature or version adaptations; no assertion removed, loosened, skipped or excluded; no threshold changed. (Other deletions in the range belong to other features merged from main.)

`tasks.md`: all T001-T027 are `[X]`; no list to compare against, so no ticked-without-evidence finding beyond the missing evidence itself.

## Findings

### (a) Unbuilt behaviour

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| A1 | MED | US1 s4 ("each comment remains reachable from the review panel under its own pull request"): clicking the panel row of another pull request's comment does nothing (`jump_target` returns `None`); the test pins that as intended. Ledger declined it as "M3/later"; M3 did not build it. | `crates/micold-client/src/features/changes.rs:763`, `crates/micold-client/tests/features_pr_changes.rs:1279` |
| A2 | LOW | Composer pinned open across a head re-read keeps the old line numbers (declined M3 review A F1). | `autopilot.md` declined findings |
| A3 | LOW | A large PR file not yet shown keeps its prior outdated mark instead of being judged (declined M3 review A F2); `judge_pr_comment` returns `None`. | `crates/micold-client/src/features/changes.rs:2705` |
| A4 | INFO | Quickstart part B (visual pass) not run; left to the close unit. SC-005 (byte-identical on three platforms) is only checked on the host. | `tasks.md` Implementation Strategy |

### (b) Test strength

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| B1 | FAIL-driver (process) | No `tdd/test-list.md`, no `tdd/cycle-log.md`: no red recorded for any behavior; test and source share commits. | `specs/1149-pr-diff-review-comments/tdd/` absent |
| B2 | MED | FR-014 / US1 s3: PR-diff picking is tested only for "still loading" and binary. Non-UTF-8, over-size (`large`, not shown) and too-large-for-GitHub diffs have no test, though T008 lists them. | `crates/micold-client/tests/features_pr_changes.rs:1231`; `crates/micold-client/src/features/changes.rs:2884` |
| B3 | MED | Panel group heading render (`{heading} · {place}`, "not in the list" flag) has no render-level test; only the model's `PanelGroup::heading()` is asserted. T010's draw code is unverified. | `crates/micold-client/src/ui/changes.rs:184-200`; test at `features_pr_changes.rs:1329` |
| B4 | MED | Re-implemented expectation: the W20 delivery test computes the expected bytes with `prompt::build_pr` itself. Safe only because `p1_pr_example_gives_the_exact_bytes` pins the bytes in core. | `crates/micold-daemon/tests/review_send.rs:1295` |
| B5 | MED | Eager test: one test covers a PR send, a refusal for another PR and then a working-tree send. | `crates/micold-daemon/tests/review_send.rs:1183-1311` |
| B6 | LOW | Dead code in the same test: `all` built up, shadowed, then `let _ = all;`. | `crates/micold-daemon/tests/review_send.rs:1193-1260` |
| B7 | LOW | Magic index `panel_groups(&state)[3]`. | `crates/micold-client/tests/features_pr_changes.rs:1329` |
| B8 | LOW | No daemon-level restart test with a PR comment in it (SC-006 / US1 s5): covered by the store round-trip (`store.rs` version 3) and by the 946 restart test, which carries no PR comment. | `crates/micold-daemon/tests/review_edit.rs:460` |

Properties: tests are isolated and deterministic; core/client tests run in under a second; the daemon tests use the serialized `ENV` lock and real temp git repos, no sleeps found in the new tests.

### (c) Docs

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| C1 | MED | Stale: "a comment on a pull request's diff ... is only stored there once you open the entry's working tree". Since M3 a PR comment's move is sent as `Reanchor` (W21) and stored at once. | `docs/user-guide/reviewing-changes.md:256-258` |
| C2 | LOW | `tasks.md` names `features_changes.rs` for T008/T017/T025; the tests are in `features_pr_changes.rs`. | `specs/1149-pr-diff-review-comments/tasks.md` T008, T017, T025 |
| C3 | INFO | FR-017 satisfied: guide has the three sections and states nothing is posted to GitHub (`reviewing-changes.md:266-362`). | |

## Mutation results

No mutation tool in the profile. Six deliberate mutants, one at a time, on changed files only, each restored with `git checkout` and the touched suites re-run green (`review` lib 175 passed; `features_pr_changes` 59 passed). A sample, not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `prompt.rs:102` `!=` to `==` (switch-branch note) | T019/T012 | No | caught by 3 tests |
| `prompt.rs:146` `take(200)` to `take(201)` | T012 | No | boundary pinned |
| `comment.rs` `clear_sent` scope test removed | T002 | No | caught |
| `store.rs:70` PR comments no longer force version 3 | T003 | No | caught |
| `changes.rs:2694` cross-PR guard inverted | T025 | No | caught, but by whole-flow tests; a dedicated cross-PR test would be sharper |
| `changes.rs:2732` PR moves not sent as `Reanchor` | T025 | No | caught |

Coverage tooling: none (profile `coverage: null`).

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1, FR-001/002 | `a_pick_on_a_pull_request_file_builds_an_add_with_its_anchor`, `w18_a_pull_request_add_is_checked_and_stored_with_its_anchor` | Yes (daemon real; client at `State::update`) |
| US1 s2 | `a_pull_request_comment_is_edited_deleted_and_forgotten_with_its_worktree` | Yes |
| US1 s3, FR-014 | `nothing_is_picked_on_a_pull_request_diff_that_is_not_ready_text` | Partial (B2) |
| US1 s4, FR-003/004/013 | `comments_are_filtered_by_the_shown_scope`, `the_panel_groups_the_working_tree_first_...` | Partial (A1, B3) |
| US1 s5, FR-005, SC-006 | `store.rs` v3 tests, 946 restart test, forget test | Partial (B8) |
| US2 s1-6, FR-006/007/008/010, SC-002/003 | `w20_*`, `p1_pr_example_gives_the_exact_bytes`, `no_variant_asks_for_a_post_or_names_github` | Yes |
| US2 s5, FR-011 | `fr011_a_report_on_a_pull_request_comment_is_accepted_...` | Yes |
| US3 s1-4, FR-009 | `us3_*`, `review_git.rs` tests, `p3_note_*` | Yes |
| US4 s1-3, FR-012, SC-004 | `features_pr_changes.rs` M3 tests, `anchor.rs` HunkLines tests, `w21_*` | Yes |
| FR-015 | `tidy_emits_the_shown_scope`, `w19_*` | Yes |
| FR-016 | all above run with no window, session or network | Yes |
| FR-017 | docs, no test (CI user-guide gate) | n/a |

Untested or partly tested: FR-014 sub-cases (B2). Tests tracing to nothing: none found.

## What was not audited

- No mutation tool: only 6 hand-picked mutants; the daemon `review_send` (`state.rs`) and `GitCli` reads were not mutated (their tests were read, not mutated).
- Whole-diff `git diff 0ebf14ba..HEAD` includes other features merged from main (668 files); only 1149's commits and files were graded.
- Smell pass sampled `features_pr_changes.rs`, `review_send.rs` (W20 tests) and the `prompt.rs`/`store.rs`/`comment.rs` tests; `review_edit.rs`, `review_git.rs`, and the `us3_*` daemon tests were skimmed by name and diff only.
- Cross-platform byte-identity (SC-005), the visual pass, performance, and the layout snapshot gate were not assessed.
- Suite wall time includes lock waiting; true run time is under ~7 minutes.

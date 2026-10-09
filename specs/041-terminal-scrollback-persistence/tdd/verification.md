---
feature: 041-terminal-scrollback-persistence
verdict: PASS_WITH_GAPS
verified_at: 0d8737b7
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md (no override or preset found)
independent: yes (auditor did not write the tests)
suite: 6123 passed, 0 failed, 9 ignored, 471 binaries (MICOLD_SKIP_GH_LAUNCH_TEST=1, `cargo test --workspace`; ~118 min wall, almost all build-lock wait)
mutation: no cargo-mutants in the profile; 10 deliberate mutants on the feature's core store, schedule and daemon files: 7 killed, 3 survived (1 likely equivalent)
---

# TDD verification: 041 terminal history persistence

**Verdict: PASS_WITH_GAPS.** All 30 criteria have real tests and the suite is green; the decisive gaps are
a stale test list, test-after evidence for M5/M6, and two undetected mutants in data-loss guards.

## Test-first evidence

The list holds 166 rows: 71 DONE, 95 PENDING. tasks.md has 77 of 77 tasks ticked. Every sampled
PENDING row has a test (history_periodic_save, history_stop_request, history_setting,
history_damaged, history_removal, terminal_history_schedule, sandbox_real_history, client settings
tests), so the list was never updated after M2. Evidence class per milestone, from the cycle log
and history:

| Milestone | Behaviors | Class |
| --- | --- | --- |
| M1, M2 (U1-U64, A1-A6, A9, A10, A19) | red commits precede green (`84b421fb`, `91506214`, `d3a07daa`, `eca64ece`) | PROVEN |
| M3 (U65-U78) | schedule red `fb32a8ec`; saver red shown by stub afterwards (cycle 76) | PROVEN (U65-U71), LIKELY (U72-U78) |
| M4 (U79-U85, U132) | one commit `84f71510`; red run recorded in cycle log | LIKELY |
| M5 (U86-U105, U131, A11-A18) | log: "red not recorded", code and tests in one commit | TEST_AFTER |
| M6 (U12, U13, U29, U106, U107, A20-A24) | red by mutation after the code | TEST_AFTER |
| M7 (A25-A30, U108-U114), M8 (U115-U120, U136), M9 (U121-U130) | one commit each; Windows and sandbox reds are CI's | LIKELY |

Existing tests: none weakened. `0a7d1417` changed A19 from "no history" to the notice line, which
spec A20 requires. `030c1c30` cut U132 on Windows to 500 lines, documented and `cfg(windows)` only.
No skips, no `#[ignore]`, no thresholds touched.

## Findings

- F1 HIGH (docs): test-list.md marks 95 behaviors PENDING while all 77 tasks are ticked. Completion claims with no evidence on the list; fail-closed, so reported HIGH.
- F2 MED (docs): M5 and M6 are test-after (F-class above). Cannot be repaired after the fact; keep as a recorded exception.
- F3 MED (survivor M3): `crates/micold-core/src/terminal_history/store.rs:300` (the re-check of `enabled` under the lock in `save`) can be deleted and all 33 store tests pass. U97 `a_save_racing_turning_saving_off_leaves_no_file` is a probabilistic race.
- F4 MED (survivor M5): `crates/micold-daemon/src/state.rs:3363` (`sweep_saved_histories` skips when the catalog `load_status` is not `Loaded`) can be deleted and `history_removal` stays green. A recovered empty catalog would delete every saved history (FR-024 data loss). No test names it.
- F5 LOW (survivor M10): `store.rs:128`. `set_enabled(true)` after an unlistable directory does not re-delete; no test turns saving on after that failure (U96 only covers the off path).
- F6 LOW (survivor M8, likely equivalent): `store.rs:222` `!name.starts_with('.')` in `sweep`; no real file name reaches it.
- F7 LOW: `history_stop_request.rs::a_save_that_blocks_holds_the_unwind_for_at_most_three_seconds` has no outer timeout. With the 3 s bound mutated to 3000 s (M6) it hangs rather than fails. Killed only by hang.
- F8 LOW: not run here: 6 `sandbox_real_history` tests (feature-gated, need podman) and all `cfg(windows)` cases (U57, U115-U120). Their pass state rests on the cycle log and CI.

## Mutation (deliberate, one at a time, restored by `git checkout`, tree clean, targeted suites re-run green)

| Id | Mutant | Result |
| --- | --- | --- |
| M1 | `schedule.rs` `>= SAVE_SPACING` to `>` | killed (U68-U71) |
| M2 | `store.rs` drop `forgotten` guard in `save` | killed (2 tests) |
| M3 | `store.rs:300` drop the `enabled` re-check | SURVIVED (F3) |
| M4 | `store.rs` `load` ignores `undeleted` | killed |
| M5 | `state.rs:3363` drop the catalog-loaded guard | SURVIVED (F4) |
| M6 | `history.rs` stop-save bound x1000 | killed by hang only (F7) |
| M7 | `store.rs` `forget` skips the `.tmp` file | killed |
| M8 | `store.rs:222` drop the dot-name check | survived, likely equivalent (F6) |
| M9 | `store.rs` never `Unchanged` | killed (3 tests) |
| M10 | `store.rs:128` `set_enabled(true)` skips re-delete | SURVIVED (F5) |

## Traceability

All 30 criteria (US1 1-10, US2 1-8, US3 1-6, US4 1-6) map to at least one test through a real
`DaemonState`, a real service process (`SIGTERM`/`SIGINT`/`SIGHUP`), or the container runtime. US2-1
(the control on screen) is the render-free reader plus the recorded visual pass (T076). Every sampled
`traces` target exists. Stale list rows are F1, not missing tests.

## Smell pass

Sampled: schedule, store, removal, stop-request, setting tests. No tautological or re-implemented
expectations found; assertions carry rule messages; the suite uses the `support/history.rs`
helpers. Race tests (U97, U109) loop 50 rounds rather than control the interleaving (F3).

## What was not audited

Full line-by-line smell pass of all 7,363 lines of feature tests (sampled). Windows and sandbox tests
(F8). Mutation of `format.rs`, `text.rs`, `owner_only.rs`, `history.rs` capture/seed and the client
settings code (no mutants run there). Coverage (not in the profile). Test speed beyond the suite
total. `tasks.md` was not touched (no remediation section appended).

## Resolved at close

F1 fixed (91 test-list rows refreshed; A11, U102, U104, U105 stay PENDING). F4, F5, F7 got tests (cycle-log close section). F2 recorded as an exception. F3 not fixed (needs a hook inside `save`); F6 equivalent; F8 rests on CI.

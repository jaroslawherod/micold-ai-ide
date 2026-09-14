---
feature: 027-sandboxed-daemon-runtime
scope: BUG-004 increment (Phases 18 and 19, T167–T188), committed
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: 139bd640
behaviors: 32 # A1–A4, U1–U12, U14–U29 (no U13 on the list)
proven: 0
likely: 27
test_after: 1
no_test: 0
not_applicable: 4
high_smells: 1
criteria_total: 8
criteria_covered: 8 # every criterion has a test through update_inner or core; none end to end through the app with a real runtime
mutation_score: 94 # 16 of 17 deliberate mutants caught; scope: the 5 source files BUG-004 changed; no mutation tool in the profile
mutants_survived: 1 # not equivalent
suite: 2962 passed, 0 failed, 2 ignored, ~58s test time # cargo test --workspace after all mutants restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-004 increment (re-audit)

**Verdict: FAIL.** Deleting `app.sandbox.answered()` from `on_connected` (mutant N3) leaves the
whole suite green. That is inside U26, which is marked `DONE`. U26's fixture connects with a catalog
whose projects differ from the boot plan. So `adopt_mount_set` marks the sandbox `Stale`, and `Stale` is
never coming up whether or not the service answered. The test passes for a reason unrelated to the
behavior it names.

U4 is the one other blocking item: it is still `TEST_AFTER`. M3 now shows the test is strong, but its
ordering cannot be shown.

Every finding of the previous report (2026-09-13, 16 findings, 6 survivors) is cleared. M10, M11 and
M13–M16 are all caught now. T187/T188 closed FR-036b's banner and card gaps with tests that fail
against their mutants (N1, N2, N4–N9).

**The audit is not independent.** The session that wrote T173–T188 also ran this audit, and no
fresh-context subagent was used: the user asked for none. Every cited line was re-opened for this
report, and every mutant reported was run.

## Test-first evidence

Commits: `eb3edf21` (Phase 18 and Phase 19 code with their tests), `cd7fc151` (cycle log cycles 1–12
and the T172 record), `f8f12e4a` (T187/T188 code and tests, with cycles 13–18), `139bd640` (T178
re-run evidence). Each code commit carries its tests in the same commit, but it squashes several
cycles. So history cannot show the order inside a commit, and every red-recorded behavior is
`LIKELY`, not `PROVEN`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | LIKELY | cycle 1 red; test and code together in `eb3edf21` |
| A2 | LIKELY | cycle 8 red; `eb3edf21` |
| A3 | LIKELY | red via U21 (cycle 9); `eb3edf21` |
| A4 | LIKELY | red via U23 (cycle 11); `eb3edf21` |
| U1, U2, U3 | LIKELY | Phase 18 reds copied into `cycle-log.md:13-22` from the session scratchpad after the fact; U3 re-driven by T183 |
| U4 | **TEST_AFTER** | never red: a `None` stub already satisfied it (`cycle-log.md:13-22`). M3 below now catches it, so strength is fine; ordering is not |
| U5, U6, U7, U8, U9 | LIKELY | Phase 18 reds, recorded after the fact; `eb3edf21` |
| U10, U11, U12, U18 | NOT_APPLICABLE | characterization (`BASELINE`); N10 catches U12, M13/M14 catch U10 |
| U14, U15, U16, U17, U19, U20 | LIKELY | cycles 2–7 reds; `eb3edf21` |
| U21, U22, U23 | LIKELY | cycles 9–11 reds; `eb3edf21` |
| U24, U25, U26, U27, U28, U29 | LIKELY | cycles 13–18 reds; test and code together in `f8f12e4a`; the red line numbers match the file today |

**Existing tests weakened: none.** Across `b53e3449..139bd640`, the 13 deleted lines in the test files
are renames and message changes from T182/T185, each recorded in the cycle log. No test is skipped,
filtered or loosened, and no threshold changed.

**`tasks.md` against the list:** T173–T188 are all `[X]`, and every behavior they name is `DONE` or
`BASELINE`. No task has been ticked without its evidence. T181 carries no behavior marker (it is out of
scope on the list).

**Log against history:** cycle 15's red for U26 was genuine when recorded: `awaiting_service` was then
a flag that nothing cleared. Cycle 18 made `is_coming_up` honour the wait only while `Running`, and
U26 stopped proving its behavior at that step without anyone noticing (finding 1). Cycles 13–18 record
`commit: see git history` rather than a SHA (finding 9).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Surviving mutant inside `DONE` U26; the fixture's catalog makes the sandbox `Stale`.** `the_service_answers` connects with `snapshot_with("/repo/demo", …)`, while `app_with_a_failed_sandbox`'s plan has `projects: Vec::new()`. `adopt_mount_set` sees the mismatch and calls `mounts_changed()`, so `Running` becomes `Stale`. `Sandbox::is_coming_up` requires `Running` for the wait to count, so the disconnect is reported whether or not `answered()` ran. N3 (drop `answered()`) passes. In the application, that mutant hides the banner on the first disconnect after a service answered, contrary to FR-027. The test should set up a catalog that matches the plan (or assert the state is `Running` before the disconnect), and then fail against N3. | `crates/micold-client/src/main.rs:3264` (catalog), `:1746` (plan), `:3280-3297` (U26); `crates/micold-client/src/shell/daemon_sync.rs:439-443`; `crates/micold-client/src/features/sandbox.rs:286-289` |
| 2 | HIGH (verdict rule) | **U4 is `TEST_AFTER`.** `absence_only_brings_up_a_sandbox_that_has_failed` passed on its first run, and no deliberate-mutant red was recorded at the time. M3 (`Failed \| Stale` brought up) fails it today, so the test is sound; the loop's evidence is what is missing. | `crates/micold-core/tests/sandbox_state.rs:547`; `specs/027-sandboxed-daemon-runtime/tdd/cycle-log.md:13-22` |
| 3 | MED | **Non-specific assertion on the bring-up task.** `work.units() > 0` accepts any non-empty task, such as a notification or a log write, not specifically the bring-up. N6 and M13 are caught only because their replacement is `Task::none()`. A mutant that returns some other task would pass. | `crates/micold-client/src/main.rs:3025`, `:3195` |
| 4 | MED | **No automated test crosses the application with a real runtime.** T181 drives core `container_lost`, `service_absent` and `bring_up` in its own loop, which re-implements the client's orchestration. The app's `Msg::Lost` → `bring_up_again` wiring, the sleep before an unattended bring-up, and the `awaiting_service` grace meet a real container only in the manual T172/T178 passes. | `crates/micold-core/tests/sandbox_real_lifecycle.rs:512`, `:652-668` |
| 5 | LOW | **`awaiting_service` outlives `Running`, neutralised only by the state check.** `failed()` and `mounts_changed()` leave it set. U29 pins the `Failed` path (N5 caught). The `Stale` path is exercised only by accident, in U26 (finding 1), and no test states it. | `crates/micold-client/src/features/sandbox.rs:303-309`, `:317-319`, `:286-289` |
| 6 | LOW | **`REFUSALS_WHILE_STARTING = 1` rests on one manual run.** N1 and N2 pin the value, but nothing measures that one reconnect interval covers a daemon's listen time on a slower host. That time is the whole reason for the grace. | `crates/micold-client/src/features/sandbox.rs:237`; `evidence/performance.md` T178 re-run |
| 7 | LOW | **Duplicated setup.** The same `in_flight` state array is built three times. | `crates/micold-client/src/main.rs:3073`, `:3121`, `:3152` |
| 8 | LOW | **Test side effect outside the sandbox.** `connection_failed` reaches `on_connect_failed`, which calls `log_line`, which appends to the developer's real client log under `ProjectDirs`. This is the remainder of the previous report's finding 16. There is also an assertion with no rule message. | `crates/micold-client/src/main.rs:956-962` via `:2989`; `:3437` |
| 9 | LOW | **Cycle log names no commit.** Cycles 13–18 say `commit: see git history`, and cycles 1–12 say `not committed`, although both landed (`f8f12e4a`, `eb3edf21`). Ordering evidence has to be re-derived from `git log`. | `specs/027-sandboxed-daemon-runtime/tdd/cycle-log.md:311`, `:324`, `:333`, `:342`, `:353`, `:366` |

Checked and clean (smell pass): tautological assertion, re-implemented expectation, over-mocked
collaborators, vacuous assertion, self-approving snapshot, empty or skipped test, assertion roulette,
mystery guest, non-deterministic, framework under test, redundant test, bypassed test utility
(`a_started_sandbox()` is shared), conditional logic (the `in_flight` loops use fixed data), foreign
style (the tests follow the neighbouring `main::tests` idiom, with `update_inner` and rule messages). No
secrets, and no repository content addressed to the auditor.

Sleepy test, judged acceptable: U19 sleeps a real 200 ms (`shell/sandbox.rs:588`). It asserts only a
lower bound on elapsed time, which a sleep guarantees, so it cannot flake; it costs 0.2 s.

Properties:
- **Determinism:** good.
- **Speed:** good. The client binary's 136 tests run in about 1 s once built, and the workspace in about 58 s.
- **Specificity:** good, except findings 3 and 8.
- **Refactor-insensitivity:** acceptable. Assertions go through `connection_status`,
  `persistent_notice()`, `fallback_offer()` and notification level, not prose.
- **Fixture realism:** weak in finding 1, where a catalog/plan mismatch drives the result.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`), so the mutants were deliberate.
Each was applied alone by `mutants.py`, restored from a snapshot and sha256-verified (`restored
sha_ok=True` for all 17). The five files were then checked against the pre-audit hashes (all `OK`),
and `git status` was clean.

- **Commands:** client mutants ran `scripts/build-lock.sh cargo test -p micold-client --bin
  micold-ai-ide`; core mutants ran `… -p micold-core --test sandbox_state`; M15 ran both.
- **Full suite afterwards:** `scripts/build-lock.sh cargo test --workspace` → 2962 passed, 0 failed,
  2 ignored. The first attempt died in the linker (`ld terminated with signal 7 [Bus error]`), an
  environmental crash with no test run; the re-run is the result recorded.
- **Build freshness:** each mutant run shows `Compiling micold-client` and a distinct expected failure,
  so the fast rebuilds were not stale.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| N1 `features/sandbox.rs:237` `REFUSALS_WHILE_STARTING` 1 → 0 | U28 | No | caught at `main.rs:3341` |
| N2 same, 1 → 2 | U27 | No | caught at `main.rs:3315` |
| N3 `daemon_sync.rs` `app.sandbox.answered()` removed | U26 | **Yes** | real: the fixture makes the sandbox `Stale` (finding 1) |
| N4 `daemon_sync.rs` `app.sandbox.service_refused()` removed | U27 | No | caught at `main.rs:3315` |
| N5 `features/sandbox.rs` wait counted in any state | U29 | No | caught at `main.rs:3379` |
| N6 `shell/sandbox.rs` `Msg::Lost` never brings up | U24 | No | caught at `main.rs:3194` (see finding 3) |
| N7 `main.rs` `connection_status` reads the bare state | U25, U28 | No | caught at `:3250`, `:3341` |
| N8 `features/sandbox.rs` `started()` sets no wait | U25, U28 | No | caught at `:3250`, `:3341` |
| N9 `daemon_sync.rs` `refused_dial` silence reads the bare state | U28 | No | caught at `main.rs:3346` |
| N10 `daemon_sync.rs` `LocalSandbox` guard in `bring_up_again` removed | U12, U15 | No | caught at `main.rs:3518` |
| M3 `lifecycle.rs` `Failed \| Stale` brought up on absence | U4 | No | caught at `sandbox_state.rs:552` |
| M10 `shell/sandbox.rs` wait before an unattended bring-up removed | U19 | No | previously survived; caught at `shell/sandbox.rs:602` |
| M11 `daemon_sync.rs` `BringUp { after: ZERO }` | U17 | No | previously survived; caught at `main.rs:3063` |
| M13 `daemon_sync.rs` bring-up task replaced by `Task::none()` | A1, A2 | No | previously survived; caught at `main.rs:3024`, `:3468` |
| M14 `features/sandbox.rs` `self.unattended = again.budget` dropped | A2, U9, U17 | No | previously survived; caught at `main.rs:3458`, `:3558`, `:3063` |
| M15 `lifecycle.rs` `Probing`, `Starting` not coming up | U14, U16, A3, U24 | No | previously survived; caught in core (`sandbox_state.rs:570`) and client (`main.rs:3098`, `:3135`, `:3208`) |
| M16 `shell/sandbox.rs` production `observe` → `&mut \|_\| {}` | U20 | No | previously survived; caught at `shell/sandbox.rs:561` |

Score: 16 of 17 caught (94%). The one survivor is inside a `DONE` behavior and is not equivalent. The
sample favours the behaviors FR-036a/b depend on; it is not exhaustive. Coverage is not configured,
so it was not run.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-002a (every placement starts its service when absent) | A1, U1, U5, U12, U15, U24 | Partial: T181 real runtime through core functions, not the app; T172/T178 manual |
| FR-036a (bring back up; bounded, spaced, each reports why) | A2, A4, U2, U3, U17, U19, U23 | No: bound exhaustion never met a real runtime; T178 manual covers one recovery |
| FR-036b (report the stage; no connection failure while starting) | A3, U6, U14, U16, U21, U22, U24, U25, U28, U29 | No: T178 re-run (manual, Xvfb) only |
| SC-004c (progress measured at the application; no connection failure) | A3, U7, U8, U20, U21, U25 | No: T178 re-run (manual) only |
| US6 scenario 9 | A1, A4, U23 | Partial: T181 (core loop), T172 manual |
| S-6 (`data-model.md` §7) | A2, U1, U2, U3, U9, U14–U17, U19 | No |
| S-7 | U4, U12, U15 | No |
| C-8 caller half (`contracts/container-runtime.md`) | U7, U20 | No: T181 calls `bring_up` directly, not the app's stream |

**Untested criteria:** none at unit or `update_inner` level. No criterion has an automated test through
the application entry point against a real runtime (finding 4).

**Tests outside these eight criteria:** U10 and U11 (FR-034), U18 (FR-036), U22 and U24 (also
FR-035a), U26 and U27 (FR-027). These trace to the spec's existing requirements, not to nothing.

## What was not audited

- **T181 was not re-run.** It needs the `micold-daemon:dev` image, and rebuilding that image needs
  approval because it is shared across worktrees. Its last recorded verdict is in `cycle-log.md:269`.
- **The rest of feature 027 (T001–T166)** was out of scope; it has no test-list evidence.
- **Mutation was a hand sample:** 17 deliberate mutants on the five changed source files, with no tool
  and no exhaustive run. Coverage was not measured (no tool configured).
- **macOS and Windows** were not run; everything here ran on Linux.
- **Visual and timing behavior** (banner and card around a real bring-up, the ~0.1 s before `Lost`
  arrives) rests on the manual T172/T178 passes, run on Xvfb + lavapipe. It was not re-assessed here.
- **Independence:** the tests' author ran the whole audit, smell pass included, with no fresh-context
  reviewer.

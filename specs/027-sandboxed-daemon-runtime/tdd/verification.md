---
feature: 027-sandboxed-daemon-runtime
scope: BUG-004 increment (Phases 18–22, T167–T200), committed
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: a23bb637
behaviors: 34 # A1–A4, U1–U12, U14–U31 (no U13 on the list)
proven: 0
likely: 30
test_after: 0
no_test: 0
not_applicable: 4
high_smells: 1
criteria_total: 8
criteria_covered: 8 # every criterion has a test through update_inner or core; none end to end through the app with a real runtime
mutation_score: 86 # 24 of 28 deliberate mutants caught; scope: the 6 source files BUG-004 changed; no mutation tool in the profile
mutants_survived: 4 # 3 real (X3, X5, X6), 1 contrived (X4); none equivalent
suite: 2964 passed, 0 failed, 2 ignored, 143s wall incl. build # cargo test --workspace after X1–X5 restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-004 increment (fifth audit)

**Verdict: FAIL.** A bring-up handed back with its messages discarded still passes every test. T200
made A1 and U24 hold the returned work and count a live token carried inside the bring-up's stream. That
catches a bring-up swapped for other work (X1, X2). It does not show that the work *delivers* anything.
`.discard()` keeps the stream, and therefore the token, alive while throwing away every message it
produces. X3 puts `.discard()` inside `BringUp::task()`, which breaks the startup bring-up and both
recovery paths. X5 and X6 put it on the `Msg::Lost` and refused-dial returns. All 138 client tests
stay green under each. In the application, the sandbox stays in `Probing`: the runtime starts the
container, but `Progress`, `Started` and `Failed` never reach `update`, so the stage never advances and
the client never dials. No test runs a bring-up task: U20 drives `BringUp::stream`, one layer below
`task()`. A1, U24 and the recorder read the task's shape, and a `Task` is opaque to them.

Everything the fourth report found in scope is cleared. X1 now fails U24 (`main.rs:3209`), X2 fails A1
(`main.rs:3040`), and the 22 older mutants are all still caught.

**The audit is not independent.** The session that wrote T200 also ran this audit, and no fresh-context
subagent was used: the user asked for none. Every cited line was re-opened for this report, and every
mutant listed was run at `a23bb637`.

## Test-first evidence

Commits: `eb3edf21` (Phases 18–19 code and tests), `cd7fc151` (cycles 1–12), `f8f12e4a` (cycles 13–18),
`6b802791` (Phase 20), `ae018388` (Phase 21), `af3d8afd` (fourth report), `a23bb637` (T200, with its
cycle-log entry). Every code commit squashes its test changes with the rest. History cannot show order
inside a commit, so every behavior with a recorded red is `LIKELY`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | LIKELY | cycle 1 red; `eb3edf21`. T191 in `6b802791`, T197 in `ae018388`. T200's red (`left: 0, right: 1` at `main.rs:3040`) was recorded against the test before the seam existed, and X2 now fails it; `a23bb637` |
| A2, A3, A4 | LIKELY | cycles 8, 9 and 11 reds; `eb3edf21` |
| U1, U2, U3, U5–U9 | LIKELY | Phase 18 reds, copied into the log after the fact; `eb3edf21` |
| U4 | LIKELY | T190 records M3's red after the commit. Weakest `LIKELY` on the list; M3 re-run and caught |
| U10, U11, U12, U18 | NOT_APPLICABLE | characterization (`BASELINE`); N10 catches U12, M14 and N12 catch U10 |
| U14–U23 (except U18) | LIKELY | cycles 2–11 reds; `eb3edf21`. U20 tests `stream`, not `task` (finding 1) |
| U24 | LIKELY | cycle 13 red; `f8f12e4a`. T197 in `ae018388`. T200's red at `main.rs:3209`, and X1 now fails it; `a23bb637` |
| U25–U29 | LIKELY | cycles 14–18 reds; `f8f12e4a` |
| U30 | LIKELY | cycle 19 real red; `6b802791`. N11 and T199a both fail it |
| U31 | LIKELY | cycle 20 pin with N3's deliberate-mutant red; `6b802791` |

**Existing tests weakened: none.** In `ae018388..a23bb637` the only test changes add one assertion each to
A1 (`main.rs:3040-3044`) and U24 (`main.rs:3209-3213`): `BringUp::live_tasks() == 1`, with a rule
message. No assertion was removed or loosened. No test was skipped, filtered or renamed, and no threshold
changed. The production change is a `#[cfg(test)]` wrapper in `BringUp::task()`
(`shell/sandbox.rs:215-232`) and the `live_tasks()` reader (`:244`).

**`tasks.md` against the list:** T200 is `[X]`, and A1 and U24 are `DONE`. T200's stated proof was met and
re-run here: X1 fails U24, X2 fails A1, and N12, N13, T191a and T191b still fail them. The tick is not a
false completion. Its intent, that the returned work is the bring-up, is held only for the task's
identity, not for its output (finding 1).

**Log against history:** T200's entry and the gates after Phase 22 match `a23bb637` and the runs above.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Survivors inside `DONE` A1, U24 and U20: a bring-up whose messages are discarded passes.** `Task::discard()` keeps a task's units and its stream alive, so `scheduled()`, `units() == 1` and `live_tasks() == 1` all hold while no message reaches `update`. X3 (`iced::Task::stream(stream).discard()` in `BringUp::task()`), X5 (`return bring_up.task().discard();` in `Msg::Lost`) and X6 (`map_or_else(Task::none, \|b\| b.task().discard())` in the refused-dial path) each leave all 138 client tests green. X3 also covers startup, which reaches the same `task()` through `boot()`. U20 proves the *stream* reports `Probing` first and the outcome last, but nothing proves that `task()` hands that stream's messages to the runtime. Every check of the returned work reads its shape. A test has to run the work and see the bring-up's first message come out of it. `iced` 0.14 re-exports `Task` but not `iced_runtime::task::into_stream`, the public function that turns a `Task` into its stream. `iced_runtime` is already in `Cargo.lock` at the version `iced` uses. Naming it as a dev-dependency of `micold-client` would add no new crate, but it **is a dependency change and needs the user's approval**. The test must also not shell out to a real runtime, so `task()` needs a runner seam (for example `task_with(runner)`, with `task()` passing `SystemRunner`). | `crates/micold-client/src/shell/sandbox.rs:215-232` (`task`), `:184` (`boot`), `:457` (`Msg::Lost`), `:586` (U20); `crates/micold-client/src/shell/daemon_sync.rs:293`; `crates/micold-client/src/main.rs:3024-3050` (A1), `:3188-3218` (U24) |
| 2 | MED | **Two save tests write the developer's real `settings.json` on every client test run (outside this increment).** Unchanged from the fourth audit. `settings_saved_sends_settings_set_to_a_connected_daemon` and `settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected` save through `base_app()` (`Capabilities::real()`). The file's mtime moved during this audit's mutant runs (13:33:58) and full suite (13:37:21). No task added: the tests predate feature 027's BUG-004 work. The fix is `app.caps = Capabilities::real().without_settings()`, as U30 does. | `crates/micold-client/src/main.rs:2583`, `:2630`; `main.rs:1694` (`base_app`) |
| 3 | LOW | **Test awareness in production code, now three hooks.** The bring-up recorder and `live_tasks` token (`shell/sandbox.rs:198-248`), and `log_line`'s `cfg!(test)` return (`main.rs:959`). The token is compiled out of release builds and justified by `Task` being opaque. Finding 1's remedy, reading the task directly, would make both bring-up hooks unnecessary. | as cited |
| 4 | LOW | **The token seam can be defeated by keeping the task alive without returning it.** X4 (`std::mem::forget(bring_up.task()); return iced::Task::done(Message::EscapePressed);` in `Msg::Lost`) passes. No realistic change leaks a task on purpose, so this is recorded as the seam's limit, not a gap to close separately. Finding 1's remedy also catches it. | `crates/micold-client/src/shell/sandbox.rs:457` |
| 5 | LOW | **No automated test crosses the application with a real runtime; documented, not closed.** Unchanged (T192). Finding 1 is the third survivor class in a row that such a test would have caught. | `crates/micold-core/tests/sandbox_real_lifecycle.rs:652-668`; `evidence/us6-failures.md` |
| 6 | LOW | **T194's margin is an inference from frames.** Unchanged; the entry states its limits. | `evidence/performance.md`, T194 section |

The fourth report's finding 1 (X1, X2) is cleared.

Checked and clean (smell pass over the T200 test changes): tautological assertion, doubled subject,
re-implemented expectation, assertion roulette (both assertions carry a rule message), mystery guest,
conditional logic, redundant test, bypassed test utility, and foreign style (the assertions match the
`scheduled()` checks beside them). They are not vacuous: X1 and X2 prove it. They are insufficient, which is
finding 1, a test-strength gap rather than a smell. No secrets, and no repository content addressed to
the auditor.

Properties:
- **Isolation:** the thread-local token is per test thread, so it passes parallel and serial runs. Two
  older tests are not isolated (finding 2).
- **Determinism:** good.
- **Speed:** good. 138 client tests finish in about 2s including the incremental rebuild.
- **Specificity:** good for a dropped or swapped bring-up. Missing for a silenced one (finding 1).
- **Refactor-insensitivity:** acceptable. The token counts a task whether or not it is batched, so batching
  the bring-up with other work fails on `units() == 1` and nothing else.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`), so the mutants were deliberate.
`mutants.py` applied each one alone, rebuilt, restored it from a snapshot and verified its sha256
(`restored sha_ok=True` for all 28). Every client run's output contains `Compiling micold-client`.
`git status` was clean afterwards.

- **Commands:** client mutants ran `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
  Core mutants ran `… -p micold-core --test sandbox_state`. M15 ran both.
- **Full suite afterwards:** `scripts/build-lock.sh cargo test --workspace` → 2964 passed, 0 failed,
  2 ignored. X6 ran after it and was restored and sha-verified the same way.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| N1 `REFUSALS_WHILE_STARTING` 1 → 0 | U28, U30 | No | caught |
| N2 same, 1 → 2 | U27 | No | caught |
| N3 `answered()` removed | U26, U31 | No | caught |
| N4 `service_refused()` removed | U27 | No | caught |
| N5 wait counted in any state | U29 | No | caught |
| N6 `Msg::Lost` never brings up | U24 | No | caught |
| N7 `connection_status` reads the bare state | U25, U28, U30 | No | caught |
| N8 `started()` sets no wait | U25, U28, U30 | No | caught |
| N9 `refused_dial` silence reads the bare state | U28, U30 | No | caught |
| N10 `LocalSandbox` guard removed | U12 | No | caught |
| N11 `is_coming_up` wait for `Running` only | U30 | No | caught |
| N12 refused-dial bring-up built, then `Task::none()` | A1, A2 | No | caught |
| N13 `Msg::Lost` bring-up built, then `Task::none()` | U24 | No | caught |
| M3 `Failed \| Stale` brought up on absence | U4 | No | caught |
| M10 wait before an unattended bring-up removed | U19 | No | caught |
| M11 `BringUp { after: ZERO }` | U17 | No | caught |
| M14 `self.unattended = again.budget` dropped | A2, U9, U17 | No | caught |
| M15 `Probing`, `Starting` not coming up | U14, U16, A3, U24 | No | caught in core and client |
| M16 production `observe` → `&mut \|_\| {}` | U20 | No | caught |
| T191a refused-dial bring-up → `Task::done(EscapePressed)` | A1 | No | caught |
| T191b `Msg::Lost` bring-up → `Task::done(EscapePressed)` | U24 | No | caught |
| T199a `survive_logout_changed()` removed from the save | U30 | No | caught |
| X1 `Msg::Lost` bring-up built, then `Task::done(EscapePressed)` | U24 | No | **previously survived**; caught at `main.rs:3209` |
| X2 refused-dial bring-up built, then `Task::done(EscapePressed)` | A1, A2 | No | **previously survived**; caught at `main.rs:3040` |
| X3 `BringUp::task()` returns `Task::stream(stream).discard()` | U20, A1, U24 | **Yes** | real: no bring-up at startup or in recovery ever reports, so the sandbox stays in `Probing` (finding 1) |
| X5 `Msg::Lost` returns `bring_up.task().discard()` | U24 | **Yes** | real: a stopped container is started but the client never learns it (finding 1) |
| X6 refused-dial returns `b.task().discard()` | A1, A2 | **Yes** | real: a failed sandbox is restarted but the client never learns it (finding 1) |
| X4 `Msg::Lost` leaks the task with `mem::forget`, returns `Task::done(EscapePressed)` | U24 | **Yes** | contrived: defeats the token by construction (finding 4) |

X3–X6 are new, aimed at the token check T200 added.

Score: 24 of 28 caught (86%). Three survivors are inside `DONE` behaviors and are real; one is contrived.
None is equivalent. The sample favours FR-002a, FR-036a and FR-036b and every assertion Phases 20–22
changed; it is not exhaustive. Coverage is not configured, so it was not run.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-002a (every placement starts its service when absent) | A1, U1, U5, U12, U15, U24 | Partial: T181 drives core against a real runtime, not the app; T172/T178 manual. **A silenced bring-up is not caught on any path (finding 1)** |
| FR-036a (bring back up; bounded, spaced, each reports why) | A2, A4, U2, U3, U17, U19, U23 | No: T178 manual covers one recovery |
| FR-036b (report the stage; no connection failure while starting) | A3, U6, U14, U16, U21, U22, U24, U25, U28, U29, U30 | No: T178 re-run (manual, Xvfb) only. **The stage reaching `update` from a real task is untested (finding 1)** |
| SC-004c (progress measured at the application; no connection failure) | A3, U7, U8, U20, U21, U25 | No: T178 re-run (manual) only. U20 measures the stream, not what the application receives |
| US6 scenario 9 | A1, A4, U23, U24 | Partial: T181 (core loop), T178 manual; gap recorded by T192 |
| S-6 (`data-model.md` §7) | A2, U1, U2, U3, U9, U14–U17, U19 | No |
| S-7 | U4, U12, U15 | No |
| C-8 caller half (`contracts/container-runtime.md`) | U7, U20 | No: T181 calls `bring_up` directly |

**Untested criteria:** none at unit or `update_inner` level. No criterion has an automated test through
the application entry point against a real runtime (finding 5). No test runs the `Task` any update returns
(finding 1).

**Tests outside these eight criteria:** U10 and U11 (FR-034), U18 (FR-036), U26, U27 and U31 (FR-027).
These trace to existing requirements.

## What was not audited

- **T181 was not re-run.** It needs the shared `micold-daemon:dev` image, and rebuilding that image needs
  approval.
- **T178 (visual pass) was not re-run.** No Phase 20–22 change was checked on a display.
- **The rest of feature 027 (T001–T166)** was out of scope; finding 2 was noticed, not audited.
- **Mutation was a hand sample:** 28 deliberate mutants on the six changed source files, with no tool.
  Coverage was not measured (no tool configured).
- **Finding 1's remedy was not tried.** Whether `into_stream` can yield the bring-up's first message
  without touching a real runtime was reasoned from the iced 0.14 source, not built.
- **T194's numbers** were read from the report, not re-measured.
- **macOS and Windows** were not run; everything here ran on Linux.
- **Independence:** the tests' author ran the whole audit, smell pass included, with no fresh-context
  reviewer.

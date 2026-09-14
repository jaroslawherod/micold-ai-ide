---
feature: 027-sandboxed-daemon-runtime
scope: BUG-005 increment (Phases 18–23, T167–T201), committed
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: b983395e
behaviors: 34 # A1–A4, U1–U12, U14–U31 (no U13 on the list)
proven: 0
likely: 30
test_after: 0
no_test: 0
not_applicable: 4
high_smells: 1
criteria_total: 8
criteria_covered: 8 # every criterion has a test through update_inner or core; none end to end through the app with a real runtime
mutation_score: 90 # 28 of 31 deliberate mutants caught, all run at b983395e; scope: the 6 source files BUG-005 changed; no mutation tool in the profile
mutants_survived: 3 # X7, X8, X9; not equivalent
suite: 2964 passed, 0 failed, 2 ignored, 512s wall incl. build and a wait for another worktree's build lock # cargo test --workspace after X7–X9 restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-005 increment (sixth audit)

**Verdict: FAIL.** A bring-up that delivers its first message and drops how it ended still passes every
test. T201 made A1 and U24 run the work an update returns, which catches a silenced bring-up (X3, X5, X6)
and a leaked one (X4). But `first_message` stops at the first message, and the assertion checks only that
it is `Progress(Probing)`. X7 keeps only the first message of every bring-up task (`take(1)` in
`BringUp::task()`), and X8 and X9 keep only `Progress` on the `Msg::Lost` and refused-dial paths. All 138
client tests stay green under each. In the application, `Started` and `Failed` never reach `update`. A
bring-up that fails is never recorded, so no card appears and no attempt is counted. A bring-up that
succeeds leaves the sandbox in `Probing` until a dial happens to connect. Both assertion messages say "the
work handed back has to report the bring-up's stages". The tests check one stage.

Everything the fifth report found is cleared: X3–X6 fail at the new assertions (`main.rs:3085`,
`:3262`). The dependency T201 needed was approved by the user and is scoped to dev builds.

**The audit is not independent.** The session that wrote T201 also ran this audit, and no fresh-context
subagent was used: the user asked for none. Every cited line was re-opened for this report, and every
mutant listed was run at `b983395e`.

## Test-first evidence

Commits: `eb3edf21` (Phases 18–19 code and tests), `cd7fc151` (cycles 1–12), `f8f12e4a` (cycles 13–18),
`6b802791` (Phase 20), `ae018388` (Phase 21), `a23bb637` (T200), `ea715f5c` (fifth report), `b983395e`
(T201, with its cycle-log entry). Every code commit squashes its test changes with the rest. History
cannot show order inside a commit, so every behavior with a recorded red is `LIKELY`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | LIKELY | cycle 1 red; `eb3edf21`. Strengthened by T191, T197, T200 and T201. T201's own red is a deliberate-mutant red (X3, X6 at `main.rs:3085`), recorded as such in the log; the log states that the assertion passed on first run |
| A2, A3, A4 | LIKELY | cycles 8, 9 and 11 reds; `eb3edf21` |
| U1, U2, U3, U5–U9 | LIKELY | Phase 18 reds, copied into the log after the fact; `eb3edf21` |
| U4 | LIKELY | T190 records M3's red after the commit. Weakest `LIKELY` on the list; M3 re-run and caught |
| U10, U11, U12, U18 | NOT_APPLICABLE | characterization (`BASELINE`); N10 catches U12, M14 and N12 catch U10 |
| U14–U23 (except U18) | LIKELY | cycles 2–11 reds; `eb3edf21` |
| U24 | LIKELY | cycle 13 red; `f8f12e4a`. Strengthened by T197, T200 and T201. T201's red is X3, X4 and X5 at `main.rs:3262` |
| U25–U29 | LIKELY | cycles 14–18 reds; `f8f12e4a` |
| U30 | LIKELY | cycle 19 real red; `6b802791`. N11 and T199a both fail it |
| U31 | LIKELY | cycle 20 pin with N3's deliberate-mutant red; `6b802791` |

**Existing tests weakened: none.** In `a23bb637..b983395e` the only removed test lines are A1's and U24's
`let mut app = app_with_a_failed_sandbox();`. Each became `app_with_a_failed_sandbox_in(state_dir.path())`,
the same fixture with its boot plan's `state_dir` in a `tempdir` (`main.rs:3048-3056`). Each test gained
one assertion (`main.rs:3084-3089`, `:3261-3267`). No test was skipped, filtered or renamed, and no
threshold changed.

**Production and manifest changes:** `BringUp::task()` drives `RecordingRunner` under `cfg(test)` and
`SystemRunner` otherwise (`shell/sandbox.rs:217-223`). `iced_runtime = "=0.14.0"` is a `micold-client`
dev-dependency (`Cargo.toml:79-82`, `crates/micold-client/Cargo.toml:55-56`). `Cargo.lock` gains only
`iced_runtime` in `micold-client`'s dependency list; no package was added. The user approved the
dependency in this session before T201 started.

**`tasks.md` against the list:** T201 is `[X]`, and A1 and U24 are `DONE`. T201's stated proof was met and
re-run here: X3 and X5 fail U24, X3 and X6 fail A1, and X1, X2, N12, N13, T191a and T191b still fail them.
The tick is not a false completion. Its intent, "the work handed back has to report the bring-up's
stages", is held for the first stage only (finding 1).

**Log against history:** T201's entry and the gates after Phase 23 match `b983395e` and the runs above.
The entry is explicit that the red came from mutants, not from a failing first run.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Survivors inside `DONE` A1 and U24: a bring-up that delivers its first message and drops the rest passes.** `first_message` runs the returned work only until its first `Action::Output`, and `is_probing` checks that one message. X7 (`iced::Task::stream(StreamExt::take(stream, 1))` in `BringUp::task()`), X8 (`bring_up.task().then(..)` keeping only `SandboxMsg::Progress` in `Msg::Lost`) and X9 (the same filter on the refused-dial path) each leave all 138 client tests green. U20 checks that the *stream* ends with `Started` or `Failed`. Nothing checks that the task hands that ending to `update`. The fix needs no new dependency. Run the returned work to its end, and assert what U20 asserts of the stream: `Progress(Probing)` first and `Started` or `Failed` last. The recording runner ends a bring-up in well under a second (both tests take 0.00s). | `crates/micold-client/src/main.rs:3022-3045` (`first_message`, `is_probing`), `:3084-3089` (A1), `:3261-3267` (U24); `crates/micold-client/src/shell/sandbox.rs:215-238` (`task`), `:462` (`Msg::Lost`), `:618` (U20's last-message assertion); `crates/micold-client/src/shell/daemon_sync.rs:293` |
| 2 | MED | **Two save tests write the developer's real `settings.json` on every client test run (outside this increment).** Unchanged. The file's mtime moved again during this audit (20:05:21 during mutant runs, 20:13:30 during the suite). No task added: the tests predate feature 027's BUG-005 work. The fix is `app.caps = Capabilities::real().without_settings()`, as U30 does. | `crates/micold-client/src/main.rs:2583`, `:2630`; `main.rs:1694` (`base_app`) |
| 3 | LOW | **Test awareness in production code, and the production runner is untested by construction.** Three `cfg(test)` hooks sit in `BringUp::task()`: the recorder, the runner swap and the `live_tasks` token (`shell/sandbox.rs:198-252`). A fourth is `log_line`'s `cfg!(test)` return (`main.rs:959`). The runner swap is justified: without it a test that runs a bring-up would start a container on the developer's host. It also means the `SystemRunner` arm is compiled out of every unit test, so replacing it would pass them all. That was equally true before T201, when no test ran the task at all. Only T181 (core, real runtime) and T178 (manual) cover that arm. `live_tasks()` is now subsumed by finding 1's run of the work, and T201's log says so. | as cited |
| 4 | LOW | **No automated test crosses the application with a real runtime; documented, not closed.** Unchanged (T192). Findings 1 and 3 are both cases such a test would catch. | `crates/micold-core/tests/sandbox_real_lifecycle.rs:652-668`; `evidence/us6-failures.md` |
| 5 | LOW | **T194's margin is an inference from frames.** Unchanged; the entry states its limits. | `evidence/performance.md`, T194 section |

The fifth report's finding 1 (X3, X5, X6) and finding 4 (X4) are cleared.

Checked and clean (smell pass over the T201 test changes): tautological assertion, doubled subject (the
work runs the production `task()`, only its runner is recorded), re-implemented expectation, vacuous
assertion (X3–X6 prove it), assertion roulette (both carry a rule message and print the message they got),
mystery guest (the boot plan's state is now a `tempdir`; before T201 it named `/tmp/micold-test-state`,
which nothing wrote), conditional logic (`first_message`'s `match` only filters `Action`s, and a task with
no stream returns `None`, which fails the assertion), bypassed test utility (`tempfile::tempdir`, as U20),
and foreign style. No secrets, and no repository content addressed to the auditor.

Properties:
- **Isolation:** checked by running A1 and U24 alone, on the build the suite made, with `HOME`, `TMPDIR`
  and `XDG_*` pointed at an empty directory. Nothing was written outside the test's own `tempdir`. Two
  older tests are not isolated (finding 2).
- **Determinism:** good. The bring-up here has no delay (the first unattended attempt starts at once), so
  no timer is involved.
- **Speed:** good. Both tests report 0.00s, each building its own tokio runtime. The client target runs 138
  tests in 0.21s.
- **Specificity:** good for a dropped, swapped, silenced or leaked bring-up. Missing for one that drops its
  outcome (finding 1).
- **Refactor-insensitivity:** acceptable. A bring-up that later reports a different first stage would fail
  A1 and U24 alongside U20, which is the intent.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`), so the mutants were deliberate.
`mutants.py` applied each one alone, rebuilt, restored it from a snapshot and verified its sha256 (no
`sha_ok=False` in the log). Every client run's output contains `Compiling micold-client`. `git status` was
clean afterwards. After the last mutant, the client target (138 passed) and `sandbox_state` (20 passed) were
re-run on the restored source.

- **Commands:** client mutants ran `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
  Core mutants ran `… -p micold-core --test sandbox_state`. M15 ran both.
- **Full suite:** `scripts/build-lock.sh cargo test --workspace` → 2964 passed, 0 failed, 2 ignored, after
  X7–X9. The older 18 mutants ran after it (above).

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
| M16 production `observe` → `&mut \|_\| {}` | U20 | No | caught by U20 (`sandbox.rs:607`), and now also A1 (`main.rs:3085`) and U24 (`:3262`) |
| T191a refused-dial bring-up → `Task::done(EscapePressed)` | A1 | No | caught |
| T191b `Msg::Lost` bring-up → `Task::done(EscapePressed)` | U24 | No | caught |
| T199a `survive_logout_changed()` removed from the save | U30 | No | caught |
| X1 `Msg::Lost` bring-up built, then `Task::done(EscapePressed)` | U24 | No | caught |
| X2 refused-dial bring-up built, then `Task::done(EscapePressed)` | A1, A2 | No | caught |
| X3 `BringUp::task()` returns `Task::stream(stream).discard()` | U20, A1, U24 | No | **previously survived**; caught at `main.rs:3085` and `:3262` |
| X4 `Msg::Lost` leaks the task with `mem::forget`, returns `Task::done(EscapePressed)` | U24 | No | **previously survived**; caught at `main.rs:3262` |
| X5 `Msg::Lost` returns `bring_up.task().discard()` | U24 | No | **previously survived**; caught at `main.rs:3262` |
| X6 refused-dial returns `b.task().discard()` | A1, A2 | No | **previously survived**; caught at `main.rs:3085` |
| X7 `BringUp::task()` keeps only the stream's first message | A1, U24, U20 | **Yes** | real: no bring-up at startup or in recovery reports how it ended (finding 1) |
| X8 `Msg::Lost` keeps only `Progress` messages | U24 | **Yes** | real: a failed restart of a stopped container is never recorded or counted (finding 1) |
| X9 refused-dial keeps only `Progress` messages | A1, A2 | **Yes** | real: the same, on the refused-dial path (finding 1) |

X7–X9 are new, aimed at the first-message check T201 added. Not run: replacing `SystemRunner` in the
`cfg(not(test))` arm, which is compiled out of every unit test and so survives by construction (finding 3).

Score: 28 of 31 caught (90%). The three survivors are inside `DONE` behaviors and are not equivalent. The
sample favours FR-002a, FR-036a and FR-036b and every assertion Phases 20–23 changed; it is not exhaustive.
Coverage is not configured, so it was not run.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-002a (every placement starts its service when absent) | A1, U1, U5, U12, U15, U24 | Partial: T181 drives core against a real runtime, not the app; T172/T178 manual |
| FR-036a (bring back up; bounded, spaced, each reports why) | A2, A4, U2, U3, U17, U19, U23 | No: T178 manual covers one recovery. **An outcome dropped between the task and `update` is not caught (finding 1)** |
| FR-036b (report the stage; no connection failure while starting) | A3, U6, U14, U16, U21, U22, U24, U25, U28, U29, U30 | No: T178 re-run (manual, Xvfb) only. The first stage now reaches the returned work's output; the ending does not have to (finding 1) |
| SC-004c (progress measured at the application; no connection failure) | A3, U7, U8, U20, U21, U25 | No: T178 re-run (manual) only |
| US6 scenario 9 | A1, A4, U23, U24 | Partial: T181 (core loop), T178 manual; gap recorded by T192 |
| S-6 (`data-model.md` §7) | A2, U1, U2, U3, U9, U14–U17, U19 | No |
| S-7 | U4, U12, U15 | No |
| C-8 caller half (`contracts/container-runtime.md`) | U7, U20 | No: T181 calls `bring_up` directly |

**Untested criteria:** none at unit or `update_inner` level. No criterion has an automated test through
the application entry point against a real runtime (finding 4).

**Tests outside these eight criteria:** U10 and U11 (FR-034), U18 (FR-036), U26, U27 and U31 (FR-027).
These trace to existing requirements.

## What was not audited

- **T181 was not re-run.** It needs the shared `micold-daemon:dev` image, and rebuilding that image needs
  approval. It is the only automated cover for the production runner arm (finding 3).
- **T178 (visual pass) was not re-run.** No Phase 20–23 change was checked on a display.
- **The rest of feature 027 (T001–T166)** was out of scope; finding 2 was noticed, not audited.
- **Mutation was a hand sample:** 31 deliberate mutants on the six changed source files, with no tool.
  Coverage was not measured (no tool configured).
- **The suite's wall time** includes an unmeasured wait behind another worktree's build lock, so 512s says
  nothing about the suite's speed.
- **Finding 1's remedy was not tried.** That the recording runner's bring-up ends promptly is taken from
  U20 and the 0.00s timings, not from a run of the whole task.
- **T194's numbers** were read from the report, not re-measured.
- **macOS and Windows** were not run; everything here ran on Linux.
- **Independence:** the tests' author ran the whole audit, smell pass included, with no fresh-context
  reviewer.

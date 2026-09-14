---
feature: 027-sandboxed-daemon-runtime
scope: BUG-004 increment (Phases 18–21, T167–T199), committed
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: ae018388
behaviors: 34 # A1–A4, U1–U12, U14–U31 (no U13 on the list)
proven: 0
likely: 30
test_after: 0
no_test: 0
not_applicable: 4
high_smells: 1
criteria_total: 8
criteria_covered: 8 # every criterion has a test through update_inner or core; none end to end through the app with a real runtime
mutation_score: 92 # 22 of 24 deliberate mutants caught; scope: the 6 source files BUG-004 changed; no mutation tool in the profile
mutants_survived: 2 # not equivalent
suite: 2964 passed, 0 failed, 2 ignored, 106s wall incl. build # cargo test --workspace after all mutants restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-004 increment (fourth audit)

**Verdict: FAIL.** A bring-up that is built and then swapped for another single task still passes A1 and
U24. T197 added `work.units() == 1` next to `BringUp::scheduled().len() == 1`, which catches a bring-up
built and dropped for nothing (N12, N13). It does not catch one built and dropped for *something*.
Mutant X1 makes `Msg::Lost` call `bring_up.task()`, throw the result away and return
`Task::done(Message::EscapePressed)`. X2 does the same on the refused-dial path. All 138 client tests
stay green under both. In the application, the sandbox moves to `Probing` and nothing starts it: BUG-004.
The recorder counts the bring-up when it is built, and `units()` counts any task, so together they still
do not show that the returned unit *is* the bring-up. The third audit's example remedy ("`scheduled()`
plus `units() == 1`") was this same insufficient pair, and T197 followed it.

Everything else the previous report found is cleared. All 22 mutants from earlier audits and from Phase
21 are caught, N13 included. The spec and plan now state the start grace for `Stale` (T198). U30 reaches
`Stale` through the settings save, and T199a (the save no longer marks the sandbox) fails its setup.

**The audit is not independent.** The session that wrote T197–T199 also ran this audit, and no
fresh-context subagent was used: the user asked for none. Every cited line was re-opened for this report,
and every mutant listed was run at `ae018388`.

## Test-first evidence

Commits: `eb3edf21` (Phases 18–19 code and tests), `cd7fc151` (cycles 1–12), `f8f12e4a` (cycles 13–18),
`6b802791` (Phase 20), `6cddbb3e` (third report), `ae018388` (all of Phase 21, with its cycle-log
entries). Every code commit squashes its test changes with the rest. History cannot show order inside a
commit, so every behavior with a recorded red is `LIKELY`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | LIKELY | cycle 1 red; `eb3edf21`. T191's red in `6b802791`. T197's added assertion is proven by N12 (`main.rs:3035`); test-only change in `ae018388` |
| A2, A3, A4 | LIKELY | cycles 8, 9 and 11 reds; `eb3edf21` |
| U1, U2, U3, U5–U9 | LIKELY | Phase 18 reds, copied into the log after the fact; `eb3edf21` |
| U4 | LIKELY | T190 records M3's red after the commit. Weakest `LIKELY` on the list; M3 re-run and caught |
| U10, U11, U12, U18 | NOT_APPLICABLE | characterization (`BASELINE`); N10 catches U12, M14 and N12 catch U10 |
| U14–U17, U19–U23 | LIKELY | cycles 2–11 reds; `eb3edf21` |
| U24–U29 | LIKELY | cycles 13–18 reds; `f8f12e4a`. U24's red for T197 is the third audit's surviving N13; N13 now fails it at `main.rs:3198` |
| U30 | LIKELY | cycle 19 real red; `6b802791`. T199 changed its setup only; N11 and T199a both fail it (`main.rs:3473`, `:3466`) |
| U31 | LIKELY | cycle 20 pin with N3's deliberate-mutant red; `6b802791` |

**Existing tests weakened: none.** In `6b802791..ae018388` the only test changes are:

- A1 (`main.rs:3027`) and U24 (`main.rs:3191`): `let _ =` became `let work =`, and `assert_eq!(work.units(),
  1, ..)` was added. No assertion was removed. A strengthening (N13 is now caught).
- U30 (`main.rs:3449-3465`): `app.sandbox.survive_logout_changed()` became `SettingsMsg::Opened`,
  `SurviveLogoutToggled(true)` and `Saved` through `update_inner`, on `Capabilities::real().without_settings()`.
  The setup assertion and the three final assertions are unchanged. A strengthening (T199a is caught).

No test was skipped, filtered or renamed, and no threshold changed.

**`tasks.md` against the list:** T197–T199 are `[X]`. A1, U24 and U30 are `DONE`. T197's own proof
criteria (N13 fails U24, N12 fails A1, T191a and T191b still fail them) were met and re-run here, so the
tick is not a false completion. Its intent, that the bring-up is in the returned work, is not held
(finding 1).

**Log against history:** T197–T199's entries match `ae018388` and the mutant runs above. T199's entry
records its own deviation: the first version wrote the developer's settings file, and was fixed before the
commit.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Survivors inside `DONE` A1 and U24: a bring-up built, dropped, and replaced by another one-unit task passes.** `BringUp::task()` pushes to the `#[cfg(test)]` recorder when the task is built. `work.units() == 1` holds for any single task. X1 (`let _ = bring_up.task(); return iced::Task::done(Message::EscapePressed);` in `Msg::Lost`) and X2 (`map_or_else(Task::none, \|b\| { let _ = b.task(); Task::done(Message::EscapePressed) })` in `refused_dial`) leave all 138 client tests green; A2's `units() > 0` loop does not catch X2 either. The assertions have to tie the returned work to the bring-up. `iced_runtime::task::into_stream` would let a test read the task, but `iced_runtime` is not a direct dependency. One seam that needs no dependency: under `cfg(test)`, give the bring-up's stream a guard that is counted while it is alive, and assert after the update that exactly one bring-up task is still alive while the test holds `work`. A task dropped inside the update drops its guard. | `crates/micold-client/src/main.rs:3027-3040` (A1), `:3191-3203` (U24); `crates/micold-client/src/shell/sandbox.rs:198-224` (recorder), `:430` (`Msg::Lost`); `crates/micold-client/src/shell/daemon_sync.rs:293` |
| 2 | MED | **Two save tests write the developer's real `settings.json` on every client test run (outside this increment).** `settings_saved_sends_settings_set_to_a_connected_daemon` and `settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected` save through `base_app()`, whose `Capabilities::real()` resolves `~/.local/share/micold-ai-ide/settings.json`. The file's mtime moved during this audit's mutant runs (13:09:07) and full suite (13:09:55), and it holds their fixture values (`default_ai_cli: Copilot`, `env_include_script_path: /tmp/does-not-exist.sh`). Not isolated, and destructive to the developer's data. The fix is the pattern already used at `main.rs:2664`: `app.caps = Capabilities::real().without_settings()`. No task added: the tests predate feature 027's BUG-004 work. | `crates/micold-client/src/main.rs:2583`, `:2630`; `main.rs:1694` (`base_app`) |
| 3 | LOW | **Test awareness in production code.** Unchanged from the third audit: the bring-up recorder (`shell/sandbox.rs:198-224`) and `log_line`'s `cfg!(test)` return (`main.rs:959`). Finding 1's remedy would add a second hook of the same kind. | as cited |
| 4 | LOW | **No automated test crosses the application with a real runtime; documented, not closed.** Unchanged (T192). Finding 1 is again an example of what it lets through. | `crates/micold-core/tests/sandbox_real_lifecycle.rs:652-668`; `evidence/us6-failures.md` |
| 5 | LOW | **T194's margin is an inference from frames.** Unchanged; the entry states its limits. | `evidence/performance.md`, T194 section |

Previous findings 2 (T193 wording) and 3 (U30 bypassed the save) are cleared. `spec.md:672` and
`plan.md:423` state the rule, and the plan's claim that `on_connected` calls `answered()` before any mount
set is adopted holds (`daemon_sync.rs:861`, `adopt_mount_set` at `:458` and `:901`).

Checked and clean (smell pass over the Phase 21 test changes): tautological assertion, doubled subject,
re-implemented expectation, vacuous assertion (U30's setup assertion guards the save route, proven by
T199a), assertion roulette (both new assertions carry rule messages), mystery guest (U30 no longer reads the
real settings store), conditional logic, redundant test, bypassed test utility (`without_settings()` is
the recorded helper for save tests), and foreign style. No secrets, and no repository content addressed to
the auditor.

Properties:
- **Isolation:** the Phase 21 tests are isolated. Two older tests are not (finding 2).
- **Determinism:** good. The recorder passes parallel and serial runs.
- **Speed:** good. 138 client tests in well under a second once built.
- **Specificity:** good for a dropped bring-up; missing for a substituted one (finding 1).
- **Refactor-insensitivity:** acceptable. `units() == 1` makes A1 and U24 fail if the update ever batches
  the bring-up with other work; that would be a deliberate change to revisit, not a bug.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`), so the mutants were deliberate.
`mutants.py` applied each one alone, rebuilt (`Compiling micold-client` in every client run), restored it
from a snapshot and verified its sha256 (`restored sha_ok=True` for all 24). `git status` was clean
afterwards.

- **Commands:** client mutants ran `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
  Core mutants ran `… -p micold-core --test sandbox_state`. M15 ran both.
- **Full suite afterwards:** `scripts/build-lock.sh cargo test --workspace` → 2964 passed, 0 failed,
  2 ignored.

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
| M3 `Failed \| Stale` brought up on absence | U4 | No | caught |
| M10 wait before an unattended bring-up removed | U19 | No | caught |
| M11 `BringUp { after: ZERO }` | U17 | No | caught |
| M14 `self.unattended = again.budget` dropped | A2, U9, U17 | No | caught |
| M15 `Probing`, `Starting` not coming up | U14, U16, A3, U24 | No | caught in core and client |
| M16 production `observe` → `&mut \|_\| {}` | U20 | No | caught |
| T191a refused-dial bring-up → `Task::done(EscapePressed)` | A1 | No | caught |
| T191b `Msg::Lost` bring-up → `Task::done(EscapePressed)` | U24 | No | caught |
| N12 refused-dial bring-up built, then `Task::none()` | A1, A2 | No | **now caught by A1** (`main.rs:3035`) |
| N13 `Msg::Lost` bring-up built, then `Task::none()` | U24 | No | **previously survived**; caught at `main.rs:3198` |
| T199a `survive_logout_changed()` removed from the save | U30 | No | caught at `main.rs:3466` (new, T199) |
| X1 `Msg::Lost` bring-up built, then `Task::done(EscapePressed)` | U24 | **Yes** | real: a stopped container is never brought up (finding 1) |
| X2 refused-dial bring-up built, then `Task::done(EscapePressed)` | A1, A2 | **Yes** | real: a failed sandbox is never brought up (finding 1) |

X1 and X2 are new: N12/N13 combined with T191a/T191b, aimed at the pair of checks T197 added.

Score: 22 of 24 caught (92%). Both survivors are inside `DONE` behaviors and are not equivalent. The
sample favours FR-036a and FR-036b and every assertion Phases 20–21 changed; it is not exhaustive.
Coverage is not configured, so it was not run.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-002a (every placement starts its service when absent) | A1, U1, U5, U12, U15, U24 | Partial: T181 drives core against a real runtime, not the app; T172/T178 manual. **A substituted bring-up is not caught on either path (finding 1)** |
| FR-036a (bring back up; bounded, spaced, each reports why) | A2, A4, U2, U3, U17, U19, U23 | No: T178 manual covers one recovery |
| FR-036b (report the stage; no connection failure while starting) | A3, U6, U14, U16, U21, U22, U24, U25, U28, U29, U30 | No: T178 re-run (manual, Xvfb) only. `Stale` in the grace is now in the spec note |
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
  approval.
- **T178 (visual pass) was not re-run.** No Phase 20–21 change was checked on a display, including a
  settings save during a real start.
- **The rest of feature 027 (T001–T166)** was out of scope; finding 2 was noticed, not audited.
- **Mutation was a hand sample:** 24 deliberate mutants on the six changed source files, with no tool.
  Coverage was not measured (no tool configured).
- **`/speckit.bugfix.verify`,** T198's stated proof, was not run by this audit; the notes were read against
  the code instead.
- **T194's numbers** were read from the report, not re-measured.
- **macOS and Windows** were not run; everything here ran on Linux.
- **Independence:** the tests' author ran the whole audit, smell pass included, with no fresh-context
  reviewer.

---
feature: 027-sandboxed-daemon-runtime
scope: BUG-004 increment (Phases 18–20, T167–T196), committed
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: 6b802791
behaviors: 34 # A1–A4, U1–U12, U14–U31 (no U13 on the list)
proven: 0
likely: 30
test_after: 0
no_test: 0
not_applicable: 4
high_smells: 1
criteria_total: 8
criteria_covered: 8 # every criterion has a test through update_inner or core; none end to end through the app with a real runtime
mutation_score: 95 # 20 of 21 deliberate mutants caught; scope: the 5 source files BUG-004 changed; no mutation tool in the profile
mutants_survived: 1 # not equivalent
suite: 2964 passed, 0 failed, 2 ignored, 126s wall incl. build # cargo test --workspace after all mutants restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-004 increment (third audit)

**Verdict: FAIL.** T191 weakened U24. Mutant N13 makes `Msg::Lost` build the bring-up's task, throw it
away and return `Task::none()`, and the whole suite stays green. U24's old `work.units() > 0` would have
failed on it. The new `BringUp::scheduled().len() == 1` records the bring-up when `BringUp::task()` is
called, not when the task is handed back to the runtime. So a stopped container is moved to `Probing`
and nothing starts it. That is BUG-004 again on the `Lost` path, inside a `DONE` behavior.

Everything else the previous report found is cleared. N3 now fails U26 and U31 (T189). M3's red for U4
is recorded (T190). T191a and T191b, the task's own proof, are caught. U30 and U31 pin both routes into
`Stale` (T193). The documentation (T192, T194), the refactor (T195) and the commit note (T196) are done.

**The audit is not independent.** The session that wrote T189–T196 also ran this audit, and no
fresh-context subagent was used: the user asked for none. Every cited line was re-opened for this
report, and every mutant listed was run.

## Test-first evidence

Commits: `eb3edf21` (Phases 18–19 code and tests), `cd7fc151` (cycles 1–12), `f8f12e4a` (cycles 13–18),
`8370818b` (previous report), `6b802791` (all of Phase 20, with its cycle-log entries). Every code commit
squashes several cycles with their tests. History cannot show the order inside a commit, so every behavior
with a recorded red is `LIKELY`.

| Behavior | Class | Evidence |
| --- | --- | --- |
| A1 | LIKELY | cycle 1 red; `eb3edf21`. T191's red for the new assertion (`main.rs:3024`, `left: 0 right: 1`) is in `6b802791` |
| A2, A3, A4 | LIKELY | cycles 8, 9 and 11 reds; `eb3edf21` |
| U1, U2, U3, U5–U9 | LIKELY | Phase 18 reds, copied into the log after the fact; `eb3edf21` |
| U4 | LIKELY | **reclassified from `TEST_AFTER`.** T190 records M3's red verbatim (`sandbox_state.rs:552`). That is the playbook's red for a test that passes on its first run, and the rubric's `TEST_AFTER` test ("no red recorded") no longer holds. It is the weakest `LIKELY` on the list: the red was taken after the commit, so it shows the test's strength, not that the test came first. The auditor re-ran M3 and it is caught |
| U10, U11, U12, U18 | NOT_APPLICABLE | characterization (`BASELINE`); N10 catches U12, M14 and N12 catch U10 |
| U14–U17, U19–U23 | LIKELY | cycles 2–11 reds; `eb3edf21` |
| U24–U29 | LIKELY | cycles 13–18 reds; `f8f12e4a`. U24's assertion was replaced by T191 (see weakened tests) |
| U30 | LIKELY | cycle 19 real red (`main.rs:3418`, `left: Disconnected right: Disconnected`); test and code together in `6b802791` |
| U31 | LIKELY | cycle 20: a pin (it passed first time), with a deliberate-mutant red from N3 recorded before the commit; `6b802791` |

**Existing tests weakened: one (finding 1).** In `139bd640..6b802791`:

- `main.rs:3187-3190` (U24): **before** `let work = update_inner(.., Lost); assert!(work.units() > 0, ..)`.
  **After** `let _ = update_inner(.., Lost); assert_eq!(BringUp::scheduled().len(), 1, ..)`. The new
  check is stronger against a different task (T191b) and a second bring-up. It is weaker against a
  dropped task (N13 survives).
- `main.rs:3027-3034` (A1): the same replacement. N12, the same drop on the refused-dial path, still
  fails the suite, but through A2's `while connection_failed(..).units() > 0` loop (`main.rs:3538`), not
  through A1.
- U26's fixture catalog (`/repo/demo` → `CatalogSnapshot::default()`), with a setup assertion added. This
  is a strengthening, and the log justifies it: N3 is now caught.
- U18's `assert_eq!` gained a rule message. This is a strengthening.

No test was skipped, filtered or renamed, and no threshold changed.

**`tasks.md` against the list:** T189–T196 are all `[X]`. Every behavior they name is `DONE`, and no
task is ticked without evidence. T193's wording and U30 disagree (finding 2).

**Log against history:** the reds in the log match the file today, allowing for the line shift from
U31, which was added above U30. T191's entry says the old `units() > 0` "passed both" T191a and T191b.
That is true, but the entry does not say what the old check caught that the new one does not.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Weakened test and a survivor inside `DONE` U24: a bring-up that is built but never run passes.** `BringUp::task()` pushes to the `#[cfg(test)]` recorder before it builds the stream. The returned `Task` is no longer asserted: `update_inner`'s result is discarded with `let _`. N13 (`let _ = bring_up.task(); return iced::Task::none();` in `Msg::Lost`) leaves all 138 client tests green. In the application, a container found stopped moves to `Probing` and nothing runs it, which is BUG-004. A1 has the same gap (N12), and only A2's loop catches it. Assert both halves: exactly one bring-up was scheduled, **and** it is in the work returned to the runtime. For example, keep `scheduled().len() == 1` and assert the returned task's `units() == 1`. Or record the bring-up where its task joins the returned `Task`, rather than where it is built. | `crates/micold-client/src/main.rs:3187-3193` (U24), `:3027-3034` (A1); `crates/micold-client/src/shell/sandbox.rs:213-224` (recorder), `:430` (`Msg::Lost`); `crates/micold-client/src/shell/daemon_sync.rs:293` |
| 2 | MED | **T193 is ticked against its own wording, and the decision lives only in the cycle log.** T193 asks to pin that a sandbox going `Stale` during the start grace "is not coming up". U30 pins the opposite for the settings-save route: `is_coming_up` now counts `Running \| Stale`. U31 pins T193's wording for the catalog route, which can only happen after the answer. The reasoning is sound (FR-036b: the container is up and its service is still starting). But `spec.md` FR-036b, `plan.md` and `data-model.md` §7 say nothing about `Stale`, so the rule U30 enforces traces to a cycle-log note, not to a requirement. | `specs/027-sandboxed-daemon-runtime/tasks.md` T193; `tdd/cycle-log.md` cycle 19 notes; `crates/micold-client/src/features/sandbox.rs:286-292`; `spec.md:653` |
| 3 | LOW | **U30 reaches `Stale` by calling the model directly, not the route it names.** Its doc names "the keep-running opt-in saved while it started". The test calls `app.sandbox.survive_logout_changed()` instead of sending the settings save through `update_inner`, which reaches `persist.rs:348`. It still pins `is_coming_up` for `Stale`. It does not show that the save, arriving in the grace, leaves the banner alone. | `crates/micold-client/src/main.rs:3446`; `crates/micold-client/src/shell/persist.rs:348` |
| 4 | LOW | **Test awareness in production code.** The bring-up recorder is a `#[cfg(test)]` thread-local inside `BringUp::task` (`shell/sandbox.rs:198-224`). `log_line` returns early under `cfg!(test)` (`main.rs:959`). Both are justified in the log: `Task` is opaque in iced 0.14, and T195 stopped writes to the real log. The recorder relies on libtest's one thread per test. A serial run (`--test-threads=1`) still passes 138 of 138, so it is sound today. The price is that no client test can observe what `log_line` writes. | as cited |
| 5 | LOW | **No automated test crosses the application with a real runtime; documented, not closed.** T192 took the documentation route, which the task allows (`evidence/us6-failures.md`, `test-list.md` out-of-scope). This is the previous report's finding 4, lowered because the gap is now stated where readers look. Finding 1 is an example of what it lets through. | `crates/micold-core/tests/sandbox_real_lifecycle.rs:652-668`; `evidence/us6-failures.md` |
| 6 | LOW | **T194's margin is an inference from frames.** "`Started` to listening: at most 0ms" is bounded by frames 0.13s apart, because the client log has no timestamps. It was measured on one machine with a warm image. The conclusion (milliseconds against a 1s backoff) holds at that resolution, and U27 turns a wrong constant into a visible banner. The entry states its limits. | `evidence/performance.md`, T194 section |

Checked and clean (smell pass over the Phase 20 test changes): tautological assertion, doubled subject,
re-implemented expectation, vacuous assertion (the setup assertions in U26, U30 and U31 guard their
fixtures), assertion roulette (every new assertion carries a rule message), mystery guest, conditional
logic (`bring_up_stages()` is fixed data), redundant test (U30 and U31 cover distinct routes), bypassed
test utility (`the_service_answers_with`, `bring_up_stages`, `connection_failed` and
`nothing_was_reported` are shared), and foreign style (both follow the `main::tests` idiom). No secrets,
and no repository content addressed to the auditor.

Properties:
- **Determinism:** good. The thread-local recorder passes both parallel and serial runs.
- **Speed:** good. The client binary's 138 tests finish in 0.24s once built.
- **Specificity:** good, except finding 1: T191b is now named exactly, but a dropped task is not.
- **Refactor-insensitivity:** acceptable. `scheduled()` couples A1 and U24 to `BringUp::task` being the
  one constructor of the runtime task.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`), so the mutants were deliberate.
`mutants.py` applied each one alone, restored it from a snapshot and verified its sha256
(`restored sha_ok=True` for all 21). `git status` was clean afterwards.

- **Commands:** client mutants ran `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
  Core mutants ran `… -p micold-core --test sandbox_state`. M15 ran both.
- **Full suite afterwards:** `scripts/build-lock.sh cargo test --workspace` → 2964 passed, 0 failed,
  2 ignored.
- **Build freshness:** after the first two runs, rebuilds took 1–7s. Each run failed a distinct test
  matching its mutant, so no run was a stale binary.
- **N5's pattern was updated** to the `cargo fmt` layout of `is_coming_up`. The mutant is unchanged.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| N1 `REFUSALS_WHILE_STARTING` 1 → 0 | U28, U30 | No | caught |
| N2 same, 1 → 2 | U27 | No | caught |
| N3 `daemon_sync.rs` `answered()` removed | U26, U31 | No | **previously survived**; caught by both |
| N4 `service_refused()` removed | U27 | No | caught |
| N5 wait counted in any state | U29 | No | caught |
| N6 `Msg::Lost` never brings up | U24 | No | caught |
| N7 `connection_status` reads the bare state | U25, U28, U30 | No | caught |
| N8 `started()` sets no wait | U25, U28, U30 | No | caught |
| N9 `refused_dial` silence reads the bare state | U28, U30 | No | caught |
| N10 `LocalSandbox` guard removed | U12 | No | caught |
| N11 `is_coming_up` wait for `Running` only | U30 | No | caught (new, T193) |
| M3 `Failed \| Stale` brought up on absence | U4 | No | caught at `sandbox_state.rs:552` |
| M10 wait before an unattended bring-up removed | U19 | No | caught at `shell/sandbox.rs:616` |
| M11 `BringUp { after: ZERO }` | U17 | No | caught |
| M14 `self.unattended = again.budget` dropped | A2, U9, U17 | No | caught |
| M15 `Probing`, `Starting` not coming up | U14, U16, A3, U24 | No | caught in core (`sandbox_state.rs:570`) and client |
| M16 production `observe` → `&mut \|_\| {}` | U20 | No | caught at `shell/sandbox.rs:575` |
| T191a refused-dial bring-up → `Task::done(EscapePressed)` | A1 | No | caught (T191's proof) |
| T191b `Msg::Lost` bring-up → `Task::done(EscapePressed)` | U24 | No | caught (T191's proof) |
| N12 refused-dial bring-up built, then `Task::none()` | A1, A2 | No | caught **only by A2** (`main.rs:3550`, `left: 0 right: 3`), not by A1 (finding 1) |
| N13 `Msg::Lost` bring-up built, then `Task::none()` | U24 | **Yes** | real: a stopped container is never brought up (finding 1) |

M13 from the previous report is superseded by T191a and N12 (the same call site), and was not re-run.

Score: 20 of 21 caught (95%). The one survivor is inside a `DONE` behavior and is not equivalent. The
sample favours the behaviors FR-036a and FR-036b depend on, plus every assertion Phase 20 changed; it is
not exhaustive. Coverage is not configured, so it was not run.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| FR-002a (every placement starts its service when absent) | A1, U1, U5, U12, U15, U24 | Partial: T181 drives core functions against a real runtime, not the app; T172/T178 manual. **The `Lost` path is not held (finding 1)** |
| FR-036a (bring back up; bounded, spaced, each reports why) | A2, A4, U2, U3, U17, U19, U23 | No: T178 manual covers one recovery |
| FR-036b (report the stage; no connection failure while starting) | A3, U6, U14, U16, U21, U22, U24, U25, U28, U29, U30 | No: T178 re-run (manual, Xvfb) only. `Stale` in the grace is not in the requirement (finding 2) |
| SC-004c (progress measured at the application; no connection failure) | A3, U7, U8, U20, U21, U25 | No: T178 re-run (manual) only |
| US6 scenario 9 | A1, A4, U23, U24 | Partial: T181 (core loop), T178 manual; gap recorded by T192 |
| S-6 (`data-model.md` §7) | A2, U1, U2, U3, U9, U14–U17, U19 | No |
| S-7 | U4, U12, U15 | No |
| C-8 caller half (`contracts/container-runtime.md`) | U7, U20 | No: T181 calls `bring_up` directly |

**Untested criteria:** none at unit or `update_inner` level. No criterion has an automated test through
the application entry point against a real runtime (finding 5).

**Tests outside these eight criteria:** U10 and U11 (FR-034), U18 (FR-036), U26, U27 and U31 (FR-027).
These trace to existing requirements, not to nothing.

## What was not audited

- **T181 was not re-run.** It needs the shared `micold-daemon:dev` image, and rebuilding that image needs
  approval.
- **T178 (visual pass) was not re-run.** No Phase 20 change was checked on a display. That includes U30's
  path, a settings save during a real start.
- **The rest of feature 027 (T001–T166)** was out of scope; it has no test-list evidence.
- **Mutation was a hand sample:** 21 deliberate mutants on the five changed source files, with no tool
  and no exhaustive run. Coverage was not measured (no tool configured).
- **T194's numbers** were read from the report, not re-measured.
- **macOS and Windows** were not run; everything here ran on Linux.
- **Independence:** the tests' author ran the whole audit, smell pass included, with no fresh-context
  reviewer.

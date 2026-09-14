---
feature: 027-sandboxed-daemon-runtime
scope: BUG-004 increment (Phase 18, T167–T171), uncommitted working tree
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset resolved
verified_at: b53e3449 # plus the uncommitted BUG-004 diff (git diff sha256 b4de1752dedb)
behaviors: 14
proven: 0
likely: 8
test_after: 1
no_test: 2
not_applicable: 3
high_smells: 7
criteria_total: 8
criteria_covered: 1 # every clause pinned; 7 partially, 0 end to end
mutation_score: 62 # 10 of 16 deliberate mutants caught; scope: the 5 source files BUG-004 changed; no mutation tool in the profile
mutants_survived: 6 # none equivalent
suite: 2947 passed, 0 failed, 2 ignored, 197s # cargo test --workspace, after all mutants restored
---

# TDD Verification: 027 sandboxed daemon runtime — BUG-004 increment

**Verdict: FAIL.** The spacing S-6 and FR-036a require between unattended bring-ups can be deleted
from both client points that apply it (M10, M11). The whole suite stays green, inside T171, which is
marked done and claims "Bounded and spaced".

That is not the only survivor. A connect failure that starts no bring-up at all passes (M13). So does a
budget that is never spent at the client (M14), and so does the original SC-004c defect reintroduced
in `boot_after` (M16). FR-036b's "not a connection failure" clause is claimed by tests that look only at
toasts, while the banner the user sees contradicts it; T172 recorded it failing in the application.
FR-036a's "each attempt MUST report why it failed" has no test.

**The audit is not independent.** The session that wrote T167–T171 also ran this audit. The smell pass
was delegated to a fresh-context subagent. Every line it cited was re-opened here, and the four mutants
it had only reasoned about (M13–M16) were run before being reported.

## Test-first evidence

The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, and BUG-004 is uncommitted. That leaves
nothing in the repository to corroborate ordering. The rubric's `BLOCKED` row says a feature with no
test list cannot be audited; the command says to audit against `spec.md` with a weaker ordering
verdict. The command was followed.

Red output does exist, but only in the session scratchpad (`t167-red.txt`, `t168-red.txt`), which will
not outlive the session. It is quoted below for that reason. The behaviors are the tests T167/T168
added, plus the two clauses T168/T171 claim that no test asserts.

| Behavior | Test | Class | Evidence |
| --- | --- | --- | --- |
| B1 failed + absent → `Probing`, one attempt spent | `sandbox_state.rs::a_failed_sandbox_found_absent_is_brought_up_again_without_anyone_asking` | LIKELY | red against a `None` stub: panicked at `sandbox_state.rs:491` "a sandbox that is not running, with attempts left, has to be brought up again"; not committed |
| B2 bound, then the failure stands | `…::unattended_bring_ups_are_bounded_and_then_the_failure_stands` | LIKELY | red at `:532` "a bound of zero is the bug"; not committed |
| B3 core spacing beyond the 1 Hz retry | `…::unattended_bring_ups_are_spaced_further_apart_than_the_connection_retries` | LIKELY | red at `:558` "no attempt was made to space"; not committed |
| B4 only `Failed` is brought up (S-7) | `…::absence_only_brings_up_a_sandbox_that_has_failed` | TEST_AFTER | never red: a `None` stub already satisfies it. `tasks.md` T167 reports a mutant confirmation (self-reported); M3 here caught it. Strength is fine, ordering is unproven |
| B5 refused dial on failed sandbox brings it up | `main.rs::a_failed_sandbox_the_client_cannot_reach_is_brought_up_again` | LIKELY | red at `main.rs:3002` ("9 passed; 5 failed"); not committed |
| B6 refused dial during a bring-up is silent | `main.rs::a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure` | LIKELY | red at `main.rs:3032` |
| B7 every stage reported before the outcome | `shell/sandbox.rs::every_stage_a_bring_up_enters_is_reported_before_how_it_ended` | LIKELY | red at `sandbox.rs:489` |
| B8 `Msg::Progress` moves the state | `main.rs::a_reported_stage_is_what_the_sandbox_shows` | LIKELY | red at `main.rs:3099` |
| B9 a started sandbox earns attempts back | `main.rs::a_sandbox_that_came_up_earns_its_unattended_bring_ups_back` | LIKELY | red at `main.rs:3134` |
| B10 spent budget → failure reported | `main.rs::once_the_unattended_bring_ups_are_spent_the_failure_is_reported` | NOT_APPLICABLE | characterizes today's reporting; green at red by design; T168 note reports a mutant |
| B11 no boot plan → failure reported | `main.rs::without_a_boot_plan_nothing_is_brought_up` | NOT_APPLICABLE | as B10 |
| B12 host placement never brings a sandbox up | `main.rs::a_host_process_placement_never_brings_a_sandbox_up` | NOT_APPLICABLE | as B10; M5 caught it |
| B13 FR-036b: no connection failure *shown* during a bring-up | none | NO_TEST | B5/B6 assert only the toast queue (finding 6) |
| B14 FR-036a: each failed attempt's reason is reported | none | NO_TEST | finding 7 |

**Existing tests weakened: none.** In `crates/micold-core/tests/sandbox_state.rs` the diff removes
only the import list and one doc line on `only_an_explicit_request_restarts_the_sandbox`
(`:431-432`, "The only edge back into bring-up" narrowed to "from a running sandbox"). Its body and
assertions are unchanged. `main.rs` and `shell/sandbox.rs` lose no test lines. No test is skipped or
filtered, and no threshold was changed.

**`tasks.md` against the evidence:**
- T168 is ticked and claims the connect failure "yields the bring-up task". No test inspects the task
  (finding 2).
- T171 is ticked and claims "Bounded and spaced". The client spacing and bookkeeping are unpinned
  (findings 1, 3).
- T172 is ticked honestly as a measurement, and its own result records SC-004c's FR-036b clause as
  failing with no open task behind it (finding 6).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | **Surviving mutants inside T171: the client applies no tested spacing.** Removing the wait in `reported` (M10), or passing `Duration::ZERO` instead of the budget's delay (M11), leaves every test green. So an implementation that retries at 1 Hz passes. Core pins the *values* (B3); nothing pins that the client *waits* them. | `crates/micold-client/src/shell/sandbox.rs:221`, `crates/micold-client/src/shell/daemon_sync.rs:307` |
| 2 | HIGH | **Assertion free on the value that is the behavior.** `connection_failed` discards the `Task` `update_inner` returns, so B5 checks only that the state went to `Probing`. M13 returns `Task::none()` in place of the bring-up and passes the suite. That mutant is worse than the bug: the sandbox then sits in `Probing` with nothing running, and `is_coming_up` silences every later dial. B6's "must not be started a second time" cannot see a second task either. | `crates/micold-client/src/main.rs:2981-2988` (`let _ =`), used at `:3005`, `:3031`; M13 at `daemon_sync.rs:307` |
| 3 | HIGH | **The attempt budget is unpinned at the client.** Tests spend the budget by calling core `service_absent` and writing `app.sandbox.unattended` directly, so `Sandbox::service_absent` never has its bookkeeping checked. M14, which drops `self.unattended = again.budget`, passes. The result is unbounded bring-ups, contrary to FR-036a and S-6. | `crates/micold-client/src/features/sandbox.rs:317`; setup at `main.rs:3048-3052`, `:3118-3122` |
| 4 | HIGH | **`is_coming_up` is pinned for one state of three.** Only `Acquiring` is exercised (M4 caught it). M15, which makes `Probing` and `Starting` return false, passes. `Probing` is the state held through the 5 s and 15 s waits, so FR-036b's silence is unpinned for most of a bring-up. | `crates/micold-core/src/sandbox/lifecycle.rs:122-128`; only client case `main.rs:3020-3024` |
| 5 | HIGH | **Doubled subject: the progress test supplies its own bring-up work.** B7 feeds `reported` a closure it wrote itself, so the production closure in `boot_after` (passing `observe` to `start`) never runs. M16 restores the original defect (`&mut \|_\| {}` in `boot_after`) and the whole `micold-client` suite passes. This reverts T170 undetected, so SC-004c's "at the application" is not verified. | `crates/micold-client/src/shell/sandbox.rs:492-497` (test closure), `:200` (production `observe`) |
| 6 | HIGH | **FR-036b / SC-004c clause claimed by an assertion narrower than the requirement, and failing in the application.** B5 and B6 cite FR-036b but check only `notifications.queue.visible()`. `on_connect_failed` sets `app.disconnected = true` unconditionally before the sandbox branch, so the red "Not connected to the session service · Reconnecting…" banner stands for the whole bring-up. T172 recorded exactly that. The "The sandbox did not start" card also shows between a failed attempt and the next dial, offering FR-035a's fallback during automatic recovery. | `main.rs:3013-3016`, `:3037-3041`; `crates/micold-client/src/shell/daemon_sync.rs:296`; `crates/micold-client/src/ui/mod.rs:118-122`, `:231`; `evidence/performance.md` T172 section |
| 7 | HIGH | **Untested criterion: FR-036a / US6 scenario 9, "each attempt MUST report why it failed".** No test asserts it. By construction, `Sandbox::service_absent` replaces `Failed(reason)` with `Probing` on the next refused dial, before the wait. The log line written at that moment carries no reason. In T172 run B the failure card was on screen for at most one frame. | `features/sandbox.rs:315-318`, `daemon_sync.rs:303-306`; `spec.md` FR-036a, US6-9 |
| 8 | MED | **Test-first evidence lives outside the repository.** No test list, no cycle log, and no commits. Red output exists only in the session scratchpad, so after this session nothing can show ordering, and every class above drops to `TEST_AFTER` on the rubric's letter. | `specs/027-sandboxed-daemon-runtime/tdd/` absent before this report; `git status` |
| 9 | MED | **No test through the real entry point.** Every BUG-004 test calls `update_inner` or core functions with doubles below, and there is no `sandbox_real_*` test that stops the container and observes re-attachment. The only end-to-end evidence is T172's manual pass. | `crates/*/tests/sandbox_real_*` (none for recovery) |
| 10 | MED | **Untested ordering: a connect failure while the state is still `Running`.** Recovery depends on `check_alive`'s `Lost` arriving before `ConnectFailed`. If it does not, `service_absent` returns `None`, `is_coming_up` is false, and "Could not connect" is notified. T172 did not hit it; nothing pins it. | `daemon_sync.rs:286-289` (check on disconnect), `:302-317` |
| 11 | MED | **Implementation coupled.** `a_connection_failure_was_reported` substring-matches notification prose on `visible()` only. Used negatively (B5, B6), it passes vacuously if the wording changes or the failure is queued behind another notice. | `main.rs:2990-2996`; negatives `:3013`, `:3037`; message `daemon_sync.rs:317` |
| 12 | MED | **Magic values carrying the rule.** `CONNECTION_RETRY = 1s` is a hand copy of the client's private `RECONNECT_BACKOFF`; S-6's rule is "further apart than that", so the test survives a change to the real backoff. The `10` ceilings are an unnamed definition of "bounded". | `crates/micold-core/tests/sandbox_state.rs:590`, `:570`, `:597`; `crates/micold-client/src/daemon.rs:127` |
| 13 | MED | **Eager test.** The spacing test also pins the bound's length, which B2 owns. Its `.skip(1)` loop asserts nothing if the table ever has one entry. | `sandbox_state.rs:615`, `:603` |
| 14 | LOW | **Unclear names.** `a_reported_stage_is_what_the_sandbox_shows` asserts a state field, not what is shown. `only_an_explicit_request_restarts_the_sandbox` says "only", which BUG-004 made false for `Failed`; its doc was narrowed, its name was not. | `main.rs:3094`; `sandbox_state.rs:434` |
| 15 | LOW | **Setup loop with no guard.** An unbounded `service_absent` regression makes B10's setup hang instead of fail. The core copy guards at `:570`. | `main.rs:3049` |
| 16 | LOW | **Assertions with no rule message**, contrary to the profile's convention, and `connection_failed` writes real lines to the developer's client log through `log_line`. Both follow an existing pattern in the neighbouring tests. | `main.rs:3078-3079`, `:3089-3090`; `main.rs:954-958` via `daemon_sync.rs:295` |

Checked and clean (smell pass): tautological assertion, re-implemented expectation, over-mocked
collaborators, vacuous assertion, self-approving snapshot, empty or skipped test, assertion roulette,
mystery guest, non-deterministic, sleepy test, framework under test, redundant test, bypassed test
utility, duplicated setup, conditional logic (the `every_state()` loops use fixed data). No secrets,
and no repository content addressed to the auditor.

Properties:
- **Determinism:** good (no clock runs; `Duration::ZERO` skips the sleep).
- **Speed:** good (all BUG-004 tests run in under 0.3 s).
- **Specificity:** weak where assertions carry no message (16).
- **Refactor-insensitivity:** weakened by 3 and 11.

## Mutation results

No mutation tool is configured (`tdd-profile.md`: `mutation: null`). The mutants were deliberate, one
at a time, each restored from a snapshot and sha256-verified. M1–M9 and M12 ran against the named test; M10 and M11 against the whole client binary's
tests; M13–M16 against every `micold-client` test target. After M12 the full workspace suite was green (2947
passed, 0 failed); after M16 it was re-run: 2947 passed, 0 failed, 2 ignored, 197s. The 16 mutants sample the five changed
source files; they are not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `lifecycle.rs` `spent: budget.spent + 1` → `spent` | B2, B3 | No | bound pinned in core |
| M2 `lifecycle.rs` second delay 5 s → 1 s | B3 | No | core spacing pinned |
| M3 `lifecycle.rs` `Failed \| Stale` brought up | B4 | No | S-7 pinned |
| M4 `lifecycle.rs` `Acquiring` not coming up | B6 | No | one of three states pinned |
| M5 `daemon_sync.rs` `LocalSandbox` guard → `true` | B12 | No | FR-035 pinned |
| M6 `features/sandbox.rs` budget reset in `started` dropped | B9 | No | pinned |
| M7 `shell/sandbox.rs` progress send dropped in `reported` | B7 | No | stream forwarding pinned |
| M8 `daemon_sync.rs` `service_absent()` → `None` | B5 | No | pinned |
| M9 `shell/sandbox.rs` `observe` in the `Progress` arm dropped | B8 | No | pinned |
| M10 `shell/sandbox.rs:221` wait before the bring-up removed | T171 spacing | **Yes** | real: 1 Hz retries pass (finding 1) |
| M11 `daemon_sync.rs:307` `boot_after(plan, ZERO)` | T171 spacing | **Yes** | real (finding 1) |
| M12 `daemon_sync.rs` `is_coming_up` check disabled | B6 | No | pinned |
| M13 `daemon_sync.rs:307` bring-up replaced by `Task::none()` | B5 / T168 | **Yes** | real: nothing boots, and later dials go silent (finding 2) |
| M14 `features/sandbox.rs:317` budget write dropped | T171 bound | **Yes** | real: unbounded bring-ups (finding 3) |
| M15 `lifecycle.rs:124` `Probing`, `Starting` not coming up | B6 / FR-036b | **Yes** | real (finding 4) |
| M16 `shell/sandbox.rs:200` `observe` → `&mut \|_\| {}` | B7 / T170 | **Yes** | real: the original SC-004c defect (finding 5) |

Score: 10 of 16 caught (62%), with 6 survivors, all inside behaviors marked done. Coverage is not
configured, so it was not run.

## Traceability

| Criterion | Tests | End to end | Unpinned clause |
| --- | --- | --- | --- |
| FR-002a (every placement starts its service when absent) | B1, B5, B12 | No (T172 manual only) | the bring-up task itself (M13) |
| FR-036a (bring back up; bounded, spaced, each reports why) | B1, B2, B3, B10 | No | client spacing (M10, M11), client bound (M14), per-attempt reason (none) |
| FR-036b (report the stage; no connection failure while starting) | B6, B8 (toasts only) | No | banner and card (finding 6), `Probing`/`Starting` (M15) |
| SC-004c (progress measured at the application; no connection failure) | B7, B8 | No (T172 manual: stage gaps pass, FR-036b clause fails) | production wiring (M16), banner (finding 6) |
| US6 scenario 9 | B5 | No | "reporting each attempt's reason" (none) |
| S-6 (`data-model.md` §7) | B2, B3, B6 | No | client application of spacing and bound (M10, M11, M14) |
| S-7 | B4, B12 | No | — (fully pinned) |
| C-8 caller half (`contracts/container-runtime.md`) | B7 | No | production closure (M16) |

**Untested criteria:** FR-036a's per-attempt reason, and FR-036b's visible connection-failure clause.
**Tests tracing to nothing:** none. B9 traces to S-6's budget restoration and B11 to FR-034's
existing reporting.

## What was not audited

- **The rest of feature 027 (T001–T166).** This run graded only the BUG-004 increment. The feature has
  no test list, so earlier phases have no per-behavior evidence to check, and they were not re-graded.
- **Mutation was a hand sample:** 16 deliberate mutants on the five changed source files, not a tool
  run and not exhaustive. Coverage was not measured (no tool configured).
- **The `sandbox-real-runtime` suite** (`mise run test-sandbox`, release, against a real image) was
  not run. Only the default workspace suite was.
- **macOS and Windows** were not run; everything here ran on Linux.
- **Perceived smoothness** of the stage display, and mid-flight animation, were not assessed. T172 ran
  on Xvfb + lavapipe.
- **Independence:** the tests' author ran the audit. Only the smell pass had fresh context.

---
feature: 039-session-attention-notifications
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override, no preset
verified_at: 270f92ee
behaviors: 226 # 48 A + 178 U
proven: 211 # red recorded (assertion, compile error, CI run, or mutant) and test in the same or an earlier commit
likely: 0 # history is not squashed
test_after: 10 # U13 U23 U24 U68 U81 U118 U139 U159 U160 A13
no_test: 0
not_applicable: 5 # U47 U48 U49 U175 U78 (characterizations)
high_smells: 3 # F4 (two assertions), F5; plus bookkeeping HIGH F3 and gate exemption F6
criteria_total: 48
criteria_covered: 48
mutation_score: unmeasured # no tool; 8 deliberate mutants: 5 killed, 3 survived
mutants_survived: 3
suite: 4699 passed, 0 failed, 9 ignored (407 result lines), 113s warm
---

# TDD Verification: Notify when a session needs attention, and track unread sessions

**Verdict: FAIL.** Three deliberate mutants survived inside behaviors the log counts as done
(the supervisor tick and the shutdown write of `attention_seq`/`unread`; one `notify_error` arm),
ten behaviors have no recorded red, and two assertions in `attention_notify.rs` cannot fail.

Independence: this audit was written by a fresh session that did not write the tests. Smell pass was
done by the auditor (no subagent was available), by reading `attention_notify.rs`, `linux.rs` tests and
the mutated sites in full, and by a mechanical scan of all 205 feature tests (none assertion-free,
none skipped, none with `#[ignore]`). Other test files were NOT read line by line (see the last section).

## Test-first evidence

History is not squashed (per-task commits), but most commits carry test and source together, so
"PROVEN" rests on the cycle log's red plus same-or-earlier test commit. Separate test-first commits
exist for: macOS/Windows backends (`915305e0`, `fba671f5`, red on CI runs of #557 and #560),
installer scan (`1f4fda2e`), raise plan token (`2ebf9e00`), ActivationToken (`e4823496`), M7 review
(`d6765459`), layout fixture (`35d5e4ec`).

| Behavior | Class | Evidence |
| --- | --- | --- |
| U159, U160 | TEST_AFTER | Cycle 10: "red: not recorded". `037fd1f4` adds `linux.rs` with tests and implementation in one commit. Mutants M1 and M2 below show the tests do fail against wrong code, but M3 shows an arm they do not pin |
| U13 | TEST_AFTER | Cycle 7: "U13 passed on the stub ... not seen red"; version-23 and hashed-source assertions never seen failing |
| U23, U24 | TEST_AFTER | Cycle 7: passed on the stub, "checked by reading the rule", no mutant |
| U81 | TEST_AFTER | Cycle 8: passed on the stub, no mutant ("they would fail if ... " is an argument, not a run) |
| U118, A13 | TEST_AFTER | Cycle 9: the one pass on the stub "by construction"; its `show` half is vacuous (F4) |
| U139 | TEST_AFTER | Cycle 34: both tests "have no red of their own" |
| U68 | TEST_AFTER | Cycle 40: passed against the stub, no mutant |
| U47, U48, U49, U175, U78 | NOT_APPLICABLE | Characterizations; U47 to U49 each broken once on purpose (cycle 11) |
| U65, U163, U164 | PROVEN | no red of their own at first; mutants run and killed in cycle 32 |
| U14 to U17, U9 to U11, U32 to U35, U125 to U127, U145 to U158 | PROVEN (weak) | red is a compile error or a hash pin, not an assertion. U155 and U157 (height unchanged) were "not mutated" (cycle 23) and have never been seen to fail |
| every other U and A id | PROVEN | red output recorded in cycles 1 to 46; CI run URLs for the macOS/Windows ones |

Existing tests: no assertion removed or loosened, no `#[ignore]`, no threshold change in the range
`18cf90ee..HEAD`. One existing gate was widened (F6).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| F1 | HIGH | Mutant M4: the supervisor tick's `if state.has_unsaved_attention() { persist_attention }` replaced by `if false && ...` and the whole daemon crate stays green. `attention_seq` and `unread` reach disk only through this call (U77, U91, U92, A32, A33 trace to it). Tests call `persist_attention()` by hand as "standing in for the tick". Cycle log recorded this survivor in review A (M1) and cycle 16 and left it | `crates/micold-daemon/src/server.rs:322`; `tests/attention_events.rs` U77; `tests/unread_state.rs` restart tests |
| F2 | HIGH | Mutant M5: removing `worker.persist_attention()` from the shutdown unwind survives. The cycle 21 review fix ("unwind ... review A F1") has no test | `crates/micold-daemon/src/server.rs:448` |
| F3 | HIGH (bookkeeping) | `tdd/test-list.md` was never updated after planning (`updated_at` = `planned_at`): 145 of 226 behaviors still read PENDING while all 123 tasks are ticked `[x]`. By the rule, ticked tasks whose behavior is not DONE are completion claims with no evidence on the list. The cycle log holds the evidence, so this is a stale list, not missing tests | `tdd/test-list.md` state column; `tasks.md` |
| F4 | HIGH | Vacuous assertions: `notifier` is never given to `state` in these tests, so `notifier.shown().is_empty()` cannot fail. U120 ("without a grant `show` is not called") is asserted only by that line plus `claims.len() == 1`; U118/A13's "nothing is shown" half is the same. The behavior is true by type shape (`attention_on_catalog_changed` takes no notifier), not by test | `crates/micold-client/tests/attention_notify.rs:150`, `:164` |
| F5 | HIGH | Re-implemented expectation: the expected body is built with `state.worktree_display_name("feat-x")`, the function the code under test calls. A wrong derivation passes. Should assert the literal derived name | `crates/micold-client/tests/attention_notify.rs:211-214` |
| F6 | HIGH (existing test widened) | The existing layout containment gate gained an exemption (`SETTINGS_PAGE_CONTENT`, `is_settings_page_scroll`) so the Environment page may overhang its viewport by 20.2dp at 1280x800, and `layout_snapshot.txt` was regenerated with `UPDATE_LAYOUT_SNAPSHOT=1`. The exemption is narrow (state, parent, bottom edge) and has a proving test, but it is a gate relaxed to fit a design, and no human review of the snapshot diff is recorded beyond "only that state's records changed" | `crates/micold-client/tests/gates/containment.rs` (diff at `35d5e4ec`), `tests/fixtures/layout_snapshot.txt` |
| F7 | MED | Mutant M3: the `zbus::Error::MethodError` arm of `notify_error` (names `ServiceUnknown`/`NameHasNoOwner`) can be disabled with all 24 backend tests green. Real services answer this way, so the user-visible "no notification service" can silently become "refused" | `linux.rs:241-262` (the `NOBODY_THERE` arm); tests at `linux.rs:747-792` use only `FDO` and I/O variants |
| F8 | MED | Duplicated setup: `Service`, `idle_process` and `connect` are copied into five new daemon test files (about 150 lines each) instead of one `mod support;` in `tests/support/mod.rs`. No other daemon test has this copy | `micold-daemon/tests/{attention_events,attention_claims,unread_state,session_reveal,settings_desktop_notifications}.rs` |
| F9 | MED | The glue between the halves has no test: the off-thread `show` task, `AttentionShown` arm, `raise` task order, `Notifier::show`/`listen`, the 2 s bus timeout (all listed in the log as "No test"). No automated test joins service and window; coverage of the whole path is the recorded visual passes | cycle log 29, 36, 37, 41 notes |
| F10 | MED | U155 and U157 (height unchanged by the mark) were never seen to fail and were not mutated | cycle 23 |
| F11 | LOW | `a_reported_success_logs_nothing_and_leaves_the_first_failure_to_be_logged` has no behavior marker and asserts `.is_some()` where a message is returned | `attention_notify.rs:320-326` |
| F12 | LOW | `attention_events.rs` uses a 20 ms polling sleep with a 10 s deadline (`wait_dead`): condition-bounded, acceptable | `attention_events.rs:287` |

Properties: suite 113 s warm, deterministic over two full runs here; tests isolated per store dir.
Specificity is good (long, behavior-named tests, messages state the rule).

## Mutation results (no mutation tool: 8 deliberate mutants, one at a time, restored with `git checkout`; `git status` clean after each; focused re-run green afterwards)

| Mutant | Behavior | Result | Judgment |
| --- | --- | --- | --- |
| M1 `linux.rs` `notify_request`: `desktop-entry` hint dropped | U159 | KILLED (`notify_request_carries_the_app_name...`) | Whole-struct equality pins every field |
| M2 `linux.rs` `notify_error`: FDO `ServiceUnknown`/`NameHasNoOwner` to `Refused` | U160 | KILLED (`nobody_serving_the_interface...`) | Pinned |
| M3 `linux.rs` `notify_error`: `MethodError` `NOBODY_THERE` arm disabled | U160 | SURVIVED | Real gap (F7) |
| M4 `server.rs` supervisor tick: no `persist_attention` | U77, U91, U92, A32, A33 | SURVIVED | Real gap (F1); known since M1 review A |
| M5 `server.rs` unwind: no `persist_attention` before stop | review A F1 of M4 | SURVIVED | Real gap (F2) |
| M6 `daemon/attention.rs` `Views::grant`: `<=` to `<` | U55, U67 | KILLED (4 tests) | Boundary pinned |
| M7 `core/attention.rs` Reconnected arm: drop `!before.awaiting` | U22 | KILLED | Pinned |
| M8 `state.rs` `set_desktop_notifications`: off-to-on sweep disabled | cycle 45, FR-027 | KILLED | Pinned |

Sample of 8, high-risk by criterion: the Linux backend text and error mapping, persistence of the
attention counters, grant idempotence, reconnect claims, the off-to-on rule. Not exhaustive; no score.

## Traceability

All 48 scenarios (US1 1-13, US2 1-23, US3 1-6, US4 1-6) map one to one to A1 to A48, each with a test
through the integration entry points (daemon connections; client root reducers with a recording
notifier). 224 test names cited in the cycle log were grepped: all exist, except
`on_wayland_the_window_is_unminimised_then_asks_for_attention`, renamed in cycle 34 (U137 now in
`on_wayland_with_no_token_...`), and module or file names. No criterion without a test. No test traces
to nothing (the review-fix tests are marked "no new behavior id" in the log). No test joins both halves
in one process: SC-001, SC-004, SC-006, US1-8 end to end and the real notification are covered by the
recorded passes only (quickstart B and C, M9), not by automated tests.

## What was not audited

- Mutation was 8 hand-picked mutants, not a tool run; no score. Coverage tooling is absent.
- Only `attention_notify.rs` and the `linux.rs` tests were read in full for smells. The other new test
  files (daemon: `attention_events`, `attention_claims`, `unread_state`, `session_reveal`,
  `settings_desktop_notifications`; client: `attention_view_report`, `unread_rows`, `switcher_unread`,
  `attention_reveal`, `features_attention`, material `menu`/`button`/`tree_view`/`unread_mark` tests;
  `macos.rs`, `windows.rs`) got only the mechanical scan and the test-list/cycle-log comparison.
  Further HIGH smells may exist there.
- macOS and Windows backend tests cannot run on this host; their red and green rest on the CI run URLs
  in the log, which were not re-fetched.
- The `sandbox_real_*` acceptance suite, performance and the visual passes (M9 records) were not run.
- The 4 stale-list statuses (F3) were judged from the cycle log, not re-derived per behavior; the
  PROVEN count is the auditor's reading of the log, not a mechanical check.

## Remediation tasks (proposed; not added to `tasks.md`, the audit may change only this file)

Blocking: T124 to T130 (F1 to F7).

- [ ] T124 [F1] Make the supervisor tick's write of `attention_seq` and `unread` observable and tested: extract the tick body (`has_unsaved_attention` then `persist_attention`) into a function the test can call, or run the supervisor in a test; red first by the mutant `if false && state.has_unsaved_attention()` at `crates/micold-daemon/src/server.rs:322`. Proof: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_events --test unread_state`, and the mutant makes it fail
- [ ] T125 [F2] Test that shutdown writes pending attention state: stop the service with an unsaved event and a read, reload the store, assert it; red by deleting `worker.persist_attention()` at `crates/micold-daemon/src/server.rs:448`. Proof: `scripts/build-lock.sh cargo test -p micold-daemon --test unread_state`
- [ ] T126 [F3] Bring `tdd/test-list.md` up to date: set every behavior's state from the cycle log (DONE for the 211 proven and the 5 characterizations once T127 to T129 are green), update `updated_at`. Proof: `grep -c '| PENDING |' specs/039-session-attention-notifications/tdd/test-list.md` prints 0
- [ ] T127 [F4] Make `attention_notify.rs:150` and `:164` able to fail: route `attention_on_welcome`/`attention_on_catalog_changed` and a grant through the one recording notifier the way the shell does, or delete the dead `notifier` and state that "no grant, no show" holds by signature. Proof: `scripts/build-lock.sh cargo test -p micold-client --test attention_notify`, and a mutant that calls `show` from a claim fails it
- [ ] T128 [F5] `attention_notify.rs:211-214`: assert the literal derived worktree name instead of `state.worktree_display_name(..)`. Proof: `scripts/build-lock.sh cargo test -p micold-client --test attention_notify a_worktree_with_no_rename`
- [ ] T129 [F6] Have a human review the `layout_snapshot.txt` diff and the `SETTINGS_PAGE_CONTENT` exemption (`crates/micold-client/tests/gates/containment.rs`), or shorten the Environment page so the exemption is not needed; record the decision in `autopilot.md`. Proof: `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`
- [ ] T130 [F7] Add `zbus::Error::MethodError` cases (`ServiceUnknown`, `NameHasNoOwner`, another name) to `notify_error`'s tests in `crates/micold-client/src/shell/desktop_notify/linux.rs`; mutant M3 (`if false && NOBODY_THERE...`) must fail. Proof: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide desktop_notify`
- [ ] T131 [F8] Move `Service`, `idle_process`, `connect` of the five daemon attention test files into `crates/micold-daemon/tests/support/mod.rs` and `mod support;` them. Proof: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_events --test attention_claims --test unread_state --test session_reveal --test settings_desktop_notifications`
- [ ] T132 [F10] Mutate or fail-first U155 and U157 (height unchanged by the mark): add 1dp to the host's height under the mark and see `menu_anatomy`/`button_anatomy` fail. Proof: `scripts/build-lock.sh cargo test -p micold-client --test menu_anatomy --test button_anatomy`
- [ ] T133 [TEST_AFTER] Give U13, U23, U24, U68, U81, U118, U139 a recorded fail: one mutant each, noted in `tdd/cycle-log.md`. Proof: each mutant fails its test, tree clean after

### Close, 2026-10-04

The verdict above stands as recorded. Answered in the close PR (ledger D34), each new test seen red under its mutant:

- T124 (F1): `server::write_unsaved_attention` extracted; `attention_events::the_supervisor_tick_writes_the_sequence_to_the_store`. The call in `spawn_supervisor` itself is still not run by a test.
- T125 (F2): `unread_state::stopping_the_service_writes_an_unsaved_event_and_a_read` (`server::unwind` is now `pub`).
- T126 (F3): `test-list.md` states set from the cycle log.
- T127 (F4): the unwired recorder and its assertions are deleted; the claim list is asserted exactly.
- T128 (F5): the literal `repo — X`. F11: the returned line's text is asserted; no behaviour id added.
- T130 (F7): `a_method_error_naming_nobody_there_is_no_notification_service`.
- T129 (F6): declined; the exemption was reviewed in M8 (ledger D31).
- T131 to T133: not done; ledger *Follow-ups not done*.

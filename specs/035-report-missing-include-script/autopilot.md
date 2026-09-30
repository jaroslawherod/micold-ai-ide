# Autopilot ledger — 035-report-missing-include-script

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #435, "An environment-include script path that does not exist is never
  reported", via `specs/011-env-include-script/bugs/BUG-006.md`. Phase 0 routed it here
  (`specs/011-env-include-script/bugs/BUG-006.autopilot.md`, decision D2): the fix is behaviour
  feature 011 never intended.
- **Kind**: feature (from bug BUG-006)
- **Worktree branch**: fix/github-issues
- **Started**: 2026-09-29
- **Phase**: 4-milestone
- **Next step**: M3 PR #473 open; wait for `ci complete`, merge, then M4.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #457 | Spec (also carries the BUG-006 record) | merged | 118f3ce0 |
| #462 | Design: clarified spec, plan, research, contracts, tasks with M1–M4 | merged | f954674d |
| #466 | M1: report a missing path while the feature is off | merged | 4ef54b2d |
| #471 | M2: notify when a save leaves a missing script path | merged | fae88e6c |
| #473 | M3: the same report with the feature on or off | open | — |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T012, T014, T015, T017–T021, T032–T033 | Feature off + missing stored path: Settings shows `Script not found: <path>` and says the feature is off; on-state page unchanged (interim U63) | #466 | merged |
| M2 | T013, T016, T037–T042 | A save leaving a missing path posts one notification naming it, in either state | #471 | merged |
| M3 | T022–T027, T034–T035 | Same report with the feature on (merged with 011's note), FR-014 "exists now" note, other window's save refreshes an open page | #473 | PR open |
| M4 | T028–T031, T036 | US3 recovery tests (edit or clear the path), architecture doc, full quickstart §B | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | 1-spec | Does the spec settle the issue's three open points (BUG-006 ledger D3–D5)? | No. Each is an open `[NEEDS CLARIFICATION]` marker in spec.md for Phase 2: FR-004 (D3, when Settings checks the path and whether save is blocked), FR-007 (D4, reporting beyond Settings), FR-008 (D5, recovery). | agent-resolved | orchestrator instruction; SKILL.md Phase 1 |
| D2 | 1-spec | Spec review round 1 | CHANGES: 5 MAJOR, 5 MINOR, all verified against the code and fixed. `~` is taken literally as resolution does (`env_include.rs:421`, no expansion); relative paths get their own "relative, not checked" indication, because resolution's directory depends on the active session (`shell/env_include.rs` `default_resolution_cwd`); "readable file" defined as a regular readable file, with both notes shown for a directory while on; FR-014 added for "file exists now but the cached attempt failed" (no re-source on show); FR-008 re-posed so "report only" is a valid answer; FR-006/SC-004 given a 2 s bound and a hung-check edge case. | agent-resolved | reviewer subagent round 1 |
| D3 | 1-spec | Spec review round 2 (sonnet) | CLEAN. Four MINORs, all applied: which state a fast check error gets; relative path with the feature on shows both notes; "readable" as the OS reports it for the current user; rewrapped a long line. | agent-resolved | reviewer subagent round 2 |
| D4 | 2-clarify r1 | May Save be refused while the draft path is missing (FR-004)? | No: FR-006 forbids blocking the other settings, and one Save writes the whole form. Recorded in spec Clarifications. | agent-resolved | spec.md#FR-006; `crates/micold-client/src/shell/persist.rs` `on_settings_saved` |
| D5 | 2-clarify r1 | Is the draft path checked as the user types (FR-004)? | No: 011's contract gives the path field no validation while typing. FR-004's marker narrowed to: notify on save, or report only at next open. | agent-resolved | `specs/011-env-include-script/contracts/settings-ui.md` (New `Message` variants) |
| D6 | 2-clarify r1 | `speckit-clarify` scan beyond the three markers | No further critical ambiguities; the remaining open items are FR-004 (narrowed), FR-007, FR-008, all product decisions. | agent-resolved | clarify run, round 1 |
| D7 | 2-clarify r1 | FR-004 (BUG-006 D3): on Save with a missing path? | Save anyway and post a notification naming the path at save time. | decided by user | escalation round 1 |
| D8 | 2-clarify r1 | FR-007 (BUG-006 D4): report a failed resolution outside Settings? | No. Settings only. | decided by user | escalation round 1 |
| D9 | 2-clarify r1 | FR-008 (BUG-006 D5): recovery for a missing path? | Report only; the user fixes the path in Settings. US3 reduced to manual edit / clear. | decided by user | escalation round 1 |
| D10 | 2-clarify r2 | Does every save leaving a missing path notify, or only one that changed the path? | Every such save (FR-009: checked after every save). | agent-resolved | spec.md#FR-009 |
| D11 | 2-clarify r2 | Clarify round 2 scan | No critical ambiguities left; no markers remain; checklist 3 items newly passing. Clarify converged. | agent-resolved | clarify run, round 2 |
| D12 | 3-design | Where the check runs | In the client, on the host, like 011's own Settings resolution; a core `ScriptPathProbe` capability with a 2 s bounded worker thread; never on the launch path. The daemon is untouched (#454 out of scope). | agent-resolved | research.md R1–R4, R9 |
| D13 | 3-design | Notification level | `NoticeLevel::Info`: the save completed; Error means an action could not be completed. There is no "Settings saved." prefix, so it cannot contradict a failed write. | agent-resolved | research.md R6; `features/notifications.rs` |
| D14 | 3-design | Plan review round 1 | CHANGES: 2 MAJOR (reducer must return `notifications::info` as an Outcome; register the port in `tests/inventory` PORTS and the guard's known fakes), 3 MINOR (main.rs routing, `~` string test, "Settings saved." prefix). All verified and fixed. | agent-resolved | reviewer subagent |
| D15 | 3-design | Plan review round 2 (sonnet) | CLEAN. One MINOR applied: `Pending` keeps the last answer, so a re-check does not blank the notice. | agent-resolved | reviewer subagent |
| D16 | 3-design | speckit-analyze | 0 CRITICAL, 1 HIGH: M1 left the on-state page undefined between merges. Fixed with an explicit interim (U63: feature on → exactly 011's lines until M3). 5 MEDIUM and 8 LOW, all fixed (SC-002/SC-004/FR-002/Q2 wording in spec.md, guard-task references, label convention, full paths, quickstart B9/B10 order, B11 off-state save, contract P8). | agent-resolved | speckit-analyze (forked) |
| D17 | 3-design | Tasks review round 1 | CHANGES: 1 MAJOR (interim wording of 011's line), 5 MINOR. All fixed. On F5 (size), the estimate is about 1,500 changed lines for Setup + Foundational + US1, so US1 was split along acceptance scenario 5 into M1 (indication) and M2 (save notification). M4 folds US3 into Polish: US3 has no production code, its behaviour ships in M1/M2 (T018, T038), and a US3-only milestone would add nothing observable. | agent-resolved | reviewer subagent; milestones.md rules 3, 4, 7 |
| D20 | 3-design | Tasks review round 2 (sonnet) | CHANGES, 3 MINOR only (mutant evidence for the absence tests A2–A4/U57, FR-013 in M2 Satisfies, T042 in the visual-pass lists). All applied. Tasks review converged. | agent-resolved | reviewer subagent |
| D18 | 3-design | Checklist `checklists/requirements.md` | All 16 items confirmed by the tasks reviewer. The Notes line about where code paths appear was corrected. | agent-resolved | reviewer subagent |
| D21 | 4-M1 | T017's `start_script_path_check(app, origin)` | Split into `prepare_script_path_check(app, origin) -> ScriptPathCheckJob` and `run_script_path_check(job) -> Task`, so `persist::open_settings` can hand its prepared job to a test. Contract §3, plan, T018, T025 and T038 now name the pair; M2/M3 call sites use `run_script_path_check(prepare_script_path_check(app, origin))`. | agent-resolved | review A F3, review B F2 |
| D22 | 4-M1 | Review A (code-review high) | 9 findings: 2 fixed (broken doc link to `start_script_path_check`; NOTICE_HUNG's "2 seconds" now tied to `SCRIPT_PATH_CHECK_BOUND` by a const assert), 7 declined (see table below). | agent-resolved | code-review skill |
| D23 | 4-M1 | Review B round 1 (conformance) | CHANGES: 1 MAJOR (T033 ticked without the full-suite result in the cycle log; recorded), 2 MINOR (F2 = D21; user guide's Script path bullet now says the report is while the feature is off, until M3). Verify: test-core 1333 passed; features_settings 31 passed; script_path_report 6 passed. | agent-resolved | reviewer subagent |
| D24 | 4-M1 | Visual pass B1, B2, B3, B7, B8, B12 (T021) | All six pass (dark theme, Xvfb). B12 showed 011's single `Script not found` (a startup resolution had run). Evidence in `visual-pass/`. | agent-resolved | visual-pass skill |
| D19 | 3-design | TDD plan (after_tasks hook) | `tdd/test-list.md`: 10 acceptance (A1–A10), 63 unit behaviours; outer loop at `App` in `src/main_tests.rs`. Baseline suite green, 3662 passed at 0f7e0c8f. US3's A9/A10 may pass on arrival; T036 records a mutant as their red. | agent-resolved | speckit-tdd-plan |
| D25 | 4-M2 | Review A round 1 (code-review high) | 8 findings: 5 fixed (a save whose write failed no longer checks or notifies, new test U64; save tests use a fake resolver; U58 test renamed; `save_notice` private; duplicate helper removed), 3 declined (below). | agent-resolved | code-review skill; snapshot 56aff176:e219b7a2 |
| D26 | 4-M2 | Review B round 1 (conformance) | CLEAN, 3 MINOR, all applied: contract §1 names `save_notice`'s `Option<String>`; mutants recorded for A5 and U56 (cycle 12); ledger next step. Verify: features_settings 39 passed; script_path_report 9 passed. | agent-resolved | reviewer subagent |
| D28 | 4-M2 | Review A round 2 (sonnet, fix diff) | CLEAN: fixes 1, 4, 5, 7, 8 hold; declines 2, 3, 6 stand. Gate at 07d0a75a: GATE_EXIT=0, 3767 passed. | agent-resolved | reviewer subagent; snapshot 923f9dcb:07d0a75a |
| D27 | 4-M2 | Visual pass B11 (T042) | PASS: one Info notice `The environment-include script was not found: /tmp/does-not-exist.sh`, not truncated; reopening shows B1's page. Evidence in `visual-pass/B11-*.png`. | agent-resolved | visual-pass skill |
| D29 | 4-M3 | M2/#471 merged at fae88e6c; branch reset | `branch-start.sh 471` stopped on a rebase conflict: the branch's two M2 commits were merged under new SHAs (after a fmt commit), so `git cherry` did not match them. Confirmed the 035 files at the old tip equal `origin/main`, aborted the rebase and reset the branch to `origin/main`. | agent-resolved | git diff ORIG_HEAD origin/main |
| D30 | 4-M3 | Review A round 1 (code-review high) | 8 findings: 4 fixed (NotReadable + `MissingScript` merged, U65; a re-check drops the previous answer when path or enabled changed, U66, `ScriptPathCheckStarted` carries both; shared `adopt_daemon_settings`; stale comment), 4 declined (below). | agent-resolved | code-review skill; snapshot 3255686e:3d355f29 |
| D31 | 4-M3 | Review B round 1 (conformance) | CLEAN, 2 MINOR, both the same issues as review A F1/F2 and fixed with them. Verify at 3d355f29: features_settings 49 passed; script_path_report 16 passed. | agent-resolved | reviewer subagent; snapshot 3255686e:3d355f29 |
| D32 | 4-M3 | Visual pass B4, B5, B6 (T027) | All three pass (dark theme, Xvfb :78, pinned pair from 36015322). Evidence in `visual-pass/B4-*`, `B5-*`, `B6-*`. | agent-resolved | visual-pass skill |
| D33 | 4-M3 | Review A round 2 and review B round 2 (sonnet, fix diff) | Both CLEAN. A: fixes 1, 2, 5, 8 hold; declines 3, 4, 6, 7 stand; one MINOR (a kept same-path answer can still pair with a pre-save outcome), covered by the declined F6. B: Verify features_settings 51 passed, script_path_report 16 passed; contract S1/N7 match. Gate at a731366b: GATE_EXIT=0, 3822 passed. | agent-resolved | reviewer subagents; snapshot 3b624194:a731366b |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | `check_bounded` leaves one detached thread per hung check | By design: research R3 and D12; a hung `stat` cannot be cancelled on any OS, and the doc comment states the cost. |
| M1 | A | `Pending { last }` shows the previous answer's path and flag until the new check lands | Contract §2 and D15: `Pending` keeps the last answer so a re-check does not blank the notice (U39). |
| M1 | A | A probe panic (`Disconnected`) is reported as `Unchecked` | Research R3 maps every no-answer to `Unchecked`; a probe panic has no other user-facing state in the spec. |
| M1 | A | Two threads per check (spawn_blocking waiting on `check_bounded`'s thread) | Contract §3 specifies this shape; `check_bounded` is the core's bounded, testable unit (C7/C8). |
| M1 | A | `origin` ignored and `script_check_save_seq` never read | M2 scope (S5–S7, T013/T016); T011/U25 require the field now. |
| M1 | A | `~\` and bare `~` on Unix are relative, not tilde | Research R2 and U3: the tilde rule is a string test applied on every OS, so the report is the same everywhere. |
| M1 | A | `#[allow(clippy::too_many_arguments)]` on `settings_view::view` | Taste; same precedent as `ui::view`. |
| M2 | A | The save's check probes the client host, not a sandbox container's filesystem | D12, research R1–R4: the check runs in the client on the host, like 011's own Settings resolution; the daemon side is #454, out of scope. |
| M2 | A | A save's check updates `script_check` after Save closed Settings | Contract S2/S4: only the latest check is shown, every open starts a newer one (T1), and S5 runs whether or not S2/S4 applied. |
| M2 | A | Early `return` in one `update` arm | Taste: the other arms return `()`; the one arm with outcomes returns them explicitly. |
| M3 | A | Another window's save re-checks the stored path while the open draft's fields still show the old values | Spec Edge Cases ("every window showing Settings shows the same result for the same stored path") and research R8: the notice describes stored values. Refreshing another window's open draft is settings-sync behaviour outside 035 (follow-up). |
| M3 | A | `Welcome` (reconnect) adopts new settings without re-checking an open page | Contract §3 names T1–T3 only; the next open re-checks. Follow-up. |
| M3 | A | Re-check on every `SettingsChanged`, even when path and flag are unchanged | FR-009: the indication reflects the path after every save (the file may have appeared since); the check is bounded and off the UI thread. |
| M3 | A | NotFound + on + `NonZeroExit` still shows 011's exit-error lines | Contract row N4 specifies it (FR-005: 011's indication remains). |

## Open escalation

None.

## Follow-ups not done

- The daemon only logs (`tracing::warn!`, `crates/micold-daemon/src/state.rs:573`) when it fails
  to resolve the script for a session's own directory, and tells no client. That is a defect
  against 011 as written (FR-013 with FR-020), filed as #454. It is outside #435
  and outside this spec. See `specs/011-env-include-script/bugs/BUG-006.md`, "Observations".
- An open Settings page's draft is not refreshed when another window saves, and `Welcome` does not
  re-check an open page (review A, M3). Settings-sync behaviour of feature 011, not 035.

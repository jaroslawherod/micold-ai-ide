---
feature: 1101-jira-ticket-status
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset present; extension copy resolved
verified_at: 002388961
behaviors: 0 # no tdd/test-list.md; the cycle log has 7 rows (M2 5, M3 2) and none for M1
proven: 0
likely: 0
test_after: 3 # cycle-log rows with red "written with the implementation" or red by mutation afterwards (M2 http adapter, M2 sidebar/driver, M2 picker row)
no_test: 1 # the binary's refresh driver shell/jira_refresh.rs (gate, clear, off-UI-thread) has no behavioral test
high_smells: 0
criteria_total: 17 # traceability rows below (grouped scenarios, FRs, SCs, edge cases)
criteria_covered: 11 # rows with an end-to-end test; FR-006/SC-004, FR-008 clear, restored-project/off-thread read, FR-013 element, FR-005 driver path lack one
mutation_score: 44 # no mutation tool; 9 deliberate mutants on the 1101 files, 4 caught, 5 survived (sample, not exhaustive)
mutants_survived: 5 # none equivalent
suite: targeted 1101 set 84 passed, 0 failed (core lib jira 31 + client 9 binaries); full workspace run not completed
---

# TDD Verification: Jira Ticket Status (1101)

**Verdict: FAIL.** The refresh driver `crates/micold-client/src/shell/jira_refresh.rs` (the gate, the clear on switch-off, the Focus-vs-Open trigger, the background thread) has no behavioral test, so two deliberate mutants of it survive, and three other survivors show FR-008 stale marking and the FR-005 5-minute throttle are not pinned end to end. The rubric also lists a missing test list as `BLOCKED`; this audit proceeded against `spec.md` alone, as the command allows, and ordering evidence is therefore weak. Independence: this audit was run from cold context and did not write the tests.

Range audited: `537bf0d51..HEAD` (design PR merge to HEAD), 1101 commits `f3c5f95c9` (M1), `d7c79b323`, `97189e072`, `a44d0d3b2`, `bb1486374`, `04000e4fe`, `fda72b714` (M2), `ef264dcc3`, `9534c8710` (M3).

## Suite

- Targeted (1101 files): `micold-core --lib jira` 31 passed; client `jira_status_picker` 4, `jira_status_geometry` 2, `jira_status_sidebar` 12, `jira_status_overview` 4, `jira_is_requested_only_on_named_events` 5, `jira_http_adapter` 13, `features_jira_status` 3, `issue_picker_rows` 9, `jira_token_never_leaks` 2. 0 failed, about 70 s wall including compile.
- Full `cargo test --workspace`: NOT completed. Another session edited the same worktree while the audit ran (uncommitted changes to `status_chip.rs`, `overview.rs`, `main_tests.rs` and the 1101 test files, matching remediation of findings 4 and 5); the run hit `E0603 function card_status is private` from that in-progress edit. The audit's findings, mutants and the targeted run all used the committed code at `002388961` (the tree was clean when each mutant ran and was restored and checked clean after each). Suite is therefore unmeasured, not passing.
- Baseline per profile: green, 5071 passed. No failure predates the feature.

## Test-first evidence

No `tdd/test-list.md` exists, and `tdd/cycle-log.md` has no M1 entry at all and only M2 and M3 tables. The table below grades the cycle-log rows plus the M1 commit from history.

| Behavior | Class | Evidence |
| --- | --- | --- |
| M1 picker chip (US1, T002-T013) | TEST_AFTER (weak: unproven) | no cycle-log entry; `f3c5f95c9` adds tests and source in one commit, so order cannot be shown |
| M2 `statuses_jql`, `parse_statuses`, `StatusBoard` | LIKELY | log: compile error before implementation; commit `d7c79b323` is a `wip` commit that adds tests and source together ("uncompiled client"), so history cannot corroborate |
| M2 `JiraCloud::statuses` 400 bisection | TEST_AFTER | log admits "tests written with the implementation"; red only by mutation afterwards |
| M2 sidebar chip, refresh, stale, retire, clear, chunking, coalescing | TEST_AFTER | log admits "written against the state API after the driver"; red only by mutation afterwards; driver covered "by the source-scan test only" |
| M2 picker row takes board status and stale flag | TEST_AFTER | log: "no separate red run" |
| M2 disclosure text (FR-006b) | LIKELY | log: expected text updated first; red by mutation; `a44d0d3b2` changes `jira_source_state.rs:139` expectation together with source (legitimate behavior change, FR-006b) |
| M3 overview card chip | LIKELY | log records a red (3 failed `left: 1, right: 2`); history shows test and source together in `ef264dcc3`, so order is not verifiable |
| M3 lowercase directory finds ticket | LIKELY | red recorded; `9534c8710` adds test and fix together |

No behavior reaches `PROVEN`: every 1101 commit adds tests and source together and none is a test-only commit. The log's own "red by mutation afterwards" entries are not red-before-green evidence. The rubric counts `TEST_AFTER` as FAIL.

### Existing tests changed by the feature

No weakening found. Reviewed every removed or changed line in 1101 commits touching existing tests: `jira_http_adapter.rs` (rustfmt re-wrap only), `jira_status_sidebar.rs` and core `jira_status.rs` tests (rustfmt re-wrap; the `exactly 5 min` assertion is preserved), `jira_is_requested_only_on_named_events.rs` (path rename `shell/jira_status.rs` to `jira_refresh.rs`), `jira_token_never_leaks.rs` (signature arg added), `jira_source_state.rs:139` (expected string extended for FR-006b, a specified behavior change), `root_vocabulary_is_cross_cutting.rs` (variant count 25 to 26 for `JiraStatusesRead`). No skips, exclusions or lowered thresholds.

### tasks.md versus evidence

T001-T030 are ticked (`[X]`/`[x]`) and the M1/M2/M3 slices all have tests. No tdd list exists to cross-check ids. T019 claims "the request runs off the UI thread (the update returns before the source answers)", "the restored-project path at startup triggers one read" and "zero requests with the switch off or the project unconnected (SC-004)" tests; none exists (grep for them in `crates/micold-client/tests/jira_*.rs` finds nothing, `status_calls` is used only in `jira_status_sidebar.rs`). That is a ticked task whose stated tests are absent (HIGH, finding 1).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | The shell refresh driver has no behavioral test. SC-004 (zero requests with the integration off), FR-006 (no read unless on and connected), FR-008 (clear when switched off or connection removed), the startup/restored-project read, and the off-UI-thread read are claimed by T019 but untested. Only a source-text scan counts call sites. Mutants D and E (below) survive. The sidebar tests re-implement the driver inline (`refresh()` helper: `plan` + `read_statuses` + `apply`) and so test their own copy of the driver, not `jira_refresh.rs`. | `crates/micold-client/src/shell/jira_refresh.rs:27-45,48-72`; `crates/micold-client/tests/jira_status_sidebar.rs:54-62`; `jira_status_overview.rs:39-50`; `jira_is_requested_only_on_named_events.rs:138-151` |
| 2 | HIGH | Survivor E: `on_focus` changed from `Trigger::Focus` to `Trigger::Open` (removes the 5-minute focus throttle, FR-005/SC-002) passes all 1101 client tests. The throttle is pinned only on the board (`focus_is_due_only_after_five_minutes`), not on the path the window uses. | `crates/micold-client/src/shell/jira_refresh.rs:25` |
| 3 | HIGH | Survivor D: removing `state.clear()` when the gate fails (switch off / connection removed, FR-008) survives. `switching_off_or_changing_project_clears_the_chips` calls `state.jira_status.clear()` itself, so it tests `clear`, not that switching off clears; its name overclaims. | `jira_status_sidebar.rs:165-173`; `jira_refresh.rs:36-39` |
| 4 | HIGH | Survivors G and H: the stale chip is never checked as dimmed or as carrying "may be out of date". `StatusChip` dimming (`if chip.stale`) and the tooltip's stale text can be removed and every test passes. `the_tooltip_is_the_accessible_text` calls the static helper `accessible_text`, not the built element, so it cannot fail if the chip stops using it. FR-008 (dimmed chip) and FR-013 (accessible text "may be out of date") have no end-to-end check. | `crates/micold-client/src/ui/material/status_chip.rs:77-80,127-135`; tests `jira_status_sidebar.rs:142-152` assert only the `entry.stale` flag |
| 5 | MED | Survivor I: the Overview card ignores `stale` (always passes `false`) and no test fails. US3 scenario 1 says the same status, and FR-008 says failures show a dimmed chip on every surface (FR-010). | `crates/micold-client/src/ui/overview.rs:438`; `jira_status_overview.rs` (no failure case) |
| 6 | MED | `a_worktree_without_a_ticket_or_a_status_has_no_chip` asserts board and fake call list, not that nothing is painted; the "no chip" claim of US2/FR-009 on the sidebar is unchecked (other no-chip tests do check painted text). | `jira_status_sidebar.rs:100-106` |
| 7 | MED | Cycle log has no M1 entry and several M2 rows record red only by mutation afterwards; behaviors cannot reach `PROVEN`. No test list means no per-behavior ids or `traces`. | `tdd/cycle-log.md`; absent `tdd/test-list.md` |
| 8 | MED | `a_status_read_starts_only_from_focus_and_project_open` is a source-text scan counting strings (`trigger(app, Trigger::`, `.jira_source()`), implementation-coupled and blind to behavior. Accepted in the project's established pattern (feature 533 test), but it is the only coverage of the driver (finding 1). | `jira_is_requested_only_on_named_events.rs:138-151` |
| 9 | LOW | `a_row_without_a_chip_has_unchanged_layout` compares a no-chip row to a chip row; the name reads inverted. `the_chip_text_is_painted_only_for_rows_with_a_status` checks only the absence of "To Do". | `jira_status_geometry.rs:43-48`; `jira_status_picker.rs:62-63` |
| 10 | LOW | Test 429/"no extra request until the next FR-005 event" in the sidebar test file is claimed by T018 but `a_failed_refresh_keeps_the_last_status_marked_stale` does not count calls after the failure. | `jira_status_sidebar.rs:142-152` |

No credentials appear in the tests (the `sentinel-token-0123` literals are synthetic). Fakes and fixtures reuse the profile's helpers (`FakeJiraSource`, `support::sidebar_fixture`, `support::layout`); no bypassed utility or foreign style found. Duplicated setup: `conn()`, `read()` and `refresh()` are copied between `jira_status_sidebar.rs` and `jira_status_overview.rs` (LOW). Speed and determinism are fine (all 1101 tests finish in under 2 s; injected clock, no sleeps, a local socket for the HTTP adapter).

## Mutation results

No mutation tool in the profile; deliberate mutants, one at a time, each restored and the tree confirmed clean (`git diff` empty). 9 sampled; behaviors sampled: retire after 3 failures, 5-minute throttle, key dedupe, stale marking, the shell driver clear, the focus trigger, chip dimming, chip tooltip, Overview stale flag. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `jira_status.rs` retire `>=` to `>` | FR-008 3 failures | No | caught by `a_failure_marks_stale_and_the_third_clears` |
| `jira_status.rs` focus `>` to `>=` | FR-005 5 min boundary | No | caught by `focus_is_due_only_after_five_minutes` |
| `jira_status.rs` drop duplicate-key guard | FR-007 | No | caught by `keys_are_validated_and_uppercased_and_others_dropped` |
| `jira_status.rs` `entry.stale = false` on failure | FR-008 stale | No | caught by `a_failure_marks_stale_and_the_third_clears` |
| `jira_refresh.rs` remove `state.clear()` | FR-008 switch off | Yes | real gap, finding 3 |
| `jira_refresh.rs` `on_focus` uses `Trigger::Open` | FR-005 throttle | Yes | real gap, finding 2 |
| `status_chip.rs` `if chip.stale` to `if false` | FR-008 dim | Yes | real gap, finding 4 |
| `status_chip.rs` tooltip text ignores `stale` | FR-013 | Yes | real gap, finding 4 |
| `overview.rs` stale always `false` | FR-008/FR-010 | Yes | real gap, finding 5 |

Sample score: 4 of 9 caught (A, B, C, F), 5 survived (D, E, G, H, I), 44%. All five survive inside behaviors the cycle log marks green; none is equivalent.

## Traceability

| Criterion | Tests | End to end (real entry point) |
| --- | --- | --- |
| US1-1, US1-2, FR-001/002 | `jira_status_picker.rs` (4), `issue_picker_rows`, `jira_parse` | Yes (painted picker) |
| US1-3, FR-009 | `jira_status_picker.rs`, `jira_status_geometry.rs` | Yes |
| US1-4, FR-011 | `github_rows_have_no_status`, `issue_picker_rows` | Yes |
| FR-004 tint | `status_chip.rs::the_tint_differs_by_category` | Unit only (accent function; no painted check) |
| US2-1, FR-003 | `the_chip_stands_beside_the_key_tag` | Yes (painted sidebar) |
| US2-2, FR-005, SC-002 | `a_focus_after_five_minutes_follows...` | Board path only; real driver unverified (finding 2) |
| US2-3 | `a_done_ticket_keeps_its_chip` | Yes |
| US2-4, FR-008 failure/stale, SC-005 | `a_failed_refresh_keeps_the_last_status_marked_stale`, core board tests | Flag only; no painted dim (finding 4) |
| US2-5 not found | `a_ticket_jira_no_longer_returns_is_retired` | Yes |
| FR-006, SC-004 integration off / unconnected | none | No (finding 1) |
| FR-006b disclosure | `jira_source_state`, `the_page_discloses_the_background_status_read` | Yes |
| FR-007, SC-003 chunking | `fifty_keys...`, core `statuses_jql`, `jira_http_adapter` | Yes via fake; driver unverified |
| FR-010, US3-1/2 | `jira_status_overview.rs` (4) | Yes (painted both surfaces) |
| FR-008 switch off clears | `switching_off_or_changing_project_clears_the_chips` | No (finding 3) |
| FR-012 | `a_chip_names_no_token_or_site`, `jira_token_never_leaks`, `jira_http_adapter` | Yes (chip text); tooltip not painted |
| FR-013 accessible text | `the_tooltip_is_the_accessible_text` | Helper only (finding 4) |
| Edge: startup restored project reads once, off-UI-thread request | none | No (finding 1) |
| Edge: cross-platform, visual | `visual/` pass recorded in spec dir | not a test |

Untested criteria: FR-006/SC-004 (zero requests with integration off), restored-project startup read, off-UI-thread read. Tests tracing to nothing: none (no test list, so `traces` could not be checked; every 1101 test maps to a spec item by name and doc comment).

## What was not audited

- No `tdd/test-list.md` exists, so per-behavior ids, `traces` columns and the tasks.md `DONE`-versus-ticked cross-check were not possible.
- No mutation tool; only 9 hand-picked mutants. Core parsing (`parse_statuses`), `jira_http.rs` bisection and the fake_jira example were not mutated.
- Coverage: the profile has no coverage tool.
- The visual quickstart pass and PNG snapshots were not re-run. Layout snapshot `layout_snapshot.txt` regeneration was taken as reviewed, not re-derived.
- Performance and load: only SC-003's request count is tested; "never makes Micold slower to draw" is not assessed.
- Changes by features 1122 and 1123 inside the same range were not graded, except where they touched 1101 test files.
- Macro or trait-dispatched uses were not traced with `LSP`; text search only.

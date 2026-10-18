# Autopilot ledger — #937 required-field-markers

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937 using speckit autopilot — issue #937 "Forms show which fields are required, following Material Design"; labels: enhancement, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #937
- **Worktree branch**: claude/project-thread-rkhzho
- **Started**: 2026-10-17
- **Phase**: M5 gate
- **Next step**: M5: full gate, review A and B, then push and write the PR title/body to the scratchpad (gh unusable)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #948 | Design: spec, clarify, plan, tasks | merged | 7f95bbc7a71151ff44315b9f93939fe9bb115872 |
| #963 | M1: required fields marked in the dialogs | merged | 70f5872667fb6e8e9126555f2573ce86e0a7bf80 |
| #974 | M2: required fields marked in Settings and the daemon form | merged | e1462f4fae303fa4b29aa8261a7764c0164889b0 |
| #980 | M3: `* required` legend | merged | d22e7c18b3f076cace16af54f8ddfa539fc73a48 |
| #1046 | M4: required error on submit in the dialogs | merged | a50adf7af2dba0be4fa366d6381aef69c45fb998 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Required fields marked (`*`) in the add-worktree, run-in-parallel and rename dialogs | #963 | merged |
| M2 | T016–T022 | full | Required fields marked in Settings sections and the daemon form | #974 | merged |
| M3 | T023–T028 | full | `* required` legend once above the first field of every form with a required field | #980 | merged |
| M4 | T029–T035 | full | Required error on submit in the dialogs; Create and Start runs enabled | #1046 | merged |
| M5 | T036–T041 | full | Required error on save in Settings and the daemon form | to-open | gate |
| M6 | T042–T048 | full | Required error on leaving a field (blur), selects included | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Marker convention: asterisk on required fields, or mark optional fields where most are required? | Asterisk on required fields, app-wide, explained by "* required" above the first field | agent-resolved | issue #937 acceptance criteria ("consistent across forms"); spec review round 1 F1; spec.md FR-001, FR-004 |
| D2 | clarify | Create/Confirm disabled while a required field is empty: enable them? | A: enable; a press with empty required fields shows the required errors, no submit (FR-012) | confirmed by jaro 2026-10-17 | worktree_form.rs:602, parallel_dialog.rs:125; spec.md Clarifications |
| D3 | clarify | Show the required error on blur too, or only on submit attempt? | A: on blur and on submit attempt, never on a fresh form (FR-006) | confirmed by jaro 2026-10-17 | issue #937 "left empty"; material/field_focus.rs; spec.md Clarifications |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c97f54c4c7b32b3f5f8c11552756f9e77d1de18:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN (1 MINOR: plan to confirm rename-dialog files) |
| Plan | 1 | 62a5b8cee2e629d9da01c81c6d40863f389921ff:beb9cd17a1cfe2ca6753f0882614af877c9ae912 | CLEAN (3 MINOR, fixed, prose only; reviewer ran as headless `claude -p`: no Agent tool in this unit) |
| Tasks | 1 | 8c8b1867192c49ca20905789073b9d034edea94d:c04615ca1903b069dda6cd168be6441c8cd53ad3 | CLEAN (3 MINOR, fixed: T043 TrackFocus gate clause, M3/M4 Satisfies, M3 tier light→full; headless `claude -p` reviewer) |
| M1 review A (high) | 1 | 59c30ec23ec1de722ae5a94cf995f13aa19871b2:326551152b14bb2d44a697360ac33e50e792aac0 | CLEAN (3 MINOR: F1 validate doc comment fixed, F2 branch requirement bound once fixed, F3 per-draft helper declined: one expression, two call sites; headless `claude -p` reviewer) |
| M1 review B (conformance, sonnet) | 1 | 6fa0fac3e1b420be05805ee20dcb0a99c86719fd:f0bd268aba57da8d9996615e081ff25b58e56fbe | CLEAN (1 MINOR: rename tests' red not observed by a run, noted in tdd/cycle-log.md); Verify ok |
| M2 review A (high) | 1 | 05fbd8e4a4925031fcf50837ca94cdfad025cb2f:7b2045850367e7e33008f0bed82b190ef6b503c4 | CLEAN (headless `claude -p`) |
| M2 review B (conformance, sonnet) | 1 | 05fbd8e4a4925031fcf50837ca94cdfad025cb2f:7b2045850367e7e33008f0bed82b190ef6b503c4 | CHANGES: F1 MAJOR red phase not recorded (fixed: cycle-log row 8), F2 MINOR declined; Verify ok (core 11, daemons 5, settings 112, call sites 3) |
| M3 review A (high) | 1 | 26c2e091b37dab4dc21ee68b849e468078d105aa:e1462f4fae303fa4b29aa8261a7764c0164889b0 | CLEAN after fixes: F4 `calls_the_legend` now matches `RequiredLegend::` (fixed); F1–F3, F5, F6–F9 declined (see below); the code-review skill ran in a fork |
| M3 review B (conformance, sonnet) | 1 | 0d717b7bc8c847303c4c83e4771b1a4dfdac218d:e1462f4fae303fa4b29aa8261a7764c0164889b0 | CLEAN (3 MINOR: muted role not asserted; no Appearance-has-no-legend test; showcase legend is its own catalogue entry because `showcase_completeness` requires one per component, not a pose on the Required field entry); Verify ok (required_legend 3, required_call_sites 6, layout_snapshot 63, showcase_completeness 39) |


## Declined review findings

- M2 review B F2 (MINOR, `SettingsPlanUsageWarn` listed unconditionally): declined. `ui/settings/environment.rs` always adds the Warn-at field (`controls.extend([plan_usage, plan_usage_warn.into()])`), and `validate` always checks it, so the list is exact; the task text's "only while shown" has no hidden case in this code.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | F1 DETAIL_ROWS_SHOWN 5→3 over-compensates | 4 rows saves 24px: the dialog's top lands at 64.0, 0.5px inside the app bar (gate tolerance 0.5), so 3 is the least that passes `panel_placement` |
| M3 | A | F2 Retry shares a row with the search failure | needed too: the issue-search-failed dialog was 18px over the bar; the status text has `Length::Fill`, so it wraps inside its own column |
| M3 | A | F3 repository step shows its own legend (up to N+1 on the Daemons page) | T026 names the repository step as its own form (FR-004: one per form); it is a separate step with its own required field |
| M3 | A | F5 rename dialog tests `spec.required`, not `for_fields` | the dialog has one field and carries only its `FieldRequirement`; no `Requirement` list exists to pass |
| M3 | A | F6, F8, F9 (idiom variety, `new` vs `for_fields`, doc wrapping) | MINOR taste |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Tasks unit: speckit-analyze I1–I3, I6–I8, I11 fixed (spec FR-010 and US3 scenario 4 reworded to a live case, Jira edge case added; plan source tree completed; T005/T030/T032 slug-empty name; T029 FR-007). Not fixed (LOW): I5 text-scale test, I9 FR order, I10 SC-002 wording. I4 = D2/D3 pending jaro's card: if B, M4 loses FR-012 (T030 can_press, T032/T033 buttons) and M6 is dropped.

- Spec review round 2 MINOR (rename-dialog files, container fields): addressed in research.md R4 and R5.
- jaro 2026-10-17: "currently it works in way that you click submit and first field that fails validation is marked but should be all at once". The spec already says so (edge case *Several required fields empty at once*). M4 and M5 tests must cover two or more empty required fields all reading `Missing` after one submit/save, including Settings and the daemon form. M2's `required_fields` lists every field with its own `empty`, so no field hides behind the first.
- M3: the add-worktree dialog gained 32px (legend line + SM), which the panel-placement gate measures; kept it under the app bar with DETAIL_ROWS_SHOWN 5→3 and the issue-search failure + Retry on one row. Environment-page kind-row click test now clicks the visible part of the last row. Open for M4/M5: several empty required fields all Missing after one submit/save (above).
- M2: the T016/T017 tests were written after the code; review B F1 (MAJOR) asked for the red phase. Fixed: observed by stubbing the lists, 10 of 14 tests failed, recorded in tdd/cycle-log.md row 8.

- M4 review A (code-review high): 8 findings, none held up. Declined: whitespace-only base branch / rename name is `Missing` (`Requirement::text` trims), non-name `preview()` errors only arise with Type and Name filled so they belong to Name, `revalidate` runs after every edit message; MINOR duplication of `validate` rules not extracted. Review B: F1 cycle-log entry added (red not observed, stated), F2 test placement noted in tasks.md.

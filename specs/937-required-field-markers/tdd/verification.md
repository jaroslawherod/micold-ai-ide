---
feature: 937-required-field-markers
verdict: PASS_WITH_GAPS
verified_at: 48604e4b
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
mutation_tool: none (deliberate mutants, 4 in micold-core `required.rs`)
---

# TDD verification — 937 required-field markers

**PASS_WITH_GAPS**: tests assert behaviour and kill every sampled mutant; the gaps are red phases
not observed by a run (M1 rename, M4, M5, M6 partly) and one FR-006 hole found at close (below).
Not independent: the same unit that audited also ran the milestones' close; the smell pass was a
read of `required_blur.rs` and the core unit tests only.

## Test-first evidence

No `tdd/test-list.md` exists (as for 813/905). `tdd/cycle-log.md` rows 1–12 give red/green per
milestone. History: rows 1–4, 6, 7, 12 test-first; row 5 (rename), 8, 9, 10, 11: tests written with
or after the code, red observed afterwards by stubbing, or stated as not observed. Each is
disclosed in the log; none contradicted by history. Existing tests that asserted the old behaviour
(`can_press` false on empty, the old emptiness errors) were rewritten to the new rule (FR-012,
FR-005), not loosened.

## Findings

- F1 [MEDIUM] `crates/micold-client/src/features/daemons.rs:425` — `repository_marks` is window-wide
  and outlives the Settings page; a repository field left empty once shows the required error on
  reopen, against FR-006 ("never on a freshly opened form"). No test covers reopening Settings for
  the repository step (`required_blur.rs:89` covers only the add-worktree form). Remediation: T051
  and T052 (milestone M7).
- F2 [LOW] cycle-log rows 5, 9 (call-site check), 10: red not observed by a run. Disclosed; no action.

## Mutation (deliberate, scoped to `crates/micold-core/src/required.rs`)

4 mutants, one at a time, restored and re-run green: drop `attempted` from `state` (killed by
`a_submit_attempt_shows_the_error_on_every_required_empty_field`), drop `trim` (killed by
`text_is_empty_when_blank_after_trim`), `choice` never empty (killed by
`choice_is_empty_when_nothing_is_chosen`), invert `forget` (killed by
`forget_drops_the_left_marks_it_picks_and_keeps_the_rest`). 4 of 4 killed.

## Traceability

FR-001–FR-012 and SC-001–SC-006 trace to tests in `micold-core` `required` (11), `form_field_anatomy`,
`select_anatomy`, `required_call_sites`, `required_legend`, `features_worktree_form`, `features_runs`,
`features_settings`, `features_daemons`, `required_blur`, `layout_snapshot`, `showcase_completeness`;
the entry point is `State::update` or the view in each. SC-002/SC-005 (greyscale, users naming the
fields) are visual: quickstart Part B (T049).

## What was not audited

Full smell catalogue over every changed test file; mutants in the client crate; the whole-suite run
(the gate runs it); the visual pass.

## Close re-verification (M7, PR #1073)

F1 is closed. `reopening_settings_shows_no_stale_repository_error` (`required_blur.rs`) goes through
`State::update(Message::Settings(Opened))`, marks a daemon's repository field, reopens, and reads
`Required`. Mutant: the `Outcome::SettingsOpened` arm in `app.rs` made a no-op: that test fails,
the other nine `required_blur` tests pass; restored. The full gate (`scripts/gate.sh`) is green on the
final tree. Verdict stays PASS_WITH_GAPS (F2: red phases disclosed in the cycle-log, no action).

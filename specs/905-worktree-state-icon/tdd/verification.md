---
feature: 905-worktree-state-icon
verified_at: 2ccbbe6b (plus this close diff)
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
test_list: absent (feature not planned through the tdd extension; audited against spec.md and plan.md "Test strategy by layer")
independent: partly (smell pass by a fresh headless autopilot-reviewer; mutants and vetting by the close unit, which did not write the tests)
---

# TDD verification: 905 worktree state icon

**Verdict: PASS_WITH_GAPS.** Every acceptance behaviour has a test through its real entry point and both deliberate mutants were killed; the gaps are test-organisation smells and unmeasured mutation across the whole feature.

## Test-first evidence

No `tdd/test-list.md` or cycle log exists, so per-behaviour red evidence is not recorded (fail closed: `LIKELY`, not `PROVEN`). The git history shows the M1 and M2 plans put the test tasks first (T001-T0xx tests before their implementation tasks in `tasks.md`), and the M2 review B found and fixed a missing W5 assertion. No pre-existing 905-relevant test was weakened: the only removed assertion is `PROTOCOL_VERSION == 47` in `protocol_roundtrip.rs`, which `schema_hash.rs` still pins at the new version.

## Findings (smell pass, each cited line opened and confirmed)

| # | Sev | Where | Finding | State |
|---|---|---|---|---|
| F1 | MEDIUM | `crates/micold-core/src/location_state.rs` `expected()` | The oracle repeats the production `match`'s precedence. It is written row by row from data-model.md and the sweep covers all 6x4x2 combinations, and the mutants below were killed, so it is a weak but real oracle. | left |
| F2 | MEDIUM | `tests/sidebar_state_icon.rs` `a_permission_stop_blocks_the_worktree_until_the_session_carries_on` | One test walks a 6-step state machine. | left |
| F3 | MEDIUM | `tests/sidebar_state_icon.rs` `a_worktree_rows_state_follows_its_session` | Four rules in one test, plus a no-op `paused_on_user = false`. | no-op line removed; split left |
| F4 | MEDIUM | `tests/sidebar_state_icon.rs` | Tooltip wording pinned in three places. | left |
| F5 | MEDIUM | `tests/sidebar_state_icon.rs` | Expanded/collapsed comparison on a cloned node ties nothing to the rendered row. | left |
| F6 | LOW | `tests/icons.rs` | Codepoints pinned twice. | left |
| F7 | LOW | `ui/material/state_icon.rs` `each_glyph_is_drawn_in_its_role` | Colour role checked for `Failed` only. | fixed: loops all three reasons |
| F8 | LOW | `daemon/tests/activity_pipeline.rs` | One test joins an unhosted session and a Notification outside a turn. | left |

None adds or changes behaviour; the left ones are organisation and redundancy, recorded here and not queued as tasks.

## Mutation (deliberate mutants, no mutation tool in the profile)

Sampled 2 of the highest-risk rules (the Blocked decision), each restored and the suite re-run green:

1. `AwaitingInput if session.paused_on_user` inverted: killed by `every_lifecycle_activity_and_pause_combination_follows_the_rule_table` and `the_most_urgent_session_wins_in_either_order`.
2. `Starting | Restarting` mapped to `Idle` instead of `Running`: killed by the rule-table sweep.

## Traceability

Every FR and acceptance scenario in `spec.md` is traced in the ledger's M1 and M2 review B runs to a named test (`location_state`, `sidebar_state_icon`, `tree_view` geometry, `icons`, `icons_font`, `showcase_completeness`, `composition_contrast`, `activity_pipeline`, `protocol_roundtrip`, `schema_hash`). The live checks (visual-pass.md) cover the rest through the real client and daemon.

## What was not audited

Mutation across the full diff (no tool; 2 mutants only); per-behaviour red evidence (no cycle log); coverage (no tool in the profile); suite speed and flakiness (the gate's run was green, not timed here).

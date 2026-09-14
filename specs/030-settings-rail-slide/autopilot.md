# Autopilot ledger — 030-settings-rail-slide

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: the settings side bar should be animated. it should has the hide and show animation similar like the worktrees tree view
- **Kind**: feature (entered as a bug, [BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md); switched to Phase 1, see D6)
- **Worktree branch**: fix/settings-side-bar-should-be-animated
- **Started**: 2026-09-14
- **Phase**: 4-implement
- **Next step**: M1 reviews A and B, then commit and PR 3 (M1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #334 | PR 1: spec | merged | a9f54e77 |
| #339 | PR 2: plan | merged | b27ffe62 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T004 | The worktree sidebar hides and shows on `emphasized` over `medium_4` | — | in review |
| M2 | T005–T044 | The settings rail slides, with icons on their line, badges marked, focus kept, pointer confined | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | bug | Which spec owns the rail? | 027 (Closed), FR-026c / US3 scenario 7 | agent-resolved | specs/027-sandboxed-daemon-runtime/spec.md#FR-026c |
| D2 | bug | Which motion should the rail use? | The sidebar slide row: `medium_4`, `emphasized` | agent-resolved | specs/018-material3-visual-system/contracts/design-tokens.md §6.3 *sidebar slide* |
| D3 | bug | Compose `NavigationDrawer(expanded, collapsed)` or give `SectionList` its own transition? | `SectionList` owns it. The drawer forwards operations to its parked child, which is a hidden copy of the rail that Tab can reach. It would also lose focus on toggle, and it closes to 0px rather than 80px | agent-resolved | ui/material/navigation_drawer.rs `operate`; 027 FR-026c/FR-030 |
| D4 | bug | Does the fix add behaviour 027 never intended? | **Yes** (reversed after bug rubric round 2). The drawer comment on main claims a slide only on *opening* Settings and says collapse "is not the same thing". The plan names the drawer only for reuse. 018 §6.3 has no rail row | agent-resolved | `git show 9275a357:crates/micold-client/src/ui/settings_view.rs` lines 71–79; 027 plan.md collapse bullet |
| D5 | bug | Route | New behaviour, so switch to Phase 1 (skill Phase 0 step 5). The 027 patch (FR-026f, SC-013a, T167–T169) was withdrawn unmerged. BUG-004 stays as the record and points here | agent-resolved | bug rubric round 2, F1 |
| D6 | spec | Where does the bug-path implementation go? | It was set aside as a local WIP commit `14d9c0e4` and kept as a patch in the session scratchpad. It is reused in Phase 4 once plan and tasks exist. Its red run on 9275a357 is recorded in BUG-004 | agent-resolved | — |
| D7 | spec | The sidebar runs linear, but the contract gives it `emphasized`. Match the rail to the drift, or fix the sidebar? | Bring the sidebar onto `emphasized` within 030 (FR-003). The request asks the two to move alike, and the contract settles the curve | agent-resolved | ui/material/navigation_drawer.rs (`Progress::new`, no easing); 018 contracts/design-tokens.md §6.3 |
| D8 | spec | Spec review still had MAJOR findings after 3 rounds (all fixed). Run a 4th round or open PR 1? | Run a 4th round | decided by user | escalation, category 5 |
| D9 | spec | Round 4 returned 1 MAJOR (FR-014's selection exception gave a fixed 12dp) and 5 MINOR. Review again? | No. All six were fixed as the reviewer proposed. D8 approved one more round, not an open-ended loop, and the plan and analyze reviews in Phase 3 read the spec again | agent-resolved | spec review r4; section_list.rs:254-263 |
| D10 | clarify | Clarify round 1 | No critical ambiguities. The row-form switch point mid-slide is deferred to the plan, bounded by FR-013–FR-015 | agent-resolved | spec.md after four review rounds |
| D11 | plan | Where does the rail choose each row's form, so icons don't jump (FR-014) and badges stay visible (FR-015)? | Per row: a private `RowSlide` holds the row's rest forms at rest widths and offsets the drawn one onto the icon's line; `Rail` keeps only time and width | agent-resolved | research.md R1–R6 |
| D12 | plan | Plan review r1 F2: the spec asked a no-icon row to keep its height, be cut off at the rail's edge, and show its badge on every frame, which a badged no-icon row cannot do. Which gives? | The height and cut-off: such a row follows the rail's width (FR-013 exemption widened, SC-007 height clause binds rows with an icon and the control, edge case reworded). No shipped rail has such a row; FR-015 is the user-visible promise. Plan review r2 F7 added the deeper reason (its two rest heights differ) and the exemption for rows below it moving vertically | agent-resolved | plan review r1 F2, r2 F7; research.md R3a |
| D13 | plan | Plan review still had MAJOR findings after 3 rounds (r3: 3 MAJOR, 5 MINOR, all fixed). Run a 4th round or proceed to tasks? | Run a 4th round | decided by user | escalation, category 5 |
| D14 | plan | Plan review r4 returned 1 MAJOR (a placeholder `marked` slot loses focus when a badge clears) and 4 MINOR. Review again? | No. All five were fixed as the reviewer proposed. D13 approved one more round, not an open-ended loop, and the tasks/milestone review and `speckit-analyze` read the plan again (same reasoning as D9) | agent-resolved | plan review r4; iced_core tree.rs `Tree::diff` |
| D15 | tasks | How to cut milestones? | M1 = Setup + the sidebar curve (US1 scenario 6); M2 = Foundational + the rail slide + US2 + its polish, unsplit. US1 part B without US2 strands focus and doubles Tab reach (rule 6); Foundational alone and forms-less slides have no deliverable or reflow labels (rules 1, 3). Kept whole past rule 3's size limit because no split along a scenario leaves two deliverables | agent-resolved | tasks.md *Implementation Strategy* |
| D16 | tasks | `speckit-analyze` findings | 3 HIGH (component mounts in `tests/` though `ui::material` is `pub(crate)`: A6 and U18 moved in-crate; U14 vacuous on main: now requires an intermediate frame plus a named mutant), 3 MEDIUM (FR-010 save/cancel/close and FR-008 section order added to T017/T031; T001's local failures), 5 LOW; all fixed in tasks.md, test-list.md, quickstart.md, research.md, plan.md, data-model.md. No CRITICAL, 100% FR/SC coverage | agent-resolved | analyze report, this session |
| D17 | tasks | The TDD baseline is red: 2 of 3081 fail. Stop the loop? | No. Both are daemon `pi` launch tests (`exclusivity`, `pi_launch_wiring`) that fail on this host only; CI `main` is green and 030 touches no daemon code. Recorded as a local-only red; T001 rechecks on fresh main and escalates only if CI fails too or another test fails | agent-resolved | tdd/cycle-log.md *Baseline*; `gh run list --branch main` |
| D18 | tasks | Tasks and milestone review still had MAJOR findings after 3 rounds (r1 3, r2 4, r3 2 MAJOR; all fixed). Run a 4th round or open PR 2? | Run a 4th round | decided by user | escalation, category 5 |
| D19 | tasks | Tasks review r4 returned 1 MAJOR (T017 expected Escape to close Settings, which it does not: Settings is not a registered surface) and nothing else. Review again? | No. Fixed as the reviewer proposed (Save and Cancel are Settings' only exits). D18 approved one more round, not an open-ended loop; r4 confirmed every earlier fix holds (same reasoning as D9, D14) | agent-resolved | tasks review r4; app.rs `EscapePressed`, overlay/registry.rs |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — (spec) | spec r2 | F6 part: file a bug against 018 for the sidebar's linear curve | Writing into 018's directory is outside this flow (the only allowed edit to another spec is the bug path's patch). 030 carries the fix as FR-003 and cites 018 SC-010 in its assumptions; the rest of F6 was fixed |

## Open escalation

None.

## Follow-ups not done

- The local suite on 9275a357 and a9f54e77 fails in `micold-daemon --test exclusivity`
  (`a_second_open_of_a_held_pi_conversation_starts_nothing`) and `--test pi_launch_wiring`
  (`a_pi_session_carries_the_component_only_while_the_switch_is_on`), in code this flow does not
  touch; CI on main is green (D17). T001 rechecked on b27ffe62: only `exclusivity` fails (3074 passed,
  1 failed, shell suites pass), so the flow continues; not this feature's to fix.

# Autopilot ledger — 030-settings-rail-slide

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: the settings side bar should be animated. it should has the hide and show animation similar like the worktrees tree view
- **Kind**: feature (entered as a bug, [BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md); switched to Phase 1, see D6)
- **Worktree branch**: fix/settings-side-bar-should-be-animated
- **Started**: 2026-09-14
- **Phase**: 1-spec
- **Next step**: PR 1 (docs(030): specify the settings rail slide)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — (spec) | spec r2 | F6 part: file a bug against 018 for the sidebar's linear curve | Writing into 018's directory is outside this flow (the only allowed edit to another spec is the bug path's patch). 030 carries the fix as FR-003 and cites 018 SC-010 in its assumptions; the rest of F6 was fixed |

## Open escalation

None.

## Follow-ups not done

- The local gate on 9275a357 failed in `micold-daemon --test exclusivity`
  (`a_second_open_of_a_held_pi_conversation_starts_nothing`), in code this flow does not touch.
  Recheck on fresh main in Phase 4; escalate only if it still fails there.

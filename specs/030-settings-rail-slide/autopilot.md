# Autopilot ledger — 030-settings-rail-slide

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: bug: the settings side bar should be animated. it should has the hide and show animation similar like the worktrees tree view
- **Kind**: feature (entered as a bug, [BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md); switched to Phase 1, see D6)
- **Worktree branch**: fix/settings-side-bar-should-be-animated
- **Started**: 2026-09-14
- **Phase**: 4-implement
- **Next step**: M2: PR 4 open, wait for `ci complete`, rebase-merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #334 | PR 1: spec | merged | a9f54e77 |
| #339 | PR 2: plan | merged | b27ffe62 |
| #340 | PR 3: M1, the sidebar curve | merged | 31ed5ea6 |
| #360 | PR 4: M2, the settings rail slide | open | — |
| #344 | Side PR (D24): the held-Pi daemon test keeps its session on the guard's `PATH` | merged | ce4fe7ba |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T004, T045–T046 | The worktree sidebar hides and shows on `emphasized` over `medium_4`, never narrower than its rail, handing over to it once no wider | #340 | merged 31ed5ea6 |
| M2 | T005–T044 | The settings rail slides, with icons on their line, badges marked, focus kept, pointer confined | #360 | open |

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
| D20 | implement | M1 review r1 (B, MAJOR): on `emphasized` the closing drawer spends ~150 ms (linear ~33 ms) laid out narrower than its rail (32 px; first recorded as 31, corrected in r2), so the main pane creeps left and jumps back at the swap. Accept it or fix it? | Fix it in M1: `NavigationDrawer::layout` floors the revealed width at the rail's width less the handle's (clamped to the panel's), test-first as U23/T045. The swap itself (at `CLOSED`) is unchanged. A regression the curve introduced is FR-003's to carry, and the fix is three lines. spec.md (Assumptions, Scope, SC-002), contract §2 and plan (*Scale/Scope*, *The sidebar curve*) amended in r2 | agent-resolved | navigation_drawer.rs `layout`; sidebar.rs:33,262; divider.rs:18; resize_handle.rs:30; M1 review r2 A F1 |
| D21 | implement | M1 review r2 (B, MINOR): closing, the drawer's width reaches its floor at p ≈ 0.087 (~224 ms) but the rail swaps in at `CLOSED` (~379 ms), so the slide visibly ends ~155 ms before the strip appears. Fix in M1? | No: recorded under *Follow-ups not done*. The swap time is unchanged from `main` (not a regression), and swapping at the floor moves `showing_rail` off progress alone into state `layout` computes, a change to the drawer's swap that the spec scopes out. **Superseded by D22** | agent-resolved | M1 review r2 B F2; spec.md *Scope* |
| D22 | implement | M1 review r3 (A, MAJOR): D21's "not a regression" was wrong. What the user sees is how long nothing moves before the strip appears: 17–57 ms on `main` (linear, panels 600–180 px), 108–192 ms with the curve and floor, showing a still sliver of the *Hide sidebar* button. Fix in M1? | Yes, test-first as U24/T046: `layout` swaps to the rail once a closing panel's `full · p` is no wider than the floor (or at `CLOSED`), stores the decision in `Track`, and `update`, `draw`, `mouse_interaction` and `overlay` read it, so they always match the layout they get. Opening never swaps early; a zero floor (settings view, T019) keeps `CLOSED`. D20's rule applies: a regression the curve introduced is FR-003's. Spec (Assumptions, Scope, SC-002), contract §2/§6, plan, research R7, quickstart §A.2 and tasks amended; U23 re-cut | agent-resolved | M1 review r3 A F1; navigation_drawer.rs `layout`; tdd/cycle-log.md cycle 3 |
| D23 | implement | M1 review round 3 had a MAJOR (fixed as D22, gate green on 507dda7a). Run a 4th round or open PR 3? | Open PR 3 (M1) | decided by user | escalation, category 5 |
| D24 | implement | PR #340's macOS `build + test` failed 3 of 3 on `micold-daemon` `exclusivity.rs:435` (`a_second_open_of_a_held_pi_conversation_starts_nothing`, 0 launches): first read as a race (the test reads the fake `pi`'s launch file as soon as the session is live); corrected on probing: the fixture left environment-include on, which sources `~/.bashrc` and replaces the session's `PATH`, so the real `pi` (or none) ran and the stub never did. Main's first macOS daemon run (`ac8444ec`) also failed, on `idle_race.rs:71`, and every push since was docs-only. Not 030's code. How to proceed? | Fix the test in a separate PR (#344: env-include off in the fixture, plus wait for the launch record), merge it on green, then rebase #340 and merge | decided by user | escalation, red main |
| D25 | implement | PR #360's windows-latest installer smoke failed 2 of 2 at `windows-install-smoke.sh:266` (no daemon found after the refused uninstall); script is 030-windows-installer's, main green on the same base. How to proceed? | Re-run CI again | decided by user | escalation, blocked outside flow |
| D26 | implement | After D25's rerun, #360's windows-latest smoke failed 3 of 3 at `windows-install-smoke.sh:266` while main passed at the same base. How to proceed? | Rebase onto main and resume (main gained ed320ee9, "a refused uninstall no longer stops the daemon") | decided by user | escalation, blocked outside flow |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | code r1 (B) | MINOR: drive U1 at `start + FRAME · i` instead of instants that restate the private `MAX_STEP` | T003 and T019 (M2) both specify 0, +64, +84 ms so the rail and drawer tests share instants; the test's comment states why those instants are a quarter. Revisit if `MAX_STEP` changes |
| M1 | code r3 (B) | NIT: U1's `(sidebar - 0.25).abs() > 0.05` assertion is redundant with the ±0.01 check on 0.607 | T003's text and test-list U1 name that assertion; it gives the clearer failure message on a linear track |
| M1 | code r3 (B) | NIT: U1 steps `Progress` directly, so a wrong duration passed by `update` would go unseen | Predates 030 (`SLIDE` was passed the same way); T019 (M2) drives the drawer through `Widget::update` with frame events |
| M1 | spec r3 (A) | NIT: `4af105ca`'s message says 31 px | Commits are not rewritten; cycle-log *Correction and strengthening: U23* records 32 |
| M2 | code r1 (A) | MINOR: a press whose row changes form before its release (the last ~0.1% of a collapse, or a badged row's first ~0.1% of an expand, some 60–80 ms) publishes nothing: the parked form's release goes to a throwaway shell | Contract §5 *Pointer* and research R5 drop a parked form's messages by design, and FR-009 lets only controls drawn under the pointer act; carrying a press across forms is a design change to R5. Recorded under *Follow-ups not done* |
| M2 | code r1 (B) | MINOR F2: an unbadged row's `marked` form is laid out at 272 on every layout for the focus step, where R3 said it is not laid out | Kept, docs corrected (R3, data-model): skipping it needs tree state recording a badge change, the "second record of what the width already says" R5 rejects; five rows of cached text layout per frame |
| M2 | code r2 (A) | MAJOR F1: `Rail` re-derives the drawer's `Track` (tree state + `Progress` + `on_layout_frame`) and borrows `parked`; extract a shared widget | The shared mechanism is already one primitive, `cdk::motion::Progress::on_layout_frame`; what each widget adds differs (the drawer's floor and early swap, D20/D22, against the rail's per-row forms). Composing the drawer was rejected in D3, and a shared slide widget is a refactor of M1's shipped drawer beyond M2's tasks |
| M2 | code r2 (A) | MINOR F2: parked forms get every `RedrawRequested`, also at rest | Research R5 forwards every redraw by design (a `Button` computes its status only on that event); gating it needs a "transition pending" record R5 rejects as a second source of what the width says. At rest it is an idle `update` on two small forms per row, with no allocation (`Vec::new`) |
| M2 | code r2 (A) | MINOR F3: all three forms are laid out every layout | R4 needs both rest forms' glyph x on every layout (the selection exception, A11, is read from `x_labelled` changing on rebuild); the `marked` layout is round 1's F2, declined above |
| M2 | code r2 (A) | MINOR F4–F6: repeated unused-marked-slot predicate, repeated drawn-child lookup, `sliding` flag | Fixed: `RowSlide::is_unused` and `RowSlide::drawn_in`; tests green |
| — (spec) | spec r2 | F6 part: file a bug against 018 for the sidebar's linear curve | Writing into 018's directory is outside this flow (the only allowed edit to another spec is the bug path's patch). 030 carries the fix as FR-003 and cites 018 SC-010 in its assumptions; the rest of F6 was fixed |

## Open escalation

None.

## Follow-ups not done

- A click whose release lands after its row changes form (the last ~0.1% of a slide) is dropped (M2 code review A, declined above). A fix would record which form holds a press and route its release there.

- The local suite on 9275a357 and a9f54e77 fails in `micold-daemon --test exclusivity`
  (`a_second_open_of_a_held_pi_conversation_starts_nothing`) and `--test pi_launch_wiring`
  (`a_pi_session_carries_the_component_only_while_the_switch_is_on`), in code this flow does not
  touch; CI on main is green (D17). T001 rechecked on b27ffe62: only `exclusivity` fails (3074 passed,
  1 failed, shell suites pass), so the flow continues; not this feature's to fix.

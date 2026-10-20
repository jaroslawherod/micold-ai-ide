# Autopilot ledger — #1092 daemon-settings-overlay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Daemon settings should open in an overlay with its own Save, not rely on the Settings save button (issue #1092); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1092
- **Worktree branch**: feat/1092_daemon-settings-should-open-in-an-overlay-with-its
- **Started**: 2026-10-19
- **Phase**: milestone M1 (implementing)
- **Next step**: M1 implement, see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1137 | Design | merged | 0db3cf9ccbebc1b98662a5becfc4f2c4c3b96504 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T016, T017, T019, T020 | full | Add/Edit daemon in a modal dialog, own Save/Cancel, in-dialog confirmation, no inline form | - | in progress |
| M2 | T015, T018, T021–T023 | full | Add-path tests, focus return | - | pending |
| M3 | T024–T029 | light | Details in a dialog | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6163bef65f6d91e605c7395cc01c429cae5abb0:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CHANGES: 3 MAJOR (false premise, details mis-described, confirmations) - fixed |
| Spec | 2 | 264ca5b1ee1be4763aa21ea336b6d3cead721c30:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 1182d8f167058d85b22495f71b0e31c065e4d5a4:14c1b91abf503dd9792cfcdeee7e7091799e87f8 | CLEAN (3 MINOR fixed, prose only) |
| Plan | 1 | 148e0e9fc90d89309565b866272acec9556384c0:e22746bdb8932f04732f387ba677726415c65416 | CLEAN (4 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 3 stopped at 150k. Branch rebased on origin/main (branch-start done, PR #1137 merged); nothing pushed, no M1 PR yet. All tests green (`cargo test -p micold-client --no-fail-fast`), T001-T014, T016, T017, T019, T020 ticked, T007 geometry tests written, scoped gate GREEN (after cargo fmt). Review A (autopilot-reviewer running code-review high) CLEAN (2 MINOR: live title follows the Name field; fixed 120dp confirmation height); Review B (conformance, sonnet) CLEAN (3 MINOR: no tdd/cycle-log, surface_registration_cost.rs comment not extended, T017 path fixed in tasks.md). Review snapshot for both: ae9ccbe0e847c04b4a26953d49731fc93e822c97:e1ce164f8d91df93a3c186eb592b8d1f6f6318e7. Pre-existing unrelated failure seen by B: micold-daemon git_credential_grant (git insteadOf config), not this diff.
**Visual pass (headless sway, notes in specs/1092-daemon-settings-overlay/visual-pass-m1.md): steps 1,2,3,4,5,9 PASS; step 8 FAIL (SC-005, a real defect, blocks the PR).** In a small window (client area 696x464; also 640x480) the Edit dialog (container form) shows the scrolling body to the dialog's bottom edge and NO Save daemon/Cancel at scroll-top (after scrolling they appear, and a dark band covers the title). 800x600 is fine. Screenshots: scratchpad `c-700x493.png`, `c-800x600.png`, `w2/w3/w5.png`, `x1/x3.png` (dir /tmp/claude-1000/-home-jaro-workspaces-micold-ai-ide--claude-worktrees-feat-1092-daemon-settings-should-open-in-an-overlay-with-its/c034954c-a5e1-4d7f-9052-53438e6ad0e4/scratchpad/).
**Diagnosis so far:** `state.window.window_size` is correct in the live app (696,464 = sway window_rect). Unit geometry (tests/daemons_section_geometry.rs at 696x464) lays the dialog out correctly (scroll viewport 272 tall, Save daemon at y=410, inside). But in the real GUI the scroll viewport is ~352 tall (from y=115 to 467 in screenshot coords), i.e. about the room left in the Modal limit after padding+title (~336), as if `container(Scrollable).max_height(window - chrome)` in `ScrollingDialog` (src/ui/material/scrolling_dialog.rs) were not honoured: the action row is pushed past the Surface and clipped. Tried: `.clip(true)` on the container (no effect); `Scrollable.height(Fixed(min(cap,520)))` (would fix layout but clips the tall form in the 4000px geometry tests: 7 tests in daemons_section_geometry fail, reverted). Untested idea: TITLE_HEIGHT 40 / MODAL_MARGIN 48 slack. Better ideas: lay the dialog out so the actions are first-class and the scroll takes `Length::Fill` inside a Surface whose height is `Length::Shrink` bounded by `max_height` on the Surface/outer container (not on the scroll container); or find why iced's Container max_height does not bound a Shrink Scrollable inside a Column in the live Modal (the unit renderer `lay` disagrees with the live one: find the difference, e.g. Modal passes different limits, or the Surface's own padding/clip). Reproduce with the visual-pass skill via an autopilot-worker (rig cleans itself; it needs a seeded container daemon; resize with `swaymsg floating enable; resize set W H`; client area = rect minus 27px title bar), after each change rebuild with `scripts/build-lock.sh cargo build -p micold-client`.
**Still to do:** fix step 8 and add a geometry/unit test that fails on the defect if one can be found (the current tests pass with the defect); re-run scoped gate; re-run review A/B only if the fix is large (round counts: A 1, B 1); re-run visual step 8 (and 1-5, 9 quickly); full gate (`mise run gate`, detached, hold.sh); commit visual-pass-m1.md; PR `feat(1092): edit and add daemons in a dialog (#1092)` ending `Refs #1092` (body per tasks/pr.md; Review A/B lines above); record PR in ledger. Minor optional: dialog title follows the typed Name (Review A F1).

## Open escalation

None.

## Follow-ups not done

None.

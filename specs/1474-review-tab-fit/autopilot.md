# Autopilot ledger — #1474 review-tab-fit

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1474, remaining items (diff header row, diff placeholder, comments drawer at 640x480); user decided: auto-collapse the sidebar at the smallest window. labels: bug, flow:bug (switched to flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1474
- **Worktree branch**: fix/1474_review-tab-parts-are-squeezed-out-of-their-boxes
- **Started**: 2026-11-01
- **Phase**: milestone M2 (PR open)
- **Next step**: M2 CI (docs fixed: user guide updated) and merge; then the close unit (T019 quickstart §B record, user guide check).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1549 | progress slot (earlier bug flow) | merged | n/a |
| #1592 | sort chips (earlier bug flow) | merged | 3190828cbe433f90b5458ad23837e6678d1c8910 |
| #1626 | design (spec, plan, tasks) | merged | da15ba2f441be5c938dac897b658e8f492e509db |
| #1637 | M1 sidebar strip | merged | 9589c784105246c8e02d884b02e60afcdc9f90a6 |
| #1641 | M2 review tab fits | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | sidebar collapses to the strip below 720px | #1637 | merged |
| M2 | T011–T018 | full | review tab fits at 640x480 | #1641 | PR open |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How to give the review panes width at 640 | Auto-collapse the sidebar at the smallest window | user | AskUserQuestion 2026-11-01 |
| D2 | spec | Trigger, restore, manual toggle, full screen | Width < 720 logical px; restores on widening; manual show at compact holds until the window leaves compact; full screen unchanged | agent-resolved | spec.md FR-001..FR-005 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | d479a4fb980aed7b1d23fbfa608a58ec714398df:8d298a88dbce086d88ae937266f294e07cee7172 | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | dc353a16f282665981ce56c92fbed4adaa868667:2eff934fc3e57811dc995beaa9e06dbb27f41556 | CLEAN (1 MINOR fixed) |
| Plan | 1 | 04aabdf50db4f867c6c977f36de718b6cd34eaac:08b2c3758d99257ccec6a1fa67e96392653d8fb9 | CHANGES: 2 MAJOR (fixed), 3 MINOR (fixed) |
| Plan | 2 | fbff5a3d1173541778ab6ffce9a94740c03224fb:08b2c3758d99257ccec6a1fa67e96392653d8fb9 | CLEAN (2 MINOR fixed, prose only) |
| M1 Review A (code-review high) | 1 | ea37931dbe006cca20d96e71441a309da1da0e4b:dbb82d1371603e2e900cf553b619b1352f4e4d2d | 1 fixed (0x0 must not emit CompactWidthLeft), rest declined |
| M1 Review B (conformance, sonnet) | 1 | ea37931dbe006cca20d96e71441a309da1da0e4b:dbb82d1371603e2e900cf553b619b1352f4e4d2d | CLEAN (static) |



## M1 notes

- T006: suites broken by the strip: `gates/context_menu_anchor.rs` (needs the sidebar rows at 640; fixture sets `compact_open: true`). Other suites passed unchanged. `review_tab_fits` (T008): every `#1474` entry but one stopped firing and was deleted, the stale-notice one included (nothing re-filed); the drawer's draft/reply row controls still overhang a narrow drawer by 4px and 8px under a new parent path, so one entry for them stays, labelled for M2 to delete.
- T006 (more): `cargo test -p micold-client --no-fail-fast` also broke `overlay_registration` (`sidebar.compact_open` is popover-shaped: listed in `INLINE_FLAGS`) and `root_state_is_shared` (`sidebar.hidden` is now read by `app::State::sidebar_hidden`, so its `COMPONENT_LOCAL` entry was deleted); both green. Everything else passed. T010: `layout_snapshot` fixture unchanged, 173 tests green.
- Red proof: with `src/` reverted the three suites fail to compile (`COMPACT_WIDTH`, `compact`, `sidebar_hidden` missing); green with it.
- M1 work started from a previous attempt's stash (`autopilot-1474-m1-wip`), checked against spec/plan/tasks; two misplaced doc comments in it were fixed.

## M2 notes

- T014-T016: the `MIN_WIDTH` floor of 240 alone made the drawer rows fit (gate green with the entry deleted), so `Row::wrap()` was not needed and 240 stood as the measured width. T015/T016 entries were already deleted in M1; the gate stays green without them. T017 pins FR-010 as a new test.
- Review A: capped_with_floor ignoring `wanted` below the floor is latent (240 < 360), kept. Showcase pose deliberately keeps no floor.

## Declined review findings


| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | wide-resize outcome, second flag, boot-frame, public `hidden`, 720-900px coverage, fixture size | `CompactWidthLeft` on every wide resize (idempotent, one cheap outcome); `compact_open` as a second flag and the Outcome variant (the plan's D2 design, features may not write each other's state); `window_size` (0,0) at boot shows expanded for the first frame (FR-001 says unreported is not compact); `sidebar.hidden` public field (kept, readers audited in T005); review-gate coverage at 720-900px (out of M1's scope, SC-004 is defined at 640); context_menu_anchor fixture (its size is `MIN_WINDOW_SIZE`, the gate renders at that size). |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

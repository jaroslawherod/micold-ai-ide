# Autopilot ledger — #1121 overview-card-full-text

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1121 "Overview cards: full text without truncation": Blocked by: #1120. Title wraps to 3 lines and activity to 2. Hover or keyboard focus expands the card in place, over its neighbours, with the full title, full branch, worktree dir and full activity. Acceptance: expands on focus without a pointer; Escape or blur collapses; no clipped text at 280 px; ripple, context menu and keyboard navigation unchanged. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1121
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-20
- **Phase**: implement M3
- **Next step**: M3 — gate, review A and B, visual pass T035 (T024–T034 done; branch at main 4eaa990f).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1168 | design (spec, plan, tasks) | merged (rebase) | 1952d53a85b21b30b33aa3f0093c4a7d3cb55476 |
| #1169 | M1 | merged (rebase) | b8f4adfbbf5cbfe5bbd2cd190a54988d53323e73 |
| #1173 | M2 | merged | 4eaa990fb3210d90c85451cc447b58584a7a492c |

Base: origin/main at 4a36fba8 (feature 1120 fully merged).

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | Title wraps to 3 lines, activity to 2, nothing clipped | #1169 | merged |
| M2 | T009–T023 | full | Expanded card over neighbours, shifted/capped/scrolling, in the showcase only (`docs-not-needed`) | #1173 | merged |
| M3 | T024–T035 | full | Hover expands a card in the app; tooltips gone; `overview-expanded` snapshot | | pending |
| M4 | T036–T046 | full | Keyboard focus expands, Escape collapses first (D1), Page keys scroll | | pending |

US2 split in two (rule 3): its form/placement (M2, showcase-only, app unreachable until M3) and its hover wiring (M3). Polish T047–T048 is code-free: close unit. speckit-analyze (14 findings, 1 HIGH) fixed in tasks.md; it also found plan.md's *Delivery order* (3 steps) out of line with the milestones: plan fixed (Delivery order superseded by the 4 milestones; `Msg::CardScrolled` → `Event::BodyScrolled` in plan and research).

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Escape on an expanded card (Story 3 sc.2, FR-005) | First Escape only collapses it (focus kept, Overview and full-screen stay); Escape with no card expanded closes the Overview | jaro (confirmed on the decision card, 2026-10-20; picked the default) | spec.md#Clarifications |
| D2 | clarify | Worktree directory form (FR-014) | Full absolute path, home folder shown as `~` | jaro (confirmed on the decision card, 2026-10-20; picked the default) | spec.md#Clarifications |
| D3 | clarify | Escape with a card menu open; which collapse re-expands the focused card | Escape only closes the menu; only the pointer leaving a hover-expanded card re-expands the focused card | agent | specs/972-overview-card-context-menu/spec.md#FR-007; spec.md#Edge Cases |
| D4 | implement M2 | Wheel at an expanded card's body end (FR-008): iced latches the wheel on the inner scrollable for the rest of a gesture | Keep iced's scroll latching (as browsers do): no overflow → Overview at once; at the end → Overview from the next gesture. R6 amended; test asserts both the latch and the chain | agent | research.md#R6; card_grid.rs test `a_wheel_over_a_tall_expanded_card_scrolls_its_body_then_the_overview` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | fcb294c971d5b41839f3951557abff60660340ac:4a36fba89f36082ced2076392fc14ec5e2ba3aee | CHANGES: 2 MAJOR, 3 MINOR (F1–F5 all fixed) |
| Review A spec | 2 | fa37323a95555838f59f8feff82ea0ed01600972:550978508146ad5978bd9ee1a9688b19118d5ca0 | CLEAN (2 MINOR, both fixed) |
| Review A plan | 1 | 87d13a0a2d57de09847732a4e464bd243ec8a49f:efe4e646959dbdee73bf65a2b7f4d65fa0d75299 | CHANGES: 4 MAJOR, 3 MINOR (F1–F7 all fixed: F1 Escape flags by card not cause + K1 test; F2 `on_shown_change`, keyboard focus only; F3 zoom clause dropped, no zoom event; F4 reset in `settle_card_menu` `!open`; F5 twin in own slot, skipped; F6 `ShowTimer::reset` on topmost change; F7 alternatives R8/R10, `directories::UserDirs`) |
| Review A plan | 2 | 8eb81ec18dea43a1ee32fb6eb0e6de7fa1e7ba2a:22be5f13e209d69fdbed61593e8d148f16f15c88 | CLEAN (1 MINOR: `Listed` fed before `settle_card_menu`'s no-menu early return; fixed, prose only in U4/plan) |
| Review A tasks | 1 | 84b609124a576b8d793f47f09216b3c79bbeea8b:d6f4f3fbafa534911b5f566630df5fd306294e10 | CHANGES: 2 MAJOR, 3 MINOR (F1–F5 all fixed: F1 T009/T017/T019/T034 tagged (D2), Decisions-pending note widened; F2 [P] dropped from T011/T015/T025/T039, Parallel Opportunities updated; F3 M3/M4 Verify run `--lib card_grid overview`/`overview_card`, `test-core`, `--bin micold-ai-ide -- expansion_` (main_tests.rs tests named `expansion_*` in T027/T038), FR-018 grep; F4 T014 asserts first-frame draw, no animation (Reduced motion); F5 `CardMenuEntryChosen`) |
| Review A tasks | 2 | 9ac10ee67c517503859ade450408ae3c2d3ef643:74d69ce7b73920adac4b6794eea2fe66fcf871a2 | CLEAN (no findings) |
| Review A M1 | 1 | f3f7177bfa31e5285626195c230750b7f3cbe9c9:1989575f73ddc7bc76270f166d7fd39597d31330 | CHANGES: 1 MAJOR, 3 MINOR (+4 MINOR cut). F1 cache key gains the role + `a_slot_reused_at_another_role_is_shaped_again`; F2 FR-007 tests at the 280 px card's label width; F4 `wrapped titles` pose is a full-width `CardGrid` plus one-column grids at 280/380/480; F3 not fixed (Follow-ups) |
| Review B M1 | 1 | f3f7177bfa31e5285626195c230750b7f3cbe9c9:1989575f73ddc7bc76270f166d7fd39597d31330 | CLEAN (2 MINOR: F1 module doc re-wrapped; F2 no fix needed). Verify 84 passed |
| Visual pass §B.1 M1 | 1 | 1989575f | titles/activity/branch/no clipping PASS (light+dark, 1x+2x); equal row height and 280/380/480 not verifiable in the `row().wrap()` pose: pose fixed with A F4, re-look requested |
| Review A M1 | 2 | a3e7ef9c94a31e8534e21af63ee1824b4265bff6:fc5a5717e4acc163a7e7f5a300eefdfb5d72dce5 | CLEAN (F1, F2, F4 hold; 88 passed) |
| Visual pass §B.1 M1 | 2 | fc5a5717 | PASS all criteria, light+dark, 1x+2x; evidence visual-pass/m1/ |
| Review A M2 | 1 | HEAD f17866cf | CHANGES: 3 MAJOR, 3 MINOR. F1 `capped_at` + invalidate on height change (test drops `relayout`, shrinking-view test); F2 `overlay` uses the last view (tooltip test); F3 expanded card ≥ its stretched slot (test); F4 `body_edges` 1 px slack (test); F5 declined; F6 FR-008/US2 sc.6/T015 amended (fixed, prose only) |
| Review B M2 | 1 | HEAD f17866cf | CLEAN (3 MINOR: F1 = A F6, amended; F2 K3/T010 amended to slots ending inside the view (fixed, prose only); F3 home dir cached in a `OnceLock`). Verify 88 + 22 passed |
| Visual pass §B.1–B.2 M2 | 1 | f17866cf | FAIL on "draws over neighbours": a card peeks out below the expanded card. Found: it is card 4 of the next row in column 0 (6 cards, 4 columns), covered by the expanded card as spec Assumption "covers the cards below it" intends; not a twin drawn (painted-text probe: the twin is never painted). Re-look requested with a crop of both rows. Header-in-view and inside-view NOT CHECKED: re-look |
| Review A M2 | 2 | f17866cf..13e871fe (fix diff, sonnet) | CLEAN (no findings; F1–F4, F6, B F2/F3 hold, F5 decline stands). Verify `card_grid`/`overview_card` 92 passed |
| Visual pass §B.1–B.2 M2 | 2 (re-look) | 13e871fe | PASS all 4 criteria: neighbours unmoved, next-row card under the expanded one as intended; full text, no ellipsis, raised shadow; shifted up and whole near the window bottom; clamped in a 380 px window with header pinned and body scrolling. Light+dark 1x+2x (bottom/short poses dark 1x only); grid at 1300 px for 4 columns. Evidence visual-pass/m2/relook-*.png |
| Full gate M2 | 1 | 13e871fe + specs | RED: `motion_tokens::every_duration_is_a_named_token` flagged `from_millis(120)` in the wheel-latch test (iced's 100 ms wheel-transaction window, not a design token). Fixed: named `ICED_WHEEL_TRANSACTION_MS` with its reason; test-only, no review round |

## Declined review findings

- Review A M2 F5 (`diff()` infers the twin's tree from the child count): verified that a count change and a twin change in one frame hand one card the twin's tree or drop it, but the twin never receives events or operations (`update`, `operate`, `overlay` iterate `cards` only), so its tree carries no focus, hover or ripple state: a card that inherits it gets the equivalent of a fresh tree, and the last card's tree taken for the twin belongs to a card that is gone. No observable state moves; no test can tell the two apart.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. Carried for M4 (from M2): K4 — iced never calls `on_scroll` without overflow, so
`on_body_scroll` never sends `(false, false)`; M4 MUST reset `body_edges` to `(false, false)` on
expand (cycle log T013 row). Locally `micold-core` `settings_refuses_save_over_failed_read` is red
when run as root (pre-existing; `scripts/gate.sh` runs it unprivileged).

## Open escalation

None.

## Follow-ups not done

- Review A M1 F3 (MINOR): during a zoom `card_width` is interpolated each frame, so a wrapping title can change line count mid-zoom and its row height jumps a line; measure naturals at the zoom's end width, or pin the jump in a test. Not in M1's tasks or FRs.


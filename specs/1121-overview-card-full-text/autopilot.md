# Autopilot ledger — #1121 overview-card-full-text

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1121 "Overview cards: full text without truncation": Blocked by: #1120. Title wraps to 3 lines and activity to 2. Hover or keyboard focus expands the card in place, over its neighbours, with the full title, full branch, worktree dir and full activity. Acceptance: expands on focus without a pointer; Escape or blur collapses; no clipped text at 280 px; ripple, context menu and keyboard navigation unchanged. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1121
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-20
- **Phase**: plan
- **Next step**: plan unit: review A plan round 2 (sonnet, fix diff of round 1), then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

Base: origin/main at 4a36fba8 (feature 1120 fully merged).

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Escape on an expanded card (Story 3 sc.2, FR-005) | First Escape only collapses it (focus kept, Overview and full-screen stay); Escape with no card expanded closes the Overview | orchestrator default (recommended), pending jaro's confirmation on the decision card | spec.md#Clarifications |
| D2 | clarify | Worktree directory form (FR-014) | Full absolute path, home folder shown as `~` | orchestrator default (recommended), pending jaro's confirmation on the decision card | spec.md#Clarifications |
| D3 | clarify | Escape with a card menu open; which collapse re-expands the focused card | Escape only closes the menu; only the pointer leaving a hover-expanded card re-expands the focused card | agent | specs/972-overview-card-context-menu/spec.md#FR-007; spec.md#Edge Cases |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | fcb294c971d5b41839f3951557abff60660340ac:4a36fba89f36082ced2076392fc14ec5e2ba3aee | CHANGES: 2 MAJOR, 3 MINOR (F1–F5 all fixed) |
| Review A spec | 2 | fa37323a95555838f59f8feff82ea0ed01600972:550978508146ad5978bd9ee1a9688b19118d5ca0 | CLEAN (2 MINOR, both fixed) |
| Review A plan | 1 | 87d13a0a2d57de09847732a4e464bd243ec8a49f:efe4e646959dbdee73bf65a2b7f4d65fa0d75299 | CHANGES: 4 MAJOR, 3 MINOR (F1–F7 all fixed: F1 Escape flags by card not cause + K1 test; F2 `on_shown_change`, keyboard focus only; F3 zoom clause dropped, no zoom event; F4 reset in `settle_card_menu` `!open`; F5 twin in own slot, skipped; F6 `ShowTimer::reset` on topmost change; F7 alternatives R8/R10, `directories::UserDirs`) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done


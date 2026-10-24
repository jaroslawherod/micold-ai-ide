# Autopilot ledger — #1124 bottom-status-bar

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1124 — "Layout: bottom status bar for connection, sandbox, CLI and plan usage" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1124
- **Worktree branch**: claude/project-thread-6jo3w5
- **Started**: 2026-10-24
- **Phase**: M1 pr
- **Next step**: orchestrator opens the M1 PR from scratchpad pr-title.md / pr-body.md; after merge, milestone M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1317 | design (spec, plan, tasks) | merged (rebase) | f55628dfd03663553ffeef46dc83c99bc6b56fed |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Status bar on every body branch with connection and sandbox items; StatusBar in the showcase | (orchestrator opens) | done: rebased on 4910aeee, gate green at 2045d640, A CLEAN (r2 scoped CLEAN), B CLEAN r2, visual pass 5/5 |
| M2 | T016–T020 | light | Bring-up shown in the bar; no row above the body | — | planned |
| M3 | T021–T025 | light | Lost connection in the bar only; 7 action banners kept | — | planned |
| M4 | T026–T032 | light | CLI item follows the focused pane; Overview/Jump key hints | — | planned |
| M5 | T033–T038 | light | Plan usage moves from the app bar into the status bar | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Where is plan usage shown today? | App bar trailing actions (`ui/toolbar.rs` usage_indicator, feature 490); mockup board draws it in the title bar, canvas note puts it in the status bar — left as a clarify marker | agent-resolved | crates/micold-client/src/ui/toolbar.rs:95; mockup project/Main.dc.html, canvas.json note s1 |
| D2 | spec | CLI item when the session view is not the visible body | Omitted (FR-007, after spec review F2) | agent-resolved | spec.md FR-007 |
| D3 | clarify | Does a lost connection (no action) still raise a banner? | No: bar only (error dot, reconnecting text); conditions with an action keep their banner (FR-003, FR-009, US3 s5) | issue text | issue #1124 body "Failures that need action still raise a banner…"; ui/mod.rs connection_banner (Disconnected has no action) |
| D5 | tasks | Milestone cut | One milestone per story (M1 US1+setup+foundational, M2 US2, M3 US3, M4 US4, M5 US5); the full connection/sandbox tables land in M1 because the model's match must be exhaustive; T039 (visual pass) left to close | agent-resolved | tasks.md §Milestones; milestones.md rules 1, 2, 4 |
| D4 | clarify | Plan usage in status bar or title bar? | Status bar, after the CLI item; removed from the app bar; #1127 builds on that (FR-011, US5) | issue text | issue #1124 title; canvas note s1 |
| D6 | implement (M1, after rebase on 4910aeee) | 941's crowded-composer gate failed (transcript 135 < (800−28)/4−40 = 153): change the composer's caps or the gate? | The gate. The composer's caps are fixed heights by design (FR-008: MAX_HEIGHT 220, QUEUE_MAX_HEIGHT 120), so every fixed chrome row — #943 M9's Open in terminal in the session-bar row (~18 px, leaving main 3 px of slack) and this bar's 28 dp — comes out of the transcript one for one; the absolute quarter-of-the-pane floor broke each time chrome was added (#943 M8 loosened it once already). The gate's purpose (941 T055 mutant: queue cap removed) is now asserted directly: the crowded composer takes at most QUEUE_MAX_HEIGHT + 2·SM + MAX_HEIGHT (356) from the same pane's idle transcript (measured 340); the mutant still fails (475 → 0). Making the caps follow the window is a #941 design change, left as a follow-up | agent-resolved | crates/micold-client/tests/gates/composer_fits.rs; specs/941-structured-prompt-input/tdd/cycle-log.md:182–190; review A r2 CLEAN |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | dc7a76326c7ffef720f23d0c9ebe55080ecfac9a:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | dfed0a366bf36453ce50a47b075d694ef9d0d313:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (1 MINOR: US4 scenario order — fixed, prose only) |
| Plan | 1 | 2e9c5ba755a6058d8286f9f7db0235b26e5fb28b:fe7a79742614d045774446cb51b847fe9564bb1e | CHANGES: 1 MAJOR (anatomy figure shape/ALL/tests), 1 MINOR (R6 location key) — both fixed |
| Plan | 2 | a336c6f1ebfd5e1598c453a4535ea5aeff6790b2:fe7a79742614d045774446cb51b847fe9564bb1e | CLEAN (2 MINOR: R11 typo, R6 None fallback — fixed, prose only) |
| Tasks | 1 | c125de1a0438346af357aa10e590a942959da084:697e95407233a56669e0a960535abc7d181ec6f6 | CHANGES: 1 MAJOR (M4 Verify ran main_tests with --lib), 3 MINOR (SC-001 split, checklist note, macOS chord example) — all fixed |
| Tasks | 2 | c42ec5e995bdd8f5e3c6560f3f5651acdf8c85f1:697e95407233a56669e0a960535abc7d181ec6f6 | CLEAN |
| B M1 (conformance, sonnet) | 1 | 9bcb0c9d1fceee0a76897160ebb03fbe36ef3391:e4cc913076548077f3026bcb11c688c30388e4b1 | CHANGES: 1 MAJOR (F1 no red recorded for T003/T007/T009–T011 — fixed: red runs against a mutated build recorded in tdd/cycle-log.md), 1 MINOR (F2 banner-words refactor — noted in the log) |
| B M1 (conformance, sonnet) | 2 | bc81fc61c3a72f35e00d9e3c432be7e7f6da6680:8dec2fd824c32932a342e929595d8405e244a948 | CLEAN (1 MINOR: wording in the log — fixed, prose only) |
| A M1 (code-review high) | 1 | 1d17c59cc149d73c326b84b00dd8f37f9e20fae5:8a60fd0f91a8e9b92cd69538ff8e8903fe9ad44d | CLEAN (4 MINOR: F1 dot duplicated — fixed, shared `unread_mark::filled_circle`; F3 operate visits dropped items — fixed; F2, F4 declined) |
| A M1 (scoped, sonnet; crowded-composer gate after rebase) | 2 | 399b94d98548a63965b1986cd30a1e0a8048087c:13ea198a2ddf30a6139418aa2f48b8f14f081636 | CLEAN (1 MINOR: bound slack 56 px — fixed, the idle input row's 40dp subtracted) |

## Declined review findings

| Review | Finding | Reason |
|---|---|---|
| A M1 | F2 (MINOR) clock and zone looked up each view while `model` ignores them | `model`'s signature is fixed by the contract's test hook; M5's plan usage reads them; the lookup is two cheap calls |
| A M1 | F4 (MINOR) each item laid out twice, floor measured even when everything fits | a handful of items in a 28 dp bar; the pass stays one simple shape |

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Pre-existing (#941): the composer's caps are fixed, so a crowded composer (~520 px at its limits) overflows the pane at the 640×480 minimum window. Caps that follow the window's height would fix it; out of 1124's scope (D6).
- M1 visual §B7: at 640 px the M1 bar's texts fit whole, so the client never showed "…"; the shortening is covered by the showcase's too-narrow instance and `status_bar_fits`. Re-check at 640 px once M4/M5 add the CLI item, hints and usage.

Notes: speckit-analyze (sonnet) 0 CRITICAL/HIGH; its MEDIUM/LOW findings fixed in tasks.md, plan.md (state count), quickstart.md, spec.md (US3 wording). Optional after_tasks hooks (tdd.plan, docguard.score) skipped. Checklist requirements.md: last item ticked (markers resolved in clarify).

# Autopilot ledger — #1259 markdown-italic-face

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: "start work on all tickets that are not yet in progress use speckit autopilot. Check if the other sessions don't work on them yet" — this run: issue #1259 "Markdown italic renders upright: no italic font face bundled"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1259
- **Worktree branch**: claude/project-thread-wxdcto
- **Started**: 2026-10-24
- **Phase**: design PR open
- **Next step**: orchestrator merges the design PR on green CI, then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | emphasis draws in the bundled Roboto italic at the surrounding weight; bold italic as heavy as bold; code in emphasis upright; showcase shows both | — | planned |
| M2 | T019–T022 | light | `pin_roboto_italics`: a host Roboto italic or off-weight upright never replaces the bundled faces | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Bundle italic faces or drop the italic mapping (issue's two options)? | Bundle italic faces; keep the mapping (FR-010) | agent-resolved | spec.md#Context; orchestrator prompt prefers option 1 |
| D2 | spec | Which italic weights? Issue suggests Italic + Medium Italic | 400, 500 and 600: one italic twin per shipped upright weight, so bold (resolves to 600) + italic keeps its weight (FR-002/FR-003) | agent-resolved | link_text.rs `painted` sets `Weight::Bold`; assets/fonts ships 400/500/600 only |
| D3 | spec | Upstream reachable from the sandbox? | Yes: raw.githubusercontent.com google/fonts `ofl/roboto/Roboto-Italic[wdth,wght].ttf` returns 200; `uv` present for fonttools | agent-resolved | curl -I 2026-10-24 |
| D4 | clarify | FR-009 sample: extend shared core fixture or showcase-only sample? | Left to the plan; spec requires only a bold-italic run | agent-resolved | spec review F2 |
| D5 | plan | FR-009 sample location | Shared `markdown-subset.md` gains `***bold italic***`; core test asserts bold+italic style | agent-resolved | research R5; showcase draws MARKDOWN_SUBSET |
| D6 | plan | How to make emphasis beat a host Roboto Italic (FR-006) | `install_roboto` loads six faces, `pin_roboto_italics` removes non-Binary non-normal Roboto faces only (upright host faces kept for FR-004) | agent-resolved | research R2; mono.rs precedent |
| D7 | plan | Code inside emphasis | `painted` sets no slant on code runs | agent-resolved | research R4; spec Edge: inline code |
| D8 | plan review 1 | F1: host Roboto Bold 700 makes upright bold heavier than bold-italic (600). Pin off-weight host faces (a) or narrow FR-003 (b)? | (a): the pin also removes non-Binary upright Roboto faces whose weight is not 400/500/600; FR-004 gains that one exception; C-2 gains weight-700-upright row and a `roboto.rs` case with a file-loaded Bold 700 | agent-resolved | every client upright query is 400/500/600 or 700 (`Weight::Bold`); fontdb picks nearest heavier for a missing 700, so a host Black 900 would also win; (b) leaves US2's defect on hosts with fonts-roboto |
| D9 | tasks | Milestone cut: US2 its own milestone (milestones rule 2)? | No: US1+US2 in M1, US3 (the pin) M2. FR-002 needs the 600 italic in M1 for headings/bold roles, which already makes bold italic 600; US2 alone would be one fixture line and one assertion, and shipping US1 without the 600 italic would put US2's lighter-bold-italic defect on main. M1's 18 tasks stay one milestone: apart from the binary fonts the diff is small, and any split leaves a deliverable without the italic faces | agent-resolved | spec US2 'Why this priority'; contract C-2 weight-700 rows |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 6721080f0a781ff07679652e804cc693d92581c9:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CLEAN, 2 MINOR (run by orchestrator): F1 FR-005/FR-008 implementation detail vs checklist, F2 FR-009 bold-italic vs shared fixture; both applied in clarify |
| Clarify | 1 | spec.md | 3 questions, all agent-resolved (record existing rules); no new requirement, unit ends |
| Plan review | 1 | e209d8637cc99c1c00d5b3b330033eabd3b06074:7bcd080cc6c6f537fd45f68b69b97148f3485dca | CHANGES (run by orchestrator): F1 MAJOR host Roboto Bold 700 breaks FR-003/SC-002 → option (a), D8; F2 MINOR R6/R7/R8 lacked rejected alternatives → added. Both applied |
| Plan review | 2 | b530e8ce4e792f8d5efd2595d56a740898e653ed:7dc4a597b0b322dedf551f73f0d16810cf578500 | CLEAN (orchestrator, sonnet); 1 MINOR: plan.md Test strategy roboto.rs row not updated for D8 → fixed, prose only |
| Tasks review | 1 | 73cdce2dbb41cb29369c478080c14350bb320212:d20f1b50e27026933493e4e4d962f14aadbb3a33 | CLEAN (orchestrator; also covered speckit-analyze: all FR/SC covered, paths verified); 3 MINOR applied: F1 C-3 pin joins in M2 + T007 host-Roboto note, F2 D9 M1-size note, F3 Verify adds `mise run gate` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Issue #1259 names `ui/transcript/rich.rs` for `MarkdownView`; the component lives in `crates/micold-client/src/ui/material/markdown_view.rs`, and the italic slant is set in `link_text.rs` (`painted`).

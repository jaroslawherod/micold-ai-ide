# Autopilot ledger — #1259 markdown-italic-face

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: "start work on all tickets that are not yet in progress use speckit autopilot. Check if the other sessions don't work on them yet" — this run: issue #1259 "Markdown italic renders upright: no italic font face bundled"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1259
- **Worktree branch**: claude/project-thread-wxdcto
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: orchestrator dispatches spec review round 1 (autopilot-reviewer, rubrics/spec.md) on spec.md, then clarify unit (spec has no open markers)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Bundle italic faces or drop the italic mapping (issue's two options)? | Bundle italic faces; keep the mapping (FR-010) | agent-resolved | spec.md#Context; orchestrator prompt prefers option 1 |
| D2 | spec | Which italic weights? Issue suggests Italic + Medium Italic | 400, 500 and 600: one italic twin per shipped upright weight, so bold (resolves to 600) + italic keeps its weight (FR-002/FR-003) | agent-resolved | link_text.rs `painted` sets `Weight::Bold`; assets/fonts ships 400/500/600 only |
| D3 | spec | Upstream reachable from the sandbox? | Yes: raw.githubusercontent.com google/fonts `ofl/roboto/Roboto-Italic[wdth,wght].ttf` returns 200; `uv` present for fonttools | agent-resolved | curl -I 2026-10-24 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 6721080f0a781ff07679652e804cc693d92581c9:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | not run: this unit has no Agent tool to dispatch a fresh reviewer; orchestrator to dispatch it (rubrics/spec.md) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Issue #1259 names `ui/transcript/rich.rs` for `MarkdownView`; the component lives in `crates/micold-client/src/ui/material/markdown_view.rs`, and the italic slant is set in `link_text.rs` (`painted`).

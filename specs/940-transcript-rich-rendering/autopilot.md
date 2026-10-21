# Autopilot ledger — #940 transcript-rich-rendering

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #940 "Structured session view: markdown, syntax-highlighted code and diffs for Edit/Write" (spec 940; depends on #939, whose design merged and whose code M1–M6 has not landed); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #940
- **Worktree branch**: claude/project-thread-x8n8q6
- **Started**: 2026-10-21
- **Phase**: tasks
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Plan (2026-10-21): planned against #939's planned names (plan.md, tasks.md, contracts/); #940 implementation starts only after #939 M6 merges, first re-checking those names (research R14). New core deps pulldown-cmark 0.13 and similar 2 (research R1, R6).
- Clarify round 1 (2026-10-21): FR-020 Edit/MultiEdit/Write cards start folded (#939 FR-010); FR-021 no tool result gets rich rendering (issue #940 scope; sub-agent output is #944). Both agent-resolved, no requirement added; unit ends CLEAN.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | feee0a64dd03cc16117948539e16660f7f842da3:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 754f7fe793b60b5c4e0cd62f61f515be16d06b54:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CLEAN |
| Review plan | 1 | cbeaa4e165ca7c3e9f2ea7b08be207eb01604534:9a46f37fbf00a575587b6d7999a054954d903020 | CLEAN: 3 MINOR (all fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Possible later issue: draw Task (sub-agent) results as markdown and highlight Read results (out of #940 scope, FR-021).

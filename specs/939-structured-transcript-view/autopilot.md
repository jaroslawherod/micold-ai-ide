# Autopilot ledger — #939 structured-transcript-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 939 — GitHub issue #939 "Structured session view: read-only transcript (assistant text, thinking, tool calls)"; coordinator scope note: design per-block rendering behind a small seam (one renderer per block kind) that #940 can swap; #940's rendering out of scope. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #939
- **Worktree branch**: claude/project-thread-61i2k8
- **Started**: 2026-10-21
- **Phase**: plan
- **Next step**: plan unit (handover): run plan review round 2 (sonnet, fix diff vs round-1 snapshot), then commit and set Next step to `tasks unit`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify (2026-10-21, 2 rounds, agent-resolved): sub-agent events (parent tool use set) are kept out of the main transcript and counted per parent tool-call card (FR-024; orphans on one notice); deltas not counted. #944 adds nesting on top. #944 treats the per-turn cost in #939's turn footer as #939's; #944 adds context meter, plan_usage feed and sub-agent nesting.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | be36b8772c7a0c2eb6acdfe0feb6de62a0ce7f56:9ad0504870e12886d671ea3f4124162315386f78 | CLEAN: 3 MINOR (F1 fixed, prose only) |
| Review plan | 1 | d0912186067579c21f5295b44c37fdcac714e0bd:dac0eb5478f3f39151b5deba2fcffc6b02a82619 | CHANGES: F1 MAJOR (no geometry-gate layer) fixed — geometry layer + DisclosureCard outline/ripple/contrast gates, header-fits and jump-control gates, layout_snapshot transcript state; F2 MINOR fixed (Shown from on_viewport_resize, R9); F3 MINOR fixed (Transcript::nested scope for #944, R7); F4 MINOR fixed (renderer-seam behavioural test) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

- Done: plan.md, research.md (R1–R15), data-model.md, quickstart.md, contracts/transcript-model.md,
  contracts/block-renderers.md written; plan review round 1 run (CHANGES, 1 MAJOR + 3 MINOR, all
  fixed in the working tree and committed with this handover).
- Next: plan review round 2 on `model: sonnet`, scoped to the fix diff
  (`scripts/autopilot/review-snapshot.sh diff d0912186067579c21f5295b44c37fdcac714e0bd:dac0eb5478f3f39151b5deba2fcffc6b02a82619`),
  with round 1's findings as listed in *Review rounds*. On CLEAN: set Next step to `tasks unit`,
  Handover back to `None.`, commit, return DONE (PR: none).
- Open findings: none unfixed. PR: none.

## Open escalation

None.

## Follow-ups not done


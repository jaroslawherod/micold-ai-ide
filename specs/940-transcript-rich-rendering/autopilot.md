# Autopilot ledger — #940 transcript-rich-rendering

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #940 "Structured session view: markdown, syntax-highlighted code and diffs for Edit/Write" (spec 940; depends on #939, whose design merged and whose code M1–M6 has not landed); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #940
- **Worktree branch**: claude/project-thread-x8n8q6
- **Started**: 2026-10-21
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR; after it merges and #939 M6 has merged, milestone M1 (T001 re-checks #939 names)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| pending | design (spec, clarify, plan, tasks, milestones) | to open (orchestrator, via MCP) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | markdown model in core: settled blocks final, incremental, total (after #939 M6; T001 re-checks #939 names) | — | pending |
| M2 | T013–T015 | full | edit diff in core: recorded edits round-trip, malformed → None | — | pending |
| M3 | T016–T024 | full | window's render-free rich cache: derive/prune, highlight jobs, checked copy, web/mail link opens | — | pending |
| M4 | T025–T034 | full | shared MarkdownView, CodeBlock, InlineDiff, LinkText, DisclosureCard::meta with gates and showcase | — | pending |
| M5 | T035–T042 | full | transcript draws with BlockRenderers::RICH; budget; user guide; Part B | — | pending |

## Decisions

- Plan (2026-10-21): planned against #939's planned names (plan.md, tasks.md, contracts/); #940 implementation starts only after #939 M6 merges, first re-checking those names (research R14). New core deps pulldown-cmark 0.13 and similar 2 (research R1, R6).
- Tasks (2026-10-21): 42 tasks, five milestones as the plan's Delivery (layered like #939's: core, cache, components, renderers last; no half-wired UI). M1 has 12 tasks (setup + re-check + fixtures + model; no split along a scenario leaves a deliverable). speckit-analyze (inline): M-5 moved to markdown_subset as plan names it; `refresh` takes an injected `Grammar { knows, token }` so the render-free cache does not depend on `ui::syntax` (data-model.md updated).
- Clarify round 1 (2026-10-21): FR-020 Edit/MultiEdit/Write cards start folded (#939 FR-010); FR-021 no tool result gets rich rendering (issue #940 scope; sub-agent output is #944). Both agent-resolved, no requirement added; unit ends CLEAN.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | feee0a64dd03cc16117948539e16660f7f842da3:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 754f7fe793b60b5c4e0cd62f61f515be16d06b54:cee4e71942a66b77ba2d7107e57fd6e433d38b7e | CLEAN |
| Review plan | 1 | cbeaa4e165ca7c3e9f2ea7b08be207eb01604534:9a46f37fbf00a575587b6d7999a054954d903020 | CLEAN: 3 MINOR (all fixed, prose only) |
| Review tasks | 1 | df307d2ead90c7a22675e927c76fda956075f993:48dae9c37b7f429569d6b850335973fabda4a734 | CHANGES: 2 MAJOR (MarkdownView copy/large plumbing; CopyExpired timer request unnamed → `Outcome::CopyConfirmTimer`), 1 MINOR (`fills` arity) — all fixed; data-model, research R9, contracts/rich-renderers.md updated |
| Review tasks | 2 | 09be2f38d7faea105eb95db889edb95def0aba7a:48dae9c37b7f429569d6b850335973fabda4a734 | CLEAN: 1 MINOR (double blank line, fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Possible later issue: draw Task (sub-agent) results as markdown and highlight Read results (out of #940 scope, FR-021).

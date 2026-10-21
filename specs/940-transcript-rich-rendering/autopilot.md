# Autopilot ledger — #940 transcript-rich-rendering

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #940 "Structured session view: markdown, syntax-highlighted code and diffs for Edit/Write" (spec 940; depends on #939, fully merged through M6, PR #1206, b6c1dd92); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #940
- **Worktree branch**: claude/project-thread-x8n8q6
- **Started**: 2026-10-21
- **Phase**: milestone M1
- **Next step**: M1 reviews (A high, B conformance), full gate, then the orchestrator opens the M1 PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1187 | design (spec, clarify, plan, tasks, milestones) | merged | a7df5dd37517543447b150ff02d6a5f195c7c392 |
| pending | M1 markdown model | to open (orchestrator, via MCP) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | markdown model in core: settled blocks final, incremental, total (after #939 M6; T001 re-checks #939 names) | pending | implemented, in review |
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
| Review A M1 (code-review high) | 1 | c52ec378bfd53e2b9e289fa241e1faced7b8f684:4009ce61770beaba9d569a6c263284283ca295fb | 9 findings: nested definitions cut/hid (fixed), image in link lost its link (fixed), whole-text hash per delta (fixed: kept source text, memcmp), container-closed fence stayed open (fixed), empty task item paragraph (fixed), `lines` off on trailing blanks (fixed), duplicated inline prologue (fixed), eager `code_blocks` (declined), lone trailing `\r` (declined) |
| Review B M1 (conformance, sonnet) | 1 | c52ec378bfd53e2b9e289fa241e1faced7b8f684:4009ce61770beaba9d569a6c263284283ca295fb | CHANGES: 2 MAJOR (cycle log untracked → committed; fence red not shown → shown against a stub, logged), 1 MINOR (batched cycles; noted). Verify: subset 18, stream 4, lib 10 passed |

## Declined review findings

- Review A M1 #8 (`code_blocks` builds a Vec, `code` scans): MINOR; ids are few per reply and the renderer reads them from M3's cache, not per frame.
- Review A M1 #9 (lone trailing `\r` read as a complete line ending): a lone `\r` is a CommonMark line ending; the `\n` that may follow only completes the same CRLF ending, so the line before it is complete either way.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Possible later issue: draw Task (sub-agent) results as markdown and highlight Read results (out of #940 scope, FR-021).

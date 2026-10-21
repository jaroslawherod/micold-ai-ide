# Autopilot ledger — #939 structured-transcript-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 939 — GitHub issue #939 "Structured session view: read-only transcript (assistant text, thinking, tool calls)"; coordinator scope note: design per-block rendering behind a small seam (one renderer per block kind) that #940 can swap; #940's rendering out of scope. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #939
- **Worktree branch**: claude/project-thread-61i2k8
- **Started**: 2026-10-21
- **Phase**: milestone M1
- **Next step**: M1 review A + scoped gate, review B, full gate, push

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1183 | Design: spec, clarify, plan, research, contracts, tasks | merged (rebase) | cee4e71942a66b77ba2d7107e57fd6e433d38b7e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Transcript model for finished events (entries, cards, pairing, footers, raw rows, notices, summaries, figures, sub-agent counts) | — | in progress |
| M2 | T016–T019 | full | Streaming, supersede, bound and re-sync; G1–G4 on every fixture | — | pending |
| M3 | T020–T023 | full | Window's per-session transcript state (fold, follow, TranscriptMsg, scroll tasks) beside the plain log | — | pending |
| M4 | T024–T030 | full | Shared DisclosureCard, AccordionSection slots, Text::monospace, icons, showcase | — | pending |
| M5 | T031–T037 | full | ui::transcript::view behind the renderer seam, lazy entries, SC-004 budget (showcase only) | — | pending |
| M6 | T038–T043 | full | Pane shows the transcript (single, tiled); plain log removed; user guide; Part B | — | pending |

## Decisions

- speckit-analyze (2026-10-21): 0 CRITICAL/HIGH; 1 MEDIUM + 6 LOW fixed in tasks.md (T032 green at T037, T017/T019 → US3, T039 file named, FR-001/FR-004 cited, T002 dependency noted).
- Tasks (2026-10-21): milestones follow the plan's reviewed layered delivery (core → state → components → view → pane) rather than one milestone per story, because the UI of US1/US2 cannot finish before M6 (milestones.md rule 6); M1 carries both P1 stories at the model (15 tasks, kept whole: splitting it splits one build.rs rule table across PRs with no new observable).
- Clarify (2026-10-21, 2 rounds, agent-resolved): sub-agent events (parent tool use set) are kept out of the main transcript and counted per parent tool-call card (FR-024; orphans on one notice); deltas not counted. #944 adds nesting on top. #944 treats the per-turn cost in #939's turn footer as #939's; #944 adds context meter, plan_usage feed and sub-agent nesting.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | be36b8772c7a0c2eb6acdfe0feb6de62a0ce7f56:9ad0504870e12886d671ea3f4124162315386f78 | CLEAN: 3 MINOR (F1 fixed, prose only) |
| Review plan | 1 | d0912186067579c21f5295b44c37fdcac714e0bd:dac0eb5478f3f39151b5deba2fcffc6b02a82619 | CHANGES: F1 MAJOR (no geometry-gate layer) fixed — geometry layer + DisclosureCard outline/ripple/contrast gates, header-fits and jump-control gates, layout_snapshot transcript state; F2 MINOR fixed (Shown from on_viewport_resize, R9); F3 MINOR fixed (Transcript::nested scope for #944, R7); F4 MINOR fixed (renderer-seam behavioural test) |
| Review plan | 2 | 8156502f4d59c5450a15b614c9c4f442756c771e:dad297fe3e0f1334887ccfe7eda90b93e46ac3c6 | CLEAN: 1 MINOR (F1 fixed, prose only) |
| Review tasks | 1 | cdfaee5481a594e3449ee2b0253893520d5cc0ad:76cdeb51e56a37bb1c3ee20d2dcd18ea98d4e3d8 | CHANGES: F1 MAJOR (stale checklist item: no NEEDS CLARIFICATION marker remains) fixed, prose only — ticked, note rewritten; F2 MINOR fixed (T007 [P] dropped) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done


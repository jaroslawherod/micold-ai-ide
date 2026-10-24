# Autopilot ledger — #1276 review-ui-docs-sync

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1276 "Review UI: docs, layout snapshot, visual pass and mockup sync" (body and owner comment quoted in spec.md Input); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1276
- **Worktree branch**: claude/project-thread-kxels1
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan (clarify done: round 1 agent-resolved 3 markers, round 2 found no critical ambiguity)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-004: is the guide-equals-bindings check the micold-core logic + unit tests clause? | Yes: micold-core test compares a marked shortcut table in reviewing-changes.md with the bound review key table; page gets -micold-docs | agent | crates/micold-core/tests/terminal_only_doc.rs; .gitattributes:125 |
| 2 | clarify | FR-009: visual pass widths | 1280×800 only, both themes; other widths via FR-007 gates | agent | review-redesign.md#Tasks; issue acceptance |
| 3 | clarify | FR-012: unbuilt mockup elements removed or labelled? | Doc-listed follow-ups (split option, FR-013 list) stay labelled "follow-up, not built"; other unbuilt elements removed | agent | review-redesign.md#0 ("Split option … Follow-up") |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd6df384829f4a9cf135e66858ff627637928a05:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 3 MAJOR, 2 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e130e157ec45aa2c55f68f1cd624e18698a09441:7783f026de44f8644464cd7933653c0bed11d5c1 | CLEAN, 2 MINOR (sonnet): FR-005 groups reworded to match US2 sc.1 — fixed, prose only; FR-009 check list wrapped — fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Environment: `gh` is not authenticated and `mise` is missing in this cloud container; GitHub reads
  went through the GitHub MCP tools. Blockers #1269–#1275 and #1128 are unmerged at spec time; the
  spec describes their deliverables from their issue bodies and `docs/development/review-redesign.md`.

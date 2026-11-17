# Autopilot ledger — #1732 client-ws-api

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1732 "JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume" (part of #1722; FR-017R, FR-030). Owner goal for this run: a WORKING web UI against the real orchestrator instead of the dev-only mock — replace the web client's mocked OrchestratorApi (specs/1735-rn-client-scaffold, `mise run web`) with a real one over /api/ws, i18n en+pl for every new string, and a `mise run` task starting orchestrator + >=1 daemon + web client locally. No desktop/iced UI work. Aim 3-4 milestones. Cloud thread: no mise/gh; scripts/gate.sh and GitHub MCP; merge main in, never force-push. labels: flow:feature, effort:high (in-progress, claimed)
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1732
- **Worktree branch**: claude/project-thread-mezck0
- **Started**: 2026-11-17
- **Phase**: plan
- **Next step**: plan unit. Clarify round 1: no critical ambiguities (retention window, backoff timings, ports, FR-012 sidebar approach deferred to plan).

Blockers: none. #1726 merged (#2007, #2012, #2016 at c316a62be); #1948 merged (#2031 at 15a603c3c). Base: origin/main ff1592a91.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

| spec review | 1 | spec.md | ff1592a91dce73ab8455582abb7ab45d7e31ad09 | ab6207b8a448c2329fff6fbbbcd821854aa4f518:ff1592a91dce73ab8455582abb7ab45d7e31ad09 | CHANGES: 2 MAJOR (F1 browser parity, F2 app-level heartbeat), 3 MINOR (F3 unreachable/snapshot, F4 FR-001 contract, F5 addresses/US3 check) — all fixed in spec.md; behaviour-bearing, round 2 re-review (sonnet) due |
| spec review | 2 | spec.md | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | CLEAN (sonnet re-review); nit Key Entities ping->pong fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

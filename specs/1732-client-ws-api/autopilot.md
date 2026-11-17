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
- **Next step**: plan review round 2 (sonnet, fix diff) then tasks unit. (superseded: round 1, opus, rubrics/plan.md) NOT RUN: the plan unit had no Agent tool; the orchestrator dispatches it with snapshot 1ed86d9ea:a2b09676c then, if CLEAN, the tasks unit.

Blockers: none. #1726 merged (#2007, #2012, #2016 at c316a62be); #1948 merged (#2031 at 15a603c3c). Base: origin/main ff1592a91.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Plan (R1-R12 in research.md): tokio-tungstenite (in lock via kube) with upgrade-callback origin check; new `client_api` types, not ClientMsg; schemars + json-schema-to-typescript, `scripts/api-types.sh [--check]`; resume = ring of 1024 changes + epoch, else resync_needed; backoff = existing ConnectionManager (500 ms base, 15 s cap, full jitter, offline after 8); heartbeat 20 s / dead at 40 s; change feed = one 1 s poller diffing `gather`; FR-012 = orchestrator sends computed order, parity via shared vectors; ports API 127.0.0.1:8730, web 8731; local stack = scripts/local-stack.sh on a seeded catalog.
- Milestones (4): M1 daemon ReadStructuredEvents (wire change) then orchestrator /api/ws; M2 schema/TS generation + drift gate + vectors; M3 web client on real API + i18n + minimal local-stack; M4 3-browser Playwright, CI, docs.
- Content read uses a new read-only daemon request, not Attach (ProjectBusy); wire change with version bump.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

| spec review | 1 | spec.md | ff1592a91dce73ab8455582abb7ab45d7e31ad09 | ab6207b8a448c2329fff6fbbbcd821854aa4f518:ff1592a91dce73ab8455582abb7ab45d7e31ad09 | CHANGES: 2 MAJOR (F1 browser parity, F2 app-level heartbeat), 3 MINOR (F3 unreachable/snapshot, F4 FR-001 contract, F5 addresses/US3 check) — all fixed in spec.md; behaviour-bearing, round 2 re-review (sonnet) due |
| spec review | 2 | spec.md | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | CLEAN (sonnet re-review); nit Key Entities ping->pong fixed |

| plan review | 1 | plan.md, research.md, data-model.md, contracts/, quickstart.md | a2b09676c | 1ed86d9eaafae9191caee6e33c48dc05f136a0a6:a2b09676c384b84c5601274c6aaafd1a91208141 | NOT RUN by the plan unit |
| plan review | 1 (opus, run by the orchestrator) | same | see commit | - | CHANGES: 2 HIGH, 3 MED, 3 LOW; all fixed (read_session via new daemon ReadStructuredEvents, early M1 test; poller restart and boundaries; origin port/null; FR-012 vectors cover grouping; i18n key groups; local-stack moved into M3; subscribed/offline notes); behaviour-bearing, round 2 re-review (sonnet) due |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

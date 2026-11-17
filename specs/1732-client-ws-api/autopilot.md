# Autopilot ledger — #1732 client-ws-api

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1732 "JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume" (part of #1722; FR-017R, FR-030). Owner goal for this run: a WORKING web UI against the real orchestrator instead of the dev-only mock — replace the web client's mocked OrchestratorApi (specs/1735-rn-client-scaffold, `mise run web`) with a real one over /api/ws, i18n en+pl for every new string, and a `mise run` task starting orchestrator + >=1 daemon + web client locally. No desktop/iced UI work. Aim 3-4 milestones. Cloud thread: no mise/gh; scripts/gate.sh and GitHub MCP; merge main in, never force-push. labels: flow:feature, effort:high (in-progress, claimed)
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1732
- **Worktree branch**: claude/project-thread-fcckol
- **Started**: 2026-11-17
- **Phase**: milestones (M2 in review)
- **Next step**: M2 code done (T001-T002, T005-T019), committed and pushed on claude/project-thread-fcckol; reviews not run (no Agent tool): the orchestrator reviews `origin/main..HEAD`, opens the PR from specs/1732-client-ws-api/m2-pr-body.md, then merges and starts M3.

Blockers: none. #1726 merged (#2007, #2012, #2016 at c316a62be); #1948 merged (#2031 at 15a603c3c). Base: origin/main ff1592a91.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #2032 | Design PR (spec, plan, tasks) | merged | fbfa8360e |
| #2033 | M1 daemon ReadStructuredEvents | merged | 10f82e095 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T003–T004 | full | daemon ReadStructuredEvents (wire change) | #2033 | merged |
| M2 | T001–T002, T005–T019 | full | /api/ws listener: list, read_session, subscribe+resume, heartbeat, origin refusal | - | in review (REVIEWS PENDING) |
| M3 | T020–T026 | full | generated schema+TS, drift gate, FR-012 vectors | - | pending |
| M4 | T027–T037 | full | web client on the real API, i18n, minimal local-stack | - | pending |
| M5 | T038–T042 | full | 3-browser Playwright, CI, docs, polish | - | pending |

M1 split off the daemon wire change (review F5), so M2 has 17 tasks, kept whole: one wire surface with no observable split.

## Decisions

- Plan (R1-R12 in research.md): tokio-tungstenite (in lock via kube) with upgrade-callback origin check; new `client_api` types, not ClientMsg; schemars + json-schema-to-typescript, `scripts/api-types.sh [--check]`; resume = ring of 1024 changes + epoch, else resync_needed; backoff = existing ConnectionManager (500 ms base, 15 s cap, full jitter, offline after 8); heartbeat 20 s / dead at 40 s; change feed = one 1 s poller diffing `gather`; FR-012 = orchestrator sends computed order, parity via shared vectors; ports API 127.0.0.1:8730, web 8731; local stack = scripts/local-stack.sh on a seeded catalog.
- Milestones (5): M1 daemon ReadStructuredEvents (wire change); M2 orchestrator /api/ws; M3 schema/TS generation + drift gate + vectors; M4 web client on real API + i18n + minimal local-stack; M5 3-browser Playwright, CI, docs.
- Content read uses a new read-only daemon request, not Attach (ProjectBusy); wire change with version bump.
- M2 (base 10f82e095, PR #2033 merged): `LinkDirectory` has `list() -> Snapshot` (one gather of projects, which carry worktrees and sessions) instead of `list(kind)`, plus `changed()` for the poke; ids on the wire: project `<daemon>:<identity>`, location `<project>#<worktree dir or empty>`, session uuid. Epoch is 53-bit so a JS number holds it. Dead-client release is tested with a 300 ms idle option rather than paused time (real sockets and paused time auto-advance badly); the 40 s constant has its own assertion. Real linked daemon helper moved to tests/support/linked_daemon.rs, shared by both orchestrator suites.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

| diff review | 1 (opus, orchestrator) | M1 fbfa8360e..684310c01 | see commit | - | CHANGES, minor: F1 test name, F2 reply on read-task failure, F3 empty-history comment fixed; F4 declined (see Declined) |
| spec review | 1 | spec.md | ff1592a91dce73ab8455582abb7ab45d7e31ad09 | ab6207b8a448c2329fff6fbbbcd821854aa4f518:ff1592a91dce73ab8455582abb7ab45d7e31ad09 | CHANGES: 2 MAJOR (F1 browser parity, F2 app-level heartbeat), 3 MINOR (F3 unreachable/snapshot, F4 FR-001 contract, F5 addresses/US3 check) — all fixed in spec.md; behaviour-bearing, round 2 re-review (sonnet) due |
| spec review | 2 | spec.md | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | 5bd79793e23caf69bef8e78804b1ad82fe5c90e4 | CLEAN (sonnet re-review); nit Key Entities ping->pong fixed |

| plan review | 1 | plan.md, research.md, data-model.md, contracts/, quickstart.md | a2b09676c | 1ed86d9eaafae9191caee6e33c48dc05f136a0a6:a2b09676c384b84c5601274c6aaafd1a91208141 | NOT RUN by the plan unit |
| plan review | 2 (sonnet, fix diff) | same | a4609c73c | - | CLEAN (recorded by the tasks unit from the orchestrator's report) |
| tasks review | 2 (sonnet) | fix diff | see commit | - | CHANGES: 2 text-only (stale M-numbers after renumbering, one filename); fixed, prose only, no round 3 |
| tasks review | 1 (opus, orchestrator) | tasks.md | see commit | - | CHANGES: 5 MAJOR (T025 dep, shared main.rs, SC-006 client test, `mise run web` missing -> new client-web-mock task, M1 split into M1+M2, renumbered to 5), 3 MINOR; all fixed; behaviour-bearing, round 2 due |
| plan review | 1 (opus, run by the orchestrator) | same | see commit | - | CHANGES: 2 HIGH, 3 MED, 3 LOW; all fixed (read_session via new daemon ReadStructuredEvents, early M1 test; poller restart and boundaries; origin port/null; FR-012 vectors cover grouping; i18n key groups; local-stack moved into M3; subscribed/offline notes); behaviour-bearing, round 2 re-review (sonnet) due |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | diff review round 1 (opus) | F4 double lock in the ReadStructuredEvents path | harmless: the second lock is uncontended and held briefly |

## Follow-ups

- CI llvm-cov merge intermittently hits a corrupt `_1.profraw` (signature 17540017307748215547); M1 added a ci.yml step dropping unreadable profiles; root cause unknown, to investigate.

## Handover

None.

## Open escalation

None.

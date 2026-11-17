# Autopilot ledger — #1732 client-ws-api

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1732 "JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume" (part of #1722; FR-017R, FR-030). Owner goal for this run: a WORKING web UI against the real orchestrator instead of the dev-only mock — replace the web client's mocked OrchestratorApi (specs/1735-rn-client-scaffold, `mise run web`) with a real one over /api/ws, i18n en+pl for every new string, and a `mise run` task starting orchestrator + >=1 daemon + web client locally. No desktop/iced UI work. Aim 3-4 milestones. Cloud thread: no mise/gh; scripts/gate.sh and GitHub MCP; merge main in, never force-push. labels: flow:feature, effort:high (in-progress, claimed)
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1732
- **Worktree branch**: claude/project-thread-7iamtv
- **Started**: 2026-11-17
- **Phase**: milestones (M4 in review)
- **Next step**: M4 diff review (A, B) by the orchestrator, then PR; then M5.

Blockers: none. #1726 merged (#2007, #2012, #2016 at c316a62be); #1948 merged (#2031 at 15a603c3c). Base: origin/main ff1592a91.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #2032 | Design PR (spec, plan, tasks) | merged | fbfa8360e |
| #2033 | M1 daemon ReadStructuredEvents | merged | 10f82e095 |
| #2034 | M2 /api/ws listener | merged | a1c9514b3 |
| #2037 | M3 schema + TS types | merged | a7d7a3c12 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T003–T004 | full | daemon ReadStructuredEvents (wire change) | #2033 | merged |
| M2 | T001–T002, T005–T019 | full | /api/ws listener: list, read_session, subscribe+resume, heartbeat, origin refusal | #2034 | merged |
| M3 | T020–T026 | full | generated schema+TS, drift gate, FR-012 vectors | #2037 | merged |
| M4 | T027–T037 | full | web client on the real API, i18n, minimal local-stack | - | implemented, in review |
| M5 | (new, tasks to write) | full | local kind stack: `mise run local-stack` starts a kind cluster and runs the full setup (orchestrator, daemons, Postgres, web) inside it; real-cluster deploy later (jaro 2026-11-17). Reuse #1981 kind+Helm e2e; no design exists yet (kind-run design thread paused) |
| M6 | T038–T042 | full | 3-browser Playwright, CI, docs, polish | - | pending |

M1 split off the daemon wire change (review F5), so M2 has 17 tasks, kept whole: one wire surface with no observable split.

## Decisions

- Plan (R1-R12 in research.md): tokio-tungstenite (in lock via kube) with upgrade-callback origin check; new `client_api` types, not ClientMsg; schemars + json-schema-to-typescript, `scripts/api-types.sh [--check]`; resume = ring of 1024 changes + epoch, else resync_needed; backoff = existing ConnectionManager (500 ms base, 15 s cap, full jitter, offline after 8); heartbeat 20 s / dead at 40 s; change feed = one 1 s poller diffing `gather`; FR-012 = orchestrator sends computed order, parity via shared vectors; ports API 127.0.0.1:8730, web 8731; local stack = scripts/local-stack.sh on a seeded catalog.
- Milestones (5): M1 daemon ReadStructuredEvents (wire change); M2 orchestrator /api/ws; M3 schema/TS generation + drift gate + vectors; M4 web client on real API + i18n + minimal local-stack; M5 3-browser Playwright, CI, docs.
- Content read uses a new read-only daemon request, not Attach (ProjectBusy); wire change with version bump.
- M2 (base 10f82e095, PR #2033 merged): `LinkDirectory` has `list() -> Snapshot` (one gather of projects, which carry worktrees and sessions) instead of `list(kind)`, plus `changed()` for the poke; ids on the wire: project `<daemon>:<identity>`, location `<project>#<worktree dir or empty>`, session uuid. Epoch is 53-bit so a JS number holds it. Dead-client release is tested with a 300 ms idle option rather than paused time (real sockets and paused time auto-advance badly); the 40 s constant has its own assertion. Real linked daemon helper moved to tests/support/linked_daemon.rs, shared by both orchestrator suites. The heartbeat test module is `ws_liveness.rs`, not `ws_heartbeat.rs`: `netns.rs` selects `heartbeat::` tests by substring. Local gate: scoped (fmt, workspace clippy, nextest core+daemon green bar env-only failures); full gate left to CI (container disk).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- M3 (base a1c9514b3, PR #2034 merged): schema root = anyOf(ClientRequest, ServerMessage), draft-07 `definitions`, feed types named too. json-schema-to-typescript 16.0.0 (latest). Wire stays snake_case; `packages/protocol/src/wire.ts` maps it onto the client's camelCase types (client `Session` gained optional `reachable`). Drift: Rust tests `client_api_schema`/`client_api_vectors` check schema and vectors in every cargo test; `scripts/api-types.sh --check` in gate.sh (when json2ts installed) and `--check --ts-only` + `scripts/tests/api-types.test.sh` in CI's web client job (the Rust lint job has no node, so the shell test skips there).

## Review rounds

| diff review | 1 (opus, orchestrator) | M4 1c83be7~1..1c83be7 | 1c83be7 | - | CHANGES: 1 MAJOR (F1 sidebar stopped applying changes after a resync: listener `since` never reset) fixed in handleSubscribed; MINOR F2 connected only on `subscribed` (attempt reset there too), F3 visible while reconnecting keeps the attempt and only shortens the timer, F4 retry on a stopped client returns early, F5 local-stack re-checks ports before starting and names the exited process, all fixed with tests; behaviour-bearing, round 2 re-review (sonnet) due |
| diff review | 1 (opus, orchestrator) | M3 a1c9514b3..43fc62389 | see commit | - | CHANGES, 4 minor, all fixed: F1 --ts-only missing schema now DRIFT; F2 literal expected_keys for the grouping case; F3 expected_daemons in vectors, asserted in jest; F4 duplicate test replaced by upsert-after-gone-row. Test-only/script fixes, no round 2 needed |

| diff review | 2 (sonnet, orchestrator) | M4 fix diff 1c83be7..361bc9e | 361bc9e | - | CLEAN; F1 test proven red without the fix |

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

| diff review | 1 (opus, orchestrator) | M2 10f82e095..e7ad7f4 | see commit | - | CHANGES: 2 MAJOR (F1 handshake timeout + socket cap, F2 serialize feed passes) fixed; MINOR F3 stale snapshot after stop fixed, F5 single close fixed, F7 tests (non-UTF-8 Origin, Lagged fallback) added; F4, F6 declined/noted (see Declined); MAJOR fixes are behaviour-bearing, round 2 re-review (sonnet) due |
| diff review | 2 (sonnet, orchestrator) | M2 fix diff e7ad7f4..c77b398b6 | c77b398b6 | - | CLEAN; low notes: close() untimed while permit held, timing-based lagged test |
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
| M2 | diff review round 1 (opus) | F4 a non-malformed error resets the malformed counter | the contract counts malformed frames in a row; a typed non-malformed error breaks the run, as written |
| M2 | diff review round 1 (opus) | F6 read_session awaited inline blocks the select loop | noted, accepted for M2: a lagging feed falls back to the log (now tested); revisit if reads get slow |
| M1 | diff review round 1 (opus) | F4 double lock in the ReadStructuredEvents path | harmless: the second lock is uncontended and held briefly |

## Follow-ups

- CI llvm-cov merge intermittently hits a corrupt `_1.profraw` (signature 17540017307748215547); M1 added a ci.yml step dropping unreadable profiles; root cause unknown, to investigate.

## Handover

None. M4 (T027–T037) is implemented and committed on this branch; the unit stopped at the review snapshot because it has no Agent tool. Next: the orchestrator runs review A (code-review high) and B (conformance), sends findings back, and opens the PR from `specs/1732-client-ws-api/m4-pr-body.md` (title in `m4-pr-title.txt`).

M4 notes for the reviewers and for M5/M6:
- No new dependency (nothing added to any `package.json` or `Cargo.toml`).
- T031/T032: the real client does not reuse `ConnectionManager` as a class (its frame format is `{id, method}`/`{seq}`, the wire is `{id, type}` with `subscribe {since}`); the backoff is extracted to `connection/backoff.ts` and both use it, and `ConnectionManager` gained the `connecting` state. `real/wire.ts` was not created: `src/wire.ts` (M3) already brands ids at one seam.
- `OrchestratorApi` gained `onSnapshot`, `readSession` and `retry`; `ConnectionState` gained `connecting`; `Transport.onClose` carries `code`/`fatal`, `Transport.onVisible` is optional.
- The real API has no sign-in until #1733: `real/localAuth.ts` offers one local provider so the existing gate stays. FR-013 text unchanged.
- T029 tests live in `apps/client/tests/{session,sidebar,components}` (Jest only matches `tests/`), not `features/*/__tests__`. The banner stays in `components/` (T034 says reuse).
- T033: no `connection.connected` key (the banner is empty while connected). `error.*` keys are camelCase of each code plus `unavailable` and `unauthenticated` (the client's own codes).
- T035: `client-start` (alias `web`, already on main) now runs the real client; `client-web-mock` is the mock one; `start:mock` script added.
- T036: `scripts/local-stack.sh` already has the port check, overrides and trap that T040 lists; seed is `projects.json` with embedded sessions (no AI CLI; session content is empty until a daemon holds events). M6 T040 only needs to extend the test if more is wanted; T042 can fold into `docs/user-guide/web-client.md`.
- Gate: `scripts/gate.sh` ran fmt and clippy (core, workspace) green, then stopped at `cargo nextest` (not installed in the container, as in M3). JS: `pnpm --filter @micold/client lint|typecheck|test` and `@micold/protocol typecheck` green (217 and 73 tests); `scripts/tests/*.test.sh` run by hand; web export and `scripts/client/bundle-check.mjs` green. Local gate: scoped only, full gate left to CI.
- Not done: the real daemon+orchestrator stack was not started end to end in this container (quickstart Part B is M6 T041).

## Open escalation

None.

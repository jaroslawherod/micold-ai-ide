# Autopilot ledger — #1732 client-ws-api

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1732 "JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume" (part of #1722; FR-017R, FR-030). Owner goal for this run: a WORKING web UI against the real orchestrator instead of the dev-only mock — replace the web client's mocked OrchestratorApi (specs/1735-rn-client-scaffold, `mise run web`) with a real one over /api/ws, i18n en+pl for every new string, and a `mise run` task starting orchestrator + >=1 daemon + web client locally. No desktop/iced UI work. Aim 3-4 milestones. Cloud thread: no mise/gh; scripts/gate.sh and GitHub MCP; merge main in, never force-push. labels: flow:feature, effort:high (in-progress, claimed)
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1732
- **Worktree branch**: claude/project-thread-j62svx
- **Started**: 2026-11-17
- **Phase**: milestones (M6 next)
- **Next step**: M6 (T038–T042): 3-browser Playwright, CI, docs, polish (fresh thread).

Blockers: none. #1726 merged (#2007, #2012, #2016 at c316a62be); #1948 merged (#2031 at 15a603c3c). Base: origin/main ff1592a91.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #2032 | Design PR (spec, plan, tasks) | merged | fbfa8360e |
| #2033 | M1 daemon ReadStructuredEvents | merged | 10f82e095 |
| #2034 | M2 /api/ws listener | merged | a1c9514b3 |
| #2037 | M3 schema + TS types | merged | a7d7a3c12 |
| #2038 | M4 web client on real API | merged | 5ebc01334 |
| #2039 | M5 local kind stack | merged | 1db9333f1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T003–T004 | full | daemon ReadStructuredEvents (wire change) | #2033 | merged |
| M2 | T001–T002, T005–T019 | full | /api/ws listener: list, read_session, subscribe+resume, heartbeat, origin refusal | #2034 | merged |
| M3 | T020–T026 | full | generated schema+TS, drift gate, FR-012 vectors | #2037 | merged |
| M4 | T027–T037 | full | web client on the real API, i18n, minimal local-stack | #2038 | merged |
| M5 | T043–T055 | full | local kind stack: `mise run local-stack` starts a kind cluster and runs the full setup (orchestrator, daemons, Postgres, web) inside it; real-cluster deploy later (jaro 2026-11-17). Reuse #1981 kind+Helm e2e; FR-019–025, US5, SC-008 | #2039 | merged |
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
| M5 amendment review | 1 | spec.md, plan.md, tasks.md | 508a1ad305d6399c558739a5d25e8e9dfbfcf5e6 | 20e9ba559138a6384580a39f47cb10993f1a93b8:508a1ad305d6399c558739a5d25e8e9dfbfcf5e6 | CHANGES: 7 (5 MAJOR, 2 MINOR), all fixed in c6a8945310f0fe9bfb6f0e8038105825ba617a88; round 2 (sonnet) due on bd4e06335766c53cb100154977c0787a18720671:c6a8945310f0fe9bfb6f0e8038105825ba617a88 |
| M5 amendment review | 2 (sonnet) | fix diff | see commit | - | PASSED with 3 text nits (ledger T-range, T046/T049 configured() input, T052 run order), fixed |
| diff review | 1 (orchestrator) | M5 1ba83dcdd5b9b9e32815c5cbedc4004932d2dd09..b62f39a30829d27512a5c135834a3ef5d8a0b467 | b62f39a30829d27512a5c135834a3ef5d8a0b467 | 1ba83dcdd5b9b9e32815c5cbedc4004932d2dd09:b62f39a30829d27512a5c135834a3ef5d8a0b467 | CHANGES: F1 BLOCKER, F2-F4 MAJOR, F5-F6 MINOR; all fixed except helm pin (declined, R13); round 2 due |
| diff review | 2 (pending) | M5 fix diff b62f39a30829d27512a5c135834a3ef5d8a0b467..6d65b3cdaeb97cba5c38df4bd9b15e272342327f | 6d65b3cdaeb97cba5c38df4bd9b15e272342327f | b62f39a30829d27512a5c135834a3ef5d8a0b467:6d65b3cdaeb97cba5c38df4bd9b15e272342327f | pending |
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

## Red runs

| Task | Test | Failing run (before the code) |
|---|---|---|
| T044 | scripts/tests/local-stack-kind.test.sh | 14 FAIL: `local-stack.sh: line 93: mktemp: command not found` (no kind mode; process path ran) |
| T045 | scripts/tests/chart-local.test.sh | ci/local.yaml missing; `state.mode=sqlite ... sqlite arrives with #1948 and postgres with #1740`; bogus refused by schema enum, not the message |
| T046 | orchestrator::tests::a_given_data_dir_skips_the_users_daemon_toml | E0425 cannot find function `implicit_config` |
| T053 | scripts/tests/dockerfile-web.test.sh | FAIL no Dockerfile.web |
| T054 | scripts/tests/local-stack-probe.test.sh | 3 FAIL: scripts/local-stack-probe.sh: No such file or directory |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | diff review round 1 (opus) | F4 a non-malformed error resets the malformed counter | the contract counts malformed frames in a row; a typed non-malformed error breaks the run, as written |
| M2 | diff review round 1 (opus) | F6 read_session awaited inline blocks the select loop | noted, accepted for M2: a lagging feed falls back to the log (now tested); revisit if reads get slow |
| M1 | diff review round 1 (opus) | F4 double lock in the ReadStructuredEvents path | harmless: the second lock is uncontended and held briefly |

## Follow-ups

- CI llvm-cov merge intermittently hits a corrupt `_1.profraw` (signature 17540017307748215547); M1 added a ci.yml step dropping unreadable profiles; root cause unknown, to investigate.

## Handover

M5 merged as 1db9333f1 (PR #2039, squash). Spec 2 review rounds, diff 2 rounds + sonnet review of CI-fix commits (CLEAN). First live kind runs took 5 CI rounds: Postgres 18 PGDATA layout, orchestrator config missing [daemon] name, kubernetes sidecar must not have [listen] socket, probe request lacked id, kubectl port-forward does not exit on pod delete. Follow-ups: helm v3.19.0 -> v3.22.1 (no checksum reachable from cloud); stale header comment scripts/local-stack-kind-live.sh:5-6; kube_real_support NODE_IMAGE still v1.34.0. Next: M6 T038-T042.

## Open escalation

None.

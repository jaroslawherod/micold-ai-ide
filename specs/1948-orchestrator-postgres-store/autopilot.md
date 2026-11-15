# Autopilot ledger — #1948 orchestrator-postgres-store

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1948 "feat: PostgreSQL store for the orchestrator (sqlx, migrations, store traits; SQLite for local mode)" (part of #1722; FR-042, FR-046). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1948
- **Worktree branch**: claude/project-thread-8aowf1
- **Started**: 2026-11-15
- **Phase**: milestones
- **Next step**: M1 PR #1970 CI green, squash-merge; then M2 in a fresh thread

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1967 | Design (spec, plan, tasks) | merged | f5f8e812a |
| #1970 | M1 | open (draft) | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Registry, auth and audit stores in memory, shared suite (MVP) | #1970 | open, reviews CLEAN |
| M2 | T014–T020 | full | SQLite handle, schema, registry store | | pending |
| M3 | T021–T025 | full | SQLite auth and audit stores | | pending |
| M4 | T026–T031 | full | PostgreSQL store and CI service container | | pending |
| M5 | T032–T035 | full | Migration safety | | pending |
| M6 | T036–T038 | full | No private key in any table | | pending |
| M7 | T039–T044 | full | [database] config and backend rule | | pending |
| M8 | T045–T050 | full | Orchestrator starts on the chosen store | | pending |
| M9 | T051–T054 | full | Import registry.json once | | pending |
| M10 | T055–T060 | full | Ownership columns and adoption; quickstart recorded | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec | d3937c8969e34c08a0ce2f78559cde430b5c65f9 | 09cb2cfc0a611043c58abc651afcc426257803e6:d65a0b90b4bab89ddf4da88fcd33c56490018866 | CHANGES: 5 MAJOR 3 MINOR, all fixed |
| Spec | 2 | spec | 69983c1058338b7230c9e70066dc73eca578a606 | 1faf1b6e8adebd019dd1d2e6f8675ad1d5a15b12:d3937c8969e34c08a0ce2f78559cde430b5c65f9 | CLEAN (2 MINOR fixed, prose only) |
| Plan | 1 | plan | (pending commit) | 039972fca59028890bb8b22565c8ee7ab6bca81a:b1cd1965e0f01444b33b7d262dbda5e068ff3b06 | CHANGES: 1 BLOCKER 1 MAJOR 3 MINOR, all fixed |
| Plan | 2 | plan fix diff | (this commit) | 1c7862375e7765f0e55c0e9fc8331fd4e4c8c56b:8123b3e49d68aa61553fe7273ad201b44137eebf | CLEAN (1 MINOR fixed, prose only) |
| Plan | 2 | plan | 190e9973277b43ecb3c28cee728d38243108c2e5 | 1c7862375e7765f0e55c0e9fc8331fd4e4c8c56b:8123b3e49d68aa61553fe7273ad201b44137eebf | CLEAN |
| Tasks | 1 | tasks | (pending) | e8c4d28d157e06b94aac968d5bbbf982906eeb65:7e5052b09b288667942286e771b4b91a672f3412 | CHANGES: 4 MAJOR 3 MINOR, all fixed (M1+M2 merged, milestones renumbered, Verify one filter per command, T012 not [P], T027 unreachable case, T041 config test before T042, quickstart wording) |
| Tasks | 2 | tasks fix diff | (pending) | b2d38ef36db8d9a9c95fa37b0ededa0f6b612709:ab2d68a634bf73201e1ffe5c3c6d7b122d741812 | CHANGES: 1 MAJOR (Verify joined with "and" not &&), fixed |
| Tasks | 3 | design | 51d20197b6bb9ca92ffbf64dec4be986cd56f4fa | 3d3ec8c874fcffe3ca8aecf860c97d3efe962a9a:c9db7e595c1f9181c01606cca34854d161c8a40e | CLEAN |
| M1 A | 1 | M1 diff (code-review high, run by the unit: no Agent tool) | 6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | 311e34200b5cce2ae68cd49c3a73e1e8bd98ffea:6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | CLEAN (1 MINOR noted as a follow-up) |
| M1 A | 1 | M1 diff | 6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | 311e34200b5cce2ae68cd49c3a73e1e8bd98ffea:6d4d88cadd058fbd51cb0e2c2de1ebbf21dae2c9 | CLEAN |
| M1 B | 1 | M1 | 7e2b729df2134721ab66d77f85e9f3b65a3422c6 | ffae1d956ca5a1bf2f113872c8d6ce62e5df7d1c:7e2b729df2134721ab66d77f85e9f3b65a3422c6 | CLEAN (2 MINOR: test-after cycle 3 accepted as recorded; redeem_invite unknown user -> M3 follow-up) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1967 | Design (spec, plan, tasks) | merged | f5f8e812a |
| #1970 | M1 | open (draft) | |
| tasks | analyze | D1 Principle IV CRITICAL | Decided by jaro (2026-11-14/15), recorded in plan Complexity Tracking; #1949 amends the text; tasks.md header says a network deployment waits on it |
| tasks | analyze | D2 Principle VI CRITICAL | Linux only per jaro (#1897); CI linux-daemons job covers the build |

## Handover

None. M1 done and committed (6d4d88cad), full gate green on it, plan-drift clean (drift-M1.md), cycle-log written. Review B (conformance, sonnet) pending: the orchestrator dispatches it, then opens the PR from pr-M1.md. Next unit: M2.



## Open escalation

None.

## Follow-ups not done

- M2 (SQL auth store): `rotate_refresh` on a revoked token must commit the family revocation and then return `Refused`, so it cannot be one transaction that rolls back on the error; the memory store revokes then refuses.
- M2 and M4: `load` is ordered by `(tenant_id, id)`; the JSON and memory stores sort in `load`, the SQL ones with `ORDER BY`.
- `sqlx` and `sha1` are in `micold-daemon`'s dependencies but unused until M2 (T001 asked for both).
- Cloud gate: `scripts/gate.sh` was red twice on script tests that passed alone (`check-assertions-frozen` case 028, `autopilot.test.sh` facts cases) and green on the third run on the same tree; the scoped gate fails two root-only permission tests in micold-core as root (gate.sh runs them unprivileged).

- M1–M6 and M10 carry no user-guide change (daemon crate is not micold-user-facing): use label docs-not-needed. M7–M9 carry guide tasks.
- Cloud thread: preflight failed only on `mise` (missing) and `gh` auth (known cloud setup); the run proceeded. Use `scripts/gate.sh` and the GitHub MCP tools.
- #1949 (constitution 2.0.0, PostgreSQL for network deployments) is not merged: nothing here depends on it; it must merge before a network deployment ships. Constitution Principle IV currently says local-first storage.
- Constitution conflict: it still says local-only storage and three OSes; jaro's Linux-only decision (#1897, 2026-11-14) and #1949 supersede. The spec keeps Linux-only and cites #1897.
- kube_real tests cannot run in cloud (no kind).
- M3 (SQL auth store): `redeem_invite` must refuse an unknown redeeming user, like `put_refresh`, `rotate_refresh` and `link_identity`, or contracts/store-traits.md must say the caller guarantees it (M1 review B F2). Also M1 review A: a SQL `rotate_refresh` commits the family revocation before returning `Refused`.

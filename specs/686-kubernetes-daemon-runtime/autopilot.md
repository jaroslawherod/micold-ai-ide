# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: plan
- **Next step**: plan review round 2 (scoped, sonnet) on the snapshot in *Review rounds*; then set Next step to `tasks unit`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-004 default image source | #806 build, pushed to a user-named registry; pull stays an option | orchestrator default, pending owner | decision card 2026-10-17 |
| 2 | clarify | FR-004 provider CLIs in the pod | installed at build time | orchestrator default, pending owner | decision card 2026-10-17 |
| 3 | clarify | FR-007 clone source | project git remote with allowed credential; no-remote refused | orchestrator default, pending owner | decision card 2026-10-17 |
| 4 | clarify | private registry pull; push auth | pull secret on FR-008 allow-list; push via local runtime login | agent | spec.md FR-008, FR-002 |
| 5 | clarify | image tag/rebuild; CI image loading | deferred to plan | agent | spec.md FR-017, FR-021 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 7330b10a0e165a38d678df142737f8a7c1ea47d0:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 pod-gone → stopped + user Start; F2 FR-023 + multi-client edges, FR-016 identity; F3 kubeconfig/exec-plugin edges, FR-022; F4 per-phase bounds in FR-012; F5 Reconnect scenario; F6 provider CLIs in FR-004 marker; F7 version wording) |
| Spec | 2 (scoped, sonnet) | aed1af6291d0f65b2e21df970370031d92b506f6:171d36233aff56cab8399d2c2c08b7ed1c109dc2 | CLEAN — 1 MINOR fixed in clarify (SC-003, story 4.5, story 6.2 say "applicable phase bound (FR-012)") |
| Plan | 1 | c2d5b880c183edc0c6dcd702539b137bbeaf2b7e:744377ac1576676ae85cc3ae5c9897190bba1bf2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 shared `DaemonResources` with separate requests/limits, FR-005 manifest tests, contract wording; F2 `CliRuntime` inherent `build_image`/`push_image`/`repo_digest`, tests over `RecordingRunner`; F3 `sha2` dropped for `sha256_hex`; F4 fields extracted in `container_fields.rs`, recorded in K1; F5 K4 guard restated; F6 FR-007 client reducer tests) |
| Plan | 2 (scoped, sonnet) | b26d31e28d1362689a3bc021fc0af7cdff72869c:48a879226ceb5fe45e669e6aade3331e2c742d6f | pending (dispatched by orchestrator) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. Clarify round 1 questions are with the owner as decision cards (2026-10-17); the spec uses the recommended defaults meanwhile, marked "pending owner answer" in its Clarifications.

## Follow-ups not done

None yet.

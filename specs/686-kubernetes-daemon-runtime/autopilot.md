# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: tasks
- **Next step**: tasks unit (continue from *Handover*): tasks review round 1 on the snapshot in *Review rounds*

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T033, T079–T082 | full | Add (given image), start/adopt, tunnel to connected, stop, remove | — | planned |
| M2 | T034–T046 | full | Build → push → pod by digest; requests and limits | — | planned |
| M3 | T047–T055 | full | Clone remote + worktrees in the pod (RepoClone, protocol 49) | — | planned |
| M4 | T056–T062 | full | Reconnect; pod gone → stopped with reason | — | planned |
| M5 | T063–T071 | full | Allowed credentials only; pod security context | — | planned |
| M6 | T072–T078 | full | Failure reasons (preflight RBAC, pull, schedule, PVC) | — | planned |
| M7 | T083–T087 | full | Rest of the form: placement, volume, Reconnect, Logs | — | planned |
| M8 | T088–T096 | full | kube_real_* suite, `mise run test-kube`, CI job kube-cluster | — | planned |

Cut notes: US1 (>10 tasks) split along its acceptance scenarios and the plan's separable parts (3a given image, 3b image source, 3c clone source). US5 Stop/Remove (T079–T082) ship in M1 so the Kubernetes row reachable from the form has working Stop and Remove (no half-wired UI). FR-013 rollback and per-phase timeouts moved into M1 (analyze D5/D8). Polish T097–T098 change no code: close unit. M8 adds no user-visible behaviour: `docs-not-needed` if its PR is `feat`.

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
| Plan | 2 (scoped, sonnet) | b26d31e28d1362689a3bc021fc0af7cdff72869c:48a879226ceb5fe45e669e6aade3331e2c742d6f | CLEAN — 1 MINOR fixed, prose only (plan.md Primary Dependencies, contracts/daemon-image.md: `program_of` is the free function `sandbox::cli::program_of`, not a `CliRuntime` method) |
| Tasks | 1 | 1aab2e66eaf532352e358ce882e50c398d808e64:b3c1e46cab50933fb87b51b5d85bae4ad7b90da8 | pending (orchestrator dispatches; rubric rubrics/tasks.md) |

## Declined review findings

- speckit-analyze D13 (start failure maps to `Unreachable`): already the 491/489 rule, stated in data-model.md state machine delta; no change.
- speckit-analyze D10 (owner decisions pending): no task change; phases 3b/3c are the separable parts.
- speckit-analyze D14 (tdd test-list): optional hook; tasks already order tests first.
- speckit-analyze D1 alternative (kind smoke test in M1): declined; pure `to_api_failure`/`delete_params` table tests in M1 instead, real cluster in M8.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Tasks unit over the context cap after speckit-tasks + speckit-analyze. Done: plan review round 2 recorded CLEAN (MINOR `program_of` fixed); tasks.md written (T001–T098, 8 milestones, all `full`); speckit-analyze run and its D1–D9, D11, D12 fixed in tasks.md (plan.md Constitution row I corrected for D1); ledger Milestones filled. Next: (1) tasks review round 1 on the *Review rounds* snapshot (orchestrator dispatches); act on findings per review-rounds.md. (2) Close checklists: checklists/requirements.md "No [NEEDS CLARIFICATION] markers remain" is unchecked; spec.md has 0 markers (grep), pending owner answers are recorded defaults — needs a reviewer confirmation, then tick. (3) Specs-only gate (`scripts/check-criteria-observables.sh specs/686-kubernetes-daemon-runtime/spec.md`). (4) Design PR files: title `#686: Support a Kubernetes session daemon runtime (#686)`, body per pr.md design PR ending `Refs #686`, in the orchestrator's scratchpad paths. No open findings.

## Open escalation

None. Clarify round 1 questions are with the owner as decision cards (2026-10-17); the spec uses the recommended defaults meanwhile, marked "pending owner answer" in its Clarifications.

## Follow-ups not done

None yet.

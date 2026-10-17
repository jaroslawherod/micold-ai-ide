# Autopilot ledger — #686 kubernetes-daemon-runtime

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #686 "Support a Kubernetes session daemon runtime" (with owner comment: align image source with #806); labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #686
- **Worktree branch**: claude/project-thread-xelisc
- **Started**: 2026-10-17
- **Phase**: implement M1
- **Next step**: M1 implement — continue the handover: make cycle 1 (core types) green, then the rest of M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #965 | design | merged | 0d36906050f9ad7fedde4b06fa0ee0ca31d231c9 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T033, T079–T082 | full | Add (given image), start/adopt, tunnel to connected, stop, remove | — | in progress |
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
| Tasks | 1 | 1aab2e66eaf532352e358ce882e50c398d808e64:b3c1e46cab50933fb87b51b5d85bae4ad7b90da8 | CHANGES: 4 MAJOR, 3 MINOR — all fixed (F1 speckit-tdd-plan run: tdd/test-list.md A1–A27, U1–U116, markers on every behavioral task, acceptance tasks T099–T105 per milestone, tdd/cycle-log.md baseline; D14 un-declined; F2 M2/M7 Verify one filter per command; F3 M1 Verify adds `--lib kube::live` and `micold-daemon server::`; F4 credentialed-clone clause moved from M5 to M8 (T092), M3 text points there; F5 checklist ticked, Notes say defaults pending owner; F6 T002 not [P]; F7 T075 names only the M6 classifiers) |
| Tasks | 2 (scoped, sonnet) | bf9cd12b8a7d863016aa8c342f4bf594a34f477e:a8551c98307dc5eb20c1a8b684f92be48c98641b | CLEAN — 1 MINOR fixed (Milestones preamble names T097/T098 Polish as in no milestone, left to the close unit) |

## Declined review findings

- speckit-analyze D13 (start failure maps to `Unreachable`): already the 491/489 rule, stated in data-model.md state machine delta; no change.
- speckit-analyze D10 (owner decisions pending): no task change; phases 3b/3c are the separable parts.
- speckit-analyze D1 alternative (kind smoke test in M1): declined; pure `to_api_failure`/`delete_params` table tests in M1 instead, real cluster in M8.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit 1 of M1 (context cap reached). Done: branch restarted from origin/main (0d369060); T001–T003
setup (workspace `kube` 4.2 / `k8s-openapi` 0.28 in the client only, no `openssl-sys`; `kube-fake`
feature in core, `kube-real-runtime` in client; empty modules registered); cycle 1 RED (see
tdd/cycle-log.md "Cycle 1"): tests + stubs in `core/kube/{settings,names,classify,credentials,start}.rs`,
`core/daemon_image/mod.rs`, `core/protocol/auth.rs` (`listener_auth`). Tasks not yet ticked.

Next: implement the stub bodies (validate, Toleration::parse, DaemonImageSource::default_for/is_moving,
DaemonResources default + validate, DaemonKey::of, slug, ResourceNames, is_ours, KubeFailure Display,
listener_auth) → green with `cargo test -p micold-core --lib -- kube:: daemon_image:: listener_auth_tests`;
mutant-check the four tests that passed on the stubs; log green in the cycle log; tick T004, T005,
T007, T009 (tests) and T011, T012, T014, T018 (impl) — T008/T016 also need the daemon side
(`adopt_auth_token` via `listener_auth`, U22 characterization). Then T006/T013 (`DaemonRuntime::
Kubernetes` in core/daemons.rs: serde arm `"kubernetes"`, label, startable, the registry `check`
arm calling `KubeSettings::validate`; client `features/daemons.rs` endpoint arm and
`sandbox/placement.rs::from_runtime` need a Kubernetes arm), then T010/T017, T015, T019–T033,
T079–T082, T099. U16 "naming the first": `RegistryError::DuplicateName` carries no name today —
decide (assert the existing variant, or add a field) and record it. Build env: CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0; run builds detached + hold.sh.

## Open escalation

None. Clarify round 1 questions are with the owner as decision cards (2026-10-17); the spec uses the recommended defaults meanwhile, marked "pending owner answer" in its Clarifications.

## Follow-ups not done

None yet.

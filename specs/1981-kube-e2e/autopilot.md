# Autopilot ledger — #1981 kube-e2e

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Epic: end-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI (#1981); jaro's words in specs/1981-kube-e2e/spec.md. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1981
- **Worktree branch**: claude/project-thread-ltu9ny
- **Started**: 2026-11-15
- **Phase**: milestones
- **Next step**: M1 (#1982) milestone unit.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1987 | Design | merged | 6c2819edede47166c5034a37335c7c62c775fa4d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 (#1982) | T001–T009 | full | Image + chart + data-dir/socket flags | #… | review A done, PR pending |
| M2 (#1983) | T010–T015 | full | Ops /healthz /readyz, probes, --ops-addr (before harness, see plan) | #… | pending |
| M3 (#1984) | T016–T022 | full | kind harness, mise task, CI job kube-e2e | #… | pending |
| M4 (#1985) | T023–T030 | full | API e2e suite (ops, link, restart, seeded breakages) | #… | pending |
| M5 (#1986) | T031–T032 | light | network/ui placeholder slots; real tests after #1732/#1954 | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | How does the test daemon reach the orchestrator? | Test-only daemon container sharing the emptyDir socket | agent-resolved (orchestrator; jaro can overturn) | spec.md FR-011 |
| D2 | clarify | Default chart state mode? | `file` on a PVC; sqlite/postgres fail rendering until #1948/#1740 | agent-resolved (orchestrator; jaro can overturn) | spec.md FR-017 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec | n/a | 3d44f8b336c799ba30e46d236c626356b3939dc1:a089ee32c8f892cce0e56df14602c477047c6834 | CHANGES: 3 MAJOR (F1 F2 F3), 3 MINOR; all fixed, not re-reviewed |
| spec | 2 | scoped | 5c0712c5da20b4c7631132c4264657df9548e803 | 6b26e7bcb50218985959800c72c608652ee03be7 | CLEAN |
| plan | 1 | full | n/a | a93ee5534a7e31dc2375b7e0774a822515594c74:2a8afeac5f380ebde8010c8aac79ded2d55c4283 | CHANGES: 4 MAJOR (F1-F4), 2 MINOR (F5 F6); all fixed, not re-reviewed |
| plan | 2 | scoped | af46640a697fa39efc4643087b4a875ff9152982 | b99f4c78af95f4dad1805307dae6165ce0f9da7b | CLEAN |
| tasks | 1 | full | n/a | e7e613c5573b0aacf77dd0b865f2a0f06bb12567:76a663f4950b1e5ecac3627481decad6be462b8c | CHANGES: 4 MAJOR (F1-F4), 3 MINOR (F5-F7); all fixed, re-review pending |
| tasks | 2 | scoped | 469a61270cd7ff81c61dd0847b774db5f8a7add3 | 31eb208412cadeb4b4503ef792e3e979a633bb3d | CLEAN |
| M1 A (code-review high) | 1 | full | 50a73bfd4 | b35b38afe6f6464849753a5f314e2f2e0fe5be34:50a73bfd479d489c2da3343d57dfc183857d97ae | CHANGES: 2 MAJOR (kube set misses image inputs; misses job's own files), 8 MINOR; 9 fixed, 1 declined (appVersion tag), 1 noted (socket dir mode, documented) |
| tasks | 3 | scoped | afc1ab21c041d57654e68c85da428d2bb04adb9e | d6036c4e0ce6be2d465dfaf07f3eee75f4dc983a | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
| M1 | A r1 | `Chart.yaml` appVersion 0.18.0 is a tag nothing publishes; release-please does not bump it | No image is published by this epic; the release workflow (build, sign, push, version in step) is #1740's per the 1722 plan. README now says so and shows `--set image.tag=dev`. Follow-up recorded. |
| M1 | A r1 | `endpoint_at` skips the owner-only 0700 directory check for an explicit `--socket` | An explicit path is the operator's choice, as `run_with` and `[orchestrator_link] socket` already are; in the chart it is a pod-private emptyDir. Documented on `endpoint_at`. |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None.

## Follow-ups not done

- #1740: publish `micold-orchestrator` (and `micold-web`) images from the release workflow and keep `deploy/helm/micold/Chart.yaml` `appVersion` in step with releases (M1 review A, declined there).

## Plan notes

- Clarify unit skipped by the orchestrator (no markers left in the spec). Milestones reordered: M2 ops before M3 harness (smoke needs the readiness probe); spec table updated. Plan written, plan review pending (reviewer round 1 not run: unit has no Agent tool).
- Tasks cut (T001-T030, M1-M5). Plan deviation: install-kube-tools.sh is created in M1 with helm and kubeconform pins (kube-chart job needs them); M3 adds kind and kubectl. No user-guide tasks (no end-user surface). Tasks review pending; gate and PR pending.
- M1 thread (2026-11-15): agent types listed; units have no Agent tool, so the orchestrator dispatches review B.
- Tasks review 1 fixes: T003 check-first, M4 reordered and split (T023-T030, adds refusal suite harness wiring), M5 now T031-T032, SC-001 under M3, spec FR-019/ownership/FR-013 and harness-env.md corrected. Issues updated by orchestrator.

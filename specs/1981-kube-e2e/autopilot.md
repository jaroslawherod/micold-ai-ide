# Autopilot ledger — #1981 kube-e2e

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Epic: end-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI (#1981); jaro's words in specs/1981-kube-e2e/spec.md. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1981
- **Worktree branch**: claude/project-thread-mxfy6x
- **Started**: 2026-11-15
- **Phase**: milestones
- **Next step**: M2 (#1983) PR open; orchestrator waits on CI and merges.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1991 | M1 | merged | a6fc96478a60ddf1960e98ff83ebce27ddc7fb09 |
| #1987 | Design | merged | 6c2819edede47166c5034a37335c7c62c775fa4d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 (#1982) | T001–T009 | full | Image + chart + data-dir/socket flags | #1991 | merged a6fc96478 |
| M2 (#1983) | T010–T015 | full | Ops /healthz /readyz, probes, --ops-addr (before harness, see plan) | #… | implemented, in review |
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
| M1 A (code-review high) | 2 | scoped | (fix commit) | bddf4e635cd5000ff61cda4c9101081df8458227:bb8c35ea2c6d251b8f1849f72cc1e4531a5acc00 | CHANGES: 2 MAJOR (smoke trap on a local var; pr_proven skip loses the main-push net), 8 MINOR; 9 fixed, 1 declined (M3 gate width) |
| M1 A (code-review high) | 3 | scoped | (fix commit) | cbba162d5b20441d5f920eec269f7ec432a34a65:12c6d39d415b4c76ddbb1a7a0c81cec4b4601bed | CLEAN: 5 MINOR; 2 fixed (norepo pattern tolerant of older helm; ci/default.yaml must be `{}`), 3 left (fail-safe kube_paths on an unclassifiable proven push; created-not-started smoke container; schema reformat) |
| M1 A (code-review high) | 1 | full | 50a73bfd4 | b35b38afe6f6464849753a5f314e2f2e0fe5be34:50a73bfd479d489c2da3343d57dfc183857d97ae | CHANGES: 2 MAJOR (kube set misses image inputs; misses job's own files), 8 MINOR; 8 fixed, 2 declined (appVersion tag; socket dir mode, documented) |
| tasks | 3 | scoped | afc1ab21c041d57654e68c85da428d2bb04adb9e | d6036c4e0ce6be2d465dfaf07f3eee75f4dc983a | CLEAN |
| M2 A+B (opus) | 1 | full | 5272ef54e | abb5d2e251df1baf302e677ce8074d310c49842a:5272ef54ed19168fc8a76e056116d3135072b743 | CHANGES: 1 MAJOR (ops served only inside serve, probes hang while the store opens), 3 MINOR (head size check, silent timeout, tests); all fixed |
| Review B | 1 | M1 | 49dcf2b3b4c6d7bf015c741fa78e6f67e1a9c18e | 7c47de4556526f50ff9beb7412373b4d950c17e4:49dcf2b3b4c6d7bf015c741fa78e6f67e1a9c18e | CLEAN (1 MINOR, no fix needed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | `Chart.yaml` appVersion 0.18.0 is a tag nothing publishes; release-please does not bump it | No image is published by this epic; the release workflow (build, sign, push, version in step) is #1740's per the 1722 plan. README now says so and shows `--set image.tag=dev`. Follow-up recorded. |
| M1 | A r1 | `endpoint_at` skips the owner-only 0700 directory check for an explicit `--socket` | An explicit path is the operator's choice, as `run_with` and `[orchestrator_link] socket` already are; in the chart it is a pod-private emptyDir. Documented on `endpoint_at`. |
| M1 | A r2 | image inputs in `micold-kube` will also widen M3's kind job | M3 (T019/T020) decides kube-e2e's gate; if the kind job should not run on every daemon change it splits an image attribute then. Today only kube-chart reads `kube_paths`. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Implicit `<data_dir>/daemon.toml` lookup in `orchestrator::configured` uses the user's data dir even when `--data-dir`/env is set (main's behaviour; merge review MINOR). Consider skipping it when a data dir is given, so a container never reads `$HOME`.
- Chart `state.mode` refuses `sqlite`/`postgres` naming #1948/#1740; #1948's store has merged since, so revisit which modes the chart can render (M2+ or #1740).
- #1740: publish `micold-orchestrator` (and `micold-web`) images from the release workflow and keep `deploy/helm/micold/Chart.yaml` `appVersion` in step with releases (M1 review A, declined there).

## Plan notes

- M2 (2026-11-16): T010-T015 done. `ops.rs` hand-written HTTP; `Ops` bound in `run` before the store opens, so probes see 503 not refused. Diff coverage 93.9% (cargo-llvm-cov, lib tests). Chart: `ops.enabled`/`ops.port`, `templates/service.yaml` (`<release>-orchestrator`), probes; conformance 56/56 with helm built from source (get.helm.sh blocked in cloud).

- Clarify unit skipped by the orchestrator (no markers left in the spec). Milestones reordered: M2 ops before M3 harness (smoke needs the readiness probe); spec table updated. Plan written, plan review pending (reviewer round 1 not run: unit has no Agent tool).
- Tasks cut (T001-T030, M1-M5). Plan deviation: install-kube-tools.sh is created in M1 with helm and kubeconform pins (kube-chart job needs them); M3 adds kind and kubectl. No user-guide tasks (no end-user surface). Tasks review pending; gate and PR pending.
- M1 thread (2026-11-15): agent types listed; units have no Agent tool, so the orchestrator dispatches review B.
- Tasks review 1 fixes: T003 check-first, M4 reordered and split (T023-T030, adds refusal suite harness wiring), M5 now T031-T032, SC-001 under M3, spec FR-019/ownership/FR-013 and harness-env.md corrected. Issues updated by orchestrator.

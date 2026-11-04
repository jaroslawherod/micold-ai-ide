# Autopilot ledger — #1528 daemon-config-file (Issue 7)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1528` (Deliver the configuration to Kubernetes daemons); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534 (see `autopilot-1524.md`, `autopilot-1525.md`, `autopilot-1526.md`); the tasks unit only cuts milestones for the "Issue 7" group (T036–T039, T091).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1528
- **Worktree branch**: feat/1528_deliver-the-configuration-to-kubernetes-daemons
- **Started**: 2026-11-03
- **Phase**: implement
- **Next step**: M1 PR open; wait for CI (T039 kind check runs there), merge, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1680 | design (milestone cut) | merged | 90a1522a392025444b33f429255ad10ca9314dbd |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | pod daemons read ConfigMap `<name>-config` at `/run/micold/config` and directory-mounted Secrets under `/run/micold/secrets/…`; no `MICOLD_*` env; `TZ`, `LANG=C.UTF-8` | T036–T039, T091 | full | — |

Five tasks, one acceptance scenario, so not split. Paths in the tasks checked 2026-11-03.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D2 | implement | Pod credential mounts: all to directories now? | git config only (to `/run/micold/secrets/git`, `GIT_CONFIG_GLOBAL`); the AI sign-in keeps its `subPath` mount, T065 (Issue 11) moves it to a directory and links it. Contract §3.4 puts no `sign_in_file` in a pod before that | agent-resolved | tasks.md T065 names `credentials.rs:73-79 drop subPath` |
| D3 | implement | Pod `listen.tcp` (contract §3.4, required by validation) would make the daemon serve TCP and not the socket the SSH bridge relays to | The daemon ignores `listen.tcp` when `daemon.kind = kubernetes` (`server::listen_tcp_of`) | agent-resolved | `server.rs` `tcp_listen_addr` |
| D4 | implement | Pods made before the config map exist | Recreated without asking (stamp lacks `config_map`), as for retired credentials | agent-resolved | `manifest::lacks_config_map` |
| D5 | implement | Pod `[listen]`/token (contract §3.4) cannot be satisfied: the token file is required (V15) and a Secret file in a fsGroup pod is group-readable (V14) | `[listen]` optional for `kubernetes` (validation V3 relaxed); the pod's file has no `[listen]`/`[auth]`; D3 kept for a hand-written file | agent-resolved | review A F1; `validate.rs` `listen` |
| D1 | spec | New spec for #1528? | No: parent spec, plan, contracts §3.4 and tasks T036–T039, T091 already on main (PR #1509); blockers #1524, #1525 are CLOSED | agent-resolved | specs/daemon-config-file/tasks.md "Issue 7"; autopilot-1526.md D1 |

## Review rounds

- Tasks review round 1 (snapshot 47d0d8181a03c75d1692d4824ddc42544481c27d:d0ddfd28dc82009eccfd212cbf64220e528f9722): CHANGES; F1 (MAJOR, T091 test not before code, T036 env list contradicted T091) fixed, prose only (T036 golden now expects `TZ`/`LANG`, T091 implements); F2 (MINOR) declined, column order is cosmetic; F3 (MINOR) left, T091 ID stable.
- Tasks review round 2 (fix diff vs 47d0d8181a03c75d1692d4824ddc42544481c27d:d0ddfd28dc82009eccfd212cbf64220e528f9722, sonnet): CLEAN; F1 fix holds. One MINOR (T036 fully green only after T091) fixed with a note on T038.

- Rebase-conflict review (sonnet, after rebase onto 2a88331e6 / #1527): CLEAN; one MINOR (POD_ENV doc comment wording) left. Full gate green on the rebased tree; one unrelated flake (`structured_permission::two_sessions_waiting_are_marked_separately`, passes 3/3 alone) on the first run.

## Declined review findings

- Review A: git config only via daemon env (sessions are daemon-spawned); TZ not in stamp (daemon sets own TZ; follow-up); hand-edited ConfigMap data not repaired (hash annotation); POD_ENV comment accurate; configmaps delete not in access reviews (same as secrets precedent). Fixed: unused SECRETS_MOUNT, token-file/listen (D5), TIMING placeholder.
- Review B F1 (no red run): declined, TEST_AFTER recorded honestly in cycle-log. F3 (V3/listen_tcp_of out of scope): declined, required for a working pod (D3, D5). F2 fixed: T039 left unticked until CI's kube-cluster job runs the kind test.

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- The pod's `TZ` env is not in the pod stamp: a changed time zone reaches new sessions with the file (the daemon sets its own `TZ`) and the pod's env at its next recreation.
- The pod's file carries no `[[projects]]`, `[[github.tokens]]`, `[[ai.cli]]` or `[startup]` yet (Issues 8, 11, 12, 13 deliver them); the AI sign-in keeps its `subPath` mount until T065.
- T039's live kind check did not run locally (podman provider: `kind load` hangs); CI's `kube-cluster` job runs `kube_real_the_pod_reads_its_configuration_from_the_config_map`.

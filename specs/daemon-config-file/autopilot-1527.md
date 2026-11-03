# Autopilot ledger — #1527 daemon-config-file (Issue 6)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1527` (Deliver the configuration to container daemons); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534 (see `autopilot-1524.md`, `autopilot-1526.md`); the tasks unit only cuts milestones for the "Issue 6" group (T030–T035, T090).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1527
- **Worktree branch**: feat/1527_deliver-the-configuration-to-container-daemons
- **Started**: 2026-11-03
- **Phase**: tasks (review done, design PR next)
- **Next step**: tasks unit cuts milestones for Issue 6 (T030–T035, T090) in `specs/daemon-config-file/tasks.md`

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| — | Design (milestone for Issue 6) | not opened | — |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | container daemons read the same read-only `/run/micold/config/daemon.toml` at boot and over SSH; no `MICOLD_*` env; `LANG=C.UTF-8` | T030–T035, T090 | full | — |

Checked 2026-11-03: `container_env` is at `daemon.rs:1328` (not 1283); `docs/user-guide/sandboxed-daemon.md` exists; the entrypoint reads no `MICOLD_SSH_DIR` today. Fixed in tasks.md.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec? | No: design merged on main (PR #1509, `specs/daemon-config-file/spec.md`, FR-005, FR-029, FR-039; contract §3.3). Same precedent as #1524, #1525, #1526, #1534 | agent-resolved | specs/daemon-config-file/autopilot-1526.md D1 |

## Review rounds

Tasks review A round 1 (`f688c7ee:038b306fe`): CHANGES, F1 MAJOR (TZ unassigned) fixed in T030/T033/T090, F2/F3 MINOR fixed in T033/T034; wording only, no new round. (Spec review skipped: no spec written; the shared spec was reviewed in PR #1509.)

## Declined review findings

None.

## Handover

None.

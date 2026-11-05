# Autopilot ledger — #1537 daemon-config-file (Issue 15)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1537` (Retire the old MICOLD_* start-up variables); labels: `flow:feature`, `effort:high` (switched from chore; the work changes user-visible behaviour). Spec, Clarify, Plan skipped; design on main (PR #1509); tasks unit only cuts milestones for the "Issue 15" group (T084, T084b, T085, T085b, T086a, T086b, T086; the originals T084, T085, T086a, T086 were split).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1537
- **Worktree branch**: chore/1537_retire-the-old-micold-start-up-variables
- **Started**: 2026-11-05
- **Phase**: milestone M1
- **Next step**: orchestrator waits on CI, merges #1709 (rebased onto origin/main, basic image hash repinned to dd029f826ae2f841, DAEMON_SIDE removed, cycle-log entry added); then M2 (blocked on #1530 M2)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1704 | Design (milestones for Issue 15) | merged | 5e72b83360ee747bc80ad577a468d2119f47bb48 |
| #1709 | M1 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T084, T085, T086b | full | Daemon release build ignores the six `DaemonSettings` variables, one warning each naming the config key; `test-env` seam; guide updated | #1709 | open |
| M2 | T084b, T085b, T086a, T086 | full | `MICOLD_DATA_DIR`/`MICOLD_TOKEN_PATH` retired; images, SSH launch, entrypoint stop setting them; container-over-SSH regression test; guide; quickstart | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | user | prompt |
| D2 | tasks | Split Issue 15? | Yes, two milestones: daemon-side settings + seam (M1), core reads + images/SSH launch + regression test (M2); T084/T085/T086 split into a/b tasks | agent-resolved | milestones.md rule 3; ~57 files, images and SSH launch still set the variables |
| D3 | tasks | User-guide task | T086b in M1 (visible behaviour), T086 in M2 | agent-resolved | milestones.md rule 4 |
| D4 | tasks | `MICOLD_SSH_DIR` | No read site exists in the repo; nothing to remove | agent-resolved | grep |
| D5 | tasks | Tier | full for both: process env, token check, sandbox boundary | agent-resolved | milestone-format.md |

## Review rounds

M1: review A (code-review high) snapshot cd610b30:2c612ac0 round 1: F1 MAJOR fixed (capture scene waits on an info-level line; the app overwrites the host daemon.toml), F2 declined (DATA_DIR/TOKEN_PATH stay live until M2 by design), F3-F5 MINOR left. Review B (sonnet): CLEAN, 3 MINOR (F2 fixed). The gate caught the basic image content hash; pin updated.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1537 | 1 | 72c2d2e1:3ffb138f | CHANGES: F1 capture lib.sh producer, F2 retired_env module uncreated, F3 entrypoint wording; all fixed (task text), F4-F6 MINOR fixed |

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 15 (#1537)". The quickstart run is recorded by the close unit.

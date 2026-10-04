# Flow: bugfix (medium)

Broken behaviour that a spec describes wrongly or misses: a requirement, a scenario or a task
marked done that is not. The spec is patched and the fix ships as one milestone. Two units, one PR.

| Unit | Task files, in order | `model` |
|---|---|---|
| Bugfix | [bugfix](../tasks/bugfix.md), [review](../tasks/review.md), [review-rounds](../tasks/review-rounds.md) | `effort:high` or none: omit (session model). `effort:low`, or a report that already names the root cause and the fix: `"sonnet"` |
| Milestone | [implement](../tasks/implement.md), [verify](../tasks/verify.md), review-rounds, [gate](../tasks/gate.md), [pr](../tasks/pr.md) | the `TIER` the bugfix unit returned: `full` omit, `light` `"sonnet"` |

The bugfix unit opens no PR: dispatch the milestone unit at once, with the BUG record and the fix's
task IDs. Continued with a red CI log, the milestone unit also reads [red-ci](../tasks/red-ci.md).

- **Keeps:** the issue, the BUG record and spec patch (`speckit-bugfix-report`, `-patch`,
  `-verify`), a reviewer on the patch, a regression test first, the scoped then the full local
  gate, review A at `medium`, the visual pass when something visible changed, green CI.
- **Skips:** spec, clarify, plan and tasks units, review B, the close unit.
- **Ledger:** `specs/<NNN>-*/bugs/BUG-<issue>.autopilot.md` beside the BUG record, **Kind** `bugfix`,
  one milestone row.
- **Ends with:** one PR with record, patch, regression test and fix merged, then the handoff. Or
  `NEXT: feature` (no spec owns it and behaviour is new, or the fix outgrew the cap) or
  `NEXT: bug` (no spec needs changing).
- **Cap:** [switch.md](switch.md) *Effort caps*.

# Flow: bug (least effort)

Broken behaviour that no spec covers, or whose spec is right and stays as it is. One unit, one PR.

| Unit | Task files, in order | `model` |
|---|---|---|
| Bug | [bug](../tasks/bug.md), [gate](../tasks/gate.md), [pr](../tasks/pr.md) | `effort:low` or none: `"sonnet"`. `effort:high`: omit (session model) |

Continued with a red CI log, the unit also reads [red-ci](../tasks/red-ci.md).

- **Keeps:** the issue, a red test first, the full local gate, review A at `medium`, green CI.
- **Skips:** spec, clarify, plan, tasks, the BUG record and spec patch, review B, the scoped gate,
  the close unit (`speckit-tdd-verify`, `speckit-converge`).
- **Ledger:** `specs/quick/<YYYY-MM-DD>-<issue>-<slug>.autopilot.md` (gitignored, never
  committed), **Kind** `bug`, one milestone row `B`.
- **Ends with:** the fix PR merged, then the handoff. Or `NEXT: bugfix` (a spec is wrong),
  `NEXT: feature` (the fix is new behaviour) or `NEXT: chore` (nothing a user sees is broken).
- **Cap:** [switch.md](switch.md) *Effort caps*.

# Flow: chore (no Spec Kit)

CI, build, tooling, tests, docs, a dependency or config bump, a small refactor: no behaviour a
user sees changes, and no spec requirement is touched. One unit, one PR.

| Unit | Task files, in order | `model` |
|---|---|---|
| Chore | [chore](../tasks/chore.md), [gate](../tasks/gate.md), [pr](../tasks/pr.md) | `effort:high`: omit (session model). `effort:low`: `"sonnet"`. None: `"sonnet"`, or `"haiku"` when the issue names only docs or config files |

Continued with a red CI log, the unit also reads [red-ci](../tasks/red-ci.md).

- **Keeps:** the issue, a red test first for a code change, the local gate that fits the diff,
  review A at `low`, green CI.
- **Skips:** every Spec Kit skill, the BUG record, review B, the scoped gate, the visual pass, the
  close unit.
- **Ledger:** `specs/quick/<YYYY-MM-DD>-<issue>-<slug>.autopilot.md` (gitignored, never
  committed), **Kind** `chore`, one milestone row `C`.
- **Ends with:** the PR merged, then the handoff. Or `NEXT: bug` (behaviour is broken),
  `NEXT: bugfix` (a spec is wrong) or `NEXT: feature` (new behaviour).
- **Cap:** [switch.md](switch.md) *Effort caps*. A chore the user sized `effort:high` may pass the
  line cap when it stays one PR and changes no behaviour; say so in the PR body.

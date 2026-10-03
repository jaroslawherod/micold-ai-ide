# Switching flow, and the effort caps

Read when a unit returns `NEXT: bug | bugfix | feature | chore`, or a bug or chore unit hands over.

## Switching

- **The user named the flow** (in the command or with a `flow:*` label) and the unit wants a
  costlier one: ask, as in [choose.md](choose.md) *When to ask*, with the unit's reason.
- **Otherwise follow it** when the unit names a hard reason: a spec requirement is wrong → bugfix;
  new behaviour → feature; no spec involved → bug; no behaviour changes → chore. Unsure: ask.

The issue stays the same. Move its label with
`scripts/autopilot/issue.sh flow <n> <new-flow> [high|low]`, then start the new flow's first unit
with what the old unit found as its scope. The new unit creates its own ledger; the old unit
closed its own before it returned.

## Effort caps

| Flow | Cap |
|---|---|
| bug, chore | one PR, and the size its task file gives; a `HANDOVER` limit in [../tasks/dispatch.md](../tasks/dispatch.md) |
| bugfix | the patch adds at most 10 tasks, one milestone, one PR |
| feature | none; a milestone splits at about 15 tasks or 800 changed lines |

Past its cap a unit returns `NEXT:` with the flow that fits. A bug past its cap with no spec to
change fits no lighter flow: the unit escalates (category 1), and you ask.

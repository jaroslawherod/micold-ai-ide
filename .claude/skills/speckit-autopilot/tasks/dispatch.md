# Task: dispatch a unit (orchestrator)

When: the flow's file names the next unit, and the previous PR (if any) has merged.

Run each unit in its own `autopilot-unit` subagent (`general-purpose` when your agent types lack
it: the type drops tool schemas a unit never uses, about 10k tokens on each of its calls). The
`model` is the one the flow's file gives. A unit keeps its model when continued with `SendMessage`.

- **Description:** name the unit (`Milestone M2 553`, `Plan 553`, `Chore 587`). Token reports
  group by it.
- **Prompt:** keep this opening fixed for every unit, so the prompt cache reuses it:
  `You are a speckit-autopilot unit. Read .claude/skills/speckit-autopilot/rules/unit.md and follow it.`
  Then, in this order:
  1. the unit's description (the unit passes it to `checkpoint.sh`);
  2. **the task files to read**: exactly the paths the flow's file lists for this unit, and no
     others;
  3. the flow and level, the issue number, and the labels read at entry;
  4. the ledger path (`none yet` for the first unit), the worktree path and branch;
  5. the previous PR and its merge SHA (`none` for the first unit and after a unit without a PR);
  6. the scope: for the first unit, the user's prompt or the issue's title and body verbatim; after
     a `NEXT:`, what the old unit found; for a milestone, its ID and task IDs (and `BUG-<issue>` in the
     bugfix flow).
- **Then hold:** [../rules/waiting.md](../rules/waiting.md), *The orchestrator*.

## When it returns

Read the ledger (`scripts/autopilot/brief.py ledger <ledger>`), never the transcript.

| Return | Do |
|---|---|
| `DONE` with a PR | [merge.md](merge.md) |
| `DONE` with `PR: none` | The commits stay on the branch: dispatch the next unit at once |
| `DONE` with `NEXT: <flow>` | [../flows/switch.md](../flows/switch.md) |
| `DONE` with `NEW MILESTONES: …` | Run those milestone units, then a new close unit |
| `ESCALATE` | Ask the returned questions yourself ([ask.md](ask.md)), then continue the unit with the answers (*Continuing a unit*) |
| `FAILED` | Read the ledger and the five lines. A unit on a cheaper model: retry once on the session model, from the ledger's *Handover*. Otherwise retry once with a fresh unit, or escalate |
| `HANDOVER` | Its context passed 150k. Dispatch a fresh unit of the same kind, model, task files and scope, with `part <n>` added to the description and `Continue from the ledger's Handover.` in the prompt. A fourth part (a third, in the bug and chore flows) is an escalation (category 5), with what is done and what is left. If it had opened its PR, the new part finishes the work on that PR; send any `RED` log to the latest part |

## Continuing a unit

With answers, a red CI log or other feedback. Waking a unit re-writes its whole context, and every
later call re-reads it; in measured runs that was 1.8M cost_eq.

- **Its return said `CONTEXT` under 100k:** `SendMessage` to the same subagent.
- **100k or more, or no `CONTEXT` line:** a fresh unit of the same kind, model, task files and
  scope, with `part <n>` in the description, `Continue from the ledger.` and the feedback verbatim.
  It counts as a part toward the `HANDOVER` limit.

A unit that prepares a later milestone in another worktree follows the same rules, the 150k
handover included; the hooks cover it there.

Next: the row above.

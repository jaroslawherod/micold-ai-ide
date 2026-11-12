# Data model

## Supervisor ledger (`.autopilot/supervisor.md`)

```markdown
# Autopilot supervisor ledger
- **Owner**: <supervisor session id>
- **Cap**: 3
- **Grace**: 600          # seconds a fresh start may stay dead-no-ledger
- **Source**: label autopilot:queue | issues 12 14 15

| Issue | State | Touches | Worktree | Session | Started | Tries | Note |
|---|---|---|---|---|---|---|---|
| 12 | running | crates/a | /path/wt | s-1 | 1700000000 | 1 | |
```

Rules: cells hold no `|`; `Touches` is comma separated, `-` when none; `Started` is epoch seconds
(set when the state becomes `starting`); one row per issue, issue numbers unique.

## Issue states

`queued` -> `starting` -> `running` -> `done`. Any state may go to `lost`. `queued` or `starting`
-> `failed` after 3 tries. `queued` -> `skipped` (closed, missing, or the issue already has a worktree or a ledger of its own: `supervisor.sh ledger set <n> State skipped` with the reason in Note).
Occupying a slot: `starting`, `running`. Released (never block a later issue): `done`, `failed`,
`lost`, `skipped`. `queued` holds no slot but is ordered by issue number.

## State mapping (FR-011)

| run-status `state` | seconds since `Started` | ledger state |
|---|---|---|
| `done` | any | `done` |
| `dead-no-ledger` | <= Grace | `running` (no alarm) |
| `dead-no-ledger` | > Grace | `lost` (reported) |
| `asking`, `stuck`, `blocked`, `waiting-ci`, `running` | any | `running`, reported when asking/stuck/blocked |
| (run-status exit 2: worktree gone, or exit 3: several ledgers) | any | `lost`, the exit code in the note |
| (row `starting`, Worktree or Session cell empty) | any | the skill looks for an existing worktree and session of the issue; found: fill the cells, `running`; none: back to `queued`, a try counted |
| (session id not in live list, state not `done`) | any | `lost` |

## Gate reading

`mem_mib`, `load1`, `cores`, `running` (rows in `starting`/`running`), `cap`. Fails in this order:
cap, memory (< 4096 MiB), load (> cores). Prints the first failing check.

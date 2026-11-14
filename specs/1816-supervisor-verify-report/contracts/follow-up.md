# Contract: follow-up.sh

`scripts/autopilot/follow-up.sh <worktree-dir> --supervisor-ledger <file> --session live|stopped
[--session-id <id>] [--check <id>]`

Verifies the run (`verify-run.sh`) and prints the next action for the supervisor to perform. The
supervisor makes the Micold call; the script has no session transport.

## Output (first token of line 1 is the action)
| Action | When | Further lines |
|---|---|---|
| `DONE` | the run is `VERIFIED` | none; count unchanged |
| `SEND <session-id>` | `NOT DONE`, session `live`, count < 3 | the message: `Run not done. Remaining:` then the remaining lines; count += 1 |
| `RESUME <session-id>` | `NOT DONE`, session `stopped`, count < 3 | the remaining lines; count += 1 |
| `SKIP` | the remaining-set hash and `--check` id equal the row's | none; count unchanged |
| `ESCALATE` | `NOT DONE`, count is 3, no block yet | the banner; one category 5 block written to `## Escalations` (what remains, what was sent) |
| `HOLD` | `NOT DONE`, count is 3, block exists | none; nothing written |

Defaults: `--session-id` absent prints the action with an empty id; `--check` absent disables `SKIP`. Order: `DONE`, then `HOLD`/`ESCALATE` (cap), then `SKIP`, then `SEND`/`RESUME`.

Exit 0 for every action; 2 usage; 3 several ledgers (as `verify-run.sh`). The ledger is written only
by the actions that change the count or write the block.

## Supervisor ledger parts
See data-model *Supervisor ledger*. A missing file is created with only the needed section; a
missing section is appended; rows are keyed by the run's branch, so two runs never share a count.

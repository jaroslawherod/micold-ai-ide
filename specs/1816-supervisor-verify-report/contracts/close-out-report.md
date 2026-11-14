# Contract: close-out-report.sh and safe-to-remove.sh

## close-out-report.sh
`scripts/autopilot/close-out-report.sh <worktree-dir>... [--supervisor-ledger <file>] [--live <path>]...
[--no-live-sessions]`

Markdown on stdout, exit 0 (also for zero runs). Sections, always present, in this order:

```
## Completed
### #<issue> <branch>
- PRs: #<n> MERGED, ...
- Decisions: <u> by user, <a> by agent
- Interventions: none | <one line per supervisor-ledger row>
- Tokens: <n> | unknown
## Not done
### #<issue> <branch>
- PRs: ...
- Remaining: <one line per item>       (a run whose verify exits 2 or 3 gives one error line here)
## Escalated
### #<issue> <branch>
- <the escalation block's Question line>
## Safe to remove
- <path> | none
```

A run is under Completed only when `verify-run.sh` returns `VERIFIED`. A run that is not `VERIFIED` and has a block in the
supervisor ledger's `## Escalations` (also when verify exits 2 or 3) is under Escalated and not under Not done. Precedence: Completed, then Escalated, then Not done. A run absent from
the ledgers' lists or errored never aborts the report.

## safe-to-remove.sh
`scripts/autopilot/safe-to-remove.sh <worktree-dir>... [--live <path>]... [--no-live-sessions]
[--supervisor-ledger <file>]`

One line per worktree: `SAFE <path>` or `KEEP <path> <reason>` (data-model *Safe-to-remove entry*).
With `- **Auto-remove**: yes` in the supervisor ledger, each `SAFE` line is followed by `REMOVE
<path>`: a request the supervisor passes to the app's `delete_worktree`. The script itself removes
nothing. Exit 0; 2 usage.

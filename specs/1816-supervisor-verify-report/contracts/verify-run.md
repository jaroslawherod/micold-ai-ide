# Contract: verify-run.sh

`scripts/autopilot/verify-run.sh <worktree-dir>` (default: the current directory)

## Behaviour
Runs `run-status.sh --json <worktree-dir>` (which runs `handoff-check.sh` inside that worktree, never
the caller's), then `gh issue view <n> --json state,labels` for the ledger's issue. Read-only: no
ledger, branch, PR or issue change.

## Output
```
VERIFIED
```
or
```
NOT DONE
- <one remaining item per line>
```
Items: see data-model *Verdict*. Verdict is `VERIFIED` only when the ledger phase is `done`, the
handoff check passed, and the issue is closed, without `in-progress`, with `flow:<flow>` (the step
is skipped for issue `none`; the ledger's `#<n>` is passed to `gh` without the `#`).

## Exit
| Code | Meaning |
|---|---|
| 0 | `VERIFIED` |
| 1 | `NOT DONE` |
| 2 | usage error, or the path is not a git worktree (stderr names it) |
| 3 | several ledgers name the branch (listed on stderr, no verdict) |

Env: `AUTOPILOT_PR_STATE`, `AUTOPILOT_NOW` pass through to `run-status.sh`.

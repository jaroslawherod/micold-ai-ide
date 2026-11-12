# Quickstart: supervisor verify handoff and report

No visual pass applies (shell tooling, no GUI).

## A. Automated
```
bash scripts/tests/close-out.test.sh     # every verdict, the cap, report sections, safe list
mise run gate                            # CI's order, before pushing
```

## B. By hand (SC-003, SC-006), from any worktree
```
scripts/autopilot/verify-run.sh <run-worktree>          # VERIFIED, or NOT DONE and what remains
time scripts/autopilot/verify-run.sh <run-worktree>     # under 10 s with GitHub reachable
scripts/autopilot/follow-up.sh <run-worktree> --supervisor-ledger <file> --session live --session-id <id>
scripts/autopilot/close-out-report.sh <run-worktree>... --supervisor-ledger <file> --no-live-sessions
scripts/autopilot/safe-to-remove.sh <run-worktree>... --no-live-sessions
```
Check: an open-PR run is under Not done, never Completed; no `REMOVE` line appears without the
`- **Auto-remove**: yes` field in the supervisor ledger; no worktree disappears.

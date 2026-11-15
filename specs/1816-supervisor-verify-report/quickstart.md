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

## Results (2026-11-15, close unit)

- **A. Automated**: `scripts/tests/close-out.test.sh` 110 cases, 0 failed; `supervisor.test.sh` 291 cases and
  `supervisor-ownership.test.sh` 39 cases, 0 failed.
- **B. By hand**: not run. `gh` is not authenticated in the cloud session, so a real run's issue and PR
  state cannot be read; SC-003 and SC-006 (under 10 s) stay unmeasured against live GitHub.

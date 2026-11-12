# Quickstart

## Part A: automated
`scripts/tests/supervisor.test.sh` (run by `mise run gate`): gate, next, map, ledger, watcher and
skill-text cases. Single run: `bash scripts/tests/supervisor.test.sh`.

## Part B: once by hand, at the project root
1. Label two cheap issues `autopilot:queue`, run `/autopilot-supervisor` with `cap 1`.
2. Expect: one worktree and session for the lower issue; a table; the second stays `queued`.
3. Kill the supervisor session, start it again: it keeps the first as running, starts nothing twice.
4. In the started session, `scripts/autopilot/supervised.sh` exits 0 (the `supervised` argument took effect).
5. When the first reports `done`, the second starts.

# Cycle Log: 1816 M1 verify-run.sh

Append only. Newest last.

## Cycle 1 — US1.1 to US1.5, FR-001..FR-005 (T001–T006 → T007)

- tests: `scripts/tests/close-out.test.sh` (new): all-merged VERIFIED, open and UNKNOWN PR, dirty tree,
  unmerged commit, open phase, no ledger, no PRs (verified), several open ledgers, not a worktree,
  issue open / in-progress / flow label missing / UNKNOWN, second-clone isolation, read-only checksums.
- red: the same file run with `scripts/autopilot/verify-run.sh` removed → `close-out: 22 cases, 21 failed`
  (the one pass is the "not a worktree" case, which fails as exit 127 text-match only by chance of
  the usage text). Evidence kept in the session scratchpad (`red1.log`).
- green: `verify-run.sh` added (T007) → `close-out: 22 cases, 0 failed`.

## Cycle 2 — review A F1, F2, F3 (ledger-less open PR, several done ledgers, run-status failure)

- tests: three cases in `close-out.test.sh`, committed before the fix (commit "test(1816): red cases …").
- red: `close-out: 25 cases, 3 failed` — "the branch's open PR missing from the ledger is listed" (exit 0,
  want 1: VERIFIED), "several done ledgers are exit 3 too" (exit 0, want 3), "a failing run-status.sh
  is NOT DONE" (exit 0, want 1).
- green: `verify-run.sh` lists every PR run-status.sh reports not MERGED once, refuses several
  ledgers with exit 3, maps a run-status.sh failure to NOT DONE, and needs state `done` for VERIFIED
  → `close-out: 25 cases, 0 failed`.

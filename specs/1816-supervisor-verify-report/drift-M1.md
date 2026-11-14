# Plan drift, M1

| Path | Reason |
|---|---|
| `contracts/verify-run.md` | A spec reference read by T007, not a planned edit. |
| `run-status.sh` | Called by `verify-run.sh` (FR-002), not changed. |
| `scripts/tests/lib/autopilot-fixtures.sh` | T002 is conditional ("only if missing"): the `gh` stub already serves `issue-<n>.json`, `pr-list.json` and the ledger writer lives in the test, so the library is unchanged. |

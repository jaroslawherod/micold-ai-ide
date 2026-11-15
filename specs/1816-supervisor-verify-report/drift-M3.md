# Plan drift, M3

| Path | Reason |
|---|---|
| `contracts/close-out-report.md` | A spec reference read by T014 (and updated for `--parent`/`--rows`), not a planned new file. |
| `scripts/autopilot/runs-of-parent.sh` | An existing script (#1860) that T020 calls, not a planned edit. |
| `scripts/autopilot/supervisor-ledger.sh` | An existing script (M2) that T014 sources, not a planned edit. |
| `scripts/autopilot/verify-run.sh` | An existing script (M1) that T014 runs per run, not a planned edit. |

Planned-path check (`plan-drift.sh` reads bare file names): `runs-of-parent.sh`, `supervisor-ledger.sh` and `verify-run.sh` are named by T014 and T020 as dependencies the new script calls or sources; M3's diff does not touch them, by design.

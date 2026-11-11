# Plan drift, M1

Measured over `07de67eaa~1..653cd7215` (the milestone's own commits; a diff against the merge base also sweeps in other issues' commits on `main`).

- `.autopilot.md`: a glob named in a task (`<feature>.autopilot.md` ledgers), not a file to touch.
- `contracts/cli.md`: lives under `specs/1802-verdict-head-sha/`, a spec artifact the tasks only cite.
- `ledger-of.sh`: cited as the pattern source for `LEDGER_PATHSPECS` (T002), deliberately unchanged.
- `.test.sh`: fragment of the test-file names the tasks mention, not a path.

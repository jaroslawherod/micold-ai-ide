Drift M4: one unplanned file change, one command named as a file.

- `scripts/autopilot/supervisor.sh` — no M4 task names it. `touches` now takes the first non-empty `Touches:` line (an empty one before a populated one used to win). It is a carried MINOR from Review A M3 round 2 (the M3 `touches` behaviour), fixed with a test in `scripts/tests/supervisor.test.sh`.
- `run-status.sh` — T022 names the command, not a file to change; plan-drift prints `MISSING run-status.sh` for it. It is unchanged and exists at `scripts/autopilot/run-status.sh`.
- TDD: no `tdd/` cycle-log; the red evidence is in `autopilot.md`'s *Red runs*.

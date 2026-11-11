# Plan drift, M2

Measured over `829d1f1cd~1..c6817744c`.

- `scripts/autopilot/wait-merge.sh`: review A finding F1 (commit 15b4d1e45): a `CARRIED` guard result must pin `--match-head-commit` to the guarded head, so `wait-merge.sh` takes the head from the guard's last line for `CARRIED` as it does for `ALLOWED`. A two-line fix, no new behaviour beyond FR-009.

# Plan drift, M1 (#1820)

No unplanned files. Planned paths the diff does not touch, with reasons:

- brief.py, checkpoint.sh, review-snapshot.sh, scripts/autopilot/review-snapshot.sh: existing readers the tasks run against the fixture; they are inputs, not edited (T007a only verifies them).
- contracts/facts-file.md: lives under specs/1804-staged-review/contracts/, written in the design PR #1809.

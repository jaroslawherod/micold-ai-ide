# Quickstart: validate

Prerequisites: `git`, `jq`; no GitHub access (the test stubs `gh`).

1. `bash scripts/tests/verdict-guard.test.sh` : every FR-010 case passes.
2. Manual, in a throw-away repo from the test: record `CLEAN` at head A with
   `scripts/autopilot/record-verdict.sh`, amend the message (carried), then change a hunk
   (`VERDICT-REFUSED ... re-review`, exit 7).
3. `bash scripts/tests/autopilot.test.sh` : existing cases pass except the hook cases at lines 514 and 518 (merges of PR 11 and 12), which now expect the use-wait-merge deny.
4. `mise run gate` : SC-004.
